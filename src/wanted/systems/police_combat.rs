//! 警察戰鬥與無線電通訊

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};
use crate::core::rapier_real_to_f32;

use crate::player::Player;
use crate::combat::{DamageEvent, DamageSource, CombatVisuals, spawn_bullet_tracer, spawn_muzzle_flash, TracerStyle};
use crate::core::PoliceSpatialHash;

use super::super::components::*;
use super::super::config::*;

// ============================================================================
// 戰鬥輔助函數
// ============================================================================

fn check_line_of_sight(
    rapier: &RapierContext,
    ray_origin: Vec3,
    ray_direction: Vec3,
    distance: f32,
    exclude_entity: Entity,
    player_entity: Entity,
) -> bool {
    let filter = QueryFilter::default().exclude_rigid_body(exclude_entity);

    match rapier.cast_ray(ray_origin, ray_direction, distance as RapierReal, true, filter) {
        Some((hit_entity, toi)) => hit_entity == player_entity || rapier_real_to_f32(toi) >= distance - 1.0,
        None => true,
    }
}

fn calc_hit_chance(distance: f32, config: &PoliceConfig) -> f32 {
    let distance_penalty = (distance / config.attack_range) * config.distance_hit_penalty;
    (config.base_hit_chance - distance_penalty).max(0.1)
}

fn calc_tracer_end(player_pos: Vec3, is_hit: bool) -> Vec3 {
    let target_height = player_pos + Vec3::Y * TARGET_HEIGHT_OFFSET;

    if is_hit {
        target_height
    } else {
        let miss_offset = Vec3::new(
            rand::random::<f32>() * MISS_OFFSET_RANGE - 1.0,
            rand::random::<f32>() * MISS_OFFSET_Y_RANGE - 0.5,
            rand::random::<f32>() * MISS_OFFSET_RANGE - 1.0,
        );
        target_height + miss_offset
    }
}

// ============================================================================
// 戰鬥系統
// ============================================================================

/// 警察戰鬥系統
pub fn police_combat_system(
    mut commands: Commands,
    mut police_query: Query<(Entity, &Transform, &mut PoliceOfficer)>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<PoliceOfficer>)>,
    mut damage_events: MessageWriter<DamageEvent>,
    time: Res<Time>,
    config: Res<PoliceConfig>,
    rapier_context: ReadRapierContext,
    combat_visuals: Option<Res<CombatVisuals>>,
) {
    let Ok((player_entity, player_transform)) = player_query.single() else { return; };
    let player_pos = player_transform.translation;
    let Ok(rapier) = rapier_context.single() else { return; };

    for (police_entity, transform, mut officer) in &mut police_query {
        if officer.state != PoliceState::Engaging { continue; }

        officer.attack_cooldown -= time.delta_secs();
        if officer.attack_cooldown > 0.0 { continue; }

        let police_pos = transform.translation;
        let to_player = player_pos - police_pos;
        let distance = to_player.length();
        if distance > config.attack_range { continue; }

        let ray_origin = police_pos + Vec3::Y * MUZZLE_FLASH_HEIGHT;
        let ray_direction = to_player.normalize();

        if !check_line_of_sight(&rapier, ray_origin, ray_direction, distance, police_entity, player_entity) {
            continue;
        }

        let hit_chance = calc_hit_chance(distance, &config);
        let is_hit = rand::random::<f32>() < hit_chance;
        let muzzle_pos = police_pos + Vec3::Y * 1.2 + ray_direction * MUZZLE_FORWARD_OFFSET;
        let tracer_end = calc_tracer_end(player_pos, is_hit);

        if let Some(ref visuals) = combat_visuals {
            spawn_muzzle_flash(&mut commands, visuals, muzzle_pos);
            spawn_bullet_tracer(&mut commands, visuals, muzzle_pos, tracer_end, TracerStyle::Pistol);
        }

        if is_hit {
            damage_events.write(DamageEvent {
                target: player_entity,
                amount: config.damage,
                source: DamageSource::Bullet,
                attacker: Some(police_entity),
                hit_position: Some(player_pos),
                is_headshot: false,
                force_knockback: false,
            });
        }

        officer.attack_cooldown = config.attack_cooldown;
    }
}

// ============================================================================
// 無線電呼叫系統
// ============================================================================

fn can_send_radio(officer: &PoliceOfficer) -> bool {
    (officer.state == PoliceState::Pursuing || officer.state == PoliceState::Engaging)
        && officer.radio_cooldown <= 0.0
        && officer.can_see_player
}

fn can_receive_radio(state: PoliceState) -> bool {
    matches!(state, PoliceState::Patrolling | PoliceState::Alerted | PoliceState::Searching)
}

fn collect_radio_senders(
    police_query: &mut Query<(Entity, &Transform, &mut PoliceOfficer)>,
    dt: f32,
) -> Vec<(Entity, Vec3)> {
    let mut senders = Vec::new();

    for (entity, transform, mut officer) in police_query.iter_mut() {
        if officer.radio_cooldown > 0.0 {
            officer.radio_cooldown -= dt;
        }

        if can_send_radio(&officer) {
            senders.push((entity, transform.translation));
            officer.radio_cooldown = RADIO_CALL_COOLDOWN;
            debug!("🔊 警察在 ({:.1}, {:.1}) 發送無線電呼叫", transform.translation.x, transform.translation.z);
        }
    }
    senders
}

fn collect_receivers(
    senders: &[(Entity, Vec3)],
    police_hash: &PoliceSpatialHash,
) -> Vec<Entity> {
    let mut receivers = Vec::new();

    for (sender_entity, sender_pos) in senders {
        for (receiver_entity, _, _) in police_hash.query_radius(*sender_pos, RADIO_CALL_RANGE) {
            if receiver_entity != *sender_entity && !receivers.contains(&receiver_entity) {
                receivers.push(receiver_entity);
            }
        }
    }
    receivers
}

fn notify_receivers(
    police_query: &mut Query<(Entity, &Transform, &mut PoliceOfficer)>,
    receivers: Vec<Entity>,
    player_pos: Vec3,
) {
    for receiver_entity in receivers {
        if let Ok((_, _, mut officer)) = police_query.get_mut(receiver_entity) {
            if can_receive_radio(officer.state) {
                officer.radio_alerted = true;
                officer.radio_alert_position = Some(player_pos);

                if officer.state == PoliceState::Patrolling {
                    officer.state = PoliceState::Alerted;
                    officer.target_player = true;
                }
            }
        }
    }
}

/// 警察無線電呼叫系統
pub fn police_radio_call_system(
    mut police_query: Query<(Entity, &Transform, &mut PoliceOfficer)>,
    police_hash: Res<PoliceSpatialHash>,
    player_query: Query<&Transform, (With<Player>, Without<PoliceOfficer>)>,
    wanted: Res<WantedLevel>,
    time: Res<Time>,
) {
    if wanted.stars == 0 { return; }

    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;

    let senders = collect_radio_senders(&mut police_query, time.delta_secs());
    if senders.is_empty() { return; }

    let receivers = collect_receivers(&senders, &police_hash);
    notify_receivers(&mut police_query, receivers, player_pos);
}
