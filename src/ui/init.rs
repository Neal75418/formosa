//! UI 核心初始化系統
//!
//! 包含字體載入和 UI 縮放設定

use bevy::prelude::*;
use bevy::ui::UiScale;
use bevy::window::WindowResized;

use super::components::ChineseFont;
use super::constants::BASE_RESOLUTION_HEIGHT;

/// 載入中文字體
pub fn setup_chinese_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/STHeiti.ttc");
    commands.insert_resource(ChineseFont { font });
}

/// 初始化 UI 縮放（根據視窗大小等比縮放）
pub fn setup_ui_scale(mut ui_scale: ResMut<UiScale>, windows: Query<&Window>) {
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.height() / BASE_RESOLUTION_HEIGHT;
    ui_scale.0 = scale;
    info!(
        "📐 UI Scale 初始化: {:.2} (視窗: {}x{})",
        scale,
        window.width(),
        window.height()
    );
}

/// 動態更新 UI 縮放（視窗大小改變時）
pub fn update_ui_scale(mut resize_events: MessageReader<WindowResized>, mut ui_scale: ResMut<UiScale>) {
    for event in resize_events.read() {
        let scale = event.height / BASE_RESOLUTION_HEIGHT;
        ui_scale.0 = scale;
        info!(
            "📐 UI Scale 更新: {:.2} (視窗: {}x{})",
            scale, event.width, event.height
        );
    }
}
