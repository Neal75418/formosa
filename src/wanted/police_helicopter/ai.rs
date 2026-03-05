//! 直升機 AI 狀態機與移動系統

use super::super::WantedLevel;
#[allow(clippy::wildcard_imports)]
use super::components::*;
use crate::core::{rapier_real_to_f32, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE};
use crate::player::Player;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};

// ============================================================================
// AI 系統
// ============================================================================

/// 計算水平距離平方
fn calc_horizontal_distance_sq(pos1: Vec3, pos2: Vec3) -> f32 {
    Vec2::new(pos1.x - pos2.x, pos1.z - pos2.z).length_squared()
}

/// 處理接近狀態
fn handle_approaching_state(helicopter: &mut PoliceHelicopter, horizontal_distance_sq: f32) {
    if horizontal_distance_sq < HELICOPTER_ATTACK_RANGE_CLOSE_SQ {
        helicopter.state = HelicopterState::Hovering;
        helicopter.hover_timer = 0.0;
    }
}

/// 處理懸停狀態
fn handle_hovering_state(
    helicopter: &mut PoliceHelicopter,
    horizontal_distance_sq: f32,
    player_visible: bool,
    dt: f32,
) {
    helicopter.hover_timer += dt;

    // 玩家可見性追蹤（脫逃機制）
    if player_visible {
        helicopter.search_timer = 0.0;
    } else {
        helicopter.search_timer += dt;
        // 超過脫逃時間，直升機放棄追蹤回到接近狀態（重新搜索）
        if helicopter.search_timer > PLAYER_ESCAPE_TIME {
            helicopter.state = HelicopterState::Approaching;
            helicopter.search_timer = 0.0;
            helicopter.hover_timer = 0.0;
            return;
        }
    }

    if helicopter.hover_timer > 2.0 && player_visible {
        helicopter.state = HelicopterState::Attacking;
    } else if horizontal_distance_sq > HELICOPTER_ATTACK_RANGE_FAR_SQ {
        helicopter.state = HelicopterState::Pursuing;
    }
}

/// 處理追擊狀態
fn handle_pursuing_state(
    helicopter: &mut PoliceHelicopter,
    horizontal_distance_sq: f32,
    player_visible: bool,
    dt: f32,
) {
    // 玩家可見性追蹤（脫逃機制）
    if player_visible {
        helicopter.search_timer = 0.0;
    } else {
        helicopter.search_timer += dt;
        if helicopter.search_timer > PLAYER_ESCAPE_TIME {
            helicopter.state = HelicopterState::Approaching;
            helicopter.search_timer = 0.0;
            return;
        }
    }

    if horizontal_distance_sq < HELICOPTER_ATTACK_RANGE_CLOSE_SQ {
        helicopter.state = HelicopterState::Hovering;
        helicopter.hover_timer = 0.0;
    }
}

/// 處理攻擊狀態
fn handle_attacking_state(
    helicopter: &mut PoliceHelicopter,
    horizontal_distance_sq: f32,
    player_visible: bool,
    dt: f32,
) {
    if horizontal_distance_sq > HELICOPTER_ATTACK_RANGE_SQ {
        helicopter.state = HelicopterState::Pursuing;
        return;
    }

    if player_visible {
        helicopter.search_timer = 0.0;
    } else {
        helicopter.search_timer += dt;
        if helicopter.search_timer > 5.0 {
            helicopter.state = HelicopterState::Hovering;
            helicopter.search_timer = 0.0;
        }
    }
}

/// 處理規避狀態
fn handle_evading_state(helicopter: &mut PoliceHelicopter, dt: f32) {
    helicopter.evade_timer -= dt;
    if helicopter.evade_timer <= 0.0 {
        helicopter.state = HelicopterState::Pursuing;
    }
}

/// 檢查是否需要觸發規避
fn should_trigger_evade(helicopter: &PoliceHelicopter, current_time: f32) -> bool {
    current_time - helicopter.last_hit_time < 0.5
        && helicopter.state != HelicopterState::Crashing
        && helicopter.state != HelicopterState::Evading
}

/// 檢查直升機是否能看到玩家（獨立 LOS 檢測）
fn check_helicopter_los(heli_pos: Vec3, player_pos: Vec3, rapier: &RapierContext) -> bool {
    let ray_origin = heli_pos + Vec3::new(0.0, -1.0, 0.0); // 從機腹往下看
    let to_player = player_pos - ray_origin;
    let distance = to_player.length();
    if distance < 0.1 {
        return true;
    }

    let ray_dir = to_player.normalize();

    let filter = QueryFilter::default().groups(CollisionGroups::new(
        Group::ALL,
        COLLISION_GROUP_STATIC | COLLISION_GROUP_VEHICLE,
    ));

    match rapier.cast_ray(ray_origin, ray_dir, distance as RapierReal, true, filter) {
        Some((_, toi)) => rapier_real_to_f32(toi) >= distance - 1.0,
        None => true,
    }
}

/// 直升機 AI 系統
pub fn helicopter_ai_system(
    time: Res<Time>,
    _wanted: Res<WantedLevel>,
    player_query: Query<&Transform, With<Player>>,
    mut helicopter_query: Query<(&mut PoliceHelicopter, &Transform)>,
    rapier_context: ReadRapierContext,
) {
    let dt = time.delta_secs();
    let current_time = time.elapsed_secs();

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    for (mut helicopter, transform) in &mut helicopter_query {
        if helicopter.state == HelicopterState::Crashing {
            continue;
        }

        let horizontal_distance_sq = calc_horizontal_distance_sq(transform.translation, player_pos);
        helicopter.target_position = Some(player_pos);

        // 直升機獨立視線檢測
        let heli_can_see_player = check_helicopter_los(transform.translation, player_pos, &rapier);

        match helicopter.state {
            HelicopterState::Approaching => {
                handle_approaching_state(&mut helicopter, horizontal_distance_sq);
            }
            HelicopterState::Hovering => {
                handle_hovering_state(
                    &mut helicopter,
                    horizontal_distance_sq,
                    heli_can_see_player,
                    dt,
                );
            }
            HelicopterState::Pursuing => {
                handle_pursuing_state(
                    &mut helicopter,
                    horizontal_distance_sq,
                    heli_can_see_player,
                    dt,
                );
            }
            HelicopterState::Attacking => {
                handle_attacking_state(
                    &mut helicopter,
                    horizontal_distance_sq,
                    heli_can_see_player,
                    dt,
                );
            }
            HelicopterState::Evading => {
                handle_evading_state(&mut helicopter, dt);
            }
            HelicopterState::Crashing => {}
        }

        if should_trigger_evade(&helicopter, current_time) {
            helicopter.state = HelicopterState::Evading;
            helicopter.evade_timer = EVADE_DURATION;
        }
    }
}

// ============================================================================
// 移動系統
// ============================================================================

/// 處理墜毀移動
fn handle_crash_movement(transform: &mut Transform, crash_velocity: Vec3, dt: f32) {
    transform.translation += crash_velocity * dt;
    transform.rotate_y(CRASH_ROTATION_SPEED.to_radians() * dt);
    transform.rotate_x(30.0_f32.to_radians() * dt);

    if transform.translation.y < 0.0 {
        transform.translation.y = 0.0;
    }
}

/// 取得狀態對應的速度倍率
fn get_speed_multiplier(state: HelicopterState) -> f32 {
    match state {
        HelicopterState::Approaching => 1.0,
        HelicopterState::Pursuing => 1.2,
        HelicopterState::Evading => 1.5,
        HelicopterState::Hovering | HelicopterState::Attacking => 0.2,
        HelicopterState::Crashing => 0.0,
    }
}

/// 計算規避時的移動方向
fn calc_evade_direction(horizontal_dir: Vec3, elapsed_secs: f32) -> Vec3 {
    let evade_angle = (elapsed_secs * 2.0).sin() * 0.5;
    Quat::from_rotation_y(evade_angle) * horizontal_dir
}

/// 處理正常飛行移動
fn handle_normal_flight(
    transform: &mut Transform,
    helicopter: &PoliceHelicopter,
    elapsed_secs: f32,
    dt: f32,
) {
    let Some(target) = helicopter.target_position else {
        return;
    };

    let to_target = target - transform.translation;
    let horizontal_dir = Vec3::new(to_target.x, 0.0, to_target.z).normalize_or_zero();
    let speed_mult = get_speed_multiplier(helicopter.state);

    let move_dir = if helicopter.state == HelicopterState::Evading {
        calc_evade_direction(horizontal_dir, elapsed_secs)
    } else {
        horizontal_dir
    };

    // 水平移動
    let horizontal_distance_sq = Vec2::new(to_target.x, to_target.z).length_squared();
    if horizontal_distance_sq > HELICOPTER_MOVE_THRESHOLD_SQ
        || helicopter.state == HelicopterState::Evading
    {
        transform.translation += move_dir * HELICOPTER_SPEED * speed_mult * dt;
    }

    // 垂直移動
    let altitude_diff = helicopter.target_altitude - transform.translation.y;
    if altitude_diff.abs() > 1.0 {
        transform.translation.y += altitude_diff.signum() * HELICOPTER_VERTICAL_SPEED * dt;
    }

    // 高度限制
    transform.translation.y = transform
        .translation
        .y
        .clamp(HELICOPTER_MIN_ALTITUDE, HELICOPTER_MAX_ALTITUDE);

    // 面向目標（使用標準 Bevy 坐標系統慣例）
    if horizontal_dir != Vec3::ZERO {
        let target_rotation = Quat::from_rotation_y((-horizontal_dir.x).atan2(-horizontal_dir.z));
        transform.rotation = transform
            .rotation
            .slerp(target_rotation, HELICOPTER_TURN_RATE * dt);
    }
}

/// 直升機移動系統
pub fn helicopter_movement_system(
    time: Res<Time>,
    mut helicopter_query: Query<(&mut Transform, &PoliceHelicopter)>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for (mut transform, helicopter) in &mut helicopter_query {
        if helicopter.state == HelicopterState::Crashing {
            handle_crash_movement(&mut transform, helicopter.crash_velocity, dt);
        } else {
            handle_normal_flight(&mut transform, helicopter, elapsed, dt);
        }
    }
}
