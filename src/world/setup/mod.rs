//! 世界場景建構（西門町街道、建築、裝飾）
//!
//! 子模組：
//! - `roads_layout` - 道路網格佈局
//! - `buildings_layout` - 建築與霓虹燈配置
//! - `street_elements` - 街道家具、斑馬線、特殊元素
//! - `vehicles_spawn` - 玩家與車輛生成

mod buildings_layout;
mod roads_layout;
mod street_elements;
mod vehicles_spawn;

// ============================================================================
// 外部 Crate
// ============================================================================
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

// ============================================================================
// 其他模組 (crate::)
// ============================================================================
use crate::core::COLLISION_GROUP_STATIC;

// ============================================================================
// 本模組 (super::)
// ============================================================================
use super::constants::BuildingTracker;
use super::{Moon, Sun, WorldMaterials};

/// 場景建構入口 — 依序初始化各子系統
pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // === 初始化共用材質快取 ===
    let world_mats = WorldMaterials::new(&mut materials);
    commands.insert_resource(world_mats.clone());

    // === 初始化建築物重疊追蹤器 ===
    let mut building_tracker = BuildingTracker::new();

    setup_camera_and_lighting(&mut commands, &mut meshes, &mut materials);
    setup_ground(&mut commands, &mut meshes, &mut materials);
    roads_layout::setup_roads(&mut commands, &mut meshes, &mut materials, &asset_server);
    buildings_layout::setup_buildings(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
    );
    vehicles_spawn::setup_player_and_vehicles(&mut commands, &mut meshes, &mut materials);
    buildings_layout::setup_neon_signs(
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
    );
    street_elements::setup_street_furniture(&mut commands, &mut meshes, &mut materials);
    street_elements::setup_zebra_crossings(&mut commands, &mut meshes, &world_mats);
    street_elements::setup_special_elements(&mut commands, &mut meshes, &mut materials);

    info!("✅ 西門町 (重構版) 載入完成！");
}

/// 攝影機、光源、月亮設定
fn setup_camera_and_lighting(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // === 0. 攝影機與光照 ===
    // 遊戲主攝影機 (由 camera_follow 系統接管位置，這裡只需生成)
    commands.spawn((
        Name::new("Main Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Bloom 效果：針對霓虹燈和發光材質優化
        bevy::post_process::bloom::Bloom {
            intensity: 0.2, // 略高於 NATURAL (0.15)，讓霓虹燈更夢幻
            ..bevy::post_process::bloom::Bloom::NATURAL
        },
        crate::camera::GameCamera,
        // 陰影過濾：Hardware2x2 提供基礎柔和陰影，效能佳
        ShadowFilteringMethod::Hardware2x2,
        // 天氣系統：霧效果
        DistanceFog {
            color: Color::srgba(0.5, 0.5, 0.6, 0.0), // 初始：無霧
            falloff: FogFalloff::Exponential { density: 0.0 },
            ..default()
        },
    ));

    // 環境光
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 800.0,
        affects_lightmapped_meshes: true,
    });

    // 全域陰影品質設定 (2048x2048 解析度)
    commands.insert_resource(DirectionalLightShadowMap { size: 2048 });

    // 主光源 (太陽) - 含級聯陰影配置
    // 初始角度會由 sun_moon_rotation_system 根據 WorldTime 自動更新
    commands.spawn((
        Name::new("Sun"),
        Sun, // 標記組件，用於識別太陽實體
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        // 級聯陰影：近處銳利，遠處適當模糊
        CascadeShadowConfigBuilder {
            num_cascades: 3,               // 3 層級聯
            first_cascade_far_bound: 10.0, // 第一層 10m（最銳利）
            maximum_distance: 120.0,       // 最大陰影距離 120m
            ..default()
        }
        .build(),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.5, 0.0)),
    ));

    // 月亮 - 大型球體在遠處天空，夜間可見
    // 位置會由 sun_moon_rotation_system 根據時間更新（與太陽相對）
    let moon_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.95),       // 淡黃白色
        emissive: LinearRgba::new(0.8, 0.8, 0.9, 1.0), // 夜間發光
        unlit: true,                                   // 不受光照影響，自發光
        ..default()
    });

    commands.spawn((
        Name::new("Moon"),
        Moon {
            phase: 0.5, // 初始滿月
            emissive_intensity: 1.0,
        },
        Mesh3d(meshes.add(Sphere::new(15.0).mesh().uv(32, 18))), // 大球體
        MeshMaterial3d(moon_material),
        Transform::from_xyz(0.0, 200.0, -500.0), // 初始位置（會被系統更新）
    ));

    info!("📷 攝影機與光源已設置");
}

/// 地面生成
fn setup_ground(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // === 1. 地面 (擴大至完整西門町範圍) ===
    // 地圖範圍：X: -120 ~ +100, Z: -100 ~ +70
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 400.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.18, 0.20), // 深色瀝青地面基底
            perceptual_roughness: 0.85,
            ..default()
        })),
        Transform::from_xyz(-10.0, 0.0, -15.0), // 稍微偏移以覆蓋整個區域
        RigidBody::Fixed,
        Collider::cuboid(200.0, 0.1, 200.0),
    ));

    // === 2. 隱形邊界牆（防止玩家和載具離開地圖）===
    let walls: &[(Vec3, Vec3)] = &[
        // (位置, 半尺寸) — 東西南北各一面
        (Vec3::new(110.0, 10.0, -15.0), Vec3::new(0.5, 20.0, 100.0)), // 東（中華路外）
        (Vec3::new(-120.0, 10.0, -15.0), Vec3::new(0.5, 20.0, 100.0)), // 西（康定路外）
        (Vec3::new(-10.0, 10.0, 65.0), Vec3::new(130.0, 20.0, 0.5)),  // 南（成都路外）
        (Vec3::new(-10.0, 10.0, -95.0), Vec3::new(130.0, 20.0, 0.5)), // 北（漢口街外）
    ];

    for &(pos, half_ext) in walls {
        commands.spawn((
            Transform::from_translation(pos),
            RigidBody::Fixed,
            Collider::cuboid(half_ext.x, half_ext.y, half_ext.z),
            CollisionGroups::new(COLLISION_GROUP_STATIC, Group::ALL),
        ));
    }
}
