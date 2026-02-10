//! 雜項組件 — UI 狀態、字體、外送 App、互動提示、天氣 HUD、劇情任務 HUD

#![allow(dead_code)]

use bevy::prelude::*;

/// UI 遊戲狀態
#[derive(Resource)]
pub struct UiState {
    pub paused: bool,
    pub show_full_map: bool,
    pub minimap_zoom: f32,       // 小地圖縮放倍率 (0.5 ~ 2.0)
    pub show_delivery_app: bool, // 是否顯示外送 App
    pub show_weapon_wheel: bool, // 是否顯示武器輪盤
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            paused: false,
            show_full_map: false,
            minimap_zoom: 1.0,
            show_delivery_app: false,
            show_weapon_wheel: false,
        }
    }
}

/// 中文字體資源
#[derive(Resource)]
pub struct ChineseFont {
    pub font: Handle<Font>,
}

// ============================================================================
// 外送 App UI 組件
// ============================================================================

/// 外送 App 容器
#[derive(Component)]
pub struct DeliveryAppContainer;

/// 外送訂單卡片
#[derive(Component)]
pub struct DeliveryOrderCard {
    pub order_index: usize,
}

/// 外送訂單列表容器
#[derive(Component)]
pub struct DeliveryOrderList;

/// 外送狀態面板（進行中的訂單）
#[derive(Component)]
pub struct DeliveryStatusPanel;

/// 外送評價顯示
#[derive(Component)]
pub struct DeliveryRatingDisplay;

/// 外送連擊顯示
#[derive(Component)]
pub struct DeliveryStreakDisplay;

/// 文字陰影標記（用於陰影層）
#[derive(Component)]
pub struct TextShadowLayer;

// ============================================================================
// 互動提示 UI 組件
// ============================================================================

/// 互動提示容器（螢幕中央偏下）
#[derive(Component)]
pub struct InteractionPromptContainer;

/// 互動提示按鍵框
#[derive(Component)]
pub struct InteractionPromptKey;

/// 互動提示文字
#[derive(Component)]
pub struct InteractionPromptText;

/// 互動提示狀態資源
#[derive(Resource)]
pub struct InteractionPromptState {
    /// 是否顯示提示
    pub visible: bool,
    /// 當前提示文字
    pub text: String,
    /// 按鍵提示
    pub key: String,
    /// 淡入淡出進度 (0.0 ~ 1.0)
    pub fade_progress: f32,
    /// 目標可見度
    pub target_visibility: f32,
}

impl Default for InteractionPromptState {
    fn default() -> Self {
        Self {
            visible: false,
            text: String::new(),
            key: "F".to_string(),
            fade_progress: 0.0,
            target_visibility: 0.0,
        }
    }
}

impl InteractionPromptState {
    /// 顯示提示
    pub fn show(&mut self, text: impl Into<String>, key: impl Into<String>) {
        self.visible = true;
        self.text = text.into();
        self.key = key.into();
        self.target_visibility = 1.0;
    }

    /// 隱藏提示
    pub fn hide(&mut self) {
        self.target_visibility = 0.0;
    }

    /// 更新淡入淡出
    pub fn update(&mut self, delta: f32) {
        let speed = 8.0;
        if self.fade_progress < self.target_visibility {
            self.fade_progress = (self.fade_progress + speed * delta).min(self.target_visibility);
        } else if self.fade_progress > self.target_visibility {
            self.fade_progress = (self.fade_progress - speed * delta).max(self.target_visibility);
        }

        // 完全淡出後設為不可見
        if self.fade_progress <= 0.01 && self.target_visibility <= 0.0 {
            self.visible = false;
        }
    }
}

// ============================================================================
// 天氣 HUD 組件
// ============================================================================

/// 天氣 HUD 容器
#[derive(Component)]
pub struct WeatherHudContainer;

/// 天氣圖示容器
#[derive(Component)]
pub struct WeatherIconContainer;

/// 天氣圖示元素（根據類型顯示/隱藏）
#[derive(Component)]
pub struct WeatherIconElement {
    pub weather_type: WeatherIconType,
}

/// 天氣圖示類型
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WeatherIconType {
    Sun,
    Cloud,
    Rain,
    Fog,
}

/// 太陽光芒（動畫用）
#[derive(Component)]
pub struct SunRay {
    pub index: usize,
}

/// 雨滴圖示（動畫用）
#[derive(Component)]
pub struct RainDropIcon {
    pub index: usize,
    pub offset: f32,
}

/// 天氣名稱文字
#[derive(Component)]
pub struct WeatherNameText;

// ============================================================================
// 劇情任務 HUD 組件 (GTA 5 風格)
// ============================================================================

/// 劇情任務 HUD 容器（右上角）
#[derive(Component)]
pub struct StoryMissionHud;

/// 任務標題文字
#[derive(Component)]
pub struct StoryMissionTitle;

/// 任務階段描述文字
#[derive(Component)]
pub struct StoryMissionPhaseText;

/// 任務目標列表容器
#[derive(Component)]
pub struct StoryMissionObjectiveList;

/// 任務目標項目
#[derive(Component)]
pub struct StoryMissionObjectiveItem {
    /// 目標索引
    pub index: usize,
}

/// 任務目標勾選框
#[derive(Component)]
pub struct StoryMissionObjectiveCheck {
    pub index: usize,
}

/// 任務目標文字
#[derive(Component)]
pub struct StoryMissionObjectiveText {
    pub index: usize,
}

/// 任務計時器顯示
#[derive(Component)]
pub struct StoryMissionTimer;

/// 任務進度條背景
#[derive(Component)]
pub struct StoryMissionProgressBg;

/// 任務進度條填充
#[derive(Component)]
pub struct StoryMissionProgressFill;
