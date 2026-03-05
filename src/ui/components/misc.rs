//! 雜項組件 — UI 狀態、字體、外送 App、互動提示、天氣 HUD、劇情任務 HUD

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

/// UI 遊戲狀態
#[allow(clippy::struct_excessive_bools)]
#[derive(Resource)]
pub struct UiState {
    pub paused: bool,
    pub show_full_map: bool,
    pub minimap_zoom: f32,       // 小地圖縮放倍率 (0.5 ~ 2.0)
    pub show_delivery_app: bool, // 是否顯示外送 App
    pub show_weapon_wheel: bool, // 是否顯示武器輪盤
    pub show_save_slots: bool,   // 是否顯示存檔槽 UI
    pub show_phone: bool,        // 是否顯示手機 UI
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            paused: false,
            show_full_map: false,
            minimap_zoom: 1.0,
            show_delivery_app: false,
            show_weapon_wheel: false,
            show_save_slots: false,
            show_phone: false,
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

// ============================================================================
// 手機 UI 組件
// ============================================================================

/// 手機 App 分頁
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhoneApp {
    /// 主畫面（App 圖標列表）
    #[default]
    Home,
    /// 聯絡人
    Contacts,
    /// 任務日誌
    MissionLog,
    /// 地圖
    Map,
    /// 設定
    Settings,
    /// 股市
    StockMarket,
    /// 改裝店
    ModShop,
}

impl PhoneApp {
    /// 顯示名稱
    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "主畫面",
            Self::Contacts => "聯絡人",
            Self::MissionLog => "任務日誌",
            Self::Map => "地圖",
            Self::Settings => "設定",
            Self::StockMarket => "股市",
            Self::ModShop => "改裝店",
        }
    }

    /// App 圖標字元（簡易圖示）
    pub fn icon(self) -> &'static str {
        match self {
            Self::Home => "H",
            Self::Contacts => "C",
            Self::MissionLog => "M",
            Self::Map => "G",
            Self::Settings => "S",
            Self::StockMarket => "$",
            Self::ModShop => "W",
        }
    }

    /// 所有 App（不含 Home）
    pub fn all_apps() -> &'static [PhoneApp] {
        &[
            PhoneApp::Contacts,
            PhoneApp::MissionLog,
            PhoneApp::Map,
            PhoneApp::Settings,
            PhoneApp::StockMarket,
            PhoneApp::ModShop,
        ]
    }
}

/// 任務日誌分頁
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissionJournalTab {
    /// 進行中
    #[default]
    Active,
    /// 已完成
    Completed,
    /// 統計
    Stats,
}

impl MissionJournalTab {
    /// 顯示名稱
    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "進行中",
            Self::Completed => "已完成",
            Self::Stats => "統計",
        }
    }

    /// 所有分頁
    pub fn all() -> &'static [MissionJournalTab] {
        &[Self::Active, Self::Completed, Self::Stats]
    }
}

/// 股市分頁
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StockMarketTab {
    /// 行情列表
    #[default]
    StockList,
    /// 我的持倉
    Portfolio,
    /// 交易
    Trade,
}

impl StockMarketTab {
    /// 顯示名稱
    pub fn label(self) -> &'static str {
        match self {
            Self::StockList => "行情",
            Self::Portfolio => "持倉",
            Self::Trade => "交易",
        }
    }

    /// 所有分頁
    pub fn all() -> &'static [StockMarketTab] {
        &[Self::StockList, Self::Portfolio, Self::Trade]
    }
}

/// 手機 UI 狀態資源
#[derive(Resource)]
pub struct PhoneUiState {
    /// 當前開啟的 App
    pub current_app: PhoneApp,
    /// 主畫面選中的 App 索引
    pub selected_index: usize,
    /// 任務日誌當前分頁
    pub journal_tab: MissionJournalTab,
    /// 股市當前分頁
    pub stock_tab: StockMarketTab,
    /// 選中的股票索引（0-5）
    pub selected_stock_index: usize,
    /// 交易數量
    pub trade_quantity: u32,
    /// 剛從行情頁切到交易頁（防止同幀誤觸買入）
    pub trade_enter_cooldown: bool,
}

impl Default for PhoneUiState {
    fn default() -> Self {
        Self {
            current_app: PhoneApp::Home,
            selected_index: 0,
            journal_tab: MissionJournalTab::default(),
            stock_tab: StockMarketTab::default(),
            selected_stock_index: 0,
            trade_quantity: 1,
            trade_enter_cooldown: false,
        }
    }
}

/// 手機外框容器
#[derive(Component)]
pub struct PhoneContainer;

/// 手機螢幕區域
#[derive(Component)]
pub struct PhoneScreen;

/// 手機 App 圖標按鈕
#[derive(Component)]
pub struct PhoneAppIcon {
    pub app: PhoneApp,
}

/// 手機內容區域（各 App 內容）
#[derive(Component)]
pub struct PhoneContentArea;

/// 手機頂部狀態列
#[derive(Component)]
pub struct PhoneStatusBar;

/// 手機聯絡人列表容器
#[derive(Component)]
pub struct PhoneContactList;

/// 手機任務日誌容器
#[derive(Component)]
pub struct PhoneMissionLogList;

/// 手機股市列表容器
#[derive(Component)]
pub struct PhoneStockMarketList;
