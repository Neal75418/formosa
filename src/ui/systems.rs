//! UI 系統 - 共用輔助函數與主設置入口
//!
//! 各分區設置已移至 setup_hud、setup_map、setup_menu 子模組

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use super::components::ChineseFont;
use super::constants::*;

// ============================================================================
// 共用輔助函數（供 setup_hud / setup_map / setup_menu 使用）
// ============================================================================

/// 生成狀態條填充（填充層+高光層）
pub(super) fn spawn_status_bar_fill(
    parent: &mut ChildSpawnerCommands,
    fill_color: Color,
    highlight_color: Color,
    component: impl Component,
    width: Val,
) {
    // 填充層
    parent.spawn((
        Node {
            width,
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(fill_color),
        BorderRadius::all(Val::Px(4.0)),
        component,
    ));
    // 高光層
    parent.spawn((
        Node {
            width,
            height: Val::Px(6.0),
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(highlight_color),
        BorderRadius::top(Val::Px(4.0)),
    ));
}

/// 生成狀態條數值標籤（帶陰影）
pub(super) fn spawn_status_bar_label(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    font: &Handle<Font>,
    font_size: f32,
    shadow_component: impl Component,
    label_component: impl Component,
) {
    parent
        .spawn((Node { ..default() },))
        .with_children(|label_container| {
            // 陰影層
            label_container.spawn((
                Text::new(text),
                TextFont {
                    font_size,
                    font: font.clone(),
                    ..default()
                },
                TextColor(TEXT_SHADOW_COLOR),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(TEXT_SHADOW_OFFSET),
                    top: Val::Px(TEXT_SHADOW_OFFSET),
                    ..default()
                },
                shadow_component,
            ));
            // 主文字
            label_container.spawn((
                Text::new(text),
                TextFont {
                    font_size,
                    font: font.clone(),
                    ..default()
                },
                TextColor(Color::WHITE),
                label_component,
            ));
        });
}

/// 生成方位標示
pub(super) fn spawn_compass_marker(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    font: &Handle<Font>,
    font_size: f32,
    color: Color,
    bg_size: f32,
    position: (Val, Val, Val, Val), // Top, Bottom, Left, Right
) {
    let (top, bottom, left, right) = position;
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top,
                bottom,
                left,
                right,
                width: Val::Px(bg_size),
                height: Val::Px(bg_size),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(COMPASS_BG),
            BorderRadius::all(Val::Px(bg_size / 2.0)),
        ))
        .with_children(|bg| {
            bg.spawn((
                Text::new(text),
                TextFont {
                    font_size,
                    font: font.clone(),
                    ..default()
                },
                TextColor(color),
            ));
        });
}

/// 生成全螢幕覆蓋層（初始隱藏，用於暫停選單/大地圖）
pub(super) fn spawn_full_screen_overlay<'a>(
    commands: &'a mut Commands,
    bg_color: Color,
    component: impl Component,
    flex_direction: FlexDirection,
    row_gap: Val,
) -> EntityCommands<'a> {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction,
            row_gap,
            ..default()
        },
        BackgroundColor(bg_color),
        Visibility::Hidden,
        component,
    ))
}

// ============================================================================
// 主設置入口
// ============================================================================

/// 設置 UI（使用中文字體）- 協調各分區設置函數
pub fn setup_ui(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    super::setup_hud::setup_player_status_hud(&mut commands, &font);
    super::setup_map::setup_minimap_hud(&mut commands, &font);
    super::setup_hud::setup_info_displays(&mut commands, &font);
    super::setup_hud::setup_control_hints(&mut commands, &font);
    super::setup_menu::setup_pause_menu(&mut commands, &font);
    super::setup_map::setup_full_map(&mut commands, &font);
}
