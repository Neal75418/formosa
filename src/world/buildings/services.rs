//! 服務類建築
//!
//! 飯店、刺青店、誠品、現代網格店、潮流服飾店

use super::{spawn_building_base, BuildingMaterialConfig, BuildingParams};
use crate::world::{Building, BuildingType};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// 誠品 (植生牆風格)
#[allow(clippy::cast_precision_loss)]
pub fn spawn_eslite(
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

/// 現代網格 (H&M / Uniqlo)
#[allow(clippy::cast_precision_loss)]
pub fn spawn_modern_grid(
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
        InheritedVisibility::default(),
        ViewVisibility::default(),
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

/// 飯店 (Hotel)
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn spawn_hotel(
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

/// 刺青店 (Tattoo Shop)
pub fn spawn_tattoo_shop(
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

/// 潮流服飾店 (Streetwear Shop)
pub fn spawn_streetwear_shop(
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
