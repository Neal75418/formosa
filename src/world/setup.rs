//! 世界場景建構（西門町街道、建築、裝飾）

#![allow(clippy::too_many_arguments)]

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
use crate::core::{COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE};
use crate::player::Player;
use crate::vehicle::{spawn_scooter, VehicleModifications, VehiclePreset};

// ============================================================================
// 本模組 (super::)
// ============================================================================
// 組件與類型 (只匯入實際使用的)
use super::{Moon, NeonSign, Sun, WorldMaterials, spawn_neon_sign};
// 建築系統
use super::buildings::spawn_rich_building;
// 角色生成
use super::characters::{spawn_cover_points, spawn_player_character};
// 常數定義
use super::constants::{
    BuildingTracker, BUILDING_ROAD_BUFFER, ROAD_Y,
    W_ALLEY, W_MAIN, W_PEDESTRIAN, W_SECONDARY, W_ZHONGHUA,
    X_HAN, X_KANGDING, X_XINING, X_ZHONGHUA,
    Z_CHENGDU, Z_EMEI, Z_HANKOU, Z_KUNMING, Z_WUCHANG,
};
// 道路系統
use super::roads::{spawn_road_segment, spawn_zebra_crossing, RoadType};
// 街道設施
use super::street_furniture::{
    spawn_graffiti_wall, spawn_lamppost, spawn_movie_billboard, spawn_parking_garage,
    spawn_trash_can, spawn_vending_machine,
};


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
    setup_roads(&mut commands, &mut meshes, &mut materials, &asset_server);
    setup_buildings(&mut commands, &mut meshes, &mut materials, &mut building_tracker);
    setup_player_and_vehicles(&mut commands, &mut meshes, &mut materials);
    setup_neon_signs(&mut commands, &mut meshes, &mut materials, &building_tracker);
    setup_street_furniture(&mut commands, &mut meshes, &mut materials);
    setup_zebra_crossings(&mut commands, &mut meshes, &world_mats);
    setup_special_elements(&mut commands, &mut meshes, &mut materials);

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
        (Vec3::new(110.0, 10.0, -15.0), Vec3::new(0.5, 20.0, 100.0)),   // 東（中華路外）
        (Vec3::new(-120.0, 10.0, -15.0), Vec3::new(0.5, 20.0, 100.0)),  // 西（康定路外）
        (Vec3::new(-10.0, 10.0, 65.0), Vec3::new(130.0, 20.0, 0.5)),    // 南（成都路外）
        (Vec3::new(-10.0, 10.0, -95.0), Vec3::new(130.0, 20.0, 0.5)),   // 北（漢口街外）
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

/// 道路材質與道路網格生成
fn setup_roads(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
) {
    // === 道路材質 (支援貼圖載入) ===
    // 嘗試載入貼圖，若載入失敗則使用純色 fallback

    // 柏油路材質
    let asphalt_texture: Handle<Image> = asset_server.load("textures/roads/asphalt.jpg");
    let road_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        base_color_texture: Some(asphalt_texture),
        perceptual_roughness: 0.7,
        ..default()
    });

    // 道路標線 (黃線) - 純色即可
    let line_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.8, 0.0), // 黃色
        unlit: true,
        ..default()
    });

    // 徒步區材質 (紅磚鋪石貼圖 - 西門町風格)
    let paving_texture: Handle<Image> = asset_server.load("textures/roads/paving.jpg");
    let pedestrian_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75, 0.55, 0.45), // 暖紅磚色 (貼圖調色)
        base_color_texture: Some(paving_texture),
        perceptual_roughness: 0.8,
        ..default()
    });

    // === 2. 生成完整西門町道路網格 ===
    // 參考真實 Google Maps 佈局

    // ========== 南北向道路 (車行道) ==========

    // 中華路 (東邊界) - 主幹道，貫穿南北
    spawn_road_segment(
        commands,
        meshes,
        materials,
        road_mat.clone(),
        line_mat.clone(),
        Vec3::new(X_ZHONGHUA, ROAD_Y, -15.0),
        W_ZHONGHUA,
        180.0,
        RoadType::Asphalt,
    );

    // 西寧南路 - 貫穿南北
    spawn_road_segment(
        commands,
        meshes,
        materials,
        road_mat.clone(),
        line_mat.clone(),
        Vec3::new(X_XINING, ROAD_Y, -15.0),
        W_SECONDARY,
        180.0,
        RoadType::Asphalt,
    );

    // 康定路 (西邊界) - 貫穿南北
    spawn_road_segment(
        commands,
        meshes,
        materials,
        road_mat.clone(),
        line_mat.clone(),
        Vec3::new(X_KANGDING, ROAD_Y, -15.0),
        W_MAIN,
        180.0,
        RoadType::Asphalt,
    );

    // ========== 南北向道路 (徒步區) ==========

    // 漢中街 - 徒步區主軸 (從武昌街到成都路)
    let hanzhong_len = Z_CHENGDU - Z_WUCHANG - W_PEDESTRIAN;
    let hanzhong_center_z = (Z_WUCHANG + Z_CHENGDU) / 2.0;
    spawn_road_segment(
        commands,
        meshes,
        materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(X_HAN, ROAD_Y + 0.15, hanzhong_center_z),
        W_PEDESTRIAN,
        hanzhong_len,
        RoadType::Pedestrian,
    );

    // ========== 東西向道路 (車行道) ==========

    // 漢口街 (北邊界) - 車行道
    spawn_road_segment(
        commands,
        meshes,
        materials,
        road_mat.clone(),
        line_mat.clone(),
        Vec3::new(-10.0, ROAD_Y, Z_HANKOU),
        200.0,
        W_SECONDARY,
        RoadType::Asphalt,
    );

    // 成都路 (南邊界) - 主幹道
    spawn_road_segment(
        commands,
        meshes,
        materials,
        road_mat.clone(),
        line_mat.clone(),
        Vec3::new(-10.0, ROAD_Y, Z_CHENGDU),
        200.0,
        W_MAIN,
        RoadType::Asphalt,
    );

    // ========== 東西向道路 (徒步區) ==========

    // 徒步區東西範圍：西寧南路東緣 到 中華路西緣
    let ped_west_edge = X_XINING + W_SECONDARY / 2.0; // 西寧東緣
    let ped_east_edge = X_ZHONGHUA - W_ZHONGHUA / 2.0; // 中華西緣
    let han_half_w = W_PEDESTRIAN / 2.0;

    // 分段計算 (避開漢中街)
    let west_len = (X_HAN - han_half_w) - ped_west_edge;
    let west_center = (ped_west_edge + X_HAN - han_half_w) / 2.0;
    let east_len = ped_east_edge - (X_HAN + han_half_w);
    let east_center = (X_HAN + han_half_w + ped_east_edge) / 2.0;

    // 武昌街二段 - 徒步區 (分東西兩段)
    spawn_road_segment(
        commands,
        meshes,
        materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(west_center, ROAD_Y + 0.15, Z_WUCHANG),
        west_len,
        W_PEDESTRIAN,
        RoadType::Pedestrian,
    );
    spawn_road_segment(
        commands,
        meshes,
        materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(east_center, ROAD_Y + 0.15, Z_WUCHANG),
        east_len,
        W_PEDESTRIAN,
        RoadType::Pedestrian,
    );

    // 昆明街 - 小巷 (分東西兩段，連接武昌與峨嵋)
    spawn_road_segment(
        commands,
        meshes,
        materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(west_center, ROAD_Y + 0.15, Z_KUNMING),
        west_len,
        W_ALLEY,
        RoadType::Pedestrian,
    );
    spawn_road_segment(
        commands,
        meshes,
        materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(east_center, ROAD_Y + 0.15, Z_KUNMING),
        east_len,
        W_ALLEY,
        RoadType::Pedestrian,
    );

    // 峨嵋街 - 徒步區 (分東西兩段)
    spawn_road_segment(
        commands,
        meshes,
        materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(west_center, ROAD_Y + 0.15, Z_EMEI),
        west_len,
        W_PEDESTRIAN,
        RoadType::Pedestrian,
    );
    spawn_road_segment(
        commands,
        meshes,
        materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(east_center, ROAD_Y + 0.15, Z_EMEI),
        east_len,
        W_PEDESTRIAN,
        RoadType::Pedestrian,
    );
}

/// 地標建築與商店生成
fn setup_buildings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    building_tracker: &mut BuildingTracker,
) {
    // === 3. 地標建築 (根據真實西門町位置) ===

    // 交叉路口建築（road1 = X 軸道路, road2 = Z 軸道路）
    let corner_buildings = [
        // 西寧南路沿線
        BuildingSpec { road1: RoadSide { center: X_XINING, width: W_SECONDARY, align: -1.0 }, road2: RoadSide { center: Z_EMEI, width: W_PEDESTRIAN, align: -1.0 }, width: 20.0, height: 28.0, depth: 15.0, name: "萬年大樓" },
        BuildingSpec { road1: RoadSide { center: X_XINING, width: W_SECONDARY, align: -1.0 }, road2: RoadSide { center: Z_WUCHANG, width: W_PEDESTRIAN, align: -1.0 }, width: 22.0, height: 24.0, depth: 22.0, name: "獅子林" },
        BuildingSpec { road1: RoadSide { center: X_XINING, width: W_SECONDARY, align: -1.0 }, road2: RoadSide { center: Z_KUNMING, width: W_ALLEY, align: -1.0 }, width: 23.0, height: 4.0, depth: 18.0, name: "電影公園" },
        BuildingSpec { road1: RoadSide { center: X_XINING, width: W_SECONDARY, align: 1.0 }, road2: RoadSide { center: Z_WUCHANG, width: W_PEDESTRIAN, align: 1.0 }, width: 28.0, height: 35.0, depth: 22.0, name: "Don Don Donki" },
        // 漢中街沿線
        BuildingSpec { road1: RoadSide { center: X_HAN, width: W_PEDESTRIAN, align: -1.0 }, road2: RoadSide { center: Z_EMEI, width: W_PEDESTRIAN, align: -1.0 }, width: 18.0, height: 20.0, depth: 16.0, name: "誠品西門" },
        BuildingSpec { road1: RoadSide { center: X_HAN, width: W_PEDESTRIAN, align: -1.0 }, road2: RoadSide { center: Z_WUCHANG, width: W_PEDESTRIAN, align: 1.0 }, width: 14.0, height: 18.0, depth: 14.0, name: "誠品武昌" },
        BuildingSpec { road1: RoadSide { center: X_HAN, width: W_PEDESTRIAN, align: 1.0 }, road2: RoadSide { center: Z_EMEI, width: W_PEDESTRIAN, align: -1.0 }, width: 12.0, height: 15.0, depth: 12.0, name: "Uniqlo" },
        BuildingSpec { road1: RoadSide { center: X_HAN, width: W_PEDESTRIAN, align: 1.0 }, road2: RoadSide { center: Z_CHENGDU, width: W_MAIN, align: -1.0 }, width: 14.0, height: 18.0, depth: 14.0, name: "H&M" },
        // 中華路沿線
        BuildingSpec { road1: RoadSide { center: X_ZHONGHUA, width: W_ZHONGHUA, align: -1.0 }, road2: RoadSide { center: Z_CHENGDU, width: W_MAIN, align: -1.0 }, width: 12.0, height: 8.0, depth: 12.0, name: "捷運6號出口" },
        BuildingSpec { road1: RoadSide { center: X_ZHONGHUA, width: W_ZHONGHUA, align: -1.0 }, road2: RoadSide { center: Z_CHENGDU, width: W_MAIN, align: 1.0 }, width: 22.0, height: 14.0, depth: 22.0, name: "西門紅樓" },
        BuildingSpec { road1: RoadSide { center: X_ZHONGHUA, width: W_ZHONGHUA, align: 1.0 }, road2: RoadSide { center: Z_CHENGDU, width: W_MAIN, align: -1.0 }, width: 16.0, height: 22.0, depth: 16.0, name: "錢櫃KTV" },
        BuildingSpec { road1: RoadSide { center: X_ZHONGHUA, width: W_ZHONGHUA, align: -1.0 }, road2: RoadSide { center: Z_WUCHANG, width: W_PEDESTRIAN, align: 1.0 }, width: 10.0, height: 8.0, depth: 10.0, name: "鴨肉扁" },
        BuildingSpec { road1: RoadSide { center: X_ZHONGHUA, width: W_ZHONGHUA, align: -1.0 }, road2: RoadSide { center: Z_EMEI, width: W_PEDESTRIAN, align: -1.0 }, width: 18.0, height: 28.0, depth: 16.0, name: "新光三越" },
        BuildingSpec { road1: RoadSide { center: X_ZHONGHUA, width: W_ZHONGHUA, align: 1.0 }, road2: RoadSide { center: Z_HANKOU, width: W_SECONDARY, align: 1.0 }, width: 20.0, height: 25.0, depth: 18.0, name: "遠東百貨" },
        BuildingSpec { road1: RoadSide { center: X_ZHONGHUA, width: W_ZHONGHUA, align: 1.0 }, road2: RoadSide { center: Z_WUCHANG, width: W_PEDESTRIAN, align: -1.0 }, width: 14.0, height: 20.0, depth: 12.0, name: "商業大樓A" },
        // 康定路沿線
        BuildingSpec { road1: RoadSide { center: X_KANGDING, width: W_MAIN, align: 1.0 }, road2: RoadSide { center: Z_HANKOU, width: W_SECONDARY, align: 1.0 }, width: 28.0, height: 12.0, depth: 23.0, name: "西門國小" },
        BuildingSpec { road1: RoadSide { center: X_KANGDING, width: W_MAIN, align: 1.0 }, road2: RoadSide { center: Z_EMEI, width: W_PEDESTRIAN, align: -1.0 }, width: 12.0, height: 10.0, depth: 12.0, name: "7-ELEVEN" },
        // 漢口街建築群
        BuildingSpec { road1: RoadSide { center: X_XINING, width: W_SECONDARY, align: 1.0 }, road2: RoadSide { center: Z_HANKOU, width: W_SECONDARY, align: 1.0 }, width: 10.0, height: 8.0, depth: 10.0, name: "全家便利" },
        BuildingSpec { road1: RoadSide { center: X_HAN, width: W_PEDESTRIAN, align: -1.0 }, road2: RoadSide { center: Z_HANKOU, width: W_SECONDARY, align: 1.0 }, width: 14.0, height: 12.0, depth: 12.0, name: "麥當勞" },
        BuildingSpec { road1: RoadSide { center: X_HAN, width: W_PEDESTRIAN, align: 1.0 }, road2: RoadSide { center: Z_HANKOU, width: W_SECONDARY, align: 1.0 }, width: 10.0, height: 10.0, depth: 10.0, name: "摩斯漢堡" },
        // 康定路南段
        BuildingSpec { road1: RoadSide { center: X_KANGDING, width: W_MAIN, align: 1.0 }, road2: RoadSide { center: Z_CHENGDU, width: W_MAIN, align: 1.0 }, width: 12.0, height: 10.0, depth: 12.0, name: "大創" },
        BuildingSpec { road1: RoadSide { center: X_KANGDING, width: W_MAIN, align: 1.0 }, road2: RoadSide { center: Z_CHENGDU, width: W_MAIN, align: -1.0 }, width: 14.0, height: 12.0, depth: 14.0, name: "彈珠台" },
    ];
    for spec in &corner_buildings {
        spawn_building_at_corner(commands, meshes, materials, building_tracker, spec);
    }

    // 道路沿線建築（位於兩條橫路之間的路側）
    spawn_building_at_linear(commands, meshes, materials, building_tracker, Z_CHENGDU, W_MAIN, -1.0, X_XINING, X_HAN, 8.0, 6.0, "阿宗麵線");
    spawn_building_at_linear(commands, meshes, materials, building_tracker, X_HAN, W_PEDESTRIAN, -1.0, Z_EMEI, Z_CHENGDU, 6.0, 6.0, "KFC");
    spawn_building_at_linear(commands, meshes, materials, building_tracker, Z_EMEI, W_PEDESTRIAN, 1.0, X_XINING, X_HAN, 5.0, 5.0, "小吃街");

    // 直接定位建築（電影街、補充店面等）
    let direct_buildings: &[(Vec3, f32, f32, f32, &str)] = &[
        (Vec3::new(X_ZHONGHUA - W_ZHONGHUA / 2.0 - 10.0, 15.0, 25.0), 16.0, 30.0, 14.0, "統一元氣館"),
        (Vec3::new(41.0, 16.0, -68.0), 22.0, 32.0, 18.0, "國賓影城"),
        (Vec3::new(36.0, 14.0, -34.0), 18.0, 28.0, 16.0, "樂聲影城"),
        (Vec3::new(59.0, 15.0, -62.0), 20.0, 30.0, 20.0, "日新威秀"),
        (Vec3::new(40.0, 10.0, -64.0), 18.0, 20.0, 15.0, "湯姆熊"),
        (Vec3::new(-20.0, 6.0, 33.0), 10.0, 12.0, 10.0, "肯德基"),
        (Vec3::new(14.0, 4.0, 33.0), 6.0, 8.0, 6.0, "50嵐"),
        (Vec3::new(26.0, 5.0, 33.0), 8.0, 10.0, 8.0, "夾娃娃機"),
        (Vec3::new(28.0, 7.0, -10.0), 10.0, 14.0, 10.0, "潮牌店"),
        (Vec3::new(40.0, 6.0, -10.0), 8.0, 12.0, 8.0, "古著店"),
        (Vec3::new(52.0, 7.5, 14.0), 12.0, 15.0, 12.0, "球鞋專賣"),
        (Vec3::new(20.0, 6.0, -17.0), 8.0, 12.0, 8.0, "刺青店"),
        (Vec3::new(30.0, 5.0, -17.0), 6.0, 10.0, 6.0, "潮流刺青"),
    ];
    for &(pos, width, height, depth, name) in direct_buildings {
        try_spawn_rich_building(commands, meshes, materials, building_tracker, pos, width, height, depth, name);
    }

    info!("🏢 已新增 {} 棟建築", corner_buildings.len() + 3 + direct_buildings.len());
}

/// 玩家、停車場、載具生成
fn setup_player_and_vehicles(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // === 6. 玩家與 NPC ===
    // 玩家生成：漢中街徒步區中央（開闘區域，避免被建築擋住視線）
    // 位置：漢中街與峨嵋街交叉口附近，四周較空曠
    let start_pos = Vec3::new(5.0, 0.0, -5.0); // Y=0 因為角色自帶高度

    // 使用人形角色生成函數
    spawn_player_character(
        commands,
        meshes,
        materials,
        start_pos,
        Player {
            speed: 8.0,
            rotation_speed: 3.0,
            ..default()
        },
    );

    // 峨嵋立體停車場 (康定路與峨嵋街交叉口)
    // 保持原位置 X=-75, Z=20，由移動大創來避免重疊
    spawn_parking_garage(
        commands,
        meshes,
        materials,
        Vec3::new(X_KANGDING + 25.0, 10.0, Z_EMEI + 20.0),
        22.0,
        22.0,
        32.0,
        "峨嵋停車場",
    );

    // === 7. 共享載具材質與機車停放區 ===
    // 初始化共享材質（效能優化：減少重複材質創建）
    let vehicle_mats = crate::vehicle::VehicleMaterials::new(materials);
    commands.insert_resource(vehicle_mats.clone());

    // 徒步區閒置車輛 - 只放置一台機車和一台汽車讓玩家使用
    // 漢中街徒步區旁（玩家起始點旁）
    spawn_scooter(
        commands,
        meshes,
        materials,
        &vehicle_mats,
        Vec3::new(12.0, 0.0, -8.0),
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        Color::srgb(0.9, 0.1, 0.1),
    ); // 紅色機車

    // 徒步區閒置汽車（漢中街，稍南）
    spawn_vehicle(
        commands,
        meshes,
        materials,
        Vec3::new(-8.0, 0.0, -15.0),
        VehiclePreset::car(),
        Color::srgb(0.2, 0.3, 0.6),
    ); // 深藍色汽車

    info!("🚗 已生成 1 台機車和 1 台汽車於徒步區");
}

/// 霓虹燈招牌生成
fn setup_neon_signs(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    building_tracker: &BuildingTracker,
) {
    // === 8. 霓虹燈招牌 ===
    // 西門町的靈魂 - 五光十色的霓虹燈
    // 座標計算公式: x = road1_center + align1 * (road1_width/2 + building_width/2)
    //              z = road2_center + align2 * (road2_width/2 + building_depth/2)

    // 萬年大樓 - 經典紅色招牌
    // 建築: X_XINING(-55) + (-1)*(12/2+20/2) = -71, Z_EMEI(0) + (-1)*(15/2+18/2) = -16.5
    // 招牌貼在南面（面向峨嵋街）: z + depth/2 = -16.5 + 9 = -7.5
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "萬年大樓",
        Vec3::new(-71.0, 20.0, -7.5), // 南面牆上
        Vec3::new(6.0, 1.5, 0.3),
        "萬年",
        NeonSign::flickering(Color::srgb(1.0, 0.2, 0.1), 10.0),
    );

    // 錢櫃 KTV - 粉紫色
    // 建築: X_ZHONGHUA(80) + 1*(40/2+16/2) = 108, Z_CHENGDU(50) + (-1)*(16/2+16/2) = 34
    // 招牌貼在西面（面向中華路）: x - width/2 = 108 - 8 = 100
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "錢櫃KTV",
        Vec3::new(100.0, 15.0, 34.0), // 西面牆上
        Vec3::new(5.0, 1.2, 0.3),
        "錢櫃KTV",
        NeonSign::flickering(Color::srgb(0.9, 0.3, 0.9), 8.0),
    );

    // 西門紅樓 - 溫暖黃光
    // 建築: X_ZHONGHUA(80) + (-1)*(40/2+22/2) = 49, Z_CHENGDU(50) + 1*(16/2+22/2) = 69
    // 招牌貼在北面（面向成都路）: z - depth/2 = 69 - 11 = 58
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "西門紅樓",
        Vec3::new(49.0, 10.0, 58.0), // 北面牆上
        Vec3::new(4.0, 1.0, 0.3),
        "紅樓",
        NeonSign::steady(Color::srgb(1.0, 0.8, 0.3), 6.0),
    );

    // 誠品西門 - 綠色霓虹
    // 建築: X_HAN(0) + (-1)*(15/2+20/2) = -17.5, Z_EMEI(0) + (-1)*(15/2+16/2) = -15.5
    // 招牌貼在東面（面向漢中街）: x + width/2 = -17.5 + 10 = -7.5
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "誠品西門",
        Vec3::new(-7.5, 14.0, -15.5), // 東面牆上
        Vec3::new(4.0, 1.0, 0.3),
        "誠品",
        NeonSign::steady(Color::srgb(0.2, 0.9, 0.4), 7.0),
    );

    // 阿宗麵線 - 橘紅色（美食招牌）
    // 建築位於成都路北側，西寧與漢中之間
    // 中心: x=(-55+0)/2=-27.5, z=50+(-1)*(16/2+6/2)=39
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "阿宗麵線",
        Vec3::new(-27.5, 5.0, 42.0), // 南面牆上 (z+3)
        Vec3::new(3.0, 0.8, 0.3),
        "阿宗麵線",
        NeonSign::flickering(Color::srgb(1.0, 0.5, 0.1), 8.0),
    );

    // 唐吉訶德 (Don Don Donki) - 藍色霓虹
    // 建築: X_XINING(-55) + 1*(12/2+28/2) = -35, Z_WUCHANG(-50) + 1*(15/2+22/2) = -31.5
    // 招牌貼在南面: z + depth/2 = -31.5 + 11 = -20.5
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "Don Don Donki",
        Vec3::new(-35.0, 25.0, -20.5), // 南面牆上
        Vec3::new(5.0, 1.2, 0.3),
        "Donki",
        NeonSign::flickering(Color::srgb(0.2, 0.5, 1.0), 9.0),
    );

    // Uniqlo - 紅色霓虹
    // 建築: X_HAN(0) + 1*(15/2+15/2) = 15, Z_EMEI(0) + (-1)*(15/2+12/2) = -13.5
    // 招牌貼在西面（面向漢中街）: x - width/2 = 15 - 7.5 = 7.5
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "Uniqlo",
        Vec3::new(7.5, 9.0, -13.5), // 西面牆上
        Vec3::new(3.5, 1.0, 0.3),
        "UNIQLO",
        NeonSign::steady(Color::srgb(0.9, 0.1, 0.1), 8.0),
    );

    // 誠品武昌 - 綠色霓虹
    // 建築: X_HAN(0) + (-1)*(15/2+18/2) = -16.5, Z_WUCHANG(-50) + 1*(15/2+14/2) = -35.5
    // 招牌貼在東面: x + width/2 = -16.5 + 9 = -7.5
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "誠品武昌",
        Vec3::new(-7.5, 11.0, -35.5), // 東面牆上
        Vec3::new(4.0, 1.0, 0.3),
        "誠品",
        NeonSign::steady(Color::srgb(0.2, 0.9, 0.4), 7.0),
    );

    // 故障的老舊招牌 - 增加氛圍（獅子林附近）
    // 獅子林: X_XINING(-55) + (-1)*(12/2+22/2) = -72, Z_WUCHANG(-50) + (-1)*(15/2+18/2) = -66.5
    // 招牌貼在南面: z + depth/2 = -66.5 + 9 = -57.5
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "獅子林",
        Vec3::new(-72.0, 17.0, -57.5), // 南面牆上
        Vec3::new(3.0, 0.8, 0.3),
        "老店",
        NeonSign::broken(Color::srgb(0.8, 0.2, 0.3), 6.0),
    );

    // H&M - 紅白霓虹
    // 建築: X_HAN(0) + 1*(15/2+18/2) = 16.5, Z_CHENGDU(50) + (-1)*(16/2+14/2) = 35
    // 招牌貼在西面: x - width/2 = 16.5 - 9 = 7.5
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "H&M",
        Vec3::new(7.5, 13.0, 35.0), // 西面牆上
        Vec3::new(3.0, 1.5, 0.3),
        "H&M",
        NeonSign::steady(Color::srgb(1.0, 0.0, 0.0), 10.0),
    );

    // === Phase 7: 新增霓虹燈 ===

    // 國賓影城 - 紅色閃爍
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "國賓影城",
        Vec3::new(41.0, 25.0, -58.0), // 建築南面
        Vec3::new(5.0, 1.2, 0.3),
        "國賓",
        NeonSign::flickering(Color::srgb(1.0, 0.2, 0.2), 9.0),
    );

    // 樂聲影城 - 青色閃爍
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "樂聲影城",
        Vec3::new(36.0, 20.0, -26.0), // 建築南面
        Vec3::new(4.0, 1.0, 0.3),
        "樂聲",
        NeonSign::flickering(Color::srgb(0.2, 0.9, 0.9), 8.0),
    );

    // 麥當勞 M - 金色穩定
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "麥當勞",
        Vec3::new(-17.0, 8.0, -72.0), // 漢口街麥當勞上
        Vec3::new(2.5, 2.5, 0.3),
        "M",
        NeonSign::steady(Color::srgb(1.0, 0.8, 0.0), 12.0),
    );

    // 湯姆熊 - 橘色閃爍
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "湯姆熊",
        Vec3::new(40.0, 15.0, -64.0), // 湯姆熊遊樂場（配合建築位置更新）
        Vec3::new(4.5, 1.0, 0.3),
        "湯姆熊",
        NeonSign::flickering(Color::srgb(1.0, 0.5, 0.1), 7.0),
    );

    // 刺青街 TATTOO - 紫色故障風格
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "刺青店",
        Vec3::new(20.0, 8.0, -17.0), // 刺青店
        Vec3::new(3.5, 0.8, 0.3),
        "TATTOO",
        NeonSign::broken(Color::srgb(0.7, 0.2, 0.9), 8.0),
    );

    // 潮牌店 HYPE - 紅色穩定
    try_spawn_neon_sign(
        commands,
        meshes,
        materials,
        building_tracker,
        "潮牌店",
        Vec3::new(28.0, 10.0, -8.0), // 潮牌店
        Vec3::new(3.0, 0.8, 0.3),
        "HYPE",
        NeonSign::steady(Color::srgb(1.0, 0.1, 0.2), 9.0),
    );

    info!("✨ 已生成 16 個霓虹燈招牌");
}

/// 路燈、自動販賣機、垃圾桶生成
fn setup_street_furniture(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // === 9. 街道家具 - Phase 5 ===

    // 路燈 (優化後 - 間隔約 25-30 米，避免過於密集)
    // 從 40 盞減少到 28 盞，更符合現實密度
    let lamppost_positions = [
        // === 漢中街兩側 (徒步區主軸) - 每側 4 盞，間隔 25m ===
        // 東側 (X=8) - 錯開十字路口中心
        Vec3::new(X_HAN + 8.0, 0.0, -35.0),
        Vec3::new(X_HAN + 8.0, 0.0, -10.0),
        Vec3::new(X_HAN + 8.0, 0.0, 15.0),
        Vec3::new(X_HAN + 8.0, 0.0, 40.0),
        // 西側 (X=-8)
        Vec3::new(X_HAN - 8.0, 0.0, -35.0),
        Vec3::new(X_HAN - 8.0, 0.0, -10.0),
        Vec3::new(X_HAN - 8.0, 0.0, 15.0),
        Vec3::new(X_HAN - 8.0, 0.0, 40.0),
        // === 峨嵋街沿線 - 3 盞，避開漢中街交叉口 ===
        Vec3::new(-25.0, 0.0, Z_EMEI + 8.0),
        Vec3::new(25.0, 0.0, Z_EMEI + 8.0),
        Vec3::new(45.0, 0.0, Z_EMEI + 8.0),
        // === 中華路西側 (主幹道) - 4 盞，間隔 30m ===
        Vec3::new(X_ZHONGHUA - 25.0, 0.0, -60.0),
        Vec3::new(X_ZHONGHUA - 25.0, 0.0, -25.0),
        Vec3::new(X_ZHONGHUA - 25.0, 0.0, 10.0),
        Vec3::new(X_ZHONGHUA - 25.0, 0.0, 40.0),
        // === 西寧路東側 - 3 盞 (移除重複的西側) ===
        Vec3::new(X_XINING + 8.0, 0.0, -55.0),
        Vec3::new(X_XINING + 8.0, 0.0, -15.0),
        Vec3::new(X_XINING + 8.0, 0.0, 25.0),
        // === 漢口街沿線 (北邊界) - 3 盞 ===
        Vec3::new(-60.0, 0.0, Z_HANKOU + 8.0),
        Vec3::new(-20.0, 0.0, Z_HANKOU + 8.0),
        Vec3::new(35.0, 0.0, Z_HANKOU + 8.0),
        // === 成都路沿線 (南邊界) - 3 盞 ===
        Vec3::new(-60.0, 0.0, Z_CHENGDU - 10.0),
        Vec3::new(-20.0, 0.0, Z_CHENGDU - 10.0),
        Vec3::new(35.0, 0.0, Z_CHENGDU - 10.0),
        // === 康定路東側 (西邊界) - 3 盞 ===
        Vec3::new(X_KANGDING + 12.0, 0.0, -50.0),
        Vec3::new(X_KANGDING + 12.0, 0.0, -5.0),
        Vec3::new(X_KANGDING + 12.0, 0.0, 35.0),
    ];

    for pos in lamppost_positions {
        spawn_lamppost(commands, meshes, materials, pos);
    }
    info!("💡 已生成 {} 盞路燈", lamppost_positions.len());

    // 自動販賣機 (店鋪旁，避開馬路)
    let vending_positions = [
        (Vec3::new(12.0, 0.0, -15.0), 0.0, 0u8), // Uniqlo 旁 - 飲料（移離馬路）
        (Vec3::new(-70.0, 0.0, -15.0), 0.0, 0),  // 萬年大樓旁 - 飲料（移離馬路）
        (Vec3::new(-32.0, 0.0, -22.0), 0.0, 1),  // 唐吉訶德旁 - 零食（OK）
        (Vec3::new(42.0, 0.0, 36.0), 0.0, 0),    // 捷運站旁 - 飲料（OK）
        (Vec3::new(-75.0, 0.0, 12.0), std::f32::consts::PI, 2), // 7-11 旁 - 香菸（移離馬路）
    ];

    for (pos, rot, variant) in vending_positions {
        spawn_vending_machine(
            commands,
            meshes,
            materials,
            pos,
            rot,
            variant,
        );
    }
    info!("🥤 已生成 {} 台自動販賣機", vending_positions.len());

    // 垃圾桶 (人行道上，避開馬路)
    let trash_positions = [
        Vec3::new(8.0, 0.0, -10.0),  // 十字路口東北角人行道
        Vec3::new(-8.0, 0.0, -10.0), // 十字路口西北角人行道
        Vec3::new(8.0, 0.0, -55.0),  // 武昌街東側人行道
        Vec3::new(-8.0, 0.0, -55.0), // 武昌街西側人行道
        Vec3::new(-30.0, 0.0, 12.0), // 峨嵋街北側人行道
        Vec3::new(30.0, 0.0, 12.0),  // 峨嵋街北側人行道
    ];

    for pos in trash_positions {
        spawn_trash_can(commands, meshes, materials, pos);
    }
    info!("🗑️ 已生成 {} 個垃圾桶", trash_positions.len());
}

/// 斑馬線生成
fn setup_zebra_crossings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    world_mats: &WorldMaterials,
) {
    // === 10. 斑馬線系統 - Phase A ===
    // 主要交叉口配置 (center_x, center_z, road1_width, road2_width)
    // road1 = 南北向道路 (X), road2 = 東西向道路 (Z)

    // 使用共用斑馬線材質
    let zebra_mat = world_mats.zebra_white.clone();

    // 主要交叉口斑馬線
    let intersections = [
        // (X中心, Z中心, 南北道路寬, 東西道路寬, 名稱)
        (X_HAN, Z_EMEI, W_PEDESTRIAN, W_PEDESTRIAN, "漢中/峨嵋"),
        (X_HAN, Z_WUCHANG, W_PEDESTRIAN, W_PEDESTRIAN, "漢中/武昌"),
        (X_HAN, Z_CHENGDU, W_PEDESTRIAN, W_MAIN, "漢中/成都"),
        (X_XINING, Z_EMEI, W_SECONDARY, W_PEDESTRIAN, "西寧/峨嵋"),
        (X_XINING, Z_WUCHANG, W_SECONDARY, W_PEDESTRIAN, "西寧/武昌"),
        (X_XINING, Z_CHENGDU, W_SECONDARY, W_MAIN, "西寧/成都"),
    ];

    let mut zebra_count = 0;
    for (cx, cz, road_ns_w, road_ew_w, _name) in intersections {
        // 每個交叉口生成 4 條斑馬線 (東西南北各一)
        // 北側斑馬線 (東西向)
        spawn_zebra_crossing(
            commands,
            meshes,
            &zebra_mat,
            Vec3::new(cx, ROAD_Y + 0.01, cz - road_ew_w / 2.0 - 2.5),
            road_ns_w,
            true,
        );
        // 南側斑馬線 (東西向)
        spawn_zebra_crossing(
            commands,
            meshes,
            &zebra_mat,
            Vec3::new(cx, ROAD_Y + 0.01, cz + road_ew_w / 2.0 + 2.5),
            road_ns_w,
            true,
        );
        // 西側斑馬線 (南北向)
        spawn_zebra_crossing(
            commands,
            meshes,
            &zebra_mat,
            Vec3::new(cx - road_ns_w / 2.0 - 2.5, ROAD_Y + 0.01, cz),
            road_ew_w,
            false,
        );
        // 東側斑馬線 (南北向)
        spawn_zebra_crossing(
            commands,
            meshes,
            &zebra_mat,
            Vec3::new(cx + road_ns_w / 2.0 + 2.5, ROAD_Y + 0.01, cz),
            road_ew_w,
            false,
        );
        zebra_count += 4;
    }
    info!(
        "🦓 已生成 {} 條斑馬線於 {} 個交叉口",
        zebra_count,
        intersections.len()
    );
}

/// 電影看板、塗鴉牆、掩體點生成
fn setup_special_elements(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // === 11. 特色元素 - Phase 6 ===

    // 電影看板 (武昌街電影街)
    let billboard_configs = [
        (
            Vec3::new(25.0, 8.0, -58.0),
            Color::srgb(1.0, 0.3, 0.2),
            "動作片",
        ), // 紅色
        (
            Vec3::new(35.0, 8.0, -58.0),
            Color::srgb(0.2, 0.5, 1.0),
            "科幻片",
        ), // 藍色
        (
            Vec3::new(45.0, 8.0, -58.0),
            Color::srgb(1.0, 0.8, 0.2),
            "喜劇片",
        ), // 金色
        (
            Vec3::new(55.0, 8.0, -58.0),
            Color::srgb(0.6, 0.1, 0.8),
            "恐怖片",
        ), // 紫色
    ];

    for (pos, color, _genre) in billboard_configs {
        spawn_movie_billboard(commands, meshes, materials, pos, color);
    }
    info!("🎬 已生成 {} 個電影看板", billboard_configs.len());

    // 塗鴉牆（移到康定路西側，避開道路與建築）
    // 位置：康定路西緣再外推 2m，Z 位於峨嵋～成都之間
    let graffiti_pos = Vec3::new(X_KANGDING - W_MAIN / 2.0 - 7.5 - 2.0, 2.5, Z_EMEI + 18.0);
    spawn_graffiti_wall(commands, meshes, materials, graffiti_pos);

    // === 12. AI 掩體點生成 ===
    spawn_cover_points(commands);

    // NPC (由 spawn_initial_traffic 系統統一管理)
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 道路側面參數（用於交叉路口建築定位）
struct RoadSide {
    center: f32,
    width: f32,
    /// -1.0 = 低座標側（西/北）, 1.0 = 高座標側（東/南）
    align: f32,
}

/// 交叉路口建築規格
struct BuildingSpec {
    road1: RoadSide,
    road2: RoadSide,
    width: f32,
    height: f32,
    depth: f32,
    name: &'static str,
}

fn spawn_building_at_corner(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tracker: &mut BuildingTracker,
    spec: &BuildingSpec,
) {
    let x = spec.road1.center + spec.road1.align * (spec.road1.width / 2.0 + spec.width / 2.0 + BUILDING_ROAD_BUFFER);
    let z = spec.road2.center + spec.road2.align * (spec.road2.width / 2.0 + spec.depth / 2.0 + BUILDING_ROAD_BUFFER);
    let pos = Vec3::new(x, spec.height / 2.0, z);
    if tracker.try_record(pos, spec.width, spec.height, spec.depth, spec.name) {
        spawn_rich_building(commands, meshes, materials, pos, spec.width, spec.height, spec.depth, spec.name);
    }
}

fn spawn_building_at_linear(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tracker: &mut BuildingTracker,
    road_center: f32,
    road_width: f32,
    align: f32, // 主路 alignment
    start_cross: f32,
    end_cross: f32, // 橫路範圍
    width: f32,
    depth: f32,
    name: &str,
) {
    // 放在兩條橫路的中間
    let center_cross = (start_cross + end_cross) / 2.0;

    // 判斷是針對 X 軸還是 Z 軸的道路
    // 這裡假設 road_center 是 X 軸 (垂直路)，building 沿 Z 軸分佈
    // 若要通用化需要更多參數，這裡簡化為 "Vertical Road Side"

    // 如果是垂直路 (Hanzhong)，align 決定 X（含緩衝距離）
    let x = road_center + align * (road_width / 2.0 + width / 2.0 + BUILDING_ROAD_BUFFER);
    let z = center_cross;

    let height = 20.0;
    let pos = Vec3::new(x, height / 2.0, z);

    // 檢查重疊，若重疊則跳過生成
    if tracker.try_record(pos, width, height, depth, name) {
        spawn_rich_building(commands, meshes, materials, pos, width, height, depth, name);
    }
}

/// 嘗試生成建築物（帶重疊檢測）
fn try_spawn_rich_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tracker: &mut BuildingTracker,
    pos: Vec3,
    width: f32,
    height: f32,
    depth: f32,
    name: &str,
) {
    if tracker.try_record(pos, width, height, depth, name) {
        spawn_rich_building(commands, meshes, materials, pos, width, height, depth, name);
    }
}

/// 嘗試生成霓虹招牌（檢查對應建築是否存在）
fn try_spawn_neon_sign(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tracker: &BuildingTracker,
    building_name: &str, // 對應的建築名稱
    position: Vec3,
    size: Vec3,
    text: &str,
    neon_config: NeonSign,
) {
    // 檢查建築是否存在（精確匹配或包含匹配）
    if tracker.is_spawned(building_name) || tracker.is_spawned_contains(building_name) {
        spawn_neon_sign(
            commands,
            meshes,
            materials,
            position,
            size,
            text,
            neon_config,
        );
    } else {
        info!(
            "🚫 跳過招牌 \"{}\" (建築 \"{}\" 未生成)",
            text, building_name
        );
    }
}

fn spawn_vehicle(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    preset: VehiclePreset,
    color: Color,
) {
    use crate::vehicle::{
        VehicleHealth, VehicleId, VehiclePhysicsMode, VehicleType, VehicleVisualRoot,
    };

    let vehicle_type = preset.vehicle.vehicle_type;

    // 根據類型定義尺寸變數
    let (chassis_size, wheel_offset_z) = match vehicle_type {
        VehicleType::Car | VehicleType::Taxi => (Vec3::new(2.0, 0.6, 4.0), 1.2),
        VehicleType::Bus => (Vec3::new(2.8, 1.2, 8.0), 2.5),
        _ => (Vec3::new(2.0, 0.6, 4.0), 1.2),
    };

    commands
        .spawn((
            Transform::from_translation(pos + Vec3::new(0.0, 0.5, 0.0)),
            GlobalTransform::default(),
            Visibility::default(),
            Collider::cuboid(chassis_size.x / 2.0, 0.75, chassis_size.z / 2.0),
            VehicleHealth::for_vehicle_type(vehicle_type),        // 車輛血量
            VehicleId::new(),                                     // 穩定識別碼（用於存檔）
            VehicleModifications::default(),                      // 改裝狀態（用於存檔）
            preset.into_components(),                             // Vehicle + 7 個子元件
            RigidBody::KinematicPositionBased,                    // 閒置車輛預設 Kinematic
            VehiclePhysicsMode::Kinematic,
            CollisionGroups::new(
                COLLISION_GROUP_VEHICLE,
                COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
            ), // 載具與角色、載具、靜態物碰撞
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    VehicleVisualRoot,
                ))
                .with_children(|parent| {
                    // === 視覺模型構建 ===

                    // A. 底盤 (Chassis)
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(chassis_size))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: color,
                            perceptual_roughness: 0.3,
                            metallic: 0.5,
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                        GlobalTransform::default(),
                    ));

                    // B. 車艙 (Cabin) (更強健的判斷)
                    let is_bus = chassis_size.x > 2.5;
                    let cabin_size = if is_bus {
                        Vec3::new(2.7, 1.0, 7.5)
                    } else {
                        Vec3::new(1.8, 0.5, 2.0)
                    };

                    let cabin_y = chassis_size.y / 2.0 + cabin_size.y / 2.0;
                    let cabin_z_offset = if is_bus { 0.0 } else { -0.2 };

                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(cabin_size))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.1, 0.1, 0.1),
                            perceptual_roughness: 0.1,
                            metallic: 0.8,
                            ..default()
                        })),
                        Transform::from_xyz(0.0, cabin_y, cabin_z_offset),
                        GlobalTransform::default(),
                    ));

                    // C. 輪子 (Wheels)
                    let wheel_mesh = meshes.add(Cylinder::new(0.35, 0.3));
                    let wheel_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.0, 0.0, 0.0),
                        perceptual_roughness: 0.9,
                        ..default()
                    });

                    let wheel_y = -chassis_size.y / 2.0;
                    let wheel_x = chassis_size.x / 2.0;

                    let wheel_positions = [
                        Vec3::new(-wheel_x, wheel_y, -wheel_offset_z),
                        Vec3::new(wheel_x, wheel_y, -wheel_offset_z),
                        Vec3::new(-wheel_x, wheel_y, wheel_offset_z),
                        Vec3::new(wheel_x, wheel_y, wheel_offset_z),
                    ];

                    for pos in wheel_positions {
                        parent.spawn((
                            Mesh3d(wheel_mesh.clone()),
                            MeshMaterial3d(wheel_mat.clone()),
                            Transform::from_translation(pos)
                                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                            GlobalTransform::default(),
                        ));
                    }

                    // D. 車燈
                    let headlight_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 1.0, 1.0),
                        emissive: LinearRgba::new(20.0, 18.0, 10.0, 1.0),
                        ..default()
                    });

                    let light_x = chassis_size.x / 2.0 - 0.4;
                    let light_mesh = meshes.add(Cuboid::new(0.4, 0.2, 0.1));

                    let taillight_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 0.0, 0.0),
                        emissive: LinearRgba::new(15.0, 0.0, 0.0, 1.0),
                        ..default()
                    });

                    let lights: [(Handle<StandardMaterial>, f32); 2] = [
                        (headlight_mat, -chassis_size.z / 2.0 - 0.05),
                        (taillight_mat, chassis_size.z / 2.0 + 0.05),
                    ];
                    for (mat, z) in lights {
                        for x in [-light_x, light_x] {
                            parent.spawn((
                                Mesh3d(light_mesh.clone()),
                                MeshMaterial3d(mat.clone()),
                                Transform::from_xyz(x, 0.1, z),
                                GlobalTransform::default(),
                            ));
                        }
                    }
                });
        });
}
