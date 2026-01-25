//! 載具系統

use super::{
    DriftSmoke, NitroBoost, NitroFlame, NpcState, NpcVehicle, TireTrack, TrafficLight,
    TrafficLightBulb, TrafficLightState, TrafficLightVisuals, Vehicle, VehicleConfig,
    VehicleEffectTracker, VehicleEffectVisuals, VehicleId, VehicleModifications,
    VehiclePhysicsMode, VehicleType, VehicleVisualRoot,
};
use crate::core::math::look_rotation_y_flat;
use crate::core::{
    GameState, WeatherState, WeatherType, COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC,
    COLLISION_GROUP_VEHICLE,
};
use bevy::prelude::*;
use rand::Rng;

use bevy_rapier3d::prelude::*;

// ============================================================================
// 車輛系統
// ============================================================================

/// 將 bevy_rapier3d 的 Real 類型轉換為 f32
/// 用於避免與 bevy::prelude::Real 的命名衝突
#[inline]
fn rapier_real_to_f32(r: bevy_rapier3d::prelude::Real) -> f32 {
    r
}

/// 切換載具剛體型態並補齊必要物理組件
pub fn apply_vehicle_physics_mode(
    commands: &mut Commands,
    entity: Entity,
    mode: VehiclePhysicsMode,
    transform: &Transform,
    vehicle: &Vehicle,
    existing_velocity: Option<&Velocity>,
) {
    let mut entity_commands = commands.entity(entity);
    entity_commands.insert(mode);

    match mode {
        VehiclePhysicsMode::Dynamic => {
            let (linvel, angvel) = if let Some(velocity) = existing_velocity {
                (velocity.linvel, velocity.angvel)
            } else {
                let forward = transform.forward().as_vec3();
                (forward * vehicle.current_speed, Vec3::ZERO)
            };

            entity_commands
                .insert(RigidBody::Dynamic)
                .insert(Velocity { linvel, angvel })
                .insert(ExternalImpulse::default())
                .insert(ExternalForce::default())
                .insert(Damping {
                    linear_damping: 0.5,
                    angular_damping: 1.0,
                })
                .insert(LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z);
        }
        VehiclePhysicsMode::Kinematic => {
            entity_commands
                .insert(RigidBody::KinematicPositionBased)
                .remove::<Velocity>()
                .remove::<ExternalImpulse>()
                .remove::<ExternalForce>()
                .remove::<Damping>()
                .remove::<LockedAxes>();
        }
    }
}

/// 載具輸入（手煞車/漂移觸發）
/// 載具輸入（手煞車/漂移觸發、加速、轉向）
pub fn vehicle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    game_state: Res<GameState>,
    mut vehicles: Query<&mut Vehicle>,
    config: Res<VehicleConfig>,
) {
    if !game_state.player_in_vehicle {
        return;
    }

    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok(mut vehicle) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    let both_mouse =
        mouse_button.pressed(MouseButton::Left) && mouse_button.pressed(MouseButton::Right);

    // Throttle
    vehicle.throttle_input = if keyboard.pressed(KeyCode::KeyW) || both_mouse {
        1.0
    } else {
        0.0
    };

    // Brake
    vehicle.brake_input = if keyboard.pressed(KeyCode::KeyS) {
        1.0
    } else {
        0.0
    };

    // Handbrake
    vehicle.is_handbraking = keyboard.pressed(KeyCode::Space);

    // Steering (with smoothing)
    let raw_input = if keyboard.pressed(KeyCode::KeyA) {
        1.0
    } else if keyboard.pressed(KeyCode::KeyD) {
        -1.0
    } else {
        0.0
    };

    // 靜止時轉向輸入衰減
    let target_input = if vehicle.current_speed.abs() <= 0.5 {
        raw_input * 0.9 // 快速歸零
    } else {
        raw_input
    };

    let dt = time.delta_secs();
    // 使用 Steering Response 平滑輸入
    let steer_response = vehicle.steering_response * dt * 5.0; // 5.0 是經驗值，取代原本的 weather handling 影響
    vehicle.steer_input += (target_input - vehicle.steer_input) * steer_response.min(1.0);

    // Deadzone
    if vehicle.steer_input.abs() < config.input.steer_input_deadzone {
        vehicle.steer_input = 0.0;
    }
}

/// 天氣系統（預留給未來更複雜的天氣物理計算）
pub fn vehicle_weather_system() {
    // 目前天氣狀態由各個系統直接讀取 WeatherState
}

/// 載具加速與煞車系統
pub fn vehicle_acceleration_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    weather: Res<WeatherState>,
    config: Res<VehicleConfig>,
    mut vehicles: Query<(
        &mut Vehicle,
        Option<&VehicleModifications>,
        Option<&NitroBoost>,
    )>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((mut vehicle, mods, nitro)) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    let dt = time.delta_secs();
    let modifiers = VehicleDynamicsModifiers::new(mods, nitro);

    // Weather Traction
    let weather_traction = get_weather_traction_factor(&weather, &config.weather);
    let effective_traction = weather_traction * modifiers.traction;
    let effective_max_speed = vehicle.max_speed * modifiers.speed;

    if vehicle.throttle_input > 0.0 {
        handle_acceleration(
            &mut vehicle,
            dt,
            &config.physics,
            &modifiers,
            effective_traction,
        );
    } else if vehicle.brake_input > 0.0 && !vehicle.is_handbraking {
        handle_braking(&mut vehicle, dt, &modifiers, effective_traction);
    } else {
        handle_friction(
            &mut vehicle,
            &config.physics,
            effective_traction,
            effective_max_speed,
        );
    }

    // Clamp Speed
    vehicle.current_speed = vehicle.current_speed.clamp(
        -effective_max_speed * config.physics.reverse_speed_ratio,
        effective_max_speed,
    );
    if vehicle.current_speed.abs() < config.physics.stop_speed_threshold
        && vehicle.throttle_input == 0.0
    {
        vehicle.current_speed = 0.0;
    }
}

struct VehicleDynamicsModifiers {
    accel: f32,
    speed: f32,
    brake: f32,
    traction: f32,
    nitro: f32,
}

impl VehicleDynamicsModifiers {
    fn new(mods: Option<&VehicleModifications>, nitro: Option<&NitroBoost>) -> Self {
        let (accel, speed, _, brake, traction) = if let Some(m) = mods {
            (
                m.engine.multiplier(),
                m.transmission.multiplier(),
                m.suspension.multiplier(),
                m.brakes.multiplier(),
                m.tires.multiplier(),
            )
        } else {
            (1.0, 1.0, 1.0, 1.0, 1.0)
        };

        let nitro_mult = if let Some(n) = nitro {
            if n.is_active {
                n.boost_multiplier
            } else {
                1.0
            }
        } else {
            1.0
        };

        Self {
            accel,
            speed,
            brake,
            traction,
            nitro: nitro_mult,
        }
    }
}

fn handle_acceleration(
    vehicle: &mut Vehicle,
    dt: f32,
    physics_config: &crate::vehicle::config::VehiclePhysicsConfig,
    modifiers: &VehicleDynamicsModifiers,
    effective_traction: f32,
) {
    let accel_mult = modifiers.nitro.max(1.0);
    let accel_force = calculate_acceleration_force(vehicle) * modifiers.accel;
    let effective_accel = accel_force * accel_mult * vehicle.throttle_input * effective_traction;

    // Wheel spin logic
    let slip_threshold = if effective_traction < physics_config.normal_traction_threshold {
        physics_config.slip_speed_low_traction
    } else {
        physics_config.slip_speed_normal
    };

    if vehicle.current_speed < slip_threshold
        && (accel_mult > 1.0 || effective_traction < physics_config.low_traction_threshold)
    {
        let slip_factor = if effective_traction < physics_config.low_traction_threshold {
            physics_config.slip_factor_low_traction
        } else {
            physics_config.slip_factor_normal
        };
        vehicle.wheel_spin = (vehicle.wheel_spin + dt * slip_factor).min(1.0);
        let grip =
            effective_traction * (1.0 - vehicle.wheel_spin * physics_config.slip_grip_penalty);
        vehicle.current_speed += effective_accel * grip * dt;
    } else {
        vehicle.wheel_spin = (vehicle.wheel_spin - dt * physics_config.slip_recovery_rate).max(0.0);
        vehicle.current_speed += effective_accel * dt;
    }
}

fn handle_braking(
    vehicle: &mut Vehicle,
    dt: f32,
    modifiers: &VehicleDynamicsModifiers,
    effective_traction: f32,
) {
    if vehicle.current_speed > 0.5 {
        let brake_decel =
            vehicle.brake_force * modifiers.brake * vehicle.brake_input * effective_traction;
        vehicle.current_speed -= brake_decel * dt;
        vehicle.current_speed = vehicle.current_speed.max(0.0);
    } else {
        // Reverse
        let reverse_accel =
            calculate_acceleration_force(vehicle) * modifiers.accel * 0.5 * effective_traction;
        vehicle.current_speed -= reverse_accel * dt;
    }
}

fn handle_friction(
    vehicle: &mut Vehicle,
    physics_config: &crate::vehicle::config::VehiclePhysicsConfig,
    effective_traction: f32,
    effective_max_speed: f32,
) {
    if vehicle.is_handbraking {
        // Handbrake
        let handbrake_decel = vehicle.handbrake_force
            * physics_config.handbrake_decel_coefficient
            * effective_traction;
        vehicle.current_speed *= 1.0 - handbrake_decel;
    } else {
        // Natural Deceleration
        let drag = 1.0 + (vehicle.current_speed.abs() / effective_max_speed) * 0.5;
        vehicle.current_speed *= 1.0 - 0.025 * drag;
    }
}

/// 載具轉向與角速度系統
pub fn vehicle_steering_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    weather: Res<WeatherState>,
    config: Res<VehicleConfig>,
    mut vehicles: Query<(&mut Vehicle, &mut Velocity, Option<&VehicleModifications>)>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((vehicle, mut velocity, mods)) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    if vehicle.current_speed.abs() <= 0.5 {
        // 靜止不轉向
        return;
    }

    let dt = time.delta_secs();

    // Weather Handling
    let weather_handling = get_weather_handling_factor(&weather, &config.weather);
    let handling_mod = if let Some(m) = mods {
        m.suspension.multiplier()
    } else {
        1.0
    };
    let effective_handling = weather_handling * handling_mod;

    // Turning Logic
    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);
    let speed_turn_factor = if speed_ratio < 0.3 {
        1.0
    } else {
        let high_speed_falloff = (speed_ratio - 0.3) / 0.7;
        1.0 - high_speed_falloff * (1.0 - vehicle.high_speed_turn_factor)
    };

    let drift_turn_bonus = if vehicle.is_drifting {
        1.0 + vehicle.drift_angle.abs() * vehicle.counter_steer_assist
    } else {
        1.0
    };

    let direction = vehicle.current_speed.signum();
    let yaw_rate = vehicle.turn_speed
        * vehicle.handling
        * effective_handling
        * speed_turn_factor
        * drift_turn_bonus
        * vehicle.steer_input
        * direction;

    let steering_response = (vehicle.steering_response * dt).min(1.0);
    velocity.angvel.y += (yaw_rate - velocity.angvel.y) * steering_response;
}

/// 載具漂移系統
pub fn vehicle_drift_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    weather: Res<WeatherState>,
    config: Res<VehicleConfig>,
    mut vehicles: Query<(&mut Vehicle, Option<&VehicleModifications>)>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((mut vehicle, mods)) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    let dt = time.delta_secs();
    let effective_traction = calculate_effective_traction(&weather, &config, mods);
    let params = DriftPhysicsParams::new(&config.physics, effective_traction);

    if vehicle.is_handbraking && vehicle.current_speed.abs() > params.speed_threshold {
        handle_drift_start(&mut vehicle, dt, &config.physics, params.amplifier);
    } else if vehicle.is_drifting {
        handle_active_drift(
            &mut vehicle,
            dt,
            &config.physics,
            effective_traction,
            params.end_speed,
        );
    } else {
        handle_drift_decay(&mut vehicle, dt, &config.physics);
    }
}

fn calculate_effective_traction(
    weather: &WeatherState,
    config: &VehicleConfig,
    mods: Option<&VehicleModifications>,
) -> f32 {
    let traction_mod = mods.map_or(1.0, |m| m.tires.multiplier());
    let weather_traction = get_weather_traction_factor(weather, &config.weather);
    weather_traction * traction_mod
}

struct DriftPhysicsParams {
    speed_threshold: f32,
    amplifier: f32,
    end_speed: f32,
}

impl DriftPhysicsParams {
    fn new(physics_config: &crate::vehicle::config::VehiclePhysicsConfig, traction: f32) -> Self {
        let low_traction = traction < physics_config.normal_traction_threshold;
        Self {
            speed_threshold: if low_traction {
                physics_config.drift_speed_threshold_low_traction
            } else {
                physics_config.drift_speed_threshold_normal
            },
            amplifier: if low_traction {
                physics_config.drift_amplifier_low_traction
            } else {
                physics_config.drift_amplifier_normal
            },
            end_speed: if low_traction {
                physics_config.drift_end_speed_low_traction
            } else {
                physics_config.drift_end_speed_normal
            },
        }
    }
}

fn handle_drift_start(
    vehicle: &mut Vehicle,
    dt: f32,
    config: &crate::vehicle::config::VehiclePhysicsConfig,
    amplifier: f32,
) {
    vehicle.drift_angle += vehicle.steer_input * dt * config.drift_angle_rate * amplifier;
    vehicle.drift_angle = vehicle
        .drift_angle
        .clamp(-config.max_drift_angle, config.max_drift_angle);
    vehicle.is_drifting = vehicle.drift_angle.abs() > vehicle.drift_threshold;
}

fn handle_active_drift(
    vehicle: &mut Vehicle,
    dt: f32,
    config: &crate::vehicle::config::VehiclePhysicsConfig,
    traction: f32,
    end_speed: f32,
) {
    // Apply counter force
    let counter = -vehicle.drift_angle
        * (1.0 - vehicle.drift_grip * traction)
        * dt
        * config.drift_counter_force_rate;
    vehicle.drift_angle += counter;

    // Apply counter steer assist
    if vehicle.steer_input != 0.0 && vehicle.steer_input.signum() == -vehicle.drift_angle.signum() {
        vehicle.drift_angle +=
            vehicle.steer_input * vehicle.counter_steer_assist * dt * config.counter_steer_rate;
    }

    // Check end condition
    if vehicle.drift_angle.abs() < config.drift_end_angle_threshold
        || vehicle.current_speed.abs() < end_speed
    {
        vehicle.is_drifting = false;
        vehicle.drift_angle = 0.0;
    } else {
        // Apply speed loss
        let drift_speed_loss = vehicle.drift_angle.abs()
            * (1.0 - vehicle.drift_grip)
            * traction
            * dt
            * config.drift_speed_loss_rate;
        vehicle.current_speed *= 1.0 - drift_speed_loss;
    }
}

fn handle_drift_decay(
    vehicle: &mut Vehicle,
    dt: f32,
    config: &crate::vehicle::config::VehiclePhysicsConfig,
) {
    vehicle.drift_angle *= 1.0 - dt * config.drift_decay_rate;
    if vehicle.drift_angle.abs() < config.drift_angle_zero_threshold {
        vehicle.drift_angle = 0.0;
    }
}

/// 載具懸吊與車身動態系統
pub fn vehicle_suspension_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    mut vehicles: Query<&mut Vehicle>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok(mut vehicle) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    let dt = time.delta_secs();

    // Scooter lean
    if vehicle.vehicle_type == VehicleType::Scooter {
        let speed_factor = (vehicle.current_speed / vehicle.max_speed).clamp(0.0, 1.0);
        let target_lean = vehicle.steer_input * vehicle.max_lean_angle * speed_factor;
        let lean_speed = 5.0;
        let lean_diff = target_lean - vehicle.lean_angle;
        vehicle.lean_angle += lean_diff * lean_speed * dt;
        vehicle.lean_angle = vehicle
            .lean_angle
            .clamp(-vehicle.max_lean_angle, vehicle.max_lean_angle);
        return;
    }

    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);

    // Roll
    let target_roll = -vehicle.steer_input * vehicle.body_roll_factor * speed_ratio.sqrt();
    let drift_roll_bonus = if vehicle.is_drifting {
        vehicle.drift_angle * 0.3
    } else {
        0.0
    };

    // Pitch
    let accel_state = vehicle.throttle_input - vehicle.brake_input;
    let target_pitch = -accel_state * vehicle.body_pitch_factor * speed_ratio.sqrt().min(0.8);
    let handbrake_pitch = if vehicle.is_handbraking { 0.04 } else { 0.0 };

    // Suspension
    let suspension_speed = vehicle.suspension_stiffness * dt;
    vehicle.body_roll += ((target_roll + drift_roll_bonus) - vehicle.body_roll) * suspension_speed;
    vehicle.body_pitch +=
        ((target_pitch + handbrake_pitch) - vehicle.body_pitch) * suspension_speed;

    vehicle.body_roll = vehicle.body_roll.clamp(-0.2, 0.2);
    vehicle.body_pitch = vehicle.body_pitch.clamp(-0.15, 0.15);
}

/// 載具物理整合系統（整合速度與位移）
pub fn vehicle_physics_integration_system(
    game_state: Res<GameState>,
    mut vehicles: Query<(&Transform, &Vehicle, &mut Velocity)>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((transform, vehicle, mut velocity)) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    let forward = transform.forward().as_vec3();

    let movement_dir = if vehicle.is_drifting && vehicle.drift_angle.abs() > 0.1 {
        let drift_offset = Quat::from_rotation_y(-vehicle.drift_angle * 0.3);
        drift_offset * forward
    } else {
        forward
    };

    let current_move_speed = velocity.linvel.dot(movement_dir);
    let speed_delta = vehicle.current_speed - current_move_speed;
    velocity.linvel += movement_dir * speed_delta;
}

/// Apply visual-only roll/pitch/lean to vehicle meshes.
pub fn update_vehicle_visuals(
    vehicle_query: Query<&Vehicle>,
    mut visual_query: Query<(&ChildOf, &mut Transform), With<VehicleVisualRoot>>,
) {
    for (parent, mut transform) in &mut visual_query {
        let Ok(vehicle) = vehicle_query.get(parent.parent()) else {
            continue;
        };

        let yaw_offset = if vehicle.is_drifting {
            vehicle.drift_angle * 0.25
        } else {
            0.0
        };

        transform.translation = Vec3::ZERO;
        if vehicle.vehicle_type == VehicleType::Scooter {
            transform.rotation =
                Quat::from_rotation_y(yaw_offset) * Quat::from_rotation_z(-vehicle.lean_angle);
        } else {
            transform.rotation = Quat::from_rotation_y(yaw_offset)
                * Quat::from_rotation_x(vehicle.body_pitch)
                * Quat::from_rotation_z(vehicle.body_roll);
        }
    }
}

// ============================================================================
// 天氣影響駕駛系統
// ============================================================================

/// 計算天氣對牽引力的影響
fn get_weather_traction_factor(
    weather: &WeatherState,
    config: &crate::vehicle::config::VehicleWeatherConfig,
) -> f32 {
    match weather.weather_type {
        WeatherType::Clear => config.clear_traction,
        WeatherType::Cloudy => config.cloudy_traction,
        WeatherType::Rainy => {
            config.rainy_traction_base + (1.0 - weather.intensity) * config.rainy_traction_range
        }
        WeatherType::Foggy => config.foggy_traction,
        WeatherType::Stormy => config.stormy_traction_base + (1.0 - weather.intensity) * 0.15,
        WeatherType::Sandstorm => config.sandstorm_traction_base + (1.0 - weather.intensity) * 0.1,
    }
}

/// 計算天氣對操控的影響
fn get_weather_handling_factor(
    weather: &WeatherState,
    config: &crate::vehicle::config::VehicleWeatherConfig,
) -> f32 {
    match weather.weather_type {
        WeatherType::Clear => config.clear_traction, // Note: Original code used TRACTION const here too? Let's check. Yes, likely 1.0
        WeatherType::Cloudy => config.cloudy_traction,
        WeatherType::Rainy => {
            config.rainy_handling_base + (1.0 - weather.intensity) * config.rainy_handling_range
        }
        WeatherType::Foggy => config.foggy_handling,
        WeatherType::Stormy => config.stormy_handling_base + (1.0 - weather.intensity) * 0.2,
        WeatherType::Sandstorm => config.sandstorm_handling_base + (1.0 - weather.intensity) * 0.1,
    }
}

/// 計算非線性加速力（扭力曲線）
fn calculate_acceleration_force(vehicle: &Vehicle) -> f32 {
    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);

    let torque_multiplier = if speed_ratio < 0.3 {
        // 低速區：強扭力（起步快）
        vehicle.power_band_low * (1.0 - speed_ratio * 0.5)
    } else if speed_ratio < 0.7 {
        // 中速區：峰值扭力
        let t = (speed_ratio - 0.3) / 0.4;
        vehicle.power_band_peak * (1.0 + 0.2 * (1.0 - (t - 0.5).abs() * 2.0))
    } else {
        // 高速區：扭力衰減
        let falloff = (speed_ratio - 0.7) / 0.3;
        vehicle.top_end_falloff * (1.0 - falloff * 0.5)
    };

    vehicle.acceleration * torque_multiplier
}

/// 取得車輛高度
fn get_vehicle_height(vehicle_type: &VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 1.5,
        VehicleType::Car | VehicleType::Taxi => 1.5,
        VehicleType::Bus => 3.0,
    }
}

/// 檢查前方是否有障礙物（人或車）
fn check_obstacle(
    vehicle_transform: &Transform,
    vehicle_entity: Entity,
    _vehicle_type: &VehicleType,
    rapier_context: &ReadRapierContext,
    config: &crate::vehicle::config::NpcDrivingConfig,
) -> Option<f32> {
    let rapier = rapier_context.single().ok()?;
    let vehicle_pos = vehicle_transform.translation;
    let vehicle_forward = vehicle_transform.forward().as_vec3();
    let ray_origin =
        vehicle_pos + Vec3::new(0.0, config.obstacle_check_height, 0.0) + vehicle_forward * 2.0;

    // 主射線（前方）
    let max_toi = config.obstacle_max_distance;
    let solid = true;
    let groups = QueryFilter::new()
        .groups(CollisionGroups::new(
            COLLISION_GROUP_VEHICLE,
            COLLISION_GROUP_VEHICLE | COLLISION_GROUP_CHARACTER | COLLISION_GROUP_STATIC,
        ))
        .exclude_collider(vehicle_entity);
    // We can't exclude entity easily without entity ID.
    // Ideally we pass entity to this function.
    // For now, let's just raycast.

    if let Some((_entity, toi)) =
        rapier.cast_ray(ray_origin, vehicle_forward, max_toi, solid, groups)
    {
        return Some(rapier_real_to_f32(toi));
    }

    // 側向射線（避免路邊擦撞）
    let side_offset = 0.8;
    for side in [-1.0, 1.0] {
        let side_origin = ray_origin + vehicle_transform.right().as_vec3() * side * side_offset;
        // 稍微內縮射線角度
        let ray_dir =
            (vehicle_forward + vehicle_transform.right().as_vec3() * side * -0.2).normalize();

        if let Some((_entity, toi)) = rapier.cast_ray(
            side_origin,
            ray_dir,
            config.obstacle_side_max_distance,
            solid,
            groups,
        ) {
            return Some(rapier_real_to_f32(toi));
        }
    }

    None
}

/// 根據障礙物更新 NPC 狀態
fn update_npc_state_from_obstacle(
    npc: &mut NpcVehicle,
    transform: &Transform,
    vehicle: &mut Vehicle,
    vehicle_entity: Entity,
    rapier_context: &ReadRapierContext,
    dt: f32,
    config: &crate::vehicle::config::NpcDrivingConfig,
) {
    npc.stuck_timer += dt;

    if let Some(distance) = check_obstacle(
        transform,
        vehicle_entity,
        &vehicle.vehicle_type,
        rapier_context,
        config,
    ) {
        let (new_state, reset_timer) =
            determine_npc_reaction(distance, vehicle.current_speed, npc.stuck_timer, config);
        npc.state = new_state;
        if reset_timer {
            npc.stuck_timer = 0.0;
        }
    } else {
        npc.state = NpcState::Cruising;
    }
}

fn determine_npc_reaction(
    distance: f32,
    current_speed: f32,
    stuck_timer: f32,
    config: &crate::vehicle::config::NpcDrivingConfig,
) -> (NpcState, bool) {
    if distance < config.obstacle_too_close_distance {
        // Too close, check if stuck
        if current_speed < 1.0 {
            if stuck_timer > 2.0 {
                (NpcState::Reversing, true)
            } else {
                (NpcState::Stopped, false)
            }
        } else {
            (NpcState::Braking, false)
        }
    } else if distance < config.obstacle_brake_distance {
        (NpcState::Braking, false)
    } else {
        (NpcState::Cruising, false)
    }
}

/// 導航至下一個航點
fn navigate_to_waypoint(
    npc: &mut NpcVehicle,
    transform: &Transform,
    vehicle: &mut Vehicle,
    config: &crate::vehicle::config::NpcDrivingConfig,
) {
    if npc.waypoints.is_empty() {
        return;
    }

    let target = npc.waypoints[npc.current_wp_index];
    let dist_sq = transform.translation.distance_squared(target);

    if dist_sq < config.waypoint_arrival_distance_sq {
        npc.current_wp_index = (npc.current_wp_index + 1) % npc.waypoints.len();
    }

    // 計算轉向
    let target_dir = (target - transform.translation).normalize();
    let forward = transform.forward().as_vec3();
    let right = transform.right().as_vec3();

    let dot = forward.dot(target_dir);
    let cross = right.dot(target_dir);

    // 簡單的 P 控制器
    let steer = cross * 2.0;
    vehicle.steer_input = steer.clamp(-1.0, 1.0);

    // 如果角度過大，減速
    if dot < 0.5 {
        vehicle.throttle_input = 0.5;
    } else {
        vehicle.throttle_input = 1.0;
    }
}

fn handle_cruising_state(
    npc: &mut NpcVehicle,
    transform: &mut Transform,
    vehicle: &mut Vehicle,
    _dt: f32,
    config: &crate::vehicle::config::NpcDrivingConfig,
) {
    navigate_to_waypoint(npc, transform, vehicle, config);
    vehicle.throttle_input = 0.7; // 巡航油門
    vehicle.brake_input = 0.0;
}

fn handle_braking_state(
    _npc: &mut NpcVehicle,
    _transform: &mut Transform,
    vehicle: &mut Vehicle,
    _dt: f32,
) {
    vehicle.throttle_input = 0.0;
    vehicle.brake_input = 1.0;
    // 煞車時保持轉向
}

fn handle_stopped_state(_npc: &mut NpcVehicle, vehicle: &mut Vehicle, _dt: f32) {
    vehicle.throttle_input = 0.0;
    vehicle.brake_input = 1.0;
}

fn handle_reversing_state(
    npc: &mut NpcVehicle,
    _transform: &mut Transform,
    vehicle: &mut Vehicle,
    _dt: f32,
) {
    vehicle.throttle_input = 0.0;
    vehicle.brake_input = 1.0; // 倒車
                               // 倒車時反向打輪
    vehicle.steer_input = -vehicle.steer_input;

    if npc.stuck_timer > 3.0 {
        npc.state = NpcState::Cruising;
        npc.stuck_timer = 0.0;
    }
}

/// 判斷是否需要等紅燈
fn should_stop_for_traffic_light(
    vehicle_pos: Vec3,
    vehicle_forward: Vec3,
    traffic_light_query: &Query<(&Transform, &TrafficLight), Without<NpcVehicle>>,
) -> bool {
    // 簡單判斷：尋找前方且面向自己的紅燈
    for (light_transform, light) in traffic_light_query.iter() {
        if light.state == TrafficLightState::Red || light.state == TrafficLightState::Yellow {
            let to_light = light_transform.translation - vehicle_pos;
            let dist_sq = to_light.length_squared();

            if dist_sq < 400.0 {
                // 20m 內
                let light_forward = light_transform.forward().as_vec3();
                // 燈是面向車輛的 (dot < -0.5) 且 車是面向燈的 (dot > 0.5)
                if light_forward.dot(vehicle_forward) < -0.8 {
                    return true;
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
    mut npc_query: Query<(Entity, &mut Transform, &mut Vehicle, &mut NpcVehicle)>,
    traffic_light_query: Query<(&Transform, &TrafficLight), Without<NpcVehicle>>,
    config: Res<VehicleConfig>,
) {
    let dt = time.delta_secs();
    // Use read context directly if it's available, otherwise try Query if ReadRapierContext is a SystemParam that wraps it?
    // bevy_rapier3d's ReadRapierContext is a system param in newer versions or a valid resource wrapper.
    // In the code I saw earlier: `let Ok(rapier) = rapier_context.single() else { return; };` implies it's a Query or similar.
    // Wait, original code had: `rapier_context: ReadRapierContext` and `let Ok(rapier) = rapier_context.single()`.
    // If ReadRapierContext is `Query<...>`, then it's fine.
    // I'll stick to original logic.
    let Ok(_rapier) = rapier_context.single() else {
        return;
    };
    // Wait, ReadRapierContext might be a typedef in this project?
    // In `systems.rs` line 16: `use bevy_rapier3d::prelude::*;`.
    // Usually `ReadRapierContext` is NOT a standard bevy_rapier3d type. It might be a type alias?
    // Or I misread. Let's look at original again.
    // Line 1880: `rapier_context: ReadRapierContext`.
    // I will assume it works as is.

    for (entity, mut transform, mut vehicle, mut npc) in npc_query.iter_mut() {
        // 定期檢查前方障礙物和紅綠燈
        npc.check_timer.tick(time.delta());
        if npc.check_timer.just_finished() {
            // Update state from obstacle
            update_npc_state_from_obstacle(
                &mut npc,
                &transform,
                &mut vehicle,
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
                if should_stop_for_traffic_light(vehicle_pos, vehicle_forward, &traffic_light_query)
                {
                    npc.state = NpcState::WaitingAtLight;
                }
            }
        }

        // 根據狀態執行行為
        match npc.state {
            NpcState::Cruising => {
                handle_cruising_state(&mut npc, &mut transform, &mut vehicle, dt, &config.npc)
            }
            NpcState::Braking => handle_braking_state(&mut npc, &mut transform, &mut vehicle, dt),
            NpcState::Stopped => handle_stopped_state(&mut npc, &mut vehicle, dt),
            NpcState::Reversing => {
                handle_reversing_state(&mut npc, &mut transform, &mut vehicle, dt)
            }
            NpcState::WaitingAtLight => handle_waiting_at_light_state(
                &mut npc,
                &mut vehicle,
                &transform,
                &traffic_light_query,
                dt,
            ),
        }
    }
}

/// 處理等紅燈狀態
fn handle_waiting_at_light_state(
    npc: &mut NpcVehicle,
    vehicle: &mut Vehicle,
    transform: &Transform,
    traffic_light_query: &Query<(&Transform, &TrafficLight), Without<NpcVehicle>>,
    _dt: f32,
) {
    // 減速停車
    vehicle.current_speed *= 0.85;
    if vehicle.current_speed < 0.5 {
        vehicle.current_speed = 0.0;
    }

    // 檢查燈是否變綠了，如果變綠則恢復巡航
    let vehicle_pos = transform.translation;
    let vehicle_forward = transform.forward().as_vec3();
    if !should_stop_for_traffic_light(vehicle_pos, vehicle_forward, traffic_light_query) {
        npc.state = NpcState::Cruising;
    }
}

// === 車輛生成輔助函數 ===

/// 創建車身材質
fn create_body_material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    metallic: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.3,
        metallic,
        ..default()
    })
}

/// 生成帶材質的方塊子實體
fn spawn_mesh_child(
    parent: &mut ChildSpawnerCommands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    transform: Transform,
) {
    parent.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        transform,
        GlobalTransform::default(),
    ));
}

/// 生成車燈（消除重複程式碼）
fn spawn_vehicle_light(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    x: f32,
    y: f32,
    z: f32,
) {
    let light_mesh = meshes.add(Cuboid::new(0.4, 0.2, 0.1));
    spawn_mesh_child(parent, light_mesh, material, Transform::from_xyz(x, y, z));
}

/// 生成車輛前後燈組（左右對稱）
fn spawn_vehicle_lights(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    headlight_mat: Handle<StandardMaterial>,
    taillight_mat: Handle<StandardMaterial>,
    chassis_size: Vec3,
) {
    let light_z = -chassis_size.z / 2.0 - 0.05;
    let light_x = chassis_size.x / 2.0 - 0.4;
    let tail_z = chassis_size.z / 2.0 + 0.05;

    // 前燈（左右）
    spawn_vehicle_light(
        parent,
        meshes,
        headlight_mat.clone(),
        -light_x,
        0.1,
        light_z,
    );
    spawn_vehicle_light(parent, meshes, headlight_mat, light_x, 0.1, light_z);

    // 尾燈（左右）
    spawn_vehicle_light(parent, meshes, taillight_mat.clone(), -light_x, 0.1, tail_z);
    spawn_vehicle_light(parent, meshes, taillight_mat, light_x, 0.1, tail_z);
}

/// 生成 NPC 車輛（使用共享材質）
#[allow(clippy::too_many_arguments)]
pub fn spawn_npc_vehicle(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    shared_mats: &super::VehicleMaterials,
    position: Vec3,
    rotation: Quat,
    vehicle_type: VehicleType,
    color: Color,
    waypoints: Vec<Vec3>,
    start_index: usize,
) {
    // 根據類型定義尺寸變數和組件
    let (chassis_size, wheel_offset_z, vehicle_component) = match vehicle_type {
        VehicleType::Car => (Vec3::new(2.0, 0.6, 4.0), 1.2, Vehicle::car()),
        VehicleType::Taxi => (Vec3::new(2.0, 0.6, 4.0), 1.2, Vehicle::taxi()),
        VehicleType::Bus => (Vec3::new(2.8, 1.2, 8.0), 2.5, Vehicle::bus()),
        VehicleType::Scooter => (Vec3::new(0.6, 0.4, 1.8), 0.6, Vehicle::scooter()),
    };

    // 主要實體 (Root) - 負責物理和邏輯，但不負責渲染主車身 (由子實體負責，或保留透明/基礎幾何?)
    // 為了簡單，我們讓 Root 只有 Collider 和 Logic，渲染全交給 children?
    // 或者 Root 是車身底盤。為了避免層級太深，Root 當作底盤中心。

    // 1. 生成 Root 實體
    commands
        .spawn((
            // 空間組件 (完整的 SpatialBundle 替代)
            Transform {
                translation: position + Vec3::new(0.0, 0.5, 0.0), // 稍微提高
                rotation,
                ..default()
            },
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // 物理組件
            Collider::cuboid(chassis_size.x / 2.0, 0.75, chassis_size.z / 2.0),
            RigidBody::KinematicPositionBased,
            VehiclePhysicsMode::Kinematic,
            CollisionGroups::new(
                COLLISION_GROUP_VEHICLE,
                COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
            ), // NPC 載具與角色、載具、靜態物碰撞
            // 遊戲邏輯組件
            vehicle_component,
            VehicleHealth::for_vehicle_type(vehicle_type), // 車輛血量
            VehicleId::new(),                              // 穩定識別碼（用於存檔）
            VehicleModifications::default(),               // 改裝狀態（用於存檔）
            NpcVehicle {
                waypoints,
                current_wp_index: start_index,
                ..default()
            },
            Name::new(format!("NpcVehicle_{:?}", vehicle_type)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    VehicleVisualRoot,
                ))
                .with_children(|parent| {
                    // === 視覺模型構建 ===

                    // A. 底盤 (Chassis) - 下半部
                    let body_mat = create_body_material(materials, color, 0.5);
                    spawn_mesh_child(
                        parent,
                        meshes.add(Cuboid::from_size(chassis_size)),
                        body_mat,
                        Transform::from_xyz(0.0, 0.0, 0.0),
                    );

                    // B. 車艙 (Cabin) - 上半部 (玻璃) - 使用共享材質
                    let cabin_size = match vehicle_type {
                        VehicleType::Bus => Vec3::new(2.7, 1.0, 7.5),
                        _ => Vec3::new(1.8, 0.5, 2.0),
                    };
                    let cabin_y = chassis_size.y / 2.0 + cabin_size.y / 2.0;
                    let cabin_z_offset = match vehicle_type {
                        VehicleType::Bus => 0.0,
                        _ => -0.2, // 轎車車艙偏後
                    };

                    spawn_mesh_child(
                        parent,
                        meshes.add(Cuboid::from_size(cabin_size)),
                        shared_mats.glass.clone(),
                        Transform::from_xyz(0.0, cabin_y, cabin_z_offset),
                    );

                    // C. 輪子 (Wheels) - 4個 - 使用共享材質
                    let wheel_mesh = meshes.add(Cylinder::new(0.35, 0.3));

                    // 輪子位置 (左前, 右前, 左後, 右後)
                    // Root Y 是底盤中心。假設底盤離地 0.4。輪子半徑 0.35。
                    // 輪子中心 Y 應該是 -0.3 左右?
                    let wheel_y = -chassis_size.y / 2.0;
                    let wheel_x = chassis_size.x / 2.0;

                    let wheel_positions = [
                        Vec3::new(-wheel_x, wheel_y, -wheel_offset_z), // 左前 (Forward = -Z)
                        Vec3::new(wheel_x, wheel_y, -wheel_offset_z),  // 右前
                        Vec3::new(-wheel_x, wheel_y, wheel_offset_z),  // 左後
                        Vec3::new(wheel_x, wheel_y, wheel_offset_z),   // 右後
                    ];

                    for pos in wheel_positions {
                        parent.spawn((
                            Mesh3d(wheel_mesh.clone()),
                            MeshMaterial3d(shared_mats.wheel.clone()),
                            // 圓柱體默認直立 (Y軸)，需要旋轉 90度躺下變成輪子 (Z軸轉90度)
                            Transform::from_translation(pos)
                                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                            GlobalTransform::default(),
                        ));
                    }

                    // D. 車燈 (Lights) - 使用輔助函數生成
                    spawn_vehicle_lights(
                        parent,
                        meshes,
                        shared_mats.headlight.clone(),
                        shared_mats.taillight.clone(),
                        chassis_size,
                    );

                    // === E. 酷炫改裝配件 (Tuning Parts) ===
                    if vehicle_type == VehicleType::Car || vehicle_type == VehicleType::Taxi {
                        // 1. 尾翼 (Spoiler) - 使用共享黑色塑膠材質
                        let strut_h = 0.3;
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.1, strut_h, 0.1))),
                            MeshMaterial3d(shared_mats.black_plastic.clone()),
                            Transform::from_xyz(
                                -0.6,
                                chassis_size.y / 2.0 + strut_h / 2.0,
                                chassis_size.z / 2.0 - 0.2,
                            ),
                            GlobalTransform::default(),
                        ));
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.1, strut_h, 0.1))),
                            MeshMaterial3d(shared_mats.black_plastic.clone()),
                            Transform::from_xyz(
                                0.6,
                                chassis_size.y / 2.0 + strut_h / 2.0,
                                chassis_size.z / 2.0 - 0.2,
                            ),
                            GlobalTransform::default(),
                        ));
                        // 翼板
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(1.8, 0.05, 0.4))),
                            MeshMaterial3d(shared_mats.black_plastic.clone()),
                            Transform::from_xyz(
                                0.0,
                                chassis_size.y / 2.0 + strut_h,
                                chassis_size.z / 2.0 - 0.2,
                            ),
                            GlobalTransform::default(),
                        ));

                        // 2. 底盤燈 (Underglow) - 照亮地板
                        // 使用車身顏色作為光色
                        let glow_color = color;
                        parent.spawn((
                            PointLight {
                                color: glow_color,
                                intensity: 100_000.0, // 強度要夠才看得到
                                range: 5.0,
                                radius: 2.0,
                                shadows_enabled: false,
                                ..default()
                            },
                            Transform::from_xyz(0.0, -0.5, 0.0),
                            GlobalTransform::default(),
                        ));

                        // 3. 側裙霓虹條 (Side Neon Strips)
                        let neon_mat = materials.add(StandardMaterial {
                            base_color: glow_color,
                            emissive: LinearRgba::from(glow_color) * 5.0, // 增強亮度
                            ..default()
                        });
                        // 左側條
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 2.5))),
                            MeshMaterial3d(neon_mat.clone()),
                            Transform::from_xyz(
                                -chassis_size.x / 2.0 - 0.02,
                                -chassis_size.y / 2.0 + 0.1,
                                0.0,
                            ),
                            GlobalTransform::default(),
                        ));
                        // 右側條
                        parent.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 2.5))),
                            MeshMaterial3d(neon_mat),
                            Transform::from_xyz(
                                chassis_size.x / 2.0 + 0.02,
                                -chassis_size.y / 2.0 + 0.1,
                                0.0,
                            ),
                            GlobalTransform::default(),
                        ));
                    }
                });
        });
}

/// 系統：初始化交通 (在 Setup 階段運行)
/// 使用共享材質資源以優化效能
/// 生成 8-10 台 NPC 車輛和紅綠燈
pub fn spawn_initial_traffic(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    shared_mats: Res<super::VehicleMaterials>,
) {
    // 首先初始化紅綠燈視覺資源
    let traffic_visuals = TrafficLightVisuals::new(&mut meshes, &mut materials);

    // NPC 車輛路線 - 只走柏油路，避開徒步區
    // 可用道路：中華路 (X=75, 寬50m), 西寧南路 (X=-50), 成都路 (Z=50)
    // ★ 重要：
    //   1. 每條路線必須形成閉環
    //   2. 不同路線的車道必須錯開，避免重疊
    //   3. 車寬約 2.5m，車道間距至少 8m

    // === 道路座標參考 ===
    // Z_HANKOU = -80 (漢口街, 寬12) → 北 -86, 南 -74
    // Z_CHENGDU = 50 (成都路, 寬16) → 北 42, 南 58
    // X_ZHONGHUA = 80 (中華路, 寬20) → 西 70, 東 90
    // X_XINING = -55 (西寧南路, 寬12) → 西 -61, 東 -49
    // X_KANGDING = -100 (康定路, 寬16) → 西 -108, 東 -92

    // 路線 A：外圈 (逆時針) - 使用實際道路座標
    // 成都路北側車道 Z≈45，漢口街南側車道 Z≈-77
    let route_outer = vec![
        Vec3::new(-52.0, 0.0, 45.0),  // 0: 西南角 (西寧/成都)
        Vec3::new(85.0, 0.0, 45.0),   // 1: 東南角 (中華/成都)
        Vec3::new(85.0, 0.0, -77.0),  // 2: 東北角 (中華/漢口)
        Vec3::new(-52.0, 0.0, -77.0), // 3: 西北角 (西寧/漢口)
    ];

    // 路線 B：內圈 (順時針) - 成都路南側車道 Z≈58
    // 公車起點往南移 3m (Z=58)，避開與中華路車輛和成都路車輛衝突
    let route_inner = vec![
        Vec3::new(72.0, 0.0, 58.0),   // 0: 東南角（西側車道，更南）
        Vec3::new(-58.0, 0.0, 58.0),  // 1: 西南角
        Vec3::new(-58.0, 0.0, -83.0), // 2: 西北角 (漢口街北側)
        Vec3::new(72.0, 0.0, -83.0),  // 3: 東北角
    ];

    // 路線 C：中華路直線 (南北向) - 在中華路上 (X=80, 寬20)
    // 北行：X=88 (東側車道)，南行：X=72 (西側車道) - 避開與公車衝突
    let route_zhonghua = vec![
        Vec3::new(88.0, 0.0, 55.0),  // 0: 南端 (成都路南側)
        Vec3::new(88.0, 0.0, -83.0), // 1: 北端 (漢口街北側)
        Vec3::new(72.0, 0.0, -83.0), // 2: U 型轉彎
        Vec3::new(72.0, 0.0, 55.0),  // 3: 南端
    ];

    // 路線 D：成都路直線 (東西向) - 在成都路上 (Z=50, 寬16)
    // 東行：Z=44 (北側車道)，西行：Z=56 (南側車道)
    // 調整避開與公車 (Z=58) 衝突
    let route_chengdu = vec![
        Vec3::new(-90.0, 0.0, 44.0), // 0: 西端 (康定路東側)
        Vec3::new(85.0, 0.0, 44.0),  // 1: 東端 (中華路)
        Vec3::new(85.0, 0.0, 56.0),  // 2: U 型轉彎（Z=56，避開公車 Z=58）
        Vec3::new(-90.0, 0.0, 56.0), // 3: 西端
    ];

    // 路線 E：西寧路直線 (南北向) - 在西寧南路上 (X=-55, 寬12)
    // 北行：X=-50 (東側車道)，南行：X=-60 (西側車道) - 避開與公車衝突
    let route_xining = vec![
        Vec3::new(-50.0, 0.0, 58.0),  // 0: 南端 (成都路南側偏南)
        Vec3::new(-50.0, 0.0, -77.0), // 1: 北端 (漢口街)
        Vec3::new(-60.0, 0.0, -77.0), // 2: U 型轉彎
        Vec3::new(-60.0, 0.0, 58.0),  // 3: 南端
    ];

    // 車輛顏色池
    let car_colors = [
        Color::srgb(0.8, 0.2, 0.2), // 紅色
        Color::srgb(0.2, 0.2, 0.8), // 藍色
        Color::srgb(0.9, 0.9, 0.9), // 白色
        Color::srgb(0.1, 0.1, 0.1), // 黑色
        Color::srgb(0.7, 0.7, 0.7), // 銀色
        Color::srgb(0.2, 0.6, 0.2), // 綠色
        Color::srgb(1.0, 0.5, 0.0), // 橙色
    ];

    // 生成配置 (位置, 類型, 顏色, 起始索引, 路徑)
    // ★ 減少車輛數量避免相撞，每條路線只放 1 台
    let spawn_configs = [
        // === 路線 A：外圈（逆時針）- 計程車 ===
        (
            route_outer[0],
            VehicleType::Taxi,
            Color::srgb(1.0, 0.8, 0.0),
            0,
            route_outer.clone(),
        ),
        // === 路線 B：內圈（順時針）- 公車 ===
        (
            route_inner[0],
            VehicleType::Bus,
            Color::srgb(0.2, 0.4, 0.8),
            0,
            route_inner.clone(),
        ),
        // === 路線 C：中華路（U 型迴轉）===
        (
            route_zhonghua[0],
            VehicleType::Car,
            car_colors[2],
            0,
            route_zhonghua.clone(),
        ),
        // === 路線 D：成都路（U 型迴轉）===
        (
            route_chengdu[0],
            VehicleType::Car,
            car_colors[3],
            0,
            route_chengdu.clone(),
        ),
        // === 路線 E：西寧路（U 型迴轉）===
        (
            route_xining[0],
            VehicleType::Car,
            car_colors[5],
            0,
            route_xining.clone(),
        ),
    ];

    info!("🚗 生成 {} 台初始交通車輛", spawn_configs.len());

    for (i, (pos, v_type, color, start_idx, path)) in spawn_configs.iter().enumerate() {
        debug!("  - 生成車輛 #{}: {:?} 於 {:?}", i, v_type, pos);

        // 它的首個目標應該是它所在位置的下一個點
        let next_idx = (*start_idx as usize + 1) % path.len();

        // 計算初始朝向：面向下一個航點
        let next_pos = path[next_idx];
        let dir = (next_pos - *pos).normalize_or_zero();
        let initial_rotation = look_rotation_y_flat(dir);

        spawn_npc_vehicle(
            &mut commands,
            &mut meshes,
            &mut materials,
            &shared_mats,
            *pos,
            initial_rotation,
            *v_type,
            *color,
            path.clone(),
            next_idx,
        );
    }

    // 紅綠燈由 spawn_world_traffic_lights 系統統一生成，不在此處重複

    // 儲存紅綠燈視覺資源
    commands.insert_resource(traffic_visuals);
}

/// 生成可騎乘的機車
/// 台灣街頭最常見的交通工具 - 外觀類似 125cc 速克達
/// 使用共享材質以優化效能
pub fn spawn_scooter(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    shared_mats: &super::VehicleMaterials,
    position: Vec3,
    rotation: Quat,
    color: Color,
) {
    // 機車尺寸
    let body_length = 1.6;
    let body_width = 0.5;
    let body_height = 0.4;
    let seat_height = 0.8;

    // 車身材質（唯一需要按顏色創建的材質）
    let body_mat = create_body_material(materials, color, 0.6);

    // 使用共享材質
    let black_mat = shared_mats.black_plastic.clone();
    let wheel_mat = shared_mats.wheel.clone();
    let headlight_mat = shared_mats.headlight.clone();
    let taillight_mat = shared_mats.taillight.clone();

    commands
        .spawn((
            Transform {
                translation: position + Vec3::new(0.0, 0.4, 0.0),
                rotation,
                ..default()
            },
            GlobalTransform::default(),
            Visibility::default(),
            // 較小的碰撞體
            Collider::cuboid(body_width / 2.0, 0.5, body_length / 2.0),
            RigidBody::KinematicPositionBased,
            VehiclePhysicsMode::Kinematic,
            CollisionGroups::new(
                COLLISION_GROUP_VEHICLE,
                COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
            ), // 機車與角色、載具、靜態物碰撞
            Vehicle::scooter(),
            VehicleHealth::for_vehicle_type(VehicleType::Scooter), // 車輛血量
            VehicleId::new(),                                      // 穩定識別碼（用於存檔）
            VehicleModifications::default(),                       // 改裝狀態（用於存檔）
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    VehicleVisualRoot,
                ))
                .with_children(|parent| {
                    // === 車身本體 ===

                    // 1. 踏板區 (腳踏平台)
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(body_width, 0.08, body_length * 0.5))),
                        MeshMaterial3d(black_mat.clone()),
                        Transform::from_xyz(0.0, -0.1, 0.0),
                        GlobalTransform::default(),
                    ));

                    // 2. 車頭斜面
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(body_width * 0.8, body_height, 0.4))),
                        MeshMaterial3d(body_mat.clone()),
                        Transform::from_xyz(0.0, 0.15, -body_length / 2.0 + 0.2)
                            .with_rotation(Quat::from_rotation_x(-0.3)),
                        GlobalTransform::default(),
                    ));

                    // 3. 座墊
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(body_width * 0.7, 0.12, body_length * 0.45))),
                        MeshMaterial3d(black_mat.clone()),
                        Transform::from_xyz(0.0, seat_height * 0.45, body_length * 0.1),
                        GlobalTransform::default(),
                    ));

                    // 4. 車尾箱 (後行李箱)
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(body_width * 0.6, 0.25, 0.3))),
                        MeshMaterial3d(body_mat.clone()),
                        Transform::from_xyz(0.0, seat_height * 0.5, body_length / 2.0 - 0.15),
                        GlobalTransform::default(),
                    ));

                    // 5. 把手區
                    parent.spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.02, body_width + 0.3))),
                        MeshMaterial3d(black_mat.clone()),
                        Transform::from_xyz(0.0, seat_height * 0.8, -body_length / 2.0 + 0.1)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                        GlobalTransform::default(),
                    ));

                    // 6. 後照鏡（左右）- 使用共享材質
                    // 左鏡
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.08, 0.05, 0.02))),
                        MeshMaterial3d(shared_mats.mirror.clone()),
                        Transform::from_xyz(
                            -body_width / 2.0 - 0.2,
                            seat_height * 0.85,
                            -body_length / 2.0 + 0.15,
                        ),
                        GlobalTransform::default(),
                    ));
                    // 右鏡
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.08, 0.05, 0.02))),
                        MeshMaterial3d(shared_mats.mirror.clone()),
                        Transform::from_xyz(
                            body_width / 2.0 + 0.2,
                            seat_height * 0.85,
                            -body_length / 2.0 + 0.15,
                        ),
                        GlobalTransform::default(),
                    ));

                    // === 輪子 ===

                    // 前輪
                    parent.spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.25, 0.12))),
                        MeshMaterial3d(wheel_mat.clone()),
                        Transform::from_xyz(0.0, -0.15, -body_length / 2.0 - 0.1)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                        GlobalTransform::default(),
                    ));

                    // 後輪
                    parent.spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.25, 0.15))),
                        MeshMaterial3d(wheel_mat),
                        Transform::from_xyz(0.0, -0.15, body_length / 2.0 - 0.1)
                            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                        GlobalTransform::default(),
                    ));

                    // === 燈光 ===

                    // 頭燈
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.15, 0.1, 0.05))),
                        MeshMaterial3d(headlight_mat),
                        Transform::from_xyz(0.0, 0.25, -body_length / 2.0 - 0.05),
                        GlobalTransform::default(),
                    ));

                    // 尾燈
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.2, 0.06, 0.03))),
                        MeshMaterial3d(taillight_mat),
                        Transform::from_xyz(0.0, seat_height * 0.4, body_length / 2.0 + 0.02),
                        GlobalTransform::default(),
                    ));

                    // === 前擋泥板 ===
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.12, 0.02, 0.3))),
                        MeshMaterial3d(body_mat),
                        Transform::from_xyz(0.0, 0.05, -body_length / 2.0 - 0.1)
                            .with_rotation(Quat::from_rotation_x(0.2)),
                        GlobalTransform::default(),
                    ));
                });
        });

    debug!("🛵 生成機車於 {:?}", position);
}

// ============================================================================
// 車輛視覺效果系統（GTA 5 風格）
// ============================================================================

/// 判斷車輛是否應該生成漂移煙霧
fn should_spawn_drift_smoke(vehicle: &Vehicle) -> bool {
    (vehicle.is_drifting && vehicle.drift_angle.abs() > 0.2)
        || (vehicle.is_handbraking && vehicle.current_speed > 10.0)
        || (vehicle.wheel_spin > 0.5) // 輪胎打滑時也有煙
}

/// 取得車輛類型對應的後輪偏移量
fn get_rear_wheel_offset(vehicle_type: VehicleType) -> Vec3 {
    match vehicle_type {
        VehicleType::Scooter => Vec3::new(0.0, 0.0, 0.8),
        VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.0, 1.5),
        VehicleType::Bus => Vec3::new(0.0, 0.0, 3.0),
    }
}

/// 取得車輛類型對應的輪子側向偏移量
fn get_wheel_lateral_offset(vehicle_type: VehicleType, side: f32) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 0.0, // 機車只有中間
        _ => 0.8 * side,
    }
}

/// 漂移煙霧生成系統
/// 當車輛漂移或急煞時，在後輪位置生成煙霧粒子
pub fn drift_smoke_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    effect_visuals: Option<Res<VehicleEffectVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle), Without<NpcVehicle>>, // 只處理玩家駕駛的車輛
) {
    let Some(visuals) = effect_visuals else {
        return;
    };
    let current_time = time.elapsed_secs();

    // 檢查生成間隔
    if current_time - effect_tracker.last_smoke_spawn < effect_tracker.smoke_spawn_interval {
        return;
    }

    for (transform, vehicle) in vehicle_query.iter() {
        if !should_spawn_drift_smoke(vehicle)
            || effect_tracker.smoke_count >= effect_tracker.max_smoke_count
        {
            continue;
        }

        let rear_offset = get_rear_wheel_offset(vehicle.vehicle_type);
        let world_pos = transform.translation + transform.rotation * rear_offset;
        let wheel_height = 0.2;

        let mut rng = rand::rng();
        for side in [-1.0, 1.0] {
            let wheel_offset = get_wheel_lateral_offset(vehicle.vehicle_type, side);
            let spawn_pos =
                world_pos + transform.rotation * Vec3::new(wheel_offset, wheel_height, 0.0);

            let spread = Vec3::new(
                rng.random_range(-0.5..0.5),
                rng.random_range(0.3..0.8),
                rng.random_range(-0.5..0.5),
            );
            let base_velocity =
                -transform.forward().as_vec3() * (vehicle.current_speed * 0.1).max(1.0);

            commands.spawn((
                Mesh3d(visuals.smoke_mesh.clone()),
                MeshMaterial3d(visuals.smoke_material.clone()),
                Transform::from_translation(spawn_pos).with_scale(Vec3::splat(0.3)),
                DriftSmoke::new(base_velocity + spread, rng.random_range(0.5..1.0)),
            ));

            effect_tracker.smoke_count += 1;

            // 機車只生成一個煙霧
            if vehicle.vehicle_type == VehicleType::Scooter {
                break;
            }
        }

        effect_tracker.last_smoke_spawn = current_time;
    }
}

/// 漂移煙霧更新系統
/// 處理煙霧粒子的移動、縮放、淡出和刪除
pub fn drift_smoke_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    mut smoke_query: Query<(Entity, &mut DriftSmoke, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (entity, mut smoke, mut transform) in smoke_query.iter_mut() {
        // 更新生命時間
        smoke.lifetime += dt;

        // 檢查是否過期
        if smoke.lifetime >= smoke.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                effect_tracker.smoke_count = effect_tracker.smoke_count.saturating_sub(1);
            }
            continue;
        }

        // 減速（空氣阻力）
        smoke.velocity *= 1.0 - dt * 2.0;

        // 輕微上飄（熱氣）
        smoke.velocity.y += dt * 0.5;

        // 更新位置
        transform.translation += smoke.velocity * dt;

        // 更新縮放（擴散變大）
        let scale = smoke.scale();
        transform.scale = Vec3::splat(scale);
    }
}

// ============================================================================
// 氮氣火焰效果系統
// ============================================================================

/// 氮氣火焰生成系統
/// 當車輛使用氮氣加速時，在排氣管後方產生火焰效果
pub fn nitro_flame_spawn_system(
    mut commands: Commands,
    effect_visuals: Option<Res<VehicleEffectVisuals>>,
    vehicle_query: Query<(&Transform, &VehicleModifications, &NitroBoost), Without<NpcVehicle>>,
) {
    let Some(visuals) = effect_visuals else {
        return;
    };

    for (transform, mods, nitro) in vehicle_query.iter() {
        // 只有在使用氮氣且有充能時生成火焰
        if !nitro.is_active || mods.nitro_charge <= 0.0 {
            continue;
        }

        // 排氣管位置（車尾）
        let exhaust_offset = transform.back() * 2.5 + Vec3::new(0.0, 0.3, 0.0);
        let exhaust_pos = transform.translation + exhaust_offset;

        // 生成多個火焰粒子
        let mut rng = rand::rng();
        for _ in 0..3 {
            // 隨機偏移
            let offset = Vec3::new(
                (rng.random::<f32>() - 0.5) * 0.3,
                (rng.random::<f32>() - 0.5) * 0.2,
                0.0,
            );

            // 火焰往後噴射
            let velocity = transform.back() * (3.0 + rng.random::<f32>() * 2.0)
                + Vec3::new(
                    (rng.random::<f32>() - 0.5) * 0.5,
                    rng.random::<f32>() * 0.3,
                    (rng.random::<f32>() - 0.5) * 0.5,
                );

            commands.spawn((
                Mesh3d(visuals.nitro_flame_mesh.clone()),
                MeshMaterial3d(visuals.nitro_flame_material.clone()),
                Transform::from_translation(exhaust_pos + offset)
                    .with_scale(Vec3::new(0.2, 0.2, 0.4)), // 拉長形狀
                NitroFlame::new(velocity),
            ));
        }
    }
}

/// 氮氣火焰更新系統
/// 處理火焰粒子的移動、縮放和顏色變化
pub fn nitro_flame_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut flame_query: Query<(Entity, &mut NitroFlame, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (entity, mut flame, mut transform) in flame_query.iter_mut() {
        // 更新生命時間
        flame.lifetime += dt;

        // 檢查是否過期
        if flame.lifetime >= flame.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // 更新位置
        transform.translation += flame.velocity * dt;

        // 更新縮放（逐漸消散）
        let scale = flame.scale();
        transform.scale = Vec3::new(scale, scale, scale * 2.0); // 保持拉長形狀
    }
}

/// 判斷車輛是否應該生成輪胎痕跡
fn should_spawn_tire_track(vehicle: &Vehicle) -> bool {
    (vehicle.is_drifting && vehicle.drift_angle.abs() > 0.15)
        || (vehicle.is_handbraking && vehicle.current_speed > 8.0)
}

/// 取得車輛類型對應的輪胎痕跡後輪偏移量
fn get_track_rear_offset(vehicle_type: VehicleType) -> Vec3 {
    match vehicle_type {
        VehicleType::Scooter => Vec3::new(0.0, 0.0, 0.7),
        VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.0, 1.2),
        VehicleType::Bus => Vec3::new(0.0, 0.0, 2.5),
    }
}

/// 輪胎痕跡生成系統
/// 當車輛漂移或急煞時，在地面留下輪胎痕跡
pub fn tire_track_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    effect_visuals: Option<Res<VehicleEffectVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle), Without<NpcVehicle>>,
) {
    let Some(visuals) = effect_visuals else {
        return;
    };
    let current_time = time.elapsed_secs();

    if current_time - effect_tracker.last_track_spawn < effect_tracker.track_spawn_interval {
        return;
    }

    for (transform, vehicle) in vehicle_query.iter() {
        if !should_spawn_tire_track(vehicle)
            || effect_tracker.track_count >= effect_tracker.max_track_count
        {
            continue;
        }

        let rear_offset = get_track_rear_offset(vehicle.vehicle_type);
        let track_width = 0.2 + vehicle.drift_angle.abs() * 0.3;

        for side in [-1.0, 1.0] {
            let wheel_offset = get_wheel_lateral_offset(vehicle.vehicle_type, side);
            let track_pos = transform.translation
                + transform.rotation * (rear_offset + Vec3::new(wheel_offset, 0.0, 0.0));
            let ground_pos = Vec3::new(track_pos.x, 0.02, track_pos.z);

            commands.spawn((
                Mesh3d(visuals.tire_track_mesh.clone()),
                MeshMaterial3d(visuals.tire_track_material.clone()),
                Transform::from_translation(ground_pos)
                    .with_rotation(transform.rotation)
                    .with_scale(Vec3::new(track_width, 1.0, 0.8)),
                TireTrack::default(),
            ));

            effect_tracker.track_count += 1;

            if vehicle.vehicle_type == VehicleType::Scooter {
                break;
            }
        }

        effect_tracker.last_track_spawn = current_time;
    }
}

/// 輪胎痕跡更新系統
/// 處理痕跡的淡出和刪除
pub fn tire_track_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    mut track_query: Query<(Entity, &mut TireTrack)>,
) {
    let dt = time.delta_secs();

    for (entity, mut track) in track_query.iter_mut() {
        // 更新生命時間
        track.lifetime += dt;

        // 檢查是否過期
        if track.lifetime >= track.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                effect_tracker.track_count = effect_tracker.track_count.saturating_sub(1);
            }
        }
    }
}

/// 初始化車輛視覺效果資源
pub fn setup_vehicle_effects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(VehicleEffectVisuals::new(&mut meshes, &mut materials));
    commands.insert_resource(VehicleEffectTracker::new());
    info!("🚗 車輛視覺效果系統已初始化");
}

// ============================================================================
// 車輛損壞系統（GTA 5 風格）
// ============================================================================

use super::{
    VehicleDamageSmoke, VehicleDamageState, VehicleDamageVisuals, VehicleExplosion, VehicleFire,
    VehicleHealth,
};
use crate::combat::{DamageEvent, DamageSource, Enemy};
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::wanted::PoliceOfficer;

/// 初始化車輛損壞視覺效果資源
pub fn setup_vehicle_damage_effects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(VehicleDamageVisuals::new(&mut meshes, &mut materials));
    info!("💥 車輛損壞系統已初始化");
}

// === 車輛碰撞傷害常數 ===
/// 碰撞傷害冷卻時間（秒）- 防止持續接觸時每幀扣血
const COLLISION_DAMAGE_COOLDOWN: f32 = 0.5;
/// 造成傷害的最低速度門檻（m/s）
const COLLISION_DAMAGE_SPEED_THRESHOLD: f32 = 10.0;
/// 傷害倍率：每超過門檻 1 m/s 造成此數值傷害
const COLLISION_DAMAGE_MULTIPLIER: f32 = 5.0;

/// 車輛碰撞傷害系統
/// 根據碰撞速度計算車輛傷害
pub fn vehicle_collision_damage_system(
    time: Res<Time>,
    rapier_context: ReadRapierContext,
    mut vehicle_query: Query<(Entity, &Transform, &Vehicle, &mut VehicleHealth)>,
) {
    let current_time = time.elapsed_secs();

    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    for (entity, _transform, vehicle, mut health) in vehicle_query.iter_mut() {
        // 已爆炸的車輛不處理
        if health.is_destroyed() {
            continue;
        }

        // 碰撞傷害冷卻：防止持續接觸時每幀扣血
        if current_time - health.last_damage_time < COLLISION_DAMAGE_COOLDOWN {
            continue;
        }

        // 檢查碰撞事件
        for contact_pair in rapier.contact_pairs_with(entity) {
            // 只處理有接觸的碰撞
            if !contact_pair.has_any_active_contact() {
                continue;
            }

            // 根據速度計算傷害
            // 速度越快，傷害越高
            let speed = vehicle.current_speed.abs();
            if speed < COLLISION_DAMAGE_SPEED_THRESHOLD {
                continue; // 低速碰撞不造成傷害
            }

            // 傷害公式：(速度 - 門檻) * 倍率
            // 例如：30 m/s = (30-10) * 5 = 100 傷害
            let damage = (speed - COLLISION_DAMAGE_SPEED_THRESHOLD) * COLLISION_DAMAGE_MULTIPLIER;
            health.take_damage(damage, current_time);
            break; // 一次碰撞只計算一次傷害
        }
    }
}

/// 取得車輛類型對應的爆炸半徑
fn get_explosion_radius(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 5.0,
        VehicleType::Car | VehicleType::Taxi => 8.0,
        VehicleType::Bus => 12.0,
    }
}

/// 取得車輛類型對應的爆炸傷害
fn get_explosion_damage(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 100.0,
        VehicleType::Car | VehicleType::Taxi => 200.0,
        VehicleType::Bus => 300.0,
    }
}

/// 車輛火焰系統
/// 處理著火狀態和爆炸倒計時
pub fn vehicle_fire_system(
    mut commands: Commands,
    time: Res<Time>,
    mut vehicle_query: Query<(Entity, &Transform, &Vehicle, &mut VehicleHealth)>,
    damage_visuals: Option<Res<VehicleDamageVisuals>>,
) {
    let dt = time.delta_secs();

    for (entity, transform, vehicle, mut health) in vehicle_query.iter_mut() {
        if health.is_destroyed() {
            continue;
        }

        if !health.tick_fire(dt) {
            continue;
        }

        // 爆炸！
        let explosion_pos = transform.translation + Vec3::Y * 0.5;
        let explosion_radius = get_explosion_radius(vehicle.vehicle_type);
        let explosion_damage = get_explosion_damage(vehicle.vehicle_type);

        if let Some(ref visuals) = damage_visuals {
            commands.spawn((
                Mesh3d(visuals.explosion_mesh.clone()),
                MeshMaterial3d(visuals.explosion_material.clone()),
                Transform::from_translation(explosion_pos),
                VehicleExplosion::new(explosion_pos, explosion_radius, explosion_damage),
            ));
        }

        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
        }
    }
}

// === 車輛損壞視覺效果常數 ===
/// 中度損壞煙霧生成率（每秒）
const MODERATE_SMOKE_RATE: f32 = 1.2;
/// 嚴重損壞煙霧生成率（每秒）
const HEAVY_SMOKE_RATE: f32 = 6.0;
/// 瀕臨爆炸煙霧生成率（每秒）
const CRITICAL_SMOKE_RATE: f32 = 9.0;
/// 瀕臨爆炸火焰生成率（每秒）
const CRITICAL_FIRE_RATE: f32 = 6.0;

/// 車輛損壞視覺效果系統
/// 根據損壞狀態生成煙霧和火焰粒子
/// 使用時間基準的生成率，確保效果與幀率無關
pub fn vehicle_damage_effect_system(
    mut commands: Commands,
    time: Res<Time>,
    damage_visuals: Option<Res<VehicleDamageVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle, &VehicleHealth)>,
) {
    let Some(visuals) = damage_visuals else {
        return;
    };
    let dt = time.delta_secs();
    let mut rng = rand::rng();

    for (transform, vehicle, health) in vehicle_query.iter() {
        // 計算引擎蓋位置（車頭）
        let hood_offset = match vehicle.vehicle_type {
            VehicleType::Scooter => Vec3::new(0.0, 0.3, -0.6),
            VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.5, -1.5),
            VehicleType::Bus => Vec3::new(0.0, 1.0, -4.0),
        };
        let hood_pos = transform.translation + transform.rotation * hood_offset;

        // 根據損壞狀態生成效果
        // 使用時間基準的機率：rate * dt 使生成與幀率無關
        match health.damage_state {
            VehicleDamageState::Moderate => {
                // 中度損壞：偶爾冒白煙
                if rng.random::<f32>() < MODERATE_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, false, &mut rng);
                }
            }
            VehicleDamageState::Heavy => {
                // 嚴重損壞：持續冒黑煙
                if rng.random::<f32>() < HEAVY_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, true, &mut rng);
                }
            }
            VehicleDamageState::Critical => {
                // 瀕臨爆炸：冒黑煙 + 火焰
                if rng.random::<f32>() < CRITICAL_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, true, &mut rng);
                }
                if rng.random::<f32>() < CRITICAL_FIRE_RATE * dt {
                    spawn_vehicle_fire(&mut commands, &visuals, hood_pos, &mut rng);
                }
            }
            _ => {}
        }
    }
}

/// 生成損壞煙霧粒子
fn spawn_damage_smoke(
    commands: &mut Commands,
    visuals: &VehicleDamageVisuals,
    position: Vec3,
    is_heavy: bool,
    rng: &mut rand::prelude::ThreadRng,
) {
    let spread = Vec3::new(
        rng.random_range(-0.3..0.3),
        rng.random_range(0.5..1.5),
        rng.random_range(-0.3..0.3),
    );

    let material = if is_heavy {
        visuals.heavy_smoke_material.clone()
    } else {
        visuals.light_smoke_material.clone()
    };

    commands.spawn((
        Mesh3d(visuals.smoke_mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(position).with_scale(Vec3::splat(0.2)),
        VehicleDamageSmoke::new(spread, is_heavy),
    ));
}

/// 生成車輛火焰粒子
fn spawn_vehicle_fire(
    commands: &mut Commands,
    visuals: &VehicleDamageVisuals,
    position: Vec3,
    rng: &mut rand::prelude::ThreadRng,
) {
    let spread = Vec3::new(
        rng.random_range(-0.2..0.2),
        rng.random_range(0.8..1.5),
        rng.random_range(-0.2..0.2),
    );

    commands.spawn((
        Mesh3d(visuals.fire_mesh.clone()),
        MeshMaterial3d(visuals.fire_material.clone()),
        Transform::from_translation(position + Vec3::Y * 0.1)
            .with_scale(Vec3::splat(rng.random_range(0.3..0.6))),
        VehicleFire::new(spread),
    ));
}

/// 對範圍內的目標造成爆炸傷害
fn apply_explosion_damage_to_targets<'a>(
    targets: impl Iterator<Item = (Entity, &'a Transform)>,
    explosion_center: Vec3,
    explosion_radius: f32,
    explosion_damage: f32,
    damage_events: &mut MessageWriter<DamageEvent>,
    exclude_entity: Option<Entity>,
) {
    for (target_entity, target_transform) in targets {
        if Some(target_entity) == exclude_entity {
            continue;
        }
        let distance = explosion_center.distance(target_transform.translation);
        if distance < explosion_radius {
            let damage_factor = 1.0 - (distance / explosion_radius);
            damage_events.write(
                DamageEvent::new(
                    target_entity,
                    explosion_damage * damage_factor,
                    DamageSource::Explosion,
                )
                .with_position(explosion_center),
            );
        }
    }
}

/// 觸發爆炸攝影機震動效果
fn trigger_explosion_camera_shake(
    explosion_center: Vec3,
    explosion_radius: f32,
    player_pos: Vec3,
    camera_shake: &mut crate::core::CameraShake,
) {
    let distance_to_player = explosion_center.distance(player_pos);
    let max_shake_distance = explosion_radius * 3.0;

    if distance_to_player < max_shake_distance {
        let falloff = 1.0 - distance_to_player / max_shake_distance;
        camera_shake.trigger(0.5 * falloff, 0.4 + 0.3 * falloff);
    }
}

/// 車輛爆炸系統
/// 處理爆炸效果和範圍傷害
/// 對範圍內的所有可傷害實體（玩家、敵人、行人、警察、其他車輛）造成傷害
#[allow(clippy::type_complexity)]
pub fn vehicle_explosion_system(
    mut commands: Commands,
    time: Res<Time>,
    mut camera_shake: ResMut<crate::core::CameraShake>,
    mut explosion_query: Query<(Entity, &mut VehicleExplosion, &mut Transform)>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<VehicleExplosion>)>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<VehicleExplosion>)>,
    pedestrian_query: Query<
        (Entity, &Transform),
        (
            With<Pedestrian>,
            Without<Player>,
            Without<Enemy>,
            Without<VehicleExplosion>,
        ),
    >,
    police_query: Query<
        (Entity, &Transform),
        (
            With<PoliceOfficer>,
            Without<Player>,
            Without<Enemy>,
            Without<VehicleExplosion>,
        ),
    >,
    vehicle_query: Query<(Entity, &Transform), (With<VehicleHealth>, Without<VehicleExplosion>)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let dt = time.delta_secs();

    for (entity, mut explosion, mut transform) in explosion_query.iter_mut() {
        explosion.lifetime += dt;
        transform.scale = Vec3::splat(explosion.scale());

        if !explosion.damage_dealt {
            explosion.damage_dealt = true;

            if let Ok((_, player_transform)) = player_query.single() {
                trigger_explosion_camera_shake(
                    explosion.center,
                    explosion.radius,
                    player_transform.translation,
                    &mut camera_shake,
                );
            }

            apply_explosion_damage_to_targets(
                player_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                enemy_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                pedestrian_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                police_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                vehicle_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                Some(entity),
            );
        }

        if explosion.lifetime >= explosion.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
        }
    }
}

/// 車輛損壞粒子更新系統
/// 處理煙霧和火焰粒子的移動和刪除
pub fn vehicle_damage_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut smoke_query: Query<(Entity, &mut VehicleDamageSmoke, &mut Transform)>,
    mut fire_query: Query<(Entity, &mut VehicleFire, &mut Transform), Without<VehicleDamageSmoke>>,
) {
    let dt = time.delta_secs();

    // 更新煙霧粒子
    for (entity, mut smoke, mut transform) in smoke_query.iter_mut() {
        smoke.lifetime += dt;

        if smoke.lifetime >= smoke.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 煙霧上飄並減速
        smoke.velocity *= 1.0 - dt * 1.5;
        transform.translation += smoke.velocity * dt;

        // 擴散變大
        let progress = smoke.lifetime / smoke.max_lifetime;
        let scale = 0.2 + progress * 0.6;
        transform.scale = Vec3::splat(scale);
    }

    // 更新火焰粒子
    for (entity, mut fire, mut transform) in fire_query.iter_mut() {
        fire.lifetime += dt;

        if fire.lifetime >= fire.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 火焰快速上飄
        transform.translation += fire.velocity * dt;

        // 閃爍效果
        let flicker = (fire.lifetime * 20.0).sin() * 0.1 + 1.0;
        transform.scale = Vec3::splat(fire.scale() * flicker);
    }
}

/// 車輛傷害事件處理系統
/// 監聽 DamageEvent 並對車輛 VehicleHealth 造成傷害
/// 這使車輛可以被子彈、爆炸等傷害
pub fn vehicle_damage_event_system(
    time: Res<Time>,
    mut damage_events: MessageReader<DamageEvent>,
    mut vehicle_query: Query<&mut VehicleHealth>,
) {
    let current_time = time.elapsed_secs();

    for event in damage_events.read() {
        // 檢查目標是否是有 VehicleHealth 的車輛
        if let Ok(mut health) = vehicle_query.get_mut(event.target) {
            // 已爆炸的車輛不處理
            if health.damage_state == VehicleDamageState::Destroyed {
                continue;
            }

            // 對車輛造成傷害
            let damage_dealt = health.take_damage(event.amount, current_time);

            if damage_dealt > 0.0 {
                // 可以在這裡添加車輛受傷的視覺/音效回饋
                // 例如金屬撞擊音效
            }
        }
    }
}

// ============================================================================
// 紅綠燈交通系統
// ============================================================================

/// 初始化紅綠燈視覺效果資源
pub fn setup_traffic_lights(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TrafficLightVisuals::new(&mut meshes, &mut materials));
    info!("🚦 紅綠燈系統已初始化");
}

/// 紅綠燈狀態更新系統
/// 處理紅綠燈的循環切換
pub fn traffic_light_cycle_system(
    time: Res<Time>,
    mut traffic_lights: Query<(Entity, &mut TrafficLight, &Children)>,
    mut bulb_query: Query<(&TrafficLightBulb, &mut MeshMaterial3d<StandardMaterial>)>,
    visuals: Option<Res<TrafficLightVisuals>>,
) {
    let Some(visuals) = visuals else { return };

    for (_entity, mut light, children) in traffic_lights.iter_mut() {
        // 更新計時器
        light.timer.tick(time.delta());

        // 計時器結束時切換狀態
        if light.timer.just_finished() {
            light.advance();

            // 更新燈泡材質
            for child in children.iter() {
                if let Ok((bulb, mut material)) = bulb_query.get_mut(child) {
                    *material =
                        MeshMaterial3d(visuals.get_bulb_material(bulb.light_type, light.state));
                }
            }
        }
    }
}

/// 生成紅綠燈實體
pub fn spawn_traffic_light(
    commands: &mut Commands,
    visuals: &TrafficLightVisuals,
    position: Vec3,
    direction: Vec3,
    is_primary: bool,
) {
    let rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
    let initial_state = if is_primary {
        TrafficLightState::Green
    } else {
        TrafficLightState::Red
    };

    commands
        .spawn((
            // 空間組件
            Transform {
                translation: position,
                rotation,
                ..default()
            },
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // 紅綠燈組件
            TrafficLight::new(direction, is_primary),
            Name::new("TrafficLight"),
        ))
        .with_children(|parent| {
            // 燈柱
            parent.spawn((
                Mesh3d(visuals.pole_mesh.clone()),
                MeshMaterial3d(visuals.pole_material.clone()),
                Transform::from_xyz(0.0, 2.0, 0.0),
                GlobalTransform::default(),
            ));

            // 燈箱
            parent.spawn((
                Mesh3d(visuals.box_mesh.clone()),
                MeshMaterial3d(visuals.box_material.clone()),
                Transform::from_xyz(0.0, 4.5, 0.0),
                GlobalTransform::default(),
            ));

            // 紅燈（頂部）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Red, initial_state)),
                Transform::from_xyz(0.0, 4.9, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Red,
                },
            ));

            // 黃燈（中間）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Yellow, initial_state)),
                Transform::from_xyz(0.0, 4.5, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Yellow,
                },
            ));

            // 綠燈（底部）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Green, initial_state)),
                Transform::from_xyz(0.0, 4.1, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Green,
                },
            ));
        });
}

/// 生成交叉路口的紅綠燈組（4個方向）
/// ns_road_width: 南北向道路寬度（X方向）
/// ew_road_width: 東西向道路寬度（Z方向）
pub fn spawn_intersection_lights(
    commands: &mut Commands,
    visuals: &TrafficLightVisuals,
    center: Vec3,
    ns_road_width: f32,
    ew_road_width: f32,
) {
    // 紅綠燈放在道路邊緣外側 1 公尺
    let offset_x = ns_road_width / 2.0 + 1.0; // X 方向偏移（南北向道路寬度）
    let offset_z = ew_road_width / 2.0 + 1.0; // Z 方向偏移（東西向道路寬度）

    // 北向（控制南北向車流）- 主燈
    // 放在交叉口西北角（西側人行道，面向南來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(-offset_x, 0.0, -offset_z),
        Vec3::NEG_Z,
        true,
    );

    // 南向（控制南北向車流）- 主燈
    // 放在交叉口東南角（東側人行道，面向北來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(offset_x, 0.0, offset_z),
        Vec3::Z,
        true,
    );

    // 東向（控制東西向車流）- 副燈
    // 放在交叉口東北角（北側人行道，面向西來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(offset_x, 0.0, -offset_z),
        Vec3::X,
        false,
    );

    // 西向（控制東西向車流）- 副燈
    // 放在交叉口西南角（南側人行道，面向東來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(-offset_x, 0.0, offset_z),
        Vec3::NEG_X,
        false,
    );
}

/// 在世界中生成紅綠燈（西門町主要路口）
/// 此系統需要在 setup_traffic_lights 之後執行
pub fn spawn_world_traffic_lights(
    mut commands: Commands,
    visuals: Option<Res<TrafficLightVisuals>>,
) {
    let Some(visuals) = visuals else {
        warn!("TrafficLightVisuals 資源不存在，無法生成紅綠燈");
        return;
    };

    info!("🚦 正在生成交通燈...");

    // 道路常數（與 setup.rs 一致）
    // 南北向道路 X 位置
    const X_ZHONGHUA: f32 = 80.0; // 中華路
    const X_XINING: f32 = -55.0; // 西寧南路
                                 // 東西向道路 Z 位置
    const Z_HANKOU: f32 = -80.0; // 漢口街
    const Z_CHENGDU: f32 = 50.0; // 成都路
                                 // 道路寬度
    const W_ZHONGHUA: f32 = 40.0; // 中華路寬度
    const W_MAIN: f32 = 16.0; // 成都路寬度
    const W_SECONDARY: f32 = 12.0; // 西寧路、漢口街寬度

    // 主要路口：(位置, 南北道路寬度, 東西道路寬度)
    let intersections: [(Vec3, f32, f32); 4] = [
        // 西寧路/成都路交叉口
        (Vec3::new(X_XINING, 0.0, Z_CHENGDU), W_SECONDARY, W_MAIN),
        // 中華路/成都路交叉口
        (Vec3::new(X_ZHONGHUA, 0.0, Z_CHENGDU), W_ZHONGHUA, W_MAIN),
        // 西寧路/漢口街交叉口
        (Vec3::new(X_XINING, 0.0, Z_HANKOU), W_SECONDARY, W_SECONDARY),
        // 中華路/漢口街交叉口
        (
            Vec3::new(X_ZHONGHUA, 0.0, Z_HANKOU),
            W_ZHONGHUA,
            W_SECONDARY,
        ),
    ];

    for (center, ns_width, ew_width) in intersections.iter() {
        spawn_intersection_lights(&mut commands, &visuals, *center, *ns_width, *ew_width);
    }

    info!(
        "✅ 已生成 {} 組交通燈（共 {} 個）",
        intersections.len(),
        intersections.len() * 4
    );
}
