//! 車輛設定常數與參數

use bevy::prelude::*;

/// 車輛系統全域配置
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[derive(Default)]
pub struct VehicleConfig {
    pub weather: VehicleWeatherConfig,
    pub physics: VehiclePhysicsConfig,
    pub input: VehicleInputConfig,
    pub npc: NpcDrivingConfig,
}

/// 天氣效果參數組（牽引力或操控力共用結構）
#[derive(Debug, Clone)]
pub struct WeatherFactorParams {
    pub clear: f32,
    pub cloudy: f32,
    pub rainy_base: f32,
    pub rainy_range: f32,
    pub foggy: f32,
    pub stormy_base: f32,
    pub stormy_range: f32,
    pub sandstorm_base: f32,
    pub sandstorm_range: f32,
}

/// 天氣影響配置
#[derive(Debug, Clone, Reflect)]
pub struct VehicleWeatherConfig {
    // === 牽引力 ===
    /// 晴天牽引力乘數
    pub clear_traction: f32,
    /// 陰天牽引力乘數
    pub cloudy_traction: f32,
    /// 雨天基礎牽引力乘數（最滑）
    pub rainy_traction_base: f32,
    /// 雨天牽引力恢復範圍
    pub rainy_traction_range: f32,
    /// 霧天牽引力乘數
    pub foggy_traction: f32,
    /// 暴風雨牽引力恢復範圍
    pub stormy_traction_range: f32,
    /// 沙塵暴牽引力恢復範圍
    pub sandstorm_traction_range: f32,

    // === 操控力 ===
    /// 晴天操控乘數
    pub clear_handling: f32,
    /// 陰天操控乘數
    pub cloudy_handling: f32,
    /// 雨天基礎操控乘數
    pub rainy_handling_base: f32,
    /// 雨天操控恢復範圍
    pub rainy_handling_range: f32,
    /// 霧天操控乘數
    pub foggy_handling: f32,
    /// 暴風雨操控恢復範圍
    pub stormy_handling_range: f32,
    /// 沙塵暴操控恢復範圍
    pub sandstorm_handling_range: f32,

    // === 基礎值（暴風雨/沙塵暴共用） ===
    /// 暴風雨基礎牽引力
    pub stormy_traction_base: f32,
    /// 暴風雨基礎操控乘數
    pub stormy_handling_base: f32,
    /// 沙塵暴基礎牽引力
    pub sandstorm_traction_base: f32,
    /// 沙塵暴基礎操控乘數
    pub sandstorm_handling_base: f32,
}

impl VehicleWeatherConfig {
    /// 取得牽引力參數組
    pub fn traction_params(&self) -> WeatherFactorParams {
        WeatherFactorParams {
            clear: self.clear_traction,
            cloudy: self.cloudy_traction,
            rainy_base: self.rainy_traction_base,
            rainy_range: self.rainy_traction_range,
            foggy: self.foggy_traction,
            stormy_base: self.stormy_traction_base,
            stormy_range: self.stormy_traction_range,
            sandstorm_base: self.sandstorm_traction_base,
            sandstorm_range: self.sandstorm_traction_range,
        }
    }

    /// 取得操控力參數組
    pub fn handling_params(&self) -> WeatherFactorParams {
        WeatherFactorParams {
            clear: self.clear_handling,
            cloudy: self.cloudy_handling,
            rainy_base: self.rainy_handling_base,
            rainy_range: self.rainy_handling_range,
            foggy: self.foggy_handling,
            stormy_base: self.stormy_handling_base,
            stormy_range: self.stormy_handling_range,
            sandstorm_base: self.sandstorm_handling_base,
            sandstorm_range: self.sandstorm_handling_range,
        }
    }
}

impl Default for VehicleWeatherConfig {
    fn default() -> Self {
        Self {
            // 牽引力
            clear_traction: 1.0,
            cloudy_traction: 1.0,
            rainy_traction_base: 0.7,
            rainy_traction_range: 0.3,
            foggy_traction: 0.9,
            stormy_traction_range: 0.15,
            sandstorm_traction_range: 0.1,
            // 操控力
            clear_handling: 1.0,
            cloudy_handling: 1.0,
            rainy_handling_base: 0.85,
            rainy_handling_range: 0.15,
            foggy_handling: 0.95,
            stormy_handling_range: 0.2,
            sandstorm_handling_range: 0.1,
            // 暴風雨/沙塵暴基礎值
            stormy_traction_base: 0.55,
            stormy_handling_base: 0.5,
            sandstorm_traction_base: 0.75,
            sandstorm_handling_base: 0.8,
        }
    }
}

/// 物理參數配置
#[derive(Debug, Clone, Reflect)]
pub struct VehiclePhysicsConfig {
    /// 加速模式乘數
    pub boost_multiplier: f32,
    /// 正常牽引力閾值
    pub normal_traction_threshold: f32,
    /// 低牽引力閾值
    pub low_traction_threshold: f32,
    /// 正常打滑速度閾值
    pub slip_speed_normal: f32,
    /// 低牽引力打滑速度閾值
    pub slip_speed_low_traction: f32,
    /// 正常打滑因子
    pub slip_factor_normal: f32,
    /// 低牽引力打滑因子
    pub slip_factor_low_traction: f32,
    /// 打滑對抓地力的影響
    pub slip_grip_penalty: f32,
    /// 打滑恢復速率
    pub slip_recovery_rate: f32,
    /// 倒車加速乘數
    pub reverse_acceleration_multiplier: f32,
    /// 手煞車減速係數
    pub handbrake_decel_coefficient: f32,
    /// 正常漂移速度閾值
    pub drift_speed_threshold_normal: f32,
    /// 低牽引力漂移速度閾值
    pub drift_speed_threshold_low_traction: f32,
    /// 正常漂移轉向閾值
    pub drift_steer_threshold_normal: f32,
    /// 低牽引力漂移轉向閾值
    pub drift_steer_threshold_low_traction: f32,
    /// 最大倒車速度比例
    pub reverse_speed_ratio: f32,
    /// 停止速度閾值
    pub stop_speed_threshold: f32,
    /// 低速轉向衰減閾值
    pub low_speed_turn_threshold: f32,
    /// 低速轉向衰減因子
    pub low_speed_turn_decay: f32,

    // === 漂移相關 ===
    /// 低牽引力漂移放大係數
    pub drift_amplifier_low_traction: f32,
    /// 正常漂移放大係數
    pub drift_amplifier_normal: f32,
    /// 漂移角度調整速率
    pub drift_angle_rate: f32,
    /// 最大漂移角度
    pub max_drift_angle: f32,
    /// 漂移反制力速率
    pub drift_counter_force_rate: f32,
    /// 反打方向盤救車速率
    pub counter_steer_rate: f32,
    /// 漂移結束角度閾值
    pub drift_end_angle_threshold: f32,
    /// 正常漂移結束速度閾值
    pub drift_end_speed_normal: f32,
    /// 低牽引力漂移結束速度閾值
    pub drift_end_speed_low_traction: f32,
    /// 漂移速度損失係數
    pub drift_speed_loss_rate: f32,
    /// 非漂移側滑角度衰減速率
    pub drift_decay_rate: f32,
    /// 側滑角度歸零閾值
    pub drift_angle_zero_threshold: f32,

    // === 轉向/摩擦（從 systems.rs 提取的 magic numbers）===
    /// 轉向平滑乘數
    pub steering_smoothing: f32,
    /// 靜止時轉向輸入衰減
    pub steering_stationary_decay: f32,
    /// 摩擦阻力係數
    pub friction_drag_coefficient: f32,
    /// 基礎減速率
    pub friction_base_decel: f32,
    /// 扭力曲線低速區閾值（`speed_ratio`）
    pub torque_low_speed_ratio: f32,
    /// 扭力曲線中速區閾值（`speed_ratio`）
    pub torque_mid_speed_ratio: f32,
    /// 車身側傾角度限制
    pub roll_angle_limit: f32,
    /// 車身俯仰角度限制
    pub pitch_angle_limit: f32,
}

impl Default for VehiclePhysicsConfig {
    fn default() -> Self {
        Self {
            boost_multiplier: 1.3,
            normal_traction_threshold: 0.9,
            low_traction_threshold: 0.8,
            slip_speed_normal: 5.0,
            slip_speed_low_traction: 8.0,
            slip_factor_normal: 3.0,
            slip_factor_low_traction: 4.0,
            slip_grip_penalty: 0.4,
            slip_recovery_rate: 2.0,
            reverse_acceleration_multiplier: 0.5,
            handbrake_decel_coefficient: 0.03,
            drift_speed_threshold_normal: 8.0,
            drift_speed_threshold_low_traction: 6.0,
            drift_steer_threshold_normal: 0.3,
            drift_steer_threshold_low_traction: 0.2,
            reverse_speed_ratio: 0.3,
            stop_speed_threshold: 0.1,
            low_speed_turn_threshold: 0.5,
            low_speed_turn_decay: 0.9,

            drift_amplifier_low_traction: 1.3,
            drift_amplifier_normal: 1.0,
            drift_angle_rate: 2.5,
            max_drift_angle: 0.8,
            drift_counter_force_rate: 3.0,
            counter_steer_rate: 2.0,
            drift_end_angle_threshold: 0.1,
            drift_end_speed_normal: 5.0,
            drift_end_speed_low_traction: 4.0,
            drift_speed_loss_rate: 0.5,
            drift_decay_rate: 4.0,
            drift_angle_zero_threshold: 0.05,

            // 轉向/摩擦
            steering_smoothing: 5.0,
            steering_stationary_decay: 0.9,
            friction_drag_coefficient: 0.5,
            friction_base_decel: 0.025,
            torque_low_speed_ratio: 0.3,
            torque_mid_speed_ratio: 0.7,
            roll_angle_limit: 0.2,
            pitch_angle_limit: 0.15,
        }
    }
}

/// 輸入配置
#[derive(Debug, Clone, Reflect)]
pub struct VehicleInputConfig {
    /// 轉向輸入死區
    pub steer_input_deadzone: f32,
}

impl Default for VehicleInputConfig {
    fn default() -> Self {
        Self {
            steer_input_deadzone: 0.01,
        }
    }
}

/// NPC 駕駛行為配置
#[derive(Debug, Clone, Reflect)]
pub struct NpcDrivingConfig {
    /// 障礙物檢測高度
    pub obstacle_check_height: f32,
    /// 障礙物檢測最大距離
    pub obstacle_max_distance: f32,
    /// 側向障礙物檢測最大距離
    pub obstacle_side_max_distance: f32,
    /// 太近需要倒車的距離
    pub obstacle_too_close_distance: f32,
    /// 需要煞車的距離
    pub obstacle_brake_distance: f32,
    /// 側向煞車距離
    pub obstacle_side_brake_distance: f32,
    /// 航點到達距離
    pub waypoint_arrival_distance: f32,
    /// 航點到達距離平方 (computed)
    pub waypoint_arrival_distance_sq: f32,
    /// NPC 巡航速度比例
    pub cruising_speed_ratio: f32,

    // === 卡住/倒車偵測（從 systems.rs 提取的 magic numbers）===
    /// 卡住判定速度閾值
    pub stuck_speed_threshold: f32,
    /// 卡住計時器超時（秒）
    pub stuck_timeout: f32,
    /// 倒車超時（秒）
    pub reverse_timeout: f32,
    /// 紅綠燈偵測距離平方
    pub traffic_light_detect_dist_sq: f32,
    /// 紅綠燈正前方 dot 閾值
    pub traffic_light_forward_dot: f32,
    /// 紅綠燈面向車輛 dot 閾值
    pub traffic_light_facing_dot: f32,
    /// 紅綠燈等待超時（秒）
    pub traffic_light_wait_timeout: f32,
    /// 轉向 P 控制增益
    pub steering_p_gain: f32,
    /// 障礙物射線前方偏移
    pub ray_forward_offset: f32,
}

impl Default for NpcDrivingConfig {
    fn default() -> Self {
        Self {
            obstacle_check_height: 0.6,
            obstacle_max_distance: 8.0,
            obstacle_side_max_distance: 4.5,
            obstacle_too_close_distance: 4.0,
            obstacle_brake_distance: 8.0,
            obstacle_side_brake_distance: 2.5,
            waypoint_arrival_distance: 5.0,
            waypoint_arrival_distance_sq: 25.0,
            cruising_speed_ratio: 0.6,

            // 卡住/倒車偵測
            stuck_speed_threshold: 1.0,
            stuck_timeout: 2.0,
            reverse_timeout: 3.0,
            traffic_light_detect_dist_sq: 400.0,
            traffic_light_forward_dot: 0.3,
            traffic_light_facing_dot: -0.5,
            traffic_light_wait_timeout: 30.0,
            steering_p_gain: 2.0,
            ray_forward_offset: 2.0,
        }
    }
}
