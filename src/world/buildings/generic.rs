//! 通用建築
//!
//! 用於無特定風格的建築物

use crate::world::{Building, BuildingType, BuildingWindow};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// 通用建築 (形狀變體)
pub fn spawn_generic_building(
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
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
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
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
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
