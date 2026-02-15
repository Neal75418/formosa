//! 直升機戰鬥、旋翼動畫、探照燈、傷害、清理系統

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};
use crate::core::rapier_real_to_f32;
use crate::combat::{
    DamageEvent, DamageSource, Health,
    CombatVisuals, TracerStyle, spawn_bullet_tracer, spawn_muzzle_flash,
};
use crate::player::Player;
use super::components::*;
use super::super::WantedLevel;

// ============================================================================
// 戰鬥系統
// ============================================================================

/// 檢查直升機是否可以射擊
fn can_helicopter_fire(helicopter: &PoliceHelicopter, distance_sq: f32) -> bool {
    helicopter.state == HelicopterState::Attacking
        && helicopter.fire_cooldown <= 0.0
        && distance_sq <= HELICOPTER_ATTACK_RANGE_SQ
}

/// 計算槍口位置
fn calc_muzzle_position(heli_pos: Vec3, forward: Dir3) -> Vec3 {
    heli_pos + forward * 2.0 + Vec3::new(0.0, -1.0, 0.0)
}

/// 直升機射擊系統
pub fn helicopter_combat_system(
    mut commands: Commands,
    time: Res<Time>,
    visuals: Res<CombatVisuals>,
    mut damage_events: MessageWriter<DamageEvent>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    mut helicopter_query: Query<(Entity, &mut PoliceHelicopter, &Transform)>,
    rapier_context: ReadRapierContext,
) {
    let dt = time.delta_secs();

    let Ok((player_entity, player_transform)) = player_query.single() else { return; };
    let player_pos = player_transform.translation;
    let Ok(rapier) = rapier_context.single() else { return; };

    for (heli_entity, mut helicopter, transform) in helicopter_query.iter_mut() {
        helicopter.fire_cooldown = (helicopter.fire_cooldown - dt).max(0.0);

        let heli_pos = transform.translation;
        let to_player = player_pos - heli_pos;
        let distance_sq = to_player.length_squared();

        if !can_helicopter_fire(&helicopter, distance_sq) {
            continue;
        }

        let distance = distance_sq.sqrt();
        let direction = to_player.normalize();
        let muzzle_pos = calc_muzzle_position(heli_pos, transform.forward());

        spawn_muzzle_flash(&mut commands, &visuals, muzzle_pos);

        // 計算子彈終點
        let tracer_end = rapier
            .cast_ray(muzzle_pos, direction, HELICOPTER_ATTACK_RANGE as RapierReal, true, QueryFilter::default())
            .map(|(_, toi)| muzzle_pos + direction * rapier_real_to_f32(toi))
            .unwrap_or_else(|| muzzle_pos + direction * HELICOPTER_ATTACK_RANGE);

        spawn_bullet_tracer(&mut commands, &visuals, muzzle_pos, tracer_end, TracerStyle::SMG);

        // 傷害判定
        if let Some((hit_entity, _)) = rapier.cast_ray(
            muzzle_pos, direction, distance as RapierReal, true, QueryFilter::default()
        ) {
            if hit_entity == player_entity {
                damage_events.write(DamageEvent {
                    target: player_entity,
                    amount: HELICOPTER_BULLET_DAMAGE,
                    source: DamageSource::Bullet,
                    attacker: Some(heli_entity),
                    hit_position: Some(player_pos),
                    is_headshot: false,
                    force_knockback: false,
                });
            }
        }

        helicopter.fire_cooldown = 1.0 / HELICOPTER_FIRE_RATE;
    }
}

// ============================================================================
// 旋翼動畫系統
// ============================================================================

/// 旋翼旋轉動畫系統
pub fn rotor_animation_system(
    time: Res<Time>,
    mut rotor_query: Query<(&mut Transform, &HelicopterRotor, &HelicopterParent)>,
    helicopter_query: Query<&PoliceHelicopter>,
) {
    let dt = time.delta_secs();

    for (mut transform, rotor, parent) in rotor_query.iter_mut() {
        // 檢查父直升機是否墜毀
        let is_crashing = helicopter_query
            .get(parent.0)
            .map(|h| h.state == HelicopterState::Crashing)
            .unwrap_or(false);

        // 墜毀時旋翼逐漸減速
        let speed_mult = if is_crashing { 0.3 } else { 1.0 };

        // 旋轉
        let rotation_amount = rotor.rotation_speed * speed_mult * dt;
        if rotor.is_main_rotor {
            transform.rotate_y(rotation_amount.to_radians());
        } else {
            transform.rotate_x(rotation_amount.to_radians());
        }
    }
}

// ============================================================================
// 探照燈系統
// ============================================================================

/// 探照燈追蹤系統
pub fn spotlight_tracking_system(
    player_query: Query<&Transform, With<Player>>,
    helicopter_query: Query<(&Transform, &PoliceHelicopter), Without<Player>>,
    mut spotlight_query: Query<(&mut Transform, &HelicopterSpotlight, &HelicopterParent), (Without<PoliceHelicopter>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;

    for (mut spotlight_transform, _spotlight, parent) in spotlight_query.iter_mut() {
        // 取得父直升機位置
        let Ok((heli_transform, helicopter)) = helicopter_query.get(parent.0) else { continue };

        // 墜毀時不追蹤
        if helicopter.state == HelicopterState::Crashing {
            continue;
        }

        // 計算從直升機到玩家的方向
        let heli_pos = heli_transform.translation;
        let to_player = player_pos - heli_pos;

        // 探照燈朝向玩家（在本地座標系）
        let local_target = heli_transform.rotation.inverse() * to_player;
        if local_target.length() > 0.1 {
            spotlight_transform.look_at(local_target.normalize() * 10.0, Vec3::Y);
        }
    }
}

// ============================================================================
// 傷害系統
// ============================================================================

/// 直升機受傷系統
pub fn helicopter_damage_system(
    time: Res<Time>,
    mut commands: Commands,
    mut helicopter_query: Query<(Entity, &mut PoliceHelicopter, &Health, &Transform)>,
    mut spawn_state: ResMut<HelicopterSpawnState>,
) {
    let current_time = time.elapsed_secs();

    for (entity, mut helicopter, health, transform) in helicopter_query.iter_mut() {
        // 同步生命值
        if health.current < helicopter.health {
            let damage_taken = helicopter.health - health.current;
            helicopter.health = health.current;
            helicopter.last_hit_time = current_time;

            info!("直升機受傷: -{:.0} HP, 剩餘: {:.0}", damage_taken, helicopter.health);
        }

        // 檢查是否墜毀
        if helicopter.health <= 0.0 && helicopter.state != HelicopterState::Crashing {
            helicopter.state = HelicopterState::Crashing;
            helicopter.crash_velocity = Vec3::new(
                (rand::random::<f32>() - 0.5) * 10.0,
                -15.0,
                (rand::random::<f32>() - 0.5) * 10.0,
            );

            warn!("警用直升機被擊落！");
        }

        // 墜毀到地面後移除
        if helicopter.state == HelicopterState::Crashing && transform.translation.y <= 0.5 {
            commands.entity(entity).despawn();
            spawn_state.count = spawn_state.count.saturating_sub(1);
            info!("🚁 直升機墜毀！");
        }
    }
}

// ============================================================================
// 清理系統
// ============================================================================

/// 直升機清理系統（脫離通緝時）
pub fn despawn_helicopter_system(
    mut commands: Commands,
    wanted: Res<WantedLevel>,
    helicopter_query: Query<Entity, With<PoliceHelicopter>>,
    mut spawn_state: ResMut<HelicopterSpawnState>,
) {
    // 通緝等級低於閾值時移除所有直升機
    if wanted.stars >= HELICOPTER_SPAWN_WANTED_LEVEL {
        return;
    }

    for entity in helicopter_query.iter() {
        commands.entity(entity).despawn();
    }

    spawn_state.count = 0;
    spawn_state.cooldown = 0.0;
}
