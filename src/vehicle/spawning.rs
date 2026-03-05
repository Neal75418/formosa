//! 載具生成（NPC 車輛、機車、初始交通）

use super::vehicle_damage::BodyPartDamage;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::core::math::look_rotation_y_flat;
use crate::core::{COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE};
use crate::world::{
    W_MAIN, W_SECONDARY, W_ZHONGHUA, X_KANGDING, X_XINING, X_ZHONGHUA, Z_CHENGDU, Z_HANKOU,
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

// ============================================================================
// 車輛生成輔助函數
// ============================================================================
/// 創建車身材質
fn create_body_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    metallic: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.3,
        metallic,
        ..default()
    })
}

/// 生成帶材質的方塊子實體
fn spawn_mesh_child(
    parent: &mut ChildSpawnerCommands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
) {
    parent.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
        GlobalTransform::default(),
    ));
}

/// 生成車燈（消除重複程式碼）
fn spawn_vehicle_light(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    x: f32,
    y: f32,
    z: f32,
) {
    let light_mesh = meshes.add(Cuboid::new(0.4, 0.2, 0.1));
    spawn_mesh_child(parent, light_mesh, material, Transform::from_xyz(x, y, z));
}

/// 生成車輛前後燈組（左右對稱）
fn spawn_vehicle_lights(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    headlight_mat: Handle<StandardMaterial>,
    taillight_mat: Handle<StandardMaterial>,
    chassis_size: Vec3,
) {
    let light_z = -chassis_size.z / 2.0 - 0.05;
    let light_x = chassis_size.x / 2.0 - 0.4;
    let tail_z = chassis_size.z / 2.0 + 0.05;

    // 前燈（左右）
    spawn_vehicle_light(
        parent,
        meshes,
        headlight_mat.clone(),
        -light_x,
        0.1,
        light_z,
    );
    spawn_vehicle_light(parent, meshes, headlight_mat, light_x, 0.1, light_z);

    // 尾燈（左右）
    spawn_vehicle_light(parent, meshes, taillight_mat.clone(), -light_x, 0.1, tail_z);
    spawn_vehicle_light(parent, meshes, taillight_mat, light_x, 0.1, tail_z);
}

/// 生成底盤 + 車艙 mesh
fn spawn_vehicle_body(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    shared_mats: &VehicleMaterials,
    chassis_size: Vec3,
    vehicle_type: VehicleType,
    color: Color,
) {
    // 底盤 (Chassis) - 下半部（附帶變形標記）
    let body_mat = create_body_material(materials, color, 0.5);
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(chassis_size))),
        MeshMaterial3d(body_mat),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
        VehicleChassisMesh,
        VehicleOriginalColor(color),
    ));

    // 車艙 (Cabin) - 上半部 (玻璃)
    let cabin_size = match vehicle_type {
        VehicleType::Bus => Vec3::new(2.7, 1.0, 7.5),
        _ => Vec3::new(1.8, 0.5, 2.0),
    };
    let cabin_y = chassis_size.y / 2.0 + cabin_size.y / 2.0;
    let cabin_z_offset = match vehicle_type {
        VehicleType::Bus => 0.0,
        _ => -0.2,
    };

    parent.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(cabin_size))),
        MeshMaterial3d(shared_mats.glass.clone()),
        Transform::from_xyz(0.0, cabin_y, cabin_z_offset),
        GlobalTransform::default(),
        VehicleCabinMesh {
            base_y: cabin_y,
            base_z: cabin_z_offset,
        },
    ));
}

/// 生成 4 輪 mesh
fn spawn_vehicle_wheels(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    shared_mats: &VehicleMaterials,
    chassis_size: Vec3,
    wheel_offset_z: f32,
) {
    let wheel_mesh = meshes.add(Cylinder::new(0.35, 0.3));
    let wheel_y = -chassis_size.y / 2.0;
    let wheel_x = chassis_size.x / 2.0;

    let wheel_positions = [
        Vec3::new(-wheel_x, wheel_y, -wheel_offset_z), // 左前
        Vec3::new(wheel_x, wheel_y, -wheel_offset_z),  // 右前
        Vec3::new(-wheel_x, wheel_y, wheel_offset_z),  // 左後
        Vec3::new(wheel_x, wheel_y, wheel_offset_z),   // 右後
    ];

    for pos in wheel_positions {
        parent.spawn((
            Mesh3d(wheel_mesh.clone()),
            MeshMaterial3d(shared_mats.wheel.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
            GlobalTransform::default(),
        ));
    }
}

/// 生成機車車身零件（踏板、車頭、座墊、車尾箱、把手、後照鏡）
fn spawn_scooter_body_parts(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    shared_mats: &VehicleMaterials,
    body_mat: Handle<StandardMaterial>,
    body_width: f32,
    body_length: f32,
    seat_height: f32,
) {
    let black_mat = shared_mats.black_plastic.clone();

    // 1. 踏板區 (腳踏平台)
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(body_width, 0.08, body_length * 0.5))),
        MeshMaterial3d(black_mat.clone()),
        Transform::from_xyz(0.0, -0.1, 0.0),
        GlobalTransform::default(),
    ));

    // 2. 車頭斜面
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(body_width * 0.8, 0.4, 0.4))),
        MeshMaterial3d(body_mat.clone()),
        Transform::from_xyz(0.0, 0.15, -body_length / 2.0 + 0.2)
            .with_rotation(Quat::from_rotation_x(-0.3)),
        GlobalTransform::default(),
    ));

    // 3. 座墊
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(body_width * 0.7, 0.12, body_length * 0.45))),
        MeshMaterial3d(black_mat.clone()),
        Transform::from_xyz(0.0, seat_height * 0.45, body_length * 0.1),
        GlobalTransform::default(),
    ));

    // 4. 車尾箱
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(body_width * 0.6, 0.25, 0.3))),
        MeshMaterial3d(body_mat),
        Transform::from_xyz(0.0, seat_height * 0.5, body_length / 2.0 - 0.15),
        GlobalTransform::default(),
    ));

    // 5. 把手區
    parent.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.02, body_width + 0.3))),
        MeshMaterial3d(black_mat),
        Transform::from_xyz(0.0, seat_height * 0.8, -body_length / 2.0 + 0.1)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        GlobalTransform::default(),
    ));

    // 6. 後照鏡（左右）
    for x_sign in [-1.0_f32, 1.0] {
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.08, 0.05, 0.02))),
            MeshMaterial3d(shared_mats.mirror.clone()),
            Transform::from_xyz(
                x_sign * (body_width / 2.0 + 0.2),
                seat_height * 0.85,
                -body_length / 2.0 + 0.15,
            ),
            GlobalTransform::default(),
        ));
    }
}

/// 生成機車輪子、燈光、擋泥板
fn spawn_scooter_wheels_and_lights(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    shared_mats: &VehicleMaterials,
    body_mat: Handle<StandardMaterial>,
    body_length: f32,
    seat_height: f32,
) {
    // 前輪
    parent.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.25, 0.12))),
        MeshMaterial3d(shared_mats.wheel.clone()),
        Transform::from_xyz(0.0, -0.15, -body_length / 2.0 - 0.1)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        GlobalTransform::default(),
    ));

    // 後輪
    parent.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.25, 0.15))),
        MeshMaterial3d(shared_mats.wheel.clone()),
        Transform::from_xyz(0.0, -0.15, body_length / 2.0 - 0.1)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        GlobalTransform::default(),
    ));

    // 頭燈
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.15, 0.1, 0.05))),
        MeshMaterial3d(shared_mats.headlight.clone()),
        Transform::from_xyz(0.0, 0.25, -body_length / 2.0 - 0.05),
        GlobalTransform::default(),
    ));

    // 尾燈
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.2, 0.06, 0.03))),
        MeshMaterial3d(shared_mats.taillight.clone()),
        Transform::from_xyz(0.0, seat_height * 0.4, body_length / 2.0 + 0.02),
        GlobalTransform::default(),
    ));

    // 前擋泥板
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.12, 0.02, 0.3))),
        MeshMaterial3d(body_mat),
        Transform::from_xyz(0.0, 0.05, -body_length / 2.0 - 0.1)
            .with_rotation(Quat::from_rotation_x(0.2)),
        GlobalTransform::default(),
    ));
}

/// 生成改裝配件（尾翼、底盤燈、側裙霓虹條）
fn spawn_tuning_parts(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    shared_mats: &VehicleMaterials,
    chassis_size: Vec3,
    color: Color,
) {
    // 1. 尾翼 (Spoiler) - 支柱（左右）+ 翼板
    let strut_h = 0.3;
    let spoiler_z = chassis_size.z / 2.0 - 0.2;
    let spoiler_y = chassis_size.y / 2.0 + strut_h / 2.0;

    for x in [-0.6, 0.6] {
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.1, strut_h, 0.1))),
            MeshMaterial3d(shared_mats.black_plastic.clone()),
            Transform::from_xyz(x, spoiler_y, spoiler_z),
            GlobalTransform::default(),
        ));
    }
    // 翼板
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.8, 0.05, 0.4))),
        MeshMaterial3d(shared_mats.black_plastic.clone()),
        Transform::from_xyz(0.0, chassis_size.y / 2.0 + strut_h, spoiler_z),
        GlobalTransform::default(),
    ));

    // 2. 底盤燈 (Underglow) - 使用車身顏色
    parent.spawn((
        PointLight {
            color,
            intensity: 100_000.0,
            range: 5.0,
            radius: 2.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, -0.5, 0.0),
        GlobalTransform::default(),
    ));

    // 3. 側裙霓虹條 (Side Neon Strips)
    let neon_mat = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * 5.0,
        ..default()
    });
    let neon_y = -chassis_size.y / 2.0 + 0.1;
    for x_sign in [-1.0_f32, 1.0] {
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 2.5))),
            MeshMaterial3d(neon_mat.clone()),
            Transform::from_xyz(x_sign * (chassis_size.x / 2.0 + 0.02), neon_y, 0.0),
            GlobalTransform::default(),
        ));
    }
}

/// 生成 NPC 車輛（使用共享材質）
#[allow(clippy::too_many_arguments)]
pub fn spawn_npc_vehicle(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    shared_mats: &VehicleMaterials,
    position: Vec3,
    rotation: Quat,
    vehicle_type: VehicleType,
    color: Color,
    waypoints: std::sync::Arc<Vec<Vec3>>,
    start_index: usize,
) {
    // 根據類型定義尺寸變數和組件
    let (chassis_size, wheel_offset_z, preset) = match vehicle_type {
        VehicleType::Car => (Vec3::new(2.0, 0.6, 4.0), 1.2, VehiclePreset::car()),
        VehicleType::Taxi => (Vec3::new(2.0, 0.6, 4.0), 1.2, VehiclePreset::taxi()),
        VehicleType::Bus => (Vec3::new(2.8, 1.2, 8.0), 2.5, VehiclePreset::bus()),
        VehicleType::Scooter => (Vec3::new(0.6, 0.4, 1.8), 0.6, VehiclePreset::scooter()),
    };

    // 主要實體 (Root) - 負責物理和邏輯，但不負責渲染主車身 (由子實體負責，或保留透明/基礎幾何?)
    // 為了簡單，我們讓 Root 只有 Collider 和 Logic，渲染全交給 children?
    // 或者 Root 是車身底盤。為了避免層級太深，Root 當作底盤中心。

    // 1. 生成 Root 實體
    commands
        .spawn((
            // 空間組件 (完整的 SpatialBundle 替代)
            Transform {
                translation: position + Vec3::new(0.0, 0.5, 0.0), // 稍微提高
                rotation,
                ..default()
            },
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // 物理組件
            Collider::cuboid(chassis_size.x / 2.0, 0.75, chassis_size.z / 2.0),
            RigidBody::KinematicPositionBased,
            VehiclePhysicsMode::Kinematic,
            CollisionGroups::new(
                COLLISION_GROUP_VEHICLE,
                COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
            ), // NPC 載具與角色、載具、靜態物碰撞
            // 遊戲邏輯組件
            preset.into_components(),
            VehicleHealth::for_vehicle_type(vehicle_type), // 車輛血量
            VehicleId::new(),                              // 穩定識別碼（用於存檔）
            VehicleModifications::default(),               // 改裝狀態（用於存檔）
            NpcVehicle {
                waypoints,
                current_wp_index: start_index,
                ..default()
            },
            Name::new(format!("NpcVehicle_{vehicle_type:?}")),
        ))
        .insert(TireDamage::default()) // 輪胎損壞狀態（分離插入避免 tuple 大小限制）
        .insert(BodyPartDamage::default()) // 車體部位損壞追蹤
        .with_children(|parent| {
            parent
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    VehicleVisualRoot,
                ))
                .with_children(|parent| {
                    spawn_vehicle_body(
                        parent,
                        meshes,
                        materials,
                        shared_mats,
                        chassis_size,
                        vehicle_type,
                        color,
                    );
                    spawn_vehicle_wheels(parent, meshes, shared_mats, chassis_size, wheel_offset_z);
                    spawn_vehicle_lights(
                        parent,
                        meshes,
                        shared_mats.headlight.clone(),
                        shared_mats.taillight.clone(),
                        chassis_size,
                    );
                    if vehicle_type == VehicleType::Car || vehicle_type == VehicleType::Taxi {
                        spawn_tuning_parts(
                            parent,
                            meshes,
                            materials,
                            shared_mats,
                            chassis_size,
                            color,
                        );
                    }
                });
        });
}

/// 道路人行道寬度（需與 world/setup.rs 一致）
const ROAD_SIDEWALK_WIDTH: f32 = 4.0;

/// 計算雙向車道中心偏移（以道路總寬度為基準）
fn lane_offset(total_width: f32) -> f32 {
    let drive_width = (total_width - ROAD_SIDEWALK_WIDTH * 2.0).max(0.0);
    drive_width * 0.25
}

/// 系統：初始化交通 (在 Setup 階段運行)
/// 使用共享材質資源以優化效能
/// 生成 8-10 台 NPC 車輛和紅綠燈
#[allow(clippy::too_many_lines)]
#[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
pub fn spawn_initial_traffic(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    shared_mats: Res<VehicleMaterials>,
) {
    // NPC 車輛路線 - 只走柏油路，避開徒步區
    // 可用道路：中華路 (X=75, 寬50m), 西寧南路 (X=-50), 成都路 (Z=50)
    // ★ 重要：
    //   1. 路線點必須在道路中心線左右偏移 (Lane Offset)
    //   2. 不同路線的車道必須錯開，避免重疊
    //   3. 車寬約 2.5m，車道間距至少 8m (主要道路為雙向各1車道)

    use std::sync::Arc;

    // === 道路座標參考（與 world/setup.rs 同步）===
    let lane_offset_main = lane_offset(W_MAIN);
    let lane_offset_secondary = lane_offset(W_SECONDARY);
    let lane_offset_zhonghua = lane_offset(W_ZHONGHUA);

    let z_chengdu_north = Z_CHENGDU - lane_offset_main;
    let z_chengdu_south = Z_CHENGDU + lane_offset_main;
    let z_hankou_north = Z_HANKOU - lane_offset_secondary;
    let z_hankou_south = Z_HANKOU + lane_offset_secondary;

    let x_zhonghua_east = X_ZHONGHUA + lane_offset_zhonghua;
    let x_zhonghua_west = X_ZHONGHUA - lane_offset_zhonghua;
    let x_zhonghua_mid_east = X_ZHONGHUA + lane_offset_zhonghua * 0.5;
    let x_zhonghua_mid_west = X_ZHONGHUA - lane_offset_zhonghua * 0.5;
    let x_xining_east = X_XINING + lane_offset_secondary;
    let x_xining_west = X_XINING - lane_offset_secondary;
    let x_kangding_east = X_KANGDING + lane_offset_main;
    let x_kangding_west = X_KANGDING - lane_offset_main;

    // 路線 A：外圈 (逆時針) - 走主要幹道外側
    let route_outer = Arc::new(vec![
        Vec3::new(x_xining_west, 0.0, z_chengdu_north), // 西南角
        Vec3::new(x_zhonghua_east, 0.0, z_chengdu_north), // 東南角
        Vec3::new(x_zhonghua_east, 0.0, z_hankou_south), // 東北角
        Vec3::new(x_xining_west, 0.0, z_hankou_south),  // 西北角
    ]);

    // 路線 B：內圈 (順時針) - 使用相反車道避免重疊
    let route_inner = Arc::new(vec![
        Vec3::new(x_zhonghua_west, 0.0, z_chengdu_south), // 東南角
        Vec3::new(x_xining_east, 0.0, z_chengdu_south),   // 西南角
        Vec3::new(x_xining_east, 0.0, z_hankou_north),    // 西北角
        Vec3::new(x_zhonghua_west, 0.0, z_hankou_north),  // 東北角
    ]);

    // 路線 C：中華路直線 (南北向) - 使用中間車道避免與外圈衝突
    let route_zhonghua = Arc::new(vec![
        Vec3::new(x_zhonghua_mid_east, 0.0, z_chengdu_south), // 南端
        Vec3::new(x_zhonghua_mid_east, 0.0, z_hankou_north),  // 北端
        Vec3::new(x_zhonghua_mid_west, 0.0, z_hankou_north),  // U 型轉彎
        Vec3::new(x_zhonghua_mid_west, 0.0, z_chengdu_south), // 南端
    ]);

    // 路線 D：成都路西段 (東西向) - 避開外圈主線
    let route_chengdu = Arc::new(vec![
        Vec3::new(X_KANGDING, 0.0, z_chengdu_north), // 西端
        Vec3::new(X_XINING, 0.0, z_chengdu_north),   // 東端
        Vec3::new(X_XINING, 0.0, z_chengdu_south),   // U 型轉彎
        Vec3::new(X_KANGDING, 0.0, z_chengdu_south), // 西端
    ]);

    // 路線 E：康定路直線 (南北向) - 新增西邊界車流
    let route_kangding = Arc::new(vec![
        Vec3::new(x_kangding_east, 0.0, z_chengdu_south), // 南端
        Vec3::new(x_kangding_east, 0.0, z_hankou_north),  // 北端
        Vec3::new(x_kangding_west, 0.0, z_hankou_north),  // U 型轉彎
        Vec3::new(x_kangding_west, 0.0, z_chengdu_south), // 南端
    ]);

    // 車輛顏色池
    let car_colors = [
        Color::srgb(0.8, 0.2, 0.2), // 紅色
        Color::srgb(0.2, 0.2, 0.8), // 藍色
        Color::srgb(0.9, 0.9, 0.9), // 白色
        Color::srgb(0.1, 0.1, 0.1), // 黑色
        Color::srgb(0.7, 0.7, 0.7), // 銀色
        Color::srgb(0.2, 0.6, 0.2), // 綠色
        Color::srgb(1.0, 0.5, 0.0), // 橙色
    ];

    // 生成配置 (位置, 類型, 顏色, 起始索引, 路徑)
    // ★ 減少車輛數量避免相撞，每條路線只放 1 台
    let spawn_configs = [
        // === 路線 A：外圈（逆時針）- 計程車 ===
        (
            route_outer[0],
            VehicleType::Taxi,
            Color::srgb(1.0, 0.8, 0.0),
            0,
            Arc::clone(&route_outer),
        ),
        // === 路線 B：內圈（順時針）- 公車 ===
        (
            route_inner[0],
            VehicleType::Bus,
            Color::srgb(0.2, 0.4, 0.8),
            0,
            Arc::clone(&route_inner),
        ),
        // === 路線 C：中華路（U 型迴轉）===
        (
            route_zhonghua[0],
            VehicleType::Car,
            car_colors[2],
            0,
            Arc::clone(&route_zhonghua),
        ),
        // === 路線 D：成都路西段（U 型迴轉）===
        (
            route_chengdu[0],
            VehicleType::Car,
            car_colors[3],
            0,
            Arc::clone(&route_chengdu),
        ),
        // === 路線 E：康定路（U 型迴轉）===
        (
            route_kangding[0],
            VehicleType::Car,
            car_colors[5],
            0,
            Arc::clone(&route_kangding),
        ),
    ];

    info!("🚗 生成 {} 台初始交通車輛", spawn_configs.len());

    for (i, (pos, v_type, color, start_idx, path)) in spawn_configs.iter().enumerate() {
        debug!("  - 生成車輛 #{}: {:?} 於 {:?}", i, v_type, pos);

        // 它的首個目標應該是它所在位置的下一個點
        let next_idx = (*start_idx as usize + 1) % path.len();

        // 計算初始朝向：面向下一個航點
        let next_pos = path[next_idx];
        let dir = (next_pos - *pos).normalize_or_zero();
        let initial_rotation = look_rotation_y_flat(dir);

        spawn_npc_vehicle(
            &mut commands,
            &mut meshes,
            &mut materials,
            &shared_mats,
            *pos,
            initial_rotation,
            *v_type,
            *color,
            Arc::clone(path),
            next_idx,
        );
    }

    // 紅綠燈由 spawn_world_traffic_lights 系統統一生成，不在此處重複

    // 紅綠燈視覺資源由 setup_traffic_lights 初始化
}

/// 生成可騎乘的機車
/// 台灣街頭最常見的交通工具 - 外觀類似 125cc 速克達
/// 使用共享材質以優化效能
pub fn spawn_scooter(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    shared_mats: &VehicleMaterials,
    position: Vec3,
    rotation: Quat,
    color: Color,
) {
    // 機車尺寸
    let body_length = 1.6;
    let body_width = 0.5;
    let seat_height = 0.8;

    // 車身材質（唯一需要按顏色創建的材質）
    let body_mat = create_body_material(materials, color, 0.6);

    commands
        .spawn((
            Transform {
                translation: position + Vec3::new(0.0, 0.4, 0.0),
                rotation,
                ..default()
            },
            GlobalTransform::default(),
            Visibility::default(),
            // 較小的碰撞體
            Collider::cuboid(body_width / 2.0, 0.5, body_length / 2.0),
            RigidBody::KinematicPositionBased,
            VehiclePhysicsMode::Kinematic,
            CollisionGroups::new(
                COLLISION_GROUP_VEHICLE,
                COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
            ), // 機車與角色、載具、靜態物碰撞
            VehiclePreset::scooter().into_components(),
            VehicleHealth::for_vehicle_type(VehicleType::Scooter), // 車輛血量
            TireDamage::default(),                                 // 輪胎損壞狀態
            VehicleId::new(),                                      // 穩定識別碼（用於存檔）
            VehicleModifications::default(),                       // 改裝狀態（用於存檔）
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    VehicleVisualRoot,
                ))
                .with_children(|parent| {
                    spawn_scooter_body_parts(
                        parent,
                        meshes,
                        shared_mats,
                        body_mat.clone(),
                        body_width,
                        body_length,
                        seat_height,
                    );
                    spawn_scooter_wheels_and_lights(
                        parent,
                        meshes,
                        shared_mats,
                        body_mat,
                        body_length,
                        seat_height,
                    );
                });
        });

    debug!("🛵 生成機車於 {:?}", position);
}
