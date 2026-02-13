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
