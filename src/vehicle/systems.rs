//! 載具系統
#![allow(dead_code)]


use super::{
    NitroBoost, Vehicle, VehicleBodyDynamics,
    VehicleBraking, VehicleConfig, VehicleDrift, VehicleInput, VehicleLean, VehicleModifications,
    VehiclePhysicsMode, VehiclePowerBand, VehicleSteering, VehicleType, VehicleVisualRoot,
};
use crate::core::{GameState, WeatherState, WeatherType};
use bevy::prelude::*;

use bevy_rapier3d::prelude::*;

// ============================================================================
// 車輛系統
// ============================================================================

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

/// 載具輸入（手煞車/漂移觸發、加速、轉向）
pub fn vehicle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    game_state: Res<GameState>,
    mut vehicles: Query<(&Vehicle, &VehicleSteering, &mut VehicleDrift, &mut VehicleInput)>,
    config: Res<VehicleConfig>,
) {
    if !game_state.player_in_vehicle {
        return;
    }

    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((vehicle, steering, mut drift, mut input)) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    let both_mouse =
        mouse_button.pressed(MouseButton::Left) && mouse_button.pressed(MouseButton::Right);

    // Throttle
    input.throttle_input = if keyboard.pressed(KeyCode::KeyW) || both_mouse {
        1.0
    } else {
        0.0
    };

    // Brake
    input.brake_input = if keyboard.pressed(KeyCode::KeyS) {
        1.0
    } else {
        0.0
    };

    // Handbrake
    drift.is_handbraking = keyboard.pressed(KeyCode::Space);

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
        raw_input * config.physics.steering_stationary_decay
    } else {
        raw_input
    };

    let dt = time.delta_secs();
    // 使用 Steering Response 平滑輸入
    let steer_response = steering.steering_response * dt * config.physics.steering_smoothing;
    input.steer_input += (target_input - input.steer_input) * steer_response.min(1.0);

    // Deadzone
    if input.steer_input.abs() < config.input.steer_input_deadzone {
        input.steer_input = 0.0;
    }
}

/// 天氣系統（預留給未來更複雜的天氣物理計算）
pub fn vehicle_weather_system() {
    // 目前天氣狀態由各個系統直接讀取 WeatherState
}

/// 載具加速與煞車系統
#[allow(clippy::type_complexity)]
pub fn vehicle_acceleration_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    weather: Res<WeatherState>,
    config: Res<VehicleConfig>,
    mut vehicles: Query<(
        &mut Vehicle,
        &VehiclePowerBand,
        &VehicleBraking,
        &VehicleDrift,
        &mut VehicleInput,
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
    let Ok((mut vehicle, power_band, braking, drift, mut input, mods, nitro)) =
        vehicles.get_mut(vehicle_entity)
    else {
        return;
    };

    let dt = time.delta_secs();
    let modifiers = VehicleDynamicsModifiers::new(mods, nitro);

    // Weather Traction
    let weather_traction = get_weather_factor(&weather, &config.weather.traction_params());
    let effective_traction = weather_traction * modifiers.traction;
    let effective_max_speed = vehicle.max_speed * modifiers.speed;

    apply_vehicle_motion_physics(
        &mut vehicle,
        power_band,
        braking,
        drift,
        &mut input,
        dt,
        &config.physics,
        &modifiers,
        effective_traction,
        effective_max_speed,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_vehicle_motion_physics(
    vehicle: &mut Vehicle,
    power_band: &VehiclePowerBand,
    braking: &VehicleBraking,
    drift: &VehicleDrift,
    input: &mut VehicleInput,
    dt: f32,
    physics_config: &crate::vehicle::config::VehiclePhysicsConfig,
    modifiers: &VehicleDynamicsModifiers,
    effective_traction: f32,
    effective_max_speed: f32,
) {
    if input.throttle_input > 0.0 {
        handle_acceleration(
            vehicle,
            power_band,
            input,
            dt,
            physics_config,
            modifiers,
            effective_traction,
        );
    } else if input.brake_input > 0.0 && !drift.is_handbraking {
        handle_braking(vehicle, power_band, braking, input, dt, physics_config, modifiers, effective_traction);
    } else {
        handle_friction(
            vehicle,
            braking,
            drift,
            physics_config,
            effective_traction,
            effective_max_speed,
        );
    }

    // Clamp Speed
    vehicle.current_speed = vehicle.current_speed.clamp(
        -effective_max_speed * physics_config.reverse_speed_ratio,
        effective_max_speed,
    );
    if vehicle.current_speed.abs() < physics_config.stop_speed_threshold
        && input.throttle_input == 0.0
    {
        vehicle.current_speed = 0.0;
    }
}

pub(super) struct VehicleDynamicsModifiers {
    pub(super) accel: f32,
    pub(super) speed: f32,
    pub(super) brake: f32,
    pub(super) traction: f32,
    pub(super) nitro: f32,
}

impl VehicleDynamicsModifiers {
    pub(super) fn new(mods: Option<&VehicleModifications>, nitro: Option<&NitroBoost>) -> Self {
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

#[allow(clippy::too_many_arguments)]
fn handle_acceleration(
    vehicle: &mut Vehicle,
    power_band: &VehiclePowerBand,
    input: &mut VehicleInput,
    dt: f32,
    physics_config: &crate::vehicle::config::VehiclePhysicsConfig,
    modifiers: &VehicleDynamicsModifiers,
    effective_traction: f32,
) {
    let accel_mult = modifiers.nitro.max(1.0);
    let accel_force = calculate_acceleration_force(vehicle, power_band, physics_config) * modifiers.accel;
    let effective_accel = accel_force * accel_mult * input.throttle_input * effective_traction;

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
        input.wheel_spin = (input.wheel_spin + dt * slip_factor).min(1.0);
        let grip =
            effective_traction * (1.0 - input.wheel_spin * physics_config.slip_grip_penalty);
        vehicle.current_speed += effective_accel * grip * dt;
    } else {
        input.wheel_spin = (input.wheel_spin - dt * physics_config.slip_recovery_rate).max(0.0);
        vehicle.current_speed += effective_accel * dt;
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_braking(
    vehicle: &mut Vehicle,
    power_band: &VehiclePowerBand,
    braking: &VehicleBraking,
    input: &VehicleInput,
    dt: f32,
    physics_config: &crate::vehicle::config::VehiclePhysicsConfig,
    modifiers: &VehicleDynamicsModifiers,
    effective_traction: f32,
) {
    if vehicle.current_speed > 0.5 {
        let brake_decel =
            braking.brake_force * modifiers.brake * input.brake_input * effective_traction;
        vehicle.current_speed -= brake_decel * dt;
        vehicle.current_speed = vehicle.current_speed.max(0.0);
    } else {
        // Reverse
        let reverse_accel =
            calculate_acceleration_force(vehicle, power_band, physics_config) * modifiers.accel
                * physics_config.reverse_acceleration_multiplier * effective_traction;
        vehicle.current_speed -= reverse_accel * dt;
    }
}

fn handle_friction(
    vehicle: &mut Vehicle,
    braking: &VehicleBraking,
    drift: &VehicleDrift,
    physics_config: &crate::vehicle::config::VehiclePhysicsConfig,
    effective_traction: f32,
    effective_max_speed: f32,
) {
    if drift.is_handbraking {
        // Handbrake
        let handbrake_decel = braking.handbrake_force
            * physics_config.handbrake_decel_coefficient
            * effective_traction;
        vehicle.current_speed *= 1.0 - handbrake_decel;
    } else {
        // Natural Deceleration
        let drag = 1.0 + (vehicle.current_speed.abs() / effective_max_speed) * physics_config.friction_drag_coefficient;
        vehicle.current_speed *= 1.0 - physics_config.friction_base_decel * drag;
    }
}

/// 載具轉向與角速度系統
pub fn vehicle_steering_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    weather: Res<WeatherState>,
    config: Res<VehicleConfig>,
    mut vehicles: Query<(
        &Vehicle,
        &VehicleSteering,
        &VehicleDrift,
        &VehicleInput,
        &mut Velocity,
        Option<&VehicleModifications>,
    )>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((vehicle, steering, drift, input, mut velocity, mods)) =
        vehicles.get_mut(vehicle_entity)
    else {
        return;
    };

    if vehicle.current_speed.abs() <= 0.5 {
        // 靜止不轉向
        return;
    }

    let dt = time.delta_secs();

    // Weather Handling
    let weather_handling = get_weather_factor(&weather, &config.weather.handling_params());
    let handling_mod = if let Some(m) = mods {
        m.suspension.multiplier()
    } else {
        1.0
    };
    let effective_handling = weather_handling * handling_mod;

    // Turning Logic
    let speed_ratio = if vehicle.max_speed > 0.0 {
        (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0)
    } else {
        0.0
    };
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

    let steer_response = (steering.steering_response * dt).min(1.0);
    velocity.angvel.y += (yaw_rate - velocity.angvel.y) * steer_response;
}

/// 載具漂移系統
pub fn vehicle_drift_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    weather: Res<WeatherState>,
    config: Res<VehicleConfig>,
    mut vehicles: Query<(
        &mut Vehicle,
        &VehicleSteering,
        &mut VehicleDrift,
        &VehicleInput,
        Option<&VehicleModifications>,
    )>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((mut vehicle, steering, mut drift, input, mods)) =
        vehicles.get_mut(vehicle_entity)
    else {
        return;
    };

    let dt = time.delta_secs();
    let effective_traction = calculate_effective_traction(&weather, &config, mods);
    let params = DriftPhysicsParams::new(&config.physics, effective_traction);

    if drift.is_handbraking && vehicle.current_speed.abs() > params.speed_threshold {
        handle_drift_start(&mut drift, input, dt, &config.physics, params.amplifier);
    } else if drift.is_drifting {
        handle_active_drift(
            &mut vehicle,
            steering,
            &mut drift,
            input,
            dt,
            &config.physics,
            effective_traction,
            params.end_speed,
        );
    } else {
        handle_drift_decay(&mut drift, dt, &config.physics);
    }
}

fn calculate_effective_traction(
    weather: &WeatherState,
    config: &VehicleConfig,
    mods: Option<&VehicleModifications>,
) -> f32 {
    let traction_mod = mods.map_or(1.0, |m| m.tires.multiplier());
    let weather_traction = get_weather_factor(weather, &config.weather.traction_params());
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
    drift: &mut VehicleDrift,
    input: &VehicleInput,
    dt: f32,
    config: &crate::vehicle::config::VehiclePhysicsConfig,
    amplifier: f32,
) {
    drift.drift_angle += input.steer_input * dt * config.drift_angle_rate * amplifier;
    drift.drift_angle = drift
        .drift_angle
        .clamp(-config.max_drift_angle, config.max_drift_angle);
    drift.is_drifting = drift.drift_angle.abs() > drift.drift_threshold;
}

#[allow(clippy::too_many_arguments)]
fn handle_active_drift(
    vehicle: &mut Vehicle,
    steering: &VehicleSteering,
    drift: &mut VehicleDrift,
    input: &VehicleInput,
    dt: f32,
    config: &crate::vehicle::config::VehiclePhysicsConfig,
    traction: f32,
    end_speed: f32,
) {
    // Apply counter force
    let counter = -drift.drift_angle
        * (1.0 - drift.drift_grip * traction)
        * dt
        * config.drift_counter_force_rate;
    drift.drift_angle += counter;

    // Apply counter steer assist
    if input.steer_input != 0.0 && input.steer_input.signum() == -drift.drift_angle.signum() {
        drift.drift_angle +=
            input.steer_input * steering.counter_steer_assist * dt * config.counter_steer_rate;
    }

    // Check end condition
    if drift.drift_angle.abs() < config.drift_end_angle_threshold
        || vehicle.current_speed.abs() < end_speed
    {
        drift.is_drifting = false;
        drift.drift_angle = 0.0;
    } else {
        // Apply speed loss
        let drift_speed_loss = drift.drift_angle.abs()
            * (1.0 - drift.drift_grip)
            * traction
            * dt
            * config.drift_speed_loss_rate;
        vehicle.current_speed *= 1.0 - drift_speed_loss;
    }
}

fn handle_drift_decay(
    drift: &mut VehicleDrift,
    dt: f32,
    config: &crate::vehicle::config::VehiclePhysicsConfig,
) {
    drift.drift_angle *= 1.0 - dt * config.drift_decay_rate;
    if drift.drift_angle.abs() < config.drift_angle_zero_threshold {
        drift.drift_angle = 0.0;
    }
}

/// 載具懸吊與車身動態系統
pub fn vehicle_suspension_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    config: Res<VehicleConfig>,
    mut vehicles: Query<(
        &Vehicle,
        &mut VehicleLean,
        &mut VehicleBodyDynamics,
        &VehicleDrift,
        &VehicleInput,
    )>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((vehicle, mut lean, mut body, drift, input)) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    let dt = time.delta_secs();

    // Scooter lean
    if vehicle.vehicle_type == VehicleType::Scooter {
        let speed_factor = (vehicle.current_speed / vehicle.max_speed).clamp(0.0, 1.0);
        let target_lean = input.steer_input * lean.max_lean_angle * speed_factor;
        let lean_speed = 5.0;
        let lean_diff = target_lean - lean.lean_angle;
        lean.lean_angle += lean_diff * lean_speed * dt;
        lean.lean_angle = lean
            .lean_angle
            .clamp(-lean.max_lean_angle, lean.max_lean_angle);
        return;
    }

    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);

    // Roll
    let target_roll = -input.steer_input * body.body_roll_factor * speed_ratio.sqrt();
    let drift_roll_bonus = if drift.is_drifting {
        drift.drift_angle * 0.3
    } else {
        0.0
    };

    // Pitch
    let accel_state = input.throttle_input - input.brake_input;
    let target_pitch = -accel_state * body.body_pitch_factor * speed_ratio.sqrt().min(0.8);
    let handbrake_pitch = if drift.is_handbraking { 0.04 } else { 0.0 };

    // Suspension
    let suspension_speed = body.suspension_stiffness * dt;
    body.body_roll += ((target_roll + drift_roll_bonus) - body.body_roll) * suspension_speed;
    body.body_pitch +=
        ((target_pitch + handbrake_pitch) - body.body_pitch) * suspension_speed;

    body.body_roll = body.body_roll.clamp(-config.physics.roll_angle_limit, config.physics.roll_angle_limit);
    body.body_pitch = body.body_pitch.clamp(-config.physics.pitch_angle_limit, config.physics.pitch_angle_limit);
}

/// 載具物理整合系統（整合速度與位移）
pub fn vehicle_physics_integration_system(
    game_state: Res<GameState>,
    mut vehicles: Query<(&Transform, &Vehicle, &VehicleDrift, &mut Velocity)>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((transform, vehicle, drift, mut velocity)) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    let forward = transform.forward().as_vec3();

    let movement_dir = if drift.is_drifting && drift.drift_angle.abs() > 0.1 {
        let drift_offset = Quat::from_rotation_y(-drift.drift_angle * 0.3);
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
    vehicle_query: Query<(&Vehicle, &VehicleLean, &VehicleBodyDynamics, &VehicleDrift)>,
    mut visual_query: Query<(&ChildOf, &mut Transform), With<VehicleVisualRoot>>,
) {
    for (parent, mut transform) in &mut visual_query {
        let Ok((vehicle, lean, body, drift)) = vehicle_query.get(parent.parent()) else {
            continue;
        };

        let yaw_offset = if drift.is_drifting {
            drift.drift_angle * 0.25
        } else {
            0.0
        };

        transform.translation = Vec3::ZERO;
        if vehicle.vehicle_type == VehicleType::Scooter {
            transform.rotation =
                Quat::from_rotation_y(yaw_offset) * Quat::from_rotation_z(-lean.lean_angle);
        } else {
            transform.rotation = Quat::from_rotation_y(yaw_offset)
                * Quat::from_rotation_x(body.body_pitch)
                * Quat::from_rotation_z(body.body_roll);
        }
    }
}

// ============================================================================
// 天氣影響駕駛系統
// ============================================================================

/// 計算天氣對駕駛因子的影響（泛用版本，牽引力和操控力共用）
pub(super) fn get_weather_factor(
    weather: &WeatherState,
    params: &crate::vehicle::config::WeatherFactorParams,
) -> f32 {
    match weather.weather_type {
        WeatherType::Clear => params.clear,
        WeatherType::Cloudy => params.cloudy,
        WeatherType::Rainy => {
            params.rainy_base + (1.0 - weather.intensity) * params.rainy_range
        }
        WeatherType::Foggy => params.foggy,
        WeatherType::Stormy => params.stormy_base + (1.0 - weather.intensity) * params.stormy_range,
        WeatherType::Sandstorm => params.sandstorm_base + (1.0 - weather.intensity) * params.sandstorm_range,
    }
}

/// 計算非線性加速力（扭力曲線）
fn calculate_acceleration_force(
    vehicle: &Vehicle,
    power_band: &VehiclePowerBand,
    physics: &crate::vehicle::config::VehiclePhysicsConfig,
) -> f32 {
    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);
    let low = physics.torque_low_speed_ratio;
    let mid = physics.torque_mid_speed_ratio;

    let torque_multiplier = if speed_ratio < low {
        // 低速區：強扭力（起步快）
        power_band.power_band_low * (1.0 - speed_ratio * 0.5)
    } else if speed_ratio < mid {
        // 中速區：峰值扭力
        let t = (speed_ratio - low) / (mid - low);
        power_band.power_band_peak * (1.0 + 0.2 * (1.0 - (t - 0.5).abs() * 2.0))
    } else {
        // 高速區：扭力衰減
        let falloff = (speed_ratio - mid) / (1.0 - mid);
        power_band.top_end_falloff * (1.0 - falloff * 0.5)
    };

    vehicle.acceleration * torque_multiplier
}
