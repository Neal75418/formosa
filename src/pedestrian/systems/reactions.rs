//! 行人反應系統（槍聲反應、車輛碰撞、空間哈希）

#![allow(
    clippy::needless_pass_by_value,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names
)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::combat::{CombatState, Health, WeaponInventory, WeaponType};
use crate::core::{GameState, PedestrianSpatialHash, VehicleSpatialHash};
use crate::pedestrian::components::{
    GunshotTracker, HitByVehicle, PedState, Pedestrian, PedestrianConfig, PedestrianState,
};
use crate::player::Player;
use crate::vehicle::Vehicle;
use crate::wanted::CrimeEvent;

/// 車輛碰撞距離平方 (2.5²)
const VEHICLE_COLLISION_SQ: f32 = 6.25;

/// 射擊記錄距離平方 (1.0²)
const SHOT_RECORD_DISTANCE_SQ: f32 = 1.0;

// ============================================================================
// 反應系統
// ============================================================================

/// 行人對槍聲的反應系統
pub fn pedestrian_reaction_system(
    time: Res<Time>,
    config: Res<PedestrianConfig>,
    gunshot_tracker: Res<GunshotTracker>,
    mut ped_query: Query<(&Transform, &mut PedestrianState), With<Pedestrian>>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    for (transform, mut state) in &mut ped_query {
        let pos = transform.translation;

        // 檢查附近是否有槍聲
        if let Some(shot_pos) =
            gunshot_tracker.has_nearby_shot(pos, config.hearing_range, current_time)
        {
            // 聽到槍聲！
            state.fear_level = (state.fear_level + 0.5).min(1.0);
            state.last_threat_pos = Some(shot_pos);

            if state.fear_level > 0.3 {
                state.state = PedState::Fleeing;
                state.flee_timer = 8.0; // 逃跑 8 秒
            }
        }

        // 更新逃跑狀態
        if state.state == PedState::Fleeing {
            state.flee_timer -= dt;
            if state.flee_timer <= 0.0 {
                // 停止逃跑，恢復行走
                state.state = PedState::Walking;
                state.fear_level = 0.0;
                state.last_threat_pos = None;
            }
        }

        // 恐懼值自然衰減
        if state.state != PedState::Fleeing {
            state.fear_level = (state.fear_level - dt * 0.1).max(0.0);
        }
    }
}

/// 槍擊事件記錄系統
/// 監聽 `CombatState.last_shot_time` 的變化來偵測槍聲
/// 同時計算目擊者數量並發送犯罪事件
pub fn gunshot_tracking_system(
    time: Res<Time>,
    combat_state: Res<CombatState>,
    config: Res<PedestrianConfig>,
    player_query: Query<(&Transform, &WeaponInventory), With<Player>>,
    pedestrian_query: Query<(&Transform, &PedestrianState), With<Pedestrian>>,
    mut tracker: ResMut<GunshotTracker>,
    mut crime_events: MessageWriter<CrimeEvent>,
) {
    let current_time = time.elapsed_secs();

    // 清理過期記錄
    tracker.cleanup(current_time);

    // 檢查是否有新的槍擊
    // last_shot_time 會在射擊時更新
    let Ok((player_transform, inventory)) = player_query.single() else {
        return;
    };

    // 只有遠程武器才會發出槍聲
    if let Some(weapon) = inventory.current_weapon() {
        if weapon.stats.weapon_type == WeaponType::Fist {
            return; // 拳頭不會發出槍聲
        }
    } else {
        return;
    }

    // 檢查是否有新的射擊（last_shot_time 在最近 0.1 秒內更新）
    let shot_time_diff = current_time - combat_state.last_shot_time;
    if (0.0..0.1).contains(&shot_time_diff) {
        // 檢查這次射擊是否已經記錄過
        let player_pos = player_transform.translation;
        let already_recorded = tracker.recent_shots.iter().any(|(pos, t)| {
            (*t - combat_state.last_shot_time).abs() < 0.05
                && pos.distance_squared(player_pos) < SHOT_RECORD_DISTANCE_SQ
        });

        if !already_recorded {
            tracker.record_shot(player_pos, combat_state.last_shot_time);

            // 計算目擊者數量（聽到槍聲的行人）- 使用 distance_squared 避免 sqrt
            let hearing_range_sq = config.hearing_range * config.hearing_range;
            let witness_count = pedestrian_query
                .iter()
                .filter(|(ped_transform, state)| {
                    let distance_sq = ped_transform.translation.distance_squared(player_pos);
                    // 在聽力範圍內且沒有在逃跑的行人
                    distance_sq < hearing_range_sq && state.state != PedState::Fleeing
                })
                .count() as u32;

            // 發送犯罪事件
            crime_events.write(CrimeEvent::Shooting {
                position: player_pos,
                witness_count,
            });
        }
    }
}

// ============================================================================
// 車輛空間哈希系統
// ============================================================================

/// 更新車輛空間哈希（每幀執行，在碰撞檢測前）
///
/// 將場景中所有車輛位置插入空間哈希網格，
/// 供行人碰撞檢測和其他系統使用。
pub fn update_vehicle_spatial_hash_system(
    mut vehicle_hash: ResMut<VehicleSpatialHash>,
    vehicle_query: Query<(Entity, &Transform, &Velocity), With<Vehicle>>,
) {
    // 清空舊資料
    vehicle_hash.clear();

    // 插入所有車輛（批量插入效能更好）
    vehicle_hash.insert_batch(
        vehicle_query
            .iter()
            .map(|(entity, transform, _)| (entity, transform.translation)),
    );
}

/// 更新行人空間哈希（每幀執行，在恐慌傳播前）
///
/// 將場景中所有行人位置插入空間哈希網格，
/// 供恐慌波傳播和其他系統使用，將 O(n²) 降為 O(n)。
pub fn update_pedestrian_spatial_hash_system(
    mut ped_hash: ResMut<PedestrianSpatialHash>,
    ped_query: Query<(Entity, &Transform), With<Pedestrian>>,
) {
    // 清空舊資料
    ped_hash.clear();

    // 插入所有行人（批量插入效能更好）
    ped_hash.insert_batch(
        ped_query
            .iter()
            .map(|(entity, transform)| (entity, transform.translation)),
    );
}

// ============================================================================
// 車輛碰撞系統（使用空間哈希優化）
// ============================================================================

// ============================================================================
// 車輛碰撞輔助函數
// ============================================================================
/// 處理行人被車輛撞擊
fn apply_vehicle_hit(
    commands: &mut Commands,
    ped_entity: Entity,
    state: &mut PedestrianState,
    ped_pos: Vec3,
    vehicle_pos: Vec3,
    speed: f32,
    current_time: f32,
) {
    let impact_dir = (ped_pos - vehicle_pos).normalize_or_zero();

    commands.entity(ped_entity).insert(HitByVehicle {
        impact_direction: impact_dir,
        impact_force: speed * 50.0,
        hit_time: current_time,
    });

    state.fear_level = 1.0;
    state.state = PedState::Fleeing;
    state.flee_timer = 10.0;
    state.last_threat_pos = Some(vehicle_pos);
}

/// 發送車輛撞人犯罪事件
fn send_vehicle_hit_crime(
    crime_events: &mut MessageWriter<CrimeEvent>,
    ped_entity: Entity,
    ped_pos: Vec3,
    speed: f32,
    health: &Health,
) {
    let fatal = speed > 15.0 || health.current < speed * 5.0;
    crime_events.write(CrimeEvent::VehicleHit {
        victim: ped_entity,
        position: ped_pos,
        fatal,
    });
}

/// 車輛碰撞偵測系統（O(n) 優化版）
///
/// 使用空間哈希將 O(行人×車輛) 降為 O(行人)。
/// 每個行人只檢查其所在網格及相鄰網格內的車輛。
pub fn pedestrian_vehicle_collision_system(
    mut commands: Commands,
    time: Res<Time>,
    game_state: Res<GameState>,
    vehicle_hash: Res<VehicleSpatialHash>,
    vehicle_velocity_query: Query<&Velocity, With<Vehicle>>,
    mut ped_query: Query<
        (Entity, &Transform, &mut PedestrianState, &Health),
        (With<Pedestrian>, Without<HitByVehicle>),
    >,
    mut crime_events: MessageWriter<CrimeEvent>,
) {
    const QUERY_RADIUS: f32 = 3.0;
    const MIN_HIT_SPEED: f32 = 3.0;
    let current_time = time.elapsed_secs();
    let player_vehicle = game_state.current_vehicle;

    for (ped_entity, ped_transform, mut state, health) in &mut ped_query {
        let ped_pos = ped_transform.translation;

        for (vehicle_entity, vehicle_pos, dist_sq) in
            vehicle_hash.query_radius(ped_pos, QUERY_RADIUS)
        {
            if dist_sq >= VEHICLE_COLLISION_SQ {
                continue;
            }

            let Ok(velocity) = vehicle_velocity_query.get(vehicle_entity) else {
                continue;
            };
            let speed = velocity.linvel.length();

            if speed <= MIN_HIT_SPEED {
                continue;
            }

            apply_vehicle_hit(
                &mut commands,
                ped_entity,
                &mut state,
                ped_pos,
                vehicle_pos,
                speed,
                current_time,
            );

            if Some(vehicle_entity) == player_vehicle {
                send_vehicle_hit_crime(&mut crime_events, ped_entity, ped_pos, speed, health);
            }

            break;
        }
    }
}

/// 處理被車撞的行人
pub fn pedestrian_hit_response_system(
    mut commands: Commands,
    time: Res<Time>,
    mut ped_query: Query<(Entity, &mut Transform, &HitByVehicle), With<Pedestrian>>,
) {
    let current_time = time.elapsed_secs();

    for (entity, mut transform, hit) in &mut ped_query {
        let time_since_hit = current_time - hit.hit_time;

        // 被撞後的飛行效果（持續 1 秒）
        if time_since_hit < 1.0 {
            // 根據撞擊力計算位移
            let displacement =
                hit.impact_direction * hit.impact_force * 0.01 * (1.0 - time_since_hit);
            transform.translation += displacement * time.delta_secs() * 10.0;

            // 添加一些上升效果
            if time_since_hit < 0.3 {
                transform.translation.y += 2.0 * time.delta_secs();
            } else {
                // 下降
                transform.translation.y -= 5.0 * time.delta_secs();
            }

            // 旋轉效果
            let spin = Quat::from_rotation_x(time_since_hit * 3.0);
            transform.rotation = spin;
        } else {
            // 移除 HitByVehicle 組件
            commands.entity(entity).remove::<HitByVehicle>();
            // 確保在地面上
            if transform.translation.y < 0.85 {
                transform.translation.y = 0.85;
            }
            transform.rotation = Quat::IDENTITY;
        }
    }
}
