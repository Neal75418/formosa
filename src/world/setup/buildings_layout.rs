//! 建築與霓虹燈配置

use bevy::prelude::*;

use crate::world::buildings::spawn_rich_building;
use crate::world::constants::BuildingTracker;
use crate::world::{spawn_neon_sign, MapLayout, NeonSign};

// ============================================================================
// 建築生成
// ============================================================================

/// 地標建築與商店生成
pub(super) fn setup_buildings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    building_tracker: &mut BuildingTracker,
    layout: &MapLayout,
) {
    // === 地標與商店：資料檔的建築清單，順序＝生成順序（重疊時先蓋先贏）===
    for b in &layout.buildings {
        try_spawn_rich_building(
            commands,
            meshes,
            materials,
            building_tracker,
            b.pos,
            b.size.x,
            b.size.y,
            b.size.z,
            &b.name,
        );
    }
    info!("🏢 已新增 {} 棟建築", layout.buildings.len());
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
