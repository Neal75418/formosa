//! 直升機設置與生成

use super::super::WantedLevel;
#[allow(clippy::wildcard_imports)]
use super::components::*;
use crate::combat::Health;
use crate::player::Player;
use bevy::prelude::*;

// ============================================================================
// 設置系統
// ============================================================================

/// 初始化直升機視覺資源
pub fn setup_helicopter_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 機身材質（深藍色警用塗裝）
    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.15, 0.3),
        metallic: 0.6,
        perceptual_roughness: 0.4,
        ..default()
    });

    // 旋翼材質（灰色金屬）
    let rotor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.45),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    // 探照燈材質（發光白色）
    let spotlight_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 0.9),
        emissive: LinearRgba::rgb(10.0, 10.0, 9.0),
        ..default()
    });

    // 機身 mesh（簡化橢圓體）
    let body_mesh = meshes.add(Capsule3d::new(1.5, 4.0));

    // 主旋翼 mesh（扁平圓柱代表旋轉中的旋翼）
    let main_rotor_mesh = meshes.add(Cylinder::new(4.5, 0.1));

    // 尾旋翼 mesh（較小圓柱）
    let tail_rotor_mesh = meshes.add(Cylinder::new(1.0, 0.05));

    commands.insert_resource(HelicopterVisuals {
        body_material,
        rotor_material,
        spotlight_material,
        body_mesh,
        main_rotor_mesh,
        tail_rotor_mesh,
    });

    commands.init_resource::<HelicopterSpawnState>();
}

// ============================================================================
// 生成系統
// ============================================================================

/// 直升機生成系統
pub fn spawn_helicopter_system(
    mut commands: Commands,
    time: Res<Time>,
    wanted: Res<WantedLevel>,
    mut spawn_state: ResMut<HelicopterSpawnState>,
    visuals: Res<HelicopterVisuals>,
    player_query: Query<&Transform, With<Player>>,
    helicopter_query: Query<Entity, With<PoliceHelicopter>>,
) {
    // 更新冷卻
    spawn_state.cooldown -= time.delta_secs();

    // 更新當前數量
    spawn_state.count = helicopter_query.iter().count();

    // 檢查是否需要生成
    if wanted.stars < HELICOPTER_SPAWN_WANTED_LEVEL {
        return;
    }

    if spawn_state.count >= HELICOPTER_MAX_COUNT {
        return;
    }

    if spawn_state.cooldown > 0.0 {
        return;
    }

    // 取得玩家位置
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 在玩家後方遠處生成
    let spawn_angle = rand::random::<f32>() * std::f32::consts::TAU;
    let spawn_distance = 100.0 + rand::random::<f32>() * 50.0;
    let spawn_pos = Vec3::new(
        player_pos.x + spawn_angle.cos() * spawn_distance,
        HELICOPTER_HOVER_ALTITUDE + 20.0, // 高空進場
        player_pos.z + spawn_angle.sin() * spawn_distance,
    );

    // 生成直升機實體
    let _helicopter_id = spawn_helicopter(&mut commands, &visuals, spawn_pos);

    info!("🚁 警用直升機出動: {:?}", spawn_pos);

    // 重置冷卻（count 由下一幀的 query 自動更新）
    spawn_state.cooldown = HELICOPTER_SPAWN_COOLDOWN;
}

/// 生成單個直升機
fn spawn_helicopter(
    commands: &mut Commands,
    visuals: &HelicopterVisuals,
    position: Vec3,
) -> Entity {
    // 機身
    let helicopter_id = commands
        .spawn((
            Mesh3d(visuals.body_mesh.clone()),
            MeshMaterial3d(visuals.body_material.clone()),
            Transform::from_translation(position),
            PoliceHelicopter::default(),
            Health::new(HELICOPTER_HEALTH),
            Name::new("PoliceHelicopter"),
        ))
        .id();

    // 主旋翼（在機身上方）
    let main_rotor_id = commands
        .spawn((
            Mesh3d(visuals.main_rotor_mesh.clone()),
            MeshMaterial3d(visuals.rotor_material.clone()),
            Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
            HelicopterRotor::main(),
            HelicopterParent(helicopter_id),
            Name::new("MainRotor"),
        ))
        .id();

    // 尾旋翼（在機尾側面）
    let tail_rotor_id = commands
        .spawn((
            Mesh3d(visuals.tail_rotor_mesh.clone()),
            MeshMaterial3d(visuals.rotor_material.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.5, -4.0))
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            HelicopterRotor::tail(),
            HelicopterParent(helicopter_id),
            Name::new("TailRotor"),
        ))
        .id();

    // 探照燈（在機身下方）
    let spotlight_id = commands
        .spawn((
            SpotLight {
                color: Color::srgb(1.0, 1.0, 0.9),
                intensity: 500_000.0,
                range: SPOTLIGHT_RANGE,
                outer_angle: SPOTLIGHT_CONE_ANGLE.to_radians(),
                inner_angle: (SPOTLIGHT_CONE_ANGLE * 0.6).to_radians(),
                shadows_enabled: true,
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, -1.5, 1.0))
                .looking_at(Vec3::new(0.0, -10.0, 5.0), Vec3::Y),
            HelicopterSpotlight::default(),
            HelicopterParent(helicopter_id),
            Name::new("Spotlight"),
        ))
        .id();

    // 設置父子關係
    commands
        .entity(helicopter_id)
        .add_children(&[main_rotor_id, tail_rotor_id, spotlight_id]);

    helicopter_id
}
