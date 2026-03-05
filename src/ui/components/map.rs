//! 地圖組件 — 小地圖、大地圖、GPS 導航

use bevy::prelude::*;

/// 小地圖容器
#[derive(Component)]
pub struct MinimapContainer;

/// 小地圖玩家標記
#[derive(Component)]
pub struct MinimapPlayerMarker;

/// 大地圖容器
#[derive(Component)]
pub struct FullMapContainer;

/// 大地圖玩家標記
#[derive(Component)]
pub struct FullMapPlayerMarker;

/// 小地圖掃描線
#[derive(Component)]
pub struct MinimapScanLine;

/// 小地圖玩家標記外發光
#[derive(Component)]
pub struct MinimapPlayerGlow;

// ============================================================================
// GPS 導航系統組件 (GTA5 風格)
// ============================================================================

/// GPS 導航狀態資源
#[derive(Resource, Default)]
pub struct GpsNavigationState {
    /// 是否啟用導航
    pub active: bool,
    /// 目標位置（3D 世界座標）
    pub destination: Option<Vec3>,
    /// 目標名稱（顯示用）
    pub destination_name: String,
    /// 計算出的路線點
    pub route_waypoints: Vec<Vec3>,
    /// 到目標的預估距離
    pub distance_to_target: f32,
    /// 下一個轉彎點
    pub next_turn_point: Option<Vec3>,
    /// 路線計算冷卻（避免每幀重算）
    pub route_recalc_cooldown: f32,
    /// 下一個轉彎方向
    pub next_turn_direction: GpsTurnDirection,
    /// 到下一個轉彎的距離
    pub distance_to_next_turn: f32,
}

/// GPS 轉彎方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpsTurnDirection {
    /// 直行（無明顯轉彎）
    #[default]
    Straight,
    /// 左轉
    Left,
    /// 右轉
    Right,
    /// 迴轉
    UTurn,
    /// 到達目的地
    Arrived,
}

impl GpsNavigationState {
    /// 設置新目標
    pub fn set_destination(&mut self, position: Vec3, name: &str) {
        self.active = true;
        self.destination = Some(position);
        self.destination_name = name.to_string();
        self.route_waypoints.clear();
        self.route_recalc_cooldown = 0.0;
    }

    /// 清除導航
    pub fn clear(&mut self) {
        self.active = false;
        self.destination = None;
        self.destination_name.clear();
        self.route_waypoints.clear();
        self.distance_to_target = 0.0;
        self.next_turn_point = None;
        self.next_turn_direction = GpsTurnDirection::Straight;
        self.distance_to_next_turn = 0.0;
    }

    /// 檢查是否到達目標
    pub fn is_at_destination(&self, current_pos: Vec3, threshold: f32) -> bool {
        if let Some(dest) = self.destination {
            let dist_xz =
                ((current_pos.x - dest.x).powi(2) + (current_pos.z - dest.z).powi(2)).sqrt();
            dist_xz < threshold
        } else {
            false
        }
    }
}

impl GpsTurnDirection {
    /// 轉彎方向的顯示符號
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Straight => "↑",
            Self::Left => "←",
            Self::Right => "→",
            Self::UTurn => "↓",
            Self::Arrived => "●",
        }
    }

    /// 轉彎方向的描述文字
    pub fn label(self) -> &'static str {
        match self {
            Self::Straight => "直行",
            Self::Left => "左轉",
            Self::Right => "右轉",
            Self::UTurn => "迴轉",
            Self::Arrived => "到達",
        }
    }

    /// 從角度偏移（弧度）判斷轉彎方向
    /// 正值=右轉，負值=左轉
    pub fn from_angle(angle: f32) -> Self {
        let abs = angle.abs();
        if abs < 0.4 {
            Self::Straight
        } else if abs > 2.5 {
            Self::UTurn
        } else if angle > 0.0 {
            Self::Right
        } else {
            Self::Left
        }
    }
}

/// GPS 轉彎提示 UI
#[derive(Component)]
pub struct GpsTurnIndicator;

/// 小地圖上的 GPS 目標標記
#[derive(Component)]
pub struct MinimapGpsMarker;

/// 小地圖上的 GPS 路線段
#[allow(dead_code)]
#[derive(Component)]
pub struct MinimapGpsRouteLine {
    pub segment_index: usize,
}

/// 屏幕上的方向指示箭頭
#[derive(Component)]
pub struct GpsDirectionArrow;

/// 屏幕上的距離顯示
#[derive(Component)]
pub struct GpsDistanceDisplay;
