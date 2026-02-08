//! UI 系統 - 主要 UI 設置
//!
//! 包含：setup_ui 主函數（GTA 風格 HUD 佈局）

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use super::components::{
    ArmorBarFill, ArmorLabel, ArmorLabelShadow, ArmorSection, ButtonScaleState, ChineseFont,
    ControlHintContainer, ControlKeyArea, ControlSpeedDisplay, ControlStatusTag, FullMapContainer,
    FullMapPlayerMarker, HealthBarBg, HealthBarFill, HealthBarGlow, HealthLabel, HealthLabelShadow,
    MinimapContainer, MinimapPlayerGlow, MinimapPlayerMarker, MinimapScanLine, MissionInfo,
    MoneyDisplay, PauseMenu, PlayerStatusContainer, QuitButton, ResumeButton, TimeDisplay, UiText,
};
use super::constants::*;
use super::minimap::spawn_map_layer;

// ============================================================================
// 輔助函數
// ============================================================================
/// 生成狀態條填充（填充層+高光層）
fn spawn_status_bar_fill(
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
fn spawn_status_bar_label(
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
fn spawn_compass_marker(
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
fn spawn_full_screen_overlay<'a>(
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
                    btn.spawn((
                        Text::new(text),
                        TextFont {
                            font_size: 20.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
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

// ============================================================================
// 分區設置函數
// ============================================================================
/// 設置左下角 GTA 風格玩家狀態區（血量條、護甲條）
fn setup_player_status_hud(commands: &mut Commands, font: &Handle<Font>) {
    // 外發光層
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(56.0), // 留空間給控制提示
                left: Val::Px(16.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(HUD_GLOW_OUTER),
            BorderRadius::all(Val::Px(12.0)),
        ))
        .with_children(|glow| {
            // 主容器
            glow.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(HUD_BG),
                BorderColor::all(HUD_BORDER_HIGHLIGHT),
                BorderRadius::all(Val::Px(8.0)),
                PlayerStatusContainer,
            ))
            .with_children(|parent| {
                // === 血量區塊 ===
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        ..default()
                    },))
                    .with_children(|row| {
                        // 血量圖示（紅色圓角方塊 + 內圈模擬愛心）
                        row.spawn((
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(HEALTH_ICON),
                            BorderRadius::all(Val::Px(4.0)),
                        ))
                        .with_children(|icon| {
                            // 內圈高光
                            icon.spawn((
                                Node {
                                    width: Val::Px(8.0),
                                    height: Val::Px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                                BorderRadius::all(Val::Px(4.0)),
                            ));
                        });

                        // 血量條外發光層（低血量時脈衝）
                        row.spawn((
                            Node {
                                width: Val::Px(186.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                            BorderRadius::all(Val::Px(6.0)),
                            HealthBarGlow,
                        ))
                        .with_children(|glow| {
                            // 血量條容器
                            glow.spawn((
                                Node {
                                    width: Val::Px(180.0),
                                    height: Val::Px(18.0),
                                    ..default()
                                },
                                BackgroundColor(HEALTH_BAR_BG),
                                BorderRadius::all(Val::Px(4.0)),
                                HealthBarBg,
                            ))
                            .with_children(|bar_bg| {
                                spawn_status_bar_fill(
                                    bar_bg,
                                    HEALTH_BAR_FILL_COLOR,
                                    HEALTH_BAR_HIGHLIGHT_COLOR,
                                    HealthBarFill,
                                    Val::Percent(100.0),
                                );
                            });
                        }); // 結束 glow

                        // 血量數值標籤（帶陰影）
                        spawn_status_bar_label(
                            row,
                            "100/100",
                            font,
                            14.0,
                            HealthLabelShadow,
                            HealthLabel,
                        );
                    });

                // === 護甲區塊（有護甲時才顯示）===
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },
                        Visibility::Hidden, // 預設隱藏
                        ArmorSection,
                    ))
                    .with_children(|row| {
                        // 護甲圖示（藍色圓角方塊 + 盾牌樣式）
                        row.spawn((
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(ARMOR_ICON),
                            BorderColor::all(Color::srgba(0.6, 0.85, 1.0, 0.8)),
                            BorderRadius::all(Val::Px(4.0)),
                        ))
                        .with_children(|icon| {
                            // 內部深色區塊（模擬盾牌）
                            icon.spawn((
                                Node {
                                    width: Val::Px(6.0),
                                    height: Val::Px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.1, 0.3, 0.5, 0.6)),
                                BorderRadius::all(Val::Px(2.0)),
                            ));
                        });

                        // 護甲條容器
                        row.spawn((
                            Node {
                                width: Val::Px(180.0),
                                height: Val::Px(18.0),
                                ..default()
                            },
                            BackgroundColor(ARMOR_BAR_BG),
                            BorderRadius::all(Val::Px(4.0)),
                        ))
                        .with_children(|bar_bg| {
                            spawn_status_bar_fill(
                                bar_bg,
                                ARMOR_BAR_FILL_COLOR,
                                ARMOR_BAR_HIGHLIGHT_COLOR,
                                ArmorBarFill,
                                Val::Percent(50.0),
                            );
                        });

                        // 護甲數值標籤（帶陰影）
                        spawn_status_bar_label(
                            row,
                            "50/100",
                            font,
                            14.0,
                            ArmorLabelShadow,
                            ArmorLabel,
                        );
                    });
            }); // 結束 PlayerStatusContainer
        }); // 結束外發光層
}

/// 設置右上角小地圖（GTA 風格多層邊框）
fn setup_minimap_hud(commands: &mut Commands, font: &Handle<Font>) {
    // 外層發光框
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(316.0), // 300 + 8*2 邊框
                height: Val::Px(316.0),
                padding: UiRect::all(Val::Px(4.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(MINIMAP_GLOW),
            BorderRadius::all(Val::Px(12.0)),
        ))
        .with_children(|glow| {
            // 主邊框層
            glow.spawn((
                Node {
                    width: Val::Px(308.0),
                    height: Val::Px(308.0),
                    padding: UiRect::all(Val::Px(3.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(MINIMAP_BORDER),
                BorderColor::all(MINIMAP_INNER_BORDER),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|frame| {
                // 實際地圖容器
                frame
                    .spawn((
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(300.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(MINIMAP_BG),
                        BorderRadius::all(Val::Px(6.0)),
                        MinimapContainer,
                    ))
                    .with_children(|parent| {
                        // 內層漸層效果（四角較亮模擬）
                        // 左上角高光
                        parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(0.0),
                                left: Val::Px(0.0),
                                width: Val::Px(60.0),
                                height: Val::Px(60.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.2, 0.3, 0.2, 0.15)),
                            BorderRadius::top_left(Val::Px(6.0)),
                        ));
                        // 右上角高光
                        parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(0.0),
                                right: Val::Px(0.0),
                                width: Val::Px(60.0),
                                height: Val::Px(60.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.2, 0.3, 0.2, 0.1)),
                            BorderRadius::top_right(Val::Px(6.0)),
                        ));

                        // 小地圖標題（帶背景）
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(4.0),
                                    left: Val::Px(4.0),
                                    padding: UiRect::new(
                                        Val::Px(6.0),
                                        Val::Px(6.0),
                                        Val::Px(2.0),
                                        Val::Px(2.0),
                                    ),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                                BorderRadius::all(Val::Px(4.0)),
                            ))
                            .with_children(|title_bg| {
                                title_bg.spawn((
                                    Text::new("西門町"),
                                    TextFont {
                                        font_size: 10.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::srgba(0.8, 0.95, 0.8, 0.9)),
                                ));
                            });

                        // === 地圖內容層 ===
                        let mm_scale = 0.9;
                        let mm_off_x = 150.0;
                        let mm_off_y = 150.0;
                        let mw_fac = 0.7;

                        spawn_map_layer(
                            parent,
                            mm_scale,
                            mm_off_x,
                            mm_off_y,
                            mw_fac,
                            false,
                            font.clone(),
                        );

                        // === 雷達掃描線（GTA 風格）===
                        parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Px(3.0),
                                top: Val::Percent(0.0),
                                left: Val::Px(0.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.4, 0.9, 0.5, 0.25)),
                            MinimapScanLine,
                        ));

                        // === 玩家標記（簡潔圓形+箭頭指針）===
                        // 注意：容器需要包含整個箭頭，旋轉才能正確工作
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(20.0),
                                    height: Val::Px(34.0), // 增加高度以包含箭頭
                                    left: Val::Px(140.0),
                                    top: Val::Px(133.0), // 調整位置，讓圓心在地圖中央
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    overflow: Overflow::visible(), // 允許子元素溢出
                                    ..default()
                                },
                                Transform::default(),
                                GlobalTransform::default(),
                                MinimapPlayerMarker,
                            ))
                            .with_children(|marker| {
                                // 淡白色外圈（脈衝動畫用）- 定位在容器下半部
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(18.0),
                                        height: Val::Px(18.0),
                                        left: Val::Px(1.0),
                                        top: Val::Px(15.0), // 在容器下半部
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
                                    BorderRadius::all(Val::Px(9.0)),
                                    MinimapPlayerGlow,
                                ));
                                // 黑色描邊圓
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(14.0),
                                        height: Val::Px(14.0),
                                        left: Val::Px(3.0),
                                        top: Val::Px(17.0),
                                        ..default()
                                    },
                                    BackgroundColor(OVERLAY_BLACK_90),
                                    BorderRadius::all(Val::Px(7.0)),
                                ));
                                // 白色主圓
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(10.0),
                                        height: Val::Px(10.0),
                                        left: Val::Px(5.0),
                                        top: Val::Px(19.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::WHITE),
                                    BorderRadius::all(Val::Px(5.0)),
                                ));
                                // 方向指示三角（黑色描邊）
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(10.0),
                                        height: Val::Px(16.0),
                                        left: Val::Px(5.0),
                                        top: Val::Px(2.0),
                                        ..default()
                                    },
                                    BackgroundColor(OVERLAY_BLACK_90),
                                    BorderRadius::top(Val::Px(5.0)),
                                ));
                                // 方向指示三角（白色內部）
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(6.0),
                                        height: Val::Px(14.0),
                                        left: Val::Px(7.0),
                                        top: Val::Px(4.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::WHITE),
                                    BorderRadius::top(Val::Px(3.0)),
                                ));
                            });

                        // === 方位標示（帶圓角背景）===
                        for (label, color, position) in [
                            ("N", COMPASS_NORTH, (Val::Px(6.0), Val::Auto, Val::Px(140.0), Val::Auto)),
                            ("S", Color::WHITE, (Val::Auto, Val::Px(6.0), Val::Px(140.0), Val::Auto)),
                            ("E", Color::WHITE, (Val::Px(140.0), Val::Auto, Val::Auto, Val::Px(6.0))),
                            ("W", Color::WHITE, (Val::Px(140.0), Val::Auto, Val::Px(6.0), Val::Auto)),
                        ] {
                            spawn_compass_marker(parent, label, font, 13.0, color, 20.0, position);
                        }

                        // === 掃描線效果（模擬雷達感）===
                        for i in 0..6 {
                            parent.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(50.0 * i as f32),
                                    left: Val::Px(0.0),
                                    width: Val::Percent(100.0),
                                    height: Val::Px(1.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.3, 0.5, 0.3, 0.08)),
                            ));
                        }
                    });
            });
        });
}

/// 設置小地圖下方的時間、金錢、任務資訊顯示
fn setup_info_displays(commands: &mut Commands, font: &Handle<Font>) {
    // === 小地圖下方：時間（帶背景）===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(332.0),   // 316 + 6 + 10
                right: Val::Px(110.0), // 對齊小地圖中央
                padding: UiRect::new(Val::Px(10.0), Val::Px(10.0), Val::Px(4.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.05, 0.7)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("18:00"),
                TextFont {
                    font_size: 20.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.95, 0.9, 0.95)),
                TimeDisplay,
            ));
        });

    // === 小地圖下方：金錢顯示（GTA 風格）===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(365.0), // 時間下方
                right: Val::Px(10.0),
                padding: UiRect::new(Val::Px(12.0), Val::Px(12.0), Val::Px(6.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(MONEY_BG),
            BorderColor::all(HUD_BORDER),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("$ 5,000"),
                TextFont {
                    font_size: 28.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(MONEY_TEXT_COLOR),
                MoneyDisplay,
            ));
        });

    // === 任務資訊（小地圖下方） ===
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 16.0,
            font: font.clone(),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.8, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(190.0),
            right: Val::Px(10.0),
            ..default()
        },
        MissionInfo,
    ));
}

/// 設置左下角控制提示（GTA 風格）
fn setup_control_hints(commands: &mut Commands, font: &Handle<Font>) {
    // === 左下角：控制提示（GTA 風格）===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::new(Val::Px(10.0), Val::Px(10.0), Val::Px(6.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(CONTROL_HINT_BG),
            BorderColor::all(CONTROL_HINT_BORDER),
            BorderRadius::all(Val::Px(4.0)),
            ControlHintContainer,
        ))
        .with_children(|parent| {
            // 狀態標籤（步行/駕駛）
            parent
                .spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(8.0),
                            Val::Px(8.0),
                            Val::Px(4.0),
                            Val::Px(4.0),
                        ),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(STATUS_TAG_BG),
                    BorderColor::all(Color::srgba(0.3, 0.6, 0.35, 0.8)),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|tag| {
                    tag.spawn((
                        Text::new("步行"),
                        TextFont {
                            font_size: 12.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(Color::srgb(0.85, 0.95, 0.85)),
                        ControlStatusTag,
                    ));
                });

            // 速度顯示（駕駛時顯示）
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(SPEED_TEXT_COLOR),
                ControlSpeedDisplay,
                Visibility::Hidden,
            ));

            // 按鍵提示區域
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(4.0),
                        ..default()
                    },
                    ControlKeyArea,
                ))
                .with_children(|keys| {
                    // 按鍵圖示 helper
                    let spawn_key = |keys: &mut ChildSpawnerCommands,
                                     key: &str,
                                     action: &str,
                                     font: Handle<Font>| {
                        // 按鍵背景
                        keys.spawn((
                            Node {
                                padding: UiRect::new(
                                    Val::Px(6.0),
                                    Val::Px(6.0),
                                    Val::Px(3.0),
                                    Val::Px(3.0),
                                ),
                                border: UiRect::all(Val::Px(1.0)),
                                min_width: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(KEY_ICON_BG),
                            BorderColor::all(KEY_ICON_BORDER),
                            BorderRadius::all(Val::Px(3.0)),
                        ))
                        .with_children(|key_bg| {
                            key_bg.spawn((
                                Text::new(key),
                                TextFont {
                                    font_size: 11.0,
                                    font: font.clone(),
                                    ..default()
                                },
                                TextColor(KEY_TEXT_COLOR),
                            ));
                        });
                        // 動作說明
                        keys.spawn((
                            Text::new(action),
                            TextFont {
                                font_size: 11.0,
                                font: font.clone(),
                                ..default()
                            },
                            TextColor(ACTION_TEXT_COLOR),
                            Node {
                                margin: UiRect::right(Val::Px(6.0)),
                                ..default()
                            },
                        ));
                    };

                    spawn_key(keys, "WASD", "移動", font.clone());
                    spawn_key(keys, "R", "射擊", font.clone());
                    spawn_key(keys, "1-4", "武器", font.clone());
                    spawn_key(keys, "ESC", "暫停", font.clone());
                });
        });

    // 舊版簡單文字提示（保留作為備用更新目標）
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            font: font.clone(),
            ..default()
        },
        TextColor(Color::NONE),
        Node {
            display: Display::None,
            ..default()
        },
        UiText,
    ));
}

/// 設置暫停選單（初始隱藏，GTA 風格毛玻璃效果）
fn setup_pause_menu(commands: &mut Commands, font: &Handle<Font>) {
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
                                            // 左豎條
                                            spawn_pause_title_bar(icon_row);
                                            // 右豎條
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

                            // 分隔線
                            menu.spawn((
                                Node {
                                    width: Val::Px(220.0),
                                    height: Val::Px(1.0),
                                    margin: UiRect::new(
                                        Val::Px(0.0),
                                        Val::Px(0.0),
                                        Val::Px(5.0),
                                        Val::Px(10.0),
                                    ),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.4, 0.4, 0.45, 0.5)),
                            ));

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
                            menu.spawn((
                                Node {
                                    width: Val::Px(220.0),
                                    height: Val::Px(1.0),
                                    margin: UiRect::new(
                                        Val::Px(0.0),
                                        Val::Px(0.0),
                                        Val::Px(10.0),
                                        Val::Px(5.0),
                                    ),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.3, 0.3, 0.35, 0.4)),
                            ));

                            // 快捷鍵提示
                            menu.spawn((Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(20.0),
                                ..default()
                            },))
                                .with_children(|hint_row| {
                                    // ESC 提示
                                    hint_row
                                        .spawn((Node {
                                            flex_direction: FlexDirection::Row,
                                            align_items: AlignItems::Center,
                                            column_gap: Val::Px(6.0),
                                            ..default()
                                        },))
                                        .with_children(|hint| {
                                            // ESC 按鍵框
                                            hint.spawn((
                                                Node {
                                                    padding: UiRect::new(
                                                        Val::Px(6.0),
                                                        Val::Px(6.0),
                                                        Val::Px(3.0),
                                                        Val::Px(3.0),
                                                    ),
                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                BackgroundColor(BUTTON_BG_DARK),
                                                BorderColor::all(BUTTON_BORDER_GRAY_70),
                                                BorderRadius::all(Val::Px(4.0)),
                                            ))
                                            .with_children(|key| {
                                                key.spawn((
                                                    Text::new("ESC"),
                                                    TextFont {
                                                        font_size: 11.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                    TextColor(TEXT_LIGHT_GRAY),
                                                ));
                                            });
                                            hint.spawn((
                                                Text::new("繼續"),
                                                TextFont {
                                                    font_size: 13.0,
                                                    font: font.clone(),
                                                    ..default()
                                                },
                                                TextColor(PAUSE_HINT_COLOR),
                                            ));
                                        });

                                    // Q 提示
                                    hint_row
                                        .spawn((Node {
                                            flex_direction: FlexDirection::Row,
                                            align_items: AlignItems::Center,
                                            column_gap: Val::Px(6.0),
                                            ..default()
                                        },))
                                        .with_children(|hint| {
                                            // Q 按鍵框
                                            hint.spawn((
                                                Node {
                                                    padding: UiRect::new(
                                                        Val::Px(8.0),
                                                        Val::Px(8.0),
                                                        Val::Px(3.0),
                                                        Val::Px(3.0),
                                                    ),
                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                BackgroundColor(BUTTON_BG_DARK),
                                                BorderColor::all(BUTTON_BORDER_GRAY_70),
                                                BorderRadius::all(Val::Px(4.0)),
                                            ))
                                            .with_children(|key| {
                                                key.spawn((
                                                    Text::new("Q"),
                                                    TextFont {
                                                        font_size: 11.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                    TextColor(TEXT_LIGHT_GRAY),
                                                ));
                                            });
                                            hint.spawn((
                                                Text::new("退出"),
                                                TextFont {
                                                    font_size: 13.0,
                                                    font: font.clone(),
                                                    ..default()
                                                },
                                                TextColor(PAUSE_HINT_COLOR),
                                            ));
                                        });
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

/// 設置大地圖（GTA 風格，初始隱藏）
fn setup_full_map(commands: &mut Commands, font: &Handle<Font>) {
    spawn_full_screen_overlay(
        commands,
        FULLMAP_BG,
        FullMapContainer,
        FlexDirection::Column,
        Val::Px(12.0),
    )
    .with_children(|parent| {
        // === 標題區（帶邊框背景）===
        parent
            .spawn((Node {
                position_type: PositionType::Absolute,
                top: Val::Px(25.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },))
            .with_children(|title_row| {
                title_row
                    .spawn((
                        Node {
                            padding: UiRect::new(
                                Val::Px(30.0),
                                Val::Px(30.0),
                                Val::Px(10.0),
                                Val::Px(10.0),
                            ),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(FULLMAP_TITLE_BG),
                        BorderColor::all(FULLMAP_BORDER),
                        BorderRadius::all(Val::Px(8.0)),
                    ))
                    .with_children(|bg| {
                        bg.spawn((
                            Text::new("西門町地圖"),
                            TextFont {
                                font_size: 28.0,
                                font: font.clone(),
                                ..default()
                            },
                            TextColor(Color::srgba(0.9, 0.95, 0.9, 1.0)),
                        ));
                    });
            });

        // === 地圖主體（多層邊框）===
        // 外層發光框
        parent
            .spawn((
                Node {
                    width: Val::Px(1220.0),
                    height: Val::Px(820.0),
                    padding: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.35, 0.2, 0.4)),
                BorderRadius::all(Val::Px(12.0)),
            ))
            .with_children(|glow| {
                // 主邊框層
                glow.spawn((
                    Node {
                        width: Val::Px(1210.0),
                        height: Val::Px(810.0),
                        padding: UiRect::all(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(FULLMAP_BORDER),
                    BorderColor::all(Color::srgba(0.15, 0.2, 0.15, 0.9)),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|frame| {
                    // 實際地圖容器
                    frame
                        .spawn((
                            Node {
                                width: Val::Px(1200.0),
                                height: Val::Px(800.0),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BackgroundColor(FULLMAP_MAIN_BG),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_children(|map| {
                            // 角落高光效果
                            map.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(0.0),
                                    left: Val::Px(0.0),
                                    width: Val::Px(150.0),
                                    height: Val::Px(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.25, 0.35, 0.25, 0.12)),
                                BorderRadius::top_left(Val::Px(6.0)),
                            ));
                            map.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(0.0),
                                    right: Val::Px(0.0),
                                    width: Val::Px(150.0),
                                    height: Val::Px(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.25, 0.35, 0.25, 0.08)),
                                BorderRadius::top_right(Val::Px(6.0)),
                            ));

                            // 網格線（增加地圖質感）
                            for i in 0..9 {
                                // 水平線
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(100.0 * i as f32),
                                        left: Val::Px(0.0),
                                        width: Val::Percent(100.0),
                                        height: Val::Px(1.0),
                                        ..default()
                                    },
                                    BackgroundColor(MAP_AREA_GREEN),
                                ));
                            }
                            for i in 0..13 {
                                // 垂直線
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(0.0),
                                        left: Val::Px(100.0 * i as f32),
                                        width: Val::Px(1.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(MAP_AREA_GREEN),
                                ));
                            }

                            // 地圖內容
                            let fm_scale = 2.0;
                            let fm_off_x = 600.0;
                            let fm_off_y = 400.0;
                            let fw_fac = 1.0;

                            spawn_map_layer(
                                map,
                                fm_scale,
                                fm_off_x,
                                fm_off_y,
                                fw_fac,
                                true,
                                font.clone(),
                            );

                            // === 玩家標記（簡潔圓形+箭頭指針）===
                            // 注意：容器需要包含整個箭頭，旋轉才能正確工作
                            map.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(30.0),
                                    height: Val::Px(52.0), // 增加高度以包含箭頭
                                    left: Val::Px(585.0),
                                    top: Val::Px(374.0), // 調整位置
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    overflow: Overflow::visible(),
                                    ..default()
                                },
                                Transform::default(),
                                GlobalTransform::default(),
                                FullMapPlayerMarker,
                            ))
                            .with_children(|marker| {
                                // 淡白色外圈（脈衝動畫用）- 定位在容器下半部
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(28.0),
                                        height: Val::Px(28.0),
                                        left: Val::Px(1.0),
                                        top: Val::Px(23.0), // 在容器下半部
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
                                    BorderRadius::all(Val::Px(14.0)),
                                ));
                                // 黑色描邊圓
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(22.0),
                                        height: Val::Px(22.0),
                                        left: Val::Px(4.0),
                                        top: Val::Px(26.0),
                                        ..default()
                                    },
                                    BackgroundColor(OVERLAY_BLACK_90),
                                    BorderRadius::all(Val::Px(11.0)),
                                ));
                                // 白色主圓
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(16.0),
                                        height: Val::Px(16.0),
                                        left: Val::Px(7.0),
                                        top: Val::Px(29.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::WHITE),
                                    BorderRadius::all(Val::Px(8.0)),
                                ));
                                // 方向指示三角（黑色描邊）- 向上指
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(14.0),
                                        height: Val::Px(24.0),
                                        left: Val::Px(8.0),
                                        top: Val::Px(2.0),
                                        ..default()
                                    },
                                    BackgroundColor(OVERLAY_BLACK_90),
                                    BorderRadius::top(Val::Px(7.0)),
                                ));
                                // 方向指示三角（白色內部）
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(10.0),
                                        height: Val::Px(22.0),
                                        left: Val::Px(10.0),
                                        top: Val::Px(4.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::WHITE),
                                    BorderRadius::top(Val::Px(5.0)),
                                ));
                            });

                            // === 方位標示（帶圓角背景）===
                            for (label, color, position) in [
                                ("N", COMPASS_NORTH, (Val::Px(15.0), Val::Auto, Val::Px(582.0), Val::Auto)),
                                ("S", Color::WHITE, (Val::Auto, Val::Px(15.0), Val::Px(582.0), Val::Auto)),
                                ("E", Color::WHITE, (Val::Px(382.0), Val::Auto, Val::Auto, Val::Px(15.0))),
                                ("W", Color::WHITE, (Val::Px(382.0), Val::Auto, Val::Px(15.0), Val::Auto)),
                            ] {
                                spawn_compass_marker(map, label, font, 22.0, color, 36.0, position);
                            }
                        });
                });
            });

        // === 圖例（帶背景容器）===
        parent
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(25.0),
                    padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(8.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.1, 0.08, 0.85)),
                BorderColor::all(Color::srgba(0.3, 0.35, 0.3, 0.5)),
                BorderRadius::all(Val::Px(6.0)),
            ))
            .with_children(|legend| {
                spawn_legend_item(legend, Color::srgb(0.5, 0.5, 0.55), "道路", font.clone());
                spawn_legend_item(legend, Color::srgb(0.8, 0.25, 0.2), "地標", font.clone());
                spawn_legend_item(legend, PLAYER_MARKER_CORE, "你", font.clone());
            });

        // === 操作提示 ===
        parent
            .spawn((
                Node {
                    padding: UiRect::new(Val::Px(15.0), Val::Px(15.0), Val::Px(6.0), Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.6)),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|bg| {
                bg.spawn((
                    Text::new("[M] 關閉地圖"),
                    TextFont {
                        font_size: 14.0,
                        font: font.clone(),
                        ..default()
                    },
                    TextColor(TEXT_GRAY_90),
                ));
            });
    });
}

/// 設置 UI（使用中文字體）- 協調各分區設置函數
pub fn setup_ui(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // === 地圖常數定義 ===
    // 常數已移至 world/setup.rs 且通過 spawn_map_layer 引用

    setup_player_status_hud(&mut commands, &font);
    setup_minimap_hud(&mut commands, &font);
    setup_info_displays(&mut commands, &font);
    setup_control_hints(&mut commands, &font);
    setup_pause_menu(&mut commands, &font);
    setup_full_map(&mut commands, &font);
}

/// 生成圖例項目
fn spawn_legend_item(
    parent: &mut ChildSpawnerCommands,
    color: Color,
    label: &str,
    font: Handle<Font>,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(5.0),
            ..default()
        },))
        .with_children(|item| {
            item.spawn((
                Node {
                    width: Val::Px(15.0),
                    height: Val::Px(15.0),
                    ..default()
                },
                BackgroundColor(color),
            ));
            item.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    font,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}
