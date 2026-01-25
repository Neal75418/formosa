use bevy::prelude::*;

/// 玩家配置資源
#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct PlayerConfig {
    /// 移動配置
    pub movement: PlayerMovementConfig,
    /// 互動配置
    pub interaction: PlayerInteractionConfig,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            movement: PlayerMovementConfig::default(),
            interaction: PlayerInteractionConfig::default(),
        }
    }
}

/// 玩家移動相關配置
#[derive(Clone, Reflect)]
pub struct PlayerMovementConfig {
    /// 旋轉速度（越大越快）
    pub rotation_speed: f32,
    /// 重力加速度
    pub gravity: f32,
    /// 最大下墜速度
    pub max_fall_speed: f32,
}

impl Default for PlayerMovementConfig {
    fn default() -> Self {
        Self {
            rotation_speed: 25.0,
            gravity: 30.0,
            max_fall_speed: 50.0,
        }
    }
}

/// 玩家互動相關配置
#[derive(Clone, Reflect)]
pub struct PlayerInteractionConfig {
    /// 上車最大距離
    pub vehicle_entry_distance: f32,
    /// 射線檢測起始高度（相對玩家位置）
    pub ray_origin_height: f32,
    /// 下車時的地面高度修正
    pub exit_ground_offset: f32,
    /// 目擊者判定範圍
    pub witness_range: f32,
}

impl Default for PlayerInteractionConfig {
    fn default() -> Self {
        Self {
            vehicle_entry_distance: 4.0,
            ray_origin_height: 0.5,
            exit_ground_offset: 0.7,
            witness_range: 20.0,
        }
    }
}
