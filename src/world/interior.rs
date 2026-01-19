//! 室內建築系統
//!
//! 處理玩家進入/離開室內空間的邏輯

#![allow(dead_code)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{InteriorSpace, Door, DoorState, PlayerInteriorState, InteriorPrompt};
use crate::player::Player;
use crate::ui::{ChineseFont, NotificationQueue};
use crate::core::WorldTime;
use crate::wanted::WantedLevel;

// ============================================================================
// 系統
// ============================================================================

/// 室內互動檢測系統
/// 檢測玩家靠近門時顯示提示
pub fn interior_proximity_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    door_query: Query<(Entity, &Transform, &Door)>,
    interior_query: Query<&InteriorSpace>,
    world_time: Res<WorldTime>,
    font: Option<Res<ChineseFont>>,
    prompt_query: Query<Entity, With<InteriorPrompt>>,
) {
    let Some(font) = font else { return };
    let Ok(player_transform) = player_query.single() else { return };
    let player_pos = player_transform.translation;

    // 清除舊提示
    for entity in prompt_query.iter() {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
        }
    }

    // 檢查每個門
    for (_door_entity, door_transform, door) in door_query.iter() {
        let door_pos = door_transform.translation;
        let distance = player_pos.distance(door_pos);

        if distance < door.interact_radius {
            // 檢查是否營業中
            let is_open = if let Some(interior_entity) = door.interior_entity {
                if let Ok(interior) = interior_query.get(interior_entity) {
                    interior.is_open(world_time.hour)
                } else {
                    true
                }
            } else {
                true
            };

            // 顯示提示
            let prompt_text = if door.is_locked {
                "🔒 已上鎖"
            } else if !is_open {
                "🚫 營業時間外"
            } else {
                "按 E 進入"
            };

            // 在門上方生成提示文字
            commands.spawn((
                Text2d::new(prompt_text),
                TextFont {
                    font: font.font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.8)),
                Transform::from_translation(door_pos + Vec3::Y * 2.5)
                    .with_scale(Vec3::splat(0.015)),
                InteriorPrompt,
            ));
        }
    }
}

/// 室內進入系統
/// 處理玩家按 E 進入/離開室內
pub fn interior_enter_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&Transform, &mut PlayerInteriorState), With<Player>>,
    mut door_query: Query<(&Transform, &mut Door)>,
    interior_query: Query<&InteriorSpace>,
    world_time: Res<WorldTime>,
    mut notifications: ResMut<NotificationQueue>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    let Ok((player_transform, mut interior_state)) = player_query.single_mut() else { return };
    let player_pos = player_transform.translation;

    // 如果已在室內，嘗試離開
    if interior_state.is_inside {
        if let Some(interior_entity) = interior_state.current_interior {
            if let Ok(interior) = interior_query.get(interior_entity) {
                // 傳送到出口
                notifications.info(format!("離開 {}", interior.name));
                interior_state.is_inside = false;
                interior_state.current_interior = None;
                // 注意：實際的位置傳送需要在另一個系統中處理（因為這裡沒有可變 Transform）
            }
        }
        return;
    }

    // 尋找可進入的門
    for (door_transform, mut door) in door_query.iter_mut() {
        let door_pos = door_transform.translation;
        let distance = player_pos.distance(door_pos);

        if distance < door.interact_radius {
            // 檢查是否可進入
            if door.is_locked {
                notifications.warning("門已上鎖！");
                return;
            }

            if let Some(interior_entity) = door.interior_entity {
                if let Ok(interior) = interior_query.get(interior_entity) {
                    // 檢查營業時間
                    if !interior.is_open(world_time.hour) {
                        notifications.warning(format!("{} 營業時間外", interior.name));
                        return;
                    }

                    // 進入室內
                    notifications.success(format!("進入 {}", interior.name));
                    interior_state.is_inside = true;
                    interior_state.current_interior = Some(interior_entity);
                    door.state = DoorState::Opening;
                    return;
                }
            }
        }
    }
}

/// 室內躲藏效果系統
/// 玩家在室內時降低通緝熱度
pub fn interior_hiding_system(
    time: Res<Time>,
    player_query: Query<&PlayerInteriorState, With<Player>>,
    interior_query: Query<&InteriorSpace>,
    mut wanted: ResMut<WantedLevel>,
) {
    let Ok(interior_state) = player_query.single() else { return };

    if !interior_state.is_inside {
        return;
    }

    if let Some(interior_entity) = interior_state.current_interior {
        if let Ok(interior) = interior_query.get(interior_entity) {
            // 如果是躲藏點且通緝等級在可躲藏範圍內
            if interior.is_hiding_spot && wanted.stars <= interior.max_hide_stars {
                // 加速熱度降低
                let dt = time.delta_secs();
                let cooldown_rate = 5.0; // 每秒降低 5 點熱度
                wanted.heat = (wanted.heat - cooldown_rate * dt).max(0.0);
                wanted.stars = (wanted.heat / 20.0).ceil() as u8;
            }
        }
    }
}

/// 門動畫系統
pub fn door_animation_system(
    time: Res<Time>,
    mut door_query: Query<(&mut Door, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let open_speed = 2.0;

    for (mut door, mut transform) in door_query.iter_mut() {
        match door.state {
            DoorState::Opening => {
                // 門打開動畫（旋轉）
                let current = transform.rotation.to_euler(bevy::math::EulerRot::XYZ);
                let target_y = std::f32::consts::FRAC_PI_2; // 90 度
                let new_y = (current.1 + open_speed * dt).min(target_y);
                transform.rotation = Quat::from_euler(
                    bevy::math::EulerRot::XYZ,
                    current.0,
                    new_y,
                    current.2,
                );
                if new_y >= target_y {
                    door.state = DoorState::Open;
                }
            }
            DoorState::Closing => {
                // 門關閉動畫
                let current = transform.rotation.to_euler(bevy::math::EulerRot::XYZ);
                let new_y = (current.1 - open_speed * dt).max(0.0);
                transform.rotation = Quat::from_euler(
                    bevy::math::EulerRot::XYZ,
                    current.0,
                    new_y,
                    current.2,
                );
                if new_y <= 0.0 {
                    door.state = DoorState::Closed;
                }
            }
            _ => {}
        }
    }
}

// ============================================================================
// 生成輔助函數
// ============================================================================

/// 生成便利商店
pub fn spawn_convenience_store(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    name: &str,
) -> Entity {
    // 建築外觀材質
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.85),
        ..default()
    });
    let door_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.5, 0.7, 0.6),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // 室內空間
    let interior_entity = commands.spawn((
        Transform::from_translation(position),
        InteriorSpace::convenience_store(name, position + Vec3::new(0.0, 0.0, 3.0)),
        Name::new(format!("Interior_{}", name)),
    )).id();

    // 建築外觀
    let _building_entity = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 4.0, 6.0))),
        MeshMaterial3d(wall_material),
        Transform::from_translation(position + Vec3::Y * 2.0),
        Collider::cuboid(4.0, 2.0, 3.0),
        RigidBody::Fixed,
        Name::new(format!("Building_{}", name)),
    )).id();

    // 門
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.5, 2.5, 0.1))),
        MeshMaterial3d(door_material),
        Transform::from_translation(position + Vec3::new(0.0, 1.25, 3.0)),
        Door {
            interior_entity: Some(interior_entity),
            interact_radius: 2.5,
            ..default()
        },
        Name::new(format!("Door_{}", name)),
    ));

    interior_entity
}
