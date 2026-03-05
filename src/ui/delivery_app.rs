//! 外送 App UI 系統 (GTA 風格)
//!
//! 類似手機 App 的訂單管理介面

use bevy::prelude::*;

use super::components::{
    ChineseFont, DeliveryAppContainer, DeliveryOrderCard, DeliveryOrderList, DeliveryRatingDisplay,
    DeliveryStreakDisplay,
};
use super::constants::{
    icon_box_node, ADDRESS_TEXT_COLOR, DELIVERY_APP_BG, DELIVERY_APP_BORDER, DELIVERY_APP_GLOW,
    DELIVERY_APP_INNER_BORDER, DELIVERY_APP_SUBTITLE, DELIVERY_APP_TITLE, KEY_ICON_BG,
    KEY_ICON_BORDER, KEY_TEXT_COLOR, ORDER_CARD_BG, ORDER_CARD_BORDER, ORDER_CARD_GLOW,
    PANEL_BORDER_GRAY, RATING_STAR_COLOR, RESTAURANT_NAME_COLOR, REWARD_TEXT_COLOR, STREAK_COLOR,
    TEXT_GRAY_90, TEXT_MUTED, TEXT_SECONDARY,
};
use crate::mission::MissionManager;
use crate::ui::UiState;

/// 設置外送 App UI（GTA 風格）
pub fn setup_delivery_app(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // 外送 App 外發光層
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(46.0),
                top: Val::Px(96.0),
                width: Val::Px(362.0),
                height: Val::Auto,
                max_height: Val::Px(516.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(DELIVERY_APP_GLOW),
            BorderRadius::all(Val::Px(10.0)),
            Visibility::Hidden,
            DeliveryAppContainer,
        ))
        .with_children(|glow| {
            // 主邊框層
            glow.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(DELIVERY_APP_BORDER),
                BorderColor::all(DELIVERY_APP_BORDER),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|border| {
                // 內邊框層
                border
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(2.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(DELIVERY_APP_INNER_BORDER),
                        BorderColor::all(DELIVERY_APP_INNER_BORDER),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|inner| {
                        // 內容區
                        inner
                            .spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(12.0)),
                                    row_gap: Val::Px(10.0),
                                    ..default()
                                },
                                BackgroundColor(DELIVERY_APP_BG),
                                BorderRadius::all(Val::Px(4.0)),
                            ))
                            .with_children(|content| {
                                spawn_header(content, &font);
                                spawn_stats_row(content, &font);
                                spawn_order_list(content);
                                spawn_hint_row(content, &font);
                            });
                    });
            });
        });
}

/// 生成標題列
fn spawn_header(content: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    content
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::bottom(Val::Px(8.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(DELIVERY_APP_INNER_BORDER),
        ))
        .with_children(|header| {
            // App 圖示和名稱
            header
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                },))
                .with_children(|title| {
                    spawn_delivery_app_icon(title, font);
                    // App 名稱
                    title.spawn((
                        Text::new("西門快送"),
                        TextFont {
                            font_size: 22.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(DELIVERY_APP_TITLE),
                    ));
                });
            // 關閉提示
            header
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                },))
                .with_children(|close| {
                    close
                        .spawn((
                            Node {
                                padding: UiRect::new(
                                    Val::Px(6.0),
                                    Val::Px(6.0),
                                    Val::Px(2.0),
                                    Val::Px(2.0),
                                ),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(KEY_ICON_BG),
                            BorderColor::all(KEY_ICON_BORDER),
                            BorderRadius::all(Val::Px(3.0)),
                        ))
                        .with_children(|key| {
                            key.spawn((
                                Text::new("O"),
                                TextFont {
                                    font_size: 10.0,
                                    font: font.clone(),
                                    ..default()
                                },
                                TextColor(KEY_TEXT_COLOR),
                            ));
                        });
                    close.spawn((
                        Text::new("關閉"),
                        TextFont {
                            font_size: 11.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(DELIVERY_APP_SUBTITLE),
                    ));
                });
        });
}

/// 生成統計資訊列
fn spawn_stats_row(content: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    content
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(6.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.08, 0.12, 0.8)),
            BorderColor::all(Color::srgba(0.3, 0.25, 0.2, 0.5)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|stats| {
            // 評價顯示
            stats
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                },))
                .with_children(|rating| {
                    rating.spawn((
                        Text::new("⭐"),
                        TextFont {
                            font_size: 14.0,
                            font: font.clone(),
                            ..default()
                        },
                    ));
                    rating.spawn((
                        Text::new("4.8"),
                        TextFont {
                            font_size: 16.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(RATING_STAR_COLOR),
                        DeliveryRatingDisplay,
                    ));
                });
            // 連擊顯示
            stats
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    ..default()
                },))
                .with_children(|streak| {
                    streak.spawn((
                        Text::new("🔥"),
                        TextFont {
                            font_size: 14.0,
                            font: font.clone(),
                            ..default()
                        },
                    ));
                    streak.spawn((
                        Text::new("x0 連擊"),
                        TextFont {
                            font_size: 14.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(STREAK_COLOR),
                        DeliveryStreakDisplay,
                    ));
                });
        });
}

/// 生成訂單列表區域
fn spawn_order_list(content: &mut ChildSpawnerCommands) {
    content.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            overflow: Overflow::clip(),
            max_height: Val::Px(300.0),
            ..default()
        },
        DeliveryOrderList,
    ));
}

/// 生成提示文字
fn spawn_hint_row(content: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    content
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::top(Val::Px(4.0)),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.3, 0.3, 0.3, 0.3)),
        ))
        .with_children(|hint| {
            hint.spawn((
                Text::new("💡"),
                TextFont {
                    font_size: 12.0,
                    font: font.clone(),
                    ..default()
                },
            ));
            hint.spawn((
                Text::new("靠近餐廳按 F 接單"),
                TextFont {
                    font_size: 12.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(DELIVERY_APP_SUBTITLE),
            ));
        });
}

/// 生成 App 圖示（🛵）
fn spawn_delivery_app_icon(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn((
            icon_box_node(),
            BackgroundColor(Color::srgba(0.9, 0.4, 0.1, 0.3)),
            BorderColor::all(DELIVERY_APP_BORDER),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|icon| {
            icon.spawn((
                Text::new("🛵"),
                TextFont {
                    font_size: 16.0,
                    font: font.clone(),
                    ..default()
                },
            ));
        });
}

/// 切換外送 App 顯示（O 鍵）
pub fn toggle_delivery_app(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut app_query: Query<&mut Visibility, With<DeliveryAppContainer>>,
    mut mission_manager: ResMut<MissionManager>,
) {
    if keyboard.just_pressed(KeyCode::KeyO) {
        ui_state.show_delivery_app = !ui_state.show_delivery_app;

        if let Ok(mut visibility) = app_query.single_mut() {
            if ui_state.show_delivery_app {
                *visibility = Visibility::Visible;
                // 開啟時刷新訂單
                mission_manager.refresh_delivery_orders();
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// 更新評分顯示
fn update_rating_display(
    rating_query: &mut Query<
        &mut Text,
        (With<DeliveryRatingDisplay>, Without<DeliveryStreakDisplay>),
    >,
    mission_manager: &MissionManager,
) {
    let Ok(mut text) = rating_query.single_mut() else {
        return;
    };
    let avg = if mission_manager.total_deliveries > 0 {
        mission_manager.average_rating
    } else {
        5.0 // 新手預設滿星
    };
    **text = format!("[*] {avg:.1}");
}

/// 更新連擊顯示
fn update_streak_display(
    streak_query: &mut Query<
        &mut Text,
        (With<DeliveryStreakDisplay>, Without<DeliveryRatingDisplay>),
    >,
    streak: u32,
) {
    let Ok(mut text) = streak_query.single_mut() else {
        return;
    };
    **text = if streak > 0 {
        format!("x{streak} 連擊")
    } else {
        "x0 連擊".to_string()
    };
}

/// 生成空訂單提示（GTA 風格）
fn spawn_empty_order_hint(list: &mut ChildSpawnerCommands, font: Handle<Font>) {
    list.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(Val::Px(25.0)),
            row_gap: Val::Px(8.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.08, 0.1, 0.6)),
        BorderColor::all(PANEL_BORDER_GRAY),
        BorderRadius::all(Val::Px(6.0)),
    ))
    .with_children(|hint_box| {
        hint_box.spawn((
            Text::new("📭"),
            TextFont {
                font_size: 32.0,
                font: font.clone(),
                ..default()
            },
        ));
        hint_box.spawn((
            Text::new("暫無訂單"),
            TextFont {
                font_size: 16.0,
                font: font.clone(),
                ..default()
            },
            TextColor(TEXT_GRAY_90),
        ));
        hint_box.spawn((
            Text::new("稍後再試..."),
            TextFont {
                font_size: 12.0,
                font,
                ..default()
            },
            TextColor(TEXT_MUTED),
        ));
    });
}

/// 更新外送 App 訂單列表
/// 優化：只在訂單實際變更時重建 UI，避免每幀 despawn/spawn
#[allow(clippy::too_many_arguments)]
pub fn update_delivery_app(
    mut mission_manager: ResMut<MissionManager>,
    chinese_font: Res<ChineseFont>,
    ui_state: Res<UiState>,
    mut commands: Commands,
    order_list_query: Query<Entity, With<DeliveryOrderList>>,
    existing_cards: Query<Entity, With<DeliveryOrderCard>>,
    mut rating_query: Query<
        &mut Text,
        (With<DeliveryRatingDisplay>, Without<DeliveryStreakDisplay>),
    >,
    mut streak_query: Query<
        &mut Text,
        (With<DeliveryStreakDisplay>, Without<DeliveryRatingDisplay>),
    >,
) {
    if !ui_state.show_delivery_app {
        return;
    }

    // 更新統計資訊
    update_rating_display(&mut rating_query, &mission_manager);
    update_streak_display(&mut streak_query, mission_manager.delivery_streak);

    // 只在訂單變更時重建卡片
    if !mission_manager.delivery_orders_changed {
        return;
    }
    mission_manager.delivery_orders_changed = false;

    let font = chinese_font.font.clone();

    // 清除舊卡片
    for entity in &existing_cards {
        commands.entity(entity).despawn();
    }

    // 生成新卡片
    let Ok(list_entity) = order_list_query.single() else {
        return;
    };
    commands.entity(list_entity).with_children(|list| {
        if mission_manager.delivery_orders.is_empty() {
            spawn_empty_order_hint(list, font);
        } else {
            for (idx, order) in mission_manager.delivery_orders.iter().enumerate() {
                spawn_delivery_order_card(list, idx, order, font.clone());
            }
        }
    });
}

/// 生成單個訂單卡片（GTA 風格）
fn spawn_delivery_order_card(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    order: &crate::mission::MissionData,
    font: Handle<Font>,
) {
    let delivery_info = order.delivery_order.as_ref();

    // 外層發光
    parent
        .spawn((
            Node {
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(ORDER_CARD_GLOW),
            BorderRadius::all(Val::Px(6.0)),
            DeliveryOrderCard { order_index: index },
        ))
        .with_children(|glow| {
            // 主卡片
            glow.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(1.0)),
                    width: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(ORDER_CARD_BG),
                BorderColor::all(ORDER_CARD_BORDER),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|card| {
                spawn_order_title_row(card, delivery_info, &font);
                spawn_order_address_row(card, delivery_info, &font);
                spawn_order_info_row(card, order, delivery_info, &font);
            });
        });
}

/// 生成訂單標題行
fn spawn_order_title_row(
    card: &mut ChildSpawnerCommands,
    delivery_info: Option<&crate::mission::DeliveryOrder>,
    font: &Handle<Font>,
) {
    let restaurant_name = delivery_info.map_or("未知餐廳", |d| d.restaurant_name.as_str());
    let food_item = delivery_info.map_or("外送品項", |d| d.food_item.as_str());

    card.spawn((Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        ..default()
    },))
        .with_children(|title_row| {
            // 食物圖示
            title_row.spawn((
                Text::new("🍜"),
                TextFont {
                    font_size: 16.0,
                    font: font.clone(),
                    ..default()
                },
            ));
            // 餐廳名稱
            title_row.spawn((
                Text::new(restaurant_name),
                TextFont {
                    font_size: 14.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(RESTAURANT_NAME_COLOR),
            ));
            // 分隔
            title_row.spawn((
                Text::new("-"),
                TextFont {
                    font_size: 14.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
            ));
            // 餐點
            title_row.spawn((
                Text::new(food_item),
                TextFont {
                    font_size: 13.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// 生成訂單地址行
fn spawn_order_address_row(
    card: &mut ChildSpawnerCommands,
    delivery_info: Option<&crate::mission::DeliveryOrder>,
    font: &Handle<Font>,
) {
    let address = delivery_info.map_or("未知地址", |d| d.customer_address.as_str());

    card.spawn((Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        ..default()
    },))
        .with_children(|addr_row| {
            addr_row.spawn((
                Text::new("📍"),
                TextFont {
                    font_size: 12.0,
                    font: font.clone(),
                    ..default()
                },
            ));
            addr_row.spawn((
                Text::new(address),
                TextFont {
                    font_size: 12.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(ADDRESS_TEXT_COLOR),
            ));
        });
}

/// 生成訂單資訊行（報酬、距離、時間）
fn spawn_order_info_row(
    card: &mut ChildSpawnerCommands,
    order: &crate::mission::MissionData,
    delivery_info: Option<&crate::mission::DeliveryOrder>,
    font: &Handle<Font>,
) {
    let distance = delivery_info.map_or(0.0, |d| d.distance);
    let time_limit = order.time_limit.unwrap_or(0.0);

    card.spawn((
        Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::top(Val::Px(4.0)),
            border: UiRect::top(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.3, 0.25, 0.2, 0.4)),
    ))
    .with_children(|info_row| {
        // 報酬（醒目綠色）
        info_row
            .spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(4.0),
                ..default()
            },))
            .with_children(|reward| {
                reward.spawn((
                    Text::new("💰"),
                    TextFont {
                        font_size: 14.0,
                        font: font.clone(),
                        ..default()
                    },
                ));
                reward.spawn((
                    Text::new(format!("${}", order.reward)),
                    TextFont {
                        font_size: 15.0,
                        font: font.clone(),
                        ..default()
                    },
                    TextColor(REWARD_TEXT_COLOR),
                ));
            });
        // 距離和時間
        info_row
            .spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },))
            .with_children(|meta| {
                meta.spawn((
                    Text::new(format!("🗺 {distance:.0}m")),
                    TextFont {
                        font_size: 11.0,
                        font: font.clone(),
                        ..default()
                    },
                    TextColor(TEXT_SECONDARY),
                ));
                meta.spawn((
                    Text::new(format!("⏱ {time_limit:.0}s")),
                    TextFont {
                        font_size: 11.0,
                        font: font.clone(),
                        ..default()
                    },
                    TextColor(TEXT_SECONDARY),
                ));
            });
    });
}

pub(super) struct DeliveryAppPlugin;

impl Plugin for DeliveryAppPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_delivery_app.in_set(super::UiSetup))
            .add_systems(
                Update,
                (toggle_delivery_app, update_delivery_app).in_set(super::UiActive),
            );
    }
}
