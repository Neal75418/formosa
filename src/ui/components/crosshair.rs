//! 準星組件 — 準星 UI、命中標記、動態散佈

use bevy::prelude::*;

/// 準星容器
#[derive(Component)]
pub struct Crosshair;

/// 準星中心點
#[derive(Component)]
pub struct CrosshairDot;

/// 準星線條
#[derive(Component)]
pub struct CrosshairLine {
    pub direction: CrosshairDirection,
}

/// 準星線條方向
#[derive(Clone, Copy)]
pub enum CrosshairDirection {
    Top,
    Bottom,
    Left,
    Right,
}

/// 準星外圈（GTA 風格）
#[derive(Component)]
pub struct CrosshairOuterRing;

/// 準星命中標記（X 形）
#[derive(Component)]
pub struct CrosshairHitMarker;

/// 準星命中標記線條
#[derive(Component)]
pub struct HitMarkerLine;

/// 準星動態狀態資源
#[derive(Resource)]
pub struct CrosshairDynamics {
    /// 當前散佈值 (射擊時增加，恢復時減少)
    pub current_spread: f32,
    /// 目標散佈值 (瞄準時較小)
    pub target_spread: f32,
    /// 命中反彈縮放
    pub hit_bounce_scale: f32,
}

impl Default for CrosshairDynamics {
    fn default() -> Self {
        Self {
            current_spread: 1.0,
            target_spread: 1.0,
            hit_bounce_scale: 1.0,
        }
    }
}
