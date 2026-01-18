//! 載具系統

use bevy::prelude::*;
use super::{Vehicle, VehicleType, NpcVehicle, NpcState, DriftSmoke, TireTrack, VehicleEffectVisuals, VehicleEffectTracker, TrafficLight, TrafficLightState, TrafficLightBulb, TrafficLightVisuals};
use rand::Rng;
use crate::core::{GameState, WeatherState, WeatherType, COLLISION_GROUP_CHARACTER, COLLISION_GROUP_VEHICLE, COLLISION_GROUP_STATIC};
use bevy_rapier3d::prelude::*;

// ============================================================================
// 車輛系統常數
// ============================================================================

// === 天氣影響因子 ===
/// 晴天牽引力乘數
const WEATHER_CLEAR_TRACTION: f32 = 1.0;
/// 陰天牽引力乘數
const WEATHER_CLOUDY_TRACTION: f32 = 1.0;
/// 雨天基礎牽引力乘數（最滑）
const WEATHER_RAINY_TRACTION_BASE: f32 = 0.7;
/// 雨天牽引力恢復範圍
const WEATHER_RAINY_TRACTION_RANGE: f32 = 0.3;
/// 霧天牽引力乘數
const WEATHER_FOGGY_TRACTION: f32 = 0.9;
/// 雨天基礎操控乘數
const WEATHER_RAINY_HANDLING_BASE: f32 = 0.85;
/// 雨天操控恢復範圍
const WEATHER_RAINY_HANDLING_RANGE: f32 = 0.15;
/// 霧天操控乘數
const WEATHER_FOGGY_HANDLING: f32 = 0.95;

// === 駕駛物理 ===
/// 加速模式乘數
const BOOST_MULTIPLIER: f32 = 1.3;
/// 正常牽引力閾值
const NORMAL_TRACTION_THRESHOLD: f32 = 0.9;
/// 低牽引力閾值
const LOW_TRACTION_THRESHOLD: f32 = 0.8;
/// 正常打滑速度閾值
const SLIP_SPEED_NORMAL: f32 = 5.0;
/// 低牽引力打滑速度閾值
const SLIP_SPEED_LOW_TRACTION: f32 = 8.0;
/// 正常打滑因子
const SLIP_FACTOR_NORMAL: f32 = 3.0;
/// 低牽引力打滑因子
const SLIP_FACTOR_LOW_TRACTION: f32 = 4.0;
/// 打滑對抓地力的影響
const SLIP_GRIP_PENALTY: f32 = 0.4;
/// 打滑恢復速率
const SLIP_RECOVERY_RATE: f32 = 2.0;
/// 倒車加速乘數
const REVERSE_ACCELERATION_MULTIPLIER: f32 = 0.5;
/// 手煞車減速係數
const HANDBRAKE_DECEL_COEFFICIENT: f32 = 0.03;
/// 正常漂移速度閾值
const DRIFT_SPEED_THRESHOLD_NORMAL: f32 = 8.0;
/// 低牽引力漂移速度閾值
const DRIFT_SPEED_THRESHOLD_LOW_TRACTION: f32 = 6.0;
/// 正常漂移轉向閾值
const DRIFT_STEER_THRESHOLD_NORMAL: f32 = 0.3;
/// 低牽引力漂移轉向閾值
const DRIFT_STEER_THRESHOLD_LOW_TRACTION: f32 = 0.2;
/// 最大倒車速度比例
const REVERSE_SPEED_RATIO: f32 = 0.3;
/// 停止速度閾值
const STOP_SPEED_THRESHOLD: f32 = 0.1;
/// 低速轉向衰減閾值
const LOW_SPEED_TURN_THRESHOLD: f32 = 0.5;
/// 低速轉向衰減因子
const LOW_SPEED_TURN_DECAY: f32 = 0.9;
/// 轉向輸入死區
const STEER_INPUT_DEADZONE: f32 = 0.01;

// === NPC 車輛 ===
/// 障礙物檢測高度
const NPC_OBSTACLE_CHECK_HEIGHT: f32 = 0.6;
/// 障礙物檢測最大距離
const NPC_OBSTACLE_MAX_DISTANCE: f32 = 8.0;
/// 航點到達距離
const NPC_WAYPOINT_ARRIVAL_DISTANCE: f32 = 8.0;
/// 航點到達距離平方 (8.0² = 64.0)
const NPC_WAYPOINT_ARRIVAL_DISTANCE_SQ: f32 = 64.0;
/// NPC 巡航速度比例
const NPC_CRUISING_SPEED_RATIO: f32 = 0.6;

/// 將 bevy_rapier3d 的 Real 類型轉換為 f32
/// 用於避免與 bevy::prelude::Real 的命名衝突
#[inline]
fn rapier_real_to_f32(r: bevy_rapier3d::prelude::Real) -> f32 {
    r
}

/// 載具輸入（手煞車/漂移觸發）
pub fn vehicle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    mut vehicles: Query<&mut Vehicle>,
) {
    if !game_state.player_in_vehicle {
        return;
    }

    let Some(vehicle_entity) = game_state.current_vehicle else { return; };
    let Ok(mut vehicle) = vehicles.get_mut(vehicle_entity) else { return; };

    // Space = 手煞車（漂移觸發器）
    vehicle.is_handbraking = keyboard.pressed(KeyCode::Space);
}

/// 載具移動（GTA 風格街機物理 + 天氣影響）
/// - W/S = 加速/減速
/// - A/D = 左右轉（速度敏感）
/// - Space = 手煞車/漂移
/// - Shift = 加速模式
/// - 雨天：牽引力 -30%，更容易打滑
/// - 霧天：視野降低（AI 用），操控略微下降
pub fn vehicle_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    game_state: Res<GameState>,
    weather: Res<WeatherState>,
    mut vehicles: Query<(&mut Transform, &mut Vehicle)>,
) {
    if !game_state.player_in_vehicle {
        return;
    }

    let Some(vehicle_entity) = game_state.current_vehicle else { return; };
    let Ok((mut transform, mut vehicle)) = vehicles.get_mut(vehicle_entity) else { return; };

    let dt = time.delta_secs();

    // 計算天氣影響因子
    let weather_traction = get_weather_traction_factor(&weather);
    let weather_handling = get_weather_handling_factor(&weather);

    // 1. 處理加速/煞車（非線性扭力曲線 + 天氣影響）
    update_vehicle_speed_with_weather(&keyboard, &mouse_button, &mut vehicle, dt, weather_traction);

    // 2. 處理轉向（速度敏感 + 平滑輸入 + 天氣影響）
    update_vehicle_turning_with_weather(&keyboard, &mut transform, &mut vehicle, dt, weather_handling);

    // 3. 漂移物理（雨天更容易漂移）
    update_drift_physics_with_weather(&mut vehicle, &mut transform, dt, weather_traction);

    // 4. 車身動態（側傾/前後傾）
    update_body_dynamics(&mut transform, &mut vehicle, dt);

    // 5. 移動車輛
    let forward = transform.forward();

    // 漂移時的移動方向修正
    let movement_dir = if vehicle.is_drifting && vehicle.drift_angle.abs() > 0.1 {
        // 漂移中：實際移動方向與車頭方向有夾角
        let drift_offset = Quat::from_rotation_y(-vehicle.drift_angle * 0.3);
        drift_offset * forward
    } else {
        forward
    };

    transform.translation += movement_dir * vehicle.current_speed * dt;
    transform.translation.y = get_vehicle_height(&vehicle.vehicle_type);
}

// ============================================================================
// 天氣影響駕駛系統
// ============================================================================

/// 計算天氣對牽引力的影響
fn get_weather_traction_factor(weather: &WeatherState) -> f32 {
    match weather.weather_type {
        WeatherType::Clear => WEATHER_CLEAR_TRACTION,
        WeatherType::Cloudy => WEATHER_CLOUDY_TRACTION,
        WeatherType::Rainy => WEATHER_RAINY_TRACTION_BASE + (1.0 - weather.intensity) * WEATHER_RAINY_TRACTION_RANGE,
        WeatherType::Foggy => WEATHER_FOGGY_TRACTION,
    }
}

/// 計算天氣對操控的影響
fn get_weather_handling_factor(weather: &WeatherState) -> f32 {
    match weather.weather_type {
        WeatherType::Clear => WEATHER_CLEAR_TRACTION,
        WeatherType::Cloudy => WEATHER_CLOUDY_TRACTION,
        WeatherType::Rainy => WEATHER_RAINY_HANDLING_BASE + (1.0 - weather.intensity) * WEATHER_RAINY_HANDLING_RANGE,
        WeatherType::Foggy => WEATHER_FOGGY_HANDLING,
    }
}

/// 更新車輛速度（含天氣影響）
fn update_vehicle_speed_with_weather(
    keyboard: &ButtonInput<KeyCode>,
    mouse_button: &ButtonInput<MouseButton>,
    vehicle: &mut Vehicle,
    dt: f32,
    traction_factor: f32,
) {
    let accel_mult = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) { 1.3 } else { 1.0 };
    let both_mouse = mouse_button.pressed(MouseButton::Left) && mouse_button.pressed(MouseButton::Right);

    // 記錄輸入狀態
    vehicle.throttle_input = if keyboard.pressed(KeyCode::KeyW) || both_mouse { 1.0 } else { 0.0 };
    vehicle.brake_input = if keyboard.pressed(KeyCode::KeyS) { 1.0 } else { 0.0 };

    // === 加速處理（非線性曲線 + 天氣影響）===
    if vehicle.throttle_input > 0.0 {
        let accel_force = calculate_acceleration_force(vehicle);
        let effective_accel = accel_force * accel_mult * vehicle.throttle_input * traction_factor;

        // 輪胎打滑模擬（Shift 加速起步時 + 雨天更容易打滑）
        let slip_threshold = if traction_factor < 0.9 { 8.0 } else { 5.0 };
        if vehicle.current_speed < slip_threshold && (accel_mult > 1.0 || traction_factor < 0.8) {
            let slip_factor = if traction_factor < 0.8 { 4.0 } else { 3.0 };
            vehicle.wheel_spin = (vehicle.wheel_spin + dt * slip_factor).min(1.0);
            let grip = traction_factor * (1.0 - vehicle.wheel_spin * 0.4);
            vehicle.current_speed += effective_accel * grip * dt;
        } else {
            vehicle.wheel_spin = (vehicle.wheel_spin - dt * 2.0).max(0.0);
            vehicle.current_speed += effective_accel * dt;
        }
    }

    // === 煞車處理（S 鍵 + 天氣影響）===
    if vehicle.brake_input > 0.0 && !vehicle.is_handbraking {
        if vehicle.current_speed > 0.5 {
            // 雨天煞車距離更長
            let brake_decel = vehicle.brake_force * vehicle.brake_input * traction_factor;
            vehicle.current_speed -= brake_decel * dt;
            if vehicle.current_speed < 0.0 {
                vehicle.current_speed = 0.0;
            }
        } else {
            let reverse_accel = calculate_acceleration_force(vehicle) * 0.5 * traction_factor;
            vehicle.current_speed -= reverse_accel * dt;
        }
    }

    // === 手煞車處理（漂移觸發 + 雨天更容易）===
    if vehicle.is_handbraking {
        let handbrake_decel = vehicle.handbrake_force * 0.03 * traction_factor;
        vehicle.current_speed *= 1.0 - handbrake_decel;

        // 觸發漂移條件：雨天更容易漂移
        let drift_speed_threshold = if traction_factor < 0.9 { 6.0 } else { 8.0 };
        let drift_steer_threshold = if traction_factor < 0.9 { 0.2 } else { 0.3 };
        if vehicle.current_speed.abs() > drift_speed_threshold && vehicle.steer_input.abs() > drift_steer_threshold {
            if !vehicle.is_drifting {
                vehicle.is_drifting = true;
            }
        }
    }

    // === 自然減速（引擎煞車 + 阻力）===
    let no_input = vehicle.throttle_input == 0.0 && vehicle.brake_input == 0.0 && !vehicle.is_handbraking;
    if no_input {
        let drag = 1.0 + (vehicle.current_speed.abs() / vehicle.max_speed) * 0.5;
        vehicle.current_speed *= 1.0 - 0.025 * drag;
    }

    // 速度限制
    vehicle.current_speed = vehicle.current_speed.clamp(
        -vehicle.max_speed * 0.3,
        vehicle.max_speed,
    );

    // 極低速歸零
    if vehicle.current_speed.abs() < 0.1 && vehicle.throttle_input == 0.0 {
        vehicle.current_speed = 0.0;
    }
}

/// 更新車輛轉向（含天氣影響）
fn update_vehicle_turning_with_weather(
    keyboard: &ButtonInput<KeyCode>,
    transform: &mut Transform,
    vehicle: &mut Vehicle,
    dt: f32,
    handling_factor: f32,
) {
    // 靜止時不能轉向
    if vehicle.current_speed.abs() <= 0.5 {
        vehicle.steer_input *= 0.9;
        return;
    }

    // 讀取原始轉向輸入
    let raw_input = if keyboard.pressed(KeyCode::KeyA) {
        1.0
    } else if keyboard.pressed(KeyCode::KeyD) {
        -1.0
    } else {
        0.0
    };

    // 平滑轉向輸入
    let steer_response = vehicle.steering_response * dt * handling_factor;
    vehicle.steer_input += (raw_input - vehicle.steer_input) * steer_response.min(1.0);

    if vehicle.steer_input.abs() < 0.01 {
        return;
    }

    // === 速度影響轉向 ===
    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);

    let speed_turn_factor = if speed_ratio < 0.3 {
        1.0
    } else {
        let high_speed_falloff = (speed_ratio - 0.3) / 0.7;
        1.0 - high_speed_falloff * (1.0 - vehicle.high_speed_turn_factor)
    };

    // 漂移中的轉向加成
    let drift_turn_bonus = if vehicle.is_drifting {
        1.0 + vehicle.drift_angle.abs() * vehicle.counter_steer_assist
    } else {
        1.0
    };

    let direction = vehicle.current_speed.signum();
    let effective_turn = vehicle.turn_speed
        * vehicle.handling
        * handling_factor  // 天氣影響操控
        * speed_turn_factor
        * drift_turn_bonus
        * vehicle.steer_input
        * direction
        * dt;

    transform.rotate_y(effective_turn);
}

/// 漂移物理系統（含天氣影響）
fn update_drift_physics_with_weather(
    vehicle: &mut Vehicle,
    _transform: &mut Transform,
    dt: f32,
    traction_factor: f32,
) {
    // 雨天漂移係數增加
    let drift_amplifier = if traction_factor < 0.9 { 1.3 } else { 1.0 };

    // 手煞車觸發漂移
    let drift_speed_threshold = if traction_factor < 0.9 { 6.0 } else { 8.0 };
    if vehicle.is_handbraking && vehicle.current_speed.abs() > drift_speed_threshold {
        vehicle.drift_angle += vehicle.steer_input * dt * 2.5 * drift_amplifier;
        vehicle.drift_angle = vehicle.drift_angle.clamp(-0.8, 0.8);
        vehicle.is_drifting = vehicle.drift_angle.abs() > vehicle.drift_threshold;
    } else if vehicle.is_drifting {
        // 漂移中的反制力（雨天更難控制）
        let counter = -vehicle.drift_angle * (1.0 - vehicle.drift_grip * traction_factor) * dt * 3.0;
        vehicle.drift_angle += counter;

        // 反打方向盤救車
        if vehicle.steer_input != 0.0 && vehicle.steer_input.signum() == -vehicle.drift_angle.signum() {
            vehicle.drift_angle += vehicle.steer_input * vehicle.counter_steer_assist * dt * 2.0;
        }

        // 漂移結束判定
        let end_speed = if traction_factor < 0.9 { 4.0 } else { 5.0 };
        if vehicle.drift_angle.abs() < 0.1 || vehicle.current_speed.abs() < end_speed {
            vehicle.is_drifting = false;
            vehicle.drift_angle = 0.0;
        }

        // 漂移損失速度（雨天損失更少，因為更滑）
        let drift_speed_loss = vehicle.drift_angle.abs() * (1.0 - vehicle.drift_grip) * traction_factor * dt * 0.5;
        vehicle.current_speed *= 1.0 - drift_speed_loss;
    } else {
        // 非漂移狀態：側滑角度歸零
        vehicle.drift_angle *= 1.0 - dt * 4.0;
        if vehicle.drift_angle.abs() < 0.05 {
            vehicle.drift_angle = 0.0;
        }
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

/// 車身動態效果（側傾/前後傾）
fn update_body_dynamics(
    transform: &mut Transform,
    vehicle: &mut Vehicle,
    dt: f32,
) {
    // 機車使用專門的傾斜系統
    if vehicle.vehicle_type == VehicleType::Scooter {
        update_scooter_lean(transform, vehicle, vehicle.steer_input, dt);
        return;
    }

    let speed_ratio = (vehicle.current_speed.abs() / vehicle.max_speed).clamp(0.0, 1.0);

    // === 車身側傾（轉彎時）===
    let target_roll = -vehicle.steer_input * vehicle.body_roll_factor * speed_ratio.sqrt();

    // 漂移時增加側傾
    let drift_roll_bonus = if vehicle.is_drifting {
        vehicle.drift_angle * 0.3
    } else {
        0.0
    };

    // === 車身前後傾（加速/煞車時）===
    let accel_state = vehicle.throttle_input - vehicle.brake_input;
    let target_pitch = -accel_state * vehicle.body_pitch_factor * speed_ratio.sqrt().min(0.8);

    // 手煞車時額外前傾
    let handbrake_pitch = if vehicle.is_handbraking { 0.04 } else { 0.0 };

    // === 懸吊模擬（平滑過渡）===
    let suspension_speed = vehicle.suspension_stiffness * dt;
    vehicle.body_roll += ((target_roll + drift_roll_bonus) - vehicle.body_roll) * suspension_speed;
    vehicle.body_pitch += ((target_pitch + handbrake_pitch) - vehicle.body_pitch) * suspension_speed;

    // 限制傾斜範圍
    vehicle.body_roll = vehicle.body_roll.clamp(-0.2, 0.2);
    vehicle.body_pitch = vehicle.body_pitch.clamp(-0.15, 0.15);

    // === 應用旋轉 ===
    let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);

    // 漂移時的車頭視覺偏移
    let visual_yaw = if vehicle.is_drifting {
        yaw + vehicle.drift_angle * 0.25
    } else {
        yaw
    };

    // 組合旋轉：Yaw -> Pitch -> Roll
    transform.rotation = Quat::from_rotation_y(visual_yaw)
        * Quat::from_rotation_x(vehicle.body_pitch)
        * Quat::from_rotation_z(vehicle.body_roll);
}

/// 更新機車傾斜效果
/// 機車過彎時會自然傾斜，增加真實感
fn update_scooter_lean(
    transform: &mut Transform,
    vehicle: &mut Vehicle,
    turn_input: f32,
    dt: f32,
) {
    // 目標傾斜角度：根據轉向輸入和速度計算
    // 速度越快，傾斜越明顯
    let speed_factor = (vehicle.current_speed / vehicle.max_speed).clamp(0.0, 1.0);
    let target_lean = turn_input * vehicle.max_lean_angle * speed_factor;

    // 平滑過渡到目標傾斜角度
    let lean_speed = 5.0; // 傾斜過渡速度
    let lean_diff = target_lean - vehicle.lean_angle;
    vehicle.lean_angle += lean_diff * lean_speed * dt;
    // 硬限制傾斜角度，防止浮點數累積誤差
    vehicle.lean_angle = vehicle.lean_angle.clamp(-vehicle.max_lean_angle, vehicle.max_lean_angle);

    // 取得當前的 yaw 旋轉（繞 Y 軸）
    let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);

    // 重建旋轉：先 yaw，再傾斜
    transform.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_z(-vehicle.lean_angle);
}

/// 取得車輛高度
fn get_vehicle_height(vehicle_type: &VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 0.5,
        VehicleType::Car | VehicleType::Taxi => 0.6,
        VehicleType::Bus => 1.25,
    }
}

// === NPC AI 輔助函數 ===

/// 檢測障礙物結果
enum ObstacleCheckResult {
    TooClose,    // 太近，需要倒車
    NeedBrake,   // 需要煞車
    Clear,       // 前方淨空
}

/// 檢查前方障礙物
fn check_obstacle(
    rapier: &RapierContext,
    entity: Entity,
    transform: &Transform,
) -> ObstacleCheckResult {
    let ray_pos = transform.translation + Vec3::new(0.0, 0.6, 0.0);
    let ray_dir = transform.forward().as_vec3();
    let max_toi: bevy_rapier3d::prelude::Real = 8.0;
    let filter = QueryFilter::new().exclude_rigid_body(entity);

    if let Some((_hit_entity, toi)) = rapier.cast_ray(
        ray_pos, ray_dir, max_toi, true, filter
    ) {
        let toi_f32 = rapier_real_to_f32(toi);
        if toi_f32 < 4.0 {
            ObstacleCheckResult::TooClose
        } else {
            ObstacleCheckResult::NeedBrake
        }
    } else {
        ObstacleCheckResult::Clear
    }
}

/// 更新 NPC 狀態（根據障礙物檢測結果）
fn update_npc_state_from_obstacle(
    npc: &mut NpcVehicle,
    vehicle: &mut Vehicle,
    result: ObstacleCheckResult,
) {
    match result {
        ObstacleCheckResult::TooClose => {
            if npc.state != NpcState::Reversing {
                npc.state = NpcState::Reversing;
                npc.stuck_timer = 0.0;
                vehicle.current_speed = -3.0;
            }
        }
        ObstacleCheckResult::NeedBrake => {
            npc.state = NpcState::Braking;
        }
        ObstacleCheckResult::Clear => {
            if npc.state != NpcState::Cruising && npc.state != NpcState::Reversing {
                npc.state = NpcState::Cruising;
            }
        }
    }
}

/// 導航至下一個路點
fn navigate_to_waypoint(
    npc: &mut NpcVehicle,
    transform: &mut Transform,
    vehicle: &Vehicle,
    dt: f32,
) {
    if npc.waypoints.is_empty() {
        // 無路點時，若超出邊界則掉頭
        if transform.translation.x.abs() > 300.0 || transform.translation.z.abs() > 300.0 {
            transform.rotate_y(std::f32::consts::PI * dt);
        }
        return;
    }

    let target = npc.waypoints[npc.current_wp_index];
    let current_pos = transform.translation;
    let distance_sq = current_pos.distance_squared(target);

    // 到達路點，切換到下一個 (使用 distance_squared 避免 sqrt)
    if distance_sq < NPC_WAYPOINT_ARRIVAL_DISTANCE_SQ {
        npc.current_wp_index = (npc.current_wp_index + 1) % npc.waypoints.len();
        return;
    }

    // 計算轉向
    let target_flat = Vec3::new(target.x, current_pos.y, target.z);
    let dir_to_target = (target_flat - current_pos).normalize_or_zero();
    if dir_to_target == Vec3::ZERO {
        return;
    }

    let current_forward = transform.forward().as_vec3();
    let cross = current_forward.cross(dir_to_target);
    let dot = current_forward.dot(dir_to_target);
    let angle = dot.clamp(-1.0, 1.0).acos();

    if angle > 0.01 {
        let turn_dir = if cross.y > 0.0 { 1.0 } else { -1.0 };
        let actual_turn = angle.min(vehicle.turn_speed * dt);
        transform.rotate_y(actual_turn * turn_dir);
    }
}

/// 處理巡航狀態
fn handle_cruising_state(
    npc: &mut NpcVehicle,
    transform: &mut Transform,
    vehicle: &mut Vehicle,
    dt: f32,
) {
    npc.stuck_timer = 0.0;

    // 加速到目標速度
    let target_speed = vehicle.max_speed * NPC_CRUISING_SPEED_RATIO;
    if vehicle.current_speed < target_speed {
        vehicle.current_speed += vehicle.acceleration * 0.5 * dt;
    }

    // 導航
    navigate_to_waypoint(npc, transform, vehicle, dt);

    // 移動
    let forward = transform.forward();
    transform.translation += forward * vehicle.current_speed * dt;
    transform.translation.y = get_vehicle_height(&vehicle.vehicle_type);
}

/// 處理煞車狀態
fn handle_braking_state(
    npc: &mut NpcVehicle,
    transform: &mut Transform,
    vehicle: &mut Vehicle,
    dt: f32,
) {
    npc.stuck_timer = 0.0;
    vehicle.current_speed *= 0.8;

    if vehicle.current_speed < 0.1 {
        vehicle.current_speed = 0.0;
        npc.state = NpcState::Stopped;
    }

    let forward = transform.forward();
    transform.translation += forward * vehicle.current_speed * dt;
}

/// 處理停止狀態
fn handle_stopped_state(npc: &mut NpcVehicle, vehicle: &mut Vehicle, dt: f32) {
    vehicle.current_speed = 0.0;
    npc.stuck_timer += dt;

    // 防卡死：停止太久就倒車
    if npc.stuck_timer > 3.0 {
        npc.state = NpcState::Reversing;
        npc.stuck_timer = 0.0;
        vehicle.current_speed = -3.0;
    }
}

/// 處理倒車狀態
fn handle_reversing_state(
    npc: &mut NpcVehicle,
    transform: &mut Transform,
    vehicle: &mut Vehicle,
    dt: f32,
) {
    npc.stuck_timer += dt;

    // 直直倒車
    let forward = transform.forward();
    transform.translation += forward * vehicle.current_speed * dt;

    // 倒車 1.5 秒後嘗試前進
    if npc.stuck_timer > 1.5 {
        npc.state = NpcState::Cruising;
        vehicle.current_speed = 0.0;
        npc.stuck_timer = 0.0;
    }
}

/// NPC 車輛 AI（含避障功能和紅綠燈遵守）
pub fn npc_vehicle_ai(
    time: Res<Time>,
    rapier_context: ReadRapierContext,
    mut npc_query: Query<(Entity, &mut Transform, &mut Vehicle, &mut NpcVehicle)>,
    traffic_light_query: Query<(&Transform, &TrafficLight), Without<NpcVehicle>>,
) {
    let dt = time.delta_secs();
    let Ok(rapier) = rapier_context.single() else { return };

    for (entity, mut transform, mut vehicle, mut npc) in npc_query.iter_mut() {
        // 定期檢查前方障礙物和紅綠燈
        npc.check_timer.tick(time.delta());
        if npc.check_timer.just_finished() {
            let result = check_obstacle(&rapier, entity, &transform);
            update_npc_state_from_obstacle(&mut npc, &mut vehicle, result);

            // 檢查紅綠燈（只在巡航狀態下）
            if npc.state == NpcState::Cruising {
                let vehicle_pos = transform.translation;
                let vehicle_forward = transform.forward().as_vec3();

                for (light_transform, light) in traffic_light_query.iter() {
                    if light.should_vehicle_stop(vehicle_pos, vehicle_forward, light_transform.translation) {
                        npc.state = NpcState::WaitingAtLight;
                        break;
                    }
                }
            }
        }

        // 根據狀態執行行為
        match npc.state {
            NpcState::Cruising => handle_cruising_state(&mut npc, &mut transform, &mut vehicle, dt),
            NpcState::Braking => handle_braking_state(&mut npc, &mut transform, &mut vehicle, dt),
            NpcState::Stopped => handle_stopped_state(&mut npc, &mut vehicle, dt),
            NpcState::Reversing => handle_reversing_state(&mut npc, &mut transform, &mut vehicle, dt),
            NpcState::WaitingAtLight => handle_waiting_at_light_state(&mut npc, &mut vehicle, &transform, &traffic_light_query, dt),
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

    // 檢查燈是否變綠了
    let vehicle_pos = transform.translation;
    let vehicle_forward = transform.forward().as_vec3();

    let mut should_wait = false;
    for (light_transform, light) in traffic_light_query.iter() {
        if light.should_vehicle_stop(vehicle_pos, vehicle_forward, light_transform.translation) {
            should_wait = true;
            break;
        }
    }

    // 燈變綠了，恢復巡航
    if !should_wait {
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
    spawn_vehicle_light(parent, meshes, headlight_mat.clone(), -light_x, 0.1, light_z);
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
    commands.spawn((
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
        CollisionGroups::new(
            COLLISION_GROUP_VEHICLE,
            COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
        ),  // NPC 載具與角色、載具、靜態物碰撞
        // 遊戲邏輯組件
        vehicle_component,
        VehicleHealth::for_vehicle_type(vehicle_type),  // 車輛血量
        NpcVehicle {
            waypoints,
            current_wp_index: start_index,
            ..default()
        },
        Name::new(format!("NpcVehicle_{:?}", vehicle_type)),
    )).with_children(|parent| {
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
                Transform::from_translation(pos).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
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
                Transform::from_xyz(-0.6, chassis_size.y/2.0 + strut_h/2.0, chassis_size.z/2.0 - 0.2),
                GlobalTransform::default(),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.1, strut_h, 0.1))),
                MeshMaterial3d(shared_mats.black_plastic.clone()),
                Transform::from_xyz(0.6, chassis_size.y/2.0 + strut_h/2.0, chassis_size.z/2.0 - 0.2),
                GlobalTransform::default(),
            ));
            // 翼板
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.8, 0.05, 0.4))),
                MeshMaterial3d(shared_mats.black_plastic.clone()),
                Transform::from_xyz(0.0, chassis_size.y/2.0 + strut_h, chassis_size.z/2.0 - 0.2),
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
                Transform::from_xyz(-chassis_size.x/2.0 - 0.02, -chassis_size.y/2.0 + 0.1, 0.0),
                GlobalTransform::default(),
            ));
            // 右側條
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.05, 0.05, 2.5))),
                MeshMaterial3d(neon_mat),
                Transform::from_xyz(chassis_size.x/2.0 + 0.02, -chassis_size.y/2.0 + 0.1, 0.0),
                GlobalTransform::default(),
            ));
        }
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
    // 中華路車道範圍：X = 50 ~ 100

    // 路線 1：外圈 (東側車道) - 逆時針
    let route_east = vec![
        Vec3::new(-48.0, 0.0, 48.0),  // 西寧/成都 西側
        Vec3::new(90.0, 0.0, 48.0),   // 中華/成都 東側
        Vec3::new(90.0, 0.0, -70.0),  // 中華路北端
        Vec3::new(-48.0, 0.0, -70.0), // 西寧路北端
        Vec3::new(-48.0, 0.0, 48.0),  // 返回起點
    ];

    // 路線 2：內圈 (西側車道) - 順時針
    let route_west = vec![
        Vec3::new(60.0, 0.0, 52.0),   // 中華/成都 西側車道
        Vec3::new(-52.0, 0.0, 52.0),  // 西寧/成都 西側
        Vec3::new(-52.0, 0.0, -70.0), // 西寧路北端
        Vec3::new(60.0, 0.0, -70.0),  // 中華路北端西側
        Vec3::new(60.0, 0.0, 52.0),   // 返回起點
    ];

    // 路線 3：南北向主要道路（中華路 X=75，地圖範圍內）
    let route_north_south = vec![
        Vec3::new(75.0, 0.0, -75.0),  // 北端（漢口街附近）
        Vec3::new(75.0, 0.0, 55.0),   // 南端（成都路附近）
        Vec3::new(75.0, 0.0, -75.0),  // 返回
    ];

    // 路線 4：東西向道路（沿成都路 Z=50，避開徒步區 Z=0）
    let route_east_west = vec![
        Vec3::new(-90.0, 0.0, 50.0),  // 西端起點（成都路西側）
        Vec3::new(70.0, 0.0, 50.0),   // 東端（成都路東側）
        Vec3::new(-90.0, 0.0, 50.0),  // 返回
    ];

    // 車輛顏色池
    let car_colors = [
        Color::srgb(0.8, 0.2, 0.2),   // 紅色
        Color::srgb(0.2, 0.2, 0.8),   // 藍色
        Color::srgb(0.9, 0.9, 0.9),   // 白色
        Color::srgb(0.1, 0.1, 0.1),   // 黑色
        Color::srgb(0.7, 0.7, 0.7),   // 銀色
        Color::srgb(0.2, 0.6, 0.2),   // 綠色
        Color::srgb(1.0, 0.5, 0.0),   // 橙色
    ];

    // 生成配置 (位置, 類型, 顏色, 起始索引, 路徑)
    let spawn_configs = [
        // 計程車 1 - 東行路線 (外圈)
        (route_east[0], VehicleType::Taxi, Color::srgb(1.0, 0.8, 0.0), 0, route_east.clone()),

        // 公車 - 西行路線 (內圈)
        (route_west[0], VehicleType::Bus, Color::srgb(0.2, 0.4, 0.8), 0, route_west.clone()),

        // 汽車 1 - 外圈（不同起點）
        (route_east[1], VehicleType::Car, car_colors[0], 1, route_east.clone()),

        // 汽車 2 - 內圈
        (route_west[2], VehicleType::Car, car_colors[1], 2, route_west.clone()),

        // 計程車 2 - 外圈另一位置
        (route_east[2], VehicleType::Taxi, Color::srgb(1.0, 0.8, 0.0), 2, route_east.clone()),

        // 汽車 3 - 南北向道路
        (route_north_south[0], VehicleType::Car, car_colors[2], 0, route_north_south.clone()),

        // 汽車 4 - 東西向道路
        (route_east_west[0], VehicleType::Car, car_colors[3], 0, route_east_west.clone()),

        // 汽車 5 - 外圈第三位置
        (route_east[3], VehicleType::Car, car_colors[4], 3, route_east.clone()),

        // 汽車 6 - 內圈第二位置
        (route_west[1], VehicleType::Car, car_colors[5], 1, route_west.clone()),

        // 汽車 7 - 南北向反向
        (route_north_south[1], VehicleType::Car, car_colors[6], 1, route_north_south.clone()),
    ];

    info!("🚗 生成 {} 台初始交通車輛", spawn_configs.len());

    for (i, (pos, v_type, color, start_idx, path)) in spawn_configs.iter().enumerate() {
        debug!("  - 生成車輛 #{}: {:?} 於 {:?}", i, v_type, pos);

        // 它的首個目標應該是它所在位置的下一個點
        let next_idx = (*start_idx as usize + 1) % path.len();

        spawn_npc_vehicle(
            &mut commands, &mut meshes, &mut materials, &shared_mats,
            *pos, Quat::IDENTITY, *v_type, *color,
            path.clone(), next_idx
        );
    }

    // === 生成紅綠燈 ===
    // 交叉路口 1：中華路/成都路交叉口
    let intersection1 = Vec3::new(75.0, 0.0, 50.0);
    spawn_intersection_lights(&mut commands, &traffic_visuals, intersection1, 15.0);

    // 交叉路口 2：西寧路/成都路交叉口
    let intersection2 = Vec3::new(-50.0, 0.0, 50.0);
    spawn_intersection_lights(&mut commands, &traffic_visuals, intersection2, 12.0);

    // 交叉路口 3：主要十字路口（地圖中心）
    let intersection3 = Vec3::new(0.0, 0.0, 0.0);
    spawn_intersection_lights(&mut commands, &traffic_visuals, intersection3, 15.0);

    info!("🚦 生成 3 個交叉路口紅綠燈（共 12 座）");

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

    commands.spawn((
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
        CollisionGroups::new(
            COLLISION_GROUP_VEHICLE,
            COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
        ),  // 機車與角色、載具、靜態物碰撞
        Vehicle::scooter(),
        VehicleHealth::for_vehicle_type(VehicleType::Scooter),  // 車輛血量
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
            Transform::from_xyz(-body_width / 2.0 - 0.2, seat_height * 0.85, -body_length / 2.0 + 0.15),
            GlobalTransform::default(),
        ));
        // 右鏡
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.08, 0.05, 0.02))),
            MeshMaterial3d(shared_mats.mirror.clone()),
            Transform::from_xyz(body_width / 2.0 + 0.2, seat_height * 0.85, -body_length / 2.0 + 0.15),
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

    debug!("🛵 生成機車於 {:?}", position);
}

// ============================================================================
// 車輛視覺效果系統（GTA 5 風格）
// ============================================================================

/// 漂移煙霧生成系統
/// 當車輛漂移或急煞時，在後輪位置生成煙霧粒子
pub fn drift_smoke_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    effect_visuals: Option<Res<VehicleEffectVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle), Without<NpcVehicle>>,  // 只處理玩家駕駛的車輛
) {
    let Some(visuals) = effect_visuals else { return };
    let current_time = time.elapsed_secs();

    // 檢查生成間隔
    if current_time - effect_tracker.last_smoke_spawn < effect_tracker.smoke_spawn_interval {
        return;
    }

    for (transform, vehicle) in vehicle_query.iter() {
        // 只在漂移或高速手煞車時生成煙霧
        let should_spawn = (vehicle.is_drifting && vehicle.drift_angle.abs() > 0.2)
            || (vehicle.is_handbraking && vehicle.current_speed > 10.0)
            || (vehicle.wheel_spin > 0.5);  // 輪胎打滑時也有煙

        if !should_spawn {
            continue;
        }

        // 檢查粒子數量限制
        if effect_tracker.smoke_count >= effect_tracker.max_smoke_count {
            continue;
        }

        // 計算後輪位置（相對於車輛）
        let rear_offset = match vehicle.vehicle_type {
            VehicleType::Scooter => Vec3::new(0.0, 0.0, 0.8),
            VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.0, 1.5),
            VehicleType::Bus => Vec3::new(0.0, 0.0, 3.0),
        };

        // 轉換到世界座標
        let world_pos = transform.translation + transform.rotation * rear_offset;
        let wheel_height = 0.2;

        // 生成煙霧粒子（左右各一個）
        let mut rng = rand::rng();
        for side in [-1.0, 1.0] {
            let wheel_offset = match vehicle.vehicle_type {
                VehicleType::Scooter => 0.0,  // 機車只有中間
                _ => 0.8 * side,
            };

            let spawn_pos = world_pos + transform.rotation * Vec3::new(wheel_offset, wheel_height, 0.0);

            // 隨機散射速度
            let spread = Vec3::new(
                rng.random_range(-0.5..0.5),
                rng.random_range(0.3..0.8),
                rng.random_range(-0.5..0.5),
            );

            // 煙霧往後飄（相對於車輛移動方向）
            let base_velocity = -transform.forward().as_vec3() * (vehicle.current_speed * 0.1).max(1.0);
            let velocity = base_velocity + spread;

            commands.spawn((
                Mesh3d(visuals.smoke_mesh.clone()),
                MeshMaterial3d(visuals.smoke_material.clone()),
                Transform::from_translation(spawn_pos)
                    .with_scale(Vec3::splat(0.3)),
                DriftSmoke::new(velocity, rng.random_range(0.5..1.0)),
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

/// 輪胎痕跡生成系統
/// 當車輛漂移或急煞時，在地面留下輪胎痕跡
pub fn tire_track_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    effect_visuals: Option<Res<VehicleEffectVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle), Without<NpcVehicle>>,
) {
    let Some(visuals) = effect_visuals else { return };
    let current_time = time.elapsed_secs();

    // 檢查生成間隔
    if current_time - effect_tracker.last_track_spawn < effect_tracker.track_spawn_interval {
        return;
    }

    for (transform, vehicle) in vehicle_query.iter() {
        // 只在漂移或急煞時生成痕跡
        let should_spawn = (vehicle.is_drifting && vehicle.drift_angle.abs() > 0.15)
            || (vehicle.is_handbraking && vehicle.current_speed > 8.0);

        if !should_spawn {
            continue;
        }

        // 檢查痕跡數量限制
        if effect_tracker.track_count >= effect_tracker.max_track_count {
            continue;
        }

        // 計算後輪位置
        let rear_offset = match vehicle.vehicle_type {
            VehicleType::Scooter => Vec3::new(0.0, 0.0, 0.7),
            VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.0, 1.2),
            VehicleType::Bus => Vec3::new(0.0, 0.0, 2.5),
        };

        // 痕跡寬度根據漂移角度調整
        let track_width = 0.2 + vehicle.drift_angle.abs() * 0.3;

        // 生成左右輪胎痕跡
        for side in [-1.0, 1.0] {
            let wheel_offset = match vehicle.vehicle_type {
                VehicleType::Scooter => 0.0,
                _ => 0.8 * side,
            };

            let track_pos = transform.translation
                + transform.rotation * (rear_offset + Vec3::new(wheel_offset, 0.0, 0.0));

            // 貼地
            let ground_pos = Vec3::new(track_pos.x, 0.02, track_pos.z);

            // 創建輪胎痕跡
            commands.spawn((
                Mesh3d(visuals.tire_track_mesh.clone()),
                MeshMaterial3d(visuals.tire_track_material.clone()),
                Transform::from_translation(ground_pos)
                    .with_rotation(transform.rotation)
                    .with_scale(Vec3::new(track_width, 1.0, 0.8)),
                TireTrack::default(),
            ));

            effect_tracker.track_count += 1;

            // 機車只生成一條痕跡
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
    VehicleHealth, VehicleDamageState, VehicleDamageVisuals,
    VehicleDamageSmoke, VehicleFire, VehicleExplosion, TireDamage,
};
use crate::combat::{DamageEvent, DamageSource, Health, Enemy, Damageable};
use crate::player::Player;
use crate::pedestrian::Pedestrian;
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
                continue;  // 低速碰撞不造成傷害
            }

            // 傷害公式：(速度 - 門檻) * 倍率
            // 例如：30 m/s = (30-10) * 5 = 100 傷害
            let damage = (speed - COLLISION_DAMAGE_SPEED_THRESHOLD) * COLLISION_DAMAGE_MULTIPLIER;
            health.take_damage(damage, current_time);
            break;  // 一次碰撞只計算一次傷害
        }
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
        // 已爆炸的車輛不處理
        if health.is_destroyed() {
            continue;
        }

        // 更新著火計時器
        if health.tick_fire(dt) {
            // 爆炸！
            let explosion_pos = transform.translation + Vec3::Y * 0.5;
            let explosion_radius = match vehicle.vehicle_type {
                VehicleType::Scooter => 5.0,
                VehicleType::Car | VehicleType::Taxi => 8.0,
                VehicleType::Bus => 12.0,
            };
            let explosion_damage = match vehicle.vehicle_type {
                VehicleType::Scooter => 100.0,
                VehicleType::Car | VehicleType::Taxi => 200.0,
                VehicleType::Bus => 300.0,
            };

            // 生成爆炸效果
            if let Some(ref visuals) = damage_visuals {
                commands.spawn((
                    Mesh3d(visuals.explosion_mesh.clone()),
                    MeshMaterial3d(visuals.explosion_material.clone()),
                    Transform::from_translation(explosion_pos),
                    VehicleExplosion::new(explosion_pos, explosion_radius, explosion_damage),
                ));
            }

            // 移除車輛實體
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
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
    let Some(visuals) = damage_visuals else { return };
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
        Transform::from_translation(position)
            .with_scale(Vec3::splat(0.2)),
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

/// 車輛爆炸系統
/// 處理爆炸效果和範圍傷害
/// 對範圍內的所有可傷害實體（玩家、敵人、行人、警察、其他車輛）造成傷害
#[allow(clippy::type_complexity)]
pub fn vehicle_explosion_system(
    mut commands: Commands,
    time: Res<Time>,
    mut camera_shake: ResMut<crate::core::CameraShake>,
    mut explosion_query: Query<(Entity, &mut VehicleExplosion, &mut Transform)>,
    // 玩家查詢（排除爆炸實體以避免 Query 衝突）
    player_query: Query<(Entity, &Transform), (With<Player>, Without<VehicleExplosion>)>,
    // 敵人查詢（排除爆炸實體以避免 Query 衝突）
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<VehicleExplosion>)>,
    // 行人查詢
    pedestrian_query: Query<(Entity, &Transform), (With<Pedestrian>, Without<Player>, Without<Enemy>, Without<VehicleExplosion>)>,
    // 警察查詢
    police_query: Query<(Entity, &Transform), (With<PoliceOfficer>, Without<Player>, Without<Enemy>, Without<VehicleExplosion>)>,
    // 其他車輛查詢（有 VehicleHealth 的實體，排除爆炸實體）
    vehicle_query: Query<(Entity, &Transform), (With<VehicleHealth>, Without<VehicleExplosion>)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let dt = time.delta_secs();

    for (entity, mut explosion, mut transform) in explosion_query.iter_mut() {
        // 更新生命時間
        explosion.lifetime += dt;

        // 更新縮放
        let scale = explosion.scale();
        transform.scale = Vec3::splat(scale);

        // 造成範圍傷害（只在第一幀）
        if !explosion.damage_dealt {
            explosion.damage_dealt = true;

            // 爆炸攝影機震動（依據距離玩家的距離調整強度）
            if let Ok((_, player_transform)) = player_query.single() {
                let distance_to_player = explosion.center.distance(player_transform.translation);
                let max_shake_distance = explosion.radius * 3.0; // 震動範圍為爆炸半徑的 3 倍
                if distance_to_player < max_shake_distance {
                    let shake_intensity = 0.5 * (1.0 - distance_to_player / max_shake_distance);
                    let shake_duration = 0.4 + 0.3 * (1.0 - distance_to_player / max_shake_distance);
                    camera_shake.trigger(shake_intensity, shake_duration);
                }
            }

            // 計算傷害的輔助閉包
            let calc_explosion_damage = |target_pos: Vec3| -> Option<f32> {
                let distance = explosion.center.distance(target_pos);
                if distance < explosion.radius {
                    // 距離衰減：越近傷害越高
                    let damage_factor = 1.0 - (distance / explosion.radius);
                    Some(explosion.damage * damage_factor)
                } else {
                    None
                }
            };

            // 對範圍內的玩家造成傷害
            for (target_entity, target_transform) in player_query.iter() {
                if let Some(damage) = calc_explosion_damage(target_transform.translation) {
                    damage_events.write(
                        DamageEvent::new(target_entity, damage, DamageSource::Explosion)
                            .with_position(explosion.center)
                    );
                }
            }

            // 對範圍內的敵人造成傷害
            for (target_entity, target_transform) in enemy_query.iter() {
                if let Some(damage) = calc_explosion_damage(target_transform.translation) {
                    damage_events.write(
                        DamageEvent::new(target_entity, damage, DamageSource::Explosion)
                            .with_position(explosion.center)
                    );
                }
            }

            // 對範圍內的行人造成傷害
            for (target_entity, target_transform) in pedestrian_query.iter() {
                if let Some(damage) = calc_explosion_damage(target_transform.translation) {
                    damage_events.write(
                        DamageEvent::new(target_entity, damage, DamageSource::Explosion)
                            .with_position(explosion.center)
                    );
                }
            }

            // 對範圍內的警察造成傷害
            for (target_entity, target_transform) in police_query.iter() {
                if let Some(damage) = calc_explosion_damage(target_transform.translation) {
                    damage_events.write(
                        DamageEvent::new(target_entity, damage, DamageSource::Explosion)
                            .with_position(explosion.center)
                    );
                }
            }

            // 對範圍內的其他車輛造成傷害（連鎖爆炸！）
            for (target_entity, target_transform) in vehicle_query.iter() {
                // 跳過自己（爆炸源車輛已被移除，但以防萬一）
                if target_entity == entity {
                    continue;
                }
                if let Some(damage) = calc_explosion_damage(target_transform.translation) {
                    damage_events.write(
                        DamageEvent::new(target_entity, damage, DamageSource::Explosion)
                            .with_position(explosion.center)
                    );
                }
            }
        }

        // 檢查是否過期
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
                    *material = MeshMaterial3d(visuals.get_bulb_material(bulb.light_type, light.state));
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

    commands.spawn((
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
    )).with_children(|parent| {
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
            TrafficLightBulb { light_type: TrafficLightState::Red },
        ));

        // 黃燈（中間）
        parent.spawn((
            Mesh3d(visuals.bulb_mesh.clone()),
            MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Yellow, initial_state)),
            Transform::from_xyz(0.0, 4.5, 0.16),
            GlobalTransform::default(),
            TrafficLightBulb { light_type: TrafficLightState::Yellow },
        ));

        // 綠燈（底部）
        parent.spawn((
            Mesh3d(visuals.bulb_mesh.clone()),
            MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Green, initial_state)),
            Transform::from_xyz(0.0, 4.1, 0.16),
            GlobalTransform::default(),
            TrafficLightBulb { light_type: TrafficLightState::Green },
        ));
    });
}

/// 生成交叉路口的紅綠燈組（4個方向）
pub fn spawn_intersection_lights(
    commands: &mut Commands,
    visuals: &TrafficLightVisuals,
    center: Vec3,
    road_width: f32,
) {
    let offset = road_width / 2.0 + 1.0;

    // 北向（控制南北向車流）- 主燈
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(-offset, 0.0, -offset),
        Vec3::NEG_Z,
        true,
    );

    // 南向（控制南北向車流）- 主燈
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(offset, 0.0, offset),
        Vec3::Z,
        true,
    );

    // 東向（控制東西向車流）- 副燈
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(offset, 0.0, -offset),
        Vec3::X,
        false,
    );

    // 西向（控制東西向車流）- 副燈
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(-offset, 0.0, offset),
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

    // 主要路口位置 - 根據實際車輛行駛道路設置
    // 車輛道路：中華路 (X≈75), 西寧南路 (X≈-50), 成都路 (Z≈50), 北邊道路 (Z≈-70)
    // 注意：徒步區 (X=0, Z=0 附近) 不需要紅綠燈
    let intersections = [
        // 西寧路/成都路交叉口（西南角）
        Vec3::new(-50.0, 0.0, 50.0),
        // 中華路/成都路交叉口（東南角）
        Vec3::new(75.0, 0.0, 50.0),
        // 西寧路北端路口
        Vec3::new(-50.0, 0.0, -70.0),
        // 中華路北端路口
        Vec3::new(75.0, 0.0, -70.0),
    ];

    let road_width = 12.0;  // 稍微加寬以符合主幹道

    for center in intersections.iter() {
        spawn_intersection_lights(&mut commands, &visuals, *center, road_width);
    }

    info!("✅ 已生成 {} 組交通燈（共 {} 個）", intersections.len(), intersections.len() * 4);
}

