//! 傷害組件 — 敵人血條、受傷指示器、浮動傷害數字

#![allow(dead_code)]

use bevy::prelude::*;
use crate::core::lifetime_fade_alpha;

// ============================================================================
// 敵人 UI 組件
// ============================================================================

/// 敵人血條（世界空間）
#[derive(Component)]
pub struct EnemyHealthBar {
    /// 對應的敵人實體
    pub enemy_entity: Entity,
}

/// 敵人血條填充
#[derive(Component)]
pub struct EnemyHealthBarFill {
    /// 對應的敵人實體
    pub enemy_entity: Entity,
}

/// 敵人血條高光
#[derive(Component)]
pub struct EnemyHealthBarHighlight {
    /// 對應的敵人實體
    pub enemy_entity: Entity,
}

/// 敵人血條外框（用於外發光）
#[derive(Component)]
pub struct EnemyHealthBarGlow {
    /// 對應的敵人實體
    pub enemy_entity: Entity,
}

// ============================================================================
// 受傷指示器組件
// ============================================================================

/// 受傷指示器容器（螢幕邊緣暈影）
#[derive(Component)]
pub struct DamageIndicator;

/// 受傷指示器邊緣（上下左右）
#[derive(Component)]
pub struct DamageIndicatorEdge {
    pub edge: DamageEdge,
}

/// 傷害邊緣方向
#[derive(Clone, Copy)]
pub enum DamageEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// 受傷指示器狀態
#[derive(Resource)]
pub struct DamageIndicatorState {
    /// 當前顯示強度 (0.0 ~ 1.0)
    pub intensity: f32,
    /// 上次受傷時間
    pub last_damage_time: f32,
    /// 傷害來源方向（如果有）
    pub damage_direction: Option<Vec2>,
}

impl Default for DamageIndicatorState {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            last_damage_time: 0.0,
            damage_direction: None,
        }
    }
}

// ============================================================================
// 浮動傷害數字組件
// ============================================================================

/// 浮動傷害數字（世界空間）
/// GTA 5 風格：傷害數字從敵人位置向上飄動並淡出
#[derive(Component)]
pub struct FloatingDamageNumber {
    /// 初始位置
    pub start_position: Vec3,
    /// 傷害數值
    pub damage: f32,
    /// 是否為爆頭
    pub is_headshot: bool,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 初始縮放
    pub initial_scale: f32,
    /// 向上漂浮速度
    pub float_speed: f32,
    /// 水平偏移（避免重疊）
    pub horizontal_offset: f32,
}

impl FloatingDamageNumber {
    /// 建立新實例
    pub fn new(position: Vec3, damage: f32, is_headshot: bool) -> Self {
        Self {
            start_position: position,
            damage,
            is_headshot,
            lifetime: 0.0,
            max_lifetime: 1.2,  // 1.2 秒後消失
            initial_scale: if is_headshot { 1.4 } else { 1.0 },  // 爆頭更大
            float_speed: 2.0,  // 每秒上升 2 米
            horizontal_offset: 0.0,
        }
    }

    /// 設置水平偏移（避免多個數字重疊）
    pub fn with_offset(mut self, offset: f32) -> Self {
        self.horizontal_offset = offset;
        self
    }

    /// 計算當前透明度 (0.0 ~ 1.0)
    pub fn alpha(&self) -> f32 {
        lifetime_fade_alpha(self.lifetime, self.max_lifetime, 0.3)
    }

    /// 計算當前縮放
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        // 彈出效果：開始時放大，然後縮小
        if progress < 0.1 {
            self.initial_scale * (1.0 + progress * 3.0)  // 快速放大
        } else {
            self.initial_scale * (1.3 - progress * 0.3)  // 緩慢縮小
        }
    }

    /// 計算當前 Y 偏移
    pub fn y_offset(&self) -> f32 {
        self.lifetime * self.float_speed
    }
}

/// 浮動傷害數字追蹤器（控制同時顯示的數量）
#[derive(Resource, Default)]
pub struct FloatingDamageTracker {
    /// 當前顯示的傷害數字數量
    pub active_count: usize,
    /// 最大同時顯示數量
    pub max_count: usize,
    /// 上次生成時的偏移方向（用於交替左右）
    pub last_offset_direction: f32,
}

impl FloatingDamageTracker {
    /// 建立新實例
    pub fn new() -> Self {
        Self {
            active_count: 0,
            max_count: 15,  // 最多同時 15 個
            last_offset_direction: 1.0,
        }
    }

    /// 取得下一個偏移方向並切換
    pub fn next_offset(&mut self) -> f32 {
        self.last_offset_direction *= -1.0;
        self.last_offset_direction * 0.3  // 左右偏移 0.3 米
    }
}
