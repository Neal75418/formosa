//! 行人核心生命週期（設置、生成、移動、銷毀）

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use crate::ai::{AiMovement, PatrolPath};
use crate::combat::{BodyPart, Damageable, Health, HitReaction};
use crate::core::math::look_rotation_y_flat;
use crate::core::COLLISION_GROUP_CHARACTER;
use crate::pedestrian::behavior::{DailyBehavior, ShelterSeeker};
use crate::pedestrian::pathfinding::AStarPath;
use crate::pedestrian::components::{
    GunshotTracker, PedState, Pedestrian, PedestrianArm, PedestrianConfig, PedestrianLeg,
    PedestrianPaths, PedestrianState, PedestrianType, PedestrianVisuals, SidewalkPath,
    WalkingAnimation, WitnessState,
};
use crate::pedestrian::panic::PanicState;
use crate::player::Player;

/// 最小生成距離平方 (15.0²)
const MIN_SPAWN_DISTANCE_SQ: f32 = 225.0;

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
    let Ok(player_transform) = player_query.single() else {
        return;
    };
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
    let rotation = look_rotation_y_flat(look_dir);

    // 生成行人實體（使用單一 spawn 搭配 with_children 避免 B0004 警告）
    // 注意：Bevy Bundle 限制為 15 個組件，因此分成多個 insert 調用
    commands
        .spawn((
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
            HitReaction::default(), // 受傷反應（畏縮、踉蹌、擊退）
        ))
        .insert((
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
                current_index: if start_index + 1 < waypoints.len() {
                    start_index + 1
                } else {
                    0
                },
                ping_pong,
                forward: true,
                wait_time: 0.0,
                wait_timer: 0.0,
            },
        ))
        .insert((
            // 物理組件（分開 insert 以符合 Bundle 大小限制）
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(body_height / 2.0 - 0.2, 0.25),
            CollisionGroups::new(COLLISION_GROUP_CHARACTER, Group::ALL),
            KinematicCharacterController {
                offset: CharacterLength::Absolute(0.01),
                ..default()
            },
        ))
        .with_children(|parent| {
            // 頭部（含布娃娃標記）
            parent.spawn((
                Mesh3d(visuals.head_mesh.clone()),
                MeshMaterial3d(visuals.skin_materials[indices.skin].clone()),
                Transform::from_xyz(0.0, torso_height / 2.0 + head_radius + 0.05, 0.0),
                BodyPart::head(),
            ));
            // 頭髮（不需要物理，跟隨頭部）
            parent.spawn((
                Mesh3d(visuals.hair_mesh.clone()),
                MeshMaterial3d(visuals.hair_materials[indices.hair].clone()),
                Transform::from_xyz(0.0, torso_height / 2.0 + head_radius + 0.08, -0.02)
                    .with_scale(Vec3::new(1.0, 0.8, 1.0)),
            ));
            // 軀幹（布娃娃核心）
            parent.spawn((
                Mesh3d(visuals.torso_mesh.clone()),
                MeshMaterial3d(visuals.shirt_materials[indices.shirt].clone()),
                Transform::from_xyz(0.0, 0.0, 0.0),
                BodyPart::torso(),
            ));
            // 左腿（加標記用於動畫和布娃娃）
            parent.spawn((
                Mesh3d(visuals.leg_mesh.clone()),
                MeshMaterial3d(visuals.pants_materials[indices.pants].clone()),
                Transform::from_xyz(-0.08, -torso_height / 2.0 - leg_height / 2.0, 0.0),
                PedestrianLeg { is_left: true },
                BodyPart::left_leg(),
            ));
            // 右腿（加標記用於動畫和布娃娃）
            parent.spawn((
                Mesh3d(visuals.leg_mesh.clone()),
                MeshMaterial3d(visuals.pants_materials[indices.pants].clone()),
                Transform::from_xyz(0.08, -torso_height / 2.0 - leg_height / 2.0, 0.0),
                PedestrianLeg { is_left: false },
                BodyPart::right_leg(),
            ));
            // 左腳（布娃娃標記）
            parent.spawn((
                Mesh3d(visuals.shoe_mesh.clone()),
                MeshMaterial3d(visuals.shoe_materials[indices.shoe].clone()),
                Transform::from_xyz(-0.08, -torso_height / 2.0 - leg_height - 0.025, 0.03),
                BodyPart::left_foot(),
            ));
            // 右腳（布娃娃標記）
            parent.spawn((
                Mesh3d(visuals.shoe_mesh.clone()),
                MeshMaterial3d(visuals.shoe_materials[indices.shoe].clone()),
                Transform::from_xyz(0.08, -torso_height / 2.0 - leg_height - 0.025, 0.03),
                BodyPart::right_foot(),
            ));
            // 左手臂（加標記用於動畫和布娃娃）
            parent.spawn((
                Mesh3d(visuals.arm_mesh.clone()),
                MeshMaterial3d(visuals.shirt_materials[indices.shirt].clone()),
                Transform::from_xyz(-0.22, torso_height / 4.0, 0.0)
                    .with_rotation(Quat::from_rotation_z(0.15)),
                PedestrianArm { is_left: true },
                BodyPart::left_arm(),
            ));
            // 右手臂（加標記用於動畫和布娃娃）
            parent.spawn((
                Mesh3d(visuals.arm_mesh.clone()),
                MeshMaterial3d(visuals.shirt_materials[indices.shirt].clone()),
                Transform::from_xyz(0.22, torso_height / 4.0, 0.0)
                    .with_rotation(Quat::from_rotation_z(-0.15)),
                PedestrianArm { is_left: false },
                BodyPart::right_arm(),
            ));
        });
}

// ============================================================================
// 移動系統
// ============================================================================

// ============================================================================
// 移動輔助函數
// ============================================================================
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
                flee_target.x.clamp(-95.0, 75.0), // 留5公尺邊界緩衝
                flee_target.y,
                flee_target.z.clamp(-75.0, 45.0), // 留5公尺邊界緩衝
            );
            return Some(clamped);
        }
    }
    patrol.current_waypoint()
}

/// 行人移動系統
///
/// 只處理沒有 `AStarPath` 的行人（有 `AStarPath` 的由 `astar_movement_system` 處理）。
/// 當行人有 `DailyBehavior` 時，尊重其 `speed_multiplier()`（看手機、休息等行為速度為 0）。
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
        Option<&DailyBehavior>,
    ), Without<AStarPath>>,
) {
    let dt = time.delta_secs();

    for (_ped, state, mut transform, mut patrol, movement, mut controller, daily_behavior) in ped_query.iter_mut() {
        let mut speed = get_pedestrian_speed(state.state, &config);

        // 非逃跑狀態下，尊重 DailyBehavior 的速度倍率
        if state.state != PedState::Fleeing {
            if let Some(behavior) = daily_behavior {
                speed *= behavior.behavior.speed_multiplier();
            }
        }

        if speed <= 0.0 {
            continue;
        }

        let current_pos = transform.translation;
        let Some(target_pos) = get_movement_target(state, current_pos, &patrol) else {
            continue;
        };

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
        let flat_dist_sq =
            (target_pos.x - current_pos.x).powi(2) + (target_pos.z - current_pos.z).powi(2);
        if flat_dist_sq < movement.arrival_threshold.powi(2) && state.state != PedState::Fleeing {
            patrol.advance();
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
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let dt = time.delta_secs();

    // 地圖邊界常數（與 setup.rs 一致）
    const MAP_MIN_X: f32 = -100.0; // X_KANGDING
    const MAP_MAX_X: f32 = 80.0; // X_ZHONGHUA
    const MAP_MIN_Z: f32 = -80.0; // Z_HANKOU
    const MAP_MAX_Z: f32 = 50.0; // Z_CHENGDU

    // 使用 distance_squared 避免 sqrt
    let despawn_radius_sq = config.despawn_radius * config.despawn_radius;
    for (entity, transform, mut state) in ped_query.iter_mut() {
        let current_pos = transform.translation;

        // 超出地圖邊界，立即移除
        if current_pos.x < MAP_MIN_X
            || current_pos.x > MAP_MAX_X
            || current_pos.z < MAP_MIN_Z
            || current_pos.z > MAP_MAX_Z
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
