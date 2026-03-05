//! 道路系統
//!
//! 處理道路、人行道、車道標線等的生成

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;

// ============================================================================
// 道路系統輔助結構
// ============================================================================
/// 道路類型
#[derive(PartialEq)]
pub enum RoadType {
    Asphalt,    // 柏油車道 (有黃線，兩側有人行道)
    Pedestrian, // 徒步區 (紅磚鋪面，無車道線)
}

/// 道路佈局計算結果
pub struct RoadLayout {
    pub is_horizontal: bool,
    pub road_len: f32,
    pub total_width: f32,
}

impl RoadLayout {
    /// 建立新實例
    pub fn new(width_x: f32, width_z: f32) -> Self {
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

/// 建立帶有平鋪 UV 的平面 `Mesh`
/// `tile_size`: 每個貼圖覆蓋的實際大小 (米)
pub fn create_tiled_plane(width: f32, height: f32, tile_size: f32) -> Mesh {
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

/// 生成徒步區道路
pub fn spawn_pedestrian_road(
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
pub fn spawn_lane_markings(
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
pub fn spawn_sidewalks(
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

/// 生成道路段（含車道線和人行道）
pub fn spawn_road_segment(
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
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .with_children(|parent| {
            spawn_lane_markings(parent, meshes, line_mat, &layout);
        });

    // 人行道
    spawn_sidewalks(commands, meshes, materials, pos, &layout, drive_width);
}

/// 生成斑馬線
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn spawn_zebra_crossing(
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

        let (x, z, sx, sz) = if is_east_west {
            // 東西向斑馬線：X 方向延伸，Z 方向排列條紋
            (center.x, center.z + offset, crossing_width, stripe_width)
        } else {
            // 南北向斑馬線：Z 方向延伸，X 方向排列條紋
            (center.x + offset, center.z, stripe_width, crossing_width)
        };

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(sx, 0.02, sz))),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, center.y + 0.02, z),
            GlobalTransform::default(),
        ));
    }
}
