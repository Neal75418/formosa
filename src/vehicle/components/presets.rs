//! 車輛預設配置（工廠方法）

use super::core::{Vehicle, VehicleType};
use super::physics::*;

/// 車輛預設配置，用於生成車輛時一次設定所有子元件
pub struct VehiclePreset {
    pub vehicle: Vehicle,
    pub lean: VehicleLean,
    pub power_band: VehiclePowerBand,
    pub braking: VehicleBraking,
    pub steering: VehicleSteering,
    pub drift: VehicleDrift,
    pub body_dynamics: VehicleBodyDynamics,
    pub input: VehicleInput,
}

impl VehiclePreset {
    /// 轉換為元件 tuple（可直接用於 Bevy spawn）
    #[allow(clippy::type_complexity)]
    pub fn into_components(
        self,
    ) -> (
        Vehicle,
        VehicleLean,
        VehiclePowerBand,
        VehicleBraking,
        VehicleSteering,
        VehicleDrift,
        VehicleBodyDynamics,
        VehicleInput,
    ) {
        (
            self.vehicle,
            self.lean,
            self.power_band,
            self.braking,
            self.steering,
            self.drift,
            self.body_dynamics,
            self.input,
        )
    }

    /// 機車 - 台灣街頭最常見的交通工具
    /// 特色：靈活、加速快、可傾斜過彎、容易漂移
    pub fn scooter() -> Self {
        Self {
            vehicle: Vehicle {
                vehicle_type: VehicleType::Scooter,
                max_speed: 22.0,
                acceleration: 18.0,
                turn_speed: 4.0,
                ..Default::default()
            },
            lean: VehicleLean {
                max_lean_angle: 0.5,
                ..Default::default()
            },
            power_band: VehiclePowerBand {
                power_band_low: 1.3,
                power_band_peak: 1.0,
                top_end_falloff: 0.7,
            },
            braking: VehicleBraking {
                braking_power: 0.85,
                brake_force: 25.0,
                handbrake_force: 35.0,
            },
            steering: VehicleSteering {
                handling: 1.5,
                high_speed_turn_factor: 0.5,
                steering_response: 8.0,
                counter_steer_assist: 0.3,
            },
            drift: VehicleDrift {
                drift_threshold: 0.3,
                drift_grip: 0.6,
                ..Default::default()
            },
            body_dynamics: VehicleBodyDynamics {
                body_roll_factor: 0.0,
                body_pitch_factor: 0.15,
                suspension_stiffness: 5.0,
                ..Default::default()
            },
            input: VehicleInput::default(),
        }
    }

    /// 汽車 - 平衡型，GTA 風格漂移
    pub fn car() -> Self {
        Self {
            vehicle: Vehicle {
                vehicle_type: VehicleType::Car,
                max_speed: 35.0,
                acceleration: 12.0,
                turn_speed: 2.0,
                ..Default::default()
            },
            lean: VehicleLean::default(),
            power_band: VehiclePowerBand {
                power_band_peak: 1.2,
                ..Default::default()
            },
            braking: VehicleBraking {
                handbrake_force: 40.0,
                ..Default::default()
            },
            steering: VehicleSteering {
                counter_steer_assist: 0.5,
                ..Default::default()
            },
            drift: VehicleDrift::default(),
            body_dynamics: VehicleBodyDynamics {
                body_roll_factor: 0.08,
                body_pitch_factor: 0.06,
                ..Default::default()
            },
            input: VehicleInput::default(),
        }
    }

    /// 計程車 - 平衡型，略高操控性
    pub fn taxi() -> Self {
        Self {
            vehicle: Vehicle {
                vehicle_type: VehicleType::Taxi,
                acceleration: 11.0,
                turn_speed: 2.2,
                ..Default::default()
            },
            lean: VehicleLean::default(),
            power_band: VehiclePowerBand {
                power_band_low: 1.1,
                power_band_peak: 1.1,
                top_end_falloff: 0.6,
            },
            braking: VehicleBraking {
                braking_power: 0.75,
                brake_force: 22.0,
                handbrake_force: 38.0,
            },
            steering: VehicleSteering {
                handling: 1.1,
                high_speed_turn_factor: 0.35,
                steering_response: 5.5,
                counter_steer_assist: 0.45,
            },
            drift: VehicleDrift::default(),
            body_dynamics: VehicleBodyDynamics {
                body_roll_factor: 0.07,
                suspension_stiffness: 4.5,
                ..Default::default()
            },
            input: VehicleInput::default(),
        }
    }

    /// 公車 - 笨重、難漂移、誇張傾斜（有趣）
    pub fn bus() -> Self {
        Self {
            vehicle: Vehicle {
                vehicle_type: VehicleType::Bus,
                max_speed: 15.0,
                acceleration: 8.0,
                turn_speed: 1.8,
                ..Default::default()
            },
            lean: VehicleLean::default(),
            power_band: VehiclePowerBand {
                power_band_low: 1.5,
                power_band_peak: 0.8,
                top_end_falloff: 0.3,
            },
            braking: VehicleBraking {
                braking_power: 0.6,
                brake_force: 15.0,
                handbrake_force: 25.0,
            },
            steering: VehicleSteering {
                handling: 0.7,
                high_speed_turn_factor: 0.15,
                steering_response: 2.0,
                counter_steer_assist: 0.2,
            },
            drift: VehicleDrift {
                drift_threshold: 0.6,
                drift_grip: 0.3,
                ..Default::default()
            },
            body_dynamics: VehicleBodyDynamics {
                body_roll_factor: 0.15,
                body_pitch_factor: 0.10,
                suspension_stiffness: 2.0,
                ..Default::default()
            },
            input: VehicleInput::default(),
        }
    }
}
