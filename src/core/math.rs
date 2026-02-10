//! 數學工具（避免 NaN 與數值誤差）

use bevy::prelude::Vec3;

/// 將 bevy_rapier3d 的 Real 類型轉換為 f32
/// bevy_rapier3d 0.32 的 Real 目前就是 f32，但使用明確轉換確保未來版本兼容性
#[inline]
pub fn rapier_real_to_f32(r: bevy_rapier3d::prelude::Real) -> f32 {
    r
}

/// clamp dot 以避免 acos 產生 NaN
#[inline]
pub fn clamp_dot(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}

/// 安全 normalize：向量過短時回傳零向量
#[inline]
pub fn safe_normalize(value: Vec3) -> Vec3 {
    const EPS_SQ: f32 = 1.0e-6;
    let len_sq = value.length_squared();
    if len_sq > EPS_SQ {
        value / len_sq.sqrt()
    } else {
        Vec3::ZERO
    }
}

/// 判斷兩點是否在指定距離內（使用距離平方避免 sqrt）
#[inline]
#[allow(dead_code)]
pub fn is_within_range(a: Vec3, b: Vec3, range: f32) -> bool {
    a.distance_squared(b) <= range * range
}

use bevy::prelude::Quat;

/// 計算 Y 軸的朝向旋轉（僅考慮 XZ 平面）
/// 如果方向向量過短，則返回 Identity
#[inline]
pub fn look_rotation_y_flat(dir: Vec3) -> Quat {
    if dir.length_squared() > 0.001 {
        Quat::from_rotation_y((-dir.x).atan2(-dir.z))
    } else {
        Quat::IDENTITY
    }
}
