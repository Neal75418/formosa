//! 警察 AI 狀態機

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::ai::AiMovement;
use crate::player::Player;

#[allow(clippy::wildcard_imports)]
use super::super::components::*;
#[allow(clippy::wildcard_imports)]
use super::super::config::*;

// ============================================================================
// 輔助函數
// ============================================================================

fn calc_movement_velocity(direction: Vec3, speed: f32, dt: f32) -> Vec3 {
    Vec3::new(
        direction.x * speed * dt,
        -9.81 * dt,
        direction.z * speed * dt,
    )
}

fn calc_facing_rotation(direction: Vec3) -> Quat {
    Quat::from_rotation_y((-direction.x).atan2(-direction.z))
}

fn handle_patrolling_state(
    officer: &mut PoliceOfficer,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    police_pos: Vec3,
    wanted_stars: u8,
    dt: f32,
) {
    if wanted_stars > 0 {
        officer.state = PoliceState::Alerted;
        officer.target_player = true;
        return;
    }

    if officer.patrol_route.is_empty() {
        controller.translation = Some(Vec3::ZERO);
        return;
    }

    let target = officer.patrol_route[officer.patrol_index];
    let to_target = target - police_pos;
    let distance = to_target.length();

    if distance < PATROL_WAYPOINT_THRESHOLD {
        officer.patrol_index = (officer.patrol_index + 1) % officer.patrol_route.len();
        return;
    }

    let direction = to_target.normalize();
    controller.translation = Some(direction * PATROL_SPEED * dt);
    let target_yaw = (-direction.x).atan2(-direction.z);
    let target_rotation = Quat::from_rotation_y(target_yaw);
    transform.rotation = transform.rotation.slerp(target_rotation, dt * 5.0);
}

fn handle_alerted_state(
    officer: &mut PoliceOfficer,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    police_pos: Vec3,
    distance: f32,
    movement: &AiMovement,
    wanted: &WantedLevel,
    config: &PoliceConfig,
    dt: f32,
) {
    let target_pos = officer.radio_alert_position.or(wanted.search_center);

    if let Some(search_center) = target_pos {
        let to_search = search_center - police_pos;
        let search_dist = to_search.length();

        if search_dist > 2.0 {
            let move_dir = to_search.normalize();
            let speed = if officer.radio_alerted {
                movement.run_speed
            } else {
                movement.walk_speed * 1.5
            };
            controller.translation = Some(calc_movement_velocity(move_dir, speed, dt));
            transform.rotation = transform
                .rotation
                .slerp(calc_facing_rotation(move_dir), 5.0 * dt);
        } else {
            officer.radio_alerted = false;
            officer.radio_alert_position = None;
        }
    }

    if distance < config.vision_range && officer.can_see_player {
        officer.state = PoliceState::Pursuing;
        officer.radio_alerted = false;
        officer.radio_alert_position = None;
    }
}

fn handle_pursuing_state(
    officer: &mut PoliceOfficer,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    direction: Vec3,
    distance: f32,
    movement: &AiMovement,
    player_visible: bool,
    attack_range: f32,
    dt: f32,
) {
    if distance > attack_range {
        controller.translation = Some(calc_movement_velocity(direction, movement.run_speed, dt));
        transform.rotation = transform
            .rotation
            .slerp(calc_facing_rotation(direction), 8.0 * dt);
    } else {
        officer.state = PoliceState::Engaging;
    }

    if !player_visible {
        officer.state = PoliceState::Searching;
        officer.search_timer = 0.0;
    }
}

fn handle_searching_state(
    officer: &mut PoliceOfficer,
    controller: &mut KinematicCharacterController,
    police_pos: Vec3,
    movement: &AiMovement,
    wanted: &WantedLevel,
    dt: f32,
) {
    officer.search_timer += dt;

    if let Some(last_pos) = wanted.player_last_seen_pos {
        let to_last = last_pos - police_pos;
        if to_last.length() > 2.0 {
            controller.translation = Some(calc_movement_velocity(
                to_last.normalize(),
                movement.walk_speed,
                dt,
            ));
        }
    }

    if officer.can_see_player {
        officer.state = PoliceState::Pursuing;
    }

    if officer.search_timer > POLICE_SEARCH_RETURN_THRESHOLD && wanted.stars == 0 {
        officer.state = PoliceState::Returning;
    }
}

fn handle_engaging_state(
    officer: &mut PoliceOfficer,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    direction: Vec3,
    distance: f32,
    movement: &AiMovement,
    attack_range: f32,
    elapsed_secs: f32,
    dt: f32,
) {
    transform.rotation = transform
        .rotation
        .slerp(calc_facing_rotation(direction), 10.0 * dt);

    if distance > attack_range * 1.5 {
        officer.state = PoliceState::Pursuing;
    }

    let strafe_dir = Vec3::new(-direction.z, 0.0, direction.x);
    let strafe_speed = (elapsed_secs * 2.0).sin() * 0.3 * movement.walk_speed * dt;
    controller.translation = Some(Vec3::new(
        strafe_dir.x * strafe_speed,
        -9.81 * dt,
        strafe_dir.z * strafe_speed,
    ));
}

fn handle_returning_state(officer: &mut PoliceOfficer, wanted_stars: u8) {
    officer.state = if wanted_stars > 0 {
        PoliceState::Alerted
    } else {
        PoliceState::Patrolling
    };
}

// ============================================================================
// 主系統
// ============================================================================

/// 警察 AI 系統
pub fn police_ai_system(
    mut police_query: Query<(
        &mut Transform,
        &mut PoliceOfficer,
        &AiMovement,
        &mut KinematicCharacterController,
    )>,
    player_query: Query<&Transform, (With<Player>, Without<PoliceOfficer>)>,
    wanted: Res<WantedLevel>,
    time: Res<Time>,
    config: Res<PoliceConfig>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for (mut transform, mut officer, movement, mut controller) in &mut police_query {
        let police_pos = transform.translation;
        let to_player = player_pos - police_pos;
        let distance = to_player.length();
        let direction = if distance > 0.1 {
            to_player.normalize()
        } else {
            Vec3::ZERO
        };
        let can_see = officer.can_see_player;

        match officer.state {
            PoliceState::Patrolling => {
                handle_patrolling_state(
                    &mut officer,
                    &mut transform,
                    &mut controller,
                    police_pos,
                    wanted.stars,
                    dt,
                );
            }
            PoliceState::Alerted => {
                handle_alerted_state(
                    &mut officer,
                    &mut transform,
                    &mut controller,
                    police_pos,
                    distance,
                    movement,
                    &wanted,
                    &config,
                    dt,
                );
            }
            PoliceState::Pursuing => {
                handle_pursuing_state(
                    &mut officer,
                    &mut transform,
                    &mut controller,
                    direction,
                    distance,
                    movement,
                    can_see,
                    config.attack_range,
                    dt,
                );
            }
            PoliceState::Searching => {
                handle_searching_state(
                    &mut officer,
                    &mut controller,
                    police_pos,
                    movement,
                    &wanted,
                    dt,
                );
            }
            PoliceState::Engaging => {
                handle_engaging_state(
                    &mut officer,
                    &mut transform,
                    &mut controller,
                    direction,
                    distance,
                    movement,
                    config.attack_range,
                    elapsed,
                    dt,
                );
            }
            PoliceState::Returning => {
                handle_returning_state(&mut officer, wanted.stars);
            }
        }
    }
}
