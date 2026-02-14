//! 存檔讀取邏輯
//!
//! 從 systems.rs 拆分，處理讀檔、解析、套用存檔資料。

#![allow(dead_code)]

use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use futures_lite::future;
use std::path::PathBuf;

use crate::combat::{Armor, Health, Weapon, WeaponInventory, WeaponStats};
use crate::core::{GameState, WeatherState, WorldTime};
use crate::economy::PlayerWallet;
use crate::environment::DestroyedObjectTracker;
use crate::mission::{
    RelationshipManager, RespectManager, StoryMissionManager, UnlockManager,
};
use crate::player::Player;
use crate::vehicle::{NitroBoost, Vehicle, VehicleId, VehicleModifications};

use super::components::*;
use super::systems::{
    u8_to_mod_level, validate_save_data, weapon_stats_from_type, SaveError, SaveTaskTracker,
};

// ============================================================================
// 讀檔處理
// ============================================================================

/// 處理讀檔事件（非同步版本）
pub fn handle_load_events(
    mut events: MessageReader<LoadGameEvent>,
    mut save_manager: ResMut<SaveManager>,
    mut task_tracker: ResMut<SaveTaskTracker>,
) {
    // 檢查是否已有存檔或讀檔任務在執行（互斥）
    if task_tracker.save_task.is_some() || task_tracker.load_task.is_some() {
        return;
    }

    if let Some(event) = events.read().next() {
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

        info!("💾 讀檔任務已啟動: {:?}", load_path);
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
                    info!("💾 讀檔完成，等待套用資料");
                    task_tracker.pending_load_data = Some(save_data);
                }
                Err(e) => {
                    error!("讀檔失敗: {:?}", e);
                    if task_tracker.save_task.is_none() {
                        save_manager.is_busy = false;
                    }
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
    mut game_state: ResMut<GameState>,
    mut vehicle_query: Query<&mut Vehicle>,
    mut pending_destruction: ResMut<PendingDestructionRestore>,
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
            &mut game_state,
            &mut vehicle_query,
        );

        // 破壞持久化：將 ID 放入緩衝，由下一個系統處理
        if !save_data.world.destroyed_object_ids.is_empty() {
            pending_destruction.ids = Some(save_data.world.destroyed_object_ids.clone());
        }

        if task_tracker.save_task.is_none() {
            save_manager.is_busy = false;
        }
        info!("💾 存檔資料已套用");
    }
}

/// 待恢復的破壞資料（一次性緩衝，由 apply_pending_load_data 填入，
/// 由 apply_pending_destruction_data 處理後清除）
#[derive(Resource, Default)]
pub struct PendingDestructionRestore {
    pub ids: Option<Vec<u32>>,
}

/// 套用破壞持久化資料（獨立系統，在 apply_pending_load_data 之後執行）
pub fn apply_pending_destruction_data(
    mut commands: Commands,
    mut pending: ResMut<PendingDestructionRestore>,
    mut destroyed_tracker: ResMut<DestroyedObjectTracker>,
    destructible_query: Query<(Entity, &crate::environment::DestructibleId)>,
) {
    let Some(ids) = pending.ids.take() else {
        return;
    };

    apply_destruction_data(&mut commands, &mut destroyed_tracker, &destructible_query, &ids);
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
pub fn apply_save_data(
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
    game_state: &mut GameState,
    vehicle_query: &mut Query<&mut Vehicle>,
) {
    apply_player_data(player_query, wallet, respect, save_data);
    apply_weapon_data(weapon_query, save_data);
    apply_world_data(world_time, weather_state, save_data);
    apply_mission_data(story_manager, unlocks, relationship, save_data);
    apply_vehicle_modifications(commands, vehicle_mod_query, save_data);
    apply_vehicle_state(game_state, vehicle_mod_query, vehicle_query, save_data);
}

fn apply_player_data(
    player_query: &mut Query<(
        &mut Transform,
        &mut Player,
        Option<&mut Health>,
        Option<&mut Armor>,
    )>,
    wallet: &mut PlayerWallet,
    respect: &mut RespectManager,
    save_data: &SaveData,
) {
    if let Ok((mut transform, _, health, armor)) = player_query.single_mut() {
        let pos = save_data.player.position;
        transform.translation = Vec3::new(pos[0], pos[1], pos[2]);
        transform.rotation = Quat::from_rotation_y(save_data.player.rotation_y);

        if let Some(mut h) = health {
            h.current = save_data.player.health;
            h.max = save_data.player.max_health;
        }
        if let Some(mut a) = armor {
            a.current = save_data.player.armor;
        }
    }

    wallet.cash = save_data.player.cash;
    wallet.bank = save_data.player.bank;
    wallet.total_earned = save_data.stats.total_money_earned;
    wallet.total_spent = save_data.stats.total_money_spent;
    respect.respect = save_data.player.respect;
}

fn apply_weapon_data(
    weapon_query: &mut Query<&mut WeaponInventory, With<Player>>,
    save_data: &SaveData,
) {
    if let Ok(mut inventory) = weapon_query.single_mut() {
        inventory.weapons.clear();
        for saved_weapon in &save_data.player.weapons {
            let stats = weapon_stats_from_type(saved_weapon.weapon_type);
            let mut weapon = Weapon::new(stats);
            weapon.current_ammo = saved_weapon.current_ammo;
            weapon.reserve_ammo = saved_weapon.reserve_ammo;
            inventory.weapons.push(weapon);
        }

        if inventory.weapons.is_empty() {
            inventory.weapons.push(Weapon::new(WeaponStats::fist()));
        }

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
}

fn apply_world_data(
    world_time: &mut WorldTime,
    weather_state: &mut WeatherState,
    save_data: &SaveData,
) {
    world_time.hour = save_data.world.world_hour;
    weather_state.weather_type = save_data.world.weather;
    weather_state.intensity = save_data.world.weather_intensity.clamp(0.0, 2.0);
    info!(
        "還原天氣: {:?}, 強度: {}",
        weather_state.weather_type, weather_state.intensity
    );
}

fn apply_mission_data(
    story_manager: &mut StoryMissionManager,
    unlocks: &mut UnlockManager,
    relationship: &mut RelationshipManager,
    save_data: &SaveData,
) {
    // 還原 mission_states（v2 優先，v1 向後相容）
    if !save_data.missions.mission_states.is_empty() {
        story_manager.mission_states = save_data.missions.mission_states.clone();
    } else {
        // v1 向後相容：從 completed_missions 字串重建
        story_manager.mission_states.clear();
        for id_str in &save_data.missions.completed_missions {
            if let Ok(id) = id_str.parse::<u32>() {
                story_manager
                    .mission_states
                    .insert(id, crate::mission::StoryMissionStatus::Completed);
            }
        }
    }

    // 從實際狀態計算 completed_count（取代直接用 len()）
    story_manager.completed_count = story_manager
        .mission_states
        .values()
        .filter(|s| **s == crate::mission::StoryMissionStatus::Completed)
        .count() as u32;

    // 還原 chapter、ratings、play_time
    if save_data.missions.current_chapter > 0 {
        story_manager.current_chapter = save_data.missions.current_chapter;
    }
    if !save_data.missions.best_ratings.is_empty() {
        // v2 格式優先
        story_manager.mission_ratings = save_data.missions.best_ratings.clone();
    } else if !save_data.missions.mission_ratings.is_empty() {
        // v1 fallback：從 Vec<(String, u8)> 轉換為 HashMap<u32, StoryMissionRating>
        story_manager.mission_ratings.clear();
        for (id_str, stars) in &save_data.missions.mission_ratings {
            if let Ok(id) = id_str.parse::<u32>() {
                story_manager
                    .mission_ratings
                    .insert(id, crate::mission::StoryMissionRating::from_stars(*stars));
            }
        }
    }
    story_manager.total_play_time = save_data.play_time_secs as f32;

    // 還原 unlocks
    unlocks.unlocked_items = save_data.missions.unlocked_items.iter().cloned().collect();
    unlocks.unlocked_areas = save_data
        .missions
        .unlocked_areas
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    // 還原 NPC 關係
    relationship.relationships.clear();
    for (npc_str, value) in &save_data.missions.npc_relationships {
        if let Ok(npc_id) = npc_str.parse::<u32>() {
            relationship.relationships.insert(npc_id, *value);
        }
    }

    // 還原劇情旗標
    story_manager.story_flags = save_data.missions.flags.iter().cloned().collect();
    info!(
        "還原任務進度: {} 個已完成、{} 個追蹤中、章節 {}",
        story_manager.completed_count,
        story_manager.mission_states.len(),
        story_manager.current_chapter,
    );
}

fn apply_vehicle_modifications(
    commands: &mut Commands,
    vehicle_mod_query: &mut Query<(
        Entity,
        &VehicleId,
        &mut VehicleModifications,
        Option<&NitroBoost>,
    )>,
    save_data: &SaveData,
) {
    let vehicles: Vec<_> = vehicle_mod_query.iter_mut().collect();
    for (entity, vehicle_id, mut mods, nitro) in vehicles.into_iter() {
        #[allow(deprecated)]
        let saved_mods = save_data
            .world
            .vehicle_modifications
            .iter()
            .find(|m| m.vehicle_id == vehicle_id.as_u64());

        if let Some(saved_mods) = saved_mods {
            mods.engine = u8_to_mod_level(saved_mods.engine_level);
            mods.transmission = u8_to_mod_level(saved_mods.transmission_level);
            mods.suspension = u8_to_mod_level(saved_mods.suspension_level);
            mods.brakes = u8_to_mod_level(saved_mods.brakes_level);
            mods.tires = u8_to_mod_level(saved_mods.tires_level);
            mods.armor = u8_to_mod_level(saved_mods.armor_level);
            mods.has_nitro = saved_mods.has_nitro;
            mods.nitro_charge = saved_mods.nitro_charge;

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

/// 還原玩家車內狀態
fn apply_vehicle_state(
    game_state: &mut GameState,
    vehicle_mod_query: &mut Query<(
        Entity,
        &VehicleId,
        &mut VehicleModifications,
        Option<&NitroBoost>,
    )>,
    vehicle_query: &mut Query<&mut Vehicle>,
    save_data: &SaveData,
) {
    if save_data.player.in_vehicle {
        if let Some(target_id) = save_data.player.current_vehicle_id {
            let found = vehicle_mod_query
                .iter()
                .find(|(_, vid, _, _)| vid.as_u64() == target_id)
                .map(|(entity, _, _, _)| entity);

            if let Some(entity) = found {
                game_state.player_in_vehicle = true;
                game_state.current_vehicle = Some(entity);

                // 同步設定車輛為已佔用
                if let Ok(mut vehicle) = vehicle_query.get_mut(entity) {
                    vehicle.is_occupied = true;
                } else {
                    warn!("無法設定車輛 {:?} 為已佔用狀態", entity);
                }

                info!("還原車內狀態: 車輛 ID={}", target_id);
            } else {
                warn!(
                    "存檔中的車輛 ID {} 不存在，玩家將以步行狀態還原",
                    target_id
                );
                game_state.player_in_vehicle = false;
                game_state.current_vehicle = None;
            }
        } else {
            // 防禦性處理：in_vehicle=true 但無車輛 ID
            warn!("存檔狀態不一致：in_vehicle=true 但 current_vehicle_id=None，回退為步行");
            game_state.player_in_vehicle = false;
            game_state.current_vehicle = None;
        }
    } else {
        game_state.player_in_vehicle = false;
        game_state.current_vehicle = None;
    }
}

fn apply_destruction_data(
    commands: &mut Commands,
    destroyed_tracker: &mut DestroyedObjectTracker,
    destructible_query: &Query<(Entity, &crate::environment::DestructibleId)>,
    destroyed_ids: &[u32],
) {
    // 從存檔恢復已破壞物件 ID
    destroyed_tracker.restore_from(destroyed_ids);

    // 移除場景中已破壞的物件
    let mut count = 0u32;
    for (entity, id) in destructible_query.iter() {
        if destroyed_tracker.is_destroyed(id.0) {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                count += 1;
            }
        }
    }

    info!(
        "還原破壞狀態: {} 個已破壞（存檔記錄 {} 個）",
        count,
        destroyed_ids.len()
    );
}
