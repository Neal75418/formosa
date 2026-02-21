//! HUD 設置系統 - 玩家狀態、資訊顯示、控制提示

use bevy::prelude::*;

use super::components::{
    ArmorBarFill, ArmorLabel, ArmorLabelShadow, ArmorSection, ControlHintContainer,
    ControlKeyArea, ControlSpeedDisplay, ControlStatusTag, HealthBarBg, HealthBarFill,
    HealthBarGlow, HealthLabel, HealthLabelShadow, MissionInfo, MoneyDisplay,
    PlayerStatusContainer, RadioDescription, RadioDisplayContainer, RadioFrequency, RadioIcon,
    RadioStationName, RadioVolumeBarBg, RadioVolumeBarFill, TimeDisplay, UiText,
};
use super::constants::*;
use super::systems::{spawn_status_bar_fill, spawn_status_bar_label, spawn_text_child};

fn spawn_health_section(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
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
                glow.spawn((
                    Node {
                        width: Val::Px(STATUS_BAR_WIDTH),
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
            });

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
}

/// 生成護甲區塊（圖示 + 護甲條 + 數值標籤，預設隱藏）
fn spawn_armor_section_ui(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            },
            Visibility::Hidden,
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
                    width: Val::Px(STATUS_BAR_WIDTH),
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
}

/// 生成控制提示按鍵（按鍵框 + 動作說明）
fn spawn_control_key(
    keys: &mut ChildSpawnerCommands,
    key: &str,
    action: &str,
    font: Handle<Font>,
) {
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
        spawn_text_child(key_bg, key, 11.0, KEY_TEXT_COLOR, &font);
    });
    keys.spawn((
        Text::new(action),
        TextFont {
            font_size: 11.0,
            font,
            ..default()
        },
        TextColor(ACTION_TEXT_COLOR),
        Node {
            margin: UiRect::right(Val::Px(6.0)),
            ..default()
        },
    ));
}

/// 設置左下角 GTA 風格玩家狀態區（血量條、護甲條）
pub(super) fn setup_player_status_hud(commands: &mut Commands, font: &Handle<Font>) {
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
                spawn_health_section(parent, font);
                spawn_armor_section_ui(parent, font);
            }); // 結束 PlayerStatusContainer
        }); // 結束外發光層
}

/// 設置小地圖下方的時間、金錢、任務資訊顯示
pub(super) fn setup_info_displays(commands: &mut Commands, font: &Handle<Font>) {
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
pub(super) fn setup_control_hints(commands: &mut Commands, font: &Handle<Font>) {
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
                    spawn_control_key(keys, "WASD", "移動", font.clone());
                    spawn_control_key(keys, "R", "射擊", font.clone());
                    spawn_control_key(keys, "1-4", "武器", font.clone());
                    spawn_control_key(keys, "ESC", "暫停", font.clone());
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

/// 設置右上角電台顯示（小地圖下方，GTA 5 風格）
pub(super) fn setup_radio_display(commands: &mut Commands, font: &Handle<Font>) {
    // 外發光層
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(480.0), // 天氣 HUD 下方（避免重疊）
                right: Val::Px(6.0),
                padding: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BackgroundColor(HUD_GLOW_OUTER),
            BorderRadius::all(Val::Px(10.0)),
            Visibility::Hidden, // 預設隱藏，切換電台時顯示
            RadioDisplayContainer,
        ))
        .with_children(|glow| {
            // 主容器
            glow.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    min_width: Val::Px(260.0),
                    ..default()
                },
                BackgroundColor(HUD_BG),
                BorderColor::all(HUD_BORDER_HIGHLIGHT),
                BorderRadius::all(Val::Px(6.0)),
            ))
            .with_children(|container| {
                // 電台圖示（🎵）
                container.spawn((
                    Node {
                        width: Val::Px(24.0),
                        height: Val::Px(24.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.9, 0.3, 0.4, 0.85)),
                    BorderRadius::all(Val::Px(12.0)),
                    RadioIcon,
                ))
                .with_children(|icon| {
                    icon.spawn((
                        Text::new("🎵"),
                        TextFont {
                            font_size: 14.0,
                            font: font.clone(),
                            ..default()
                        },
                    ));
                });

                // 文字資訊區域（垂直排列）
                container
                    .spawn((Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        flex_grow: 1.0,
                        ..default()
                    },))
                    .with_children(|text_area| {
                        // 電台名稱 + 頻率（同一行）
                        text_area
                            .spawn((Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(6.0),
                                ..default()
                            },))
                            .with_children(|name_row| {
                                // 電台名稱
                                name_row.spawn((
                                    Text::new("寶島流行樂"),
                                    TextFont {
                                        font_size: 16.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::srgba(0.95, 0.95, 0.95, 1.0)),
                                    RadioStationName,
                                ));

                                // 頻率標籤（小字、半透明）
                                name_row.spawn((
                                    Text::new("FM 102.7"),
                                    TextFont {
                                        font_size: 12.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::srgba(0.7, 0.85, 0.9, 0.8)),
                                    RadioFrequency,
                                ));
                            });

                        // 電台描述
                        text_area.spawn((
                            Text::new("台灣本土流行音樂 - 經典華語金曲"),
                            TextFont {
                                font_size: 11.0,
                                font: font.clone(),
                                ..default()
                            },
                            TextColor(Color::srgba(0.75, 0.75, 0.75, 0.9)),
                            RadioDescription,
                        ));

                        // 音量條（背景 + 填充）
                        text_area
                            .spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(4.0),
                                    margin: UiRect::top(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.6)),
                                BorderRadius::all(Val::Px(2.0)),
                                RadioVolumeBarBg,
                            ))
                            .with_children(|vol_bg| {
                                vol_bg.spawn((
                                    Node {
                                        width: Val::Percent(60.0), // 預設 60% 音量
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.4, 0.7, 0.9, 0.85)),
                                    BorderRadius::all(Val::Px(2.0)),
                                    RadioVolumeBarFill,
                                ));
                            });
                    });
            }); // 結束主容器
        }); // 結束外發光層
}
