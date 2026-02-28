//! 手機 UI 系統 (GTA 5 風格)
//!
//! 上箭頭鍵開啟手機，包含聯絡人、任務日誌、地圖、設定等分頁。

use bevy::prelude::*;

use super::components::{
    ChineseFont, MissionJournalTab, PhoneApp, PhoneAppIcon, PhoneContactList, PhoneContainer,
    PhoneContentArea, PhoneMissionLogList, PhoneScreen, PhoneStatusBar, PhoneStockMarketList,
    PhoneUiState, StockMarketTab,
};
use super::UiState;
use crate::economy::stock_market::{StockMarket, StockSymbol};
use crate::economy::PlayerWallet;
use crate::mission::MissionManager;
use crate::ui::notification::NotificationQueue;
use crate::core::GameState;
use super::phone_apps::*;
use super::phone_apps_stock::*;
use super::mod_shop::*;
use crate::vehicle::VehicleModifications;

// ============================================================================
// 常數
// ============================================================================

/// 手機寬度
const PHONE_WIDTH: f32 = 280.0;
/// 手機高度
const PHONE_HEIGHT: f32 = 480.0;
/// 手機背景色
const PHONE_BG: Color = Color::srgba(0.08, 0.08, 0.12, 0.95);
/// 手機邊框色
const PHONE_BORDER_COLOR: Color = Color::srgba(0.3, 0.3, 0.4, 0.8);
/// 手機螢幕背景色
const PHONE_SCREEN_BG: Color = Color::srgba(0.05, 0.08, 0.15, 1.0);
/// App 圖標背景色
const APP_ICON_BG: Color = Color::srgba(0.15, 0.2, 0.3, 0.9);
/// App 圖標選中色
const APP_ICON_SELECTED: Color = Color::srgba(0.2, 0.4, 0.7, 0.9);
/// 狀態列背景
const STATUS_BAR_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
/// 內容區項目色
const CONTENT_ITEM_BG: Color = Color::srgba(0.1, 0.12, 0.18, 0.8);

// ============================================================================
// 設置系統
// ============================================================================

/// 設置手機 UI
pub fn setup_phone_ui(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // 手機外框（右下角）
    commands
        .spawn((
            PhoneContainer,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(40.0),
                bottom: Val::Px(40.0),
                width: Val::Px(PHONE_WIDTH),
                height: Val::Px(PHONE_HEIGHT),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(3.0)),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(PHONE_BG),
            BorderColor::all(PHONE_BORDER_COLOR),
            BorderRadius::all(Val::Px(16.0)),
            Visibility::Hidden,
            ZIndex(90),
        ))
        .with_children(|phone| {
            // 狀態列
            phone
                .spawn((
                    PhoneStatusBar,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(STATUS_BAR_BG),
                    BorderRadius::px(12.0, 12.0, 0.0, 0.0),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Text::new("Island Phone"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                    ));
                });

            // 螢幕區域
            phone
                .spawn((
                    PhoneScreen,
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(6.0)),
                        row_gap: Val::Px(6.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(PHONE_SCREEN_BG),
                ))
                .with_children(|screen| {
                    // App 圖標網格（主畫面）
                    screen
                        .spawn((
                            PhoneContentArea,
                            Node {
                                width: Val::Percent(100.0),
                                flex_grow: 1.0,
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::Center,
                                align_content: AlignContent::Start,
                                padding: UiRect::all(Val::Px(10.0)),
                                row_gap: Val::Px(12.0),
                                column_gap: Val::Px(12.0),
                                ..default()
                            },
                        ))
                        .with_children(|content| {
                            // 生成 App 圖標
                            for app in PhoneApp::all_apps() {
                                spawn_app_icon(content, &font, *app);
                            }
                        });
                });

            // 底部導航提示
            phone
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(24.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(STATUS_BAR_BG),
                    BorderRadius::px(0.0, 0.0, 12.0, 12.0),
                ))
                .with_children(|nav| {
                    nav.spawn((
                        Text::new("[Arrows] Navigate  [Enter] Open  [Up] Back"),
                        TextFont {
                            font: font.clone(),
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                    ));
                });
        });
}

/// 生成單個 App 圖標
fn spawn_app_icon(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, app: PhoneApp) {
    parent
        .spawn((
            PhoneAppIcon { app },
            Node {
                width: Val::Px(56.0),
                height: Val::Px(56.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(APP_ICON_BG),
            BorderColor::all(Color::srgba(0.3, 0.3, 0.5, 0.5)),
            BorderRadius::all(Val::Px(10.0)),
        ))
        .with_children(|icon| {
            // 圖標字母
            icon.spawn((
                Text::new(app.icon()),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            // App 名稱
            icon.spawn((
                Text::new(app.label()),
                TextFont {
                    font: font.clone(),
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
            ));
        });
}

// ============================================================================
// 輸入系統
// ============================================================================

/// 手機輸入系統
/// 上箭頭開啟/關閉手機。方向鍵選擇 App，Enter 進入，Escape 返回。
pub fn phone_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut phone_state: ResMut<PhoneUiState>,
) {
    // 開啟/關閉手機
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        // 如果在某個 App 中，先回到主畫面
        if ui_state.show_phone && phone_state.current_app != PhoneApp::Home {
            phone_state.current_app = PhoneApp::Home;
            return;
        }
        ui_state.show_phone = !ui_state.show_phone;
        if ui_state.show_phone {
            phone_state.current_app = PhoneApp::Home;
            phone_state.selected_index = 0;
        }
        return;
    }

    // 手機未開啟時不處理
    if !ui_state.show_phone {
        return;
    }

    // Escape 關閉或返回
    if keyboard.just_pressed(KeyCode::Escape) {
        if phone_state.current_app != PhoneApp::Home {
            phone_state.current_app = PhoneApp::Home;
        } else {
            ui_state.show_phone = false;
        }
        return;
    }

    // 主畫面：方向鍵選擇
    if phone_state.current_app == PhoneApp::Home {
        let app_count = PhoneApp::all_apps().len();
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            phone_state.selected_index = (phone_state.selected_index + 1) % app_count;
        }
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            phone_state.selected_index = (phone_state.selected_index + app_count - 1) % app_count;
        }
        // 上下鍵也可以用（每行 2 個圖標）
        if keyboard.just_pressed(KeyCode::ArrowDown) {
            phone_state.selected_index = (phone_state.selected_index + 2).min(app_count - 1);
        }

        // Enter 進入選中 App
        if keyboard.just_pressed(KeyCode::Enter) {
            phone_state.current_app = PhoneApp::all_apps()[phone_state.selected_index];
        }
    }

    // 任務日誌分頁切換（左右鍵）
    else if phone_state.current_app == PhoneApp::MissionLog {
        let tabs = MissionJournalTab::all();
        let current_idx = tabs.iter().position(|t| *t == phone_state.journal_tab).unwrap_or(0);
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            phone_state.journal_tab = tabs[(current_idx + 1) % tabs.len()];
        }
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            phone_state.journal_tab = tabs[(current_idx + tabs.len() - 1) % tabs.len()];
        }
    }
    // 股市分頁切換 + 選股
    else if phone_state.current_app == PhoneApp::StockMarket {
        let tabs = StockMarketTab::all();
        let current_idx = tabs.iter().position(|t| *t == phone_state.stock_tab).unwrap_or(0);
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            phone_state.stock_tab = tabs[(current_idx + 1) % tabs.len()];
        }
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            phone_state.stock_tab = tabs[(current_idx + tabs.len() - 1) % tabs.len()];
        }
        // ArrowDown 循環選股
        if keyboard.just_pressed(KeyCode::ArrowDown) {
            phone_state.selected_stock_index =
                (phone_state.selected_stock_index + 1) % StockSymbol::ALL.len();
        }
        // 行情頁 Enter → 選股並跳到交易頁（設置 cooldown 防止同幀誤觸買入）
        if phone_state.stock_tab == StockMarketTab::StockList
            && keyboard.just_pressed(KeyCode::Enter)
        {
            phone_state.stock_tab = StockMarketTab::Trade;
            phone_state.trade_enter_cooldown = true;
        }
        // 交易頁 Q/E 調整數量
        if phone_state.stock_tab == StockMarketTab::Trade {
            if keyboard.just_pressed(KeyCode::KeyQ) {
                phone_state.trade_quantity = phone_state.trade_quantity.saturating_sub(1).max(1);
            }
            if keyboard.just_pressed(KeyCode::KeyE) {
                phone_state.trade_quantity = (phone_state.trade_quantity + 1).min(999);
            }
        }
    }
}

// ============================================================================
// 更新系統
// ============================================================================

/// 手機顯示/隱藏系統
pub fn phone_visibility_system(
    ui_state: Res<UiState>,
    mut phone_query: Query<&mut Visibility, With<PhoneContainer>>,
) {
    let target = if ui_state.show_phone {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut vis in &mut phone_query {
        *vis = target;
    }
}

/// 手機 App 圖標選中高亮系統
pub fn phone_icon_highlight_system(
    phone_state: Res<PhoneUiState>,
    mut icon_query: Query<(&PhoneAppIcon, &mut BackgroundColor)>,
) {
    if phone_state.current_app != PhoneApp::Home {
        return;
    }

    let apps = PhoneApp::all_apps();
    for (icon, mut bg) in &mut icon_query {
        let is_selected = apps
            .iter()
            .position(|a| *a == icon.app)
            .is_some_and(|idx| idx == phone_state.selected_index);

        *bg = if is_selected {
            BackgroundColor(APP_ICON_SELECTED)
        } else {
            BackgroundColor(APP_ICON_BG)
        };
    }
}

/// 手機內容更新系統（根據當前 App 切換顯示內容）
pub fn phone_content_system(
    phone_state: Res<PhoneUiState>,
    mission_manager: Res<MissionManager>,
    stock_market: Res<StockMarket>,
    wallet: Res<PlayerWallet>,
    game_state: Res<GameState>,
    vehicle_query: Query<&VehicleModifications>,
    mut content_query: Query<(Entity, &mut Node), With<PhoneContentArea>>,
    icon_query: Query<Entity, With<PhoneAppIcon>>,
    contact_query: Query<Entity, With<PhoneContactList>>,
    log_query: Query<Entity, With<PhoneMissionLogList>>,
    stock_query: Query<Entity, With<PhoneStockMarketList>>,
    mod_shop_query: Query<Entity, With<ModShopContent>>,
    mut commands: Commands,
    chinese_font: Res<ChineseFont>,
) {
    // 股市頁面需要在價格更新時也重建 UI
    let stock_changed =
        phone_state.current_app == PhoneApp::StockMarket && stock_market.is_changed();
    if !phone_state.is_changed() && !stock_changed {
        return;
    }

    let Ok((content_entity, mut content_node)) = content_query.single_mut() else {
        return;
    };

    // 清除舊內容（Bevy 0.17 的 despawn() 已自動清除子實體）
    for entity in icon_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in contact_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in log_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in stock_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in mod_shop_query.iter() {
        commands.entity(entity).despawn();
    }

    let font = chinese_font.font.clone();

    match phone_state.current_app {
        PhoneApp::Home => {
            // 顯示圖標網格
            content_node.flex_direction = FlexDirection::Row;
            content_node.flex_wrap = FlexWrap::Wrap;
            content_node.justify_content = JustifyContent::Center;
            content_node.align_content = AlignContent::Start;

            commands.entity(content_entity).with_children(|content| {
                for app in PhoneApp::all_apps() {
                    spawn_app_icon(content, &font, *app);
                }
            });
        }
        PhoneApp::Contacts => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.flex_wrap = FlexWrap::NoWrap;
            content_node.justify_content = JustifyContent::Start;

            commands.entity(content_entity).with_children(|content| {
                // 標題
                spawn_section_title(content, &font, "聯絡人");

                content
                    .spawn((
                        PhoneContactList,
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                    ))
                    .with_children(|list| {
                        // 固定聯絡人列表
                        let contacts = [
                            ("小明", "盟友"),
                            ("阿嬤", "家人"),
                            ("夜市老闆", "商人"),
                            ("警察局長", "官方"),
                        ];
                        for (name, role) in contacts {
                            spawn_contact_item(list, &font, name, role);
                        }
                    });
            });
        }
        PhoneApp::MissionLog => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.flex_wrap = FlexWrap::NoWrap;
            content_node.justify_content = JustifyContent::Start;

            commands.entity(content_entity).with_children(|content| {
                spawn_section_title(content, &font, "任務日誌");

                // 分頁選擇列
                spawn_journal_tabs(content, &font, phone_state.journal_tab);

                content
                    .spawn((
                        PhoneMissionLogList,
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            overflow: Overflow::clip(),
                            flex_grow: 1.0,
                            ..default()
                        },
                    ))
                    .with_children(|list| {
                        match phone_state.journal_tab {
                            MissionJournalTab::Active => {
                                spawn_journal_active(list, &font, &mission_manager);
                            }
                            MissionJournalTab::Completed => {
                                spawn_journal_completed(list, &font, &mission_manager);
                            }
                            MissionJournalTab::Stats => {
                                spawn_journal_stats(list, &font, &mission_manager);
                            }
                        }
                    });

                // 底部操作提示
                content.spawn((
                    Text::new("[Left/Right] 切換分頁"),
                    TextFont {
                        font: font.clone(),
                        font_size: 9.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.4, 0.4, 0.5, 0.7)),
                ));
            });
        }
        PhoneApp::Map => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.justify_content = JustifyContent::Center;

            commands.entity(content_entity).with_children(|content| {
                spawn_section_title(content, &font, "地圖");

                content.spawn((
                    Text::new("按 M 開啟全地圖"),
                    TextFont {
                        font: font.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.6, 0.7, 0.8, 0.9)),
                ));

                content.spawn((
                    Text::new("小地圖顯示於左下角"),
                    TextFont {
                        font: font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                ));
            });
        }
        PhoneApp::Settings => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.justify_content = JustifyContent::Start;

            commands.entity(content_entity).with_children(|content| {
                spawn_section_title(content, &font, "設定");

                let settings = [
                    "音量: 80%",
                    "畫質: 高",
                    "操控: 鍵盤滑鼠",
                    "語言: 繁體中文",
                ];
                for setting in settings {
                    content.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            margin: UiRect::bottom(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(CONTENT_ITEM_BG),
                        BorderRadius::all(Val::Px(4.0)),
                    )).with_children(|item| {
                        item.spawn((
                            Text::new(setting),
                            TextFont {
                                font: font.clone(),
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                        ));
                    });
                }
            });
        }
        PhoneApp::StockMarket => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.flex_wrap = FlexWrap::NoWrap;
            content_node.justify_content = JustifyContent::Start;

            commands.entity(content_entity).with_children(|content| {
                spawn_section_title(content, &font, "股市");
                spawn_stock_market_tabs(content, &font, phone_state.stock_tab);

                content
                    .spawn((
                        PhoneStockMarketList,
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            overflow: Overflow::clip(),
                            flex_grow: 1.0,
                            ..default()
                        },
                    ))
                    .with_children(|list| {
                        match phone_state.stock_tab {
                            StockMarketTab::StockList => {
                                spawn_stock_list(
                                    list,
                                    &font,
                                    &stock_market,
                                    phone_state.selected_stock_index,
                                );
                            }
                            StockMarketTab::Portfolio => {
                                spawn_stock_portfolio(list, &font, &stock_market);
                            }
                            StockMarketTab::Trade => {
                                spawn_stock_trade(
                                    list,
                                    &font,
                                    &stock_market,
                                    &wallet,
                                    phone_state.selected_stock_index,
                                    phone_state.trade_quantity,
                                );
                            }
                        }
                    });

                content.spawn((
                    Text::new("[←/→] 分頁　[↓] 選股"),
                    TextFont {
                        font: font.clone(),
                        font_size: 9.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.4, 0.4, 0.5, 0.7)),
                ));
            });
        }
        PhoneApp::ModShop => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.flex_wrap = FlexWrap::NoWrap;
            content_node.justify_content = JustifyContent::Start;

            commands.entity(content_entity).with_children(|content| {
                render_mod_shop_content(content, &font, &game_state, &vehicle_query, &wallet);
            });
        }
    }
}

/// 股票交易輸入系統（Enter 買入、Space 賣出）
pub fn stock_trade_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    ui_state: Res<UiState>,
    mut phone_state: ResMut<PhoneUiState>,
    mut market: ResMut<StockMarket>,
    mut wallet: ResMut<PlayerWallet>,
    mut notification: ResMut<NotificationQueue>,
) {
    if !ui_state.show_phone
        || phone_state.current_app != PhoneApp::StockMarket
        || phone_state.stock_tab != StockMarketTab::Trade
    {
        return;
    }

    // 從行情頁 Enter 切過來的同一幀，跳過交易以免誤觸
    if phone_state.trade_enter_cooldown {
        phone_state.trade_enter_cooldown = false;
        return;
    }

    let symbol = StockSymbol::ALL[phone_state.selected_stock_index];
    let quantity = phone_state.trade_quantity;

    // Enter 買入
    if keyboard.just_pressed(KeyCode::Enter) {
        match market.buy(symbol, quantity, &mut wallet) {
            Ok(price) => {
                let total = (price * quantity as f32).ceil() as i32;
                notification.success(format!(
                    "買入 {} {} 股，花費 ${}",
                    symbol.label(),
                    quantity,
                    total
                ));
                phone_state.stock_tab = StockMarketTab::Portfolio;
            }
            Err(msg) => {
                notification.warning(format!("買入失敗：{}", msg));
            }
        }
    }

    // Space 賣出
    if keyboard.just_pressed(KeyCode::Space) {
        match market.sell(symbol, quantity, &mut wallet) {
            Ok(price) => {
                let total = (price * quantity as f32).floor() as i32;
                notification.success(format!(
                    "賣出 {} {} 股，獲得 ${}",
                    symbol.label(),
                    quantity,
                    total
                ));
                phone_state.stock_tab = StockMarketTab::Portfolio;
            }
            Err(msg) => {
                notification.warning(format!("賣出失敗：{}", msg));
            }
        }
    }
}

pub(super) struct PhonePlugin;

impl Plugin for PhonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_phone_ui.in_set(super::UiSetup))
            .add_systems(
                Update,
                (
                    phone_input_system,
                    stock_trade_input_system.after(phone_input_system),
                    handle_mod_shop_buttons.after(phone_input_system),
                    phone_visibility_system.after(phone_input_system),
                    phone_icon_highlight_system.after(phone_input_system),
                    phone_content_system.after(phone_input_system),
                )
                    .in_set(super::UiActive),
            );
    }
}

