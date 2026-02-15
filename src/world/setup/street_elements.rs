//! 街道家具、斑馬線、特殊元素

use bevy::prelude::*;

use crate::world::WorldMaterials;
use crate::world::constants::{
    ROAD_MARKING_Y_OFFSET, ROAD_Y,
    W_MAIN, W_PEDESTRIAN, W_SECONDARY,
    X_HAN, X_KANGDING, X_XINING, X_ZHONGHUA,
    Z_CHENGDU, Z_EMEI, Z_HANKOU, Z_WUCHANG,
    ZEBRA_CROSSING_OFFSET,
};
use crate::world::characters::spawn_cover_points;
use crate::world::roads::spawn_zebra_crossing;
use crate::world::street_furniture::{
    spawn_graffiti_wall, spawn_lamppost, spawn_movie_billboard,
    spawn_trash_can, spawn_vending_machine,
};

/// 路燈、自動販賣機、垃圾桶生成
pub(super) fn setup_street_furniture(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // 路燈 (間隔約 25-30 米)
    let lamppost_positions = [
        // === 漢中街兩側 (徒步區主軸) ===
        Vec3::new(X_HAN + 8.0, 0.0, -35.0),
        Vec3::new(X_HAN + 8.0, 0.0, -10.0),
        Vec3::new(X_HAN + 8.0, 0.0, 15.0),
        Vec3::new(X_HAN + 8.0, 0.0, 40.0),
        Vec3::new(X_HAN - 8.0, 0.0, -35.0),
        Vec3::new(X_HAN - 8.0, 0.0, -10.0),
        Vec3::new(X_HAN - 8.0, 0.0, 15.0),
        Vec3::new(X_HAN - 8.0, 0.0, 40.0),
        // === 峨嵋街沿線 ===
        Vec3::new(-25.0, 0.0, Z_EMEI + 8.0),
        Vec3::new(25.0, 0.0, Z_EMEI + 8.0),
        Vec3::new(45.0, 0.0, Z_EMEI + 8.0),
        // === 中華路西側 ===
        Vec3::new(X_ZHONGHUA - 25.0, 0.0, -60.0),
        Vec3::new(X_ZHONGHUA - 25.0, 0.0, -25.0),
        Vec3::new(X_ZHONGHUA - 25.0, 0.0, 10.0),
        Vec3::new(X_ZHONGHUA - 25.0, 0.0, 40.0),
        // === 西寧路東側 ===
        Vec3::new(X_XINING + 8.0, 0.0, -55.0),
        Vec3::new(X_XINING + 8.0, 0.0, -15.0),
        Vec3::new(X_XINING + 8.0, 0.0, 25.0),
        // === 漢口街沿線 ===
        Vec3::new(-60.0, 0.0, Z_HANKOU + 8.0),
        Vec3::new(-20.0, 0.0, Z_HANKOU + 8.0),
        Vec3::new(35.0, 0.0, Z_HANKOU + 8.0),
        // === 成都路沿線 ===
        Vec3::new(-60.0, 0.0, Z_CHENGDU - 10.0),
        Vec3::new(-20.0, 0.0, Z_CHENGDU - 10.0),
        Vec3::new(35.0, 0.0, Z_CHENGDU - 10.0),
        // === 康定路東側 ===
        Vec3::new(X_KANGDING + 12.0, 0.0, -50.0),
        Vec3::new(X_KANGDING + 12.0, 0.0, -5.0),
        Vec3::new(X_KANGDING + 12.0, 0.0, 35.0),
    ];

    for pos in lamppost_positions {
        spawn_lamppost(commands, meshes, materials, pos);
    }
    info!("💡 已生成 {} 盞路燈", lamppost_positions.len());

    // 自動販賣機
    let vending_positions = [
        (Vec3::new(12.0, 0.0, -15.0), 0.0, 0u8),
        (Vec3::new(-70.0, 0.0, -15.0), 0.0, 0),
        (Vec3::new(-32.0, 0.0, -22.0), 0.0, 1),
        (Vec3::new(42.0, 0.0, 36.0), 0.0, 0),
        (Vec3::new(-75.0, 0.0, 12.0), std::f32::consts::PI, 2),
    ];

    for (pos, rot, variant) in vending_positions {
        spawn_vending_machine(commands, meshes, materials, pos, rot, variant);
    }
    info!("🥤 已生成 {} 台自動販賣機", vending_positions.len());

    // 垃圾桶
    let trash_positions = [
        Vec3::new(8.0, 0.0, -10.0),
        Vec3::new(-8.0, 0.0, -10.0),
        Vec3::new(8.0, 0.0, -55.0),
        Vec3::new(-8.0, 0.0, -55.0),
        Vec3::new(-30.0, 0.0, 12.0),
        Vec3::new(30.0, 0.0, 12.0),
    ];

    for pos in trash_positions {
        spawn_trash_can(commands, meshes, materials, pos);
    }
    info!("🗑️ 已生成 {} 個垃圾桶", trash_positions.len());
}

/// 斑馬線生成
pub(super) fn setup_zebra_crossings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    world_mats: &WorldMaterials,
) {
    let zebra_mat = world_mats.zebra_white.clone();

    let intersections = [
        (X_HAN, Z_EMEI, W_PEDESTRIAN, W_PEDESTRIAN, "漢中/峨嵋"),
        (X_HAN, Z_WUCHANG, W_PEDESTRIAN, W_PEDESTRIAN, "漢中/武昌"),
        (X_HAN, Z_CHENGDU, W_PEDESTRIAN, W_MAIN, "漢中/成都"),
        (X_XINING, Z_EMEI, W_SECONDARY, W_PEDESTRIAN, "西寧/峨嵋"),
        (X_XINING, Z_WUCHANG, W_SECONDARY, W_PEDESTRIAN, "西寧/武昌"),
        (X_XINING, Z_CHENGDU, W_SECONDARY, W_MAIN, "西寧/成都"),
    ];

    let mut zebra_count = 0;
    for (cx, cz, road_ns_w, road_ew_w, _name) in intersections {
        // 北側
        spawn_zebra_crossing(commands, meshes, &zebra_mat,
            Vec3::new(cx, ROAD_Y + ROAD_MARKING_Y_OFFSET, cz - road_ew_w / 2.0 - ZEBRA_CROSSING_OFFSET),
            road_ns_w, true);
        // 南側
        spawn_zebra_crossing(commands, meshes, &zebra_mat,
            Vec3::new(cx, ROAD_Y + ROAD_MARKING_Y_OFFSET, cz + road_ew_w / 2.0 + ZEBRA_CROSSING_OFFSET),
            road_ns_w, true);
        // 西側
        spawn_zebra_crossing(commands, meshes, &zebra_mat,
            Vec3::new(cx - road_ns_w / 2.0 - ZEBRA_CROSSING_OFFSET, ROAD_Y + ROAD_MARKING_Y_OFFSET, cz),
            road_ew_w, false);
        // 東側
        spawn_zebra_crossing(commands, meshes, &zebra_mat,
            Vec3::new(cx + road_ns_w / 2.0 + ZEBRA_CROSSING_OFFSET, ROAD_Y + ROAD_MARKING_Y_OFFSET, cz),
            road_ew_w, false);
        zebra_count += 4;
    }
    info!("🦓 已生成 {} 條斑馬線於 {} 個交叉口", zebra_count, intersections.len());
}

/// 電影看板、塗鴉牆、掩體點生成
pub(super) fn setup_special_elements(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // 電影看板
    let billboard_configs = [
        (Vec3::new(25.0, 8.0, -58.0), Color::srgb(1.0, 0.3, 0.2), "動作片"),
        (Vec3::new(35.0, 8.0, -58.0), Color::srgb(0.2, 0.5, 1.0), "科幻片"),
        (Vec3::new(45.0, 8.0, -58.0), Color::srgb(1.0, 0.8, 0.2), "喜劇片"),
        (Vec3::new(55.0, 8.0, -58.0), Color::srgb(0.6, 0.1, 0.8), "恐怖片"),
    ];

    for (pos, color, _genre) in billboard_configs {
        spawn_movie_billboard(commands, meshes, materials, pos, color);
    }
    info!("🎬 已生成 {} 個電影看板", billboard_configs.len());

    // 塗鴉牆
    let graffiti_pos = Vec3::new(X_KANGDING - W_MAIN / 2.0 - 7.5 - 2.0, 2.5, Z_EMEI + 18.0);
    spawn_graffiti_wall(commands, meshes, materials, graffiti_pos);

    // AI 掩體點
    spawn_cover_points(commands);
}
