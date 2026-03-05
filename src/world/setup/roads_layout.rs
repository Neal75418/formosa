//! 道路網格佈局

use bevy::prelude::*;

use crate::world::constants::{
    ROAD_Y, W_ALLEY, W_MAIN, W_PEDESTRIAN, W_SECONDARY, W_ZHONGHUA, X_HAN, X_KANGDING, X_XINING,
    X_ZHONGHUA, Z_CHENGDU, Z_EMEI, Z_HANKOU, Z_KUNMING, Z_WUCHANG,
};
use crate::world::roads::{spawn_road_segment, RoadType};

/// 道路材質與道路網格生成
#[allow(clippy::too_many_lines)]
pub(super) fn setup_roads(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
) {
    // === 道路材質 (支援貼圖載入) ===

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
    let hanzhong_center_z = f32::midpoint(Z_WUCHANG, Z_CHENGDU);
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
    let ped_west_edge = X_XINING + W_SECONDARY / 2.0;
    let ped_east_edge = X_ZHONGHUA - W_ZHONGHUA / 2.0;
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

    // 昆明街 - 小巷 (分東西兩段)
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
