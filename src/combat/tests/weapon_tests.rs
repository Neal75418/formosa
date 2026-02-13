//! Weapon、WeaponInventory、Weapon 方法測試

use crate::combat::*;

// ============================================================================
// Weapon 測試
// ============================================================================

#[test]
fn test_weapon_new() {
    let weapon = Weapon::new(WeaponStats::pistol());
    assert_eq!(weapon.current_ammo, 12);
    assert_eq!(weapon.reserve_ammo, 120);
    assert_eq!(weapon.fire_cooldown, 0.0);
    assert!(!weapon.is_reloading);
}

#[test]
fn test_weapon_can_fire() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    assert!(weapon.can_fire());

    weapon.fire_cooldown = 0.1;
    assert!(!weapon.can_fire());

    weapon.fire_cooldown = 0.0;
    weapon.is_reloading = true;
    assert!(!weapon.can_fire());

    weapon.is_reloading = false;
    weapon.current_ammo = 0;
    assert!(!weapon.can_fire());
}

#[test]
fn test_weapon_can_fire_melee() {
    let weapon = Weapon::new(WeaponStats::fist());
    // 近戰武器無彈匣限制
    assert!(weapon.can_fire());
}

#[test]
fn test_weapon_consume_ammo() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    assert_eq!(weapon.current_ammo, 12);

    weapon.consume_ammo();
    assert_eq!(weapon.current_ammo, 11);

    // 消耗到 0
    weapon.current_ammo = 1;
    weapon.consume_ammo();
    assert_eq!(weapon.current_ammo, 0);

    // 0 時不會變負數
    weapon.consume_ammo();
    assert_eq!(weapon.current_ammo, 0);
}

#[test]
fn test_weapon_consume_ammo_melee() {
    let mut weapon = Weapon::new(WeaponStats::fist());
    // 近戰武器不消耗彈藥
    weapon.consume_ammo();
    assert_eq!(weapon.current_ammo, 0);
}

#[test]
fn test_weapon_start_reload() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.current_ammo = 0;

    assert!(weapon.start_reload());
    assert!(weapon.is_reloading);
    assert_eq!(weapon.reload_timer, 1.5);
}

#[test]
fn test_weapon_start_reload_fails() {
    let mut weapon = Weapon::new(WeaponStats::pistol());

    // 彈匣滿時無法換彈
    assert!(!weapon.start_reload());

    // 正在換彈時無法再次換彈
    weapon.current_ammo = 0;
    weapon.start_reload();
    assert!(!weapon.start_reload());

    // 無後備彈藥時無法換彈
    let mut empty_weapon = Weapon::new(WeaponStats::pistol());
    empty_weapon.current_ammo = 0;
    empty_weapon.reserve_ammo = 0;
    assert!(!empty_weapon.start_reload());
}

#[test]
fn test_weapon_finish_reload() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.current_ammo = 2;
    weapon.reserve_ammo = 50;
    weapon.is_reloading = true;

    weapon.finish_reload();

    assert_eq!(weapon.current_ammo, 12);
    assert_eq!(weapon.reserve_ammo, 40);
    assert!(!weapon.is_reloading);
}

#[test]
fn test_weapon_finish_reload_partial() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.current_ammo = 0;
    weapon.reserve_ammo = 5; // 少於彈匣容量
    weapon.is_reloading = true;

    weapon.finish_reload();

    assert_eq!(weapon.current_ammo, 5);
    assert_eq!(weapon.reserve_ammo, 0);
}

#[test]
fn test_weapon_needs_reload() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    assert!(!weapon.needs_reload());

    weapon.current_ammo = 0;
    assert!(weapon.needs_reload());

    weapon.reserve_ammo = 0;
    assert!(!weapon.needs_reload());
}

#[test]
fn test_weapon_cancel_reload() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.current_ammo = 0;
    weapon.start_reload();

    weapon.cancel_reload();
    assert!(!weapon.is_reloading);
    assert_eq!(weapon.reload_timer, 0.0);
}

// ============================================================================
// WeaponInventory 測試
// ============================================================================

#[test]
fn test_weapon_inventory_default() {
    let inventory = WeaponInventory::default();
    assert_eq!(inventory.weapons.len(), 1);
    assert_eq!(inventory.current_index, 0);
    assert_eq!(inventory.max_weapons, 6);
    assert_eq!(
        inventory.current_weapon().unwrap().stats.weapon_type,
        WeaponType::Fist
    );
}

#[test]
fn test_weapon_inventory_current_weapon() {
    let inventory = WeaponInventory::default();
    let weapon = inventory.current_weapon().unwrap();
    assert_eq!(weapon.stats.weapon_type, WeaponType::Fist);
}

#[test]
fn test_weapon_inventory_next_weapon() {
    let mut inventory = WeaponInventory::default();
    inventory.add_weapon(Weapon::new(WeaponStats::pistol()));
    inventory.add_weapon(Weapon::new(WeaponStats::smg()));

    assert_eq!(inventory.current_index, 0);

    inventory.next_weapon();
    assert_eq!(inventory.current_index, 1);

    inventory.next_weapon();
    assert_eq!(inventory.current_index, 2);

    inventory.next_weapon();
    assert_eq!(inventory.current_index, 0); // 循環
}

#[test]
fn test_weapon_inventory_prev_weapon() {
    let mut inventory = WeaponInventory::default();
    inventory.add_weapon(Weapon::new(WeaponStats::pistol()));
    inventory.add_weapon(Weapon::new(WeaponStats::smg()));

    assert_eq!(inventory.current_index, 0);

    inventory.prev_weapon();
    assert_eq!(inventory.current_index, 2); // 循環到最後

    inventory.prev_weapon();
    assert_eq!(inventory.current_index, 1);
}

#[test]
fn test_weapon_inventory_select_weapon() {
    let mut inventory = WeaponInventory::default();
    inventory.add_weapon(Weapon::new(WeaponStats::pistol()));
    inventory.add_weapon(Weapon::new(WeaponStats::smg()));

    inventory.select_weapon(2); // 1-based
    assert_eq!(inventory.current_index, 1);

    inventory.select_weapon(3);
    assert_eq!(inventory.current_index, 2);

    // 無效選擇不改變
    inventory.select_weapon(0);
    assert_eq!(inventory.current_index, 2);

    inventory.select_weapon(10);
    assert_eq!(inventory.current_index, 2);
}

#[test]
fn test_weapon_inventory_add_weapon() {
    let mut inventory = WeaponInventory::default();

    assert!(inventory.add_weapon(Weapon::new(WeaponStats::pistol())));
    assert_eq!(inventory.weapons.len(), 2);

    // 先消耗一些彈藥
    inventory.weapons[1].reserve_ammo = 50;
    let original_ammo = inventory.weapons[1].reserve_ammo;

    // 添加同類型武器會補充彈藥
    let mut pistol2 = Weapon::new(WeaponStats::pistol());
    pistol2.reserve_ammo = 30;

    inventory.add_weapon(pistol2);
    assert_eq!(inventory.weapons.len(), 2); // 數量不變
    assert!(inventory.weapons[1].reserve_ammo > original_ammo);
    assert_eq!(inventory.weapons[1].reserve_ammo, 80); // 50 + 30
}

#[test]
fn test_weapon_inventory_has_weapon() {
    let mut inventory = WeaponInventory::default();

    assert!(inventory.has_weapon(WeaponType::Fist));
    assert!(!inventory.has_weapon(WeaponType::Pistol));

    inventory.add_weapon(Weapon::new(WeaponStats::pistol()));
    assert!(inventory.has_weapon(WeaponType::Pistol));
}

#[test]
fn test_weapon_inventory_max_capacity() {
    let mut inventory = WeaponInventory::default();

    // 添加 5 把武器（加上預設拳頭共 6 把）
    inventory.add_weapon(Weapon::new(WeaponStats::pistol()));
    inventory.add_weapon(Weapon::new(WeaponStats::smg()));
    inventory.add_weapon(Weapon::new(WeaponStats::shotgun()));
    inventory.add_weapon(Weapon::new(WeaponStats::rifle()));
    inventory.add_weapon(Weapon::new(WeaponStats::staff()));

    assert_eq!(inventory.weapons.len(), 6);

    // 達到上限，無法添加新武器
    assert!(!inventory.add_weapon(Weapon::new(WeaponStats::knife())));
    assert_eq!(inventory.weapons.len(), 6);
}

// ============================================================================
// Weapon 方法封裝測試（#16 新增方法）
// ============================================================================

#[test]
fn test_weapon_tick_cooldown() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.fire_cooldown = 1.0;
    weapon.tick_cooldown(0.3);
    assert!((weapon.fire_cooldown - 0.7).abs() < 0.001);

    weapon.tick_cooldown(1.0);
    assert_eq!(weapon.fire_cooldown, 0.0);
}

#[test]
fn test_weapon_tick_cooldown_already_zero() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.tick_cooldown(0.5);
    assert_eq!(weapon.fire_cooldown, 0.0);
}

#[test]
fn test_weapon_tick_reload() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.consume_ammo();
    weapon.start_reload();
    assert!(weapon.is_reloading);

    let still_reloading = weapon.tick_reload(0.5);
    assert!(still_reloading);
    assert!(weapon.is_reloading);

    let still_reloading = weapon.tick_reload(5.0);
    assert!(still_reloading);
    assert!(!weapon.is_reloading);
    assert_eq!(weapon.current_ammo, 12);
}

#[test]
fn test_weapon_tick_reload_not_reloading() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    assert!(!weapon.tick_reload(0.5));
}

#[test]
fn test_weapon_is_cooling_down() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    assert!(!weapon.is_cooling_down());

    weapon.fire_cooldown = 0.5;
    assert!(weapon.is_cooling_down());
}

#[test]
fn test_weapon_set_fire_cooldown() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.set_fire_cooldown(0.75);
    assert_eq!(weapon.fire_cooldown, 0.75);
}

#[test]
fn test_weapon_reset_fire_cooldown() {
    let mut weapon = Weapon::new(WeaponStats::pistol());
    weapon.reset_fire_cooldown();
    assert_eq!(weapon.fire_cooldown, weapon.stats.fire_rate);
}

#[test]
fn test_weapon_effective_range() {
    let weapon = Weapon::new(WeaponStats::rifle());
    assert_eq!(weapon.effective_range(), weapon.stats.range);
}

#[test]
fn test_weapon_base_damage() {
    let weapon = Weapon::new(WeaponStats::shotgun());
    assert_eq!(weapon.base_damage(), weapon.stats.damage);
}
