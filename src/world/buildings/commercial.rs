//! 商業類建築
//!
//! 便利商店、速食店、大創

use bevy::prelude::*;
use crate::world::{Door, InteriorSpace};
use super::{BuildingParams, BuildingMaterialConfig, spawn_building_base};

/// 速食店 (Fast Food) - 麥當勞/摩斯/肯德基
pub fn spawn_fast_food(
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

/// 便利商店 (Convenience Store)
pub fn spawn_convenience_store(
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

/// 大創 (Daiso)
pub fn spawn_daiso(
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
