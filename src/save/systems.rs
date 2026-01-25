//! 存檔系統
//!
//! 處理存檔、讀檔、自動存檔邏輯
//!
//! 使用非同步 IO 避免阻塞主執行緒

#![allow(dead_code)] // Phase 5+ 預留功能

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::combat::{Armor, Health, Weapon, WeaponInventory, WeaponStats, WeaponType};
use crate::core::{PlayerStats, WeatherState, WeatherType, WorldTime};
use crate::economy::PlayerWallet;
use crate::mission::{RelationshipManager, RespectManager, StoryMissionManager, UnlockManager};
use crate::player::Player;
use crate::vehicle::{ModLevel, NitroBoost, VehicleId, VehicleModifications};

use super::components::*;

// ============================================================================
// 改裝等級轉換輔助函數
// ============================================================================

/// ModLevel 轉換為 u8
fn mod_level_to_u8(level: ModLevel) -> u8 {
    match level {
        ModLevel::Stock => 0,
        ModLevel::Level1 => 1,
        ModLevel::Level2 => 2,
        ModLevel::Level3 => 3,
    }
}

/// u8 轉換為 ModLevel
fn u8_to_mod_level(value: u8) -> ModLevel {
    match value {
        1 => ModLevel::Level1,
        2 => ModLevel::Level2,
        3 => ModLevel::Level3,
        _ => ModLevel::Stock,
    }
}

// ============================================================================
// 非同步任務追蹤
// ============================================================================

/// 存檔任務追蹤資源
#[derive(Resource, Default)]
pub struct SaveTaskTracker {
    /// 當前存檔任務
    pub save_task: Option<Task<Result<PathBuf, SaveError>>>,
    /// 當前讀檔任務
    pub load_task: Option<Task<Result<SaveData, SaveError>>>,
    /// 待套用的存檔資料（讀檔完成後設置）
    pub pending_load_data: Option<SaveData>,
}

// ============================================================================
// 輸入處理
// ============================================================================

/// 處理存讀檔快捷鍵
pub fn handle_save_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    save_manager: Res<SaveManager>,
    mut save_events: MessageWriter<SaveGameEvent>,
    mut load_events: MessageWriter<LoadGameEvent>,
) {
    // F5 = 快速存檔
    if keyboard.just_pressed(KeyCode::F5) {
        if !save_manager.is_busy {
            save_events.write(SaveGameEvent {
                save_type: SaveType::QuickSave,
                slot: None,
            });
            info!("快速存檔中...");
        }
    }

    // F9 = 快速讀檔
    if keyboard.just_pressed(KeyCode::F9) {
        if !save_manager.is_busy {
            load_events.write(LoadGameEvent {
                load_type: LoadType::QuickLoad,
                slot: None,
            });
            info!("快速讀檔中...");
        }
    }
}

// ============================================================================
// 存檔處理
// ============================================================================

/// 處理存檔事件（非同步版本）
pub fn handle_save_events(
    mut events: MessageReader<SaveGameEvent>,
    mut save_manager: ResMut<SaveManager>,
    mut task_tracker: ResMut<SaveTaskTracker>,
    // 讀取遊戲狀態
    player_query: Query<(&Transform, &Player, Option<&Health>, Option<&Armor>)>,
    wallet: Res<PlayerWallet>,
    weapon_query: Query<&WeaponInventory, With<Player>>,
    world_time: Res<WorldTime>,
    weather_state: Res<WeatherState>,
    story_manager: Res<StoryMissionManager>,
    respect: Res<RespectManager>,
    unlocks: Res<UnlockManager>,
    relationship: Res<RelationshipManager>,
    player_stats: Res<PlayerStats>,
    // 車輛改裝查詢（包含穩定 ID）
    vehicle_mod_query: Query<(Entity, &VehicleId, &VehicleModifications)>,
) {
    // 檢查是否已有任務在執行
    if task_tracker.save_task.is_some() {
        return;
    }

    for event in events.read() {
        save_manager.is_busy = true;

        // 收集存檔資料（在主執行緒進行，因為需要存取 ECS）
        let save_data = collect_save_data(
            &player_query,
            &wallet,
            &weapon_query,
            &world_time,
            &weather_state,
            &story_manager,
            &respect,
            &unlocks,
            &relationship,
            &player_stats,
            &vehicle_mod_query,
        );

        // 決定存檔路徑
        let save_path = match event.save_type {
            SaveType::Slot => {
                let slot = event.slot.unwrap_or(save_manager.current_slot);
                save_manager.get_save_path(slot)
            }
            SaveType::QuickSave => save_manager.get_quick_save_path(),
            SaveType::AutoSave => save_manager.get_auto_save_path(),
        };

        // 確保目錄存在（快速同步操作）
        if let Err(e) = save_manager.ensure_directory() {
            error!("無法建立存檔目錄: {:?}", e);
            save_manager.is_busy = false;
            continue;
        }

        // 序列化資料（在主執行緒進行，通常很快）
        let json = match serde_json::to_string_pretty(&save_data) {
            Ok(j) => j,
            Err(e) => {
                error!("序列化失敗: {:?}", e);
                save_manager.is_busy = false;
                continue;
            }
        };

        // 在背景執行緒執行 IO
        let task_pool = AsyncComputeTaskPool::get();
        let path = save_path.clone();
        let task = task_pool.spawn(async move { perform_save_async(json, path).await });
        task_tracker.save_task = Some(task);

        info!("存檔任務已啟動: {:?}", save_path);
        break; // 一次只處理一個存檔事件
    }
}

/// 輪詢存檔任務完成狀態
pub fn poll_save_task(
    mut save_manager: ResMut<SaveManager>,
    mut task_tracker: ResMut<SaveTaskTracker>,
) {
    if let Some(ref mut task) = task_tracker.save_task {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            match result {
                Ok(path) => info!("存檔完成: {:?}", path),
                Err(e) => error!("存檔失敗: {:?}", e),
            }
            task_tracker.save_task = None;
            save_manager.is_busy = false;
        }
    }
}

/// 非同步執行存檔 IO
async fn perform_save_async(json: String, path: PathBuf) -> Result<PathBuf, SaveError> {
    // 使用 async-std 或直接在背景執行緒同步寫入
    // Bevy 的 AsyncComputeTaskPool 已經在背景執行緒運行
    std::fs::write(&path, json).map_err(|e| SaveError::IoError(e.to_string()))?;
    Ok(path)
}

/// 收集存檔資料
fn collect_save_data(
    player_query: &Query<(&Transform, &Player, Option<&Health>, Option<&Armor>)>,
    wallet: &PlayerWallet,
    weapon_query: &Query<&WeaponInventory, With<Player>>,
    world_time: &WorldTime,
    weather_state: &WeatherState,
    story_manager: &StoryMissionManager,
    respect: &RespectManager,
    unlocks: &UnlockManager,
    relationship: &RelationshipManager,
    _player_stats: &PlayerStats,
    vehicle_mod_query: &Query<(Entity, &VehicleId, &VehicleModifications)>,
) -> SaveData {
    let mut save_data = SaveData::default();

    // 時間戳
    save_data.timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // 玩家資料
    if let Ok((transform, _player, health, armor)) = player_query.single() {
        let pos = transform.translation;
        save_data.player.position = [pos.x, pos.y, pos.z];
        save_data.player.rotation_y = transform.rotation.to_euler(EulerRot::YXZ).0;

        if let Some(h) = health {
            save_data.player.health = h.current;
            save_data.player.max_health = h.max;
        }
        if let Some(a) = armor {
            save_data.player.armor = a.current;
        }
    }

    // 錢包資料
    save_data.player.cash = wallet.cash;
    save_data.player.bank = wallet.bank;
    save_data.player.respect = respect.respect;
    save_data.stats.total_money_earned = wallet.total_earned;
    save_data.stats.total_money_spent = wallet.total_spent;

    // 武器資料
    if let Ok(inventory) = weapon_query.single() {
        save_data.player.weapons = inventory
            .weapons
            .iter()
            .map(|w| WeaponSaveData {
                weapon_type: format!("{:?}", w.stats.weapon_type),
                current_ammo: w.current_ammo,
                reserve_ammo: w.reserve_ammo,
            })
            .collect();
        save_data.player.current_weapon_index = inventory.current_index;
    }

    // 世界資料
    save_data.world.world_hour = world_time.hour;
    save_data.world.weather = format!("{:?}", weather_state.weather_type);
    save_data.world.weather_intensity = weather_state.intensity;

    // 任務資料
    save_data.missions.completed_missions = story_manager
        .get_completed_missions()
        .iter()
        .map(|id| format!("{:?}", id))
        .collect();
    save_data.missions.unlocked_items = unlocks.unlocked_items.clone();
    save_data.missions.unlocked_areas = unlocks
        .unlocked_areas
        .iter()
        .map(|id| id.to_string())
        .collect();
    save_data.missions.npc_relationships = relationship
        .relationships
        .iter()
        .map(|(k, v)| (format!("{}", k), *v))
        .collect();
    save_data.missions.flags = story_manager
        .story_flags
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    // 車輛改裝資料（使用穩定 VehicleId）
    #[allow(deprecated)] // vehicle_index 已棄用
    {
        save_data.world.vehicle_modifications = vehicle_mod_query
            .iter()
            .map(|(_entity, vehicle_id, mods)| VehicleModificationSaveData {
                vehicle_id: vehicle_id.as_u64(),
                vehicle_index: 0, // 已棄用，保留向後相容
                engine_level: mod_level_to_u8(mods.engine),
                transmission_level: mod_level_to_u8(mods.transmission),
                suspension_level: mod_level_to_u8(mods.suspension),
                brakes_level: mod_level_to_u8(mods.brakes),
                tires_level: mod_level_to_u8(mods.tires),
                armor_level: mod_level_to_u8(mods.armor),
                has_nitro: mods.has_nitro,
                nitro_charge: mods.nitro_charge,
            })
            .collect();
    }

    save_data
}

// ============================================================================
// 讀檔處理
// ============================================================================

/// 處理讀檔事件（非同步版本）
pub fn handle_load_events(
    mut events: MessageReader<LoadGameEvent>,
    mut save_manager: ResMut<SaveManager>,
    mut task_tracker: ResMut<SaveTaskTracker>,
) {
    // 檢查是否已有任務在執行
    if task_tracker.load_task.is_some() {
        return;
    }

    for event in events.read() {
        save_manager.is_busy = true;

        // 決定讀檔路徑
        let load_path = match event.load_type {
            LoadType::Slot => {
                let slot = event.slot.unwrap_or(save_manager.current_slot);
                save_manager.get_save_path(slot)
            }
            LoadType::QuickLoad => save_manager.get_quick_save_path(),
            LoadType::AutoSave => save_manager.get_auto_save_path(),
        };

        // 在背景執行緒執行 IO
        let task_pool = AsyncComputeTaskPool::get();
        let path = load_path.clone();
        let task = task_pool.spawn(async move { perform_load_async(path).await });
        task_tracker.load_task = Some(task);

        info!("讀檔任務已啟動: {:?}", load_path);
        break; // 一次只處理一個讀檔事件
    }
}

/// 輪詢讀檔任務完成狀態
pub fn poll_load_task(
    mut save_manager: ResMut<SaveManager>,
    mut task_tracker: ResMut<SaveTaskTracker>,
) {
    if let Some(ref mut task) = task_tracker.load_task {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            match result {
                Ok(save_data) => {
                    info!("讀檔完成，等待套用資料");
                    task_tracker.pending_load_data = Some(save_data);
                }
                Err(e) => {
                    error!("讀檔失敗: {:?}", e);
                    save_manager.is_busy = false;
                }
            }
            task_tracker.load_task = None;
        }
    }
}

/// 套用待處理的讀檔資料
pub fn apply_pending_load_data(
    mut commands: Commands,
    mut task_tracker: ResMut<SaveTaskTracker>,
    mut save_manager: ResMut<SaveManager>,
    mut player_query: Query<(
        &mut Transform,
        &mut Player,
        Option<&mut Health>,
        Option<&mut Armor>,
    )>,
    mut wallet: ResMut<PlayerWallet>,
    mut weapon_query: Query<&mut WeaponInventory, With<Player>>,
    mut world_time: ResMut<WorldTime>,
    mut weather_state: ResMut<WeatherState>,
    mut story_manager: ResMut<StoryMissionManager>,
    mut respect: ResMut<RespectManager>,
    mut unlocks: ResMut<UnlockManager>,
    mut relationship: ResMut<RelationshipManager>,
    mut vehicle_mod_query: Query<(
        Entity,
        &VehicleId,
        &mut VehicleModifications,
        Option<&NitroBoost>,
    )>,
) {
    if let Some(save_data) = task_tracker.pending_load_data.take() {
        apply_save_data(
            &mut commands,
            &save_data,
            &mut player_query,
            &mut wallet,
            &mut weapon_query,
            &mut world_time,
            &mut weather_state,
            &mut story_manager,
            &mut respect,
            &mut unlocks,
            &mut relationship,
            &mut vehicle_mod_query,
        );
        save_manager.is_busy = false;
        info!("存檔資料已套用");
    }
}

/// 非同步執行讀檔 IO
async fn perform_load_async(path: PathBuf) -> Result<SaveData, SaveError> {
    // 讀取檔案
    let json = std::fs::read_to_string(&path).map_err(|e| SaveError::IoError(e.to_string()))?;

    // 檔案大小限制（10MB）
    if json.len() > 10 * 1024 * 1024 {
        return Err(SaveError::InvalidValue {
            field: "file_size".to_string(),
            value: format!("{} bytes", json.len()),
            reason: "存檔檔案過大（超過 10MB）".to_string(),
        });
    }

    // 反序列化
    let save_data: SaveData =
        serde_json::from_str(&json).map_err(|e| SaveError::DeserializeError(e.to_string()))?;

    // 驗證存檔資料
    validate_save_data(&save_data)?;

    // 版本差異警告（舊版本可接受，但提示）
    if save_data.version < SAVE_VERSION {
        warn!(
            "存檔版本較舊: 存檔={}, 當前={}，部分功能可能無法還原",
            save_data.version, SAVE_VERSION
        );
    }

    Ok(save_data)
}

/// 套用存檔資料到遊戲狀態
fn apply_save_data(
    commands: &mut Commands,
    save_data: &SaveData,
    player_query: &mut Query<(
        &mut Transform,
        &mut Player,
        Option<&mut Health>,
        Option<&mut Armor>,
    )>,
    wallet: &mut PlayerWallet,
    weapon_query: &mut Query<&mut WeaponInventory, With<Player>>,
    world_time: &mut WorldTime,
    weather_state: &mut WeatherState,
    story_manager: &mut StoryMissionManager,
    respect: &mut RespectManager,
    unlocks: &mut UnlockManager,
    relationship: &mut RelationshipManager,
    vehicle_mod_query: &mut Query<(
        Entity,
        &VehicleId,
        &mut VehicleModifications,
        Option<&NitroBoost>,
    )>,
) {
    // 玩家資料
    if let Ok((mut transform, mut player, health, armor)) = player_query.single_mut() {
        let pos = save_data.player.position;
        transform.translation = Vec3::new(pos[0], pos[1], pos[2]);
        transform.rotation = Quat::from_rotation_y(save_data.player.rotation_y);

        player.money = save_data.player.cash as u32;

        if let Some(mut h) = health {
            h.current = save_data.player.health;
            h.max = save_data.player.max_health;
        }
        if let Some(mut a) = armor {
            a.current = save_data.player.armor;
        }
    }

    // 錢包資料
    wallet.cash = save_data.player.cash;
    wallet.bank = save_data.player.bank;
    respect.respect = save_data.player.respect;

    // 武器庫存還原
    if let Ok(mut inventory) = weapon_query.single_mut() {
        inventory.weapons.clear();
        for saved_weapon in &save_data.player.weapons {
            if let Some(stats) = weapon_stats_from_type_str(&saved_weapon.weapon_type) {
                let mut weapon = Weapon::new(stats);
                weapon.current_ammo = saved_weapon.current_ammo;
                weapon.reserve_ammo = saved_weapon.reserve_ammo;
                inventory.weapons.push(weapon);
            } else {
                warn!("無法解析武器類型: {}, 跳過", saved_weapon.weapon_type);
            }
        }
        // 確保至少有拳頭
        if inventory.weapons.is_empty() {
            inventory.weapons.push(Weapon::new(WeaponStats::fist()));
        }
        // 恢復當前武器索引（確保在有效範圍內）
        if save_data.player.current_weapon_index < inventory.weapons.len() {
            inventory.current_index = save_data.player.current_weapon_index;
        } else {
            inventory.current_index = 0;
        }
        info!(
            "還原 {} 把武器，當前武器索引: {}",
            inventory.weapons.len(),
            inventory.current_index
        );
    }

    // 世界資料
    world_time.hour = save_data.world.world_hour;

    // 天氣類型還原
    weather_state.weather_type = parse_weather_type(&save_data.world.weather);
    weather_state.intensity = save_data.world.weather_intensity.clamp(0.0, 2.0);
    info!(
        "還原天氣: {:?}, 強度: {}",
        weather_state.weather_type, weather_state.intensity
    );

    // 任務進度還原
    story_manager.completed_count = save_data.missions.completed_missions.len() as u32;
    // Restore unlocks
    unlocks.unlocked_items = save_data.missions.unlocked_items.clone();
    unlocks.unlocked_areas = save_data
        .missions
        .unlocked_areas
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    // Restore relationships
    relationship.relationships.clear();
    for (npc_str, value) in &save_data.missions.npc_relationships {
        if let Ok(npc_id) = npc_str.parse::<u32>() {
            relationship.relationships.insert(npc_id, *value);
        }
    }
    // Restore flags
    story_manager.story_flags = save_data.missions.flags.iter().cloned().collect();
    info!(
        "還原任務進度: {} 個已完成任務",
        story_manager.completed_count
    );

    // 車輛改裝資料（使用穩定 VehicleId 匹配）
    let vehicles: Vec<_> = vehicle_mod_query.iter_mut().collect();
    for (entity, vehicle_id, mut mods, nitro) in vehicles.into_iter() {
        // 嘗試從存檔中找到對應的改裝資料（優先使用 vehicle_id，回退到 vehicle_index）
        #[allow(deprecated)]
        let saved_mods = save_data
            .world
            .vehicle_modifications
            .iter()
            .find(|m| m.vehicle_id == vehicle_id.as_u64());

        if let Some(saved_mods) = saved_mods {
            // 恢復改裝等級
            mods.engine = u8_to_mod_level(saved_mods.engine_level);
            mods.transmission = u8_to_mod_level(saved_mods.transmission_level);
            mods.suspension = u8_to_mod_level(saved_mods.suspension_level);
            mods.brakes = u8_to_mod_level(saved_mods.brakes_level);
            mods.tires = u8_to_mod_level(saved_mods.tires_level);
            mods.armor = u8_to_mod_level(saved_mods.armor_level);
            mods.has_nitro = saved_mods.has_nitro;
            mods.nitro_charge = saved_mods.nitro_charge;

            // 如果有氮氣但沒有 NitroBoost 組件，添加它
            if saved_mods.has_nitro && nitro.is_none() {
                commands.entity(entity).insert(NitroBoost::new());
            }

            info!(
                "恢復車輛 ID={} 改裝: 引擎={:?}, 氮氣={}",
                vehicle_id.as_u64(),
                mods.engine,
                mods.has_nitro
            );
        }
    }
}

// ============================================================================
// 自動存檔
// ============================================================================

/// 處理自動存檔
pub fn handle_auto_save(
    mut events: MessageReader<AutoSaveEvent>,
    time: Res<Time>,
    mut save_manager: ResMut<SaveManager>,
    mut save_events: MessageWriter<SaveGameEvent>,
) {
    // 處理觸發事件
    for event in events.read() {
        if save_manager.auto_save_enabled && !save_manager.is_busy {
            save_events.write(SaveGameEvent {
                save_type: SaveType::AutoSave,
                slot: None,
            });
            save_manager.time_since_auto_save = 0.0;
            info!("自動存檔觸發: {:?}", event.reason);
        }
    }

    // 定時自動存檔
    if save_manager.auto_save_enabled {
        save_manager.time_since_auto_save += time.delta_secs();

        if save_manager.time_since_auto_save >= save_manager.auto_save_interval {
            if !save_manager.is_busy {
                save_events.write(SaveGameEvent {
                    save_type: SaveType::AutoSave,
                    slot: None,
                });
                save_manager.time_since_auto_save = 0.0;
                info!("定時自動存檔");
            }
        }
    }
}

// ============================================================================
// 錯誤類型
// ============================================================================

/// 存檔錯誤
#[derive(Debug)]
pub enum SaveError {
    IoError(String),
    SerializeError(String),
    DeserializeError(String),
    /// 存檔版本來自未來版本
    FutureVersion {
        save_version: u32,
        current_version: u32,
    },
    /// 數值超出有效範圍
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },
}

// ============================================================================
// 存檔驗證
// ============================================================================

/// 驗證存檔資料有效性
fn validate_save_data(data: &SaveData) -> Result<(), SaveError> {
    // 1. 版本檢查 - 不接受未來版本
    if data.version > SAVE_VERSION {
        return Err(SaveError::FutureVersion {
            save_version: data.version,
            current_version: SAVE_VERSION,
        });
    }

    // 2. 玩家數值驗證
    if data.player.health < 0.0 || data.player.health > 500.0 {
        return Err(SaveError::InvalidValue {
            field: "player.health".to_string(),
            value: data.player.health.to_string(),
            reason: "生命值應在 0-500 之間".to_string(),
        });
    }

    if data.player.armor < 0.0 || data.player.armor > 200.0 {
        return Err(SaveError::InvalidValue {
            field: "player.armor".to_string(),
            value: data.player.armor.to_string(),
            reason: "護甲值應在 0-200 之間".to_string(),
        });
    }

    // 3. 世界時間驗證
    if data.world.world_hour < 0.0 || data.world.world_hour > 24.0 {
        return Err(SaveError::InvalidValue {
            field: "world.world_hour".to_string(),
            value: data.world.world_hour.to_string(),
            reason: "遊戲時間應在 0-24 之間".to_string(),
        });
    }

    // 4. 天氣強度驗證
    if data.world.weather_intensity < 0.0 || data.world.weather_intensity > 2.0 {
        warn!(
            "天氣強度 {} 超出正常範圍，將限制在 0-2",
            data.world.weather_intensity
        );
    }

    // 5. 武器類型驗證（僅警告，不阻止讀檔）
    for weapon in &data.player.weapons {
        if !is_valid_weapon_type(&weapon.weapon_type) {
            warn!("未知武器類型: {}, 讀檔時將跳過", weapon.weapon_type);
        }
    }

    // 6. 車輛改裝等級驗證
    for vehicle_mod in &data.world.vehicle_modifications {
        if vehicle_mod.engine_level > 3
            || vehicle_mod.transmission_level > 3
            || vehicle_mod.suspension_level > 3
            || vehicle_mod.brakes_level > 3
            || vehicle_mod.tires_level > 3
            || vehicle_mod.armor_level > 3
        {
            warn!(
                "車輛 ID={} 改裝等級超過最大值 3，將自動限制",
                vehicle_mod.vehicle_id
            );
        }
    }

    Ok(())
}

/// 檢查武器類型是否有效
fn is_valid_weapon_type(weapon_type: &str) -> bool {
    parse_weapon_type(weapon_type).is_some()
}

/// 解析武器類型字串為 WeaponType
fn parse_weapon_type(weapon_type: &str) -> Option<WeaponType> {
    match weapon_type {
        "Fist" => Some(WeaponType::Fist),
        "Staff" => Some(WeaponType::Staff),
        "Knife" => Some(WeaponType::Knife),
        "Pistol" => Some(WeaponType::Pistol),
        "SMG" => Some(WeaponType::SMG),
        "Shotgun" => Some(WeaponType::Shotgun),
        "Rifle" => Some(WeaponType::Rifle),
        // 相容舊存檔的別名
        "AssaultRifle" => Some(WeaponType::Rifle),
        "Sniper" => Some(WeaponType::Rifle),
        "RPG" => Some(WeaponType::Shotgun), // 暫時映射
        "Melee" => Some(WeaponType::Fist),
        _ => None,
    }
}

/// 根據武器類型字串取得 WeaponStats
fn weapon_stats_from_type_str(weapon_type: &str) -> Option<WeaponStats> {
    match weapon_type {
        "Fist" | "Melee" => Some(WeaponStats::fist()),
        "Staff" => Some(WeaponStats::staff()),
        "Knife" => Some(WeaponStats::knife()),
        "Pistol" => Some(WeaponStats::pistol()),
        "SMG" => Some(WeaponStats::smg()),
        "Shotgun" | "RPG" => Some(WeaponStats::shotgun()),
        "Rifle" | "AssaultRifle" | "Sniper" => Some(WeaponStats::rifle()),
        _ => None,
    }
}

/// 解析天氣類型字串為 WeatherType
fn parse_weather_type(weather: &str) -> WeatherType {
    match weather {
        "Clear" => WeatherType::Clear,
        "Cloudy" => WeatherType::Cloudy,
        "Rainy" => WeatherType::Rainy,
        "Foggy" => WeatherType::Foggy,
        "Stormy" => WeatherType::Stormy,
        "Sandstorm" => WeatherType::Sandstorm,
        _ => {
            warn!("未知天氣類型: {}, 使用預設 Clear", weather);
            WeatherType::Clear
        }
    }
}
