//! 娛樂類建築
//!
//! 萬年大樓、唐吉訶德、電影院、遊戲中心、夾娃娃機店

use super::{spawn_building_base, BuildingMaterialConfig, BuildingParams};
use crate::world::{Building, BuildingType};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// 萬年大樓 (轉角圓柱風格)
pub fn spawn_wannien(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let corner_radius = w.min(d) * 0.4;

    cmd.spawn((
        // 主體部分 (稍微內縮，讓轉角突顯)
        Mesh3d(meshes.add(Cuboid::new(w * 0.9, h, d * 0.9))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.9, 0.95), // 米白
            perceptual_roughness: 0.6,
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
        // 轉角圓柱 (Cylinder Corner)
        let cyl_h = h * 1.1; // 比樓高一點
        parent.spawn((
            Mesh3d(meshes.add(Cylinder::new(corner_radius, cyl_h))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.8, 0.9), // 稍微深一點
                ..default()
            })),
            // 放在轉角處 (假設是正向轉角)
            Transform::from_xyz(w / 2.0 - corner_radius, 0.0, d / 2.0 - corner_radius),
            GlobalTransform::default(),
        ));

        // 頂樓旋轉招牌 (Torus/Ring)
        let ring_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.0, 1.0), // Blue
            emissive: LinearRgba::new(0.0, 0.0, 1.0, 1.0) * 5.0,
            ..default()
        });
        parent.spawn((
            Mesh3d(meshes.add(Torus::new(corner_radius * 0.8, 0.5))),
            MeshMaterial3d(ring_mat),
            Transform::from_xyz(
                w / 2.0 - corner_radius,
                h / 2.0 + 2.0,
                d / 2.0 - corner_radius,
            ),
            GlobalTransform::default(),
        ));

        // 側面大型廣告看板
        let billboard_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 1.0), // White
            emissive: LinearRgba::new(1.0, 1.0, 1.0, 1.0) * 2.0,
            ..default()
        });
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.8, h * 0.6, 0.5))),
            MeshMaterial3d(billboard_mat),
            Transform::from_xyz(-0.5, 0.0, d / 2.0 * 0.9 + 0.3), // 貼在正面
            GlobalTransform::default(),
        ));
    });
}

/// 唐吉訶德 (雜亂招牌風格)
#[allow(clippy::cast_precision_loss)]
pub fn spawn_donki(
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
        base_color: Color::srgb(1.0, 0.8, 0.0), // 唐吉鮮黃
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        use rand::{Rng, SeedableRng};

        // 生成大量隨機突出的招牌
        let sign_mat_1 = mats.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.0, 0.0),
            emissive: LinearRgba::new(0.1, 0.1, 0.1, 1.0),
            ..default()
        });
        let sign_mat_2 = mats.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.0, 1.0),
            emissive: LinearRgba::new(0.0, 0.0, 1.0, 1.0) * 3.0,
            ..default()
        });

        // 以店名雜湊當種子：招牌排列每次啟動都一樣
        let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(super::name_hash(name)));

        for i in 0..10 {
            let sx = rng.random_range(1.0..3.0);
            let sy = rng.random_range(1.0..3.0);
            // 隨機位置貼在表面
            let offset_x = rng.random_range(-w / 2.0..w / 2.0);
            let offset_y = rng.random_range(-h / 2.0..h / 2.0);
            let is_blue = rng.random_bool(0.3);
            // 每個招牌 Z 軸稍微不同，避免 Z-fighting
            let z_offset = 0.2 + (i as f32) * 0.05;

            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(sx, sy, 0.2))),
                MeshMaterial3d(if is_blue {
                    sign_mat_2.clone()
                } else {
                    sign_mat_1.clone()
                }),
                Transform::from_xyz(offset_x, offset_y, d / 2.0 + z_offset)
                    .with_rotation(Quat::from_rotation_z(rng.random_range(-0.2..0.2))), // 稍微歪一點
                GlobalTransform::default(),
            ));
        }

        // 頂部大企鵝招牌 (簡化為圓球)
        parent.spawn((
            Mesh3d(meshes.add(Sphere::new(2.5))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.0, 1.0),
                emissive: LinearRgba::new(0.0, 0.0, 1.0, 1.0) * 2.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 + 2.5, d / 2.0),
            GlobalTransform::default(),
        ));
    });
}

/// 電影院 (Cinema)
#[allow(clippy::cast_precision_loss)]
pub fn spawn_cinema(
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
        base_color: Color::srgb(0.1, 0.05, 0.1), // 暗色背景
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 電影海報看板
        let poster_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.1, 0.5), // 假裝是海報色
            emissive: LinearRgba::new(0.8, 0.1, 0.5, 1.0) * 2.0,
            ..default()
        });

        // 正面掛三個大海報
        let poster_w = w / 3.5;
        let poster_h = h * 0.6;
        for i in 0..3 {
            let x_offset = -w / 3.0 + (i as f32) * (w / 3.0);
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(poster_w, poster_h, 0.2))),
                MeshMaterial3d(poster_mat.clone()),
                Transform::from_xyz(x_offset, 0.0, d / 2.0 + 0.2),
                GlobalTransform::default(),
            ));
        }
    });
}

/// 遊戲中心 (Game Center) - 湯姆熊/彈珠台
#[allow(clippy::cast_precision_loss)]
pub fn spawn_game_center(
    cmd: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    w: f32,
    h: f32,
    d: f32,
    name: &str,
) {
    let is_pachinko = name.contains("彈珠台");
    let main_color = if is_pachinko {
        Color::srgb(0.8, 0.7, 0.2) // 金黃色 (彈珠台風格)
    } else {
        Color::srgb(1.0, 0.4, 0.1) // 橘色 (湯姆熊風格)
    };

    let params = BuildingParams { pos, w, h, d, name };
    let config = BuildingMaterialConfig {
        base_color: main_color,
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        // 閃爍的霓虹燈條
        let neon_mat = mats.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0),
            emissive: LinearRgba::new(1.0, 0.0, 1.0, 1.0) * 5.0,
            ..default()
        });

        for i in 0..4 {
            let y_off = -h / 2.0 + (i as f32 + 1.0) * (h / 5.0);
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(w + 0.1, 0.3, d + 0.1))),
                MeshMaterial3d(neon_mat.clone()),
                Transform::from_xyz(0.0, y_off, 0.0),
                GlobalTransform::default(),
            ));
        }

        // 大型招牌
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.8, 2.0, 0.5))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(1.0, 0.8, 0.3, 1.0) * 4.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 + 1.5, d / 2.0),
            GlobalTransform::default(),
        ));
    });
}

/// 夾娃娃機店 (Claw Machine)
pub fn spawn_claw_machine(
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
        base_color: Color::srgb(0.9, 0.5, 0.9), // 粉紫色
        ..default()
    };
    spawn_building_base(cmd, meshes, mats, &params, config).with_children(|parent| {
        use rand::{Rng, SeedableRng};

        // 彩色閃爍燈
        let colors = [
            Color::srgb(1.0, 0.2, 0.5),
            Color::srgb(0.2, 1.0, 0.5),
            Color::srgb(0.5, 0.2, 1.0),
            Color::srgb(1.0, 1.0, 0.2),
        ];

        // 以店名雜湊當種子：燈球排列每次啟動都一樣
        let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(super::name_hash(name)));

        for i in 0..8 {
            let color = colors[i % 4];
            let x_off = rng.random_range(-w / 2.0 + 0.5..w / 2.0 - 0.5);
            let y_off = rng.random_range(-h / 3.0..h / 3.0);

            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.3))),
                MeshMaterial3d(mats.add(StandardMaterial {
                    base_color: color,
                    emissive: LinearRgba::from(color) * 5.0,
                    ..default()
                })),
                Transform::from_xyz(x_off, y_off, d / 2.0 + 0.2),
                GlobalTransform::default(),
            ));
        }

        // 招牌
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(w * 0.7, 1.2, 0.3))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.0),
                emissive: LinearRgba::new(1.0, 0.8, 0.0, 1.0) * 3.0,
                ..default()
            })),
            Transform::from_xyz(0.0, h / 2.0 + 0.8, 0.0),
            GlobalTransform::default(),
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::primitives::MeshAabb;

    /// 生成一棟樓，回傳所有子實體的位置、旋轉、縮放、mesh 外框與底色（排序後的文字）
    fn child_parts(
        spawn: impl Fn(&mut Commands, &mut ResMut<Assets<Mesh>>, &mut ResMut<Assets<StandardMaterial>>)
            + Send
            + Sync
            + 'static,
    ) -> Vec<String> {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<Image>()
            .add_systems(
                Startup,
                move |mut commands: Commands,
                      mut meshes: ResMut<Assets<Mesh>>,
                      mut materials: ResMut<Assets<StandardMaterial>>| {
                    spawn(&mut commands, &mut meshes, &mut materials);
                },
            );
        app.update();
        let world = app.world_mut();
        let parts: Vec<(
            Transform,
            Option<Handle<Mesh>>,
            Option<Handle<StandardMaterial>>,
        )> = world
            .query_filtered::<(
                &Transform,
                Option<&Mesh3d>,
                Option<&MeshMaterial3d<StandardMaterial>>,
            ), With<ChildOf>>()
            .iter(world)
            .map(|(t, mesh, material)| {
                (*t, mesh.map(|m| m.0.clone()), material.map(|m| m.0.clone()))
            })
            .collect();
        let meshes = world.resource::<Assets<Mesh>>();
        let materials = world.resource::<Assets<StandardMaterial>>();
        let mut out: Vec<String> = parts
            .iter()
            .map(|(t, mesh, material)| {
                let half_extents = mesh
                    .as_ref()
                    .and_then(|h| meshes.get(h))
                    .and_then(MeshAabb::compute_aabb)
                    .map(|aabb| aabb.half_extents);
                let color = material
                    .as_ref()
                    .and_then(|h| materials.get(h))
                    .map(|m| m.base_color);
                format!(
                    "{:?}",
                    (t.translation, t.rotation, t.scale, half_extents, color)
                )
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn donki_signs_same_every_launch() {
        let donki = || {
            child_parts(|c, m, s| {
                spawn_donki(c, m, s, Vec3::ZERO, 28.0, 35.0, 22.0, "Don Don Donki");
            })
        };
        let first = donki();
        assert_eq!(first.len(), 11, "招牌 10 個＋企鵝球 1 個");
        assert_eq!(first, donki());
    }

    #[test]
    fn claw_machine_lights_same_every_launch() {
        let claw = || {
            child_parts(|c, m, s| {
                spawn_claw_machine(c, m, s, Vec3::ZERO, 8.0, 10.0, 8.0, "夾娃娃機");
            })
        };
        let first = claw();
        assert_eq!(first.len(), 9, "燈球 8 個＋招牌 1 個");
        assert_eq!(first, claw());
    }
}
