//! 選單組件 — 暫停選單、任務失敗/結果 UI

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

/// 暫停選單
#[derive(Component)]
pub struct PauseMenu;

/// 繼續遊戲按鈕
#[derive(Component)]
pub struct ResumeButton;

/// 退出遊戲按鈕
#[derive(Component)]
pub struct QuitButton;

/// 按鈕縮放動畫狀態
#[derive(Component)]
pub struct ButtonScaleState {
    /// 目標縮放值
    pub target: f32,
    /// 當前縮放值
    pub current: f32,
}

impl Default for ButtonScaleState {
    fn default() -> Self {
        Self {
            target: 1.0,
            current: 1.0,
        }
    }
}

// ============================================================================
// 任務失敗/結果 UI
// ============================================================================

/// 任務失敗 UI 狀態
#[derive(Resource, Default)]
pub struct MissionFailState {
    /// 是否顯示失敗畫面
    pub is_showing: bool,
    /// 失敗原因
    pub fail_reason: Option<String>,
    /// 是否可從檢查點重試
    pub can_retry: bool,
    /// 當前選項（0 = 重試, 1 = 放棄）
    pub selected_option: usize,
    /// 顯示計時器（用於淡入動畫）
    pub show_timer: f32,
}

impl MissionFailState {
    /// 顯示
    pub fn show(&mut self, reason: String, can_retry: bool) {
        self.is_showing = true;
        self.fail_reason = Some(reason);
        self.can_retry = can_retry;
        self.selected_option = 0;
        self.show_timer = 0.0;
    }

    /// 隱藏
    pub fn hide(&mut self) {
        self.is_showing = false;
        self.fail_reason = None;
        self.can_retry = false;
        self.show_timer = 0.0;
    }
}

/// 任務完成結果 UI 狀態
#[derive(Resource, Default)]
pub struct MissionResultState {
    /// 是否顯示結果畫面
    pub is_showing: bool,
    /// 顯示計時器（用於動畫）
    pub show_timer: f32,
    /// 是否已確認（按下任意鍵後）
    pub confirmed: bool,
}

impl MissionResultState {
    /// 顯示
    pub fn show(&mut self) {
        self.is_showing = true;
        self.show_timer = 0.0;
        self.confirmed = false;
    }

    /// 隱藏
    pub fn hide(&mut self) {
        self.is_showing = false;
        self.show_timer = 0.0;
        self.confirmed = false;
    }
}

/// 任務失敗 UI 容器標記
#[derive(Component)]
pub struct MissionFailUI;

/// 任務失敗標題
#[derive(Component)]
pub struct MissionFailTitle;

/// 任務失敗原因文字
#[derive(Component)]
pub struct MissionFailReason;

/// 任務失敗選項（重試/放棄）
#[derive(Component)]
pub struct MissionFailOption {
    pub index: usize,
}

/// 任務結果 UI 容器標記
#[derive(Component)]
pub struct MissionResultUI;

/// 任務結果標題
#[derive(Component)]
pub struct MissionResultTitle;

/// 任務結果評分星星
#[derive(Component)]
pub struct MissionResultStars;

/// 任務結果統計
#[derive(Component)]
pub struct MissionResultStats;

/// 任務結果獎勵
#[derive(Component)]
pub struct MissionResultReward;
