//! 數學工具（避免 NaN 與數值誤差）

use bevy::prelude::Vec3;

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

