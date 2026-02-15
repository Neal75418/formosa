//! 暫停選單設置系統

use bevy::prelude::*;

use super::components::{ButtonScaleState, PauseMenu, QuitButton, ResumeButton};
use super::constants::*;
use super::systems::{spawn_full_screen_overlay, spawn_key_hint, spawn_text_child};

/// 生成暫停選單按鈕（帶邊框和縮放狀態）
fn spawn_pause_menu_button(
    parent: &mut ChildSpawnerCommands,
    border_color: Color,
    bg_color: Color,
    text: &str,
    font: &Handle<Font>,
    marker_component: impl Component,
) {
    parent
        .spawn((
            Node {
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(border_color),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|btn_border| {
            btn_border
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(BUTTON_BASE_WIDTH),
                        height: Val::Px(BUTTON_BASE_HEIGHT),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(bg_color),
                    BorderRadius::all(Val::Px(6.0)),
                    marker_component,
                    ButtonScaleState::default(),
                ))
                .with_children(|btn| {
                    spawn_text_child(btn, text, 20.0, Color::WHITE, font);
                });
        });
}

/// 生成暫停標題裝飾豎條
fn spawn_pause_title_bar(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            width: Val::Px(8.0),
            height: Val::Px(28.0),
            ..default()
        },
        BackgroundColor(PAUSE_TITLE_COLOR),
        BorderRadius::all(Val::Px(2.0)),
    ));
}

fn spawn_pause_title_section(menu: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    menu.spawn((Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(5.0),
        margin: UiRect::bottom(Val::Px(10.0)),
        ..default()
    },))
    .with_children(|title_area| {
        // 暫停圖示（用方塊模擬）
        title_area
            .spawn((Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            },))
            .with_children(|icon_row| {
                spawn_pause_title_bar(icon_row);
                spawn_pause_title_bar(icon_row);
            });

        // 標題文字
        title_area.spawn((
            Text::new("遊戲暫停"),
            TextFont {
                font_size: 32.0,
                font: font.clone(),
                ..default()
            },
            TextColor(PAUSE_TITLE_COLOR),
        ));
    });
}

/// 生成選單分隔線
fn spawn_menu_separator(
    menu: &mut ChildSpawnerCommands,
    margin_top: f32,
    margin_bottom: f32,
    color: Color,
) {
    menu.spawn((
        Node {
            width: Val::Px(220.0),
            height: Val::Px(1.0),
            margin: UiRect::new(
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(margin_top),
                Val::Px(margin_bottom),
            ),
            ..default()
        },
        BackgroundColor(color),
    ));
}

/// 設置暫停選單（初始隱藏，GTA 風格毛玻璃效果）
pub(super) fn setup_pause_menu(commands: &mut Commands, font: &Handle<Font>) {
    spawn_full_screen_overlay(
        commands,
        PAUSE_BG_OUTER,
        PauseMenu,
        FlexDirection::Row,
        Val::Px(0.0),
    )
    .with_children(|parent| {
        // 內層毛玻璃效果層
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(PAUSE_BG_INNER),
        ));

        // 面板外發光層
        parent
            .spawn((
                Node {
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(PAUSE_PANEL_GLOW),
                BorderRadius::all(Val::Px(16.0)),
            ))
            .with_children(|glow| {
                // 面板主邊框層
                glow.spawn((
                    Node {
                        padding: UiRect::all(Val::Px(3.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(PAUSE_PANEL_BORDER),
                    BorderColor::all(Color::srgba(0.5, 0.55, 0.6, 0.6)),
                    BorderRadius::all(Val::Px(12.0)),
                ))
                .with_children(|border| {
                    // 面板內邊框層
                    border
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::new(
                                    Val::Px(50.0),
                                    Val::Px(50.0),
                                    Val::Px(35.0),
                                    Val::Px(35.0),
                                ),
                                row_gap: Val::Px(18.0),
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(PAUSE_PANEL_BG),
                            BorderColor::all(PAUSE_PANEL_INNER_BORDER),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|menu| {
                            // 標題區
                            spawn_pause_title_section(menu, font);

                            // 分隔線
                            spawn_menu_separator(
                                menu,
                                5.0,
                                10.0,
                                Color::srgba(0.4, 0.4, 0.45, 0.5),
                            );

                            // 繼續遊戲按鈕（帶邊框）
                            spawn_pause_menu_button(
                                menu,
                                RESUME_BTN_BORDER,
                                RESUME_BTN_NORMAL,
                                "繼續遊戲",
                                font,
                                ResumeButton,
                            );

                            // 退出遊戲按鈕（帶邊框）
                            spawn_pause_menu_button(
                                menu,
                                QUIT_BTN_BORDER,
                                QUIT_BTN_NORMAL,
                                "退出遊戲",
                                font,
                                QuitButton,
                            );

                            // 分隔線
                            spawn_menu_separator(
                                menu,
                                10.0,
                                5.0,
                                Color::srgba(0.3, 0.3, 0.35, 0.4),
                            );

                            // 快捷鍵提示
                            menu.spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(20.0),
                                ..default()
                            },))
                                .with_children(|hint_row| {
                                    spawn_key_hint(hint_row, "ESC", "繼續", 6.0, font);
                                    spawn_key_hint(hint_row, "Q", "退出", 8.0, font);
                                });

                            // 遊戲標題
                            menu.spawn((
                                Text::new("ISLAND RAMPAGE"),
                                TextFont {
                                    font_size: 11.0,
                                    font: font.clone(),
                                    ..default()
                                },
                                TextColor(PAUSE_SUBTITLE_COLOR),
                                Node {
                                    margin: UiRect::top(Val::Px(8.0)),
                                    ..default()
                                },
                            ));
                        });
                });
            });
    });
}
