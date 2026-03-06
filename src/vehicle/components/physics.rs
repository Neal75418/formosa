//! 車輛物理相關組件（傾斜、煞車、轉向、漂移等）

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

// ============================================================================
// 車輛子元件（從 Vehicle 上帝元件拆分而來）
// ============================================================================

/// 機車傾斜系統元件
#[derive(Component)]
pub struct VehicleLean {
    /// 當前傾斜角度 (弧度)
    pub lean_angle: f32,
    /// 最大傾斜角度 (弧度)
    pub max_lean_angle: f32,
    /// 傾斜響應速度（越高越靈敏）
    pub lean_response: f32,
    /// 傾斜角速度（用於慣性計算）
    pub lean_velocity: f32,
    /// 傾斜阻尼（防止震盪）
    pub lean_damping: f32,
    /// 摔車臨界角度（超過此角度觸發摔車）
    pub crash_lean_threshold: f32,
    /// 是否正在摔車
    pub is_crashed: bool,
    /// 摔車恢復冷卻時間（秒）
    pub crash_recovery_timer: f32,
    /// 摔車恢復所需時間（秒）
    pub crash_recovery_duration: f32,
}

impl Default for VehicleLean {
    fn default() -> Self {
        Self {
            lean_angle: 0.0,
            max_lean_angle: 0.0,
            lean_response: 5.0,
            lean_velocity: 0.0,
            lean_damping: 8.0,
            crash_lean_threshold: 0.70,
            is_crashed: false,
            crash_recovery_timer: 0.0,
            crash_recovery_duration: 2.0,
        }
    }
}

/// 加速系統（非線性扭力曲線）元件
#[derive(Component)]
pub struct VehiclePowerBand {
    /// 低速扭力倍率 (0~30% 速度)
    pub power_band_low: f32,
    /// 峰值扭力倍率 (30~70% 速度)
    pub power_band_peak: f32,
    /// 高速衰減倍率 (70~100% 速度)
    pub top_end_falloff: f32,
}

impl Default for VehiclePowerBand {
    fn default() -> Self {
        Self {
            power_band_low: 1.0,
            power_band_peak: 1.0,
            top_end_falloff: 0.5,
        }
    }
}

/// 煞車系統元件
#[derive(Component)]
pub struct VehicleBraking {
    /// 一般煞車力道
    pub brake_force: f32,
    /// 手煞車力道（漂移用）
    pub handbrake_force: f32,
}

impl Default for VehicleBraking {
    fn default() -> Self {
        Self {
            brake_force: 20.0,
            handbrake_force: 30.0,
        }
    }
}

/// 轉向/操控系統元件
#[derive(Component)]
pub struct VehicleSteering {
    /// 操控靈敏度
    pub handling: f32,
    /// 高速轉向衰減 (0.0~1.0)
    pub high_speed_turn_factor: f32,
    /// 轉向響應速度
    pub steering_response: f32,
    /// 反打救車輔助
    pub counter_steer_assist: f32,
}

impl Default for VehicleSteering {
    fn default() -> Self {
        Self {
            handling: 1.0,
            high_speed_turn_factor: 0.3,
            steering_response: 5.0,
            counter_steer_assist: 0.4,
        }
    }
}

/// 漂移系統元件
#[derive(Component)]
pub struct VehicleDrift {
    /// 漂移觸發角度
    pub drift_threshold: f32,
    /// 漂移中的抓地力
    pub drift_grip: f32,
    /// 漂移狀態
    pub is_drifting: bool,
    /// 當前漂移角度
    pub drift_angle: f32,
    /// 手煞車狀態
    pub is_handbraking: bool,
}

impl Default for VehicleDrift {
    fn default() -> Self {
        Self {
            drift_threshold: 0.4,
            drift_grip: 0.5,
            is_drifting: false,
            drift_angle: 0.0,
            is_handbraking: false,
        }
    }
}

/// 車身動態（汽車/公車用）元件
#[derive(Component)]
pub struct VehicleBodyDynamics {
    /// 車身側傾係數
    pub body_roll_factor: f32,
    /// 車身前後傾係數
    pub body_pitch_factor: f32,
    /// 當前側傾角
    pub body_roll: f32,
    /// 當前前後傾角
    pub body_pitch: f32,
    /// 懸吊硬度（影響傾斜恢復速度）
    pub suspension_stiffness: f32,
}

impl Default for VehicleBodyDynamics {
    fn default() -> Self {
        Self {
            body_roll_factor: 0.05,
            body_pitch_factor: 0.05,
            body_roll: 0.0,
            body_pitch: 0.0,
            suspension_stiffness: 4.0,
        }
    }
}

/// 車輛輸入狀態元件
#[derive(Component)]
pub struct VehicleInput {
    /// 油門輸入 (0.0~1.0)
    pub throttle_input: f32,
    /// 煞車輸入 (0.0~1.0)
    pub brake_input: f32,
    /// 轉向輸入 (-1.0~1.0)
    pub steer_input: f32,
    /// 輪胎打滑程度 (0.0~1.0)
    pub wheel_spin: f32,
}

impl Default for VehicleInput {
    fn default() -> Self {
        Self {
            throttle_input: 0.0,
            brake_input: 0.0,
            steer_input: 0.0,
            wheel_spin: 0.0,
        }
    }
}
