//! 載具系統


use super::{
    Vehicle, VehicleBodyDynamics, VehicleConfig, VehicleDrift, VehicleHealth,
    VehicleInput, VehicleLean, VehiclePhysicsMode, VehicleSteering, VehicleType,
    VehicleVisualRoot,
};
use crate::core::GameState;
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
// 機車摔車系統
// ============================================================================

/// 摔車傷害常數
const MOTORCYCLE_CRASH_DAMAGE: f32 = 150.0;
/// 摔車速度損失比例
const MOTORCYCLE_CRASH_SPEED_LOSS: f32 = 0.8;
/// 觸發摔車的最低速度（m/s）
const MOTORCYCLE_CRASH_MIN_SPEED: f32 = 8.0;

/// 機車摔車偵測與恢復系統
///
/// 當機車傾斜角度超過 crash_lean_threshold 且車速夠快時觸發摔車：
/// - 大幅降速
/// - 車輛受傷
/// - 進入恢復冷卻期
pub fn motorcycle_crash_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    mut vehicles: Query<(
        &mut Vehicle,
        &mut VehicleLean,
        Option<&mut VehicleHealth>,
    )>,
) {
    if !game_state.player_in_vehicle {
        return;
    }
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((mut vehicle, mut lean, health)) = vehicles.get_mut(vehicle_entity) else {
        return;
    };

    if vehicle.vehicle_type != VehicleType::Scooter {
        return;
    }

    let dt = time.delta_secs();

    // 摔車恢復中
    if lean.is_crashed {
        lean.crash_recovery_timer -= dt;
        if lean.crash_recovery_timer <= 0.0 {
            lean.is_crashed = false;
            lean.crash_recovery_timer = 0.0;
            lean.lean_angle = 0.0;
            lean.lean_velocity = 0.0;
        }
        return;
    }

    // 檢測摔車條件：傾斜超過臨界角度 + 速度夠快
    if lean.lean_angle.abs() >= lean.crash_lean_threshold
        && vehicle.current_speed.abs() >= MOTORCYCLE_CRASH_MIN_SPEED
    {
        lean.is_crashed = true;
        lean.crash_recovery_timer = lean.crash_recovery_duration;
        lean.lean_velocity = 0.0;

        // 大幅降速
        vehicle.current_speed *= 1.0 - MOTORCYCLE_CRASH_SPEED_LOSS;

        // 對車輛造成傷害（使用 take_damage 以正確處理無敵/已毀狀態）
        if let Some(mut hp) = health {
            hp.take_damage(MOTORCYCLE_CRASH_DAMAGE, time.elapsed_secs());
        }
    }
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle::VehiclePreset;

    // --- VehicleLean 測試 ---

    #[test]
    fn test_lean_default() {
        let lean = VehicleLean::default();
        assert_eq!(lean.lean_angle, 0.0);
        assert_eq!(lean.lean_velocity, 0.0);
        assert!(!lean.is_crashed);
    }

    #[test]
    fn test_scooter_preset_max_lean_35_degrees() {
        let preset = VehiclePreset::scooter();
        // 35° ≈ 0.611 rad
        assert!((preset.lean.max_lean_angle - 0.611).abs() < 0.01);
    }

    #[test]
    fn test_scooter_preset_crash_threshold() {
        let preset = VehiclePreset::scooter();
        // 摔車角度 (~40°) 應大於最大傾斜角度 (35°)
        assert!(preset.lean.crash_lean_threshold > preset.lean.max_lean_angle);
    }

    #[test]
    fn test_crash_constants() {
        // 摔車傷害合理（機車 500 HP，摔一次 150）
        assert!(MOTORCYCLE_CRASH_DAMAGE > 0.0);
        assert!(MOTORCYCLE_CRASH_DAMAGE < 500.0);
        // 速度損失 80%
        assert!(MOTORCYCLE_CRASH_SPEED_LOSS > 0.0);
        assert!(MOTORCYCLE_CRASH_SPEED_LOSS <= 1.0);
        // 最低觸發速度
        assert!(MOTORCYCLE_CRASH_MIN_SPEED > 0.0);
    }

    #[test]
    fn test_lean_spring_damper_convergence() {
        // 模擬 spring-damper lean 計算：應該朝目標收斂
        let mut lean = VehicleLean {
            max_lean_angle: 0.611,
            lean_response: 6.0,
            lean_damping: 10.0,
            crash_lean_threshold: 0.70,
            ..Default::default()
        };

        let target = 0.4; // 目標傾斜角
        let dt = 1.0 / 60.0;

        // 模擬 300 幀（5 秒，overdamped 系統需要較長收斂時間）
        for _ in 0..300 {
            let spring_force = lean.lean_response * (target - lean.lean_angle);
            let damping_force = -lean.lean_damping * lean.lean_velocity;
            let accel = spring_force + damping_force;
            lean.lean_velocity += accel * dt;
            lean.lean_angle += lean.lean_velocity * dt;
        }

        // 5秒後應該收斂到目標附近
        assert!(
            (lean.lean_angle - target).abs() < 0.1,
            "lean_angle={} target={} diff={}",
            lean.lean_angle, target, (lean.lean_angle - target).abs()
        );
    }

    #[test]
    fn test_lean_spring_damper_no_overshoot_beyond_crash() {
        // 確認 spring-damper 不會大幅過衝超過 crash threshold
        let mut lean = VehicleLean {
            max_lean_angle: 0.611,
            lean_response: 6.0,
            lean_damping: 10.0,
            crash_lean_threshold: 0.70,
            ..Default::default()
        };

        let target = 0.611; // 全力傾斜
        let dt = 1.0 / 60.0;

        for _ in 0..120 {
            let spring_force = lean.lean_response * (target - lean.lean_angle);
            let damping_force = -lean.lean_damping * lean.lean_velocity;
            let accel = spring_force + damping_force;
            lean.lean_velocity += accel * dt;
            lean.lean_angle += lean.lean_velocity * dt;
            // 系統中有 clamp 到 crash_lean_threshold
            lean.lean_angle = lean.lean_angle.clamp(-lean.crash_lean_threshold, lean.crash_lean_threshold);
        }

        // 不應超過 crash threshold
        assert!(lean.lean_angle.abs() <= lean.crash_lean_threshold + 0.001);
    }

    #[test]
    fn test_crash_recovery_timer() {
        let mut lean = VehicleLean {
            is_crashed: true,
            crash_recovery_timer: 2.0,
            crash_recovery_duration: 2.0,
            ..Default::default()
        };

        // 模擬 1 秒
        lean.crash_recovery_timer -= 1.0;
        assert!(lean.is_crashed);
        assert!(lean.crash_recovery_timer > 0.0);

        // 模擬再 1.5 秒
        lean.crash_recovery_timer -= 1.5;
        assert!(lean.crash_recovery_timer <= 0.0);
        // 系統會在 timer <= 0 時重置 is_crashed
    }

    #[test]
    fn test_car_preset_no_lean() {
        let preset = VehiclePreset::car();
        assert_eq!(preset.lean.max_lean_angle, 0.0);
    }

    #[test]
    fn test_bus_preset_no_lean() {
        let preset = VehiclePreset::bus();
        assert_eq!(preset.lean.max_lean_angle, 0.0);
    }
}
