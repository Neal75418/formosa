//! 警車生成與消失系統

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::core::GameState;
use crate::player::Player;
use crate::vehicle::{VehicleHealth, VehicleId, VehicleModifications, VehiclePreset};
use crate::wanted::WantedLevel;

use super::{
    PoliceCar, PoliceCarConfig, PoliceCarVisuals, SirenLight, MAX_POLICE_CARS_PER_STAR,
    POLICE_CAR_DESPAWN_DISTANCE_SQ, POLICE_CAR_DESPAWN_FAR_DISTANCE_SQ,
    POLICE_CAR_SPAWN_DISTANCE_MAX, POLICE_CAR_SPAWN_DISTANCE_MIN,
};

// ============================================================================
// 設置系統
// ============================================================================

/// 設置警車視覺資源
pub fn setup_police_car_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(PoliceCarVisuals::new(&mut meshes, &mut materials));
    commands.insert_resource(PoliceCarConfig::default());
}

// ============================================================================
// 生成系統
// ============================================================================

/// 警車生成系統
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn spawn_police_car_system(
    mut commands: Commands,
    wanted: Res<WantedLevel>,
    mut config: ResMut<PoliceCarConfig>,
    game_state: Res<GameState>,
    police_car_query: Query<Entity, With<PoliceCar>>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    visuals: Option<Res<PoliceCarVisuals>>,
    time: Res<Time>,
) {
    // 2 星以上且玩家在車上才生成警車
    if wanted.stars < 2 || !game_state.player_in_vehicle {
        return;
    }

    let Some(visuals) = visuals else {
        return;
    };

    let Ok((_player_entity, player_transform)) = player_query.single() else {
        return;
    };

    let current_count = police_car_query.iter().count() as u32;
    let max_count = (u32::from(wanted.stars) - 1) * MAX_POLICE_CARS_PER_STAR;

    // 檢查是否達到上限
    if current_count >= max_count {
        return;
    }

    // 檢查生成間隔
    let elapsed = time.elapsed_secs();
    if elapsed - config.last_spawn_time < config.spawn_interval {
        return;
    }

    config.last_spawn_time = elapsed;

    // 計算生成位置（玩家後方或側面）
    let player_pos = player_transform.translation;
    let player_forward = player_transform.forward().as_vec3();

    // 隨機選擇生成方向（後方或側面）
    let spawn_angle = rand::random::<f32>() * std::f32::consts::PI + std::f32::consts::FRAC_PI_2;
    let spawn_dir = Quat::from_rotation_y(spawn_angle) * player_forward;

    let distance = POLICE_CAR_SPAWN_DISTANCE_MIN
        + rand::random::<f32>() * (POLICE_CAR_SPAWN_DISTANCE_MAX - POLICE_CAR_SPAWN_DISTANCE_MIN);

    let spawn_pos = Vec3::new(
        player_pos.x + spawn_dir.x * distance,
        0.5, // 車輛高度
        player_pos.z + spawn_dir.z * distance,
    );

    // 生成警車
    spawn_police_car(&mut commands, spawn_pos, player_pos, &visuals);

    info!(
        "生成警車 at ({:.1}, {:.1}) - 當前: {}/{}",
        spawn_pos.x,
        spawn_pos.z,
        current_count + 1,
        max_count
    );
}

/// 生成單台警車
fn spawn_police_car(
    commands: &mut Commands,
    position: Vec3,
    player_pos: Vec3,
    visuals: &PoliceCarVisuals,
) {
    // 計算朝向玩家的初始旋轉
    let to_player = (player_pos - position).normalize_or_zero();
    let rotation = Quat::from_rotation_y((-to_player.x).atan2(-to_player.z));

    // 分批添加組件以避免 tuple 過大
    let car_entity = commands
        .spawn((
            Name::new("PoliceCar"),
            Transform::from_translation(position).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .insert((
            PoliceCar::default(),
            VehiclePreset::car().into_components(),
            VehicleHealth::new(1500.0),      // 警車較耐打
            VehicleId::new(),                // 穩定識別碼（用於存檔）
            VehicleModifications::default(), // 改裝狀態（用於存檔）
        ))
        .insert((
            // 物理組件
            RigidBody::Dynamic,
            Collider::cuboid(1.0, 0.5, 2.25),
            CollisionGroups::new(
                Group::GROUP_3, // 車輛群組
                Group::ALL,
            ),
            Velocity::default(),
            ExternalForce::default(),
        ))
        .insert((
            Friction::coefficient(0.5),
            Restitution::coefficient(0.3),
            ColliderMassProperties::Density(500.0),
            Damping {
                linear_damping: 0.5,
                angular_damping: 2.0,
            },
        ))
        .id();

    // 添加視覺模型
    commands.entity(car_entity).with_children(|parent| {
        // 車身
        parent.spawn((
            Mesh3d(visuals.body_mesh.clone()),
            MeshMaterial3d(visuals.body_material.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)),
        ));

        // 警笛燈（紅）
        parent.spawn((
            Mesh3d(visuals.siren_mesh.clone()),
            MeshMaterial3d(visuals.siren_red_material.clone()),
            Transform::from_translation(Vec3::new(-0.25, 0.9, 0.0)),
            SirenLight {
                is_red: true,
                flash_timer: 0.0,
                is_on: true,
            },
        ));

        // 警笛燈（藍）
        parent.spawn((
            Mesh3d(visuals.siren_mesh.clone()),
            MeshMaterial3d(visuals.siren_blue_material.clone()),
            Transform::from_translation(Vec3::new(0.25, 0.9, 0.0)),
            SirenLight {
                is_red: false,
                flash_timer: 0.0,
                is_on: false,
            },
        ));

        // 輪胎（四個）
        let wheel_positions = [
            Vec3::new(-0.9, -0.1, 1.5),  // 左前
            Vec3::new(0.9, -0.1, 1.5),   // 右前
            Vec3::new(-0.9, -0.1, -1.5), // 左後
            Vec3::new(0.9, -0.1, -1.5),  // 右後
        ];

        for pos in wheel_positions {
            parent.spawn((
                Mesh3d(visuals.wheel_mesh.clone()),
                MeshMaterial3d(visuals.wheel_material.clone()),
                Transform::from_translation(pos)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            ));
        }
    });
}

// ============================================================================
// 消失系統
// ============================================================================

/// 警車消失系統
pub fn despawn_police_car_system(
    mut commands: Commands,
    police_car_query: Query<(Entity, &Transform, &VehicleHealth), With<PoliceCar>>,
    player_query: Query<&Transform, (With<Player>, Without<PoliceCar>)>,
    wanted: Res<WantedLevel>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (entity, transform, health) in &police_car_query {
        // 使用 length_squared 避免 sqrt 運算
        let distance_sq = (transform.translation - player_pos).length_squared();

        // 消失條件：通緝消退、距離太遠、或已爆炸超過 10 秒
        let should_despawn = (wanted.stars < 2 && distance_sq > POLICE_CAR_DESPAWN_DISTANCE_SQ)
            || distance_sq > POLICE_CAR_DESPAWN_FAR_DISTANCE_SQ
            || (health.is_destroyed() && health.explosion_cooldown > 10.0);

        if should_despawn {
            commands.entity(entity).despawn();
            debug!("警車消失: {:?}", entity);
        }
    }
}
