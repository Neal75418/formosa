//! 遊戲執行狀態與系統分組


use bevy::prelude::*;

/// 遊戲執行狀態（用於暫停/選單控制）
#[derive(States, Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum AppState {
    /// 載入資源中（顯示載入畫面，未來可用於 asset 非同步載入）
    Loading,
    #[default]
    InGame,
    Paused,
    Menu,
}

/// 主要系統分組（便於排序與統一 run_if）
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameSet {
    Player,
    Vehicle,
    World,
    Ui,
}

