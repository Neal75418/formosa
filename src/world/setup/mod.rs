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
use super::{MapLayout, Moon, Sun, WorldMaterials};

/// 場景建構入口 — 依序初始化各子系統
pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    layout: Res<MapLayout>,
) {
    // === 初始化共用材質快取 ===
    let world_mats = WorldMaterials::new(&mut materials);
    commands.insert_resource(world_mats.clone());

    // === 初始化建築物重疊追蹤器 ===
    let mut building_tracker = BuildingTracker::new();

    setup_camera_and_lighting(&mut commands, &mut meshes, &mut materials);
    setup_ground(&mut commands, &mut meshes, &mut materials, &layout);
    roads_layout::setup_roads(
        &mut commands,
        &mut meshes,
        &mut materials,
        &asset_server,
        &layout,
    );
    buildings_layout::setup_buildings(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
    );
    vehicles_spawn::setup_player_and_vehicles(&mut commands, &mut meshes, &mut materials, &layout);
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
        // 距離霧：顏色與濃度由 update_sky_and_fog 依時間、天氣設定（天空盒由 attach_skybox 掛上）
        DistanceFog {
            color: Color::srgba(0.5, 0.5, 0.6, 0.0), // 第一次更新前先不起霧
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
    let moon_material = materials.add(moon_material());

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

/// 月亮材質：自發光、不受光照影響
fn moon_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.95),       // 淡黃白色
        emissive: LinearRgba::new(0.8, 0.8, 0.9, 1.0), // 夜間發光
        unlit: true,                                   // 不受光照影響，自發光
        fog_enabled: false, // 掛在約 500 m 外，開霧的話會被晴天的薄霧整個蓋掉
        alpha_mode: AlphaMode::Blend, // 改由 sun_moon_rotation_system 依天氣調 alpha 淡出
        ..default()
    }
}

/// 地面平板的邊長（只影響畫面，碰撞體另計）：
/// 邊緣要遠到被霧完全蓋住，否則街道盡頭會看到地面在天空前截斷
const GROUND_VISUAL_SIZE: f32 = 2000.0;

/// 地面生成
fn setup_ground(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    layout: &MapLayout,
) {
    // === 1. 地面：中心與碰撞體取自地圖資料 ===
    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(GROUND_VISUAL_SIZE, GROUND_VISUAL_SIZE),
            ),
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.18, 0.20), // 深色瀝青地面基底
            perceptual_roughness: 0.85,
            ..default()
        })),
        Transform::from_translation(layout.ground_center),
        RigidBody::Fixed,
        Collider::cuboid(
            layout.ground_collider_half_extents.x,
            layout.ground_collider_half_extents.y,
            layout.ground_collider_half_extents.z,
        ),
    ));

    // === 2. 隱形邊界牆（防止玩家和載具離開地圖）：東、西、南、北，由地圖資料推算 ===
    for wall in &layout.walls {
        commands.spawn((
            Transform::from_translation(wall.center),
            RigidBody::Fixed,
            Collider::cuboid(
                wall.half_extents.x,
                wall.half_extents.y,
                wall.half_extents.z,
            ),
            CollisionGroups::new(COLLISION_GROUP_STATIC, Group::ALL),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::CLEAR_FOG_VISIBILITY;

    #[test]
    fn moon_is_not_hidden_by_fog() {
        // 月亮在約 500 m 外，晴天的霧在那裡已經完全不透明
        assert!(!moon_material().fog_enabled);
    }

    #[test]
    fn moon_can_fade_out() {
        // 壞天氣時靠 base_color 的 alpha 淡出，材質要是半透明混合才有作用
        assert_eq!(moon_material().alpha_mode, AlphaMode::Blend);
    }

    #[test]
    fn ground_edge_hidden_by_fog_from_anywhere_on_map() {
        // 從可活動範圍的任何一點看出去，地面邊緣都在晴天能見度兩倍以外（霧已完全不透明）
        let layout = crate::world::ximending_layout();
        let bounds = &layout.bounds;
        let half = GROUND_VISUAL_SIZE / 2.0;
        let margin = [
            (layout.ground_center.x + half) - bounds.max_x,
            bounds.min_x - (layout.ground_center.x - half),
            (layout.ground_center.z + half) - bounds.max_z,
            bounds.min_z - (layout.ground_center.z - half),
        ]
        .into_iter()
        .fold(f32::INFINITY, f32::min);
        assert!(
            margin >= 2.0 * CLEAR_FOG_VISIBILITY,
            "地面邊緣離地圖邊界最近只有 {margin} m"
        );
    }
}
