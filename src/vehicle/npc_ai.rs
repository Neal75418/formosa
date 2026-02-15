//! NPC 車輛 AI 駕駛系統

use super::vehicle_physics::{apply_vehicle_motion_physics, get_weather_factor, VehicleDynamicsModifiers, VehiclePhysicsFrame};
use super::*;
use crate::core::math::rapier_real_to_f32;
use crate::core::{
    WeatherState, COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE,
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};

// ============================================================================
// NPC 車輛 AI 駕駛系統
// ============================================================================

#[derive(Clone, Copy)]
enum ObstacleHitKind {
    Front,
    Side,
}

#[derive(Clone, Copy)]
struct ObstacleHit {
    distance: f32,
    kind: ObstacleHitKind,
}

/// 檢查前方是否有障礙物（人或車）
fn check_obstacle(
    vehicle_transform: &Transform,
    vehicle_entity: Entity,
    _vehicle_type: &VehicleType,
    rapier_context: &ReadRapierContext,
    config: &crate::vehicle::config::NpcDrivingConfig,
) -> Option<ObstacleHit> {
    let rapier = rapier_context.single().ok()?;
    let vehicle_pos = vehicle_transform.translation;
    let vehicle_forward = vehicle_transform.forward().as_vec3();
    let ray_origin =
        vehicle_pos + Vec3::new(0.0, config.obstacle_check_height, 0.0) + vehicle_forward * config.ray_forward_offset;

    // 主射線（前方）
    let max_toi = config.obstacle_max_distance;
    let solid = true;
    let groups = QueryFilter::new()
        .groups(CollisionGroups::new(
            COLLISION_GROUP_VEHICLE,
            COLLISION_GROUP_VEHICLE | COLLISION_GROUP_CHARACTER | COLLISION_GROUP_STATIC,
        ))
        .exclude_collider(vehicle_entity);

    if let Some((_entity, toi)) =
        rapier.cast_ray(ray_origin, vehicle_forward, max_toi as RapierReal, solid, groups)
    {
        return Some(ObstacleHit {
            distance: rapier_real_to_f32(toi),
            kind: ObstacleHitKind::Front,
        });
    }

    // 側向射線（避免路邊擦撞）— 只偵測車輛和行人，不碰建築/牆壁
    let side_groups = QueryFilter::new()
        .groups(CollisionGroups::new(
            COLLISION_GROUP_VEHICLE,
            COLLISION_GROUP_VEHICLE | COLLISION_GROUP_CHARACTER,
        ))
        .exclude_collider(vehicle_entity);
    let side_offset = 0.8;
    for side in [-1.0, 1.0] {
        let side_origin = ray_origin + vehicle_transform.right().as_vec3() * side * side_offset;
        // 稍微內縮射線角度
        let ray_dir =
            (vehicle_forward + vehicle_transform.right().as_vec3() * side * -0.2).normalize();

        if let Some((_entity, toi)) = rapier.cast_ray(
            side_origin,
            ray_dir,
            config.obstacle_side_max_distance as RapierReal,
            solid,
            side_groups,
        ) {
            let distance = rapier_real_to_f32(toi);
            if distance <= config.obstacle_side_brake_distance {
                return Some(ObstacleHit {
                    distance,
                    kind: ObstacleHitKind::Side,
                });
            }
        }
    }

    None
}

/// 根據障礙物更新 NPC 狀態
fn update_npc_state_from_obstacle(
    npc: &mut NpcVehicle,
    transform: &Transform,
    vehicle: &Vehicle,
    input: &mut VehicleInput,
    vehicle_entity: Entity,
    rapier_context: &ReadRapierContext,
    dt: f32,
    config: &crate::vehicle::config::NpcDrivingConfig,
) {
    if let Some(hit) = check_obstacle(
        transform,
        vehicle_entity,
        &vehicle.vehicle_type,
        rapier_context,
        config,
    ) {
        npc.stuck_timer += dt; // 只在有障礙物時累加
        let (new_state, reset_timer) =
            determine_npc_reaction(hit, vehicle.current_speed, npc.stuck_timer, config);
        npc.state = new_state;
        if reset_timer {
            npc.stuck_timer = 0.0;
        }

        // === 避障轉向：遇到前方障礙物時嘗試繞行 ===
        if matches!(hit.kind, ObstacleHitKind::Front) && hit.distance < config.obstacle_brake_distance {
            // 基於車輛位置生成一致的轉向方向（避免來回抖動）
            let pos_hash = (transform.translation.x * 1000.0) as i32;
            let turn_direction = if pos_hash % 2 == 0 { 1.0 } else { -1.0 };

            // 距離越近，轉向越急
            let urgency = 1.0 - (hit.distance / config.obstacle_brake_distance).clamp(0.0, 1.0);
            input.steer_input = turn_direction * urgency * 0.8;
        }
    } else {
        npc.state = NpcState::Cruising;
        npc.stuck_timer = 0.0; // 無障礙物時歸零
    }
}

fn determine_npc_reaction(
    hit: ObstacleHit,
    current_speed: f32,
    stuck_timer: f32,
    config: &crate::vehicle::config::NpcDrivingConfig,
) -> (NpcState, bool) {
    match hit.kind {
        ObstacleHitKind::Side => {
            if hit.distance < config.obstacle_side_brake_distance {
                (NpcState::Braking, false)
            } else {
                (NpcState::Cruising, false)
            }
        }
        ObstacleHitKind::Front => {
            if hit.distance < config.obstacle_too_close_distance {
                // Too close, check if stuck
                if current_speed < config.stuck_speed_threshold {
                    if stuck_timer > config.stuck_timeout {
                        (NpcState::Reversing, true)
                    } else {
                        (NpcState::Stopped, false)
                    }
                } else {
                    (NpcState::Braking, false)
                }
            } else if hit.distance < config.obstacle_brake_distance {
                (NpcState::Braking, false)
            } else {
                (NpcState::Cruising, false)
            }
        }
    }
}

/// 導航至下一個航點
fn navigate_to_waypoint(
    npc: &mut NpcVehicle,
    transform: &Transform,
    input: &mut VehicleInput,
    config: &crate::vehicle::config::NpcDrivingConfig,
) {
    if npc.waypoints.is_empty() {
        return;
    }

    let mut target = npc.waypoints[npc.current_wp_index];
    let dist_sq = transform.translation.distance_squared(target);

    if dist_sq < config.waypoint_arrival_distance_sq {
        npc.current_wp_index = (npc.current_wp_index + 1) % npc.waypoints.len();
        target = npc.waypoints[npc.current_wp_index];
    }

    // 計算轉向
    let to_target = target - transform.translation;
    if to_target.length_squared() < 1e-6 {
        input.steer_input = 0.0;
        return;
    }
    let target_dir = to_target.normalize();
    let forward = transform.forward().as_vec3();
    let right = transform.right().as_vec3();

    let dot = forward.dot(target_dir);
    let cross = right.dot(target_dir);

    // 簡單的 P 控制器
    let steer = cross * config.steering_p_gain;
    input.steer_input = steer.clamp(-1.0, 1.0);

    // 如果角度過大，減速
    if dot < 0.5 {
        input.throttle_input = 0.5;
    } else {
        input.throttle_input = 1.0;
    }
}

fn handle_cruising_state(
    npc: &mut NpcVehicle,
    transform: &mut Transform,
    input: &mut VehicleInput,
    _dt: f32,
    config: &crate::vehicle::config::NpcDrivingConfig,
) {
    navigate_to_waypoint(npc, transform, input, config);
    input.throttle_input = input.throttle_input.min(0.7); // 巡航限速，保留轉彎減速
    input.brake_input = 0.0;
}

fn handle_braking_state(
    _npc: &mut NpcVehicle,
    _transform: &mut Transform,
    input: &mut VehicleInput,
    _dt: f32,
) {
    input.throttle_input = 0.0;
    input.brake_input = 1.0;
    // 煞車時保持轉向
}

fn handle_stopped_state(_npc: &mut NpcVehicle, input: &mut VehicleInput, _dt: f32) {
    input.throttle_input = 0.0;
    input.brake_input = 1.0;
}

fn handle_reversing_state(
    npc: &mut NpcVehicle,
    _transform: &mut Transform,
    input: &mut VehicleInput,
    _dt: f32,
    config: &VehicleConfig,
) {
    input.throttle_input = 0.0;
    input.brake_input = 1.0; // 倒車
                               // 倒車時反向打輪
    input.steer_input = -input.steer_input;

    if npc.stuck_timer > config.npc.reverse_timeout {
        npc.state = NpcState::Cruising;
        npc.stuck_timer = 0.0;
    }
}

/// 判斷是否需要等紅燈
fn should_stop_for_traffic_light(
    vehicle_pos: Vec3,
    vehicle_forward: Vec3,
    traffic_light_query: &Query<(&Transform, &TrafficLight), Without<NpcVehicle>>,
    npc_config: &crate::vehicle::config::NpcDrivingConfig,
) -> bool {
    // 簡單判斷：尋找前方且面向自己的紅燈
    for (light_transform, light) in traffic_light_query.iter() {
        if light.state == TrafficLightState::Red || light.state == TrafficLightState::Yellow {
            let to_light = light_transform.translation - vehicle_pos;
            let dist_sq = to_light.length_squared();

            if dist_sq < npc_config.traffic_light_detect_dist_sq {
                let to_light_dir = to_light.normalize();
                // 燈必須在車輛前方
                if to_light_dir.dot(vehicle_forward) > npc_config.traffic_light_forward_dot {
                    let light_forward = light_transform.forward().as_vec3();
                    // 燈面向車輛
                    if light_forward.dot(vehicle_forward) < npc_config.traffic_light_facing_dot {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// NPC 車輛 AI（含避障功能和紅綠燈遵守）
pub fn npc_vehicle_ai(
    time: Res<Time>,
    rapier_context: ReadRapierContext,
    mut npc_query: Query<(
        Entity,
        &mut Transform,
        &mut Vehicle,
        &mut VehicleInput,
        &mut NpcVehicle,
    )>,
    traffic_light_query: Query<(&Transform, &TrafficLight), Without<NpcVehicle>>,
    config: Res<VehicleConfig>,
) {
    let dt = time.delta_secs();
    let Ok(_rapier) = rapier_context.single() else {
        return;
    };

    for (entity, mut transform, vehicle, mut input, mut npc) in npc_query.iter_mut() {
        // 定期檢查前方障礙物和紅綠燈
        npc.check_timer.tick(time.delta());
        if npc.check_timer.just_finished() {
            // Update state from obstacle
            update_npc_state_from_obstacle(
                &mut npc,
                &transform,
                &vehicle,
                &mut input,
                entity,
                &rapier_context,
                dt,
                &config.npc,
            );

            // 檢查紅綠燈（除了倒車和等紅燈狀態外都要檢查）
            // 避免 Braking/Stopped 狀態的車輛闖紅燈
            if npc.state != NpcState::Reversing && npc.state != NpcState::WaitingAtLight {
                let vehicle_pos = transform.translation;
                let vehicle_forward = transform.forward().as_vec3();
                if should_stop_for_traffic_light(vehicle_pos, vehicle_forward, &traffic_light_query, &config.npc)
                {
                    npc.state = NpcState::WaitingAtLight;
                }
            }
        }

        // 根據狀態執行行為
        match npc.state {
            NpcState::Cruising => {
                handle_cruising_state(&mut npc, &mut transform, &mut input, dt, &config.npc)
            }
            NpcState::Braking => handle_braking_state(&mut npc, &mut transform, &mut input, dt),
            NpcState::Stopped => handle_stopped_state(&mut npc, &mut input, dt),
            NpcState::Reversing => {
                handle_reversing_state(&mut npc, &mut transform, &mut input, dt, &config)
            }
            NpcState::WaitingAtLight => handle_waiting_at_light_state(
                &mut npc,
                &mut input,
                &transform,
                &traffic_light_query,
                dt,
                &config.npc,
            ),
        }
    }
}

/// 處理等紅燈狀態
fn handle_waiting_at_light_state(
    npc: &mut NpcVehicle,
    input: &mut VehicleInput,
    transform: &Transform,
    traffic_light_query: &Query<(&Transform, &TrafficLight), Without<NpcVehicle>>,
    dt: f32,
    npc_config: &crate::vehicle::config::NpcDrivingConfig,
) {
    // 重置輸入：鬆油門 + 踩煞車（由 npc_vehicle_motion_system 的煞車物理處理減速）
    input.throttle_input = 0.0;
    input.brake_input = 1.0;

    npc.stuck_timer += dt;

    // 檢查燈是否變綠了或等太久 failsafe，恢復巡航
    let vehicle_pos = transform.translation;
    let vehicle_forward = transform.forward().as_vec3();
    if !should_stop_for_traffic_light(vehicle_pos, vehicle_forward, traffic_light_query, npc_config)
        || npc.stuck_timer > npc_config.traffic_light_wait_timeout
    {
        npc.state = NpcState::Cruising;
        npc.stuck_timer = 0.0;
    }
}

/// NPC 車輛運動整合（使用 NPC 輸入更新速度與位置）
pub fn npc_vehicle_motion_system(
    time: Res<Time>,
    weather: Res<WeatherState>,
    config: Res<VehicleConfig>,
    map_bounds: Res<crate::world::MapBounds>,
    mut npc_query: Query<
        (
            &mut Transform,
            &mut Vehicle,
            &VehiclePowerBand,
            &VehicleBraking,
            &VehicleSteering,
            &VehicleDrift,
            &mut VehicleInput,
            Option<&VehicleModifications>,
            Option<&TireDamage>,
        ),
        With<NpcVehicle>,
    >,
) {
    let dt = time.delta_secs();

    for (mut transform, mut vehicle, power_band, braking, steering, drift, mut input, mods, tire_damage) in
        npc_query.iter_mut()
    {
        let modifiers = VehicleDynamicsModifiers::new(mods, None, tire_damage);

        let weather_traction = get_weather_factor(&weather, &config.weather.traction_params());
        let effective_traction = weather_traction * modifiers.traction;
        let effective_max_speed = vehicle.max_speed * modifiers.speed;

        let frame = VehiclePhysicsFrame {
            power_band,
            config: &config.physics,
            modifiers: &modifiers,
            dt,
            effective_traction,
            effective_max_speed,
        };
        apply_vehicle_motion_physics(&mut vehicle, braking, drift, &mut input, &frame);

        // Clamp Speed
        vehicle.current_speed = vehicle.current_speed.clamp(
            -effective_max_speed * config.physics.reverse_speed_ratio,
            effective_max_speed,
        );
        if vehicle.current_speed.abs() < config.physics.stop_speed_threshold
            && input.throttle_input == 0.0
            && input.brake_input == 0.0
        {
            vehicle.current_speed = 0.0;
        }

        // Steering
        if vehicle.current_speed.abs() > 0.5 {
            apply_npc_steering(
                &mut transform,
                &vehicle,
                steering,
                drift,
                &input,
                mods,
                &weather,
                &config,
                dt,
            );
        }

        // Move
        let forward = transform.forward().as_vec3();
        transform.translation += forward * vehicle.current_speed * dt;

        // 邊界夾持：防止 NPC 車輛駛出地圖
        let (clamped_x, clamped_z) = map_bounds.clamp_position(
            transform.translation.x,
            transform.translation.z,
        );
        transform.translation.x = clamped_x;
        transform.translation.z = clamped_z;
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_npc_steering(
    transform: &mut Transform,
    vehicle: &Vehicle,
    steering: &VehicleSteering,
    drift: &VehicleDrift,
    input: &VehicleInput,
    mods: Option<&VehicleModifications>,
    weather: &WeatherState,
    config: &VehicleConfig,
    dt: f32,
) {
    let weather_handling = get_weather_factor(weather, &config.weather.handling_params());
    let handling_mod = mods.map_or(1.0, |m| m.suspension.multiplier());
    let effective_handling = weather_handling * handling_mod;

    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);
    let low_threshold = config.physics.torque_low_speed_ratio;
    let speed_turn_factor = if speed_ratio < low_threshold {
        1.0
    } else {
        let high_speed_falloff = (speed_ratio - low_threshold) / (1.0 - low_threshold).max(0.01);
        1.0 - high_speed_falloff * (1.0 - steering.high_speed_turn_factor)
    };

    let drift_turn_bonus = if drift.is_drifting {
        1.0 + drift.drift_angle.abs() * steering.counter_steer_assist
    } else {
        1.0
    };

    let direction = vehicle.current_speed.signum();
    let yaw_rate = vehicle.turn_speed
        * steering.handling
        * effective_handling
        * speed_turn_factor
        * drift_turn_bonus
        * input.steer_input
        * direction;

    transform.rotate_y(yaw_rate * dt);
}
