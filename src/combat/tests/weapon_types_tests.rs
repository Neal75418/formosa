//! WeaponType、WeaponStats、傷害衰減測試

use crate::combat::*;

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
