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

use crate::core::{PlayerStats, WeatherState, WorldTime};
use crate::economy::PlayerWallet;
use crate::player::Player;
use crate::combat::{Health, Armor, WeaponInventory};
use crate::mission::StoryMissionManager;
use crate::vehicle::{VehicleModifications, ModLevel, NitroBoost};

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
    player_stats: Res<PlayerStats>,
    // 車輛改裝查詢
    vehicle_mod_query: Query<(Entity, &VehicleModifications)>,
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
        let task = task_pool.spawn(async move {
            perform_save_async(json, path).await
        });
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
    _player_stats: &PlayerStats,
    vehicle_mod_query: &Query<(Entity, &VehicleModifications)>,
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

    // 車輛改裝資料
    save_data.world.vehicle_modifications = vehicle_mod_query
        .iter()
        .enumerate()
        .map(|(index, (_entity, mods))| VehicleModificationSaveData {
            vehicle_index: index as u32,
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
        let task = task_pool.spawn(async move {
            perform_load_async(path).await
        });
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
    mut player_query: Query<(&mut Transform, &mut Player, Option<&mut Health>, Option<&mut Armor>)>,
    mut wallet: ResMut<PlayerWallet>,
    mut world_time: ResMut<WorldTime>,
    mut weather_state: ResMut<WeatherState>,
    mut vehicle_mod_query: Query<(Entity, &mut VehicleModifications, Option<&NitroBoost>)>,
) {
    if let Some(save_data) = task_tracker.pending_load_data.take() {
        apply_save_data(
            &mut commands,
            &save_data,
            &mut player_query,
            &mut wallet,
            &mut world_time,
            &mut weather_state,
            &mut vehicle_mod_query,
        );
        save_manager.is_busy = false;
        info!("存檔資料已套用");
    }
}

/// 非同步執行讀檔 IO
async fn perform_load_async(path: PathBuf) -> Result<SaveData, SaveError> {
    // 讀取檔案
    let json = std::fs::read_to_string(&path)
        .map_err(|e| SaveError::IoError(e.to_string()))?;

    // 反序列化
    let save_data: SaveData = serde_json::from_str(&json)
        .map_err(|e| SaveError::DeserializeError(e.to_string()))?;

    // 版本檢查
    if save_data.version != SAVE_VERSION {
        warn!(
            "存檔版本不匹配: 存檔={}, 當前={}",
            save_data.version, SAVE_VERSION
        );
    }

    Ok(save_data)
}

/// 套用存檔資料到遊戲狀態
fn apply_save_data(
    commands: &mut Commands,
    save_data: &SaveData,
    player_query: &mut Query<(&mut Transform, &mut Player, Option<&mut Health>, Option<&mut Armor>)>,
    wallet: &mut PlayerWallet,
    world_time: &mut WorldTime,
    weather_state: &mut WeatherState,
    vehicle_mod_query: &mut Query<(Entity, &mut VehicleModifications, Option<&NitroBoost>)>,
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

    // 世界資料
    world_time.hour = save_data.world.world_hour;
    // TODO: 解析天氣類型字串並設定

    // 天氣強度
    weather_state.intensity = save_data.world.weather_intensity;

    // 車輛改裝資料
    let vehicles: Vec<_> = vehicle_mod_query.iter_mut().collect();
    for (index, (entity, mut mods, nitro)) in vehicles.into_iter().enumerate() {
        // 嘗試從存檔中找到對應的改裝資料
        if let Some(saved_mods) = save_data
            .world
            .vehicle_modifications
            .iter()
            .find(|m| m.vehicle_index == index as u32)
        {
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
                "恢復車輛 {} 改裝: 引擎={:?}, 氮氣={}",
                index, mods.engine, mods.has_nitro
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
}
