//! 股市 App 渲染函數
//!
//! 從 `phone_apps`.rs 拆分，降低單檔行數。

use bevy::prelude::*;

use super::components::StockMarketTab;
use super::phone_apps::spawn_stat_row;
use crate::economy::stock_market::{StockMarket, StockSymbol};
use crate::economy::PlayerWallet;
use crate::ui::constants::{
    MARKET_CLOSED_COLOR, STOCK_DOWN_COLOR, STOCK_NEUTRAL_COLOR, STOCK_UP_COLOR,
};

/// 內容項目背景色（與 `phone_apps`.rs 同值）
const CONTENT_ITEM_BG: Color = Color::srgba(0.1, 0.12, 0.18, 0.8);

// ============================================================================
// 股市 App 渲染函數
// ============================================================================

/// 生成股市分頁選擇列
pub(super) fn spawn_stock_market_tabs(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    current_tab: StockMarketTab,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceEvenly,
                padding: UiRect::vertical(Val::Px(4.0)),
                margin: UiRect::bottom(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.8)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|row| {
            for tab in StockMarketTab::all() {
                let is_selected = *tab == current_tab;
                let bg = if is_selected {
                    Color::srgba(0.2, 0.4, 0.7, 0.9)
                } else {
                    Color::NONE
                };
                row.spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(8.0),
                            Val::Px(8.0),
                            Val::Px(3.0),
                            Val::Px(3.0),
                        ),
                        ..default()
                    },
                    BackgroundColor(bg),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|tab_btn| {
                    tab_btn.spawn((
                        Text::new(tab.label()),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(if is_selected {
                            Color::WHITE
                        } else {
                            Color::srgba(0.5, 0.5, 0.6, 0.8)
                        }),
                    ));
                });
            }
        });
}

/// 生成「行情」分頁內容
#[allow(clippy::too_many_lines)]
pub(super) fn spawn_stock_list(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    market: &StockMarket,
    selected_index: usize,
) {
    if !market.is_open {
        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.1, 0.05, 0.8)),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|warn| {
                warn.spawn((
                    Text::new("休市中（9:00-17:00 開市）"),
                    TextFont {
                        font: font.clone(),
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(MARKET_CLOSED_COLOR),
                ));
            });
    }

    for (i, symbol) in StockSymbol::ALL.iter().enumerate() {
        let stock = market.get(*symbol);
        let change_pct = stock.change_percent();
        let change_color = if change_pct.abs() < 0.01 {
            STOCK_NEUTRAL_COLOR
        } else if stock.is_up() {
            STOCK_UP_COLOR
        } else {
            STOCK_DOWN_COLOR
        };
        let is_selected = i == selected_index;
        let bg = if is_selected {
            Color::srgba(0.15, 0.2, 0.3, 0.9)
        } else {
            CONTENT_ITEM_BG
        };

        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(bg),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|card| {
                card.spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(symbol.label()),
                            TextFont {
                                font: font.clone(),
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        row.spawn((
                            Text::new(format!("${:.1}", stock.price)),
                            TextFont {
                                font: font.clone(),
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.9, 0.9, 0.95, 1.0)),
                        ));
                    });
                card.spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::top(Val::Px(2.0)),
                    ..default()
                },))
                    .with_children(|row| {
                        let sel_hint = if is_selected { " ▸" } else { "" };
                        row.spawn((
                            Text::new(format!("{}{}", symbol.ticker(), sel_hint)),
                            TextFont {
                                font: font.clone(),
                                font_size: 9.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                        ));
                        let sign = if stock.is_up() { "+" } else { "" };
                        row.spawn((
                            Text::new(format!("{sign}{change_pct:.2}%")),
                            TextFont {
                                font: font.clone(),
                                font_size: 9.0,
                                ..default()
                            },
                            TextColor(change_color),
                        ));
                    });
            });
    }
}

/// 生成「持倉」分頁內容
#[allow(clippy::too_many_lines)]
pub(super) fn spawn_stock_portfolio(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    market: &StockMarket,
) {
    let portfolio = &market.portfolio;
    let total_value = portfolio.total_value(market);
    spawn_stat_row(parent, font, "持倉總市值", &format!("${total_value:.0}"));

    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.5)),
    ));

    let mut has_holdings = false;
    for symbol in StockSymbol::ALL {
        let shares = portfolio.shares_of(symbol);
        if shares == 0 {
            continue;
        }
        has_holdings = true;
        let stock = market.get(symbol);
        let avg_cost = portfolio.avg_cost_of(symbol);
        let pl = portfolio.profit_loss(symbol, stock.price);
        let pl_color = if pl >= 0.0 {
            STOCK_UP_COLOR
        } else {
            STOCK_DOWN_COLOR
        };
        let pl_text = if pl >= 0.0 {
            format!("+${pl:.0}")
        } else {
            let neg_pl = -pl;
            format!("-${neg_pl:.0}")
        };

        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(CONTENT_ITEM_BG),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|card| {
                card.spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(format!("{} ({})", symbol.label(), symbol.ticker())),
                            TextFont {
                                font: font.clone(),
                                font_size: 11.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        row.spawn((
                            Text::new(format!("{shares} 股")),
                            TextFont {
                                font: font.clone(),
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                        ));
                    });
                card.spawn((Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::top(Val::Px(2.0)),
                    ..default()
                },))
                    .with_children(|row| {
                        row.spawn((
                            Text::new(format!("成本 ${:.1} → ${:.1}", avg_cost, stock.price)),
                            TextFont {
                                font: font.clone(),
                                font_size: 9.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                        ));
                        row.spawn((
                            Text::new(pl_text),
                            TextFont {
                                font: font.clone(),
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(pl_color),
                        ));
                    });
            });
    }

    if !has_holdings {
        parent
            .spawn((Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },))
            .with_children(|hint| {
                hint.spawn((
                    Text::new("尚未持有任何股票"),
                    TextFont {
                        font: font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                ));
            });
    }
}

/// 生成「交易」分頁內容
#[allow(clippy::too_many_lines)]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(super) fn spawn_stock_trade(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    market: &StockMarket,
    wallet: &PlayerWallet,
    selected_index: usize,
    trade_quantity: u32,
) {
    let symbol = StockSymbol::ALL[selected_index];
    let stock = market.get(symbol);
    let holdings = market.portfolio.shares_of(symbol);

    // 股票資訊卡
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.15, 0.25, 0.9)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|info| {
            info.spawn((
                Text::new(format!("{} ({})", symbol.label(), symbol.ticker())),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            info.spawn((
                Text::new(format!("現價 ${:.1}　持有 {} 股", stock.price, holdings)),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
            ));
        });

    // 數量選擇
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(6.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(CONTENT_ITEM_BG),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|qty| {
            qty.spawn((
                Text::new(format!("數量：{trade_quantity}　[Q-/E+]")),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });

    // 買入面板
    let buy_cost = (stock.price * trade_quantity as f32).ceil() as i32;
    let can_buy = wallet.cash >= buy_cost && market.is_open;
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(4.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(if can_buy {
                Color::srgba(0.05, 0.15, 0.1, 0.8)
            } else {
                Color::srgba(0.1, 0.1, 0.12, 0.6)
            }),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(format!("[Enter] 買入 ${buy_cost}")),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(if can_buy {
                    STOCK_UP_COLOR
                } else {
                    STOCK_NEUTRAL_COLOR
                }),
            ));
            panel.spawn((
                Text::new(format!("現金 ${}", wallet.cash)),
                TextFont {
                    font: font.clone(),
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
            ));
        });

    // 賣出面板
    let sell_value = (stock.price * trade_quantity as f32).floor() as i32;
    let can_sell = holdings >= trade_quantity && market.is_open;
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(if can_sell {
                Color::srgba(0.15, 0.05, 0.05, 0.8)
            } else {
                Color::srgba(0.1, 0.1, 0.12, 0.6)
            }),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(format!("[Space] 賣出 ${sell_value}")),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(if can_sell {
                    STOCK_DOWN_COLOR
                } else {
                    STOCK_NEUTRAL_COLOR
                }),
            ));
        });

    if !market.is_open {
        parent
            .spawn((Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(4.0)),
                justify_content: JustifyContent::Center,
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },))
            .with_children(|warn| {
                warn.spawn((
                    Text::new("市場已關閉，無法交易"),
                    TextFont {
                        font: font.clone(),
                        font_size: 9.0,
                        ..default()
                    },
                    TextColor(MARKET_CLOSED_COLOR),
                ));
            });
    }

    parent
        .spawn((Node {
            width: Val::Percent(100.0),
            padding: UiRect::top(Val::Px(6.0)),
            justify_content: JustifyContent::Center,
            ..default()
        },))
        .with_children(|hint| {
            hint.spawn((
                Text::new("[↓] 切換股票"),
                TextFont {
                    font: font.clone(),
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.4, 0.4, 0.5, 0.6)),
            ));
        });
}
