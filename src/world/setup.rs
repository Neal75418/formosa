#![allow(clippy::too_many_arguments)]

use crate::ai::CoverPoint;
use crate::combat::{
    Armor, Damageable, Health, HitReaction, PlayerArm, PlayerHand, Weapon, WeaponInventory,
    WeaponStats,
};
use crate::core::{COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE};
use crate::player::{DodgeState, Player};
use crate::vehicle::{spawn_scooter, Vehicle, VehicleModifications};
use crate::world::{spawn_neon_sign, NeonSign};
use crate::world::{
    Building, BuildingType, BuildingWindow, Door, InteriorSpace, Moon, PlayerInteriorState,
    StreetFurniture, StreetFurnitureType, StreetLight, Sun, WorldMaterials,
};
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

// === 西門町地圖常數定義 (Game Units) ===
// 參考真實 Google Maps 西門町街道佈局
// 原點 (0,0) = 漢中街與峨嵋街交叉口
// 比例：1 game unit ≈ 1 公尺
pub const ROAD_Y: f32 = 0.05;

// 道路 X 軸位置 (南北向道路，由東到西)
pub const X_ZHONGHUA: f32 = 80.0; // 中華路 (東邊界，主幹道)
pub const X_HAN: f32 = 0.0; // 漢中街 (徒步區中軸)
pub const X_XINING: f32 = -55.0; // 西寧南路
pub const X_KANGDING: f32 = -100.0; // 康定路 (西邊界)

// 道路 Z 軸位置 (東西向道路，由北到南)
pub const Z_HANKOU: f32 = -80.0; // 漢口街 (北邊界)
pub const Z_WUCHANG: f32 = -50.0; // 武昌街二段 (徒步區北)
pub const Z_KUNMING: f32 = -25.0; // 昆明街 (武昌與峨嵋之間)
pub const Z_EMEI: f32 = 0.0; // 峨嵋街 (徒步區中軸)
pub const Z_CHENGDU: f32 = 50.0; // 成都路 (南邊界，主幹道)

// 道路寬度 (按真實比例)
pub const W_ZHONGHUA: f32 = 40.0; // 中華路 (6-8 車道)
pub const W_MAIN: f32 = 16.0; // 成都路, 康定路 (4 車道)
pub const W_SECONDARY: f32 = 12.0; // 西寧南路, 漢口街 (2-4 車道)
pub const W_PEDESTRIAN: f32 = 15.0; // 漢中街, 峨嵋街, 武昌街 (徒步區)
pub const W_ALLEY: f32 = 8.0; // 昆明街 (小巷)

/// 建築物重疊追蹤器 - 記錄已生成建築的包圍盒，防止重疊
struct BuildingTracker {
    bounds: Vec<(Vec3, Vec3, String)>, // (min, max, name)
}

impl BuildingTracker {
    fn new() -> Self {
        Self { bounds: Vec::new() }
    }

    /// 檢查新建築是否與已有建築重疊，若無重疊則記錄並返回 true
    fn try_record(&mut self, pos: Vec3, width: f32, _height: f32, depth: f32, name: &str) -> bool {
        // 只檢查 XZ 平面重疊（Y 軸高度不考慮，建築都在地面）
        let half_w = width / 2.0;
        let half_d = depth / 2.0;
        let min = Vec3::new(pos.x - half_w, 0.0, pos.z - half_d);
        let max = Vec3::new(pos.x + half_w, 1.0, pos.z + half_d);

        // 檢查與已有建築是否重疊
        for (existing_min, existing_max, existing_name) in &self.bounds {
            if Self::aabb_overlap_xz(min, max, *existing_min, *existing_max) {
                info!("🚫 跳過建築 \"{}\" (與 \"{}\" 重疊)", name, existing_name);
                return false;
            }
        }

        // 無重疊，記錄此建築
        self.bounds.push((min, max, name.to_string()));
        true
    }

    /// 檢查兩個 AABB 在 XZ 平面是否重疊
    fn aabb_overlap_xz(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3) -> bool {
        a_min.x < b_max.x && a_max.x > b_min.x && a_min.z < b_max.z && a_max.z > b_min.z
    }

    /// 檢查建築是否已成功生成（用於招牌檢查）
    fn is_spawned(&self, name: &str) -> bool {
        self.bounds.iter().any(|(_, _, n)| n == name)
    }

    /// 檢查建築名稱是否包含指定關鍵字（模糊匹配）
    fn is_spawned_contains(&self, keyword: &str) -> bool {
        self.bounds.iter().any(|(_, _, n)| n.contains(keyword))
    }
}

pub fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>, // 新增：用於載入貼圖和模型
) {
    // === 初始化共用材質快取 ===
    let world_mats = WorldMaterials::new(&mut materials);
    commands.insert_resource(world_mats.clone());

    // === 初始化建築物重疊追蹤器 ===
    let mut building_tracker = BuildingTracker::new();

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

    // 全域陰影品質設定 (4096x4096 解析度)
    commands.insert_resource(DirectionalLightShadowMap { size: 4096 });

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
            num_cascades: 4,               // 4 層級聯
            first_cascade_far_bound: 15.0, // 第一層 15m（最銳利）
            maximum_distance: 200.0,       // 最大陰影距離 200m
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
        &mut commands,
        &mut meshes,
        &mut materials,
        road_mat.clone(),
        line_mat.clone(),
        Vec3::new(X_ZHONGHUA, ROAD_Y, -15.0),
        W_ZHONGHUA,
        180.0,
        RoadType::Asphalt,
    );

    // 西寧南路 - 貫穿南北
    spawn_road_segment(
        &mut commands,
        &mut meshes,
        &mut materials,
        road_mat.clone(),
        line_mat.clone(),
        Vec3::new(X_XINING, ROAD_Y, -15.0),
        W_SECONDARY,
        180.0,
        RoadType::Asphalt,
    );

    // 康定路 (西邊界) - 貫穿南北
    spawn_road_segment(
        &mut commands,
        &mut meshes,
        &mut materials,
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
        &mut commands,
        &mut meshes,
        &mut materials,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        road_mat.clone(),
        line_mat.clone(),
        Vec3::new(-10.0, ROAD_Y, Z_HANKOU),
        200.0,
        W_SECONDARY,
        RoadType::Asphalt,
    );

    // 成都路 (南邊界) - 主幹道
    spawn_road_segment(
        &mut commands,
        &mut meshes,
        &mut materials,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(west_center, ROAD_Y + 0.15, Z_WUCHANG),
        west_len,
        W_PEDESTRIAN,
        RoadType::Pedestrian,
    );
    spawn_road_segment(
        &mut commands,
        &mut meshes,
        &mut materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(east_center, ROAD_Y + 0.15, Z_WUCHANG),
        east_len,
        W_PEDESTRIAN,
        RoadType::Pedestrian,
    );

    // 昆明街 - 小巷 (分東西兩段，連接武昌與峨嵋)
    spawn_road_segment(
        &mut commands,
        &mut meshes,
        &mut materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(west_center, ROAD_Y + 0.15, Z_KUNMING),
        west_len,
        W_ALLEY,
        RoadType::Pedestrian,
    );
    spawn_road_segment(
        &mut commands,
        &mut meshes,
        &mut materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(east_center, ROAD_Y + 0.15, Z_KUNMING),
        east_len,
        W_ALLEY,
        RoadType::Pedestrian,
    );

    // 峨嵋街 - 徒步區 (分東西兩段)
    spawn_road_segment(
        &mut commands,
        &mut meshes,
        &mut materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(west_center, ROAD_Y + 0.15, Z_EMEI),
        west_len,
        W_PEDESTRIAN,
        RoadType::Pedestrian,
    );
    spawn_road_segment(
        &mut commands,
        &mut meshes,
        &mut materials,
        pedestrian_mat.clone(),
        line_mat.clone(),
        Vec3::new(east_center, ROAD_Y + 0.15, Z_EMEI),
        east_len,
        W_PEDESTRIAN,
        RoadType::Pedestrian,
    );

    // === 3. 地標建築 (根據真實西門町位置) ===

    // 定義 helper closure 來計算貼合位置
    // x_align: -1 (路左/西), 1 (路右/東)
    // z_align: -1 (路北), 1 (路南)

    // ========== 西寧南路沿線建築 ==========

    // 萬年大樓 (西寧/峨嵋 NW 角) - 西門町最著名的購物中心之一
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_XINING,
        W_SECONDARY,
        -1.0,
        Z_EMEI,
        W_PEDESTRIAN,
        -1.0,
        20.0,
        28.0,
        15.0,
        "萬年大樓",
        Color::srgb(0.45, 0.45, 0.5),
    );

    // 獅子林大樓 (西寧/武昌 NW 角)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_XINING,
        W_SECONDARY,
        -1.0,
        Z_WUCHANG,
        W_PEDESTRIAN,
        -1.0,
        22.0,
        24.0,
        22.0,
        "獅子林",
        Color::srgb(0.5, 0.4, 0.3),
    );

    // 電影公園 (西寧/武昌與昆明之間，西側) - 縮小尺寸確保不侵入道路
    // 原本 25x20 剛好貼邊，改為 23x18 留 1m 間距
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_XINING,
        W_SECONDARY,
        -1.0,
        Z_KUNMING,
        W_ALLEY,
        -1.0,
        23.0,
        4.0,
        18.0,
        "電影公園",
        Color::srgb(0.25, 0.4, 0.25),
    );

    // 唐吉訶德 Don Quijote (西寧東側/武昌南側) - 黃色顯眼大樓
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_XINING,
        W_SECONDARY,
        1.0,
        Z_WUCHANG,
        W_PEDESTRIAN,
        1.0,
        28.0,
        35.0,
        22.0,
        "Don Don Donki",
        Color::srgb(1.0, 0.85, 0.0),
    );

    // ========== 漢中街沿線建築 ==========

    // 誠品西門店 (峨嵋北側, 漢中西側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_HAN,
        W_PEDESTRIAN,
        -1.0,
        Z_EMEI,
        W_PEDESTRIAN,
        -1.0,
        18.0,
        20.0,
        16.0,
        "誠品西門",
        Color::srgb(0.2, 0.35, 0.25),
    );

    // 誠品武昌店 (武昌南側, 漢中西側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_HAN,
        W_PEDESTRIAN,
        -1.0,
        Z_WUCHANG,
        W_PEDESTRIAN,
        1.0,
        14.0,
        18.0,
        14.0,
        "誠品武昌",
        Color::srgb(0.2, 0.35, 0.25),
    );

    // Uniqlo (漢中東側，峨嵋北側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_HAN,
        W_PEDESTRIAN,
        1.0,
        Z_EMEI,
        W_PEDESTRIAN,
        -1.0,
        12.0,
        15.0,
        12.0,
        "Uniqlo",
        Color::srgb(0.85, 0.15, 0.15),
    );

    // H&M (漢中東側，成都北側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_HAN,
        W_PEDESTRIAN,
        1.0,
        Z_CHENGDU,
        W_MAIN,
        -1.0,
        14.0,
        18.0,
        14.0,
        "H&M",
        Color::srgb(0.85, 0.85, 0.85),
    );

    // ========== 中華路沿線建築 ==========

    // 捷運西門站6號出口 (中華/成都 NW 角)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_ZHONGHUA,
        W_ZHONGHUA,
        -1.0,
        Z_CHENGDU,
        W_MAIN,
        -1.0,
        12.0,
        8.0,
        12.0,
        "捷運6號出口",
        Color::srgb(0.2, 0.35, 0.65),
    );

    // 西門紅樓 (中華路西側，成都路南側) - 歷史地標
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_ZHONGHUA,
        W_ZHONGHUA,
        -1.0,
        Z_CHENGDU,
        W_MAIN,
        1.0,
        22.0,
        14.0,
        22.0,
        "西門紅樓",
        Color::srgb(0.7, 0.22, 0.18),
    );

    // 錢櫃 KTV (中華路東側，成都路北側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_ZHONGHUA,
        W_ZHONGHUA,
        1.0,
        Z_CHENGDU,
        W_MAIN,
        -1.0,
        16.0,
        22.0,
        16.0,
        "錢櫃KTV",
        Color::srgb(0.75, 0.45, 0.55),
    );

    // 鴨肉扁 (中華路西側，武昌街南側) - 著名小吃
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_ZHONGHUA,
        W_ZHONGHUA,
        -1.0,
        Z_WUCHANG,
        W_PEDESTRIAN,
        1.0,
        10.0,
        8.0,
        10.0,
        "鴨肉扁",
        Color::srgb(0.85, 0.65, 0.35),
    );

    // 新光三越 (中華路西側，峨嵋北側) - 大型百貨
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_ZHONGHUA,
        W_ZHONGHUA,
        -1.0,
        Z_EMEI,
        W_PEDESTRIAN,
        -1.0,
        18.0,
        28.0,
        16.0,
        "新光三越",
        Color::srgb(0.85, 0.85, 0.9),
    );

    // 統一元氣館 (中華路西側，峨嵋與成都之間) - 辦公大樓
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(X_ZHONGHUA - W_ZHONGHUA / 2.0 - 10.0, 15.0, 25.0),
        16.0,
        30.0,
        14.0,
        "統一元氣館",
    );

    // 遠東百貨 (中華路東側，漢口街南側) - 大型商場
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_ZHONGHUA,
        W_ZHONGHUA,
        1.0,
        Z_HANKOU,
        W_SECONDARY,
        1.0,
        20.0,
        25.0,
        18.0,
        "遠東百貨",
        Color::srgb(0.3, 0.4, 0.6),
    );

    // 台北車站方向商業大樓 (中華路東側，武昌北側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_ZHONGHUA,
        W_ZHONGHUA,
        1.0,
        Z_WUCHANG,
        W_PEDESTRIAN,
        -1.0,
        14.0,
        20.0,
        12.0,
        "商業大樓A",
        Color::srgb(0.6, 0.6, 0.65),
    );

    // ========== 成都路沿線建築 ==========

    // 阿宗麵線 (成都路北側，西寧與漢中之間)
    spawn_building_at_linear(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Z_CHENGDU,
        W_MAIN,
        -1.0, // 成都路北側
        X_XINING,
        X_HAN, // 西寧到漢中之間
        8.0,
        6.0,
        "阿宗麵線",
        Color::srgb(0.9, 0.5, 0.2),
    );

    // ========== 康定路沿線建築 ==========

    // 西門國小 (康定路東側，漢口街南側) - 縮小尺寸確保不侵入道路
    // 原本 30x25 剛好貼邊，改為 28x23 留 1m 間距
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_KANGDING,
        W_MAIN,
        1.0,
        Z_HANKOU,
        W_SECONDARY,
        1.0,
        28.0,
        12.0,
        23.0,
        "西門國小",
        Color::srgb(0.7, 0.65, 0.55),
    );

    // 便利商店區 (康定路東側，峨嵋北側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_KANGDING,
        W_MAIN,
        1.0,
        Z_EMEI,
        W_PEDESTRIAN,
        -1.0,
        12.0,
        10.0,
        12.0,
        "7-ELEVEN",
        Color::srgb(0.2, 0.5, 0.35),
    );

    // === 4. 裝飾：便利商店與小店 (沿街填充) ===
    // 漢中街西側 - KFC
    spawn_building_at_linear(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_HAN,
        W_PEDESTRIAN,
        -1.0,
        Z_EMEI,
        Z_CHENGDU,
        6.0,
        6.0,
        "KFC",
        Color::srgb(0.8, 0.1, 0.1),
    );

    // 峨嵋街南側 - 小吃攤
    spawn_building_at_linear(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Z_EMEI,
        W_PEDESTRIAN,
        1.0,
        X_XINING,
        X_HAN,
        5.0,
        5.0,
        "小吃街",
        Color::srgb(0.85, 0.65, 0.4),
    );

    // ========== 電影街區域 (武昌街二段東段) - Phase 1 ==========

    // 國賓影城 (武昌北側，漢中東側偏東) - 往北移 2m 避免海報進入道路
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(41.0, 16.0, -68.0),
        22.0,
        32.0,
        18.0,
        "國賓影城",
    );

    // 樂聲影城 (武昌南側)
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(36.0, 14.0, -34.0),
        18.0,
        28.0,
        16.0,
        "樂聲影城",
    );

    // 日新威秀 (更東邊) - 確保不侵入漢口街 (南邊界 Z=-74)
    // 建築深度 20，所以 Z 中心需 >= -74 + 10 + 2 = -62
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(59.0, 15.0, -62.0),
        20.0,
        30.0,
        20.0,
        "日新威秀",
    );

    // ========== 漢口街建築群 - Phase 2 ==========

    // 全家便利商店 (漢口街南側，西寧東側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_XINING,
        W_SECONDARY,
        1.0,
        Z_HANKOU,
        W_SECONDARY,
        1.0,
        10.0,
        8.0,
        10.0,
        "全家便利",
        Color::srgb(0.2, 0.5, 0.4),
    );

    // 麥當勞 (漢口街南側，漢中西側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_HAN,
        W_PEDESTRIAN,
        -1.0,
        Z_HANKOU,
        W_SECONDARY,
        1.0,
        14.0,
        12.0,
        12.0,
        "麥當勞",
        Color::srgb(0.95, 0.75, 0.1),
    );

    // 摩斯漢堡 (漢口街南側，漢中東側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_HAN,
        W_PEDESTRIAN,
        1.0,
        Z_HANKOU,
        W_SECONDARY,
        1.0,
        10.0,
        10.0,
        10.0,
        "摩斯漢堡",
        Color::srgb(0.8, 0.2, 0.2),
    );

    // 湯姆熊遊戲中心 (漢口街南側偏東) - 確保不侵入漢口街 (南邊界 Z=-74)
    // 建築深度 15，所以 Z 中心需 >= -74 + 7.5 + 2 = -64.5，取 -64
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(40.0, 10.0, -64.0),
        18.0,
        20.0,
        15.0,
        "湯姆熊",
    );

    // ========== 成都路與峨嵋街補充 - Phase 3 ==========
    // 成都路 Z=50, 寬度16, 北側邊界 Z=42
    // 峨嵋街 Z=0, 寬度15, 北側邊界 Z=-7.5, 南側邊界 Z=7.5

    // 肯德基 (成都路北側，西寧東側偏東) - 移到 Z=33 避開道路
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(-20.0, 6.0, 33.0),
        10.0,
        12.0,
        10.0,
        "肯德基",
    );

    // 50嵐飲料 (成都路北側，漢中東側偏東) - 移到 Z=33
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(14.0, 4.0, 33.0),
        6.0,
        8.0,
        6.0,
        "50嵐",
    );

    // 夾娃娃機店 (成都路北側) - 移到 Z=33
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(26.0, 5.0, 33.0),
        8.0,
        10.0,
        8.0,
        "夾娃娃機",
    );

    // 潮牌店 (峨嵋街北側，漢中東側偏東) - Z=-8 OK (建築南邊 -8+5=-3 不在道路內)
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(28.0, 7.0, -10.0),
        10.0,
        14.0,
        10.0,
        "潮牌店",
    );

    // 古著店 (峨嵋街北側) - 移到 Z=-10
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(40.0, 6.0, -10.0),
        8.0,
        12.0,
        8.0,
        "古著店",
    );

    // 球鞋專賣店 (峨嵋街南側偏東) - 移到 Z=14 避開道路
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(52.0, 7.5, 14.0),
        12.0,
        15.0,
        12.0,
        "球鞋專賣",
    );

    // ========== 刺青街特色區 - Phase 4 ==========

    // 刺青店 (昆明街南側)
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(20.0, 6.0, -17.0),
        8.0,
        12.0,
        8.0,
        "刺青店",
    );

    // 潮流刺青 (昆明街南側)
    try_spawn_rich_building(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        Vec3::new(30.0, 5.0, -17.0),
        6.0,
        10.0,
        6.0,
        "潮流刺青",
    );

    // 康定路南段補充
    // 大創 (康定路東側，成都路南側) - 移到這裡避免與峨嵋停車場重疊
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_KANGDING,
        W_MAIN,
        1.0,
        Z_CHENGDU,
        W_MAIN,
        1.0,
        12.0,
        10.0,
        12.0,
        "大創",
        Color::srgb(0.9, 0.4, 0.5),
    );

    // 彈珠台 (康定路東側，成都北側)
    spawn_building_at_corner(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut building_tracker,
        X_KANGDING,
        W_MAIN,
        1.0,
        Z_CHENGDU,
        W_MAIN,
        -1.0,
        14.0,
        12.0,
        14.0,
        "彈珠台",
        Color::srgb(0.8, 0.7, 0.2),
    );

    info!("🏢 已新增 21 棟建築填補空白區域");

    // === 6. 玩家與 NPC ===
    // 玩家生成：漢中街徒步區中央（開闘區域，避免被建築擋住視線）
    // 位置：漢中街與峨嵋街交叉口附近，四周較空曠
    let start_pos = Vec3::new(5.0, 0.0, -5.0); // Y=0 因為角色自帶高度

    // 使用人形角色生成函數
    spawn_player_character(
        &mut commands,
        &mut meshes,
        &mut materials,
        start_pos,
        Player {
            speed: 8.0,
            rotation_speed: 3.0,
            money: 5000,
            ..default()
        },
    );

    // 峨嵋立體停車場 (康定路與峨嵋街交叉口)
    // 保持原位置 X=-75, Z=20，由移動大創來避免重疊
    spawn_parking_garage(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(X_KANGDING + 25.0, 10.0, Z_EMEI + 20.0),
        22.0,
        22.0,
        32.0,
        "峨嵋停車場",
    );

    // === 7. 共享載具材質與機車停放區 ===
    // 初始化共享材質（效能優化：減少重複材質創建）
    let vehicle_mats = crate::vehicle::VehicleMaterials::new(&mut materials);
    commands.insert_resource(vehicle_mats.clone());

    // 徒步區閒置車輛 - 只放置一台機車和一台汽車讓玩家使用
    // 漢中街徒步區旁（玩家起始點旁）
    spawn_scooter(
        &mut commands,
        &mut meshes,
        &mut materials,
        &vehicle_mats,
        Vec3::new(12.0, 0.0, -8.0),
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        Color::srgb(0.9, 0.1, 0.1),
    ); // 紅色機車

    // 徒步區閒置汽車（漢中街，稍南）
    spawn_vehicle(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(-8.0, 0.0, -15.0),
        Vehicle::car(),
        Color::srgb(0.2, 0.3, 0.6),
    ); // 深藍色汽車

    info!("🚗 已生成 1 台機車和 1 台汽車於徒步區");

    // === 8. 霓虹燈招牌 ===
    // 西門町的靈魂 - 五光十色的霓虹燈
    // 座標計算公式: x = road1_center + align1 * (road1_width/2 + building_width/2)
    //              z = road2_center + align2 * (road2_width/2 + building_depth/2)

    // 萬年大樓 - 經典紅色招牌
    // 建築: X_XINING(-55) + (-1)*(12/2+20/2) = -71, Z_EMEI(0) + (-1)*(15/2+18/2) = -16.5
    // 招牌貼在南面（面向峨嵋街）: z + depth/2 = -16.5 + 9 = -7.5
    try_spawn_neon_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
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
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
        "H&M",
        Vec3::new(7.5, 13.0, 35.0), // 西面牆上
        Vec3::new(3.0, 1.5, 0.3),
        "H&M",
        NeonSign::steady(Color::srgb(1.0, 0.0, 0.0), 10.0),
    );

    // === Phase 7: 新增霓虹燈 ===

    // 國賓影城 - 紅色閃爍
    try_spawn_neon_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
        "國賓影城",
        Vec3::new(41.0, 25.0, -58.0), // 建築南面
        Vec3::new(5.0, 1.2, 0.3),
        "國賓",
        NeonSign::flickering(Color::srgb(1.0, 0.2, 0.2), 9.0),
    );

    // 樂聲影城 - 青色閃爍
    try_spawn_neon_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
        "樂聲影城",
        Vec3::new(36.0, 20.0, -26.0), // 建築南面
        Vec3::new(4.0, 1.0, 0.3),
        "樂聲",
        NeonSign::flickering(Color::srgb(0.2, 0.9, 0.9), 8.0),
    );

    // 麥當勞 M - 金色穩定
    try_spawn_neon_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
        "麥當勞",
        Vec3::new(-17.0, 8.0, -72.0), // 漢口街麥當勞上
        Vec3::new(2.5, 2.5, 0.3),
        "M",
        NeonSign::steady(Color::srgb(1.0, 0.8, 0.0), 12.0),
    );

    // 湯姆熊 - 橘色閃爍
    try_spawn_neon_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
        "湯姆熊",
        Vec3::new(40.0, 15.0, -64.0), // 湯姆熊遊樂場（配合建築位置更新）
        Vec3::new(4.5, 1.0, 0.3),
        "湯姆熊",
        NeonSign::flickering(Color::srgb(1.0, 0.5, 0.1), 7.0),
    );

    // 刺青街 TATTOO - 紫色故障風格
    try_spawn_neon_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
        "刺青店",
        Vec3::new(20.0, 8.0, -17.0), // 刺青店
        Vec3::new(3.5, 0.8, 0.3),
        "TATTOO",
        NeonSign::broken(Color::srgb(0.7, 0.2, 0.9), 8.0),
    );

    // 潮牌店 HYPE - 紅色穩定
    try_spawn_neon_sign(
        &mut commands,
        &mut meshes,
        &mut materials,
        &building_tracker,
        "潮牌店",
        Vec3::new(28.0, 10.0, -8.0), // 潮牌店
        Vec3::new(3.0, 0.8, 0.3),
        "HYPE",
        NeonSign::steady(Color::srgb(1.0, 0.1, 0.2), 9.0),
    );

    info!("✨ 已生成 16 個霓虹燈招牌");

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
        spawn_lamppost(&mut commands, &mut meshes, &mut materials, pos);
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
            &mut commands,
            &mut meshes,
            &mut materials,
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
        spawn_trash_can(&mut commands, &mut meshes, &mut materials, pos);
    }
    info!("🗑️ 已生成 {} 個垃圾桶", trash_positions.len());

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
            &mut commands,
            &mut meshes,
            &zebra_mat,
            Vec3::new(cx, ROAD_Y + 0.01, cz - road_ew_w / 2.0 - 2.5),
            road_ns_w,
            true,
        );
        // 南側斑馬線 (東西向)
        spawn_zebra_crossing(
            &mut commands,
            &mut meshes,
            &zebra_mat,
            Vec3::new(cx, ROAD_Y + 0.01, cz + road_ew_w / 2.0 + 2.5),
            road_ns_w,
            true,
        );
        // 西側斑馬線 (南北向)
        spawn_zebra_crossing(
            &mut commands,
            &mut meshes,
            &zebra_mat,
            Vec3::new(cx - road_ns_w / 2.0 - 2.5, ROAD_Y + 0.01, cz),
            road_ew_w,
            false,
        );
        // 東側斑馬線 (南北向)
        spawn_zebra_crossing(
            &mut commands,
            &mut meshes,
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
        spawn_movie_billboard(&mut commands, &mut meshes, &mut materials, pos, color);
    }
    info!("🎬 已生成 {} 個電影看板", billboard_configs.len());

    // 塗鴉牆（移到康定路西側，避開道路與建築）
    // 位置：康定路西緣再外推 2m，Z 位於峨嵋～成都之間
    let graffiti_pos = Vec3::new(X_KANGDING - W_MAIN / 2.0 - 7.5 - 2.0, 2.5, Z_EMEI + 18.0);
    spawn_graffiti_wall(&mut commands, &mut meshes, &mut materials, graffiti_pos);

    // === 12. AI 掩體點生成 ===
    spawn_cover_points(&mut commands);

    // NPC (由 spawn_initial_traffic 系統統一管理)

    info!("✅ 西門町 (重構版) 載入完成！");
}

// === 輔助函數實現 ===

/// 建立帶有平鋪 UV 的平面 Mesh
/// tile_size: 每個貼圖覆蓋的實際大小 (米)
fn create_tiled_plane(width: f32, height: f32, tile_size: f32) -> Mesh {
    use bevy::mesh::VertexAttributeValues; // Bevy 0.17: 移至 bevy_mesh

    // 計算 UV 縮放倍數
    let u_scale = width / tile_size;
    let v_scale = height / tile_size;

    // 從 Bevy 內建的 Plane 開始
    let mut mesh = Plane3d::default().mesh().size(width, height).build();

    // 修改 UV 座標以支援平鋪
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs.iter_mut() {
            uv[0] *= u_scale;
            uv[1] *= v_scale;
        }
    }

    mesh
}

// === 道路系統輔助結構 ===

#[derive(PartialEq)]
enum RoadType {
    Asphalt,    // 柏油車道 (有黃線，兩側有人行道)
    Pedestrian, // 徒步區 (紅磚鋪面，無車道線)
}

/// 道路佈局計算結果
struct RoadLayout {
    is_horizontal: bool,
    road_len: f32,
    total_width: f32,
}

impl RoadLayout {
    fn new(width_x: f32, width_z: f32) -> Self {
        let is_horizontal = width_x > width_z;
        let (road_len, total_width) = if is_horizontal {
            (width_x, width_z)
        } else {
            (width_z, width_x)
        };
        Self {
            is_horizontal,
            road_len,
            total_width,
        }
    }
}

/// 生成徒步區道路
fn spawn_pedestrian_road(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: Handle<StandardMaterial>,
    pos: Vec3,
    width_x: f32,
    width_z: f32,
) {
    let tile_size = 2.0;
    let tiled_mesh = create_tiled_plane(width_x, width_z, tile_size);

    commands.spawn((
        Mesh3d(meshes.add(tiled_mesh)),
        MeshMaterial3d(material),
        Transform::from_translation(pos),
        GlobalTransform::default(),
        Visibility::default(),
    ));
}

/// 生成車道標線
fn spawn_lane_markings(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    line_mat: Handle<StandardMaterial>,
    layout: &RoadLayout,
) {
    let line_width = 0.2;
    let line_gap = 0.15;

    let (lx, lz, gap_vec) = if layout.is_horizontal {
        (layout.road_len, line_width, Vec3::new(0.0, 0.0, line_gap))
    } else {
        (line_width, layout.road_len, Vec3::new(line_gap, 0.0, 0.0))
    };

    // 雙黃線 - 使用 Cuboid 確保正確的水平方向
    let line_height = 0.01; // 非常薄的高度
    for offset in [-gap_vec, gap_vec] {
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(lx, line_height, lz))),
            MeshMaterial3d(line_mat.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.01 + line_height / 2.0, 0.0) + offset),
            GlobalTransform::default(),
        ));
    }
}

/// 生成人行道
fn spawn_sidewalks(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    layout: &RoadLayout,
    drive_width: f32,
) {
    const SIDEWALK_WIDTH: f32 = 4.0;

    let sidewalk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.45, 0.4),
        perceptual_roughness: 0.85,
        ..default()
    });

    let (sw_x, sw_z) = if layout.is_horizontal {
        (layout.road_len, SIDEWALK_WIDTH)
    } else {
        (SIDEWALK_WIDTH, layout.road_len)
    };

    let offset = (drive_width / 2.0) + (SIDEWALK_WIDTH / 2.0);
    let offsets = if layout.is_horizontal {
        [Vec3::new(0.0, 0.25, offset), Vec3::new(0.0, 0.25, -offset)]
    } else {
        [Vec3::new(offset, 0.25, 0.0), Vec3::new(-offset, 0.25, 0.0)]
    };

    for sidewalk_offset in offsets {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(sw_x, sw_z))),
            MeshMaterial3d(sidewalk_mat.clone()),
            Transform::from_translation(pos + sidewalk_offset),
            GlobalTransform::default(),
            Visibility::default(),
        ));
    }
}

fn spawn_road_segment(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    material: Handle<StandardMaterial>,
    line_mat: Handle<StandardMaterial>,
    pos: Vec3,
    width_x: f32,
    width_z: f32,
    road_type: RoadType,
) {
    if road_type == RoadType::Pedestrian {
        spawn_pedestrian_road(commands, meshes, material, pos, width_x, width_z);
        return;
    }

    // 車行道 (Asphalt)
    let layout = RoadLayout::new(width_x, width_z);
    let sidewalk_width = 4.0;
    let drive_width = layout.total_width - sidewalk_width * 2.0;

    let (drive_x, drive_z) = if layout.is_horizontal {
        (layout.road_len, drive_width)
    } else {
        (drive_width, layout.road_len)
    };

    // 中央車道
    let asphalt_mesh = create_tiled_plane(drive_x, drive_z, 4.0);
    commands
        .spawn((
            Mesh3d(meshes.add(asphalt_mesh)),
            MeshMaterial3d(material),
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            spawn_lane_markings(parent, meshes, line_mat, &layout);
        });

    // 人行道
    spawn_sidewalks(commands, meshes, materials, pos, &layout, drive_width);
}

/// 建築物與道路之間的緩衝距離（公尺）
const BUILDING_ROAD_BUFFER: f32 = 1.5;

fn spawn_building_at_corner(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tracker: &mut BuildingTracker,
    road1_center: f32,
    road1_width: f32,
    align1: f32, // align: -1 (Low coord), 1 (High coord)
    road2_center: f32,
    road2_width: f32,
    align2: f32,
    width: f32,
    height: f32,
    depth: f32,
    name: &str,
    _color: Color, // 保留參數但標記為未使用（spawn_rich_building 會根據名稱決定顏色）
) {
    // 計算貼合座標（含緩衝距離，避免建築貼邊道路）
    // 如果 align1 = -1 (路左/西)，建築中心 x = 路中心 - 路寬/2 - 建築寬/2 - 緩衝
    // 如果 align1 = 1 (路右/東)，建築中心 x = 路中心 + 路寬/2 + 建築寬/2 + 緩衝
    let x = road1_center + align1 * (road1_width / 2.0 + width / 2.0 + BUILDING_ROAD_BUFFER);
    let z = road2_center + align2 * (road2_width / 2.0 + depth / 2.0 + BUILDING_ROAD_BUFFER);

    let pos = Vec3::new(x, height / 2.0, z);

    // 檢查重疊，若重疊則跳過生成
    if tracker.try_record(pos, width, height, depth, name) {
        spawn_rich_building(commands, meshes, materials, pos, width, height, depth, name);
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
    _color: Color, // 保留參數但標記為未使用
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
    v_type: Vehicle,
    color: Color,
) {
    use crate::vehicle::{
        VehicleHealth, VehicleId, VehiclePhysicsMode, VehicleType, VehicleVisualRoot,
    };

    // 根據類型定義尺寸變數
    let (chassis_size, wheel_offset_z) = match v_type.vehicle_type {
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
            VehicleHealth::for_vehicle_type(v_type.vehicle_type), // 車輛血量
            VehicleId::new(),                                     // 穩定識別碼（用於存檔）
            VehicleModifications::default(),                      // 改裝狀態（用於存檔）
            v_type,                                               // Vehicle Component
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

                    let light_z = -chassis_size.z / 2.0 - 0.05;
                    let light_x = chassis_size.x / 2.0 - 0.4;

                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.4, 0.2, 0.1))),
                        MeshMaterial3d(headlight_mat.clone()),
                        Transform::from_xyz(-light_x, 0.1, light_z),
                        GlobalTransform::default(),
                    ));
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.4, 0.2, 0.1))),
                        MeshMaterial3d(headlight_mat),
                        Transform::from_xyz(light_x, 0.1, light_z),
                        GlobalTransform::default(),
                    ));

                    let taillight_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 0.0, 0.0),
                        emissive: LinearRgba::new(15.0, 0.0, 0.0, 1.0),
                        ..default()
                    });
                    let tail_z = chassis_size.z / 2.0 + 0.05;

                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.4, 0.2, 0.1))),
                        MeshMaterial3d(taillight_mat.clone()),
                        Transform::from_xyz(-light_x, 0.1, tail_z),
                        GlobalTransform::default(),
                    ));
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.4, 0.2, 0.1))),
                        MeshMaterial3d(taillight_mat),
                        Transform::from_xyz(light_x, 0.1, tail_z),
                        GlobalTransform::default(),
                    ));
                });
        });
}

/// 建築類型派發（根據名稱關鍵字匹配）
fn match_building_type(name: &str) -> BuildingStyle {
    // 定義匹配規則：(關鍵字列表, 建築風格)
    const PATTERNS: &[(&[&str], BuildingStyle)] = &[
        (&["萬年", "Wannien"], BuildingStyle::Wannien),
        (&["Donki", "唐吉"], BuildingStyle::Donki),
        (&["誠品", "Eslite"], BuildingStyle::Eslite),
        (&["H&M", "UNIQLO"], BuildingStyle::ModernGrid),
        (&["Cinema", "影城"], BuildingStyle::Cinema),
        (&["Hotel", "Just Sleep"], BuildingStyle::Hotel),
        (&["湯姆熊", "遊戲", "彈珠台"], BuildingStyle::GameCenter),
        (&["刺青", "TATTOO"], BuildingStyle::TattooShop),
        (&["麥當勞", "摩斯", "肯德基"], BuildingStyle::FastFood),
        (
            &["全家", "7-ELEVEN", "50嵐"],
            BuildingStyle::ConvenienceStore,
        ),
        (&["潮牌", "古著", "球鞋"], BuildingStyle::StreetWear),
        (&["夾娃娃"], BuildingStyle::ClawMachine),
        (&["大創"], BuildingStyle::Daiso),
    ];

    for (keywords, style) in PATTERNS {
        if keywords.iter().any(|kw| name.contains(kw)) {
            return *style;
        }
    }
    BuildingStyle::Generic
}

/// 建築風格枚舉
#[derive(Clone, Copy)]
enum BuildingStyle {
    Wannien,
    Donki,
    Eslite,
    ModernGrid,
    Cinema,
    Hotel,
    GameCenter,
    TattooShop,
    FastFood,
    ConvenienceStore,
    StreetWear,
    ClawMachine,
    Daiso,
    Generic,
}

/// 建築基礎參數 (消除重複程式碼用)
struct BuildingParams<'a> {
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &'a str,
}

/// 建築材質設定
struct BuildingMaterialConfig {
    base_color: Color,
    perceptual_roughness: f32,
    metallic: f32,
}

impl Default for BuildingMaterialConfig {
    fn default() -> Self {
        Self {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            perceptual_roughness: 0.8,
            metallic: 0.0,
        }
    }
}

/// 生成建築基礎結構（消除重複程式碼）
/// 返回 EntityCommands 以供後續添加子實體
fn spawn_building_base<'a>(
    cmd: &'a mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    params: &BuildingParams,
    config: BuildingMaterialConfig,
) -> EntityCommands<'a> {
    cmd.spawn((
        Mesh3d(meshes.add(Cuboid::new(params.w, params.h, params.d))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: config.base_color,
            perceptual_roughness: config.perceptual_roughness,
            metallic: config.metallic,
            ..default()
        })),
        Transform::from_translation(params.pos),
        GlobalTransform::default(),
        Visibility::default(),
        Collider::cuboid(params.w / 2.0, params.h / 2.0, params.d / 2.0),
        Building {
            name: params.name.to_string(),
            building_type: BuildingType::Shop,
        },
    ))
}

/// 通用建築生成函數：根據名稱派發到專屬邏輯
fn spawn_rich_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    width: f32,
    height: f32,
    depth: f32,
    name: &str,
) {
    match match_building_type(name) {
        BuildingStyle::Wannien => {
            spawn_wannien(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Donki => {
            spawn_donki(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Eslite => {
            spawn_eslite(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::ModernGrid => {
            spawn_modern_grid(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Cinema => {
            spawn_cinema(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Hotel => {
            spawn_hotel(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::GameCenter => {
            spawn_game_center(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::TattooShop => {
            spawn_tattoo_shop(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::FastFood => {
            spawn_fast_food(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::ConvenienceStore => {
            spawn_convenience_store(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::StreetWear => {
            spawn_streetwear_shop(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::ClawMachine => {
            spawn_claw_machine(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Daiso => {
            spawn_daiso(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Generic => {
            spawn_generic_building(commands, meshes, materials, pos, width, height, depth, name)
        }
    }
}

// === 1. 萬年大樓 (轉角圓柱風格) ===
fn spawn_wannien(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let corner_radius = w.min(d) * 0.4;

    cmd.spawn((
        // 主體部分 (稍微內縮，讓轉角突顯)
        Mesh3d(meshes.add(Cuboid::new(w * 0.9, h, d * 0.9))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.9, 0.95), // 米白
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_translation(pos),
        GlobalTransform::default(), // Bevy 0.17: 有子實體時需要 GlobalTransform
        Collider::cuboid(w / 2.0, h / 2.0, d / 2.0),
        Building {
            name: name.to_string(),
            building_type: BuildingType::Shop,
        },
    ))
    .with_children(|parent| {
        // 轉角圓柱 (Cylinder Corner)
        let cyl_h = h * 1.1; // 比樓高一點
        parent.spawn((
            Mesh3d(meshes.add(Cylinder::new(corner_radius, cyl_h))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.8, 0.9), // 稍微深一點
                ..default()
            })),
            // 放在轉角處 (假設是正向轉角)
            Transform::from_xyz(w / 2.0 - corner_radius, 0.0, d / 2.0 - corner_radius),
            GlobalTransform::default(),
        ));

        // 頂樓旋轉招牌 (Torus/Ring)
        let ring_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.0, 1.0), // Blue
            emissive: LinearRgba::new(0.0, 0.0, 1.0, 1.0) * 5.0,
            ..default()
        });
        parent.spawn((
            Mesh3d(meshes.add(Torus::new(corner_radius * 0.8, 0.5))),
            MeshMaterial3d(ring_mat),
            Transform::from_xyz(
                w / 2.0 - corner_radius,
                h / 2.0 + 2.0,
                d / 2.0 - corner_radius,
            ),
            GlobalTransform::default(),
        ));

        // 側面大型廣告看板
        let billboard_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 1.0), // White
            emissive: LinearRgba::new(1.0, 1.0, 1.0, 1.0) * 2.0,
            ..default()
        });
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.8, h * 0.6, 0.5))),
            MeshMaterial3d(billboard_mat),
            Transform::from_xyz(-0.5, 0.0, d / 2.0 * 0.9 + 0.3), // 貼在正面
            GlobalTransform::default(),
        ));
    });
}

// === 2. 唐吉訶德 (雜亂招牌風格) ===
fn spawn_donki(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: Color::srgb(1.0, 0.8, 0.0), // 唐吉鮮黃
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 生成大量隨機突出的招牌
        let sign_mat_1 = mats.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.0, 0.0),
            emissive: LinearRgba::new(0.1, 0.1, 0.1, 1.0),
            ..default()
        });
        let sign_mat_2 = mats.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.0, 1.0),
            emissive: LinearRgba::new(0.0, 0.0, 1.0, 1.0) * 3.0,
            ..default()
        });

        use rand::Rng;
        let mut rng = rand::rng();

        for i in 0..10 {
            let sx = rng.random_range(1.0..3.0);
            let sy = rng.random_range(1.0..3.0);
            // 隨機位置貼在表面
            let offset_x = rng.random_range(-w / 2.0..w / 2.0);
            let offset_y = rng.random_range(-h / 2.0..h / 2.0);
            let is_blue = rng.random_bool(0.3);
            // 每個招牌 Z 軸稍微不同，避免 Z-fighting
            let z_offset = 0.2 + (i as f32) * 0.05;

            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(sx, sy, 0.2))),
                MeshMaterial3d(if is_blue {
                    sign_mat_2.clone()
                } else {
                    sign_mat_1.clone()
                }),
                Transform::from_xyz(offset_x, offset_y, d / 2.0 + z_offset)
                    .with_rotation(Quat::from_rotation_z(rng.random_range(-0.2..0.2))), // 稍微歪一點
                GlobalTransform::default(),
            ));
        }

        // 頂部大企鵝招牌 (簡化為圓球)
        parent.spawn((
            Mesh3d(meshes.add(Sphere::new(2.5))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.0, 1.0),
                emissive: LinearRgba::new(0.0, 0.0, 1.0, 1.0) * 2.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 + 2.5, d / 2.0),
            GlobalTransform::default(),
        ));
    });
}

// === 3. 誠品 (植生牆風格) ===
fn spawn_eslite(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: Color::srgb(0.1, 0.3, 0.15), // 深綠
        perceptual_roughness: 0.2,               // 光滑玻璃感
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 木紋/植生凸起
        let wood_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.25, 0.1),
            ..default()
        });
        for i in 0..5 {
            let y_pos = -h / 2.0 + (i as f32) * (h / 5.0) + 2.0;
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(w + 0.2, 0.5, d + 0.2))), // 環繞一圈的木條
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(0.0, y_pos, 0.0),
                GlobalTransform::default(),
            ));
        }
    });
}

// === 4. 現代網格 (H&M / Uniqlo) ===
fn spawn_modern_grid(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let main_color = if name.contains("H&M") {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::srgb(0.9, 0.9, 0.9)
    };
    let accent_color = Color::srgb(1.0, 0.0, 0.0);

    cmd.spawn((
        Mesh3d(meshes.add(Cuboid::new(w * 0.95, h, d * 0.95))), // 內部發光芯
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: accent_color,
            emissive: LinearRgba::from(accent_color) * 2.0,
            ..default()
        })),
        Transform::from_translation(pos),
        GlobalTransform::default(),
        Visibility::default(),
        Collider::cuboid(w / 2.0, h / 2.0, d / 2.0),
        Building {
            name: name.to_string(),
            building_type: BuildingType::Shop,
        },
    ))
    .with_children(|parent| {
        // 外部格柵 (白色)
        let white_mat = mats.add(StandardMaterial {
            base_color: main_color,
            ..default()
        });

        let grid_count = 6;
        let step_x = w / grid_count as f32;

        // 垂直柱子
        for i in 0..=grid_count {
            let x_off = -w / 2.0 + i as f32 * step_x;
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.5, h, d + 0.5))), // 前後貫穿
                MeshMaterial3d(white_mat.clone()),
                Transform::from_xyz(x_off, 0.0, 0.0),
                GlobalTransform::default(),
            ));
        }

        // Logo 板
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(4.0, 4.0, 0.5))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: accent_color,
                emissive: LinearRgba::from(accent_color) * 4.0,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, d / 2.0 + 0.5),
            GlobalTransform::default(),
        ));
    });
}

// === 5. 電影院 (Cinema) ===
fn spawn_cinema(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: Color::srgb(0.1, 0.05, 0.1), // 暗色背景
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 電影海報看板
        let poster_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.1, 0.5), // 假裝是海報色
            emissive: LinearRgba::new(0.8, 0.1, 0.5, 1.0) * 2.0,
            ..default()
        });

        // 正面掛三個大海報
        let poster_w = w / 3.5;
        let poster_h = h * 0.6;
        for i in 0..3 {
            let x_offset = -w / 3.0 + (i as f32) * (w / 3.0);
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(poster_w, poster_h, 0.2))),
                MeshMaterial3d(poster_mat.clone()),
                Transform::from_xyz(x_offset, 0.0, d / 2.0 + 0.2),
                GlobalTransform::default(),
            ));
        }
    });
}

// === 6. 飯店 (Hotel) ===
fn spawn_hotel(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 陽台 (Balconies)
        let balcony_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 0.1),
            ..default()
        });
        let floor_height = 3.0;
        let floors = (h / floor_height) as i32;

        for i in 1..floors {
            let y = -h / 2.0 + (i as f32) * floor_height;
            // 橫向長陽台
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(w + 0.5, 0.2, d + 0.5))),
                MeshMaterial3d(balcony_mat.clone()),
                Transform::from_xyz(0.0, y, 0.0),
                GlobalTransform::default(),
            ));
        }

        // 頂樓招牌 (HOTEL)
        let sign_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.5, 0.0),
            emissive: LinearRgba::new(1.0, 0.5, 0.0, 1.0) * 5.0,
            ..default()
        });
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.8, 1.5, 0.5))),
            MeshMaterial3d(sign_mat),
            Transform::from_xyz(0.0, h / 2.0 + 1.0, 0.0),
            GlobalTransform::default(),
        ));
    });
}

// === 7. 通用建築 (形狀變體) ===
fn spawn_generic_building(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    use rand::Rng;
    let mut rng = rand::rng();
    let shape_type = rng.random_range(0..3); // 0: Box, 1: Stepped, 2: Twin

    let color = Color::srgb(
        rng.random_range(0.2..0.5),
        rng.random_range(0.2..0.5),
        rng.random_range(0.2..0.5),
    );
    let main_mat = mats.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.8,
        ..default()
    });

    match shape_type {
        1 => {
            // Stepped (階梯狀)
            // 下層大，上層小
            cmd.spawn((
                Mesh3d(meshes.add(Cuboid::new(w, h * 0.6, d))),
                MeshMaterial3d(main_mat.clone()),
                Transform::from_translation(pos - Vec3::new(0.0, h * 0.2, 0.0)),
                GlobalTransform::default(),
                Collider::cuboid(w / 2.0, h * 0.3, d / 2.0),
                Building {
                    name: name.to_string(),
                    building_type: BuildingType::Shop,
                },
            ))
            .with_children(|parent| {
                // 上層
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(w * 0.6, h * 0.4, d * 0.6))),
                    MeshMaterial3d(main_mat),
                    Transform::from_xyz(0.0, h * 0.5, 0.0),
                    GlobalTransform::default(),
                ));
            });
        }
        2 => {
            // Twin Towers (雙塔)
            cmd.spawn((
                // 基座
                Mesh3d(meshes.add(Cuboid::new(w, h * 0.3, d))),
                MeshMaterial3d(main_mat.clone()),
                Transform::from_translation(pos - Vec3::new(0.0, h * 0.35, 0.0)),
                GlobalTransform::default(),
                Collider::cuboid(w / 2.0, h * 0.15, d / 2.0),
                Building {
                    name: name.to_string(),
                    building_type: BuildingType::Shop,
                },
            ))
            .with_children(|parent| {
                // 左塔
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(w * 0.3, h * 0.7, d * 0.3))),
                    MeshMaterial3d(main_mat.clone()),
                    Transform::from_xyz(-w * 0.25, h * 0.5, 0.0),
                    GlobalTransform::default(),
                ));
                // 右塔
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(w * 0.3, h * 0.7, d * 0.3))),
                    MeshMaterial3d(main_mat),
                    Transform::from_xyz(w * 0.25, h * 0.5, 0.0),
                    GlobalTransform::default(),
                ));
            });
        }
        _ => {
            // Standard Box with Details
            cmd.spawn((
                Mesh3d(meshes.add(Cuboid::new(w, h, d))),
                MeshMaterial3d(main_mat.clone()),
                Transform::from_translation(pos),
                GlobalTransform::default(),
                Collider::cuboid(w / 2.0, h / 2.0, d / 2.0),
                Building {
                    name: name.to_string(),
                    building_type: BuildingType::Shop,
                },
            ))
            .with_children(|parent| {
                // 隨機窗戶帶（加入日夜系統）
                let win_mat = mats.add(StandardMaterial {
                    base_color: Color::srgb(0.8, 0.8, 0.6), // 窗戶基礎色（關燈時）
                    ..default()
                });
                for _ in 0..3 {
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(w + 0.1, 1.0, d + 0.1))),
                        MeshMaterial3d(win_mat.clone()),
                        Transform::from_xyz(0.0, rng.random_range(-h / 2.0..h / 2.0), 0.0),
                        BuildingWindow::shop(), // 商店窗戶
                        GlobalTransform::default(),
                    ));
                }
            });
        }
    }
}

// === 8. 遊戲中心 (Game Center) - 湯姆熊/彈珠台 ===
fn spawn_game_center(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let is_pachinko = name.contains("彈珠台");
    let main_color = if is_pachinko {
        Color::srgb(0.8, 0.7, 0.2) // 金黃色 (彈珠台風格)
    } else {
        Color::srgb(1.0, 0.4, 0.1) // 橘色 (湯姆熊風格)
    };

    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: main_color,
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 閃爍的霓虹燈條
        let neon_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0),
            emissive: LinearRgba::new(1.0, 0.0, 1.0, 1.0) * 5.0,
            ..default()
        });

        for i in 0..4 {
            let y_off = -h / 2.0 + (i as f32 + 1.0) * (h / 5.0);
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(w + 0.1, 0.3, d + 0.1))),
                MeshMaterial3d(neon_mat.clone()),
                Transform::from_xyz(0.0, y_off, 0.0),
                GlobalTransform::default(),
            ));
        }

        // 大型招牌
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.8, 2.0, 0.5))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(1.0, 0.8, 0.3, 1.0) * 4.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 + 1.5, d / 2.0),
            GlobalTransform::default(),
        ));
    });
}

// === 9. 刺青店 (Tattoo Shop) ===
fn spawn_tattoo_shop(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: Color::srgb(0.1, 0.08, 0.1), // 深紫黑色
        perceptual_roughness: 0.3,
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 紫色霓虹燈框
        let purple_neon = mats.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.1, 0.9),
            emissive: LinearRgba::new(0.8, 0.1, 0.9, 1.0) * 6.0,
            ..default()
        });

        // 門框霓虹
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.2, h * 0.7, 0.2))),
            MeshMaterial3d(purple_neon.clone()),
            Transform::from_xyz(-w / 3.0, -h / 6.0, d / 2.0 + 0.2),
            GlobalTransform::default(),
        ));
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.2, h * 0.7, 0.2))),
            MeshMaterial3d(purple_neon.clone()),
            Transform::from_xyz(w / 3.0, -h / 6.0, d / 2.0 + 0.2),
            GlobalTransform::default(),
        ));
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.7, 0.2, 0.2))),
            MeshMaterial3d(purple_neon),
            Transform::from_xyz(0.0, h / 4.0, d / 2.0 + 0.2),
            GlobalTransform::default(),
        ));

        // 窗戶玻璃 (暗色)
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.6, h * 0.5, 0.1))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgba(0.1, 0.1, 0.15, 0.7),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, d / 2.0 + 0.25),
            GlobalTransform::default(),
        ));
    });
}

// === 10. 速食店 (Fast Food) - 麥當勞/摩斯/肯德基 ===
fn spawn_fast_food(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let (main_color, accent_color) = if name.contains("麥當勞") {
        (Color::srgb(0.95, 0.75, 0.1), Color::srgb(0.85, 0.1, 0.1)) // 金+紅
    } else if name.contains("摩斯") {
        (Color::srgb(0.8, 0.2, 0.2), Color::srgb(0.95, 0.9, 0.8)) // 紅+白
    } else {
        (Color::srgb(0.85, 0.15, 0.15), Color::srgb(0.95, 0.95, 0.95)) // 紅+白 (KFC)
    };

    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: main_color,
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 大型Logo區塊
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.5, w * 0.5, 0.5))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: accent_color,
                emissive: LinearRgba::from(accent_color) * 3.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 4.0, d / 2.0 + 0.3),
            GlobalTransform::default(),
        ));

        // 屋頂招牌
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.6, 1.5, 0.3))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: accent_color,
                emissive: LinearRgba::from(accent_color) * 2.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 + 1.0, 0.0),
            GlobalTransform::default(),
        ));
    });
}

// === 11. 便利商店 (Convenience Store) ===
fn spawn_convenience_store(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let main_color = if name.contains("全家") {
        Color::srgb(0.2, 0.5, 0.45) // 青綠
    } else if name.contains("50嵐") {
        Color::srgb(0.3, 0.6, 0.4) // 綠色
    } else {
        Color::srgb(0.2, 0.5, 0.35) // 7-11 綠
    };

    // 創建室內空間
    let door_pos = pos + Vec3::new(0.0, 1.25, d / 2.0 + 0.3);
    let interior_entity = cmd
        .spawn((
            Transform::from_translation(pos),
            GlobalTransform::default(),
            InteriorSpace::convenience_store(name, door_pos + Vec3::new(0.0, -1.25, 0.5)),
            Name::new(format!("Interior_{}", name)),
        ))
        .id();

    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: main_color,
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 招牌發光
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.9, 1.5, 0.3))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(1.0, 1.0, 1.0, 1.0) * 4.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 - 1.0, d / 2.0 + 0.2),
            GlobalTransform::default(),
        ));

        // 玻璃門面
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.8, h * 0.6, 0.1))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgba(0.7, 0.85, 0.9, 0.4),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(0.0, -h / 6.0, d / 2.0 + 0.25),
            GlobalTransform::default(),
        ));
    });

    // 創建可互動的門（世界座標，非子實體）
    let door_material = mats.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.5, 0.7, 0.5),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    cmd.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.5, 2.5, 0.1))),
        MeshMaterial3d(door_material),
        Transform::from_translation(door_pos),
        GlobalTransform::default(),
        Visibility::default(),
        Door {
            interior_entity: Some(interior_entity),
            interact_radius: 2.5,
            ..default()
        },
        Name::new(format!("Door_{}", name)),
    ));
}

// === 12. 潮流服飾店 (Streetwear Shop) ===
fn spawn_streetwear_shop(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let main_color = if name.contains("潮牌") {
        Color::srgb(0.1, 0.1, 0.1) // 黑色
    } else if name.contains("古著") {
        Color::srgb(0.5, 0.4, 0.3) // 復古棕
    } else {
        Color::srgb(0.15, 0.15, 0.15) // 深灰 (球鞋店)
    };

    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: main_color,
        perceptual_roughness: 0.2,
        metallic: 0.3,
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 大型玻璃櫥窗
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.85, h * 0.7, 0.15))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgba(0.1, 0.1, 0.1, 0.3),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(0.0, -h / 8.0, d / 2.0 + 0.2),
            GlobalTransform::default(),
        ));

        // 紅色 Logo 標誌
        let accent = if name.contains("潮牌") {
            Color::srgb(1.0, 0.1, 0.1)
        } else if name.contains("球鞋") {
            Color::srgb(1.0, 0.5, 0.0)
        } else {
            Color::srgb(0.8, 0.6, 0.3)
        };
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.4, 1.5, 0.3))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: accent,
                emissive: LinearRgba::from(accent) * 3.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 3.0, d / 2.0 + 0.2),
            GlobalTransform::default(),
        ));
    });
}

// === 13. 夾娃娃機店 (Claw Machine) ===
fn spawn_claw_machine(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: Color::srgb(0.9, 0.5, 0.9), // 粉紫色
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 彩色閃爍燈
        let colors = [
            Color::srgb(1.0, 0.2, 0.5),
            Color::srgb(0.2, 1.0, 0.5),
            Color::srgb(0.5, 0.2, 1.0),
            Color::srgb(1.0, 1.0, 0.2),
        ];

        use rand::Rng;
        let mut rng = rand::rng();

        for i in 0..8 {
            let color = colors[i % 4];
            let x_off = rng.random_range(-w / 2.0 + 0.5..w / 2.0 - 0.5);
            let y_off = rng.random_range(-h / 3.0..h / 3.0);

            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.3))),
                MeshMaterial3d(mats.add(StandardMaterial {
                    base_color: color,
                    emissive: LinearRgba::from(color) * 5.0,
                    ..default()
                })),
                Transform::from_xyz(x_off, y_off, d / 2.0 + 0.2),
                GlobalTransform::default(),
            ));
        }

        // 招牌
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.7, 1.2, 0.3))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.0),
                emissive: LinearRgba::new(1.0, 0.8, 0.0, 1.0) * 3.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 + 0.8, 0.0),
            GlobalTransform::default(),
        ));
    });
}

// === 14. 大創 (Daiso) ===
fn spawn_daiso(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: Color::srgb(0.9, 0.4, 0.5), // 粉紅色
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 白色招牌
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.8, 2.0, 0.3))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(1.0, 1.0, 1.0, 1.0) * 3.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 - 1.5, d / 2.0 + 0.2),
            GlobalTransform::default(),
        ));

        // 玻璃門面
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.85, h * 0.6, 0.1))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgba(0.85, 0.85, 0.9, 0.4),
                alpha_mode: AlphaMode::Blend,
                ..default()
            })),
            Transform::from_xyz(0.0, -h / 6.0, d / 2.0 + 0.25),
            GlobalTransform::default(),
        ));
    });
}

// === 15. 路燈 (Lamppost) ===
fn spawn_lamppost(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
) {
    let pole_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });

    commands
        .spawn((
            Transform::from_translation(position),
            GlobalTransform::default(),
            Visibility::default(),
            StreetFurniture {
                furniture_type: StreetFurnitureType::Lamppost,
                can_interact: false,
            },
            // 路燈柱碰撞體 (半徑 0.15, 高度 5.0)
            Collider::cylinder(2.5, 0.15),
            RigidBody::Fixed,
        ))
        .with_children(|parent| {
            // 燈桿
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.08, 5.0))),
                MeshMaterial3d(pole_mat.clone()),
                Transform::from_xyz(0.0, 2.5, 0.0),
                GlobalTransform::default(),
            ));

            // 燈桿底座
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.2, 0.3))),
                MeshMaterial3d(pole_mat),
                Transform::from_xyz(0.0, 0.15, 0.0),
                GlobalTransform::default(),
            ));

            // 燈頭
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.3))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.95, 0.8),
                    emissive: LinearRgba::new(8.0, 7.5, 5.0, 1.0),
                    ..default()
                })),
                Transform::from_xyz(0.0, 5.0, 0.0),
                GlobalTransform::default(),
            ));

            // 光源 (shadows_enabled: false 效能優化，38 盞燈全開陰影太吃效能)
            parent.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.95, 0.8),
                    intensity: 100_000.0,
                    range: 20.0,
                    shadows_enabled: false, // 效能優化：PointLight 陰影計算昂貴
                    ..default()
                },
                Transform::from_xyz(0.0, 5.0, 0.0),
                GlobalTransform::default(),
                StreetLight { is_on: true },
            ));
        });
}

// === 16. 自動販賣機 (Vending Machine) ===
fn spawn_vending_machine(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    rotation: f32,
    variant: u8, // 0: 飲料, 1: 零食, 2: 香菸
) {
    let colors = [
        Color::srgb(0.1, 0.4, 0.8), // 藍色飲料機
        Color::srgb(0.8, 0.3, 0.1), // 橘色零食機
        Color::srgb(0.3, 0.3, 0.3), // 灰色香菸機
    ];
    let color = colors[variant as usize % 3];

    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 1.8, 0.6))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                metallic: 0.5,
                perceptual_roughness: 0.3,
                ..default()
            })),
            Transform::from_translation(position + Vec3::new(0.0, 0.9, 0.0))
                .with_rotation(Quat::from_rotation_y(rotation)),
            GlobalTransform::default(),
            Visibility::default(),
            Collider::cuboid(0.4, 0.9, 0.3),
            StreetFurniture {
                furniture_type: StreetFurnitureType::VendingMachine,
                can_interact: true,
            },
        ))
        .with_children(|parent| {
            // 發光展示窗
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.6, 1.2, 0.05))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    emissive: LinearRgba::new(2.0, 2.0, 2.0, 1.0),
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.2, 0.28),
                GlobalTransform::default(),
            ));

            // 品牌標誌
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.5, 0.2, 0.05))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    emissive: LinearRgba::from(color) * 2.0,
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.75, 0.28),
                GlobalTransform::default(),
            ));
        });
}

// === 17. 垃圾桶 (Trash Can) ===
fn spawn_trash_can(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
) {
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.25, 0.8))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.35, 0.3),
                metallic: 0.4,
                perceptual_roughness: 0.6,
                ..default()
            })),
            Transform::from_translation(position + Vec3::new(0.0, 0.4, 0.0)),
            GlobalTransform::default(),
            Collider::cylinder(0.4, 0.25),
            StreetFurniture {
                furniture_type: StreetFurnitureType::TrashCan,
                can_interact: false,
            },
        ))
        .with_children(|parent| {
            // 垃圾桶蓋
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.28, 0.05))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.25, 0.3, 0.25),
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.4, 0.0),
                GlobalTransform::default(),
            ));

            // 垃圾分類標誌 (綠色)
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.15, 0.15, 0.02))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 0.7, 0.3),
                    ..default()
                })),
                Transform::from_xyz(0.0, 0.2, 0.24),
                GlobalTransform::default(),
            ));
        });
}

// === Phase A: 斑馬線 ===
fn spawn_zebra_crossing(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Handle<StandardMaterial>,
    center: Vec3,
    length: f32,        // 斑馬線總長度
    is_east_west: bool, // true = 東西向 (X方向), false = 南北向 (Z方向)
) {
    // 斑馬線規格：寬 5m，白線寬 0.5m，間隔 0.5m
    let stripe_width = 0.5;
    let stripe_gap = 0.5;
    let crossing_width = 5.0; // 行人穿越區域寬度

    let stripe_count = (length / (stripe_width + stripe_gap)) as i32;

    for i in 0..stripe_count {
        let offset = (i as f32 - stripe_count as f32 / 2.0) * (stripe_width + stripe_gap);

        let (pos, size) = if is_east_west {
            // 東西向斑馬線：白線沿 X 軸排列
            (
                center + Vec3::new(offset, 0.0, 0.0),
                Vec3::new(stripe_width, 0.02, crossing_width),
            )
        } else {
            // 南北向斑馬線：白線沿 Z 軸排列
            (
                center + Vec3::new(0.0, 0.0, offset),
                Vec3::new(crossing_width, 0.02, stripe_width),
            )
        };

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(pos),
            GlobalTransform::default(),
            Visibility::default(),
        ));
    }
}

// === Phase 6: 電影看板 ===
fn spawn_movie_billboard(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    color: Color,
) {
    // 看板尺寸: 4x6 公尺
    let width = 4.0;
    let height = 6.0;
    let depth = 0.3;

    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(width, height, depth))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            })),
            Transform::from_translation(position),
            GlobalTransform::default(),
            Visibility::default(),
            StreetFurniture {
                furniture_type: StreetFurnitureType::Billboard,
                can_interact: false,
            },
        ))
        .with_children(|parent| {
            // 看板邊框
            let frame_color = Color::srgb(0.15, 0.15, 0.15);
            let frame_mat = materials.add(StandardMaterial {
                base_color: frame_color,
                metallic: 0.8,
                ..default()
            });

            // 上邊框
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(width + 0.4, 0.2, depth + 0.1))),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(0.0, height / 2.0 + 0.1, 0.0),
                GlobalTransform::default(),
            ));

            // 下邊框
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(width + 0.4, 0.2, depth + 0.1))),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(0.0, -height / 2.0 - 0.1, 0.0),
                GlobalTransform::default(),
            ));

            // 左邊框
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.2, height, depth + 0.1))),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(-width / 2.0 - 0.1, 0.0, 0.0),
                GlobalTransform::default(),
            ));

            // 右邊框
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.2, height, depth + 0.1))),
                MeshMaterial3d(frame_mat.clone()),
                Transform::from_xyz(width / 2.0 + 0.1, 0.0, 0.0),
                GlobalTransform::default(),
            ));

            // 聚光燈 (頂部)
            parent.spawn((
                PointLight {
                    color,
                    intensity: 50000.0,
                    range: 15.0,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, height / 2.0 + 1.5, 2.0),
                GlobalTransform::default(),
            ));
        });
}

// === Phase 6: 塗鴉牆 ===
fn spawn_graffiti_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
) {
    // 牆面尺寸: 15x5 公尺
    let width = 15.0;
    let height = 5.0;
    let depth = 0.3;

    // 基底牆面 (灰色混凝土)
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(width, height, depth))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.43, 0.4),
                perceptual_roughness: 0.95,
                ..default()
            })),
            Transform::from_translation(position),
            GlobalTransform::default(),
            Visibility::default(),
            Collider::cuboid(width / 2.0, height / 2.0, depth / 2.0),
        ))
        .with_children(|parent| {
            // 程序化生成塗鴉色塊
            let graffiti_colors = [
                Color::srgb(1.0, 0.2, 0.3), // 紅
                Color::srgb(0.2, 0.8, 0.3), // 綠
                Color::srgb(0.2, 0.4, 1.0), // 藍
                Color::srgb(1.0, 0.9, 0.2), // 黃
                Color::srgb(1.0, 0.5, 0.1), // 橘
                Color::srgb(0.8, 0.2, 0.9), // 紫
                Color::srgb(0.1, 0.9, 0.9), // 青
                Color::srgb(1.0, 0.4, 0.7), // 粉
            ];

            // 使用確定性的位置模式 (x, y, color_idx)
            let splash_positions: [(f32, f32, usize); 15] = [
                (-5.0, 1.0, 0),
                (2.0, 1.5, 1),
                (5.5, 0.5, 2),
                (-3.0, -1.0, 3),
                (0.0, 0.0, 4),
                (4.0, -0.5, 5),
                (-6.0, -1.5, 6),
                (6.0, 1.0, 7),
                (-1.0, 2.0, 0),
                (3.0, -1.5, 1),
                (-4.5, 0.5, 2),
                (1.5, -2.0, 3),
                (-2.0, 1.8, 4),
                (5.0, -1.8, 5),
                (-5.5, -0.5, 6),
            ];

            for (x, y, color_idx) in splash_positions {
                let color = graffiti_colors[color_idx];
                let w = 1.2 + (x.abs() % 1.0) * 0.8;
                let h = 0.8 + (y.abs() % 0.5) * 0.6;

                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(w, h, 0.02))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        emissive: color.into(),
                        ..default()
                    })),
                    Transform::from_xyz(x, y, depth / 2.0 + 0.01),
                    GlobalTransform::default(),
                ));
            }

            // 中央大型標語 "TAIPEI" 風格的文字塊
            let tag_mat = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: Color::WHITE.into(),
                ..default()
            });

            // 簡化的字母形狀
            for (i, x_off) in [-3.0f32, -1.5, 0.0, 1.5, 3.0].iter().enumerate() {
                let h = 1.5 + ((i % 2) as f32) * 0.3;
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.0, h, 0.03))),
                    MeshMaterial3d(tag_mat.clone()),
                    Transform::from_xyz(*x_off, -0.5, depth / 2.0 + 0.02),
                    GlobalTransform::default(),
                ));
            }
        });
}

// === 18. 峨嵋停車場 (Parking Garage) ===
fn spawn_parking_garage(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    // 主體結構 (開放式)
    cmd.spawn((
        Mesh3d(meshes.add(Cuboid::new(w, h, d))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.55),
            ..default()
        })),
        Transform::from_translation(pos),
        GlobalTransform::default(),
        Visibility::default(),
        Collider::cuboid(w / 2.0, h / 2.0, d / 2.0),
        Building {
            name: name.to_string(),
            building_type: BuildingType::Shop,
        },
    ))
    .with_children(|parent| {
        // 樓層板 (Floors)
        let floor_h = 4.0;
        let levels = (h / floor_h) as i32;
        let floor_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.3, 0.3),
            ..default()
        });

        for i in 0..levels {
            let y_off = -h / 2.0 + (i as f32) * floor_h;
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(w + 1.0, 0.2, d + 1.0))),
                MeshMaterial3d(floor_mat.clone()),
                Transform::from_xyz(0.0, y_off, 0.0),
                GlobalTransform::default(),
            ));
        }
    });

    // 停車場內不再生成車輛，玩家需要自己找車
}

// === 9. 人形角色生成 ===
/// 生成程序化人形角色（台灣年輕人風格）
/// 完整關節系統：肩關節、肘關節、髖關節、膝關節、腳踝
/// 身高約 1.7 公尺（遊戲單位）
pub fn spawn_player_character(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    player: Player,
) -> Entity {
    // === 材質定義 ===
    let skin_color = Color::srgb(0.96, 0.80, 0.69); // 亞洲膚色
    let hair_color = Color::srgb(0.1, 0.08, 0.05); // 深黑髮
    let shirt_color = Color::srgb(0.2, 0.5, 0.9); // 藍色 T 恤
    let pants_color = Color::srgb(0.2, 0.2, 0.25); // 深色牛仔褲
    let shoe_color = Color::srgb(0.95, 0.95, 0.95); // 白色球鞋

    let skin_mat = materials.add(StandardMaterial {
        base_color: skin_color,
        perceptual_roughness: 0.6,
        ..default()
    });
    let hair_mat = materials.add(StandardMaterial {
        base_color: hair_color,
        perceptual_roughness: 0.9,
        ..default()
    });
    let shirt_mat = materials.add(StandardMaterial {
        base_color: shirt_color,
        perceptual_roughness: 0.8,
        ..default()
    });
    let pants_mat = materials.add(StandardMaterial {
        base_color: pants_color,
        perceptual_roughness: 0.7,
        ..default()
    });
    let shoe_mat = materials.add(StandardMaterial {
        base_color: shoe_color,
        perceptual_roughness: 0.5,
        ..default()
    });
    let eye_white_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        ..default()
    });
    let eye_iris_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.1, 0.05),
        ..default()
    });
    let lip_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.55, 0.55),
        perceptual_roughness: 0.4,
        ..default()
    });

    // 碰撞體參數
    const COLLIDER_HALF_HEIGHT: f32 = 0.45;
    const COLLIDER_RADIUS: f32 = 0.25;

    // 身體比例常數（相對於碰撞體中心）
    const HEAD_Y: f32 = 0.58;
    const NECK_Y: f32 = 0.42;
    const CHEST_Y: f32 = 0.18;
    const WAIST_Y: f32 = -0.02;
    // 髖部位置上調，確保腳底不超出碰撞體（底部在 -0.70）
    const HIP_Y: f32 = -0.10;

    // 生成主體
    let collider_center_y = COLLIDER_HALF_HEIGHT + COLLIDER_RADIUS;
    let spawn_pos = position + Vec3::new(0.0, collider_center_y, 0.0);

    // 初始化玩家武器庫存
    let mut weapon_inventory = WeaponInventory::default();
    weapon_inventory.add_weapon(Weapon::new(WeaponStats::pistol()));
    weapon_inventory.add_weapon(Weapon::new(WeaponStats::smg()));
    weapon_inventory.add_weapon(Weapon::new(WeaponStats::shotgun()));
    weapon_inventory.add_weapon(Weapon::new(WeaponStats::rifle()));

    let player_entity = commands
        .spawn((
            Transform::from_translation(spawn_pos),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(COLLIDER_HALF_HEIGHT, COLLIDER_RADIUS),
            KinematicCharacterController {
                slide: true,
                ..default()
            },
            player,
            crate::core::ThirdPersonCameraTarget,
            Name::new("Player"),
            weapon_inventory,
            Health::new(100.0),
            Armor::default(),
            Damageable,
        ))
        .insert(DodgeState::default()) // 閃避狀態（分開插入避免 tuple 限制）
        .insert(crate::player::ClimbState::default()) // 攀爬/翻越狀態
        .insert(HitReaction::default()) // 受傷反應
        .insert(PlayerInteriorState::default()) // 室內狀態
        .insert(crate::combat::PlayerCoverState::default()) // 掩體狀態
        .insert(crate::combat::ExplosiveInventory {
            // 爆炸物庫存
            grenades: 3,
            molotovs: 2,
            sticky_bombs: 1,
            selected: Some(crate::combat::ExplosiveType::Grenade),
            throw_cooldown: 0.0,
        })
        .insert(crate::wanted::PlayerSurrenderState::default()) // 投降狀態
        .insert(crate::vehicle::PlayerTheftState::default()) // 偷車狀態
        .insert(CollisionGroups::new(
            COLLISION_GROUP_CHARACTER,
            COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
        )) // 角色與角色、載具、靜態物碰撞
        .with_children(|parent| {
            // ============================================================
            // 頭部（含臉部細節）
            // ============================================================
            let head_radius = 0.12;

            // 頭部主體
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(head_radius))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y, 0.0).with_scale(Vec3::new(0.95, 1.0, 0.9)),
            ));

            // 眼白
            let eye_y = HEAD_Y + 0.015;
            let eye_z = head_radius * 0.85;
            let eye_spacing = 0.038;
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.02))),
                MeshMaterial3d(eye_white_mat.clone()),
                Transform::from_xyz(eye_spacing, eye_y, eye_z).with_scale(Vec3::new(1.2, 0.8, 0.5)),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.02))),
                MeshMaterial3d(eye_white_mat.clone()),
                Transform::from_xyz(-eye_spacing, eye_y, eye_z)
                    .with_scale(Vec3::new(1.2, 0.8, 0.5)),
            ));

            // 瞳孔
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.009))),
                MeshMaterial3d(eye_iris_mat.clone()),
                Transform::from_xyz(eye_spacing, eye_y, eye_z + 0.01),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.009))),
                MeshMaterial3d(eye_iris_mat.clone()),
                Transform::from_xyz(-eye_spacing, eye_y, eye_z + 0.01),
            ));

            // 眉毛
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.035, 0.008, 0.012))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(eye_spacing, eye_y + 0.028, eye_z - 0.005),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.035, 0.008, 0.012))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(-eye_spacing, eye_y + 0.028, eye_z - 0.005),
            ));

            // 鼻子
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.022, 0.04, 0.028))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y - 0.01, eye_z + 0.012),
            ));

            // 嘴巴
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.045, 0.014, 0.016))),
                MeshMaterial3d(lip_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y - 0.048, eye_z - 0.01),
            ));

            // 耳朵
            let ear_x = head_radius * 0.92;
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.028))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(ear_x, HEAD_Y, 0.0).with_scale(Vec3::new(0.4, 1.0, 0.7)),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.028))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(-ear_x, HEAD_Y, 0.0).with_scale(Vec3::new(0.4, 1.0, 0.7)),
            ));

            // 頭髮（厚實短髮）
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(head_radius * 1.12))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y + 0.05, -0.02)
                    .with_scale(Vec3::new(1.05, 0.5, 1.15)),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(head_radius * 0.9))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y + 0.02, -0.08).with_scale(Vec3::new(0.9, 0.8, 0.6)),
            ));
            // 瀏海
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.2, 0.035, 0.05))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y + head_radius * 0.85, 0.08),
            ));

            // ============================================================
            // 脖子
            // ============================================================
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.045, 0.1))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, NECK_Y, 0.0),
            ));

            // ============================================================
            // 軀幹（胸部 + 腰部 + 臀部）
            // ============================================================
            // 胸部
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.3, 0.22, 0.15))),
                MeshMaterial3d(shirt_mat.clone()),
                Transform::from_xyz(0.0, CHEST_Y, 0.0),
            ));
            // 腰部（較窄）
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.24, 0.1, 0.13))),
                MeshMaterial3d(shirt_mat.clone()),
                Transform::from_xyz(0.0, WAIST_Y, 0.0),
            ));
            // 臀部/髖部
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.28, 0.1, 0.15))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(0.0, HIP_Y, 0.0),
            ));

            // ============================================================
            // 左手臂（有關節，保留 PlayerArm 組件）
            // ============================================================
            let left_shoulder_pos = Vec3::new(0.18, CHEST_Y + 0.06, 0.0);
            let left_arm_offset = Vec3::new(0.03, -0.08, 0.0);
            let left_arm_pos = left_shoulder_pos + left_arm_offset; // 實際生成位置
            let left_arm_rot = Quat::from_rotation_z(-0.15);

            // 肩關節
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.045))),
                MeshMaterial3d(shirt_mat.clone()),
                Transform::from_translation(left_shoulder_pos),
            ));

            // 上臂（PlayerArm 組件附加在這裡）- 比例：手指到大腿中段
            parent
                .spawn((
                    Mesh3d(meshes.add(Capsule3d::new(0.035, 0.10))), // 縮短
                    MeshMaterial3d(shirt_mat.clone()),
                    Transform::from_translation(left_arm_pos).with_rotation(left_arm_rot),
                    GlobalTransform::default(),
                    PlayerArm::left(left_arm_pos, left_arm_rot), // 使用實際位置
                    Name::new("LeftArm"),
                ))
                .with_children(|arm| {
                    // 肘關節
                    arm.spawn((
                        Mesh3d(meshes.add(Sphere::new(0.03))),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.12, 0.0),
                        GlobalTransform::default(),
                    ));
                    // 前臂
                    arm.spawn((
                        Mesh3d(meshes.add(Capsule3d::new(0.028, 0.08))), // 縮短
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.22, 0.0),
                        GlobalTransform::default(),
                    ));
                    // 手腕
                    arm.spawn((
                        Mesh3d(meshes.add(Sphere::new(0.022))),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.32, 0.0),
                        GlobalTransform::default(),
                    ));
                    // 手掌（PlayerHand 組件）
                    arm.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.045, 0.055, 0.022))),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.36, 0.0),
                        GlobalTransform::default(),
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                        PlayerHand { is_right: false },
                        Name::new("LeftHand"),
                    ));
                    // 手指
                    arm.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.04, 0.035, 0.018))),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.40, 0.0),
                        GlobalTransform::default(),
                    ));
                });

            // ============================================================
            // 右手臂（有關節，保留 PlayerArm 組件）
            // ============================================================
            let right_shoulder_pos = Vec3::new(-0.18, CHEST_Y + 0.06, 0.0);
            let right_arm_offset = Vec3::new(-0.03, -0.08, 0.0);
            let right_arm_pos = right_shoulder_pos + right_arm_offset; // 實際生成位置
            let right_arm_rot = Quat::from_rotation_z(0.15);

            // 肩關節
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.045))),
                MeshMaterial3d(shirt_mat.clone()),
                Transform::from_translation(right_shoulder_pos),
            ));

            // 上臂（PlayerArm 組件附加在這裡）- 比例：手指到大腿中段
            parent
                .spawn((
                    Mesh3d(meshes.add(Capsule3d::new(0.035, 0.10))), // 縮短
                    MeshMaterial3d(shirt_mat.clone()),
                    Transform::from_translation(right_arm_pos).with_rotation(right_arm_rot),
                    GlobalTransform::default(),
                    PlayerArm::right(right_arm_pos, right_arm_rot), // 使用實際位置
                    Name::new("RightArm"),
                ))
                .with_children(|arm| {
                    // 肘關節
                    arm.spawn((
                        Mesh3d(meshes.add(Sphere::new(0.03))),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.12, 0.0),
                        GlobalTransform::default(),
                    ));
                    // 前臂
                    arm.spawn((
                        Mesh3d(meshes.add(Capsule3d::new(0.028, 0.08))), // 縮短
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.22, 0.0),
                        GlobalTransform::default(),
                    ));
                    // 手腕
                    arm.spawn((
                        Mesh3d(meshes.add(Sphere::new(0.022))),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.32, 0.0),
                        GlobalTransform::default(),
                    ));
                    // 手掌（PlayerHand 組件）- 武器會作為此實體的子實體
                    arm.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.045, 0.055, 0.022))),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.36, 0.0),
                        GlobalTransform::default(),
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                        PlayerHand { is_right: true },
                        Name::new("RightHand"),
                    ));
                    // 手指
                    arm.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.04, 0.035, 0.018))),
                        MeshMaterial3d(skin_mat.clone()),
                        Transform::from_xyz(0.0, -0.40, 0.0),
                        GlobalTransform::default(),
                    ));
                });

            // ============================================================
            // 左腿（有關節）- 比例修正：縮短腿部以符合碰撞體
            // ============================================================
            let hip_x = 0.08;

            // 髖關節
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.050))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(hip_x, HIP_Y - 0.04, 0.0),
            ));
            // 大腿（縮短）
            parent.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.050, 0.12))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(hip_x, HIP_Y - 0.16, 0.0),
            ));
            // 膝關節
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.042))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(hip_x, HIP_Y - 0.30, 0.0),
            ));
            // 小腿（縮短）
            parent.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.038, 0.11))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(hip_x, HIP_Y - 0.42, 0.0),
            ));
            // 腳踝
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.030))),
                MeshMaterial3d(shoe_mat.clone()),
                Transform::from_xyz(hip_x, HIP_Y - 0.54, 0.0),
            ));
            // 腳掌
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.06, 0.04, 0.12))),
                MeshMaterial3d(shoe_mat.clone()),
                Transform::from_xyz(hip_x, HIP_Y - 0.57, 0.025),
            ));
            // 鞋頭
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.032))),
                MeshMaterial3d(shoe_mat.clone()),
                Transform::from_xyz(hip_x, HIP_Y - 0.57, 0.075)
                    .with_scale(Vec3::new(1.0, 0.7, 1.2)),
            ));

            // ============================================================
            // 右腿（有關節）- 比例修正：縮短腿部以符合碰撞體
            // ============================================================
            // 髖關節
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.050))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(-hip_x, HIP_Y - 0.04, 0.0),
            ));
            // 大腿（縮短）
            parent.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.050, 0.12))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(-hip_x, HIP_Y - 0.16, 0.0),
            ));
            // 膝關節
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.042))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(-hip_x, HIP_Y - 0.30, 0.0),
            ));
            // 小腿（縮短）
            parent.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.038, 0.11))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(-hip_x, HIP_Y - 0.42, 0.0),
            ));
            // 腳踝
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.030))),
                MeshMaterial3d(shoe_mat.clone()),
                Transform::from_xyz(-hip_x, HIP_Y - 0.54, 0.0),
            ));
            // 腳掌
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.06, 0.04, 0.12))),
                MeshMaterial3d(shoe_mat.clone()),
                Transform::from_xyz(-hip_x, HIP_Y - 0.57, 0.025),
            ));
            // 鞋頭
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.032))),
                MeshMaterial3d(shoe_mat.clone()),
                Transform::from_xyz(-hip_x, HIP_Y - 0.57, 0.075)
                    .with_scale(Vec3::new(1.0, 0.7, 1.2)),
            ));

            // ============================================================
            // 外送背包（標誌性裝備）
            // ============================================================
            let backpack_color = Color::srgb(0.1, 0.7, 0.4);
            let backpack_mat = materials.add(StandardMaterial {
                base_color: backpack_color,
                perceptual_roughness: 0.6,
                ..default()
            });
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.28, 0.32, 0.18))),
                MeshMaterial3d(backpack_mat.clone()),
                Transform::from_xyz(0.0, CHEST_Y - 0.02, -0.22),
            ));
            // 背包蓋
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.26, 0.05, 0.16))),
                MeshMaterial3d(backpack_mat),
                Transform::from_xyz(0.0, CHEST_Y + 0.15, -0.22),
            ));
        })
        .id();

    player_entity
}

// === 13. AI 掩體點生成系統 ===

/// 掩體點 Y 座標常數（略高於地面，便於視覺調試）
const COVER_POINT_Y: f32 = 0.5;

/// 掩體類型枚舉，用於生成不同類型的掩體
#[derive(Clone, Copy)]
enum CoverKind {
    Low,
    High,
    Full,
}

/// 輔助函數：批量生成掩體點，減少代碼重複
fn spawn_cover_batch(
    commands: &mut Commands,
    positions: &[(f32, f32, Vec3)], // (x, z, direction)
    kind: CoverKind,
    label: &'static str,
) -> u32 {
    let mut count = 0;
    for (i, &(x, z, direction)) in positions.iter().enumerate() {
        let pos = Vec3::new(x, COVER_POINT_Y, z);
        let cover = match kind {
            CoverKind::Low => CoverPoint::low(direction),
            CoverKind::High => CoverPoint::high(direction),
            CoverKind::Full => CoverPoint::full(direction),
        };
        commands.spawn((
            Transform::from_translation(pos),
            GlobalTransform::default(),
            cover,
            Name::new(format!("{}_{}", label, i)),
        ));
        count += 1;
    }
    count
}

/// 在世界中策略位置生成 CoverPoint 實體
/// 掩體位置包括：建築角落、車輛旁、販賣機旁、垃圾桶旁等
fn spawn_cover_points(commands: &mut Commands) {
    let mut cover_count = 0;

    // === 建築角落掩體點 (High Cover) ===
    // 建築物四角提供高掩體，方向朝向街道
    let building_corners = [
        // 西門紅樓區域
        (-35.0, 8.0, Vec3::NEG_Z), // 紅樓北側
        (-35.0, -8.0, Vec3::Z),    // 紅樓南側
        (-28.0, 8.0, Vec3::NEG_Z), // 紅樓東北角
        (-28.0, -8.0, Vec3::Z),    // 紅樓東南角
        // Uniqlo / 萬年大樓區域
        (-60.0, -15.0, Vec3::X),     // 萬年大樓西側
        (-55.0, -15.0, Vec3::NEG_X), // 萬年大樓東側
        (-45.0, -20.0, Vec3::Z),     // 唐吉訶德南側
        (-45.0, -5.0, Vec3::NEG_Z),  // 唐吉訶德北側
        // 漢中街徒步區
        (8.0, -12.0, Vec3::NEG_X), // Uniqlo 旁
        (-8.0, -12.0, Vec3::X),    // 對面店家
        (8.0, 15.0, Vec3::NEG_X),  // 南段店家
        (-8.0, 15.0, Vec3::X),     // 南段對面
        // 武昌街電影街
        (30.0, -55.0, Vec3::Z),     // 國賓影城旁
        (45.0, -55.0, Vec3::Z),     // 樂聲影城旁
        (55.0, -60.0, Vec3::X),     // 日新威秀角
        (35.0, -40.0, Vec3::NEG_Z), // 電影街南側
        // 成都路沿線
        (-25.0, 38.0, Vec3::NEG_Z), // 阿宗麵線旁
        (15.0, 38.0, Vec3::NEG_Z),  // 50嵐旁
        (28.0, 38.0, Vec3::NEG_Z),  // 夾娃娃機旁
        // 康定路西側
        (X_KANGDING + 15.0, -60.0, Vec3::Z),
        (X_KANGDING + 15.0, -20.0, Vec3::Z),
        (X_KANGDING + 15.0, 20.0, Vec3::NEG_Z),
        (X_KANGDING + 15.0, 45.0, Vec3::NEG_Z),
        // 峨嵋街區
        (30.0, -5.0, Vec3::Z), // 潮牌店旁
    ];
    cover_count += spawn_cover_batch(
        commands,
        &building_corners,
        CoverKind::High,
        "Cover_Building",
    );

    // === 販賣機旁掩體點 (Low Cover) - 與販賣機位置對應
    let vending_covers = [
        (13.0, -15.0, Vec3::NEG_X), // Uniqlo 旁（已移離馬路）
        (-69.0, -15.0, Vec3::X),    // 萬年大樓旁（已移離馬路）
        (-31.0, -22.0, Vec3::X),    // 唐吉訶德旁
        (43.0, 36.0, Vec3::NEG_X),  // 捷運站旁
        (-74.0, 12.0, Vec3::NEG_X), // 7-11 旁（已移離馬路）
    ];
    cover_count += spawn_cover_batch(commands, &vending_covers, CoverKind::Low, "Cover_Vending");

    // === 垃圾桶旁掩體點 (Low Cover) - 與垃圾桶位置對應
    let trash_covers = [
        (9.0, -10.0, Vec3::NEG_X), // 十字路口東北角
        (-9.0, -10.0, Vec3::X),    // 十字路口西北角
        (9.0, -55.0, Vec3::NEG_X), // 武昌街東側
        (-9.0, -55.0, Vec3::X),    // 武昌街西側
        (-29.0, 12.0, Vec3::X),    // 峨嵋街北側
        (31.0, 12.0, Vec3::NEG_X), // 峨嵋街北側
    ];
    cover_count += spawn_cover_batch(commands, &trash_covers, CoverKind::Low, "Cover_Trash");

    // === 停放車輛旁掩體點 (Low Cover) ===
    let vehicle_covers = [
        // 漢中街機車停放區
        (13.0, -8.0, Vec3::NEG_X),
        (13.0, -5.0, Vec3::NEG_X),
        // 西門紅樓前
        (X_ZHONGHUA - 29.0, Z_CHENGDU + 12.0, Vec3::Z),
        (X_ZHONGHUA - 33.0, Z_CHENGDU + 12.0, Vec3::Z),
        // 峨嵋街口
        (X_XINING + 3.0, Z_EMEI + 8.0, Vec3::NEG_Z),
        (X_XINING + 5.0, Z_EMEI + 8.0, Vec3::NEG_Z),
        // 停車場出口
        (X_KANGDING + 20.0, Z_EMEI + 30.0, Vec3::Z),
        (X_KANGDING + 30.0, Z_EMEI + 30.0, Vec3::Z),
    ];
    cover_count += spawn_cover_batch(commands, &vehicle_covers, CoverKind::Low, "Cover_Vehicle");

    // === 街道轉角掩體點 (Full Cover) ===
    // 主要交叉路口的建築轉角提供完全掩護
    // 使用常見的對角方向 (45 度)
    let diag_ne = Vec3::new(1.0, 0.0, -1.0).normalize();
    let diag_nw = Vec3::new(-1.0, 0.0, -1.0).normalize();
    let diag_se = Vec3::new(1.0, 0.0, 1.0).normalize();
    let diag_sw = Vec3::new(-1.0, 0.0, 1.0).normalize();

    let corner_full_covers = [
        // 漢中/峨嵋交叉口四角
        (X_HAN + 10.0, Z_EMEI - 10.0, diag_sw),
        (X_HAN - 10.0, Z_EMEI - 10.0, diag_se),
        (X_HAN + 10.0, Z_EMEI + 10.0, diag_nw),
        (X_HAN - 10.0, Z_EMEI + 10.0, diag_ne),
        // 漢中/武昌交叉口四角
        (X_HAN + 10.0, Z_WUCHANG - 10.0, diag_sw),
        (X_HAN - 10.0, Z_WUCHANG - 10.0, diag_se),
        (X_HAN + 10.0, Z_WUCHANG + 10.0, diag_nw),
        (X_HAN - 10.0, Z_WUCHANG + 10.0, diag_ne),
    ];
    cover_count += spawn_cover_batch(
        commands,
        &corner_full_covers,
        CoverKind::Full,
        "Cover_Corner",
    );

    info!("🛡️ 已生成 {} 個 AI 掩體點", cover_count);
}
