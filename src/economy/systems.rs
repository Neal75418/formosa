//! 經濟系統
//!
//! 處理金錢同步、商店互動、ATM 操作

#![allow(dead_code)] // Phase 5+ 預留功能

use bevy::prelude::*;

use crate::core::{PlayerStats, WorldTime};
use crate::player::Player;
use crate::ui::MoneyDisplay;

use super::components::*;

// ============================================================================
// 金錢同步系統
// ============================================================================

/// 同步金錢顯示（PlayerWallet -> PlayerStats -> Player）
/// 確保所有金錢來源保持一致
pub fn sync_money_display(
    wallet: Res<PlayerWallet>,
    mut player_stats: ResMut<PlayerStats>,
    mut player_query: Query<&mut Player>,
) {
    // 只在錢包變動時同步
    if wallet.is_changed() {
        let cash = wallet.cash as u32;

        // 同步到 PlayerStats（HUD 用）
        player_stats.money = cash;

        // 同步到 Player 組件
        for mut player in player_query.iter_mut() {
            player.money = cash;
        }
    }
}

/// 更新金錢 UI 顯示
pub fn update_money_ui(
    wallet: Res<PlayerWallet>,
    mut money_query: Query<&mut Text, With<MoneyDisplay>>,
) {
    if !wallet.is_changed() {
        return;
    }

    for mut text in money_query.iter_mut() {
        // 格式化金錢顯示（加入千分位）
        let formatted = format_money(wallet.cash);
        **text = format!("$ {}", formatted);
    }
}

/// 格式化金錢（加入千分位分隔符）
fn format_money(amount: i32) -> String {
    let abs_amount = amount.abs();
    let formatted = abs_amount
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(",");

    if amount < 0 {
        format!("-{}", formatted)
    } else {
        formatted
    }
}

// ============================================================================
// 商店互動系統
// ============================================================================

/// 處理商店互動
pub fn handle_shop_interaction(
    keyboard: Res<ButtonInput<KeyCode>>,
    _time: Res<Time>,
    world_time: Res<WorldTime>,
    mut wallet: ResMut<PlayerWallet>,
    mut menu_state: ResMut<ShopMenuState>,
    mut money_events: MessageWriter<MoneyChangedEvent>,
    shop_inventory: Res<ShopInventory>,
    player_query: Query<&Transform, With<Player>>,
    shop_query: Query<(Entity, &Transform, &Shop, &Interactable)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 如果商店選單已開啟
    if menu_state.is_open {
        handle_shop_menu_input(
            &keyboard,
            &mut wallet,
            &mut menu_state,
            &mut money_events,
            &shop_inventory,
        );
        return;
    }

    // 檢查是否有可互動的商店
    for (entity, shop_transform, shop, interactable) in shop_query.iter() {
        if interactable.interaction_type != InteractionType::Shop {
            continue;
        }

        let distance_sq = player_pos.distance_squared(shop_transform.translation);
        let range_sq = interactable.range * interactable.range;

        if distance_sq > range_sq {
            continue;
        }

        // 檢查營業時間
        if !shop.is_open_at(world_time.hour) {
            // TODO: 顯示「店鋪已打烊」提示
            continue;
        }

        // 按 E 開啟商店
        if keyboard.just_pressed(KeyCode::KeyE) {
            menu_state.is_open = true;
            menu_state.current_shop = Some(entity);
            menu_state.shop_type = Some(shop.shop_type);
            menu_state.selected_index = 0;
            info!("開啟商店: {}", shop.name);
        }
    }
}

/// 處理商店選單輸入
fn handle_shop_menu_input(
    keyboard: &ButtonInput<KeyCode>,
    wallet: &mut PlayerWallet,
    menu_state: &mut ShopMenuState,
    money_events: &mut MessageWriter<MoneyChangedEvent>,
    shop_inventory: &ShopInventory,
) {
    // ESC 關閉選單
    if keyboard.just_pressed(KeyCode::Escape) {
        menu_state.is_open = false;
        menu_state.current_shop = None;
        return;
    }

    let Some(shop_type) = menu_state.shop_type else {
        return;
    };

    let items = shop_inventory.get_items(shop_type);
    if items.is_empty() {
        return;
    }

    // 上下選擇
    if keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp) {
        if menu_state.selected_index > 0 {
            menu_state.selected_index -= 1;
        }
    }
    if keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown) {
        if menu_state.selected_index < items.len() - 1 {
            menu_state.selected_index += 1;
        }
    }

    // 購買
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::KeyE) {
        if let Some(item) = items.get(menu_state.selected_index) {
            if wallet.spend_cash(item.price) {
                // 購買成功
                money_events.write(MoneyChangedEvent {
                    amount: -item.price,
                    reason: MoneyChangeReason::Purchase,
                    new_balance: wallet.cash,
                });
                info!("購買成功: {} (-${})", item.name, item.price);

                // TODO: 實際給予物品效果
                // 根據 item.category 和 item.effect_value 處理
            } else {
                // 餘額不足
                warn!("餘額不足！需要 ${}, 只有 ${}", item.price, wallet.cash);
            }
        }
    }
}

// ============================================================================
// ATM 互動系統
// ============================================================================

/// 處理 ATM 互動
pub fn handle_atm_interaction(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut wallet: ResMut<PlayerWallet>,
    mut menu_state: ResMut<AtmMenuState>,
    mut money_events: MessageWriter<MoneyChangedEvent>,
    player_query: Query<&Transform, With<Player>>,
    atm_query: Query<(Entity, &Transform, &Atm, &Interactable)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 如果 ATM 選單已開啟
    if menu_state.is_open {
        handle_atm_menu_input(
            &keyboard,
            &mut wallet,
            &mut menu_state,
            &mut money_events,
        );
        return;
    }

    // 檢查是否有可互動的 ATM
    for (entity, atm_transform, atm, interactable) in atm_query.iter() {
        if interactable.interaction_type != InteractionType::Atm {
            continue;
        }

        if !atm.is_functional {
            continue;
        }

        let distance_sq = player_pos.distance_squared(atm_transform.translation);
        let range_sq = interactable.range * interactable.range;

        if distance_sq > range_sq {
            continue;
        }

        // 按 E 開啟 ATM
        if keyboard.just_pressed(KeyCode::KeyE) {
            menu_state.is_open = true;
            menu_state.current_atm = Some(entity);
            menu_state.mode = AtmMode::Main;
            menu_state.input_amount = 0;
            info!("開啟 ATM: {}", atm.name);
        }
    }
}

/// 處理 ATM 選單輸入
fn handle_atm_menu_input(
    keyboard: &ButtonInput<KeyCode>,
    wallet: &mut PlayerWallet,
    menu_state: &mut AtmMenuState,
    money_events: &mut MessageWriter<MoneyChangedEvent>,
) {
    // ESC 返回/關閉
    if keyboard.just_pressed(KeyCode::Escape) {
        match menu_state.mode {
            AtmMode::Main => {
                menu_state.is_open = false;
                menu_state.current_atm = None;
            }
            _ => {
                menu_state.mode = AtmMode::Main;
                menu_state.input_amount = 0;
            }
        }
        return;
    }

    match menu_state.mode {
        AtmMode::Main => {
            // 主選單選項
            if keyboard.just_pressed(KeyCode::Digit1) {
                menu_state.mode = AtmMode::Withdraw;
            } else if keyboard.just_pressed(KeyCode::Digit2) {
                menu_state.mode = AtmMode::Deposit;
            } else if keyboard.just_pressed(KeyCode::Digit3) {
                menu_state.mode = AtmMode::Balance;
            }
        }
        AtmMode::Withdraw => {
            // 提款金額輸入
            handle_amount_input(keyboard, &mut menu_state.input_amount);

            if keyboard.just_pressed(KeyCode::Enter) {
                let amount = menu_state.input_amount;
                if amount > 0 && wallet.withdraw(amount) {
                    money_events.write(MoneyChangedEvent {
                        amount,
                        reason: MoneyChangeReason::AtmWithdraw,
                        new_balance: wallet.cash,
                    });
                    info!("提款成功: ${}", amount);
                    menu_state.mode = AtmMode::Main;
                    menu_state.input_amount = 0;
                } else {
                    warn!("提款失敗！銀行餘額: ${}", wallet.bank);
                }
            }
        }
        AtmMode::Deposit => {
            // 存款金額輸入
            handle_amount_input(keyboard, &mut menu_state.input_amount);

            if keyboard.just_pressed(KeyCode::Enter) {
                let amount = menu_state.input_amount;
                if amount > 0 && wallet.deposit(amount) {
                    money_events.write(MoneyChangedEvent {
                        amount: -amount,
                        reason: MoneyChangeReason::AtmDeposit,
                        new_balance: wallet.cash,
                    });
                    info!("存款成功: ${}", amount);
                    menu_state.mode = AtmMode::Main;
                    menu_state.input_amount = 0;
                } else {
                    warn!("存款失敗！現金餘額: ${}", wallet.cash);
                }
            }
        }
        AtmMode::Balance => {
            // 顯示餘額，按任意鍵返回
            if keyboard.get_just_pressed().len() > 0 {
                menu_state.mode = AtmMode::Main;
            }
        }
    }
}

/// 處理金額數字輸入
fn handle_amount_input(keyboard: &ButtonInput<KeyCode>, amount: &mut i32) {
    // 快速金額按鈕
    if keyboard.just_pressed(KeyCode::Digit1) {
        *amount = 100;
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        *amount = 500;
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        *amount = 1000;
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        *amount = 5000;
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        *amount = 10000;
    }

    // 清除
    if keyboard.just_pressed(KeyCode::Backspace) {
        *amount = 0;
    }
}

// ============================================================================
// 交易處理系統
// ============================================================================

/// 處理交易事件
pub fn process_transactions(
    mut events: MessageReader<TransactionEvent>,
    mut wallet: ResMut<PlayerWallet>,
    mut money_events: MessageWriter<MoneyChangedEvent>,
) {
    for event in events.read() {
        match event.transaction_type {
            TransactionType::BankDeposit => {
                if wallet.deposit(event.amount) {
                    money_events.write(MoneyChangedEvent {
                        amount: -event.amount,
                        reason: MoneyChangeReason::AtmDeposit,
                        new_balance: wallet.cash,
                    });
                }
            }
            TransactionType::BankWithdraw => {
                if wallet.withdraw(event.amount) {
                    money_events.write(MoneyChangedEvent {
                        amount: event.amount,
                        reason: MoneyChangeReason::AtmWithdraw,
                        new_balance: wallet.cash,
                    });
                }
            }
            TransactionType::BuyItem => {
                if wallet.spend_cash(event.amount) {
                    money_events.write(MoneyChangedEvent {
                        amount: -event.amount,
                        reason: MoneyChangeReason::Purchase,
                        new_balance: wallet.cash,
                    });
                }
            }
            TransactionType::PayFine => {
                if wallet.spend_cash(event.amount) {
                    money_events.write(MoneyChangedEvent {
                        amount: -event.amount,
                        reason: MoneyChangeReason::Fine,
                        new_balance: wallet.cash,
                    });
                }
            }
        }
    }
}

// ============================================================================
// 金錢掉落系統
// ============================================================================

/// 更新金錢掉落物
pub fn update_cash_pickups(
    mut commands: Commands,
    time: Res<Time>,
    mut wallet: ResMut<PlayerWallet>,
    mut money_events: MessageWriter<MoneyChangedEvent>,
    player_query: Query<&Transform, With<Player>>,
    mut pickup_query: Query<(Entity, &Transform, &mut CashPickup)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    const PICKUP_RANGE_SQ: f32 = 4.0; // 2m 撿取距離

    for (entity, pickup_transform, mut pickup) in pickup_query.iter_mut() {
        // 更新存在時間
        pickup.lifetime += time.delta_secs();

        // 超時消失
        if pickup.lifetime >= pickup.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // 檢查玩家距離
        let distance_sq = player_pos.distance_squared(pickup_transform.translation);
        if distance_sq <= PICKUP_RANGE_SQ {
            // 撿取金錢
            wallet.add_cash(pickup.amount);
            money_events.write(MoneyChangedEvent {
                amount: pickup.amount,
                reason: MoneyChangeReason::Pickup,
                new_balance: wallet.cash,
            });
            info!("撿取 ${}", pickup.amount);
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 生成金錢掉落物
pub fn spawn_cash_pickup(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    amount: i32,
) -> Entity {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.3, 0.1, 0.2))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.8, 0.2), // 綠色代表金錢
            emissive: LinearRgba::rgb(0.1, 0.4, 0.1),
            ..default()
        })),
        Transform::from_translation(position),
        CashPickup::new(amount),
        Interactable {
            prompt: format!("${}", amount),
            range: 2.0,
            interaction_type: InteractionType::Pickup,
        },
    )).id()
}
