//! 爆炸物輸入與更新系統
//!
//! 投擲輸入、軌跡計算、投擲事件處理、爆炸物類型更新、黏性炸彈引爆。

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::*;
use crate::core::CameraSettings;
use crate::player::Player;

// ============================================================================
// 輸入與投擲系統
// ============================================================================

/// 投擲輸入處理系統
pub fn explosive_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    camera_settings: Res<CameraSettings>,
    mut player_query: Query<(Entity, &Transform, &mut ExplosiveInventory), With<Player>>,
    mut throw_state: ResMut<ThrowPreviewState>,
    mut throw_events: MessageWriter<ThrowExplosiveEvent>,
) {
    let Ok((player_entity, player_transform, mut inventory)) = player_query.single_mut() else {
        return;
    };

    // 切換爆炸物類型：Tab 鍵
    if keyboard.just_pressed(KeyCode::Tab) {
        inventory.cycle_next();
        if let Some(selected) = inventory.selected {
            info!("切換到: {}", selected.name());
        }
    }

    // 更新冷卻
    if inventory.throw_cooldown > 0.0 {
        inventory.throw_cooldown -= time.delta_secs();
    }

    // 沒有選擇爆炸物或冷卻中則不處理投擲
    if !inventory.has_selected() || inventory.throw_cooldown > 0.0 {
        throw_state.is_previewing = false;
        return;
    }

    let Some(selected) = inventory.selected else {
        return;
    };

    // G 鍵：投擲
    if keyboard.pressed(KeyCode::KeyG) {
        // 蓄力中
        throw_state.is_previewing = true;
        throw_state.charge_time += time.delta_secs();

        // 計算投擲方向
        let throw_dir = Vec3::new(
            camera_settings.yaw.cos(),
            camera_settings.pitch.sin().max(0.1), // 至少向上一點
            camera_settings.yaw.sin(),
        )
        .normalize();
        throw_state.throw_direction = throw_dir;

        // 計算軌跡預覽
        let force = selected.throw_force() * throw_state.charge_multiplier();
        let origin = player_transform.translation + Vec3::Y * 1.5;
        throw_state.trajectory_points = calculate_trajectory(origin, throw_dir * force);
        throw_state.predicted_landing = throw_state.trajectory_points.last().copied();
    } else if keyboard.just_released(KeyCode::KeyG) && throw_state.is_previewing {
        // 釋放：投擲
        let force = selected.throw_force() * throw_state.charge_multiplier();
        let origin = player_transform.translation + Vec3::Y * 1.5;

        throw_events.write(ThrowExplosiveEvent {
            thrower: player_entity,
            explosive_type: selected,
            origin,
            direction: throw_state.throw_direction,
            force,
        });

        inventory.consume_selected();
        inventory.throw_cooldown = THROW_COOLDOWN;

        // 重置預覽狀態
        throw_state.is_previewing = false;
        throw_state.charge_time = 0.0;
        throw_state.trajectory_points.clear();
    }
}

/// 計算投擲軌跡
fn calculate_trajectory(origin: Vec3, initial_velocity: Vec3) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(TRAJECTORY_SEGMENTS);
    let gravity = Vec3::new(0.0, -9.81, 0.0);

    let mut pos = origin;
    let mut vel = initial_velocity;

    for _ in 0..TRAJECTORY_SEGMENTS {
        points.push(pos);
        vel += gravity * TRAJECTORY_TIME_STEP;
        pos += vel * TRAJECTORY_TIME_STEP;

        // 如果碰到地面就停止
        if pos.y < 0.1 {
            pos.y = 0.1;
            points.push(pos);
            break;
        }
    }

    points
}

/// 處理投擲事件
pub fn handle_throw_event_system(
    mut commands: Commands,
    mut throw_events: MessageReader<ThrowExplosiveEvent>,
    visuals: Option<Res<ExplosiveVisuals>>,
) {
    let Some(visuals) = visuals else {
        return;
    };

    for event in throw_events.read() {
        let (mesh, material, explosive) = match event.explosive_type {
            ExplosiveType::Grenade => (
                visuals.grenade_mesh.clone(),
                visuals.grenade_material.clone(),
                Explosive::grenade(event.thrower),
            ),
            ExplosiveType::Molotov => (
                visuals.molotov_mesh.clone(),
                visuals.molotov_material.clone(),
                Explosive::molotov(event.thrower),
            ),
            ExplosiveType::StickyBomb => (
                visuals.sticky_mesh.clone(),
                visuals.sticky_material.clone(),
                Explosive::sticky_bomb(event.thrower),
            ),
            ExplosiveType::Rocket => continue, // 火箭由 RPG 系統直接生成，不走投擲流程
        };

        // 生成爆炸物實體
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(event.origin),
            RigidBody::Dynamic,
            Collider::ball(0.08),
            Restitution::coefficient(0.3),
            Friction::coefficient(0.5),
            ExternalImpulse {
                impulse: event.direction * event.force,
                ..default()
            },
            CollisionGroups::new(Group::GROUP_3, Group::ALL),
            explosive,
        ));

        info!("投擲 {}", event.explosive_type.name());
    }
}

// ============================================================================
// 爆炸物更新輔助函數
// ============================================================================
/// 更新手榴彈：倒數計時並引爆
/// 返回 true 表示已引爆需要銷毀實體
#[inline]
fn update_grenade(
    explosive: &mut Explosive,
    delta_secs: f32,
    position: Vec3,
    explosion_events: &mut MessageWriter<ExplosionEvent>,
) -> bool {
    if !explosive.armed {
        return false;
    }

    explosive.fuse_timer -= delta_secs;
    if explosive.fuse_timer > 0.0 {
        return false;
    }

    // 引爆
    explosion_events.write(ExplosionEvent {
        position,
        radius: GRENADE_EXPLOSION_RADIUS,
        max_damage: GRENADE_DAMAGE,
        explosive_type: ExplosiveType::Grenade,
        source: explosive.thrower,
    });
    true
}

/// 更新燃燒瓶：撞擊即爆
/// 返回 true 表示已引爆需要銷毀實體
#[inline]
fn update_molotov(
    explosive: &Explosive,
    colliding: Option<&CollidingEntities>,
    position: Vec3,
    explosion_events: &mut MessageWriter<ExplosionEvent>,
) -> bool {
    if !explosive.armed {
        return false;
    }

    let Some(colliding) = colliding else {
        return false;
    };
    if colliding.is_empty() {
        return false;
    }

    // 撞擊地面或物體
    explosion_events.write(ExplosionEvent {
        position,
        radius: MOLOTOV_FIRE_RADIUS,
        max_damage: MOLOTOV_DPS,
        explosive_type: ExplosiveType::Molotov,
        source: explosive.thrower,
    });
    true
}

/// 更新黏性炸彈：檢查並附著到目標
/// 返回 true 表示已附著，需要移除物理組件
#[inline]
fn update_sticky_bomb(explosive: &mut Explosive, colliding: Option<&CollidingEntities>) -> bool {
    if explosive.attached {
        return false;
    }

    let Some(colliding) = colliding else {
        return false;
    };
    let Some(attached_entity) = colliding.iter().next() else {
        return false;
    };

    explosive.attached = true;
    explosive.attached_to = Some(attached_entity);
    true
}

/// 爆炸物更新系統
pub fn explosive_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut explosive_query: Query<(
        Entity,
        &Transform,
        &mut Explosive,
        Option<&CollidingEntities>,
    )>,
    mut explosion_events: MessageWriter<ExplosionEvent>,
) {
    let delta_secs = time.delta_secs();

    for (entity, transform, mut explosive, colliding) in &mut explosive_query {
        let position = transform.translation;

        match explosive.explosive_type {
            ExplosiveType::Grenade => {
                if update_grenade(&mut explosive, delta_secs, position, &mut explosion_events) {
                    commands.entity(entity).despawn();
                }
            }
            ExplosiveType::Molotov => {
                if update_molotov(&explosive, colliding, position, &mut explosion_events) {
                    commands.entity(entity).despawn();
                }
            }
            ExplosiveType::StickyBomb => {
                if update_sticky_bomb(&mut explosive, colliding) {
                    // 移除物理，附著到目標
                    commands
                        .entity(entity)
                        .remove::<RigidBody>()
                        .remove::<ExternalImpulse>();
                }
            }
            ExplosiveType::Rocket => {} // 火箭由 rpg_projectile_update_system 處理
        }
    }
}

/// 引爆黏性炸彈系統
pub fn detonate_sticky_bomb_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    sticky_query: Query<(Entity, &Transform, &Explosive)>,
    mut explosion_events: MessageWriter<ExplosionEvent>,
) {
    // H 鍵：引爆所有已附著的黏性炸彈
    if keyboard.just_pressed(KeyCode::KeyH) {
        for (entity, transform, explosive) in &sticky_query {
            if explosive.explosive_type == ExplosiveType::StickyBomb && explosive.attached {
                explosion_events.write(ExplosionEvent {
                    position: transform.translation,
                    radius: STICKY_EXPLOSION_RADIUS,
                    max_damage: STICKY_DAMAGE,
                    explosive_type: ExplosiveType::StickyBomb,
                    source: explosive.thrower,
                });
                commands.entity(entity).despawn();
                info!("引爆黏性炸彈!");
            }
        }
    }
}
