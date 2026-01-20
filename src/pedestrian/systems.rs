//! 行人系統
//!
//! 處理行人的生成、移動、反應等邏輯。

#![allow(dead_code)] // Phase 5+ 預留功能

use bevy::prelude::*;
use bevy::ecs::relationship::Relationship;
use bevy_rapier3d::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::ai::{PatrolPath, AiMovement};
use crate::player::Player;
use crate::core::{COLLISION_GROUP_CHARACTER, VehicleSpatialHash, PedestrianSpatialHash, GameState, WeatherState, WeatherType};
use crate::combat::{CombatState, WeaponInventory, WeaponType, Health, Damageable, HitReaction};
use crate::vehicle::Vehicle;
use crate::wanted::{CrimeEvent, WitnessReport};
use super::components::{
    Pedestrian, PedestrianState, PedState, PedestrianType,
    PedestrianConfig, PedestrianPaths, SidewalkPath, GunshotTracker,
    PedestrianVisuals, WalkingAnimation, PedestrianLeg, PedestrianArm,
    HitByVehicle, PathfindingGrid, AStarPath, DailyBehavior, BehaviorType,
    PointsOfInterest, PointOfInterestType,
    WitnessState, WitnessedCrime, ShelterSeeker,
    PanicWaveManager, PanicState, PanicWave,
};

// ============================================================================
// 躲雨行為常數
// ============================================================================

/// 躲雨機率係數（雨量強度 * 此值 = 每幀躲雨機率）
const SHELTER_SEEK_PROBABILITY_FACTOR: f32 = 0.02;
/// 庇護點搜索半徑
const SHELTER_SEARCH_RADIUS: f32 = 80.0;
/// 商店櫥窗搜索半徑（備用庇護）
const SHOP_FALLBACK_SEARCH_RADIUS: f32 = 50.0;
/// 到達庇護點的判定距離
const SHELTER_ARRIVAL_DISTANCE: f32 = 2.0;

// === 距離平方常數 (效能優化：避免 sqrt) ===
/// 最小生成距離平方 (15.0²)
const MIN_SPAWN_DISTANCE_SQ: f32 = 225.0;
/// 車輛碰撞距離平方 (2.5²)
const VEHICLE_COLLISION_SQ: f32 = 6.25;
/// 庇護點到達距離平方 (2.0²)
const SHELTER_ARRIVAL_SQ: f32 = 4.0;
/// 目擊者 UI 顯示距離平方 (30.0²)
const WITNESS_UI_DISTANCE_SQ: f32 = 900.0;
/// 射擊記錄距離平方 (1.0²)
const SHOT_RECORD_DISTANCE_SQ: f32 = 1.0;

// ============================================================================
// 設置系統
// ============================================================================

/// 初始化行人視覺資源
pub fn setup_pedestrian_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(PedestrianVisuals::new(&mut meshes, &mut materials));
}

/// 初始化行人路徑
pub fn setup_pedestrian_paths(mut commands: Commands) {
    let paths = PedestrianPaths {
        sidewalk_paths: vec![
            // 漢中街徒步區（南北向）- 東側
            SidewalkPath::new(
                "漢中街東",
                vec![
                    Vec3::new(8.0, 0.25, -55.0),
                    Vec3::new(8.0, 0.25, -35.0),
                    Vec3::new(8.0, 0.25, -15.0),
                    Vec3::new(8.0, 0.25, 5.0),
                    Vec3::new(8.0, 0.25, 25.0),
                    Vec3::new(8.0, 0.25, 40.0),
                ],
                true, // 往返
            ),
            // 漢中街徒步區（南北向）- 西側
            SidewalkPath::new(
                "漢中街西",
                vec![
                    Vec3::new(-8.0, 0.25, 40.0),
                    Vec3::new(-8.0, 0.25, 20.0),
                    Vec3::new(-8.0, 0.25, 0.0),
                    Vec3::new(-8.0, 0.25, -20.0),
                    Vec3::new(-8.0, 0.25, -40.0),
                    Vec3::new(-8.0, 0.25, -55.0),
                ],
                true,
            ),
            // 峨嵋街徒步區（東西向）- 北側
            SidewalkPath::new(
                "峨嵋街北",
                vec![
                    Vec3::new(-35.0, 0.25, 8.0),
                    Vec3::new(-15.0, 0.25, 8.0),
                    Vec3::new(5.0, 0.25, 8.0),
                    Vec3::new(25.0, 0.25, 8.0),
                ],
                true,
            ),
            // 峨嵋街徒步區（東西向）- 南側
            SidewalkPath::new(
                "峨嵋街南",
                vec![
                    Vec3::new(25.0, 0.25, -8.0),
                    Vec3::new(10.0, 0.25, -8.0),
                    Vec3::new(-10.0, 0.25, -8.0),
                    Vec3::new(-30.0, 0.25, -8.0),
                ],
                true,
            ),
            // 西寧南路人行道（西側）
            SidewalkPath::new(
                "西寧南路",
                vec![
                    Vec3::new(-57.0, 0.25, -50.0),
                    Vec3::new(-57.0, 0.25, -30.0),
                    Vec3::new(-57.0, 0.25, -10.0),
                    Vec3::new(-57.0, 0.25, 10.0),
                    Vec3::new(-57.0, 0.25, 30.0),
                ],
                true,
            ),
            // 武昌街人行道
            SidewalkPath::new(
                "武昌街",
                vec![
                    Vec3::new(-40.0, 0.25, -45.0),
                    Vec3::new(-20.0, 0.25, -45.0),
                    Vec3::new(0.0, 0.25, -45.0),
                    Vec3::new(20.0, 0.25, -45.0),
                    Vec3::new(40.0, 0.25, -45.0),
                ],
                true,
            ),
            // 成都路人行道
            SidewalkPath::new(
                "成都路",
                vec![
                    Vec3::new(40.0, 0.25, 45.0),
                    Vec3::new(20.0, 0.25, 45.0),
                    Vec3::new(0.0, 0.25, 45.0),
                    Vec3::new(-20.0, 0.25, 45.0),
                    Vec3::new(-40.0, 0.25, 45.0),
                ],
                true,
            ),
        ],
    };
    commands.insert_resource(paths);
    commands.insert_resource(GunshotTracker::default());
}

// ============================================================================
// 生成系統
// ============================================================================

/// 行人生成系統
pub fn pedestrian_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut config: ResMut<PedestrianConfig>,
    paths: Res<PedestrianPaths>,
    visuals: Option<Res<PedestrianVisuals>>,
    player_query: Query<&Transform, With<Player>>,
    ped_query: Query<Entity, With<Pedestrian>>,
) {
    // 等待視覺資源初始化
    let Some(visuals) = visuals else { return };

    // 更新計時器
    config.spawn_timer += time.delta_secs();
    if config.spawn_timer < config.spawn_interval {
        return;
    }
    config.spawn_timer = 0.0;

    // 檢查數量上限
    let current_count = ped_query.iter().count();
    if current_count >= config.max_count {
        return;
    }

    // 取得玩家位置
    let Ok(player_transform) = player_query.single() else { return };
    let player_pos = player_transform.translation;

    // 在玩家附近的路徑上選擇生成點
    if paths.sidewalk_paths.is_empty() {
        return;
    }

    let mut rng = rand::rng();

    // 隨機選擇一條路徑
    let path_idx = rng.random_range(0..paths.sidewalk_paths.len());
    let path = &paths.sidewalk_paths[path_idx];

    if path.waypoints.is_empty() {
        return;
    }

    // 隨機選擇路徑上的一個起點
    let start_idx = rng.random_range(0..path.waypoints.len());
    let spawn_pos = path.waypoints[start_idx];

    // 檢查是否在玩家的生成範圍內 (使用 distance_squared 避免 sqrt)
    let dist_to_player_sq = spawn_pos.distance_squared(player_pos);
    let spawn_radius_sq = config.spawn_radius * config.spawn_radius;
    if dist_to_player_sq > spawn_radius_sq || dist_to_player_sq < MIN_SPAWN_DISTANCE_SQ {
        // 太遠或太近都不生成
        return;
    }

    // 生成行人
    spawn_pedestrian(
        &mut commands,
        spawn_pos,
        path.waypoints.clone(),
        start_idx,
        path.ping_pong,
        &config,
        &visuals,
    );
}

/// 生成單個行人
fn spawn_pedestrian(
    commands: &mut Commands,
    position: Vec3,
    waypoints: Vec<Vec3>,
    start_index: usize,
    ping_pong: bool,
    config: &PedestrianConfig,
    visuals: &PedestrianVisuals,
) {
    use rand::Rng;
    let mut rng = rand::rng();

    // 隨機選擇行人類型
    let ped_type = if rng.random_bool(0.3) {
        PedestrianType::Business
    } else {
        PedestrianType::Casual
    };

    // 使用預創建的材質（隨機選擇索引）
    let indices = visuals.random_indices();

    // 人體尺寸常數
    let body_height = 1.7;
    let head_radius = 0.12;
    let torso_height = 0.5;
    let leg_height = 0.45;

    // 計算朝向
    let look_dir = if start_index + 1 < waypoints.len() {
        (waypoints[start_index + 1] - position).normalize_or_zero()
    } else if start_index > 0 {
        (position - waypoints[start_index - 1]).normalize_or_zero()
    } else {
        Vec3::NEG_Z
    };
    let rotation = if look_dir.length_squared() > 0.001 {
        Quat::from_rotation_y((-look_dir.x).atan2(-look_dir.z))
    } else {
        Quat::IDENTITY
    };

    // 生成行人實體（使用單一 spawn 搭配 with_children 避免 B0004 警告）
    // 注意：Bevy Bundle 限制為 15 個組件，因此分成多個 insert 調用
    commands.spawn((
        // 根實體 Transform 組件
        Transform::from_translation(position + Vec3::new(0.0, body_height / 2.0, 0.0))
            .with_rotation(rotation),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        // 行人組件
        Pedestrian { ped_type },
        PedestrianState::default(),
        WalkingAnimation::default(),
        WitnessState::default(),  // 目擊者狀態（GTA 5 風格報警系統）
        ShelterSeeker::default(), // 躲雨行為追蹤
        PanicState::default(),    // 恐慌狀態（GTA 5 風格群體恐慌）
        // 可受傷組件
        Health::default(),
        Damageable,
        HitReaction::default(),  // 受傷反應（畏縮、踉蹌、擊退）
    )).insert((
        // 移動組件（分開 insert 以符合 Bundle 大小限制）
        AiMovement {
            walk_speed: config.walk_speed,
            run_speed: config.flee_speed,
            is_running: false,
            arrival_threshold: 0.8,
            move_target: None,
        },
        PatrolPath {
            waypoints: waypoints.clone(),
            // 目標是下一個 waypoint（不是當前位置）
            current_index: if start_index + 1 < waypoints.len() { start_index + 1 } else { 0 },
            ping_pong,
            forward: true,
            wait_time: 0.0,
            wait_timer: 0.0,
        },
    )).insert((
        // 物理組件（分開 insert 以符合 Bundle 大小限制）
        RigidBody::KinematicPositionBased,
        Collider::capsule_y(body_height / 2.0 - 0.2, 0.25),
        CollisionGroups::new(COLLISION_GROUP_CHARACTER, Group::ALL),
        KinematicCharacterController {
            offset: CharacterLength::Absolute(0.01),
            ..default()
        },
    )).with_children(|parent| {
        // 頭部
        parent.spawn((
            Mesh3d(visuals.head_mesh.clone()),
            MeshMaterial3d(visuals.skin_materials[indices.skin].clone()),
            Transform::from_xyz(0.0, torso_height / 2.0 + head_radius + 0.05, 0.0),
        ));
        // 頭髮
        parent.spawn((
            Mesh3d(visuals.hair_mesh.clone()),
            MeshMaterial3d(visuals.hair_materials[indices.hair].clone()),
            Transform::from_xyz(0.0, torso_height / 2.0 + head_radius + 0.08, -0.02)
                .with_scale(Vec3::new(1.0, 0.8, 1.0)),
        ));
        // 軀幹
        parent.spawn((
            Mesh3d(visuals.torso_mesh.clone()),
            MeshMaterial3d(visuals.shirt_materials[indices.shirt].clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        // 左腿（加標記用於動畫）
        parent.spawn((
            Mesh3d(visuals.leg_mesh.clone()),
            MeshMaterial3d(visuals.pants_materials[indices.pants].clone()),
            Transform::from_xyz(-0.08, -torso_height / 2.0 - leg_height / 2.0, 0.0),
            PedestrianLeg { is_left: true },
        ));
        // 右腿（加標記用於動畫）
        parent.spawn((
            Mesh3d(visuals.leg_mesh.clone()),
            MeshMaterial3d(visuals.pants_materials[indices.pants].clone()),
            Transform::from_xyz(0.08, -torso_height / 2.0 - leg_height / 2.0, 0.0),
            PedestrianLeg { is_left: false },
        ));
        // 左腳
        parent.spawn((
            Mesh3d(visuals.shoe_mesh.clone()),
            MeshMaterial3d(visuals.shoe_materials[indices.shoe].clone()),
            Transform::from_xyz(-0.08, -torso_height / 2.0 - leg_height - 0.025, 0.03),
        ));
        // 右腳
        parent.spawn((
            Mesh3d(visuals.shoe_mesh.clone()),
            MeshMaterial3d(visuals.shoe_materials[indices.shoe].clone()),
            Transform::from_xyz(0.08, -torso_height / 2.0 - leg_height - 0.025, 0.03),
        ));
        // 左手臂（加標記用於動畫）
        parent.spawn((
            Mesh3d(visuals.arm_mesh.clone()),
            MeshMaterial3d(visuals.shirt_materials[indices.shirt].clone()),
            Transform::from_xyz(-0.22, torso_height / 4.0, 0.0)
                .with_rotation(Quat::from_rotation_z(0.15)),
            PedestrianArm { is_left: true },
        ));
        // 右手臂（加標記用於動畫）
        parent.spawn((
            Mesh3d(visuals.arm_mesh.clone()),
            MeshMaterial3d(visuals.shirt_materials[indices.shirt].clone()),
            Transform::from_xyz(0.22, torso_height / 4.0, 0.0)
                .with_rotation(Quat::from_rotation_z(-0.15)),
            PedestrianArm { is_left: false },
        ));
    });
}

// ============================================================================
// 移動系統
// ============================================================================

// === 移動輔助函數 ===

/// 根據狀態獲取移動速度
fn get_pedestrian_speed(state: PedState, config: &PedestrianConfig) -> f32 {
    match state {
        PedState::Fleeing => config.flee_speed,
        PedState::Walking => config.walk_speed,
        PedState::Idle | PedState::CallingPolice => 0.0,
    }
}

/// 根據狀態獲取移動目標
fn get_movement_target(
    state: &PedestrianState,
    current_pos: Vec3,
    patrol: &PatrolPath,
) -> Option<Vec3> {
    if state.state == PedState::Fleeing {
        if let Some(threat_pos) = state.last_threat_pos {
            let away_dir = (current_pos - threat_pos).normalize_or_zero();
            let flee_target = current_pos + away_dir * 20.0;
            // 將逃跑目標限制在市區範圍內
            // 地圖邊界：X: -100 ~ 80, Z: -80 ~ 50
            let clamped = Vec3::new(
                flee_target.x.clamp(-95.0, 75.0),  // 留5公尺邊界緩衝
                flee_target.y,
                flee_target.z.clamp(-75.0, 45.0),  // 留5公尺邊界緩衝
            );
            return Some(clamped);
        }
    }
    patrol.current_waypoint()
}

/// 行人移動系統
pub fn pedestrian_movement_system(
    time: Res<Time>,
    config: Res<PedestrianConfig>,
    mut ped_query: Query<(
        &Pedestrian,
        &PedestrianState,
        &mut Transform,
        &mut PatrolPath,
        &mut AiMovement,
        &mut KinematicCharacterController,
    )>,
) {
    let dt = time.delta_secs();

    for (_ped, state, mut transform, mut patrol, movement, mut controller) in ped_query.iter_mut() {
        let speed = get_pedestrian_speed(state.state, &config);
        if speed <= 0.0 {
            continue;
        }

        let current_pos = transform.translation;
        let Some(target_pos) = get_movement_target(state, current_pos, &patrol) else { continue };

        let direction = (target_pos - current_pos).normalize_or_zero();
        let flat_direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();

        if flat_direction.length_squared() < 0.001 {
            continue;
        }

        // 更新朝向
        let target_rotation = Quat::from_rotation_y((-flat_direction.x).atan2(-flat_direction.z));
        transform.rotation = transform.rotation.slerp(target_rotation, dt * 5.0);

        // 移動（加重力）
        let velocity = flat_direction * speed;
        controller.translation = Some(velocity * dt + Vec3::new(0.0, -9.8 * dt, 0.0));

        // 檢查是否到達目標
        let flat_dist_sq = (target_pos.x - current_pos.x).powi(2) + (target_pos.z - current_pos.z).powi(2);
        if flat_dist_sq < movement.arrival_threshold.powi(2) && state.state != PedState::Fleeing {
            patrol.advance();
        }
    }
}

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

    for (transform, mut state) in ped_query.iter_mut() {
        let pos = transform.translation;

        // 檢查附近是否有槍聲
        if let Some(shot_pos) = gunshot_tracker.has_nearby_shot(pos, config.hearing_range, current_time) {
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
/// 監聽 CombatState.last_shot_time 的變化來偵測槍聲
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
    let Ok((player_transform, inventory)) = player_query.single() else { return };

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
    if shot_time_diff >= 0.0 && shot_time_diff < 0.1 {
        // 檢查這次射擊是否已經記錄過
        let player_pos = player_transform.translation;
        let already_recorded = tracker.recent_shots.iter().any(|(pos, t)| {
            (*t - combat_state.last_shot_time).abs() < 0.05 && pos.distance_squared(player_pos) < SHOT_RECORD_DISTANCE_SQ
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
// 清理系統
// ============================================================================

/// 卡住判定的移動閾值（小於此距離視為沒有移動）
const STUCK_MOVEMENT_THRESHOLD_SQ: f32 = 0.25; // 0.5² 公尺
/// 卡住超時時間（秒）
const STUCK_TIMEOUT: f32 = 5.0;

/// 行人消失系統（距離玩家太遠或卡住時移除）
pub fn pedestrian_despawn_system(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<PedestrianConfig>,
    player_query: Query<&Transform, With<Player>>,
    mut ped_query: Query<(Entity, &Transform, &mut PedestrianState), With<Pedestrian>>,
) {
    let Ok(player_transform) = player_query.single() else { return };
    let player_pos = player_transform.translation;
    let dt = time.delta_secs();

    // 地圖邊界常數（與 setup.rs 一致）
    const MAP_MIN_X: f32 = -100.0;  // X_KANGDING
    const MAP_MAX_X: f32 = 80.0;    // X_ZHONGHUA
    const MAP_MIN_Z: f32 = -80.0;   // Z_HANKOU
    const MAP_MAX_Z: f32 = 50.0;    // Z_CHENGDU

    // 使用 distance_squared 避免 sqrt
    let despawn_radius_sq = config.despawn_radius * config.despawn_radius;
    for (entity, transform, mut state) in ped_query.iter_mut() {
        let current_pos = transform.translation;

        // 超出地圖邊界，立即移除
        if current_pos.x < MAP_MIN_X || current_pos.x > MAP_MAX_X
            || current_pos.z < MAP_MIN_Z || current_pos.z > MAP_MAX_Z
        {
            commands.entity(entity).despawn();
            continue;
        }

        let dist_sq = current_pos.distance_squared(player_pos);

        // 距離太遠，移除
        if dist_sq > despawn_radius_sq {
            commands.entity(entity).despawn();
            continue;
        }

        // 卡住檢測：檢查是否在同一位置停留太久
        let movement_sq = current_pos.distance_squared(state.last_recorded_pos);
        if movement_sq < STUCK_MOVEMENT_THRESHOLD_SQ {
            state.stuck_timer += dt;
            if state.stuck_timer > STUCK_TIMEOUT {
                // 卡住太久，移除
                commands.entity(entity).despawn();
                continue;
            }
        } else {
            // 有移動，重置計時器並更新位置
            state.stuck_timer = 0.0;
            state.last_recorded_pos = current_pos;
        }
    }
}

// ============================================================================
// 行走動畫系統
// ============================================================================

// === 動畫輔助函數 ===

/// 計算行人動畫目標速度
fn get_animation_target_speed(state: PedState) -> f32 {
    match state {
        PedState::Fleeing => 12.0,
        PedState::Walking => 6.0,
        PedState::Idle | PedState::CallingPolice => 0.0,
    }
}

/// 更新腿部動畫
fn update_leg_transform(transform: &mut Transform, anim: &WalkingAnimation, is_left: bool) {
    let base_x = if is_left { -0.08 } else { 0.08 };
    let base_y = -0.25 - 0.225; // torso_height/2 + leg_height/2

    if anim.speed > 0.1 {
        let phase_offset = if is_left { 0.0 } else { PI };
        let swing = (anim.phase + phase_offset).sin() * 0.4;

        transform.translation = Vec3::new(base_x, base_y, swing * 0.15);
        transform.rotation = Quat::from_rotation_x(swing);
    } else {
        transform.translation = Vec3::new(base_x, base_y, 0.0);
        transform.rotation = Quat::IDENTITY;
    }
}

/// 更新手臂動畫
fn update_arm_transform(transform: &mut Transform, anim: &WalkingAnimation, is_left: bool) {
    let base_x = if is_left { -0.22 } else { 0.22 };
    let base_z_rot = if is_left { 0.15 } else { -0.15 };

    if anim.speed > 0.1 {
        let phase_offset = if is_left { PI } else { 0.0 };
        let swing = (anim.phase + phase_offset).sin() * 0.3;

        transform.translation = Vec3::new(base_x, 0.125, swing * 0.1);
        transform.rotation = Quat::from_rotation_z(base_z_rot) * Quat::from_rotation_x(swing * 0.5);
    } else {
        transform.translation = Vec3::new(base_x, 0.125, 0.0);
        transform.rotation = Quat::from_rotation_z(base_z_rot);
    }
}

/// 行走動畫更新系統
pub fn pedestrian_walking_animation_system(
    time: Res<Time>,
    mut ped_query: Query<(&PedestrianState, &mut WalkingAnimation), With<Pedestrian>>,
    mut leg_query: Query<(&ChildOf, &PedestrianLeg, &mut Transform)>,
    mut arm_query: Query<(&ChildOf, &PedestrianArm, &mut Transform), Without<PedestrianLeg>>,
) {
    let dt = time.delta_secs();

    // 更新每個行人的動畫相位
    for (state, mut anim) in ped_query.iter_mut() {
        let target_speed = get_animation_target_speed(state.state);

        // 平滑過渡動畫速度
        anim.speed = anim.speed + (target_speed - anim.speed) * dt * 5.0;
        anim.phase += anim.speed * dt;

        // 保持相位在合理範圍
        if anim.phase > PI * 2.0 {
            anim.phase -= PI * 2.0;
        }
    }

    // 更新腿部擺動
    for (parent, leg, mut transform) in leg_query.iter_mut() {
        let Ok((_, anim)) = ped_query.get(parent.get()) else { continue };
        update_leg_transform(&mut transform, anim, leg.is_left);
    }

    // 更新手臂擺動（與腿相反）
    for (parent, arm, mut transform) in arm_query.iter_mut() {
        let Ok((_, anim)) = ped_query.get(parent.get()) else { continue };
        update_arm_transform(&mut transform, anim, arm.is_left);
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
        vehicle_query.iter().map(|(entity, transform, _)| {
            (entity, transform.translation)
        })
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
        ped_query.iter().map(|(entity, transform)| {
            (entity, transform.translation)
        })
    );
}

// ============================================================================
// 車輛碰撞系統（使用空間哈希優化）
// ============================================================================

// === 車輛碰撞輔助函數 ===

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
    let current_time = time.elapsed_secs();
    let player_vehicle = game_state.current_vehicle;
    const QUERY_RADIUS: f32 = 3.0;
    const MIN_HIT_SPEED: f32 = 3.0;

    for (ped_entity, ped_transform, mut state, health) in ped_query.iter_mut() {
        let ped_pos = ped_transform.translation;

        for (vehicle_entity, vehicle_pos, dist_sq) in vehicle_hash.query_radius(ped_pos, QUERY_RADIUS) {
            if dist_sq >= VEHICLE_COLLISION_SQ {
                continue;
            }

            let Ok(velocity) = vehicle_velocity_query.get(vehicle_entity) else { continue };
            let speed = velocity.linvel.length();

            if speed <= MIN_HIT_SPEED {
                continue;
            }

            apply_vehicle_hit(&mut commands, ped_entity, &mut state, ped_pos, vehicle_pos, speed, current_time);

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

    for (entity, mut transform, hit) in ped_query.iter_mut() {
        let time_since_hit = current_time - hit.hit_time;

        // 被撞後的飛行效果（持續 1 秒）
        if time_since_hit < 1.0 {
            // 根據撞擊力計算位移
            let displacement = hit.impact_direction * hit.impact_force * 0.01 * (1.0 - time_since_hit);
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

// ============================================================================
// A* 尋路系統
// ============================================================================

// === 尋路網格輔助函數 ===

/// 將世界座標轉換為網格座標
fn world_to_grid_coords(x: i32, z: i32) -> (usize, usize) {
    (((x + 70) / 2) as usize, ((z + 70) / 2) as usize)
}

/// 將矩形區域標記為不可通行
fn mark_area_unwalkable(grid: &mut PathfindingGrid, x1: i32, z1: i32, x2: i32, z2: i32) {
    for x in x1..x2 {
        for z in z1..z2 {
            let (gx, gz) = world_to_grid_coords(x, z);
            if gx < grid.width && gz < grid.height {
                grid.set_walkable(gx, gz, false);
            }
        }
    }
}

/// 將橫向道路標記為可通行
fn mark_horizontal_road(grid: &mut PathfindingGrid, grid_x_start: usize, grid_x_end: usize) {
    for x in grid_x_start..grid_x_end {
        for z in 0..grid.height {
            grid.set_walkable(x, z, true);
        }
    }
}

/// 將縱向道路標記為可通行
fn mark_vertical_road(grid: &mut PathfindingGrid, grid_z_start: usize, grid_z_end: usize) {
    for x in 0..grid.width {
        for z in grid_z_start..grid_z_end {
            grid.set_walkable(x, z, true);
        }
    }
}

/// 初始化 A* 尋路網格
pub fn setup_pathfinding_grid(mut commands: Commands) {
    let mut grid = PathfindingGrid::default();

    // 建築物區域座標 (世界座標)
    let buildings: &[(i32, i32, i32, i32)] = &[
        // 漢中街兩側建築（中央徒步區）
        (-15, -60, 15, 55),
        // 西側建築
        (-70, -70, -20, -50),
        (-70, -45, -20, -25),
        (-70, -20, -20, 0),
        (-70, 5, -20, 25),
        (-70, 30, -20, 60),
        // 東側建築
        (20, -70, 50, -50),
        (20, -45, 50, -25),
        (20, -20, 50, 0),
        (20, 5, 50, 25),
        (20, 30, 50, 60),
    ];

    // 將建築區域設為不可通行
    for &(x1, z1, x2, z2) in buildings {
        mark_area_unwalkable(&mut grid, x1, z1, x2, z2);
    }

    // 漢中街徒步區 (X: -15 ~ 15) → 網格 27..43
    mark_horizontal_road(&mut grid, 27, 43);

    // 峨嵋街 (Z: -15 ~ 15) → 網格 27..43
    mark_vertical_road(&mut grid, 27, 43);

    commands.insert_resource(grid);
    commands.insert_resource(PointsOfInterest::setup_ximending());
}

/// A* 路徑計算系統
pub fn astar_path_calculation_system(
    time: Res<Time>,
    grid: Res<PathfindingGrid>,
    mut ped_query: Query<(&Transform, &mut AStarPath), With<Pedestrian>>,
) {
    let dt = time.delta_secs();

    for (transform, mut path) in ped_query.iter_mut() {
        // 更新冷卻時間
        if path.recalc_cooldown > 0.0 {
            path.recalc_cooldown -= dt;
            continue;
        }

        // 檢查是否需要重新計算路徑
        if path.needs_recalc || path.waypoints.is_empty() {
            let start = transform.translation;
            let goal = path.goal;

            if let Some(new_path) = grid.find_path(start, goal) {
                path.waypoints = new_path;
                path.current_index = 0;
                path.needs_recalc = false;
                path.recalc_cooldown = 2.0; // 2 秒冷卻
            } else {
                // 找不到路徑，重設冷卻
                path.recalc_cooldown = 5.0;
            }
        }
    }
}

/// A* 路徑跟隨移動系統
pub fn astar_movement_system(
    time: Res<Time>,
    config: Res<PedestrianConfig>,
    mut ped_query: Query<(
        &PedestrianState,
        &DailyBehavior,
        &mut Transform,
        &mut AStarPath,
        &mut KinematicCharacterController,
    ), With<Pedestrian>>,
) {
    let dt = time.delta_secs();

    for (state, behavior, mut transform, mut path, mut controller) in ped_query.iter_mut() {
        // 逃跑時不使用 A* 路徑
        if state.state == PedState::Fleeing {
            continue;
        }

        // 取得速度倍率
        let speed_mult = behavior.behavior.speed_multiplier();
        if speed_mult <= 0.0 {
            continue;
        }

        // 取得當前目標點
        let Some(target) = path.current_waypoint() else {
            continue;
        };

        // 計算移動方向
        let current_pos = transform.translation;
        let direction = (target - current_pos).normalize_or_zero();
        let flat_direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();

        if flat_direction.length_squared() < 0.001 {
            continue;
        }

        // 更新朝向
        let target_rotation = Quat::from_rotation_y((-flat_direction.x).atan2(-flat_direction.z));
        transform.rotation = transform.rotation.slerp(target_rotation, dt * 5.0);

        // 移動
        let speed = config.walk_speed * speed_mult;
        let velocity = flat_direction * speed;
        controller.translation = Some(velocity * dt + Vec3::new(0.0, -9.8 * dt, 0.0));

        // 檢查是否到達當前路徑點
        let flat_dist = Vec3::new(
            target.x - current_pos.x,
            0.0,
            target.z - current_pos.z,
        ).length();

        if flat_dist < 1.5 {
            if !path.advance() {
                // 到達終點，標記需要新路徑
                path.needs_recalc = true;
            }
        }
    }
}

// ============================================================================
// 日常行為系統
// ============================================================================

/// 日常行為初始化系統（為新生成的行人添加 DailyBehavior 和 AStarPath）
pub fn daily_behavior_init_system(
    mut commands: Commands,
    pois: Option<Res<PointsOfInterest>>,
    new_peds: Query<(Entity, &Transform), (With<Pedestrian>, Without<DailyBehavior>)>,
) {
    let mut rng = rand::rng();

    for (entity, transform) in new_peds.iter() {
        // 添加日常行為組件
        commands.entity(entity).insert(DailyBehavior::default());

        // 50% 機率使用 A* 尋路（更智能的行人）
        if rng.random_bool(0.5) {
            // 選擇隨機目標點
            let goal = if let Some(ref pois) = pois {
                // 嘗試找一個興趣點作為目標
                let roll: f32 = rng.random();
                if roll < 0.3 {
                    pois.find_nearest(transform.translation, PointOfInterestType::ShopWindow, 50.0)
                } else if roll < 0.5 {
                    pois.find_nearest(transform.translation, PointOfInterestType::Bench, 50.0)
                } else if roll < 0.7 {
                    pois.find_nearest(transform.translation, PointOfInterestType::PhotoSpot, 50.0)
                } else {
                    None
                }
            } else {
                None
            };

            // 如果找到興趣點，設為目標；否則使用隨機位置
            let target = goal.unwrap_or_else(|| {
                Vec3::new(
                    rng.random_range(-30.0..30.0),
                    0.25,
                    rng.random_range(-30.0..30.0),
                )
            });

            commands.entity(entity).insert(AStarPath {
                waypoints: Vec::new(),
                current_index: 0,
                goal: target,
                needs_recalc: true,
                recalc_cooldown: 0.0,
            });
        }
    }
}

// === 日常行為輔助函數 ===

/// 處理逃跑中的行人（釋放庇護點）
fn handle_fleeing_state(
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    pois: &mut PointsOfInterest,
) {
    behavior.behavior = BehaviorType::Walking;
    if shelter_seeker.is_sheltered {
        if let Some(target) = shelter_seeker.target_shelter {
            pois.release_shelter(target);
        }
        *shelter_seeker = ShelterSeeker::default();
    }
}

/// 檢查是否到達庇護點並處理
fn handle_shelter_arrival(
    pos: Vec3,
    shelter_seeker: &mut ShelterSeeker,
    astar_path: Option<&mut AStarPath>,
    pois: &mut PointsOfInterest,
    current_time: f32,
) {
    if shelter_seeker.is_sheltered {
        return;
    }

    let Some(target) = shelter_seeker.target_shelter else { return };
    let dist_sq = pos.distance_squared(target);

    if dist_sq >= SHELTER_ARRIVAL_SQ {
        return;
    }

    if pois.occupy_shelter(target) {
        shelter_seeker.arrive_at_shelter(current_time);
    } else if let Some(new_shelter) = pois.find_nearest_shelter(pos, SHELTER_SEARCH_RADIUS) {
        shelter_seeker.target_shelter = Some(new_shelter);
        if let Some(path) = astar_path {
            path.goal = new_shelter;
            path.needs_recalc = true;
        }
    }
}

/// 處理雨停的情況
fn handle_rain_stopped(
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    pois: &mut PointsOfInterest,
    rng: &mut impl Rng,
) {
    if let Some(target) = shelter_seeker.target_shelter {
        if shelter_seeker.is_sheltered {
            pois.release_shelter(target);
        }
    }
    behavior.behavior = shelter_seeker.previous_behavior;
    behavior.timer = 0.0;
    behavior.duration = rng.random_range(5.0..15.0);
    *shelter_seeker = ShelterSeeker::default();
}

/// 嘗試開始尋找庇護點
fn try_start_shelter_seeking(
    pos: Vec3,
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    astar_path: Option<&mut AStarPath>,
    pois: &PointsOfInterest,
) -> bool {
    let shelter_target = pois
        .find_nearest_shelter(pos, SHELTER_SEARCH_RADIUS)
        .or_else(|| pois.find_nearest(pos, PointOfInterestType::ShopWindow, SHOP_FALLBACK_SEARCH_RADIUS));

    let Some(target) = shelter_target else { return false };

    shelter_seeker.start_seeking(target, behavior.behavior);
    behavior.behavior = BehaviorType::SeekingShelter;
    behavior.duration = 120.0;
    behavior.timer = 0.0;

    if let Some(path) = astar_path {
        path.goal = target;
        path.needs_recalc = true;
    }
    true
}

/// 選擇雨天行為
fn select_rainy_behavior(rng: &mut impl Rng) -> BehaviorType {
    let roll: f32 = rng.random();
    if roll < 0.6 {
        BehaviorType::SeekingShelter
    } else if roll < 0.8 {
        BehaviorType::PhoneWatching
    } else {
        BehaviorType::Resting
    }
}

/// 根據新行為更新 A* 路徑
fn update_path_for_new_behavior(
    pos: Vec3,
    new_behavior: BehaviorType,
    path: &mut AStarPath,
    pois: &PointsOfInterest,
) {
    let poi_target = match new_behavior {
        BehaviorType::WindowShopping => pois.find_nearest(pos, PointOfInterestType::ShopWindow, 20.0),
        BehaviorType::Resting => pois.find_nearest(pos, PointOfInterestType::Bench, 30.0),
        BehaviorType::TakingPhoto => pois.find_nearest(pos, PointOfInterestType::PhotoSpot, 40.0),
        BehaviorType::SeekingShelter => pois.find_nearest(pos, PointOfInterestType::Shelter, SHELTER_SEARCH_RADIUS),
        _ => None,
    };

    if let Some(target) = poi_target {
        path.goal = target;
        path.needs_recalc = true;
    }
}

/// 處理單一行人的躲雨行為
fn process_shelter_behavior(
    pos: Vec3,
    is_raining: bool,
    current_time: f32,
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    astar_path: Option<&mut AStarPath>,
    pois: &mut PointsOfInterest,
    rng: &mut impl Rng,
) -> bool {
    handle_shelter_arrival(pos, shelter_seeker, astar_path, pois, current_time);
    if !is_raining {
        handle_rain_stopped(behavior, shelter_seeker, pois, rng);
    }
    true // 表示已處理，主迴圈應 continue
}

/// 嘗試在雨中開始躲雨
fn try_rain_shelter(
    pos: Vec3,
    rain_intensity: f32,
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    astar_path: Option<&mut AStarPath>,
    pois: &PointsOfInterest,
    rng: &mut impl Rng,
) -> bool {
    let shelter_chance = rain_intensity * SHELTER_SEEK_PROBABILITY_FACTOR;
    if rng.random::<f32>() >= shelter_chance {
        return false;
    }
    try_start_shelter_seeking(pos, behavior, shelter_seeker, astar_path, pois)
}

/// 更新行為計時並檢查是否需要切換
fn update_behavior_timer(
    dt: f32,
    is_raining: bool,
    pos: Vec3,
    behavior: &mut DailyBehavior,
    astar_path: Option<&mut AStarPath>,
    pois: &PointsOfInterest,
    rng: &mut impl Rng,
) {
    behavior.timer += dt;
    if behavior.timer < behavior.duration {
        return;
    }

    let new_behavior = if is_raining {
        select_rainy_behavior(rng)
    } else {
        select_next_behavior(rng, pos, pois)
    };

    let (min_dur, max_dur) = new_behavior.duration_range();
    behavior.behavior = new_behavior;
    behavior.duration = rng.random_range(min_dur..max_dur);
    behavior.timer = 0.0;

    if let Some(path) = astar_path {
        update_path_for_new_behavior(pos, new_behavior, path, pois);
    }
}

/// 日常行為更新系統（包含天氣反應）
pub fn daily_behavior_update_system(
    time: Res<Time>,
    mut pois: Option<ResMut<PointsOfInterest>>,
    weather: Res<WeatherState>,
    mut ped_query: Query<(
        Entity,
        &Transform,
        &PedestrianState,
        &mut DailyBehavior,
        &mut ShelterSeeker,
        Option<&mut AStarPath>,
    ), With<Pedestrian>>,
) {
    let dt = time.delta_secs();
    let current_time = time.elapsed_secs();
    let mut rng = rand::rng();

    let Some(ref mut pois) = pois else { return };

    let is_raining = weather.weather_type == WeatherType::Rainy;
    let rain_intensity = if is_raining { weather.intensity } else { 0.0 };

    for (_entity, transform, state, mut behavior, mut shelter_seeker, mut astar_path) in ped_query.iter_mut() {
        let pos = transform.translation;

        if state.state == PedState::Fleeing {
            handle_fleeing_state(&mut behavior, &mut shelter_seeker, pois);
            continue;
        }

        if behavior.behavior == BehaviorType::SeekingShelter {
            process_shelter_behavior(pos, is_raining, current_time, &mut behavior, &mut shelter_seeker, astar_path.as_deref_mut(), pois, &mut rng);
            continue;
        }

        if is_raining && try_rain_shelter(pos, rain_intensity, &mut behavior, &mut shelter_seeker, astar_path.as_deref_mut(), pois, &mut rng) {
            continue;
        }

        update_behavior_timer(dt, is_raining, pos, &mut behavior, astar_path.as_deref_mut(), pois, &mut rng);
    }
}

/// 選擇下一個行為
fn select_next_behavior(
    rng: &mut impl Rng,
    pos: Vec3,
    pois: &PointsOfInterest,
) -> BehaviorType {
    // 根據附近興趣點調整機率
    let has_shop_nearby = pois.find_nearest(pos, PointOfInterestType::ShopWindow, 15.0).is_some();
    let has_bench_nearby = pois.find_nearest(pos, PointOfInterestType::Bench, 20.0).is_some();
    let has_photo_spot = pois.find_nearest(pos, PointOfInterestType::PhotoSpot, 30.0).is_some();

    let roll: f32 = rng.random();

    // 行為機率分配
    if roll < 0.40 {
        BehaviorType::Walking
    } else if roll < 0.55 {
        BehaviorType::PhoneWatching
    } else if roll < 0.70 && has_shop_nearby {
        BehaviorType::WindowShopping
    } else if roll < 0.80 && has_bench_nearby {
        BehaviorType::Resting
    } else if roll < 0.90 && has_photo_spot {
        BehaviorType::TakingPhoto
    } else if roll < 0.95 {
        BehaviorType::Chatting
    } else {
        BehaviorType::Walking
    }
}

/// 行為動畫效果系統
pub fn behavior_animation_system(
    time: Res<Time>,
    mut ped_query: Query<(&DailyBehavior, &mut Transform, &mut WalkingAnimation), With<Pedestrian>>,
) {
    let elapsed = time.elapsed_secs();

    for (behavior, mut transform, mut anim) in ped_query.iter_mut() {
        match behavior.behavior {
            BehaviorType::PhoneWatching => {
                // 看手機：微微低頭，偶爾抬頭
                let head_bob = (elapsed * 0.3).sin() * 0.05;
                // 透過動畫速度控制腿部停止
                anim.speed = 0.0;
                // 身體微微前傾
                transform.rotation = transform.rotation.slerp(
                    Quat::from_rotation_x(0.1 + head_bob),
                    time.delta_secs() * 2.0,
                );
            }
            BehaviorType::WindowShopping => {
                // 逛櫥窗：緩慢左右轉動看櫥窗
                let look_angle = (elapsed * 0.5).sin() * 0.3;
                let base_rotation = transform.rotation;
                let look_rotation = Quat::from_rotation_y(look_angle);
                transform.rotation = base_rotation.slerp(
                    base_rotation * look_rotation,
                    time.delta_secs() * 1.0,
                );
            }
            BehaviorType::TakingPhoto => {
                // 拍照：舉起手（透過手臂旋轉模擬，這裡只做身體穩定）
                anim.speed = 0.0;
            }
            BehaviorType::Chatting => {
                // 聊天：身體微微搖擺
                let sway = (elapsed * 2.0).sin() * 0.02;
                transform.rotation = transform.rotation.slerp(
                    Quat::from_rotation_z(sway),
                    time.delta_secs() * 2.0,
                );
                anim.speed = 0.0;
            }
            BehaviorType::Resting => {
                // 休息：完全靜止
                anim.speed = 0.0;
            }
            BehaviorType::Walking => {
                // 正常行走：恢復動畫
                // 動畫速度在 walking_animation_system 中處理
            }
            BehaviorType::SeekingShelter => {
                // 躲雨：快速奔跑（類似 Walking 但更快）
                // 動畫速度在 walking_animation_system 中處理
                anim.speed = 2.0; // 加快動畫速度表現匆忙感
            }
        }
    }
}

// ============================================================================
// GTA 5 風格行人報警系統
// ============================================================================

/// 報警系統常數
mod witness_constants {
    /// 目擊視野角度（度）- 行人只能看到前方的犯罪
    pub const WITNESS_FOV_DEGREES: f32 = 120.0;
    /// 報警時的逃跑機率（部分行人會選擇逃跑而不是報警）
    pub const FLEE_INSTEAD_OF_CALL_CHANCE: f32 = 0.4;
    /// 報警基礎時間（秒）
    pub const BASE_CALL_DURATION: f32 = 3.0;
    /// 玩家靠近時報警中斷距離
    pub const INTIMIDATION_DISTANCE: f32 = 5.0;
    /// 玩家持槍時的恐嚇距離（更遠）
    pub const ARMED_INTIMIDATION_DISTANCE: f32 = 10.0;
}

// === 目擊系統輔助函數 ===

/// 將犯罪事件轉換為目擊類型
fn crime_event_to_witnessed_crime(crime: &CrimeEvent) -> WitnessedCrime {
    match crime {
        CrimeEvent::Shooting { .. } => WitnessedCrime::Gunshot,
        CrimeEvent::Assault { .. } => WitnessedCrime::Assault,
        CrimeEvent::Murder { .. } => WitnessedCrime::Murder,
        CrimeEvent::VehicleTheft { .. } => WitnessedCrime::VehicleTheft,
        CrimeEvent::VehicleHit { .. } => WitnessedCrime::VehicleHit,
        CrimeEvent::PoliceKilled { .. } => WitnessedCrime::Murder,
    }
}

/// 檢查行人是否能目擊犯罪
fn can_witness_crime(
    ped_transform: &Transform,
    crime_pos: Vec3,
    witness_range_sq: f32,
    fov_cos: f32,
    witnessed_crime: WitnessedCrime,
) -> bool {
    let ped_pos = ped_transform.translation;
    let distance_sq = ped_pos.distance_squared(crime_pos);

    if distance_sq > witness_range_sq {
        return false;
    }

    // 槍聲是聽覺，不需要視野檢查
    if witnessed_crime == WitnessedCrime::Gunshot {
        return true;
    }

    let to_crime = (crime_pos - ped_pos).normalize_or_zero();
    let forward = ped_transform.forward().as_vec3();
    forward.dot(to_crime) >= fov_cos
}

/// 處理行人對犯罪的反應
fn apply_witness_reaction(
    state: &mut PedestrianState,
    witness: &mut WitnessState,
    witnessed_crime: WitnessedCrime,
    crime_pos: Vec3,
    player_pos: Vec3,
    flee_chance: f32,
    base_call_duration: f32,
) {
    let mut rng = rand::rng();

    if rng.random::<f32>() < flee_chance {
        state.state = PedState::Fleeing;
        state.flee_timer = 10.0;
        state.fear_level = 1.0;
        state.last_threat_pos = Some(player_pos);
    } else {
        witness.witness_crime(witnessed_crime, crime_pos);
        state.state = PedState::CallingPolice;
        state.fear_level = 0.8;
        state.last_threat_pos = Some(player_pos);
        witness.call_duration = base_call_duration / witnessed_crime.severity();
    }
}

/// 行人目擊犯罪偵測系統
/// 當玩家犯罪時，通知範圍內的行人
pub fn witness_crime_detection_system(
    _time: Res<Time>,
    mut crime_events: MessageReader<CrimeEvent>,
    player_query: Query<&Transform, With<Player>>,
    mut ped_query: Query<(
        &Transform,
        &mut PedestrianState,
        &mut WitnessState,
    ), With<Pedestrian>>,
) {
    use witness_constants::*;

    let Ok(player_transform) = player_query.single() else { return };
    let player_pos = player_transform.translation;
    let fov_cos = (WITNESS_FOV_DEGREES / 2.0).to_radians().cos();

    for crime in crime_events.read() {
        let crime_pos = crime.position();
        let witnessed_crime = crime_event_to_witnessed_crime(crime);
        let witness_range_sq = witnessed_crime.witness_range().powi(2);
        let flee_chance = FLEE_INSTEAD_OF_CALL_CHANCE * (1.0 - witnessed_crime.severity());

        for (ped_transform, mut state, mut witness) in ped_query.iter_mut() {
            // 跳過已經在逃跑或報警的行人
            if state.state == PedState::Fleeing || state.state == PedState::CallingPolice {
                continue;
            }

            if !can_witness_crime(ped_transform, crime_pos, witness_range_sq, fov_cos, witnessed_crime) {
                continue;
            }

            apply_witness_reaction(
                &mut state,
                &mut witness,
                witnessed_crime,
                crime_pos,
                player_pos,
                flee_chance,
                BASE_CALL_DURATION,
            );
        }
    }
}

/// 檢查玩家是否持武器
fn is_player_armed(weapon_inventory: Option<&WeaponInventory>) -> bool {
    weapon_inventory
        .and_then(|inv| inv.current_weapon())
        .map(|w| w.stats.weapon_type != WeaponType::Fist)
        .unwrap_or(false)
}

/// 獲取目擊犯罪的描述
fn get_witnessed_crime_description(crime_type: WitnessedCrime) -> &'static str {
    match crime_type {
        WitnessedCrime::Gunshot => "槍擊",
        WitnessedCrime::Assault => "攻擊",
        WitnessedCrime::Murder => "謀殺",
        WitnessedCrime::VehicleTheft => "搶車",
        WitnessedCrime::VehicleHit => "撞人",
    }
}

/// 處理被恐嚇的情況（重置報警並逃跑）
fn handle_witness_intimidation(state: &mut PedestrianState, witness: &mut WitnessState) {
    witness.reset();
    state.state = PedState::Fleeing;
    state.flee_timer = 8.0;
    state.fear_level = 1.0;
}

/// 處理報警完成
fn handle_call_completion(
    witness: &WitnessState,
    state: &mut PedestrianState,
    witness_reports: &mut MessageWriter<WitnessReport>,
) {
    if let (Some(crime_type), Some(crime_pos)) = (witness.crime_type, witness.crime_position) {
        let description = get_witnessed_crime_description(crime_type);
        witness_reports.write(WitnessReport::new(crime_pos, description));
    }
    state.state = PedState::Walking;
    state.fear_level = 0.3;
}

/// 行人報警進度系統
/// 處理報警中的行人，更新進度並在完成時發送報警事件
pub fn witness_phone_call_system(
    time: Res<Time>,
    player_query: Query<(&Transform, Option<&WeaponInventory>), With<Player>>,
    mut ped_query: Query<(
        Entity,
        &Transform,
        &mut PedestrianState,
        &mut WitnessState,
    ), With<Pedestrian>>,
    mut witness_reports: MessageWriter<WitnessReport>,
) {
    use witness_constants::*;

    let dt = time.delta_secs();

    let Ok((player_transform, weapon_inventory)) = player_query.single() else { return };
    let player_pos = player_transform.translation;

    let intimidation_dist = if is_player_armed(weapon_inventory) {
        ARMED_INTIMIDATION_DISTANCE
    } else {
        INTIMIDATION_DISTANCE
    };
    let intimidation_dist_sq = intimidation_dist * intimidation_dist;

    for (_entity, ped_transform, mut state, mut witness) in ped_query.iter_mut() {
        // 只處理正在報警的行人
        if state.state != PedState::CallingPolice {
            witness.tick(dt);
            continue;
        }

        let dist_to_player_sq = ped_transform.translation.distance_squared(player_pos);

        // 玩家靠近時被恐嚇，中斷報警並逃跑
        if dist_to_player_sq < intimidation_dist_sq {
            handle_witness_intimidation(&mut state, &mut witness);
            continue;
        }

        // 更新報警進度
        if witness.tick(dt) {
            handle_call_completion(&witness, &mut state, &mut witness_reports);
        }
    }
}

/// 報警 UI 標記組件
#[derive(Component)]
pub struct WitnessPhoneIcon {
    pub owner: Entity,
}

/// 行人報警視覺效果系統
/// 在報警中的行人頭上顯示手機圖標和進度條
pub fn witness_visual_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ped_query: Query<(Entity, &Transform, &PedestrianState, &WitnessState), With<Pedestrian>>,
    existing_icons: Query<(Entity, &WitnessPhoneIcon)>,
) {
    // 移除不再需要的圖標
    for (icon_entity, icon) in existing_icons.iter() {
        let should_remove = ped_query
            .get(icon.owner)
            .map(|(_, _, state, _)| state.state != PedState::CallingPolice)
            .unwrap_or(true);

        if should_remove {
            commands.entity(icon_entity).despawn();
        }
    }

    // 為報警中的行人添加圖標
    for (ped_entity, transform, state, _witness) in ped_query.iter() {
        if state.state != PedState::CallingPolice {
            continue;
        }

        // 檢查是否已有圖標
        let has_icon = existing_icons.iter().any(|(_, icon)| icon.owner == ped_entity);
        if has_icon {
            continue;
        }

        // 在行人頭上生成手機圖標（使用簡單的方塊表示）
        let icon_pos = transform.translation + Vec3::new(0.0, 2.2, 0.0);

        // 手機圖標（藍色小方塊）
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.15, 0.25, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.5, 1.0),
                emissive: LinearRgba::rgb(0.0, 0.3, 1.0),
                ..default()
            })),
            Transform::from_translation(icon_pos),
            WitnessPhoneIcon { owner: ped_entity },
        ));
    }
}

/// 報警圖標跟隨系統
/// 讓圖標跟隨行人移動並顯示進度
pub fn witness_icon_follow_system(
    time: Res<Time>,
    ped_query: Query<(&Transform, &WitnessState), With<Pedestrian>>,
    mut icon_query: Query<(&WitnessPhoneIcon, &mut Transform), Without<Pedestrian>>,
) {
    let elapsed = time.elapsed_secs();

    for (icon, mut icon_transform) in icon_query.iter_mut() {
        if let Ok((ped_transform, witness)) = ped_query.get(icon.owner) {
            // 跟隨行人
            let target_pos = ped_transform.translation + Vec3::new(0.0, 2.2, 0.0);
            icon_transform.translation = target_pos;

            // 旋轉動畫（模擬打電話）
            let wobble = (elapsed * 8.0).sin() * 0.1;
            icon_transform.rotation = Quat::from_rotation_z(wobble);

            // 根據報警進度縮放（越接近完成越大）
            let scale = 1.0 + witness.call_progress * 0.5;
            icon_transform.scale = Vec3::splat(scale);
        }
    }
}

/// 報警進度條系統
/// 在 UI 上顯示附近報警中行人的進度
pub fn witness_progress_ui_system(
    player_query: Query<&Transform, With<Player>>,
    ped_query: Query<(&Transform, &WitnessState), (With<Pedestrian>, Changed<WitnessState>)>,
) {
    let Ok(player_transform) = player_query.single() else { return };
    let player_pos = player_transform.translation;

    // 找出最近的報警中行人
    let mut _nearest_witness: Option<(&Transform, &WitnessState, f32)> = None;

    for (ped_transform, witness) in ped_query.iter() {
        if !witness.witnessed_crime {
            continue;
        }

        // 使用 distance_squared 避免 sqrt
        let dist_sq = ped_transform.translation.distance_squared(player_pos);
        if dist_sq < WITNESS_UI_DISTANCE_SQ {
            let is_closer = _nearest_witness.map_or(true, |(_, _, d)| dist_sq < d);
            if is_closer {
                _nearest_witness = Some((ped_transform, witness, dist_sq));
            }
        }
    }

    // UI 渲染在 ui 模組中處理，這裡只做資料準備
    // 實際的 UI 會在 ui/systems.rs 中讀取 WitnessState 並渲染進度條
}

// ============================================================================
// GTA 5 風格群體恐慌傳播系統
// ============================================================================

/// 恐慌系統常數
mod panic_constants {
    /// 恐慌消退速率（每秒減少的 panic_level）
    pub const PANIC_CALM_DOWN_RATE: f32 = 0.05;
    /// 恐慌逃跑時的速度加成
    pub const PANIC_FLEE_SPEED_MULTIPLIER: f32 = 1.5;
    /// 恐慌狀態下的隨機方向偏移（弧度）
    pub const PANIC_DIRECTION_JITTER: f32 = 0.3;
    /// 逃跑計時器基礎時間（秒）
    pub const FLEE_TIMER_BASE: f32 = 8.0;
    /// 逃跑計時器恐慌加成係數
    pub const FLEE_TIMER_PANIC_MULTIPLIER: f32 = 5.0;
    /// 旋轉插值速度
    pub const ROTATION_SLERP_SPEED: f32 = 8.0;
}

// === 恐慌傳播輔助函數 ===

/// 收集被恐慌波影響的行人
fn collect_panic_triggers(
    waves: &[PanicWave],
    ped_hash: &PedestrianSpatialHash,
    wave_front_width: f32,
) -> Vec<(Entity, f32, Vec3)> {
    let mut panic_triggers: Vec<(Entity, f32, Vec3)> = Vec::new();

    for wave in waves {
        if wave.current_radius < 0.1 {
            continue;
        }

        for (entity, _, dist_sq) in ped_hash.query_radius(wave.origin, wave.current_radius) {
            let dist = dist_sq.sqrt();
            // 檢查是否在波前緣
            if dist <= wave.current_radius && dist > wave.current_radius - wave_front_width {
                update_panic_trigger(&mut panic_triggers, entity, wave.intensity, wave.origin);
            }
        }
    }

    panic_triggers
}

/// 更新或添加恐慌觸發記錄
fn update_panic_trigger(
    triggers: &mut Vec<(Entity, f32, Vec3)>,
    entity: Entity,
    intensity: f32,
    source: Vec3,
) {
    if let Some(existing) = triggers.iter_mut().find(|(e, _, _)| *e == entity) {
        if intensity > existing.1 {
            existing.1 = intensity;
            existing.2 = source;
        }
    } else {
        triggers.push((entity, intensity, source));
    }
}

/// 對單個行人應用恐慌觸發
fn apply_panic_to_pedestrian(
    ped_state: &mut PedestrianState,
    panic_state: &mut PanicState,
    intensity: f32,
    source: Vec3,
    flee_timer_base: f32,
    flee_timer_panic_mul: f32,
) {
    panic_state.trigger_panic(intensity, source);

    if panic_state.is_panicked() && ped_state.state != PedState::Fleeing {
        ped_state.state = PedState::Fleeing;
        ped_state.fear_level = panic_state.panic_level;
        ped_state.flee_timer = flee_timer_base + panic_state.panic_level * flee_timer_panic_mul;
        ped_state.last_threat_pos = Some(source);
    }
}

/// 處理恐慌消退
fn handle_panic_fade(
    ped_state: &mut PedestrianState,
    panic_state: &mut PanicState,
    still_in_wave: bool,
    calm_down_rate: f32,
    dt: f32,
) {
    if still_in_wave {
        return;
    }

    panic_state.calm_down(calm_down_rate, dt);

    if !panic_state.is_panicked() && ped_state.state == PedState::Fleeing && ped_state.flee_timer <= 0.0 {
        ped_state.state = PedState::Walking;
        ped_state.fear_level = 0.0;
    }
}

/// 恐慌波傳播系統（空間哈希優化版）
///
/// 使用 PedestrianSpatialHash 將 O(行人×波數) 降為 O(波數×附近行人)。
/// 每個恐慌波只檢查其半徑內的行人，而非所有行人。
pub fn panic_wave_propagation_system(
    time: Res<Time>,
    mut panic_manager: ResMut<PanicWaveManager>,
    ped_hash: Res<PedestrianSpatialHash>,
    mut ped_query: Query<(
        &Transform,
        &mut PedestrianState,
        &mut PanicState,
    ), With<Pedestrian>>,
) {
    use panic_constants::*;
    let dt = time.delta_secs();
    const WAVE_FRONT_WIDTH: f32 = 2.0;

    // 更新所有恐慌波（擴展半徑、清理過期）
    panic_manager.update(dt);

    // 階段 1：使用空間哈希找出被恐慌波影響的行人
    let panic_triggers = collect_panic_triggers(&panic_manager.active_waves, &ped_hash, WAVE_FRONT_WIDTH);

    // 階段 2：處理所有行人（更新計時器）
    for (_, _, mut panic_state) in ped_query.iter_mut() {
        panic_state.update(dt);
    }

    // 階段 3：應用恐慌觸發
    for (entity, intensity, source) in panic_triggers {
        let Ok((_, mut ped_state, mut panic_state)) = ped_query.get_mut(entity) else { continue };
        apply_panic_to_pedestrian(
            &mut ped_state,
            &mut panic_state,
            intensity,
            source,
            FLEE_TIMER_BASE,
            FLEE_TIMER_PANIC_MULTIPLIER,
        );
    }

    // 階段 4：恐慌消退（僅處理正在恐慌的行人）
    for (ped_transform, mut ped_state, mut panic_state) in ped_query.iter_mut() {
        if panic_state.panic_level <= 0.0 {
            continue;
        }

        let still_in_wave = panic_manager.check_panic_at(ped_transform.translation).is_some();
        handle_panic_fade(&mut ped_state, &mut panic_state, still_in_wave, PANIC_CALM_DOWN_RATE, dt);
    }
}

/// 行人尖叫傳播恐慌系統
/// 高度恐慌的行人會尖叫，產生新的恐慌波
pub fn pedestrian_scream_system(
    time: Res<Time>,
    mut panic_manager: ResMut<PanicWaveManager>,
    mut ped_query: Query<(&Transform, &mut PanicState), With<Pedestrian>>,
) {
    let current_time = time.elapsed_secs();

    for (ped_transform, mut panic_state) in ped_query.iter_mut() {
        // 檢查是否可以尖叫傳播恐慌
        if panic_state.can_scream() {
            let ped_pos = ped_transform.translation;

            // 產生新的恐慌波
            panic_manager.create_from_scream(
                ped_pos,
                panic_state.panic_level,
                current_time,
            );

            // 標記已尖叫（設置冷卻）
            panic_state.do_scream();
        }
    }
}

/// 槍聲觸發恐慌波系統
/// 當玩家開槍時，在槍聲位置創建恐慌波
pub fn gunshot_panic_trigger_system(
    time: Res<Time>,
    mut panic_manager: ResMut<PanicWaveManager>,
    gunshot_tracker: Res<GunshotTracker>,
    mut last_processed_count: Local<usize>,
) {
    let current_time = time.elapsed_secs();
    let current_count = gunshot_tracker.recent_shots.len();

    // 只處理新增的槍擊事件
    if current_count > *last_processed_count {
        for shot in gunshot_tracker.recent_shots.iter().skip(*last_processed_count) {
            let (shot_pos, _shot_time) = *shot;
            panic_manager.create_from_gunshot(shot_pos, current_time);
        }
        *last_processed_count = current_count;
    }

    // 重置計數器（當 tracker 清理過期事件時）
    if current_count < *last_processed_count {
        *last_processed_count = 0;
    }
}

/// 恐慌逃跑方向系統
/// 讓恐慌的行人朝著遠離恐慌源的方向逃跑，並加入一些隨機偏移
pub fn panic_flee_direction_system(
    time: Res<Time>,
    config: Res<PedestrianConfig>,
    mut rng: Local<Option<rand::rngs::StdRng>>,
    mut ped_query: Query<(
        &mut Transform,
        &PedestrianState,
        &PanicState,
        &mut WalkingAnimation,
    ), With<Pedestrian>>,
) {
    use panic_constants::*;
    use rand::SeedableRng;

    let dt = time.delta_secs();

    // 初始化持久化 RNG（只在第一次調用時創建）
    let rng = rng.get_or_insert_with(|| rand::rngs::StdRng::from_rng(&mut rand::rng()));

    for (mut transform, ped_state, panic_state, mut anim) in ped_query.iter_mut() {
        // 只處理因恐慌而逃跑的行人
        if ped_state.state != PedState::Fleeing || !panic_state.is_panicked() {
            continue;
        }

        // 計算逃跑方向
        if let Some(flee_dir) = panic_state.flee_direction(transform.translation) {
            // 加入隨機方向偏移（模擬恐慌中的混亂）
            let jitter_angle = rng.random_range(-PANIC_DIRECTION_JITTER..PANIC_DIRECTION_JITTER);
            let jitter_rotation = Quat::from_rotation_y(jitter_angle);
            let jittered_dir = jitter_rotation * flee_dir;

            // 計算移動速度（恐慌程度越高越快）
            let speed = config.flee_speed * PANIC_FLEE_SPEED_MULTIPLIER * panic_state.panic_level;

            // 移動
            let movement = jittered_dir * speed * dt;
            transform.translation += movement;

            // 更新朝向
            if jittered_dir.length_squared() > 0.01 {
                let target_rotation = Quat::from_rotation_y(
                    (-jittered_dir.z).atan2(jittered_dir.x) - std::f32::consts::FRAC_PI_2
                );
                transform.rotation = transform.rotation.slerp(target_rotation, dt * ROTATION_SLERP_SPEED);
            }

            // 更新動畫速度（恐慌時動畫更快）
            anim.speed = speed / config.walk_speed;
        }
    }
}
