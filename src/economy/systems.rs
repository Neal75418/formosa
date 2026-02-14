//! 經濟系統
//!
//! 處理金錢同步、商店互動、ATM 操作


use bevy::prelude::*;

use crate::combat::{Armor, CombatState, Health, Weapon, WeaponInventory, WeaponStats, WeaponType};
use crate::core::{InteractionState, PlayerStats, WorldTime};
use crate::player::Player;
use crate::ui::{MoneyDisplay, NotificationQueue};
use crate::wanted::CrimeEvent;
use crate::world::Building;

use super::components::*;

// ============================================================================
// 金錢同步系統
// ============================================================================

/// 同步金錢顯示（PlayerWallet -> PlayerStats）
/// 確保所有金錢來源保持一致
pub fn sync_money_display(
    wallet: Res<PlayerWallet>,
    mut player_stats: ResMut<PlayerStats>,
) {
    if wallet.is_changed() {
        player_stats.money = wallet.cash as u32;
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
    // 使用 unsigned_abs() 避免 i32::MIN.abs() 導致的整數溢位 panic
    let abs_amount = amount.unsigned_abs();
    let formatted = abs_amount
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or("?"))
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
    mut player_query: Query<(&Transform, &mut Health, &mut Armor, &mut WeaponInventory), With<Player>>,
    shop_query: Query<(Entity, &Transform, &Shop, &Interactable)>,
    mut interaction: ResMut<InteractionState>,
    mut notifications: ResMut<NotificationQueue>,
) {
    let Ok((player_transform, mut health, mut armor, mut weapon_inventory)) = player_query.single_mut() else {
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
            &mut health,
            &mut armor,
            &mut weapon_inventory,
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
            if interaction.can_interact() {
                notifications.info("店鋪已打烊，請稍後再來");
            }
            continue;
        }

        // 按 F 開啟商店
        if interaction.can_interact() {
            menu_state.is_open = true;
            menu_state.current_shop = Some(entity);
            menu_state.shop_type = Some(shop.shop_type);
            menu_state.selected_index = 0;
            interaction.consume();
            info!("🏪 開啟商店: {}", shop.name);
            break;
        }
    }
}

// ============================================================================
// 商店選單輔助函數
// ============================================================================
/// 處理商店選單導航（上下選擇）
fn handle_shop_navigation(keyboard: &ButtonInput<KeyCode>, selected_index: &mut usize, item_count: usize) {
    let up_pressed = keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp);
    let down_pressed = keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown);

    if up_pressed && *selected_index > 0 {
        *selected_index -= 1;
    }
    if down_pressed && *selected_index < item_count - 1 {
        *selected_index += 1;
    }
}

/// 嘗試購買商品
fn try_purchase_item(
    item: &ShopItem,
    wallet: &mut PlayerWallet,
    money_events: &mut MessageWriter<MoneyChangedEvent>,
    health: &mut Health,
    armor: &mut Armor,
    weapon_inventory: &mut WeaponInventory,
) {
    if !wallet.spend_cash(item.price) {
        warn!("餘額不足！需要 ${}, 只有 ${}", item.price, wallet.cash);
        return;
    }

    // 根據物品類別給予效果
    match item.category {
        ItemCategory::Food | ItemCategory::Drink => {
            let healed = health.heal(item.effect_value);
            info!("🛒 購買: {} (-${}), 回復 {} HP", item.name, item.price, healed);
        }
        ItemCategory::Armor => {
            let added = armor.add(item.effect_value);
            info!("🛒 購買: {} (-${}), 護甲 +{}", item.name, item.price, added);
        }
        ItemCategory::Weapon => {
            // 根據 item.id 確定武器類型
            let weapon = match item.id.as_str() {
                ITEM_WEAPON_PISTOL => Weapon::new(WeaponStats::pistol()),
                ITEM_WEAPON_SMG => Weapon::new(WeaponStats::smg()),
                ITEM_WEAPON_SHOTGUN => Weapon::new(WeaponStats::shotgun()),
                ITEM_WEAPON_RIFLE => Weapon::new(WeaponStats::rifle()),
                _ => {
                    warn!("未知武器 ID: {}", item.id);
                    return;
                }
            };
            if weapon_inventory.add_weapon(weapon) {
                info!("🛒 購買: {} (-${})", item.name, item.price);
            } else {
                info!("🛒 購買: {} (-${}), 已有此武器，補充彈藥", item.name, item.price);
            }
        }
        ItemCategory::Ammo => {
            // 根據 item.id 確定彈藥類型並補充
            let weapon_type = match item.id.as_str() {
                ITEM_AMMO_PISTOL => WeaponType::Pistol,
                ITEM_AMMO_SMG => WeaponType::SMG,
                ITEM_AMMO_SHOTGUN => WeaponType::Shotgun,
                ITEM_AMMO_RIFLE => WeaponType::Rifle,
                _ => {
                    warn!("未知彈藥 ID: {}", item.id);
                    return;
                }
            };
            // 找到對應武器並補充彈藥
            let ammo_added = item.effect_value as u32;
            let mut found = false;
            for weapon in &mut weapon_inventory.weapons {
                if weapon.stats.weapon_type == weapon_type {
                    weapon.reserve_ammo = (weapon.reserve_ammo + ammo_added).min(weapon.stats.max_ammo);
                    found = true;
                    break;
                }
            }
            if found {
                info!("🛒 購買: {} (-${}), 彈藥 +{}", item.name, item.price, ammo_added);
            } else {
                info!("🛒 購買: {} (-${}), 尚未擁有此武器，彈藥已儲存", item.name, item.price);
            }
        }
        ItemCategory::Clothing | ItemCategory::VehiclePart => {
            info!("🛒 購買: {} (-${}), 功能開發中", item.name, item.price);
        }
    }

    money_events.write(MoneyChangedEvent {
        amount: -item.price,
        reason: MoneyChangeReason::Purchase,
        new_balance: wallet.cash,
    });
}

/// 處理商店選單輸入
fn handle_shop_menu_input(
    keyboard: &ButtonInput<KeyCode>,
    wallet: &mut PlayerWallet,
    menu_state: &mut ShopMenuState,
    money_events: &mut MessageWriter<MoneyChangedEvent>,
    shop_inventory: &ShopInventory,
    health: &mut Health,
    armor: &mut Armor,
    weapon_inventory: &mut WeaponInventory,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        menu_state.is_open = false;
        menu_state.current_shop = None;
        return;
    }

    let Some(shop_type) = menu_state.shop_type else { return };
    let items = shop_inventory.get_items(shop_type);
    if items.is_empty() { return }

    handle_shop_navigation(keyboard, &mut menu_state.selected_index, items.len());

    let purchase_pressed = keyboard.just_pressed(KeyCode::Enter);
    if purchase_pressed {
        if let Some(item) = items.get(menu_state.selected_index) {
            try_purchase_item(item, wallet, money_events, health, armor, weapon_inventory);
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
    mut interaction: ResMut<InteractionState>,
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

        // 按 F 開啟 ATM
        if interaction.can_interact() {
            menu_state.is_open = true;
            menu_state.current_atm = Some(entity);
            menu_state.mode = AtmMode::Main;
            menu_state.input_amount = 0;
            interaction.consume();
            info!("🏧 開啟 ATM: {}", atm.name);
            break;
        }
    }
}

// ============================================================================
// ATM 選單輔助函數
// ============================================================================
/// 處理 ATM ESC 按鍵
/// 返回 true 表示已處理並應返回
fn handle_atm_escape(keyboard: &ButtonInput<KeyCode>, menu_state: &mut AtmMenuState) -> bool {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return false;
    }

    if matches!(menu_state.mode, AtmMode::Main) {
        menu_state.is_open = false;
        menu_state.current_atm = None;
    } else {
        menu_state.mode = AtmMode::Main;
        menu_state.input_amount = 0;
    }
    true
}

/// 處理 ATM 主選單輸入
fn handle_atm_main_menu(keyboard: &ButtonInput<KeyCode>, menu_state: &mut AtmMenuState) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        menu_state.mode = AtmMode::Withdraw;
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        menu_state.mode = AtmMode::Deposit;
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        menu_state.mode = AtmMode::Balance;
    }
}

/// 處理 ATM 提款操作
fn handle_atm_withdraw(
    keyboard: &ButtonInput<KeyCode>,
    wallet: &mut PlayerWallet,
    menu_state: &mut AtmMenuState,
    money_events: &mut MessageWriter<MoneyChangedEvent>,
) {
    handle_amount_input(keyboard, &mut menu_state.input_amount);

    if !keyboard.just_pressed(KeyCode::Enter) { return }

    let amount = menu_state.input_amount;
    if amount > 0 && wallet.withdraw(amount) {
        money_events.write(MoneyChangedEvent {
            amount,
            reason: MoneyChangeReason::AtmWithdraw,
            new_balance: wallet.cash,
        });
        info!("🏧 提款: ${}", amount);
        menu_state.mode = AtmMode::Main;
        menu_state.input_amount = 0;
    } else {
        warn!("提款失敗！銀行餘額: ${}", wallet.bank);
    }
}

/// 處理 ATM 存款操作
fn handle_atm_deposit(
    keyboard: &ButtonInput<KeyCode>,
    wallet: &mut PlayerWallet,
    menu_state: &mut AtmMenuState,
    money_events: &mut MessageWriter<MoneyChangedEvent>,
) {
    handle_amount_input(keyboard, &mut menu_state.input_amount);

    if !keyboard.just_pressed(KeyCode::Enter) { return }

    let amount = menu_state.input_amount;
    if amount > 0 && wallet.deposit(amount) {
        money_events.write(MoneyChangedEvent {
            amount: -amount,
            reason: MoneyChangeReason::AtmDeposit,
            new_balance: wallet.cash,
        });
        info!("🏧 存款: ${}", amount);
        menu_state.mode = AtmMode::Main;
        menu_state.input_amount = 0;
    } else {
        warn!("存款失敗！現金餘額: ${}", wallet.cash);
    }
}

/// 處理 ATM 選單輸入
fn handle_atm_menu_input(
    keyboard: &ButtonInput<KeyCode>,
    wallet: &mut PlayerWallet,
    menu_state: &mut AtmMenuState,
    money_events: &mut MessageWriter<MoneyChangedEvent>,
) {
    if handle_atm_escape(keyboard, menu_state) { return }

    match menu_state.mode {
        AtmMode::Main => handle_atm_main_menu(keyboard, menu_state),
        AtmMode::Withdraw => handle_atm_withdraw(keyboard, wallet, menu_state, money_events),
        AtmMode::Deposit => handle_atm_deposit(keyboard, wallet, menu_state, money_events),
        AtmMode::Balance => {
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
            info!("💰 撿取 ${}", pickup.amount);
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 生成金錢掉落物
#[allow(dead_code)]
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

// ============================================================================
// 房產購買系統
// ============================================================================

/// 房產購買系統
/// 玩家按 F 鍵接近可購買的建築，支付購買價格取得擁有權
pub fn property_purchase_system(
    mut wallet: ResMut<PlayerWallet>,
    mut money_events: MessageWriter<MoneyChangedEvent>,
    mut notifications: ResMut<NotificationQueue>,
    mut interaction: ResMut<InteractionState>,
    player_query: Query<&Transform, With<Player>>,
    mut property_query: Query<(&Transform, &Building, &mut PropertyOwnership)>,
) {
    if !interaction.can_interact() {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let interact_dist_sq = PROPERTY_INTERACTION_DISTANCE * PROPERTY_INTERACTION_DISTANCE;

    for (building_transform, building, mut property) in property_query.iter_mut() {
        if property.owned {
            continue;
        }

        let dist_sq = building_transform.translation.distance_squared(player_pos);
        if dist_sq > interact_dist_sq {
            continue;
        }

        // 嘗試購買
        if wallet.spend_cash(property.purchase_price) {
            property.purchase();
            interaction.consume();

            money_events.write(MoneyChangedEvent {
                amount: -property.purchase_price,
                reason: MoneyChangeReason::PropertyPurchase,
                new_balance: wallet.cash,
            });

            notifications.info(format!(
                "購買 {} (-${}), 每日收入 ${}",
                building.name, property.purchase_price, property.daily_income
            ));

            info!(
                "購買房產: {} (${}) — 每日收入: ${}",
                building.name, property.purchase_price, property.daily_income
            );
            return; // 一次只能購買一棟
        } else {
            notifications.warning(format!(
                "資金不足！需要 ${}",
                property.purchase_price
            ));
            return;
        }
    }
}

/// 租金收入系統
/// 每天早上 6 點自動發放已擁有房產的租金
pub fn rental_income_system(
    world_time: Res<WorldTime>,
    mut wallet: ResMut<PlayerWallet>,
    mut money_events: MessageWriter<MoneyChangedEvent>,
    mut notifications: ResMut<NotificationQueue>,
    mut property_query: Query<(&Building, &mut PropertyOwnership)>,
) {
    // 只在早上 6 點前後的 0.5 小時窗口內檢查（避免漏掉）
    let hour = world_time.hour;
    if (hour - RENTAL_INCOME_HOUR).abs() > 0.5 {
        return;
    }

    // WorldTime.hour 是 0-24 循環
    // last_income_day 初始為 -1，確保首次觸發
    // 每次 hour 在 6.0 附近且 last_income_day != 0 時收租
    let game_day = 0_i32; // 同一天內只收一次

    let mut total_income = 0;
    let mut property_count = 0;

    for (building, mut property) in property_query.iter_mut() {
        let income = property.collect_income(game_day);
        if income > 0 {
            total_income += income;
            property_count += 1;
            debug!("租金收入: {} → ${}", building.name, income);
        }
    }

    if total_income > 0 {
        wallet.add_cash(total_income);

        money_events.write(MoneyChangedEvent {
            amount: total_income,
            reason: MoneyChangeReason::RentalIncome,
            new_balance: wallet.cash,
        });

        notifications.info(format!(
            "租金收入 +${} ({} 物件)",
            total_income, property_count
        ));

        info!("每日租金收入: ${} ({} 物件)", total_income, property_count);
    }
}

// ============================================================================
// 商店搶劫系統
// ============================================================================

/// 搶劫冷卻更新系統
pub fn robbery_cooldown_system(
    time: Res<Time>,
    mut robbery_query: Query<&mut RobberyState>,
) {
    let dt = time.delta_secs();
    for mut robbery in robbery_query.iter_mut() {
        robbery.tick(dt);
    }
}

/// 商店搶劫系統
/// 玩家持槍瞄準接近商店時按 F 搶劫，獲得隨機金額，觸發通緝
pub fn store_robbery_system(
    combat_state: Res<CombatState>,
    mut wallet: ResMut<PlayerWallet>,
    mut money_events: MessageWriter<MoneyChangedEvent>,
    mut crime_events: MessageWriter<CrimeEvent>,
    mut notifications: ResMut<NotificationQueue>,
    mut interaction: ResMut<InteractionState>,
    player_query: Query<(&Transform, &WeaponInventory), With<Player>>,
    mut shop_query: Query<(&Transform, &Shop, &mut RobberyState)>,
) {
    if !combat_state.is_aiming || !interaction.can_interact() {
        return;
    }

    let Ok((player_transform, weapon_inventory)) = player_query.single() else {
        return;
    };

    // 必須裝備武器
    if weapon_inventory.current_weapon().is_none() {
        return;
    }

    let player_pos = player_transform.translation;
    let rob_dist_sq = ROBBERY_INTERACTION_DISTANCE * ROBBERY_INTERACTION_DISTANCE;

    for (shop_transform, shop, mut robbery) in shop_query.iter_mut() {
        let dist_sq = shop_transform.translation.distance_squared(player_pos);
        if dist_sq > rob_dist_sq {
            continue;
        }

        if !robbery.can_rob() {
            notifications.warning("這家店最近已被搶過，不宜再來");
            interaction.consume();
            return;
        }

        // 執行搶劫
        robbery.rob();
        interaction.consume();

        let amount = ROBBERY_MIN_AMOUNT
            + (rand::random::<f32>() * (ROBBERY_MAX_AMOUNT - ROBBERY_MIN_AMOUNT) as f32) as i32;

        wallet.add_cash(amount);

        money_events.write(MoneyChangedEvent {
            amount,
            reason: MoneyChangeReason::Robbery,
            new_balance: wallet.cash,
        });

        crime_events.write(CrimeEvent::ShopRobbery {
            position: shop_transform.translation,
        });

        notifications.warning(format!(
            "搶劫 {} 得手 ${}！通緝上升！",
            shop.name, amount
        ));

        info!(
            "搶劫商店: {} → +${} (位置: {:.1}, {:.1})",
            shop.name, amount, shop_transform.translation.x, shop_transform.translation.z
        );
        return; // 一次只搶一家
    }
}
