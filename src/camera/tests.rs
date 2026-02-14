//! Camera 系統單元測試

use bevy::prelude::*;
use crate::core::CameraSettings;

// 從 systems.rs 複製的常數（用於測試）
const PITCH_MIN: f32 = -0.3;
const PITCH_MAX_INPUT: f32 = 1.2;
const PITCH_MAX_WITH_RECOIL: f32 = 1.5;
const CAMERA_DISTANCE_MIN: f32 = 5.0;
const CAMERA_DISTANCE_MAX: f32 = 80.0;

// ============================================================================
// Pitch 角度限制測試
// ============================================================================

#[test]
fn pitch_clamping_within_normal_range() {
    let settings = CameraSettings {
        pitch: 0.5,
        ..Default::default()
    };

    // 正常範圍內的 pitch 應該保持不變
    let clamped = settings.pitch.clamp(PITCH_MIN, PITCH_MAX_INPUT);
    assert_eq!(clamped, 0.5);
}

#[test]
fn pitch_clamped_to_min() {
    let settings = CameraSettings {
        pitch: -1.0, // 低於 PITCH_MIN
        ..Default::default()
    };

    let clamped = settings.pitch.clamp(PITCH_MIN, PITCH_MAX_INPUT);
    assert_eq!(clamped, PITCH_MIN);
}

#[test]
fn pitch_clamped_to_max_input() {
    let settings = CameraSettings {
        pitch: 2.0, // 超過 PITCH_MAX_INPUT
        ..Default::default()
    };

    let clamped = settings.pitch.clamp(PITCH_MIN, PITCH_MAX_INPUT);
    assert_eq!(clamped, PITCH_MAX_INPUT);
}

#[test]
fn pitch_with_recoil_allows_higher_max() {
    // 測試後座力可以超過正常輸入上限
    let base_pitch: f32 = 1.0;
    let recoil_offset: f32 = 0.4; // 後座力向上偏移
    let pitch_with_recoil: f32 = base_pitch + recoil_offset;

    // 後座力影響後的 pitch 可以達到 1.4，在 PITCH_MAX_INPUT (1.2) 之上
    // 但應該被限制在 PITCH_MAX_WITH_RECOIL (1.5) 以下
    let clamped = pitch_with_recoil.clamp(PITCH_MIN, PITCH_MAX_WITH_RECOIL);
    assert_eq!(clamped, 1.4);
    assert!(clamped > PITCH_MAX_INPUT);
    assert!(clamped <= PITCH_MAX_WITH_RECOIL);
}

#[test]
fn pitch_with_extreme_recoil_clamped_to_max() {
    let base_pitch: f32 = 1.2;
    let extreme_recoil: f32 = 1.0; // 極大的後座力
    let pitch_with_recoil: f32 = base_pitch + extreme_recoil;

    let clamped = pitch_with_recoil.clamp(PITCH_MIN, PITCH_MAX_WITH_RECOIL);
    assert_eq!(clamped, PITCH_MAX_WITH_RECOIL);
}

// ============================================================================
// 距離調整測試
// ============================================================================

#[test]
fn distance_clamping_within_range() {
    let mut settings = CameraSettings {
        distance: 20.0,
        ..Default::default()
    };

    settings.distance = settings.distance.clamp(CAMERA_DISTANCE_MIN, CAMERA_DISTANCE_MAX);
    assert_eq!(settings.distance, 20.0);
}

#[test]
fn distance_clamped_to_min() {
    let mut settings = CameraSettings {
        distance: 2.0, // 低於最小值
        ..Default::default()
    };

    settings.distance = settings.distance.clamp(CAMERA_DISTANCE_MIN, CAMERA_DISTANCE_MAX);
    assert_eq!(settings.distance, CAMERA_DISTANCE_MIN);
}

#[test]
fn distance_clamped_to_max() {
    let mut settings = CameraSettings {
        distance: 100.0, // 超過最大值
        ..Default::default()
    };

    settings.distance = settings.distance.clamp(CAMERA_DISTANCE_MIN, CAMERA_DISTANCE_MAX);
    assert_eq!(settings.distance, CAMERA_DISTANCE_MAX);
}

#[test]
fn distance_scroll_adjustment() {
    let mut settings = CameraSettings {
        distance: 30.0,
        ..Default::default()
    };

    // 模擬滾輪向上（y = 1.0），距離減少
    let scroll_y = 1.0;
    settings.distance -= scroll_y * 0.4;

    assert!((settings.distance - 29.6).abs() < 0.01);
}

#[test]
fn distance_mouse_y_adjustment_when_not_aiming() {
    let mut settings = CameraSettings {
        distance: 20.0,
        ..Default::default()
    };

    // 非瞄準時：滑鼠 Y 軸移動 = 調整距離
    let delta_y = 5.0;
    settings.distance += delta_y * 0.1;
    settings.distance = settings.distance.clamp(CAMERA_DISTANCE_MIN, CAMERA_DISTANCE_MAX);

    assert!((settings.distance - 20.5).abs() < 0.01);
}

// ============================================================================
// 滑鼠 Y 軸處理測試
// ============================================================================

#[test]
fn mouse_y_adjusts_pitch_when_aiming() {
    let mut settings = CameraSettings {
        pitch: 0.0,
        sensitivity: 0.002,
        ..Default::default()
    };

    let is_aiming = true;
    let delta_y = 100.0; // 滑鼠向下移動

    // 瞄準時：Y 軸移動 = pitch 調整
    if is_aiming {
        settings.pitch += delta_y * settings.sensitivity;
    }

    assert!((settings.pitch - 0.2).abs() < 0.01);
}

#[test]
fn mouse_y_adjusts_distance_when_not_aiming() {
    let mut settings = CameraSettings {
        distance: 30.0,
        pitch: 0.0,
        ..Default::default()
    };

    let is_aiming = false;
    let delta_y = 100.0;

    // 非瞄準時：Y 軸移動 = distance 調整
    if !is_aiming {
        settings.distance += delta_y * 0.1;
    }

    assert!((settings.distance - 40.0).abs() < 0.01);
}

// ============================================================================
// 角度正規化測試（camera_auto_follow 使用）
// ============================================================================

#[test]
fn angle_normalization_small_diff() {
    let target_yaw = 0.5;
    let current_yaw = 0.3;

    let mut angle_diff = target_yaw - current_yaw;

    // 正規化到 -PI ~ PI
    angle_diff = (angle_diff + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;

    assert!((angle_diff - 0.2).abs() < 0.01);
}

#[test]
fn angle_normalization_wraps_positive() {
    // 測試從 -PI 到 +PI 的過渡
    let target_yaw = std::f32::consts::PI - 0.1;
    let current_yaw = -std::f32::consts::PI + 0.1;

    let mut angle_diff = target_yaw - current_yaw;

    // 未正規化前：差值約 2*PI - 0.2（很大的正值）
    assert!(angle_diff > 6.0);

    // 正規化後：應該是 -0.2（選擇短路徑）
    angle_diff = (angle_diff + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;

    assert!((angle_diff + 0.2).abs() < 0.01);
}

#[test]
fn angle_normalization_wraps_negative() {
    // 測試從 +PI 到 -PI 的過渡
    let target_yaw = -std::f32::consts::PI + 0.1;
    let current_yaw = std::f32::consts::PI - 0.1;

    let mut angle_diff = target_yaw - current_yaw;

    // 未正規化前：差值約 -(2*PI - 0.2)（很大的負值）
    assert!(angle_diff < -6.0);

    // 正規化後：應該是 +0.2（選擇短路徑）
    angle_diff = (angle_diff + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;

    assert!((angle_diff - 0.2).abs() < 0.01);
}

#[test]
fn angle_normalization_exactly_pi() {
    // 測試邊界條件：正好是 PI
    let target_yaw = std::f32::consts::PI;
    let current_yaw = 0.0;

    let mut angle_diff = target_yaw - current_yaw;
    angle_diff = (angle_diff + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;

    // PI 應該被正規化為 -PI（在 -PI~PI 範圍的邊界）
    assert!((angle_diff.abs() - std::f32::consts::PI).abs() < 0.01);
}

// ============================================================================
// 攝影機設置默認值測試
// ============================================================================

#[test]
fn camera_settings_default_values() {
    let settings = CameraSettings::default();

    // 測試默認值是否合理
    assert!(settings.pitch >= PITCH_MIN && settings.pitch <= PITCH_MAX_INPUT);
    assert!(settings.distance >= CAMERA_DISTANCE_MIN && settings.distance <= CAMERA_DISTANCE_MAX);
    assert!(settings.sensitivity > 0.0);
    assert!(settings.aim_distance > 0.0 && settings.aim_distance < settings.distance);
}

// ============================================================================
// 瞄準模式距離測試
// ============================================================================

#[test]
fn aim_mode_distance_shorter_than_normal() {
    let settings = CameraSettings::default();

    // 瞄準模式的距離應該比正常模式短
    assert!(settings.aim_distance < settings.distance);
}

#[test]
fn aim_mode_shoulder_offset() {
    let settings = CameraSettings::default();

    // 瞄準時應該有過肩偏移
    assert!(settings.aim_shoulder_offset != 0.0);
    // 通常向右肩偏移（正值）
    assert!(settings.aim_shoulder_offset > 0.0);
}

// ============================================================================
// 攝影機偏移計算測試
// ============================================================================

#[test]
fn camera_offset_calculation() {
    let distance: f32 = 10.0;
    let pitch: f32 = 0.0; // 水平
    let yaw: f32 = 0.0;   // 朝向 +Z

    // 計算偏移（與 camera_follow 系統中的公式一致）
    let offset = Vec3::new(
        distance * pitch.cos() * yaw.sin(),
        distance * pitch.sin(),
        distance * pitch.cos() * yaw.cos(),
    );

    // pitch = 0, yaw = 0：攝影機應該在目標正後方（+Z 方向）
    assert!((offset.x - 0.0).abs() < 0.01);
    assert!((offset.y - 0.0).abs() < 0.01);
    assert!((offset.z - 10.0).abs() < 0.01);
}

#[test]
fn camera_offset_with_pitch() {
    let distance: f32 = 10.0;
    let pitch: f32 = std::f32::consts::FRAC_PI_4; // 45 度向上
    let yaw: f32 = 0.0;

    let offset = Vec3::new(
        distance * pitch.cos() * yaw.sin(),
        distance * pitch.sin(),
        distance * pitch.cos() * yaw.cos(),
    );

    // 45 度俯仰時，Y 和 Z 分量應該大致相等
    assert!((offset.x - 0.0).abs() < 0.01);
    assert!((offset.y - 7.07).abs() < 0.1); // 10 * sin(45°) ≈ 7.07
    assert!((offset.z - 7.07).abs() < 0.1); // 10 * cos(45°) ≈ 7.07
}

// ============================================================================
// 動態 FOV 測試
// ============================================================================

#[test]
fn fov_default_values() {
    let settings = CameraSettings::default();
    assert!((settings.base_fov - 70.0).abs() < f32::EPSILON);
    assert!((settings.sprint_fov - 85.0).abs() < f32::EPSILON);
    assert!((settings.aim_fov - 55.0).abs() < f32::EPSILON);
    assert!((settings.current_fov - 70.0).abs() < f32::EPSILON);
}

#[test]
fn fov_sprint_higher_than_base() {
    let settings = CameraSettings::default();
    assert!(settings.sprint_fov > settings.base_fov);
}

#[test]
fn fov_aim_lower_than_base() {
    let settings = CameraSettings::default();
    assert!(settings.aim_fov < settings.base_fov);
}

#[test]
fn fov_lerp_towards_target() {
    use crate::core::FOV_LERP_SPEED;
    let mut current_fov = 70.0_f32;
    let target_fov = 85.0_f32;
    let dt = 1.0 / 60.0; // 60 fps

    let lerp_t = (FOV_LERP_SPEED * dt).min(1.0);
    current_fov += (target_fov - current_fov) * lerp_t;

    // FOV 應已朝 85 移動
    assert!(current_fov > 70.0);
    assert!(current_fov < 85.0);
}

#[test]
fn fov_lerp_converges() {
    use crate::core::FOV_LERP_SPEED;
    let mut current_fov = 70.0_f32;
    let target_fov = 55.0_f32;
    let dt = 1.0 / 60.0;

    // 模擬 120 幀（約 2 秒）
    for _ in 0..120 {
        let lerp_t = (FOV_LERP_SPEED * dt).min(1.0);
        current_fov += (target_fov - current_fov) * lerp_t;
    }

    // 2 秒後 FOV 應非常接近目標
    assert!((current_fov - target_fov).abs() < 0.1);
}

// ============================================================================
// FPS/TPS 視角切換測試
// ============================================================================

#[test]
fn view_mode_default_is_third_person() {
    use crate::core::CameraViewMode;
    let settings = CameraSettings::default();
    assert_eq!(settings.view_mode, CameraViewMode::ThirdPerson);
}

#[test]
fn view_mode_toggle_fps_tps() {
    use crate::core::CameraViewMode;
    let mut settings = CameraSettings::default();

    // TPS → FPS
    settings.view_mode = match settings.view_mode {
        CameraViewMode::FirstPerson => CameraViewMode::ThirdPerson,
        _ => CameraViewMode::FirstPerson,
    };
    assert_eq!(settings.view_mode, CameraViewMode::FirstPerson);

    // FPS → TPS
    settings.view_mode = match settings.view_mode {
        CameraViewMode::FirstPerson => CameraViewMode::ThirdPerson,
        _ => CameraViewMode::FirstPerson,
    };
    assert_eq!(settings.view_mode, CameraViewMode::ThirdPerson);
}

#[test]
fn fps_fov_wider_than_base() {
    let settings = CameraSettings::default();
    assert!(settings.fps_fov > settings.base_fov);
}

#[test]
fn fps_eye_height_reasonable() {
    let settings = CameraSettings::default();
    // 眼睛高度應在 1.5~2.0 之間（人物身高）
    assert!(settings.fps_eye_height >= 1.5);
    assert!(settings.fps_eye_height <= 2.0);
}

#[test]
fn fps_camera_position_at_eye_level() {
    let settings = CameraSettings::default();
    let player_pos = Vec3::new(10.0, 0.0, 5.0);

    // FPS 攝影機應在玩家位置 + 眼睛高度
    let expected_eye = player_pos + Vec3::Y * settings.fps_eye_height;
    assert!((expected_eye.y - settings.fps_eye_height).abs() < 0.01);
}

#[test]
fn fps_look_direction_calculation() {
    // 驗證 FPS 注視方向計算
    let yaw = 0.0_f32;
    let pitch = 0.0_f32;

    let look_dir = Vec3::new(
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );

    // yaw=0, pitch=0 → 應朝 -Z 方向看（Bevy 預設前方）
    assert!((look_dir.x).abs() < 0.01);
    assert!((look_dir.y).abs() < 0.01);
    assert!((look_dir.z + 1.0).abs() < 0.01);
}

#[test]
fn fps_look_direction_with_pitch() {
    let yaw = 0.0_f32;
    let pitch = std::f32::consts::FRAC_PI_4; // 45° 向上

    let look_dir = Vec3::new(
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );

    // 45° 向上看 → Y 應為負值（往下），Z 也減少
    assert!(look_dir.y < 0.0);
    assert!(look_dir.z < 0.0);
}

// ============================================================================
// 車內視角測試
// ============================================================================

#[test]
fn vehicle_interior_fov_default() {
    let settings = CameraSettings::default();
    // 車內 FOV 應介於瞄準和 FPS 之間
    assert!(settings.vehicle_interior_fov > settings.aim_fov);
    assert!(settings.vehicle_interior_fov < settings.fps_fov);
}

#[test]
fn vehicle_interior_yaw_limit_reasonable() {
    let settings = CameraSettings::default();
    // 限制應在 PI/2 ~ PI 之間（90°~180°）
    assert!(settings.vehicle_interior_yaw_limit > std::f32::consts::FRAC_PI_2);
    assert!(settings.vehicle_interior_yaw_limit <= std::f32::consts::PI);
}

#[test]
fn vehicle_interior_yaw_clamping() {
    let mut settings = CameraSettings::default();
    let limit = settings.vehicle_interior_yaw_limit;

    // 超過限制時應被 clamp
    settings.vehicle_interior_yaw = 5.0;
    settings.vehicle_interior_yaw = settings.vehicle_interior_yaw.clamp(-limit, limit);
    assert!((settings.vehicle_interior_yaw - limit).abs() < 0.01);

    settings.vehicle_interior_yaw = -5.0;
    settings.vehicle_interior_yaw = settings.vehicle_interior_yaw.clamp(-limit, limit);
    assert!((settings.vehicle_interior_yaw + limit).abs() < 0.01);
}

#[test]
fn vehicle_interior_default_look_forward() {
    let settings = CameraSettings::default();
    // 初始應朝正前方看
    assert!((settings.vehicle_interior_yaw).abs() < f32::EPSILON);
    assert!((settings.vehicle_interior_pitch).abs() < f32::EPSILON);
}

#[test]
fn vehicle_interior_look_direction_forward() {
    // 車內 yaw=0, pitch=0 時應朝車輛前方 (-Z)
    let yaw = 0.0_f32;
    let pitch = 0.0_f32;

    let look_dir = Vec3::new(
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );

    assert!((look_dir.x).abs() < 0.01);
    assert!((look_dir.y).abs() < 0.01);
    assert!((look_dir.z + 1.0).abs() < 0.01); // -Z 前方
}

#[test]
fn vehicle_interior_look_direction_left() {
    // 車內向左看（yaw 為正值）
    let yaw = std::f32::consts::FRAC_PI_2; // 90° 左轉
    let pitch = 0.0_f32;

    let look_dir = Vec3::new(
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );

    // 應朝 -X 方向看
    assert!(look_dir.x < -0.9);
    assert!((look_dir.y).abs() < 0.01);
    assert!((look_dir.z).abs() < 0.1);
}

#[test]
fn driver_eye_offset_per_vehicle_type() {
    use crate::vehicle::VehicleType;
    use crate::camera::systems::driver_eye_offset;

    let scooter = driver_eye_offset(VehicleType::Scooter);
    let car = driver_eye_offset(VehicleType::Car);
    let bus = driver_eye_offset(VehicleType::Bus);

    // 公車駕駛座最高
    assert!(bus.y > car.y);
    assert!(bus.y > scooter.y);

    // 所有車種眼睛高度 > 0
    assert!(scooter.y > 0.0);
    assert!(car.y > 0.0);
    assert!(bus.y > 0.0);

    // 汽車駕駛座偏左（負 X）
    assert!(car.x < 0.0);
}

#[test]
fn vehicle_interior_auto_exit_on_dismount() {
    use crate::core::CameraViewMode;
    let mut settings = CameraSettings { view_mode: CameraViewMode::VehicleInterior, ..Default::default() };

    // 模擬下車：不在車上時應自動切回 TPS
    let player_in_vehicle = false;
    if !player_in_vehicle && settings.view_mode == CameraViewMode::VehicleInterior {
        settings.view_mode = CameraViewMode::ThirdPerson;
    }

    assert_eq!(settings.view_mode, CameraViewMode::ThirdPerson);
}

// ============================================================================
// 電影模式測試
// ============================================================================

#[test]
fn cinematic_state_defaults() {
    use crate::core::CinematicState;
    let state = CinematicState::default();
    assert!((state.letterbox_progress - 0.0).abs() < f32::EPSILON);
    assert!(state.fly_speed > 0.0);
    assert!(state.fov > 0.0 && state.fov < 120.0);
}

#[test]
fn cinematic_fov_wider_than_aim() {
    use crate::core::CinematicState;
    let settings = CameraSettings::default();
    let cinematic = CinematicState::default();
    assert!(cinematic.fov > settings.aim_fov);
}

#[test]
fn cinematic_toggle_via_c_key() {
    use crate::core::CameraViewMode;
    let mut settings = CameraSettings { view_mode: CameraViewMode::Cinematic, ..Default::default() };

    // TPS → Cinematic (已設定)
    assert_eq!(settings.view_mode, CameraViewMode::Cinematic);

    // Cinematic → TPS
    settings.view_mode = CameraViewMode::ThirdPerson;
    assert_eq!(settings.view_mode, CameraViewMode::ThirdPerson);
}

#[test]
fn letterbox_progress_animation() {
    use crate::core::{CinematicState, LETTERBOX_ANIM_SPEED};

    let mut state = CinematicState::default();
    let dt = 1.0 / 60.0;
    let target = 1.0;

    // 模擬展開動畫
    for _ in 0..60 {
        let speed = LETTERBOX_ANIM_SPEED * dt;
        if state.letterbox_progress < target {
            state.letterbox_progress = (state.letterbox_progress + speed).min(1.0);
        }
    }

    // 1 秒後應接近或達到完全展開
    assert!(state.letterbox_progress > 0.9);
}

#[test]
fn letterbox_progress_retract() {
    use crate::core::{CinematicState, LETTERBOX_ANIM_SPEED};

    let mut state = CinematicState { letterbox_progress: 1.0, ..Default::default() };
    let dt = 1.0 / 60.0;
    let target = 0.0;

    // 模擬收起動畫
    for _ in 0..60 {
        let speed = LETTERBOX_ANIM_SPEED * dt;
        if state.letterbox_progress > target {
            state.letterbox_progress = (state.letterbox_progress - speed).max(0.0);
        }
    }

    assert!(state.letterbox_progress < 0.1);
}

#[test]
fn cinematic_free_camera_movement() {
    // 驗證自由攝影的前進方向計算
    let yaw = 0.0_f32;
    let pitch = 0.0_f32;

    let forward = Vec3::new(
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );

    // yaw=0, pitch=0 → 前方 -Z
    assert!((forward.z + 1.0).abs() < 0.01);

    let right = Vec3::new(-yaw.cos(), 0.0, yaw.sin());
    // yaw=0 → 右方 -X
    assert!((right.x + 1.0).abs() < 0.01);
}

#[test]
fn cinematic_fly_speed_clamp() {
    use crate::core::CinematicState;
    let mut state = CinematicState::default();

    // 速度不可低於 1.0
    state.fly_speed = (state.fly_speed - 100.0).clamp(1.0, 100.0);
    assert!((state.fly_speed - 1.0).abs() < f32::EPSILON);

    // 速度不可高於 100.0
    state.fly_speed = (state.fly_speed + 200.0).clamp(1.0, 100.0);
    assert!((state.fly_speed - 100.0).abs() < f32::EPSILON);
}
