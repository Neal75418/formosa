//! 建築與霓虹燈配置

use bevy::prelude::*;

use crate::world::buildings::spawn_rich_building;
use crate::world::constants::{
    BuildingTracker, BUILDING_ROAD_BUFFER, W_ALLEY, W_MAIN, W_PEDESTRIAN, W_SECONDARY, W_ZHONGHUA,
    X_HAN, X_KANGDING, X_XINING, X_ZHONGHUA, Z_CHENGDU, Z_EMEI, Z_HANKOU, Z_KUNMING, Z_WUCHANG,
};
use crate::world::{spawn_neon_sign, NeonSign};

// ============================================================================
// 輔助結構
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

// ============================================================================
// 建築生成
// ============================================================================

/// 地標建築與商店生成
#[allow(clippy::too_many_lines)]
pub(super) fn setup_buildings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    building_tracker: &mut BuildingTracker,
) {
    // === 3. 地標建築 (根據真實西門町位置) ===

    // 交叉路口建築（road1 = X 軸道路, road2 = Z 軸道路）
    let corner_buildings = [
        // 西寧南路沿線
        BuildingSpec {
            road1: RoadSide {
                center: X_XINING,
                width: W_SECONDARY,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_EMEI,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            width: 20.0,
            height: 28.0,
            depth: 15.0,
            name: "萬年大樓",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_XINING,
                width: W_SECONDARY,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_WUCHANG,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            width: 22.0,
            height: 24.0,
            depth: 22.0,
            name: "獅子林",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_XINING,
                width: W_SECONDARY,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_KUNMING,
                width: W_ALLEY,
                align: -1.0,
            },
            width: 23.0,
            height: 4.0,
            depth: 18.0,
            name: "電影公園",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_XINING,
                width: W_SECONDARY,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_WUCHANG,
                width: W_PEDESTRIAN,
                align: 1.0,
            },
            width: 28.0,
            height: 35.0,
            depth: 22.0,
            name: "Don Don Donki",
        },
        // 漢中街沿線
        BuildingSpec {
            road1: RoadSide {
                center: X_HAN,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_EMEI,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            width: 18.0,
            height: 20.0,
            depth: 16.0,
            name: "誠品西門",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_HAN,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_WUCHANG,
                width: W_PEDESTRIAN,
                align: 1.0,
            },
            width: 14.0,
            height: 18.0,
            depth: 14.0,
            name: "誠品武昌",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_HAN,
                width: W_PEDESTRIAN,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_EMEI,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            width: 12.0,
            height: 15.0,
            depth: 12.0,
            name: "Uniqlo",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_HAN,
                width: W_PEDESTRIAN,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_CHENGDU,
                width: W_MAIN,
                align: -1.0,
            },
            width: 14.0,
            height: 18.0,
            depth: 14.0,
            name: "H&M",
        },
        // 中華路沿線
        BuildingSpec {
            road1: RoadSide {
                center: X_ZHONGHUA,
                width: W_ZHONGHUA,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_CHENGDU,
                width: W_MAIN,
                align: -1.0,
            },
            width: 12.0,
            height: 8.0,
            depth: 12.0,
            name: "捷運6號出口",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_ZHONGHUA,
                width: W_ZHONGHUA,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_CHENGDU,
                width: W_MAIN,
                align: 1.0,
            },
            width: 22.0,
            height: 14.0,
            depth: 22.0,
            name: "西門紅樓",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_ZHONGHUA,
                width: W_ZHONGHUA,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_CHENGDU,
                width: W_MAIN,
                align: -1.0,
            },
            width: 16.0,
            height: 22.0,
            depth: 16.0,
            name: "錢櫃KTV",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_ZHONGHUA,
                width: W_ZHONGHUA,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_WUCHANG,
                width: W_PEDESTRIAN,
                align: 1.0,
            },
            width: 10.0,
            height: 8.0,
            depth: 10.0,
            name: "鴨肉扁",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_ZHONGHUA,
                width: W_ZHONGHUA,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_EMEI,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            width: 18.0,
            height: 28.0,
            depth: 16.0,
            name: "新光三越",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_ZHONGHUA,
                width: W_ZHONGHUA,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_HANKOU,
                width: W_SECONDARY,
                align: 1.0,
            },
            width: 20.0,
            height: 25.0,
            depth: 18.0,
            name: "遠東百貨",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_ZHONGHUA,
                width: W_ZHONGHUA,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_WUCHANG,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            width: 14.0,
            height: 20.0,
            depth: 12.0,
            name: "商業大樓A",
        },
        // 康定路沿線
        BuildingSpec {
            road1: RoadSide {
                center: X_KANGDING,
                width: W_MAIN,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_HANKOU,
                width: W_SECONDARY,
                align: 1.0,
            },
            width: 28.0,
            height: 12.0,
            depth: 23.0,
            name: "西門國小",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_KANGDING,
                width: W_MAIN,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_EMEI,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            width: 12.0,
            height: 10.0,
            depth: 12.0,
            name: "7-ELEVEN",
        },
        // 漢口街建築群
        BuildingSpec {
            road1: RoadSide {
                center: X_XINING,
                width: W_SECONDARY,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_HANKOU,
                width: W_SECONDARY,
                align: 1.0,
            },
            width: 10.0,
            height: 8.0,
            depth: 10.0,
            name: "全家便利",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_HAN,
                width: W_PEDESTRIAN,
                align: -1.0,
            },
            road2: RoadSide {
                center: Z_HANKOU,
                width: W_SECONDARY,
                align: 1.0,
            },
            width: 14.0,
            height: 12.0,
            depth: 12.0,
            name: "麥當勞",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_HAN,
                width: W_PEDESTRIAN,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_HANKOU,
                width: W_SECONDARY,
                align: 1.0,
            },
            width: 10.0,
            height: 10.0,
            depth: 10.0,
            name: "摩斯漢堡",
        },
        // 康定路南段
        BuildingSpec {
            road1: RoadSide {
                center: X_KANGDING,
                width: W_MAIN,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_CHENGDU,
                width: W_MAIN,
                align: 1.0,
            },
            width: 12.0,
            height: 10.0,
            depth: 12.0,
            name: "大創",
        },
        BuildingSpec {
            road1: RoadSide {
                center: X_KANGDING,
                width: W_MAIN,
                align: 1.0,
            },
            road2: RoadSide {
                center: Z_CHENGDU,
                width: W_MAIN,
                align: -1.0,
            },
            width: 14.0,
            height: 12.0,
            depth: 14.0,
            name: "彈珠台",
        },
    ];
    for spec in &corner_buildings {
        spawn_building_at_corner(commands, meshes, materials, building_tracker, spec);
    }

    // 道路沿線建築
    spawn_building_at_linear(
        commands,
        meshes,
        materials,
        building_tracker,
        Z_CHENGDU,
        W_MAIN,
        -1.0,
        X_XINING,
        X_HAN,
        8.0,
        6.0,
        "阿宗麵線",
    );
    spawn_building_at_linear(
        commands,
        meshes,
        materials,
        building_tracker,
        X_HAN,
        W_PEDESTRIAN,
        -1.0,
        Z_EMEI,
        Z_CHENGDU,
        6.0,
        6.0,
        "KFC",
    );
    spawn_building_at_linear(
        commands,
        meshes,
        materials,
        building_tracker,
        Z_EMEI,
        W_PEDESTRIAN,
        1.0,
        X_XINING,
        X_HAN,
        5.0,
        5.0,
        "小吃街",
    );

    // 直接定位建築
    let direct_buildings: &[(Vec3, f32, f32, f32, &str)] = &[
        (
            Vec3::new(X_ZHONGHUA - W_ZHONGHUA / 2.0 - 10.0, 15.0, 25.0),
            16.0,
            30.0,
            14.0,
            "統一元氣館",
        ),
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
        try_spawn_rich_building(
            commands,
            meshes,
            materials,
            building_tracker,
            pos,
            width,
            height,
            depth,
            name,
        );
    }

    info!(
        "🏢 已新增 {} 棟建築",
        corner_buildings.len() + 3 + direct_buildings.len()
    );
}

// ============================================================================
// 霓虹燈招牌
// ============================================================================

/// 霓虹燈招牌生成
#[allow(clippy::too_many_lines)]
pub(super) fn setup_neon_signs(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    building_tracker: &BuildingTracker,
) {
    let neon_signs: Vec<(&str, Vec3, Vec3, &str, NeonSign)> = vec![
        (
            "萬年大樓",
            Vec3::new(-71.0, 20.0, -7.5),
            Vec3::new(6.0, 1.5, 0.3),
            "萬年",
            NeonSign::flickering(Color::srgb(1.0, 0.2, 0.1), 10.0),
        ),
        (
            "錢櫃KTV",
            Vec3::new(100.0, 15.0, 34.0),
            Vec3::new(5.0, 1.2, 0.3),
            "錢櫃KTV",
            NeonSign::flickering(Color::srgb(0.9, 0.3, 0.9), 8.0),
        ),
        (
            "西門紅樓",
            Vec3::new(49.0, 10.0, 58.0),
            Vec3::new(4.0, 1.0, 0.3),
            "紅樓",
            NeonSign::steady(Color::srgb(1.0, 0.8, 0.3), 6.0),
        ),
        (
            "誠品西門",
            Vec3::new(-7.5, 14.0, -15.5),
            Vec3::new(4.0, 1.0, 0.3),
            "誠品",
            NeonSign::steady(Color::srgb(0.2, 0.9, 0.4), 7.0),
        ),
        (
            "阿宗麵線",
            Vec3::new(-27.5, 5.0, 42.0),
            Vec3::new(3.0, 0.8, 0.3),
            "阿宗麵線",
            NeonSign::flickering(Color::srgb(1.0, 0.5, 0.1), 8.0),
        ),
        (
            "Don Don Donki",
            Vec3::new(-35.0, 25.0, -20.5),
            Vec3::new(5.0, 1.2, 0.3),
            "Donki",
            NeonSign::flickering(Color::srgb(0.2, 0.5, 1.0), 9.0),
        ),
        (
            "Uniqlo",
            Vec3::new(7.5, 9.0, -13.5),
            Vec3::new(3.5, 1.0, 0.3),
            "UNIQLO",
            NeonSign::steady(Color::srgb(0.9, 0.1, 0.1), 8.0),
        ),
        (
            "誠品武昌",
            Vec3::new(-7.5, 11.0, -35.5),
            Vec3::new(4.0, 1.0, 0.3),
            "誠品",
            NeonSign::steady(Color::srgb(0.2, 0.9, 0.4), 7.0),
        ),
        (
            "獅子林",
            Vec3::new(-72.0, 17.0, -57.5),
            Vec3::new(3.0, 0.8, 0.3),
            "老店",
            NeonSign::broken(Color::srgb(0.8, 0.2, 0.3), 6.0),
        ),
        (
            "H&M",
            Vec3::new(7.5, 13.0, 35.0),
            Vec3::new(3.0, 1.5, 0.3),
            "H&M",
            NeonSign::steady(Color::srgb(1.0, 0.0, 0.0), 10.0),
        ),
        (
            "國賓影城",
            Vec3::new(41.0, 25.0, -58.0),
            Vec3::new(5.0, 1.2, 0.3),
            "國賓",
            NeonSign::flickering(Color::srgb(1.0, 0.2, 0.2), 9.0),
        ),
        (
            "樂聲影城",
            Vec3::new(36.0, 20.0, -26.0),
            Vec3::new(4.0, 1.0, 0.3),
            "樂聲",
            NeonSign::flickering(Color::srgb(0.2, 0.9, 0.9), 8.0),
        ),
        (
            "麥當勞",
            Vec3::new(-17.0, 8.0, -72.0),
            Vec3::new(2.5, 2.5, 0.3),
            "M",
            NeonSign::steady(Color::srgb(1.0, 0.8, 0.0), 12.0),
        ),
        (
            "湯姆熊",
            Vec3::new(40.0, 15.0, -64.0),
            Vec3::new(4.5, 1.0, 0.3),
            "湯姆熊",
            NeonSign::flickering(Color::srgb(1.0, 0.5, 0.1), 7.0),
        ),
        (
            "刺青店",
            Vec3::new(20.0, 8.0, -17.0),
            Vec3::new(3.5, 0.8, 0.3),
            "TATTOO",
            NeonSign::broken(Color::srgb(0.7, 0.2, 0.9), 8.0),
        ),
        (
            "潮牌店",
            Vec3::new(28.0, 10.0, -8.0),
            Vec3::new(3.0, 0.8, 0.3),
            "HYPE",
            NeonSign::steady(Color::srgb(1.0, 0.1, 0.2), 9.0),
        ),
    ];

    let count = neon_signs.len();
    for (building, position, size, text, neon) in neon_signs {
        try_spawn_neon_sign(
            commands,
            meshes,
            materials,
            building_tracker,
            building,
            position,
            size,
            text,
            neon,
        );
    }

    info!("✨ 已生成 {} 個霓虹燈招牌", count);
}

// ============================================================================
// 輔助函數
// ============================================================================

fn spawn_building_at_corner(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tracker: &mut BuildingTracker,
    spec: &BuildingSpec,
) {
    let x = spec.road1.center
        + spec.road1.align * (spec.road1.width / 2.0 + spec.width / 2.0 + BUILDING_ROAD_BUFFER);
    let z = spec.road2.center
        + spec.road2.align * (spec.road2.width / 2.0 + spec.depth / 2.0 + BUILDING_ROAD_BUFFER);
    let pos = Vec3::new(x, spec.height / 2.0, z);
    if tracker.try_record(pos, spec.width, spec.height, spec.depth, spec.name) {
        spawn_rich_building(
            commands,
            meshes,
            materials,
            pos,
            spec.width,
            spec.height,
            spec.depth,
            spec.name,
        );
    }
}

fn spawn_building_at_linear(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tracker: &mut BuildingTracker,
    road_center: f32,
    road_width: f32,
    align: f32,
    start_cross: f32,
    end_cross: f32,
    width: f32,
    depth: f32,
    name: &str,
) {
    let center_cross = f32::midpoint(start_cross, end_cross);
    let x = road_center + align * (road_width / 2.0 + width / 2.0 + BUILDING_ROAD_BUFFER);
    let z = center_cross;
    let height = 20.0;
    let pos = Vec3::new(x, height / 2.0, z);

    if tracker.try_record(pos, width, height, depth, name) {
        spawn_rich_building(commands, meshes, materials, pos, width, height, depth, name);
    }
}

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

fn try_spawn_neon_sign(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    tracker: &BuildingTracker,
    building_name: &str,
    position: Vec3,
    size: Vec3,
    text: &str,
    neon_config: NeonSign,
) {
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
