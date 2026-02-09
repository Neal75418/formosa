//! 載具系統


use super::{
    NitroBoost, NpcState, NpcVehicle, TrafficLight, TrafficLightState, Vehicle, VehicleBodyDynamics,
    VehicleBraking, VehicleConfig, VehicleDrift, VehicleInput, VehicleLean, VehicleModifications,
    VehiclePhysicsMode, VehiclePowerBand, VehicleSteering, VehicleType, VehicleVisualRoot,
};
use crate::core::math::rapier_real_to_f32;
use crate::core::{
    GameState, WeatherState, WeatherType, COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC,
    COLLISION_GROUP_VEHICLE,
};
use bevy::prelude::*;

use bevy_rapier3d::prelude::{Real as RapierReal, *};

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
        raw_input * 0.9 // 快速歸零
    } else {
        raw_input
    };

    let dt = time.delta_secs();
    // 使用 Steering Response 平滑輸入
    let steer_response = steering.steering_response * dt * 5.0;
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
    let weather_traction = get_weather_traction_factor(&weather, &config.weather);
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
fn apply_vehicle_motion_physics(
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
        handle_braking(vehicle, power_band, braking, input, dt, modifiers, effective_traction);
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
    let accel_force = calculate_acceleration_force(vehicle, power_band) * modifiers.accel;
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
            calculate_acceleration_force(vehicle, power_band) * modifiers.accel * 0.5
                * effective_traction;
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
    let weather_handling = get_weather_handling_factor(&weather, &config.weather);
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
    let speed_turn_factor = if speed_ratio < 0.3 {
        1.0
    } else {
        let high_speed_falloff = (speed_ratio - 0.3) / 0.7;
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

    body.body_roll = body.body_roll.clamp(-0.2, 0.2);
    body.body_pitch = body.body_pitch.clamp(-0.15, 0.15);
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
        WeatherType::Clear => config.clear_traction,
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
fn calculate_acceleration_force(vehicle: &Vehicle, power_band: &VehiclePowerBand) -> f32 {
    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);

    let torque_multiplier = if speed_ratio < 0.3 {
        // 低速區：強扭力（起步快）
        power_band.power_band_low * (1.0 - speed_ratio * 0.5)
    } else if speed_ratio < 0.7 {
        // 中速區：峰值扭力
        let t = (speed_ratio - 0.3) / 0.4;
        power_band.power_band_peak * (1.0 + 0.2 * (1.0 - (t - 0.5).abs() * 2.0))
    } else {
        // 高速區：扭力衰減
        let falloff = (speed_ratio - 0.7) / 0.3;
        power_band.top_end_falloff * (1.0 - falloff * 0.5)
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
                if current_speed < 1.0 {
                    if stuck_timer > 2.0 {
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
    let steer = cross * 2.0;
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
) {
    input.throttle_input = 0.0;
    input.brake_input = 1.0; // 倒車
                               // 倒車時反向打輪
    input.steer_input = -input.steer_input;

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
                let to_light_dir = to_light.normalize();
                // 燈必須在車輛前方（dot > 0.3 ≈ ±72°）
                if to_light_dir.dot(vehicle_forward) > 0.3 {
                    let light_forward = light_transform.forward().as_vec3();
                    // 燈面向車輛（dot < -0.5）
                    if light_forward.dot(vehicle_forward) < -0.5 {
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
                if should_stop_for_traffic_light(vehicle_pos, vehicle_forward, &traffic_light_query)
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
                handle_reversing_state(&mut npc, &mut transform, &mut input, dt)
            }
            NpcState::WaitingAtLight => handle_waiting_at_light_state(
                &mut npc,
                &mut input,
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
    input: &mut VehicleInput,
    transform: &Transform,
    traffic_light_query: &Query<(&Transform, &TrafficLight), Without<NpcVehicle>>,
    dt: f32,
) {
    // 重置輸入：鬆油門 + 踩煞車（由 npc_vehicle_motion_system 的煞車物理處理減速）
    input.throttle_input = 0.0;
    input.brake_input = 1.0;

    npc.stuck_timer += dt;

    // 檢查燈是否變綠了或等太久（30 秒 failsafe），恢復巡航
    let vehicle_pos = transform.translation;
    let vehicle_forward = transform.forward().as_vec3();
    if !should_stop_for_traffic_light(vehicle_pos, vehicle_forward, traffic_light_query)
        || npc.stuck_timer > 30.0
    {
        npc.state = NpcState::Cruising;
        npc.stuck_timer = 0.0;
    }
}

/// NPC 車輛運動整合（使用 NPC 輸入更新速度與位置）
#[allow(clippy::type_complexity)]
pub fn npc_vehicle_motion_system(
    time: Res<Time>,
    weather: Res<WeatherState>,
    config: Res<VehicleConfig>,
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
        ),
        With<NpcVehicle>,
    >,
) {
    let dt = time.delta_secs();

    for (mut transform, mut vehicle, power_band, braking, steering, drift, mut input, mods) in
        npc_query.iter_mut()
    {
        let modifiers = VehicleDynamicsModifiers::new(mods, None);

        let weather_traction = get_weather_traction_factor(&weather, &config.weather);
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

        // 邊界夾持：防止 NPC 車輛駛出地圖（略小於邊界牆位置）
        transform.translation.x = transform.translation.x.clamp(-119.0, 109.0);
        transform.translation.z = transform.translation.z.clamp(-94.0, 64.0);
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
    let weather_handling = get_weather_handling_factor(weather, &config.weather);
    let handling_mod = mods.map_or(1.0, |m| m.suspension.multiplier());
    let effective_handling = weather_handling * handling_mod;

    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);
    let speed_turn_factor = if speed_ratio < 0.3 {
        1.0
    } else {
        let high_speed_falloff = (speed_ratio - 0.3) / 0.7;
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
