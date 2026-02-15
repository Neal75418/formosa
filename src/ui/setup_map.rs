//! 地圖設置系統 - 小地圖、大地圖

use bevy::prelude::*;

use super::components::{
    FullMapContainer, FullMapPlayerMarker, MinimapContainer, MinimapPlayerGlow,
    MinimapPlayerMarker, MinimapScanLine,
};
use super::constants::*;
use super::minimap::spawn_map_layer;
use super::systems::{spawn_compass_marker, spawn_full_screen_overlay};

/// 生成小地圖玩家標記（圓形+箭頭指針）
fn spawn_minimap_player_marker(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(20.0),
                height: Val::Px(34.0),
                left: Val::Px(140.0),
                top: Val::Px(133.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::visible(),
                ..default()
            },
            Transform::default(),
            GlobalTransform::default(),
            MinimapPlayerMarker,
        ))
        .with_children(|marker| {
            // 淡白色外圈（脈衝動畫用）
            marker.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(18.0),
                    height: Val::Px(18.0),
                    left: Val::Px(1.0),
                    top: Val::Px(15.0),
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
}

/// 生成大地圖玩家標記（大尺寸圓形+箭頭指針）
fn spawn_fullmap_player_marker(map: &mut ChildSpawnerCommands) {
    map.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(30.0),
            height: Val::Px(52.0),
            left: Val::Px(585.0),
            top: Val::Px(374.0),
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
        // 淡白色外圈（脈衝動畫用）
        marker.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(28.0),
                height: Val::Px(28.0),
                left: Val::Px(1.0),
                top: Val::Px(23.0),
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
        // 方向指示三角（黑色描邊）
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
}

/// 生成大地圖網格線
fn spawn_fullmap_grid_lines(map: &mut ChildSpawnerCommands) {
    for i in 0..9 {
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
}

fn spawn_fullmap_title(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
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
}

/// 生成大地圖圖例與操作提示
fn spawn_fullmap_legend_and_hints(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    // 圖例
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

    // 操作提示
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
}

fn spawn_minimap_decorations(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
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

    // 標題（帶背景）
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
            GlobalTransform::default(), // B0004: Text 子實體需要 GlobalTransform
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
}

/// 生成小地圖掃描線效果（模擬雷達感）
fn spawn_minimap_scan_effects(parent: &mut ChildSpawnerCommands) {
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
}

// ============================================================================
// 分區設置函數
// ============================================================================
/// 設置右上角小地圖（GTA 風格多層邊框）
pub(super) fn setup_minimap_hud(commands: &mut Commands, font: &Handle<Font>) {
    // 外層發光框
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(MINIMAP_OUTER_SIZE), // 300 + 8*2 邊框
                height: Val::Px(MINIMAP_OUTER_SIZE),
                padding: UiRect::all(Val::Px(4.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(MINIMAP_GLOW),
            BorderRadius::all(Val::Px(12.0)),
            GlobalTransform::default(), // B0004: 後代有 Transform（玩家標記）
        ))
        .with_children(|glow| {
            // 主邊框層
            glow.spawn((
                Node {
                    width: Val::Px(MINIMAP_FRAME_SIZE),
                    height: Val::Px(MINIMAP_FRAME_SIZE),
                    padding: UiRect::all(Val::Px(3.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(MINIMAP_BORDER),
                BorderColor::all(MINIMAP_INNER_BORDER),
                BorderRadius::all(Val::Px(10.0)),
                GlobalTransform::default(), // B0004: 後代有 Transform（玩家標記）
            ))
            .with_children(|frame| {
                // 實際地圖容器
                frame
                    .spawn((
                        Node {
                            width: Val::Px(MINIMAP_CONTENT_SIZE),
                            height: Val::Px(MINIMAP_CONTENT_SIZE),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(MINIMAP_BG),
                        BorderRadius::all(Val::Px(6.0)),
                        GlobalTransform::default(), // B0004: 子實體需要 GlobalTransform
                        MinimapContainer,
                    ))
                    .with_children(|parent| {
                        // 裝飾（角落高光 + 標題）
                        spawn_minimap_decorations(parent, font);

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

                        // 玩家標記（圓形+箭頭指針）
                        spawn_minimap_player_marker(parent);

                        // === 方位標示（帶圓角背景）===
                        for (label, color, position) in [
                            ("N", COMPASS_NORTH, (Val::Px(6.0), Val::Auto, Val::Px(140.0), Val::Auto)),
                            ("S", Color::WHITE, (Val::Auto, Val::Px(6.0), Val::Px(140.0), Val::Auto)),
                            ("E", Color::WHITE, (Val::Px(140.0), Val::Auto, Val::Auto, Val::Px(6.0))),
                            ("W", Color::WHITE, (Val::Px(140.0), Val::Auto, Val::Px(6.0), Val::Auto)),
                        ] {
                            spawn_compass_marker(parent, label, font, 13.0, color, 20.0, position);
                        }

                        // 掃描線效果
                        spawn_minimap_scan_effects(parent);
                    });
            });
        });
}

/// 設置大地圖（GTA 風格，初始隱藏）
pub(super) fn setup_full_map(commands: &mut Commands, font: &Handle<Font>) {
    spawn_full_screen_overlay(
        commands,
        FULLMAP_BG,
        FullMapContainer,
        FlexDirection::Column,
        Val::Px(12.0),
    )
    .insert(GlobalTransform::default()) // B0004: 後代有 Transform（玩家標記）
    .with_children(|parent| {
        // 標題區
        spawn_fullmap_title(parent, font);

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
                GlobalTransform::default(), // B0004: 後代有 Transform（玩家標記）
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
                    GlobalTransform::default(), // B0004: 後代有 Transform（玩家標記）
                ))
                .with_children(|frame| {
                    // 實際地圖容器
                    frame
                        .spawn((
                            Node {
                                width: Val::Px(FULLMAP_WIDTH),
                                height: Val::Px(FULLMAP_HEIGHT),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BackgroundColor(FULLMAP_MAIN_BG),
                            BorderRadius::all(Val::Px(6.0)),
                            GlobalTransform::default(), // B0004: 後代有 Transform（玩家標記）
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
                            spawn_fullmap_grid_lines(map);

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

                            // 玩家標記（圓形+箭頭指針）
                            spawn_fullmap_player_marker(map);

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

        // 圖例與操作提示
        spawn_fullmap_legend_and_hints(parent, font);
    });
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
