//! 道路網格佈局

use bevy::prelude::*;

use crate::world::constants::ROAD_Y;
use crate::world::roads::{spawn_road_segment, RoadType};
use crate::world::{MapLayout, RoadAxis, RoadKind};

/// 徒步區鋪面比車行道高的距離
const PAVING_LIFT: f32 = 0.15;

/// 道路材質與道路網格生成
pub(super) fn setup_roads(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    layout: &MapLayout,
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

    // === 2. 路網：資料檔的每一段路 ===
    for seg in &layout.segments {
        let (material, y, road_type) = match seg.kind {
            RoadKind::Asphalt => (road_mat.clone(), ROAD_Y, RoadType::Asphalt),
            RoadKind::Pedestrian => (
                pedestrian_mat.clone(),
                ROAD_Y + PAVING_LIFT,
                RoadType::Pedestrian,
            ),
        };
        let center = f32::midpoint(seg.from, seg.to);
        let length = seg.to - seg.from;
        let (pos, width_x, width_z) = match seg.axis {
            RoadAxis::NorthSouth => (Vec3::new(seg.at, y, center), seg.width, length),
            RoadAxis::EastWest => (Vec3::new(center, y, seg.at), length, seg.width),
        };
        spawn_road_segment(
            commands,
            meshes,
            materials,
            material,
            line_mat.clone(),
            pos,
            width_x,
            width_z,
            seg.axis,
            road_type,
        );
    }
}
