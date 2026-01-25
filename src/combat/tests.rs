//! 戰鬥系統單元測試

use super::*;
use bevy::prelude::*;

// ============================================================================
// WeaponType 測試
// ============================================================================

#[test]
fn test_weapon_type_default() {
    let weapon = WeaponType::default();
    assert_eq!(weapon, WeaponType::Fist);
}

#[test]
fn test_weapon_type_name() {
    assert_eq!(WeaponType::Fist.name(), "拳頭");
    assert_eq!(WeaponType::Staff.name(), "棍棒");
    assert_eq!(WeaponType::Knife.name(), "刀");
    assert_eq!(WeaponType::Pistol.name(), "手槍");
    assert_eq!(WeaponType::SMG.name(), "衝鋒槍");
    assert_eq!(WeaponType::Shotgun.name(), "霰彈槍");
    assert_eq!(WeaponType::Rifle.name(), "步槍");
}

#[test]
fn test_weapon_type_icon() {
    assert_eq!(WeaponType::Fist.icon(), "👊");
    assert_eq!(WeaponType::Staff.icon(), "🏏");
    assert_eq!(WeaponType::Knife.icon(), "🔪");
    assert_eq!(WeaponType::Pistol.icon(), "🔫");
    assert_eq!(WeaponType::SMG.icon(), "🔫");
    assert_eq!(WeaponType::Shotgun.icon(), "🎯");
    assert_eq!(WeaponType::Rifle.icon(), "🎯");
}

#[test]
fn test_weapon_type_is_melee() {
    assert!(WeaponType::Fist.is_melee());
    assert!(WeaponType::Staff.is_melee());
    assert!(WeaponType::Knife.is_melee());
    assert!(!WeaponType::Pistol.is_melee());
    assert!(!WeaponType::SMG.is_melee());
    assert!(!WeaponType::Shotgun.is_melee());
    assert!(!WeaponType::Rifle.is_melee());
}

#[test]
fn test_weapon_type_tracer_style() {
    assert_eq!(WeaponType::Fist.tracer_style(), TracerStyle::None);
    assert_eq!(WeaponType::Staff.tracer_style(), TracerStyle::None);
    assert_eq!(WeaponType::Knife.tracer_style(), TracerStyle::None);
    assert_eq!(WeaponType::Pistol.tracer_style(), TracerStyle::Pistol);
    assert_eq!(WeaponType::SMG.tracer_style(), TracerStyle::SMG);
    assert_eq!(WeaponType::Shotgun.tracer_style(), TracerStyle::Shotgun);
    assert_eq!(WeaponType::Rifle.tracer_style(), TracerStyle::Rifle);
}

// ============================================================================
// WeaponStats 測試
// ============================================================================

#[test]
fn test_weapon_stats_pistol() {
    let stats = WeaponStats::pistol();
    assert_eq!(stats.weapon_type, WeaponType::Pistol);
    assert_eq!(stats.damage, 25.0);
    assert_eq!(stats.fire_rate, 0.3);
    assert_eq!(stats.magazine_size, 12);
    assert_eq!(stats.max_ammo, 120);
    assert_eq!(stats.range, 50.0);
    assert_eq!(stats.reload_time, 1.5);
    assert!(!stats.is_automatic);
    assert_eq!(stats.pellet_count, 1);
}

#[test]
fn test_weapon_stats_smg() {
    let stats = WeaponStats::smg();
    assert_eq!(stats.weapon_type, WeaponType::SMG);
    assert_eq!(stats.damage, 15.0);
    assert_eq!(stats.fire_rate, 0.08);
    assert_eq!(stats.magazine_size, 30);
    assert!(stats.is_automatic);
}

#[test]
fn test_weapon_stats_shotgun() {
    let stats = WeaponStats::shotgun();
    assert_eq!(stats.weapon_type, WeaponType::Shotgun);
    assert_eq!(stats.damage, 15.0);
    assert_eq!(stats.pellet_count, 8);
    assert!(!stats.is_automatic);
    // 總傷害 = 15 * 8 = 120
}

#[test]
fn test_weapon_stats_rifle() {
    let stats = WeaponStats::rifle();
    assert_eq!(stats.weapon_type, WeaponType::Rifle);
    assert_eq!(stats.damage, 35.0);
    assert_eq!(stats.range, 100.0);
    assert!(stats.is_automatic);
}

#[test]
fn test_weapon_stats_fist() {
    let stats = WeaponStats::fist();
    assert_eq!(stats.weapon_type, WeaponType::Fist);
    assert_eq!(stats.damage, 20.0);
    assert_eq!(stats.magazine_size, 0); // 無限
    assert_eq!(stats.range, 2.5);
}

#[test]
fn test_weapon_stats_staff() {
    let stats = WeaponStats::staff();
    assert_eq!(stats.weapon_type, WeaponType::Staff);
    assert_eq!(stats.damage, 35.0);
    assert_eq!(stats.range, 3.2);
    assert_eq!(stats.spread, 60.0); // 掃擊角度
}

#[test]
fn test_weapon_stats_knife() {
    let stats = WeaponStats::knife();
    assert_eq!(stats.weapon_type, WeaponType::Knife);
    assert_eq!(stats.damage, 28.0);
    assert_eq!(stats.fire_rate, 0.25); // 快速揮刀
    assert_eq!(stats.range, 2.0);
}

// ============================================================================
// 傷害衰減測試
// ============================================================================

#[test]
fn test_damage_falloff_pistol() {
    let stats = WeaponStats::pistol();

    // 在有效射程內
    assert_eq!(stats.calculate_damage_falloff(0.0), 1.0);
    assert_eq!(stats.calculate_damage_falloff(30.0), 1.0);

    // 超過最遠距離
    assert_eq!(stats.calculate_damage_falloff(50.0), 0.25);
    assert_eq!(stats.calculate_damage_falloff(100.0), 0.25);

    // 衰減區間
    let mid_distance = 40.0;
    let falloff = stats.calculate_damage_falloff(mid_distance);
    assert!(falloff > 0.25 && falloff < 1.0);
}

#[test]
fn test_damage_falloff_melee() {
    let stats = WeaponStats::fist();
    // 近戰武器不衰減
    assert_eq!(stats.calculate_damage_falloff(0.0), 1.0);
    assert_eq!(stats.calculate_damage_falloff(2.5), 1.0);
    assert_eq!(stats.calculate_damage_falloff(100.0), 1.0);
}

#[test]
fn test_damage_falloff_linear() {
    let stats = WeaponStats::pistol();
    // falloff_start = 30, falloff_end = 50

    // 中點應該是 62.5% 傷害
    let mid_falloff = stats.calculate_damage_falloff(40.0);
    let expected = 1.0 - 0.75 * 0.5; // 0.625
    assert!((mid_falloff - expected).abs() < 0.001);
}

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
// Health 測試
// ============================================================================

#[test]
fn test_health_default() {
    let health = Health::default();
    assert_eq!(health.current, 100.0);
    assert_eq!(health.max, 100.0);
    assert_eq!(health.regeneration, 0.0);
    assert!(!health.is_dead());
    assert!(health.is_full());
}

#[test]
fn test_health_new() {
    let health = Health::new(200.0);
    assert_eq!(health.current, 200.0);
    assert_eq!(health.max, 200.0);
}

#[test]
fn test_health_with_regen() {
    let health = Health::new(100.0).with_regen(5.0, 3.0);
    assert_eq!(health.regeneration, 5.0);
    assert_eq!(health.regen_delay, 3.0);
}

#[test]
fn test_health_percentage() {
    let mut health = Health::new(100.0);
    assert_eq!(health.percentage(), 1.0);

    health.current = 50.0;
    assert_eq!(health.percentage(), 0.5);

    health.current = 0.0;
    assert_eq!(health.percentage(), 0.0);
}

#[test]
fn test_health_take_damage() {
    let mut health = Health::new(100.0);
    let actual = health.take_damage(30.0, 1.0);

    assert_eq!(actual, 30.0);
    assert_eq!(health.current, 70.0);
    assert_eq!(health.last_damage_time, 1.0);
}

#[test]
fn test_health_take_damage_overkill() {
    let mut health = Health::new(100.0);
    health.current = 20.0;

    let actual = health.take_damage(50.0, 1.0);
    assert_eq!(actual, 20.0);
    assert_eq!(health.current, 0.0);
    assert!(health.is_dead());
}

#[test]
fn test_health_heal() {
    let mut health = Health::new(100.0);
    health.current = 50.0;

    let actual = health.heal(30.0);
    assert_eq!(actual, 30.0);
    assert_eq!(health.current, 80.0);
}

#[test]
fn test_health_heal_overheal() {
    let mut health = Health::new(100.0);
    health.current = 90.0;

    let actual = health.heal(50.0);
    assert_eq!(actual, 10.0);
    assert_eq!(health.current, 100.0);
}

// ============================================================================
// Armor 測試
// ============================================================================

#[test]
fn test_armor_default() {
    let armor = Armor::default();
    assert_eq!(armor.current, 0.0);
    assert_eq!(armor.max, 100.0);
    assert_eq!(armor.damage_reduction, 0.5);
}

#[test]
fn test_armor_new() {
    let armor = Armor::new(50.0);
    assert_eq!(armor.current, 50.0);
}

#[test]
fn test_armor_percentage() {
    let mut armor = Armor::new(100.0);
    assert_eq!(armor.percentage(), 1.0);

    armor.current = 50.0;
    assert_eq!(armor.percentage(), 0.5);
}

#[test]
fn test_armor_absorb_damage() {
    let mut armor = Armor::new(50.0);

    // 護甲吸收所有傷害
    let pass_through = armor.absorb_damage(30.0);
    assert_eq!(pass_through, 0.0);
    assert_eq!(armor.current, 20.0);
}

#[test]
fn test_armor_absorb_damage_overflow() {
    let mut armor = Armor::new(30.0);

    // 護甲部分吸收，剩餘傷害減免 50%
    let pass_through = armor.absorb_damage(50.0);
    // 30 護甲吸收，剩餘 20 傷害 * 0.5 = 10
    assert_eq!(pass_through, 10.0);
    assert_eq!(armor.current, 0.0);
}

#[test]
fn test_armor_absorb_damage_no_armor() {
    let mut armor = Armor::new(0.0);
    let pass_through = armor.absorb_damage(50.0);
    assert_eq!(pass_through, 50.0);
}

#[test]
fn test_armor_is_broken() {
    let mut armor = Armor::new(10.0);
    assert!(!armor.is_broken());

    armor.current = 0.0;
    assert!(armor.is_broken());
}

#[test]
fn test_armor_took_significant_hit() {
    let armor = Armor::new(50.0);

    assert!(!armor.took_significant_hit(10.0));
    assert!(armor.took_significant_hit(15.0));
    assert!(armor.took_significant_hit(20.0));
}

// ============================================================================
// BleedEffect 測試
// ============================================================================

#[test]
fn test_bleed_effect_default() {
    let bleed = BleedEffect::default();
    assert_eq!(bleed.damage_per_second, BLEED_DAMAGE_PER_SECOND);
    assert_eq!(bleed.remaining_time, BLEED_DURATION);
    assert!(bleed.source.is_none());
    assert!(!bleed.is_finished());
}

#[test]
fn test_bleed_effect_new() {
    let source = Entity::from_bits(42);
    let bleed = BleedEffect::new(source);
    assert_eq!(bleed.source, Some(source));
}

#[test]
fn test_bleed_effect_is_finished() {
    let mut bleed = BleedEffect::default();
    assert!(!bleed.is_finished());

    bleed.remaining_time = 0.0;
    assert!(bleed.is_finished());

    bleed.remaining_time = -1.0;
    assert!(bleed.is_finished());
}

// ============================================================================
// MeleeAnimationType 測試
// ============================================================================

#[test]
fn test_melee_animation_type_default() {
    let anim = MeleeAnimationType::default();
    assert_eq!(anim, MeleeAnimationType::Punch);
}

#[test]
fn test_melee_animation_type_from_weapon() {
    assert_eq!(
        MeleeAnimationType::from_weapon(WeaponType::Fist),
        MeleeAnimationType::Punch
    );
    assert_eq!(
        MeleeAnimationType::from_weapon(WeaponType::Staff),
        MeleeAnimationType::Swing
    );
    assert_eq!(
        MeleeAnimationType::from_weapon(WeaponType::Knife),
        MeleeAnimationType::Slash
    );
    // 非近戰武器預設為 Punch
    assert_eq!(
        MeleeAnimationType::from_weapon(WeaponType::Pistol),
        MeleeAnimationType::Punch
    );
}

// ============================================================================
// Ragdoll 測試
// ============================================================================

#[test]
fn test_ragdoll_default() {
    let ragdoll = Ragdoll::default();
    assert_eq!(ragdoll.lifetime, 0.0);
    assert_eq!(ragdoll.max_lifetime, 5.0);
    assert!(!ragdoll.physics_applied);
    assert_eq!(ragdoll.impulse_strength, 300.0);
}

#[test]
fn test_ragdoll_with_impulse() {
    let direction = Vec3::new(1.0, 0.0, 1.0);
    let ragdoll = Ragdoll::with_impulse(direction, 500.0);

    assert_eq!(ragdoll.impulse_strength, 500.0);
    // 方向應該被正規化
    assert!((ragdoll.impulse_direction.length() - 1.0).abs() < 0.001);
}

#[test]
fn test_ragdoll_with_impulse_zero_direction() {
    let ragdoll = Ragdoll::with_impulse(Vec3::ZERO, 500.0);
    assert_eq!(ragdoll.impulse_direction, Vec3::ZERO);
}

// ============================================================================
// BloodParticle 測試
// ============================================================================

#[test]
fn test_blood_particle_new() {
    let velocity = Vec3::new(1.0, 2.0, 3.0);
    let particle = BloodParticle::new(velocity, 1.5);

    assert_eq!(particle.velocity, velocity);
    assert_eq!(particle.lifetime, 0.0);
    assert_eq!(particle.max_lifetime, 1.5);
}

// ============================================================================
// EnemyType 測試
// ============================================================================

#[test]
fn test_enemy_type_health() {
    assert_eq!(EnemyType::Gangster.health(), 50.0);
    assert_eq!(EnemyType::Thug.health(), 80.0);
    assert_eq!(EnemyType::Boss.health(), 200.0);
}

#[test]
fn test_enemy_type_weapon() {
    assert_eq!(EnemyType::Gangster.weapon().weapon_type, WeaponType::Pistol);
    assert_eq!(EnemyType::Thug.weapon().weapon_type, WeaponType::SMG);
    assert_eq!(EnemyType::Boss.weapon().weapon_type, WeaponType::Shotgun);
}

// ============================================================================
// DamageEvent 測試
// ============================================================================

#[test]
fn test_damage_event_new() {
    let target = Entity::from_bits(1);
    let event = DamageEvent::new(target, 25.0, DamageSource::Bullet);

    assert_eq!(event.target, target);
    assert_eq!(event.amount, 25.0);
    assert_eq!(event.source, DamageSource::Bullet);
    assert!(event.attacker.is_none());
    assert!(event.hit_position.is_none());
    assert!(!event.is_headshot);
}

#[test]
fn test_damage_event_builder() {
    let target = Entity::from_bits(1);
    let attacker = Entity::from_bits(2);
    let position = Vec3::new(10.0, 1.8, 5.0);

    let event = DamageEvent::new(target, 50.0, DamageSource::Melee)
        .with_attacker(attacker)
        .with_position(position)
        .with_headshot(true);

    assert_eq!(event.attacker, Some(attacker));
    assert_eq!(event.hit_position, Some(position));
    assert!(event.is_headshot);
}

// ============================================================================
// check_headshot 測試
// ============================================================================

#[test]
fn test_check_headshot() {
    let base_y = 0.0;

    // 身體射擊
    assert!(!check_headshot(Vec3::new(0.0, 1.0, 0.0), base_y));

    // 頭部射擊 (1.5m - 2.0m)
    assert!(check_headshot(Vec3::new(0.0, 1.5, 0.0), base_y));
    assert!(check_headshot(Vec3::new(0.0, 1.75, 0.0), base_y));
    assert!(check_headshot(Vec3::new(0.0, 2.0, 0.0), base_y));

    // 超過頭部
    assert!(!check_headshot(Vec3::new(0.0, 2.1, 0.0), base_y));
}

#[test]
fn test_check_headshot_elevated() {
    let base_y = 5.0; // 站在平台上

    assert!(!check_headshot(Vec3::new(0.0, 6.0, 0.0), base_y));
    assert!(check_headshot(Vec3::new(0.0, 6.5, 0.0), base_y));
    assert!(check_headshot(Vec3::new(0.0, 7.0, 0.0), base_y));
    assert!(!check_headshot(Vec3::new(0.0, 7.1, 0.0), base_y));
}

// ============================================================================
// HitReaction 測試
// ============================================================================

#[test]
fn test_hit_reaction_default() {
    let reaction = HitReaction::default();
    assert_eq!(reaction.phase, HitReactionPhase::None);
    assert!(!reaction.is_reacting());
    assert_eq!(reaction.get_knockback_velocity(), Vec3::ZERO);
}

#[test]
fn test_hit_reaction_trigger_flinch() {
    let mut reaction = HitReaction::default();
    let direction = Vec3::new(0.0, 0.0, -1.0);

    reaction.trigger(15.0, direction, false);
    assert_eq!(reaction.phase, HitReactionPhase::Flinch);
    assert!(reaction.is_reacting());
    assert!(reaction.is_immune);
}

#[test]
fn test_hit_reaction_trigger_stagger() {
    let mut reaction = HitReaction::default();
    let direction = Vec3::new(0.0, 0.0, -1.0);

    reaction.trigger(30.0, direction, false);
    assert_eq!(reaction.phase, HitReactionPhase::Stagger);
}

#[test]
fn test_hit_reaction_trigger_knockback() {
    let mut reaction = HitReaction::default();
    let direction = Vec3::new(0.0, 0.0, -1.0);

    reaction.trigger(50.0, direction, false);
    assert_eq!(reaction.phase, HitReactionPhase::Knockback);
}

#[test]
fn test_hit_reaction_trigger_headshot() {
    let mut reaction = HitReaction::default();
    let direction = Vec3::new(0.0, 0.0, -1.0);

    // 爆頭即使傷害低也會觸發 Knockback
    reaction.trigger(15.0, direction, true);
    assert_eq!(reaction.phase, HitReactionPhase::Knockback);
}

#[test]
fn test_hit_reaction_immunity() {
    let mut reaction = HitReaction::default();
    let direction = Vec3::new(0.0, 0.0, -1.0);

    reaction.trigger(30.0, direction, false);
    assert!(reaction.is_immune);

    // 免疫期間不會觸發新反應
    let old_phase = reaction.phase;
    reaction.trigger(50.0, direction, false);
    assert_eq!(reaction.phase, old_phase);
}

#[test]
fn test_hit_reaction_update() {
    let mut reaction = HitReaction::default();
    reaction.trigger(30.0, Vec3::NEG_Z, false);

    // 更新直到完成
    for _ in 0..100 {
        if !reaction.update(0.05) {
            break;
        }
    }

    assert_eq!(reaction.phase, HitReactionPhase::None);
    assert!(!reaction.is_reacting());
}

#[test]
fn test_hit_reaction_thresholds() {
    assert_eq!(HitReaction::FLINCH_THRESHOLD, 10.0);
    assert_eq!(HitReaction::STAGGER_THRESHOLD, 25.0);
    assert_eq!(HitReaction::KNOCKBACK_THRESHOLD, 40.0);
}

// ============================================================================
// PunchAnimation 測試
// ============================================================================

#[test]
fn test_punch_animation_default() {
    let anim = PunchAnimation::default();
    assert_eq!(anim.timer, 0.0);
    assert_eq!(anim.duration, 0.3);
    assert_eq!(anim.phase, PunchPhase::WindUp);
    assert!(!anim.is_finished());
}

#[test]
fn test_punch_animation_progress() {
    let mut anim = PunchAnimation::default();
    assert_eq!(anim.progress(), 0.0);

    anim.timer = 0.15;
    assert_eq!(anim.progress(), 0.5);

    anim.timer = 0.3;
    assert_eq!(anim.progress(), 1.0);
}

#[test]
fn test_punch_animation_is_finished() {
    let mut anim = PunchAnimation::default();
    assert!(!anim.is_finished());

    anim.timer = 0.3;
    assert!(anim.is_finished());
}

#[test]
fn test_punch_animation_phase_times() {
    let anim = PunchAnimation::default();
    let (wind_up, strike, total) = anim.phase_times();

    assert!((wind_up - 0.099).abs() < 0.001); // 33%
    assert!((strike - 0.198).abs() < 0.001); // 66%
    assert_eq!(total, 0.3);
}

// ============================================================================
// RagdollTracker 測試
// ============================================================================

#[test]
fn test_ragdoll_tracker_default() {
    let tracker = RagdollTracker::default();
    assert!(tracker.ragdolls.is_empty());
    assert_eq!(tracker.max_count, 10);
}

// ============================================================================
// 常數測試
// ============================================================================

#[test]
fn test_headshot_multiplier() {
    assert_eq!(HEADSHOT_MULTIPLIER, 2.5);
}

#[test]
fn test_bleed_constants() {
    assert_eq!(BLEED_DAMAGE_PER_SECOND, 5.0);
    assert_eq!(BLEED_DURATION, 4.0);
    assert_eq!(BLEED_CHANCE, 0.35);
}
