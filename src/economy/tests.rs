//! 經濟系統單元測試

use super::components::*;

// --- 測試輔助函數 ---

/// 建立測試用錢包（預設值：現金 1000，銀行 5000）
fn create_test_wallet() -> PlayerWallet {
    PlayerWallet {
        cash: 1000,
        bank: 5000,
        total_earned: 0,
        total_spent: 0,
    }
}

/// 建立指定現金金額的錢包
fn wallet_with_cash(cash: i32) -> PlayerWallet {
    PlayerWallet {
        cash,
        bank: 0,
        total_earned: 0,
        total_spent: 0,
    }
}

/// 建立空錢包
fn empty_wallet() -> PlayerWallet {
    PlayerWallet {
        cash: 0,
        bank: 0,
        total_earned: 0,
        total_spent: 0,
    }
}

// --- PlayerWallet 基本測試 ---

#[test]
fn test_wallet_default() {
    let wallet = PlayerWallet::default();
    assert_eq!(wallet.cash, 5000);
    assert_eq!(wallet.bank, 10000);
    assert_eq!(wallet.total_earned, 0);
    assert_eq!(wallet.total_spent, 0);
}

#[test]
fn test_wallet_total() {
    let wallet = create_test_wallet();
    assert_eq!(wallet.total(), 6000); // 1000 + 5000
}

#[test]
fn test_wallet_total_with_zero() {
    let wallet = empty_wallet();
    assert_eq!(wallet.total(), 0);
}

// --- add_cash 測試 ---

#[test]
fn test_wallet_add_cash() {
    let mut wallet = wallet_with_cash(100);
    let result = wallet.add_cash(500);

    assert_eq!(result, 600);
    assert_eq!(wallet.cash, 600);
    assert_eq!(wallet.total_earned, 500);
}

#[test]
fn test_wallet_add_cash_zero() {
    let mut wallet = wallet_with_cash(100);
    let result = wallet.add_cash(0);

    assert_eq!(result, 100);
    assert_eq!(wallet.cash, 100);
    assert_eq!(wallet.total_earned, 0); // Zero shouldn't count as earned
}

#[test]
fn test_wallet_add_cash_negative_does_not_track_earned() {
    let mut wallet = PlayerWallet {
        cash: 1000,
        total_earned: 100,
        ..Default::default()
    };

    wallet.add_cash(-200);

    assert_eq!(wallet.cash, 800);
    assert_eq!(wallet.total_earned, 100); // Should not increase for negative
}

#[test]
fn test_wallet_add_cash_large_amount() {
    let mut wallet = wallet_with_cash(0);
    let result = wallet.add_cash(i32::MAX / 2);

    assert_eq!(result, i32::MAX / 2);
    assert_eq!(wallet.total_earned, i32::MAX / 2);
}

// --- spend_cash 測試 ---

#[test]
fn test_wallet_spend_cash_success() {
    let mut wallet = wallet_with_cash(1000);
    let result = wallet.spend_cash(300);

    assert!(result);
    assert_eq!(wallet.cash, 700);
    assert_eq!(wallet.total_spent, 300);
}

#[test]
fn test_wallet_spend_cash_insufficient_funds() {
    let mut wallet = wallet_with_cash(100);
    let result = wallet.spend_cash(500);

    assert!(!result);
    assert_eq!(wallet.cash, 100); // Unchanged
    assert_eq!(wallet.total_spent, 0); // Unchanged
}

#[test]
fn test_wallet_spend_cash_exact_amount() {
    let mut wallet = wallet_with_cash(500);
    let result = wallet.spend_cash(500);

    assert!(result);
    assert_eq!(wallet.cash, 0);
    assert_eq!(wallet.total_spent, 500);
}

#[test]
fn test_wallet_spend_cash_zero() {
    let mut wallet = wallet_with_cash(1000);
    let result = wallet.spend_cash(0);

    // Zero spend should succeed (technically valid)
    assert!(result);
    assert_eq!(wallet.cash, 1000);
    assert_eq!(wallet.total_spent, 0);
}

#[test]
fn test_wallet_spend_cash_negative_amount() {
    let mut wallet = wallet_with_cash(1000);
    // Negative spend should be rejected
    let result = wallet.spend_cash(-100);

    assert!(!result);
    assert_eq!(wallet.cash, 1000); // Unchanged
    assert_eq!(wallet.total_spent, 0); // No spending recorded
}

// --- spend_up_to 測試 ---

#[test]
fn test_wallet_spend_up_to_full_amount() {
    let mut wallet = wallet_with_cash(1000);
    let actual = wallet.spend_up_to(500);

    assert_eq!(actual, 500);
    assert_eq!(wallet.cash, 500);
    assert_eq!(wallet.total_spent, 500);
}

#[test]
fn test_wallet_spend_up_to_partial_amount() {
    let mut wallet = wallet_with_cash(200);
    let actual = wallet.spend_up_to(500);

    assert_eq!(actual, 200); // Only what's available
    assert_eq!(wallet.cash, 0);
    assert_eq!(wallet.total_spent, 200);
}

#[test]
fn test_wallet_spend_up_to_zero_cash() {
    let mut wallet = empty_wallet();
    let actual = wallet.spend_up_to(500);

    assert_eq!(actual, 0);
    assert_eq!(wallet.total_spent, 0);
}

#[test]
fn test_wallet_spend_up_to_zero_amount() {
    let mut wallet = wallet_with_cash(1000);
    let actual = wallet.spend_up_to(0);

    assert_eq!(actual, 0);
    assert_eq!(wallet.cash, 1000); // Unchanged
    assert_eq!(wallet.total_spent, 0);
}

#[test]
fn test_wallet_spend_up_to_negative_amount() {
    let mut wallet = wallet_with_cash(1000);
    let actual = wallet.spend_up_to(-100);

    // Negative amount should be clamped to 0 by .max(0)
    assert_eq!(actual, 0);
    assert_eq!(wallet.cash, 1000); // Unchanged
    assert_eq!(wallet.total_spent, 0); // No spending recorded
}

#[test]
fn test_wallet_spend_up_to_negative_amount_large() {
    let mut wallet = wallet_with_cash(1000);
    let actual = wallet.spend_up_to(i32::MIN);

    // Even MIN should be clamped to 0
    assert_eq!(actual, 0);
    assert_eq!(wallet.cash, 1000);
}

// --- deposit 測試 ---

#[test]
fn test_wallet_deposit_success() {
    let mut wallet = create_test_wallet();
    let result = wallet.deposit(300);

    assert!(result);
    assert_eq!(wallet.cash, 700);
    assert_eq!(wallet.bank, 5300);
}

#[test]
fn test_wallet_deposit_insufficient_cash() {
    let mut wallet = wallet_with_cash(100);
    wallet.bank = 500;
    let result = wallet.deposit(200);

    assert!(!result);
    assert_eq!(wallet.cash, 100);
    assert_eq!(wallet.bank, 500);
}

#[test]
fn test_wallet_deposit_zero_amount() {
    let mut wallet = create_test_wallet();
    let result = wallet.deposit(0);

    assert!(!result); // Zero deposit not allowed
    assert_eq!(wallet.cash, 1000);
    assert_eq!(wallet.bank, 5000);
}

#[test]
fn test_wallet_deposit_negative_amount() {
    let mut wallet = create_test_wallet();
    let result = wallet.deposit(-100);

    // Negative deposit should fail (amount > 0 check)
    assert!(!result);
    assert_eq!(wallet.cash, 1000);
    assert_eq!(wallet.bank, 5000);
}

#[test]
fn test_wallet_deposit_exact_amount() {
    let mut wallet = wallet_with_cash(500);
    wallet.bank = 0;
    let result = wallet.deposit(500);

    assert!(result);
    assert_eq!(wallet.cash, 0);
    assert_eq!(wallet.bank, 500);
}

// --- withdraw 測試 ---

#[test]
fn test_wallet_withdraw_success() {
    let mut wallet = create_test_wallet();
    let result = wallet.withdraw(500);

    assert!(result);
    assert_eq!(wallet.cash, 1500);
    assert_eq!(wallet.bank, 4500);
}

#[test]
fn test_wallet_withdraw_insufficient_bank() {
    let mut wallet = wallet_with_cash(100);
    wallet.bank = 200;
    let result = wallet.withdraw(500);

    assert!(!result);
    assert_eq!(wallet.cash, 100);
    assert_eq!(wallet.bank, 200);
}

#[test]
fn test_wallet_withdraw_zero_amount() {
    let mut wallet = create_test_wallet();
    let result = wallet.withdraw(0);

    assert!(!result); // Zero withdraw not allowed
}

#[test]
fn test_wallet_withdraw_negative_amount() {
    let mut wallet = create_test_wallet();
    let result = wallet.withdraw(-100);

    // Negative withdraw should fail (amount > 0 check)
    assert!(!result);
    assert_eq!(wallet.cash, 1000);
    assert_eq!(wallet.bank, 5000);
}

#[test]
fn test_wallet_withdraw_exact_amount() {
    let mut wallet = wallet_with_cash(0);
    wallet.bank = 1000;
    let result = wallet.withdraw(1000);

    assert!(result);
    assert_eq!(wallet.cash, 1000);
    assert_eq!(wallet.bank, 0);
}

// --- 複合操作測試 ---

#[test]
fn test_wallet_multiple_operations() {
    let mut wallet = empty_wallet();

    // 獲得收入
    wallet.add_cash(10000);
    assert_eq!(wallet.cash, 10000);
    assert_eq!(wallet.total_earned, 10000);

    // 存入銀行
    assert!(wallet.deposit(7000));
    assert_eq!(wallet.cash, 3000);
    assert_eq!(wallet.bank, 7000);

    // 消費
    assert!(wallet.spend_cash(2000));
    assert_eq!(wallet.cash, 1000);
    assert_eq!(wallet.total_spent, 2000);

    // 提款
    assert!(wallet.withdraw(1000));
    assert_eq!(wallet.cash, 2000);
    assert_eq!(wallet.bank, 6000);

    // 總資產
    assert_eq!(wallet.total(), 8000);
}

#[test]
fn test_wallet_fine_scenario() {
    // 模擬罰款情境：玩家只有部分金額
    let mut wallet = wallet_with_cash(300);
    let fine_amount = 500;

    let paid = wallet.spend_up_to(fine_amount);

    assert_eq!(paid, 300); // 只能付 300
    assert_eq!(wallet.cash, 0);
    assert_eq!(wallet.total_spent, 300);
}

// --- Shop 測試 ---

#[test]
fn test_shop_is_open_regular_hours() {
    let shop = Shop {
        shop_type: ShopType::ConvenienceStore,
        name: "Test Shop".to_string(),
        is_open: true,
        open_hour: 9.0,
        close_hour: 21.0,
    };

    assert!(shop.is_open_at(9.0));
    assert!(shop.is_open_at(15.0));
    assert!(shop.is_open_at(20.9));
    assert!(!shop.is_open_at(8.0));
    assert!(!shop.is_open_at(21.0)); // Closed at exact closing time
    assert!(!shop.is_open_at(23.0));
}

#[test]
fn test_shop_is_open_cross_day() {
    // Night shop: 22:00 - 06:00
    let shop = Shop {
        shop_type: ShopType::ConvenienceStore,
        name: "Night Shop".to_string(),
        is_open: true,
        open_hour: 22.0,
        close_hour: 6.0,
    };

    assert!(shop.is_open_at(22.0));
    assert!(shop.is_open_at(0.0));
    assert!(shop.is_open_at(3.0));
    assert!(shop.is_open_at(5.9));
    assert!(!shop.is_open_at(6.0)); // Closed at closing time
    assert!(!shop.is_open_at(12.0));
    assert!(!shop.is_open_at(21.0));
}

#[test]
fn test_shop_always_open() {
    let shop = Shop::new(ShopType::ConvenienceStore, "24h Store").always_open();

    assert!(shop.is_open_at(0.0));
    assert!(shop.is_open_at(12.0));
    assert!(shop.is_open_at(23.9));
}

#[test]
fn test_shop_boundary_hours() {
    // 測試邊界情況：營業時間 0:00 - 24:00
    let shop = Shop {
        shop_type: ShopType::ConvenienceStore,
        name: "All Day".to_string(),
        is_open: true,
        open_hour: 0.0,
        close_hour: 24.0,
    };

    assert!(shop.is_open_at(0.0));
    assert!(shop.is_open_at(12.0));
    assert!(shop.is_open_at(23.99));
    assert!(!shop.is_open_at(24.0)); // Edge case
}

#[test]
fn test_shop_type_name() {
    assert_eq!(ShopType::ConvenienceStore.name(), "便利商店");
    assert_eq!(ShopType::WeaponShop.name(), "槍械行");
    assert_eq!(ShopType::ClothingStore.name(), "服飾店");
    assert_eq!(ShopType::VehicleDealer.name(), "車行");
    assert_eq!(ShopType::ModShop.name(), "改裝廠");
}

// --- ShopItem 測試 ---

#[test]
fn test_shop_item_builder() {
    let item = ShopItem::new("test_item", "Test Item", ItemCategory::Food, 100)
        .with_description("A test item")
        .with_effect(50.0)
        .with_stock(5);

    assert_eq!(item.id, "test_item");
    assert_eq!(item.name, "Test Item");
    assert_eq!(item.description, "A test item");
    assert_eq!(item.category, ItemCategory::Food);
    assert_eq!(item.price, 100);
    assert_eq!(item.effect_value, 50.0);
    assert_eq!(item.stock, 5);
}

#[test]
fn test_shop_item_default_stock() {
    let item = ShopItem::new("item", "Item", ItemCategory::Ammo, 50);

    assert_eq!(item.stock, -1); // Infinite by default
}

#[test]
fn test_shop_item_zero_price() {
    let item = ShopItem::new("free_item", "Free Item", ItemCategory::Food, 0);

    assert_eq!(item.price, 0);
}

// --- ShopInventory 測試 ---

#[test]
fn test_shop_inventory_convenience_items() {
    let inventory = ShopInventory::new();
    let items = inventory.get_items(ShopType::ConvenienceStore);

    assert!(!items.is_empty());
    assert!(items.iter().any(|i| i.category == ItemCategory::Food));
    assert!(items.iter().any(|i| i.category == ItemCategory::Drink));
}

#[test]
fn test_shop_inventory_weapon_items() {
    let inventory = ShopInventory::new();
    let items = inventory.get_items(ShopType::WeaponShop);

    assert!(!items.is_empty());
    assert!(items.iter().any(|i| i.category == ItemCategory::Ammo));
    assert!(items.iter().any(|i| i.category == ItemCategory::Weapon));
}

#[test]
fn test_shop_inventory_unsupported_type() {
    let inventory = ShopInventory::new();
    let items = inventory.get_items(ShopType::ClothingStore);

    assert!(items.is_empty());
}

#[test]
fn test_shop_inventory_all_items_have_valid_prices() {
    let inventory = ShopInventory::new();

    for item in &inventory.convenience_items {
        assert!(item.price >= 0, "Item {} has negative price", item.name);
    }

    for item in &inventory.weapon_items {
        assert!(item.price >= 0, "Item {} has negative price", item.name);
    }
}

// --- CashPickup 測試 ---

#[test]
fn test_cash_pickup_new() {
    let pickup = CashPickup::new(500);

    assert_eq!(pickup.amount, 500);
    assert_eq!(pickup.lifetime, 0.0);
    assert_eq!(pickup.max_lifetime, 60.0);
}

#[test]
fn test_cash_pickup_zero_amount() {
    let pickup = CashPickup::new(0);
    assert_eq!(pickup.amount, 0);
}

#[test]
fn test_cash_pickup_large_amount() {
    let pickup = CashPickup::new(1_000_000);
    assert_eq!(pickup.amount, 1_000_000);
}

// --- Atm 測試 ---

#[test]
fn test_atm_default() {
    let atm = Atm::default();

    assert_eq!(atm.name, "ATM");
    assert!(atm.is_functional);
    assert_eq!(atm.max_withdrawal, 10000);
}

// --- Interactable 測試 ---

#[test]
fn test_interactable_default() {
    let interactable = Interactable::default();

    assert_eq!(interactable.prompt, "按 F 互動");
    assert_eq!(interactable.range, 3.0);
    assert_eq!(interactable.interaction_type, InteractionType::Generic);
}

// --- 溢位保護測試 ---

#[test]
fn test_wallet_add_cash_saturating_overflow() {
    let mut wallet = wallet_with_cash(i32::MAX - 100);
    let result = wallet.add_cash(200);
    assert_eq!(result, i32::MAX); // 飽和加法，不溢位
    assert_eq!(wallet.cash, i32::MAX);
}

#[test]
fn test_wallet_total_saturating_overflow() {
    let wallet = PlayerWallet {
        cash: i32::MAX / 2 + 1,
        bank: i32::MAX / 2 + 1,
        total_earned: 0,
        total_spent: 0,
    };
    assert_eq!(wallet.total(), i32::MAX); // 飽和加法
}

#[test]
fn test_wallet_total_earned_saturating_overflow() {
    let mut wallet = PlayerWallet {
        cash: 0,
        bank: 0,
        total_earned: i32::MAX - 50,
        total_spent: 0,
    };
    wallet.add_cash(100);
    assert_eq!(wallet.total_earned, i32::MAX); // total_earned 飽和
    assert_eq!(wallet.cash, 100); // cash 正常增加
}

#[test]
fn test_wallet_deposit_bank_saturating_overflow() {
    let mut wallet = PlayerWallet {
        cash: 100,
        bank: i32::MAX - 50,
        total_earned: 0,
        total_spent: 0,
    };
    let result = wallet.deposit(100);
    assert!(result);
    assert_eq!(wallet.bank, i32::MAX); // 銀行餘額飽和
    assert_eq!(wallet.cash, 0); // 現金扣除成功
}

#[test]
fn test_wallet_withdraw_cash_saturating_overflow() {
    let mut wallet = PlayerWallet {
        cash: i32::MAX - 50,
        bank: 100,
        total_earned: 0,
        total_spent: 0,
    };
    let result = wallet.withdraw(100);
    assert!(result);
    assert_eq!(wallet.cash, i32::MAX); // 現金飽和
    assert_eq!(wallet.bank, 0); // 銀行扣除成功
}

#[test]
fn test_wallet_total_spent_saturating_overflow() {
    let mut wallet = PlayerWallet {
        cash: 100,
        bank: 0,
        total_earned: 0,
        total_spent: i32::MAX - 50,
    };
    let result = wallet.spend_cash(100);
    assert!(result);
    assert_eq!(wallet.total_spent, i32::MAX); // total_spent 飽和
}

// --- PropertyOwnership 測試 ---

#[test]
fn test_property_for_sale() {
    let property = PropertyOwnership::for_sale(50000, 500);

    assert!(!property.owned);
    assert_eq!(property.purchase_price, 50000);
    assert_eq!(property.daily_income, 500);
    assert_eq!(property.last_income_day, -1);
}

#[test]
fn test_property_purchase() {
    let mut property = PropertyOwnership::for_sale(50000, 500);
    property.purchase();

    assert!(property.owned);
}

#[test]
fn test_property_collect_income_owned() {
    let mut property = PropertyOwnership::for_sale(50000, 500);
    property.purchase();

    let income = property.collect_income(0);
    assert_eq!(income, 500);
    assert_eq!(property.last_income_day, 0);
}

#[test]
fn test_property_collect_income_not_owned() {
    let mut property = PropertyOwnership::for_sale(50000, 500);

    let income = property.collect_income(0);
    assert_eq!(income, 0); // 未擁有不收租
}

#[test]
fn test_property_collect_income_no_duplicate() {
    let mut property = PropertyOwnership::for_sale(50000, 500);
    property.purchase();

    let income1 = property.collect_income(0);
    let income2 = property.collect_income(0);

    assert_eq!(income1, 500);
    assert_eq!(income2, 0); // 同一天不重複收租
}

#[test]
fn test_property_collect_income_next_day() {
    let mut property = PropertyOwnership::for_sale(50000, 500);
    property.purchase();

    let income1 = property.collect_income(0);
    let income2 = property.collect_income(1);

    assert_eq!(income1, 500);
    assert_eq!(income2, 500); // 新的一天可以再收租
    assert_eq!(property.last_income_day, 1);
}

#[test]
fn test_property_has_collected_today() {
    let mut property = PropertyOwnership::for_sale(50000, 500);
    property.purchase();

    assert!(!property.has_collected_today(0));
    property.collect_income(0);
    assert!(property.has_collected_today(0));
    assert!(!property.has_collected_today(1)); // 新一天尚未收租
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn test_property_constants() {
    assert!(PROPERTY_INTERACTION_DISTANCE > 0.0);
    assert!(RENTAL_INCOME_HOUR >= 0.0);
    assert!(RENTAL_INCOME_HOUR < 24.0);
}

// --- RobberyState 測試 ---

#[test]
fn test_robbery_state_default() {
    let state = RobberyState::default();
    assert_eq!(state.cooldown, 0.0);
    assert!(state.can_rob());
}

#[test]
fn test_robbery_state_rob_sets_cooldown() {
    let mut state = RobberyState::default();
    state.rob();

    assert!(!state.can_rob());
    assert_eq!(state.cooldown, ROBBERY_COOLDOWN);
}

#[test]
fn test_robbery_state_tick_reduces_cooldown() {
    let mut state = RobberyState::default();
    state.rob();

    state.tick(100.0);
    assert!(!state.can_rob()); // 300 - 100 = 200，仍在冷卻

    state.tick(200.0);
    assert!(state.can_rob()); // 冷卻完畢
}

#[test]
fn test_robbery_state_tick_clamps_to_zero() {
    let mut state = RobberyState::default();
    state.rob();

    state.tick(999.0); // 超過冷卻時間
    assert_eq!(state.cooldown, 0.0);
    assert!(state.can_rob());
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn test_robbery_constants() {
    assert!(ROBBERY_INTERACTION_DISTANCE > 0.0);
    assert!(ROBBERY_MIN_AMOUNT > 0);
    assert!(ROBBERY_MAX_AMOUNT > ROBBERY_MIN_AMOUNT);
    assert!(ROBBERY_COOLDOWN > 0.0);
}
