//! 玩家設定常數與參數

use bevy::prelude::*;

/// 玩家配置資源
#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
#[derive(Default)]
pub struct PlayerConfig {
    /// 移動配置
    pub movement: PlayerMovementConfig,
    /// 互動配置
    pub interaction: PlayerInteractionConfig,
}


/// 玩家移動相關配置
#[derive(Clone, Reflect)]
pub struct PlayerMovementConfig {
    /// 旋轉速度（越大越快）- 舊版固定值，保留向後兼容
    pub rotation_speed: f32,
    /// 重力加速度
    pub gravity: f32,
    /// 最大下墜速度
    pub max_fall_speed: f32,
    /// 走路時轉向速度
    pub turn_speed_walk: f32,
    /// 衝刺時轉向速度（較慢，高速時不易急轉）
    pub turn_speed_sprint: f32,
}

impl Default for PlayerMovementConfig {
    fn default() -> Self {
        Self {
            rotation_speed: 25.0,
            gravity: 30.0,
            max_fall_speed: 50.0,
            turn_speed_walk: 30.0,   // 走路時靈活轉向
            turn_speed_sprint: 15.0, // 衝刺時轉向較慢（更真實）
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
