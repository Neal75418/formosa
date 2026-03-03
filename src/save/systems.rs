//! 存檔系統
//!
//! 處理存檔、讀檔、自動存檔邏輯
//!
//! 使用非同步 IO 避免阻塞主執行緒

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::combat::{Armor, Health, WeaponInventory, WeaponStats, WeaponType};
use crate::core::{GameState, PlayerStats, WeatherState, WorldTime};
use crate::economy::{MoneyChangeReason, MoneyChangedEvent, PlayerWallet};
use crate::environment::DestroyedObjectTracker;
use crate::mission::{
    RelationshipManager, RespectManager, StoryMissionEvent, StoryMissionManager, UnlockManager,
};
use crate::player::Player;
use crate::vehicle::{ModLevel, VehicleId, VehicleModifications};

use super::components::*;

// ============================================================================
// 改裝等級轉換輔助函數
// ============================================================================

/// ModLevel 轉換為 u8
pub(super) fn mod_level_to_u8(level: ModLevel) -> u8 {
    match level {
        ModLevel::Stock => 0,
        ModLevel::Level1 => 1,
        ModLevel::Level2 => 2,
        ModLevel::Level3 => 3,
    }
}

/// u8 轉換為 ModLevel
pub(super) fn u8_to_mod_level(value: u8) -> ModLevel {
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
    // F5 = 快速存檔, F9 = 快速讀檔（互斥，同一幀最多觸發一個）
    if keyboard.just_pressed(KeyCode::F5)
        && !save_manager.is_busy {
            save_events.write(SaveGameEvent {
                save_type: SaveType::QuickSave,
                slot: None,
            });
            info!("💾 快速存檔中...");
        }
    else if keyboard.just_pressed(KeyCode::F9)
        && !save_manager.is_busy {
            load_events.write(LoadGameEvent {
                load_type: LoadType::QuickLoad,
                slot: None,
            });
            info!("💾 快速讀檔中...");
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
    game_state: Res<GameState>,
    // 車輛改裝查詢（包含穩定 ID）
    vehicle_mod_query: Query<(Entity, &VehicleId, &VehicleModifications)>,
    // 破壞持久化
    destroyed_tracker: Res<DestroyedObjectTracker>,
) {
    // 檢查是否已有存檔或讀檔任務在執行（互斥）
    if task_tracker.save_task.is_some() || task_tracker.load_task.is_some() {
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
            &game_state,
            &vehicle_mod_query,
            &destroyed_tracker,
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

        info!("💾 存檔任務已啟動: {:?}", save_path);
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
                Ok(path) => info!("💾 存檔完成: {:?}", path),
                Err(e) => error!("存檔失敗: {:?}", e),
            }
            task_tracker.save_task = None;
            if task_tracker.load_task.is_none() && task_tracker.pending_load_data.is_none() {
                save_manager.is_busy = false;
            }
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
    game_state: &GameState,
    vehicle_mod_query: &Query<(Entity, &VehicleId, &VehicleModifications)>,
    destroyed_tracker: &DestroyedObjectTracker,
) -> SaveData {
    let mut save_data = SaveData {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        ..SaveData::default()
    };

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

    // 車內狀態
    save_data.player.current_vehicle_id = game_state.current_vehicle.and_then(|entity| {
        vehicle_mod_query
            .iter()
            .find(|(e, _, _)| *e == entity)
            .map(|(_, vid, _)| vid.as_u64())
    });
    // 確保 in_vehicle 和 current_vehicle_id 一致
    save_data.player.in_vehicle =
        game_state.player_in_vehicle && save_data.player.current_vehicle_id.is_some();

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
                weapon_type: w.stats.weapon_type,
                current_ammo: w.current_ammo,
                reserve_ammo: w.reserve_ammo,
            })
            .collect();
        save_data.player.current_weapon_index = inventory.current_index;
    }

    // 世界資料
    save_data.world.world_hour = world_time.hour;
    save_data.world.weather = weather_state.weather_type;
    save_data.world.weather_intensity = weather_state.intensity;

    // 任務資料
    // v1 格式（向後相容）
    save_data.missions.completed_missions = story_manager
        .get_completed_missions()
        .iter()
        .map(|id| format!("{:?}", id))
        .collect();
    // v2 格式（完整 round-trip）
    save_data.missions.mission_states = story_manager.mission_states.clone();
    save_data.missions.current_chapter = story_manager.current_chapter;
    save_data.missions.best_ratings = story_manager.mission_ratings.clone();
    save_data.play_time_secs = story_manager.total_play_time as f64;

    save_data.missions.unlocked_items = unlocks.unlocked_items.iter().cloned().collect();
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

    // 破壞持久化資料
    save_data.world.destroyed_object_ids = destroyed_tracker.destroyed_list();

    save_data
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
            info!("💾 自動存檔: {:?}", event.reason);
        }
    }

    // 定時自動存檔
    if save_manager.auto_save_enabled {
        save_manager.time_since_auto_save += time.delta_secs();

        if save_manager.time_since_auto_save >= save_manager.auto_save_interval
            && !save_manager.is_busy {
                save_events.write(SaveGameEvent {
                    save_type: SaveType::AutoSave,
                    slot: None,
                });
                save_manager.time_since_auto_save = 0.0;
                info!("💾 定時自動存檔");
            }
    }
}

// ============================================================================
// 錯誤類型
// ============================================================================

/// 存檔錯誤
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum SaveError {
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
pub(super) fn validate_save_data(data: &SaveData) -> Result<(), SaveError> {
    if data.version > SAVE_VERSION {
        return Err(SaveError::FutureVersion {
            save_version: data.version,
            current_version: SAVE_VERSION,
        });
    }

    validate_player_data(&data.player)?;
    validate_world_data(&data.world)?;
    validate_vehicle_data(&data.world)?;

    // 武器類型已由 serde 反序列化時驗證，無需額外檢查

    Ok(())
}

fn validate_player_data(player: &PlayerSaveData) -> Result<(), SaveError> {
    if player.health < 0.0 || player.health > 500.0 {
        return Err(SaveError::InvalidValue {
            field: "player.health".to_string(),
            value: player.health.to_string(),
            reason: "生命值應在 0-500 之間".to_string(),
        });
    }

    if player.armor < 0.0 || player.armor > 200.0 {
        return Err(SaveError::InvalidValue {
            field: "player.armor".to_string(),
            value: player.armor.to_string(),
            reason: "護甲值應在 0-200 之間".to_string(),
        });
    }
    Ok(())
}

fn validate_world_data(world: &WorldSaveData) -> Result<(), SaveError> {
    if world.world_hour < 0.0 || world.world_hour > 24.0 {
        return Err(SaveError::InvalidValue {
            field: "world.world_hour".to_string(),
            value: world.world_hour.to_string(),
            reason: "遊戲時間應在 0-24 之間".to_string(),
        });
    }

    if world.weather_intensity < 0.0 || world.weather_intensity > 2.0 {
        warn!(
            "天氣強度 {} 超出正常範圍，將限制在 0-2",
            world.weather_intensity
        );
    }
    Ok(())
}

fn validate_vehicle_data(world: &WorldSaveData) -> Result<(), SaveError> {
    for vehicle_mod in &world.vehicle_modifications {
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

/// 根據武器類型取得 WeaponStats
pub(super) fn weapon_stats_from_type(weapon_type: WeaponType) -> WeaponStats {
    match weapon_type {
        WeaponType::Fist => WeaponStats::fist(),
        WeaponType::Staff => WeaponStats::staff(),
        WeaponType::Knife => WeaponStats::knife(),
        WeaponType::Pistol => WeaponStats::pistol(),
        WeaponType::SMG => WeaponStats::smg(),
        WeaponType::Shotgun => WeaponStats::shotgun(),
        WeaponType::Rifle => WeaponStats::rifle(),
        WeaponType::SniperRifle => WeaponStats::sniper_rifle(),
        WeaponType::RPG => WeaponStats::rpg(),
    }
}

// ============================================================================
// 自動存檔觸發系統
// ============================================================================

/// 安全屋自動存檔追蹤（防止反覆觸發）
#[derive(Resource, Default)]
pub struct SafehouseAutoSaveTracker {
    /// 上次觸發存檔的安全屋 ID
    pub last_safehouse_id: Option<String>,
    /// 上次存檔時間
    pub last_save_time: f32,
}

/// 安全屋自動存檔冷卻時間（秒）
pub(crate) const SAFEHOUSE_SAVE_COOLDOWN: f32 = 30.0;
/// 安全屋觸發距離的平方（5m）
pub(crate) const SAFEHOUSE_TRIGGER_DISTANCE_SQ: f32 = 25.0;
/// 重要購買的最低金額門檻
pub(crate) const IMPORTANT_PURCHASE_THRESHOLD: i32 = 1000;

/// 任務完成時觸發自動存檔
pub fn mission_complete_auto_save_system(
    mut mission_events: MessageReader<StoryMissionEvent>,
    mut auto_save_events: MessageWriter<AutoSaveEvent>,
) {
    for event in mission_events.read() {
        if matches!(event, StoryMissionEvent::Completed { .. }) {
            auto_save_events.write(AutoSaveEvent {
                reason: AutoSaveReason::MissionComplete,
            });
            info!("💾 任務完成，觸發自動存檔");
        }
    }
}

/// 進入安全屋時觸發自動存檔
pub fn safehouse_auto_save_system(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    safehouse_query: Query<(&Transform, &Safehouse)>,
    mut auto_save_events: MessageWriter<AutoSaveEvent>,
    mut tracker: ResMut<SafehouseAutoSaveTracker>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let current_time = time.elapsed_secs();

    for (safehouse_transform, safehouse) in &safehouse_query {
        if !safehouse.is_unlocked {
            continue;
        }

        let distance_sq = player_pos.distance_squared(safehouse_transform.translation);
        if distance_sq > SAFEHOUSE_TRIGGER_DISTANCE_SQ {
            continue;
        }

        // 冷卻檢查：同一安全屋在冷卻期內不重複觸發
        let should_save = match &tracker.last_safehouse_id {
            Some(last_id) if last_id == &safehouse.id => {
                current_time - tracker.last_save_time > SAFEHOUSE_SAVE_COOLDOWN
            }
            _ => true, // 不同安全屋或首次進入
        };

        if should_save {
            tracker.last_safehouse_id = Some(safehouse.id.clone());
            tracker.last_save_time = current_time;
            auto_save_events.write(AutoSaveEvent {
                reason: AutoSaveReason::EnteredSafehouse,
            });
            info!("💾 進入安全屋 {}，觸發自動存檔", safehouse.name);
            break; // 一次只觸發一個
        }
    }
}

/// 購買重要物品時觸發自動存檔
pub fn purchase_auto_save_system(
    mut money_events: MessageReader<MoneyChangedEvent>,
    mut auto_save_events: MessageWriter<AutoSaveEvent>,
) {
    for event in money_events.read() {
        // 只處理購買類型的支出，且金額達到門檻
        if matches!(event.reason, MoneyChangeReason::Purchase)
            && event.amount.abs() >= IMPORTANT_PURCHASE_THRESHOLD
        {
            auto_save_events.write(AutoSaveEvent {
                reason: AutoSaveReason::ImportantPurchase,
            });
            info!("💾 重要購買 (${})，觸發自動存檔", event.amount.abs());
        }
    }
}
