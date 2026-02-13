//! 行人碰撞組件

use bevy::prelude::*;

/// 行人被車撞標記
#[derive(Component)]
pub struct HitByVehicle {
    /// 撞擊方向
    pub impact_direction: Vec3,
    /// 撞擊力度
    pub impact_force: f32,
    /// 撞擊時間
    pub hit_time: f32,
}
