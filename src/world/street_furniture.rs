//! 街道設施系統
//!
//! 路燈、自動販賣機、垃圾桶、電影看板、塗鴉牆、停車場

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::world::{Building, BuildingType, StreetFurniture, StreetFurnitureType, StreetLight};

/// 路燈 (Lamppost)
pub fn spawn_lamppost(
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

/// 自動販賣機 (Vending Machine)
pub fn spawn_vending_machine(
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

/// 垃圾桶 (Trash Can)
pub fn spawn_trash_can(
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

/// 電影看板 (Movie Billboard)
pub fn spawn_movie_billboard(
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

/// 塗鴉牆 (Graffiti Wall)
pub fn spawn_graffiti_wall(
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

/// 停車場 (Parking Garage)
pub fn spawn_parking_garage(
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
}
