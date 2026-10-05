//! 通用建築
//!
//! 用於無特定風格的建築物

use super::{facade_palette_color, name_hash, FacadeShell};
use crate::world::{Building, BuildingType};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// 通用建築的外型（0: 方塊、1: 階梯、2: 雙塔），由店名決定，每次啟動都一樣
pub fn generic_shape_for(name: &str) -> u32 {
    // 取高位元：牆面顏色用的是低位元 (% 6)，同一段位元會讓外型被顏色決定
    (name_hash(name) >> 16) % 3
}

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
    let shape_type = generic_shape_for(name);

    // 牆面顏色、窗戶貼圖由外牆系統依 FacadeShell 套上
    let main_mat = mats.add(StandardMaterial {
        base_color: facade_palette_color(name),
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
                FacadeShell,
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
                FacadeShell,
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
                FacadeShell,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_shape_is_stable_per_name_and_in_range() {
        for name in ["阿宗麵線", "西門紅樓", "測試大樓"] {
            let shape = generic_shape_for(name);
            assert!(shape < 3, "{name} → {shape}");
            assert_eq!(shape, generic_shape_for(name));
        }
    }

    #[test]
    fn generic_shape_not_tied_to_wall_color() {
        // 色盤 6 色、外型 3 種：兩者都用 hash % n 的話，6 是 3 的倍數，外型會被顏色決定
        let palette = crate::world::FACADE_PALETTE;
        let mut shapes_per_color: Vec<Vec<u32>> = vec![Vec::new(); palette.len()];
        for i in 0..60 {
            let name = format!("店{i}");
            let color = facade_palette_color(&name);
            let idx = palette.iter().position(|c| *c == color).unwrap();
            let shape = generic_shape_for(&name);
            if !shapes_per_color[idx].contains(&shape) {
                shapes_per_color[idx].push(shape);
            }
        }
        assert!(
            shapes_per_color.iter().any(|s| s.len() >= 2),
            "每種顏色都只對應一種外型：{shapes_per_color:?}"
        );
    }

    #[test]
    fn generic_shapes_vary_across_names() {
        let names = [
            "阿宗麵線",
            "西門紅樓",
            "測試大樓",
            "萬國戲院",
            "老天祿",
            "成都楊桃冰",
            "蜂大咖啡",
            "美觀園",
        ];
        let mut seen = [false; 3];
        for n in names {
            if let Some(s) = seen.get_mut(generic_shape_for(n) as usize) {
                *s = true;
            }
        }
        assert!(
            seen.iter().filter(|s| **s).count() >= 2,
            "8 個店名都是同一種外型"
        );
    }
}
