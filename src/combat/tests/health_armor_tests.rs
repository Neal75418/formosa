//! Health、Armor、BleedEffect 測試

use crate::combat::*;
use bevy::prelude::*;

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
