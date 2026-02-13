//! 行人動畫組件

use bevy::prelude::*;

/// 行人腿部標記（用於行走動畫）
#[derive(Component)]
pub struct PedestrianLeg {
    /// 是左腿還是右腿
    pub is_left: bool,
}

/// 行人手臂標記（用於行走動畫）
#[derive(Component)]
pub struct PedestrianArm {
    /// 是左手還是右手
    pub is_left: bool,
}

/// 行走動畫狀態
#[derive(Component, Default)]
pub struct WalkingAnimation {
    /// 動畫週期計時器
    pub phase: f32,
    /// 動畫速度（與移動速度關聯）
    pub speed: f32,
}
