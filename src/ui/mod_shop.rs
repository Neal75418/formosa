//! 改裝店 UI（ModShop）
//!
//! 手機 App 中的車輛改裝商店介面

use bevy::prelude::*;

use super::phone_apps::spawn_section_title;
use crate::vehicle::{ModCategory, VehicleModifications, PurchaseModificationEvent};
use crate::economy::PlayerWallet;
use crate::core::GameState;
use crate::ui::notification::NotificationQueue;

/// 改裝卡片背景色
const MOD_CARD_BG: Color = Color::srgba(0.1, 0.12, 0.18, 0.8);
/// 改裝卡片可購買色
const MOD_CARD_AVAILABLE: Color = Color::srgba(0.15, 0.25, 0.15, 0.9);
/// 改裝卡片無法購買色
const MOD_CARD_UNAVAILABLE: Color = Color::srgba(0.2, 0.1, 0.1, 0.8);
/// MAX 等級色
const MAX_LEVEL_COLOR: Color = Color::srgba(0.8, 0.6, 0.0, 1.0);

// ============================================================================
// Components
// ============================================================================

/// ModShop 改裝卡片按鈕標記
#[derive(Component)]
pub struct ModShopButton {
    pub category: ModCategory,
    pub vehicle: Entity,
}

/// ModShop 內容容器標記
#[derive(Component)]
pub struct ModShopContent;

// ============================================================================
// UI 渲染
// ============================================================================

/// 渲染 ModShop UI 內容
pub(super) fn render_mod_shop_content(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    game_state: &GameState,
    vehicle_query: &Query<&VehicleModifications>,
    wallet: &PlayerWallet,
) {
    // 標題
    spawn_section_title(parent, font, "車輛改裝");

    // 錢包餘額
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new("可用餘額:"),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 1.0)),
            ));
            row.spawn((
                Text::new(format!("${}", wallet.cash)),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.0, 0.8, 0.0, 1.0)),
            ));
        });

    // 檢查玩家是否有車輛
    let (vehicle_entity, mods) = if let Some(vehicle_entity) = game_state.current_vehicle {
        if let Ok(mods) = vehicle_query.get(vehicle_entity) {
            (vehicle_entity, mods)
        } else {
            show_no_vehicle_message(parent, font);
            return;
        }
    } else {
        show_no_vehicle_message(parent, font);
        return;
    };

    // 改裝類別網格（垂直排列）
    parent
        .spawn((
            ModShopContent,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
        ))
        .with_children(|content| {
            for category in ModCategory::all() {
                spawn_mod_category_card(content, font, *category, mods, wallet, vehicle_entity);
            }
        });
}

/// 生成單個改裝類別卡片
fn spawn_mod_category_card(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    category: ModCategory,
    mods: &VehicleModifications,
    wallet: &PlayerWallet,
    vehicle_entity: Entity,
) {
    let current_level = mods.get_level(category);
    let next_level = current_level.next();

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(MOD_CARD_BG),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|card| {
            // 頭部：圖標 + 名稱
            card.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
            ))
            .with_children(|header| {
                header.spawn((
                    Text::new(category.icon()),
                    TextFont {
                        font: font.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                ));
                header.spawn((
                    Text::new(category.name()),
                    TextFont {
                        font: font.clone(),
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

            // 當前等級
            card.spawn((
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(format!("當前: {}", current_level.name())),
                    TextFont {
                        font: font.clone(),
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.7, 0.7, 0.8, 1.0)),
                ));
            });

            // 性能提升資訊或 MAX 標籤
            if let Some(next) = next_level {
                let upgrade_price = current_level.upgrade_price().unwrap();
                let can_afford = wallet.cash >= upgrade_price;

                // 性能對比
                card.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        ..default()
                    },
                ))
                .with_children(|row| {
                    let current_mult = (current_level.multiplier() * 100.0) as i32;
                    let next_mult = (next.multiplier() * 100.0) as i32;
                    let improvement = next_mult - current_mult;

                    row.spawn((
                        Text::new(format!(
                            "{}% → {}% (+{}%)",
                            current_mult, next_mult, improvement
                        )),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.0, 0.8, 0.3, 1.0)),
                    ));
                });

                // 升級按鈕
                let button_bg = if can_afford {
                    MOD_CARD_AVAILABLE
                } else {
                    MOD_CARD_UNAVAILABLE
                };

                let button_text = format!("升級 - ${}", upgrade_price);

                card.spawn((
                    ModShopButton {
                        category,
                        vehicle: vehicle_entity,
                    },
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(button_bg),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .with_children(|button| {
                    let text_color = if can_afford {
                        Color::WHITE
                    } else {
                        Color::srgba(0.5, 0.5, 0.5, 1.0)
                    };

                    button.spawn((
                        Text::new(button_text),
                        TextFont {
                            font: font.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(text_color),
                    ));
                });
            } else {
                // MAX 等級標籤
                card.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                ))
                .with_children(|container| {
                    container.spawn((
                        Text::new("✓ 最高等級"),
                        TextFont {
                            font: font.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(MAX_LEVEL_COLOR),
                    ));
                });
            }

            // 描述
            card.spawn((
                Text::new(category.description()),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
            ));
        });
}

// ============================================================================
// 按鈕互動系統
// ============================================================================

/// 顯示無車輛訊息
fn show_no_vehicle_message(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(20.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .with_children(|container| {
            container.spawn((
                Text::new("⚠️ 無可用車輛"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.5, 0.0, 1.0)),
            ));
        });
}

/// 處理 ModShop 按鈕點擊
pub(super) fn handle_mod_shop_buttons(
    interaction_query: Query<
        (&Interaction, &ModShopButton),
        (Changed<Interaction>, With<Button>),
    >,
    vehicle_mods: Query<&VehicleModifications>,
    wallet: Res<PlayerWallet>,
    mut purchase_events: MessageWriter<PurchaseModificationEvent>,
    mut notifications: ResMut<NotificationQueue>,
) {
    for (interaction, button) in &interaction_query {
        if *interaction == Interaction::Pressed {
            // 取得車輛改裝資料
            let Ok(mods) = vehicle_mods.get(button.vehicle) else {
                notifications.info("車輛不存在！".to_string());
                continue;
            };

            let current_level = mods.get_level(button.category);

            // 檢查是否可升級
            let Some(next_level) = current_level.next() else {
                notifications.info(format!("{}已達最高等級！", button.category.name()));
                continue;
            };

            // 檢查資金（提供即時反饋）
            let price = current_level.upgrade_price().unwrap();
            if wallet.cash < price {
                notifications.info(format!(
                    "資金不足！需要 ${}, 目前 ${}",
                    price, wallet.cash
                ));
                continue;
            }

            // 發送購買事件（系統會處理扣款和升級）
            purchase_events.write(PurchaseModificationEvent {
                vehicle: button.vehicle,
                category: button.category,
            });

            // 成功通知
            notifications.success(format!(
                "{}升級至{}！（-${}）",
                button.category.name(),
                next_level.name(),
                price
            ));
        }
    }
}
