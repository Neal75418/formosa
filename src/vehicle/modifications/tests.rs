use super::*;

// --- ModLevel ---

#[test]
fn mod_level_multiplier_progression() {
    assert_eq!(ModLevel::Stock.multiplier(), 1.0);
    assert!((ModLevel::Level1.multiplier() - 1.10).abs() < f32::EPSILON);
    assert!((ModLevel::Level2.multiplier() - 1.25).abs() < f32::EPSILON);
    assert!((ModLevel::Level3.multiplier() - 1.50).abs() < f32::EPSILON);
}

#[test]
fn mod_level_price_progression() {
    assert_eq!(ModLevel::Stock.price(), 0);
    assert_eq!(ModLevel::Level1.price(), 5_000);
    assert_eq!(ModLevel::Level2.price(), 15_000);
    assert_eq!(ModLevel::Level3.price(), 40_000);
}

#[test]
fn mod_level_next_chain() {
    assert_eq!(ModLevel::Stock.next(), Some(ModLevel::Level1));
    assert_eq!(ModLevel::Level1.next(), Some(ModLevel::Level2));
    assert_eq!(ModLevel::Level2.next(), Some(ModLevel::Level3));
    assert_eq!(ModLevel::Level3.next(), None);
}

#[test]
fn mod_level_upgrade_price() {
    assert_eq!(ModLevel::Stock.upgrade_price(), Some(5_000));
    assert_eq!(ModLevel::Level3.upgrade_price(), None);
}

// --- VehicleModifications ---

#[test]
fn mods_upgrade_advances_level() {
    let mut mods = VehicleModifications::default();
    assert_eq!(mods.get_level(ModCategory::Engine), ModLevel::Stock);
    assert!(mods.upgrade(ModCategory::Engine));
    assert_eq!(mods.get_level(ModCategory::Engine), ModLevel::Level1);
    assert!(mods.upgrade(ModCategory::Engine));
    assert_eq!(mods.get_level(ModCategory::Engine), ModLevel::Level2);
    assert!(mods.upgrade(ModCategory::Engine));
    assert_eq!(mods.get_level(ModCategory::Engine), ModLevel::Level3);
    assert!(!mods.upgrade(ModCategory::Engine)); // 已滿
}

#[test]
fn mods_get_multiplier_reflects_level() {
    let mut mods = VehicleModifications::default();
    assert_eq!(mods.get_multiplier(ModCategory::Tires), 1.0);
    mods.upgrade(ModCategory::Tires);
    assert!((mods.get_multiplier(ModCategory::Tires) - 1.10).abs() < f32::EPSILON);
}

#[test]
fn mods_total_value_sums_all() {
    let mut mods = VehicleModifications::default();
    assert_eq!(mods.total_value(), 0);
    mods.upgrade(ModCategory::Engine); // +5000
    mods.upgrade(ModCategory::Brakes); // +5000
    assert_eq!(mods.total_value(), 10_000);
    mods.has_nitro = true; // +25000
    assert_eq!(mods.total_value(), 35_000);
}

// --- modified_* helpers ---

#[test]
fn modified_acceleration_applies_engine_multiplier() {
    let mods = VehicleModifications { engine: ModLevel::Level2, ..VehicleModifications::default() };
    let result = modified_acceleration(10.0, &mods);
    assert!((result - 12.5).abs() < f32::EPSILON);
}

#[test]
fn modified_max_speed_applies_transmission() {
    let mods = VehicleModifications { transmission: ModLevel::Level3, ..VehicleModifications::default() };
    let result = modified_max_speed(30.0, &mods);
    assert!((result - 45.0).abs() < f32::EPSILON);
}

#[test]
fn modified_health_applies_armor() {
    let mods = VehicleModifications { armor: ModLevel::Level1, ..VehicleModifications::default() };
    let result = modified_health(1000.0, &mods);
    assert!((result - 1100.0).abs() < f32::EPSILON);
}

// --- Visual Modifications ---

#[test]
fn paint_color_prices() {
    assert_eq!(PaintColor::Stock.price(), 0);
    assert_eq!(PaintColor::CrimsonRed.price(), 3_000);
    assert_eq!(PaintColor::MidnightBlue.price(), 3_000);
}

#[test]
fn paint_color_all_has_seven() {
    assert_eq!(PaintColor::all().len(), 7);
    assert!(!PaintColor::all().contains(&PaintColor::Stock));
}

#[test]
fn window_tint_price_progression() {
    assert_eq!(WindowTint::None.price(), 0);
    assert!(WindowTint::Light.price() < WindowTint::Medium.price());
    assert!(WindowTint::Medium.price() < WindowTint::Dark.price());
    assert!(WindowTint::Dark.price() < WindowTint::Mirror.price());
}

#[test]
fn spoiler_price_progression() {
    assert_eq!(SpoilerType::None.price(), 0);
    assert!(SpoilerType::LipSpoiler.price() < SpoilerType::MediumWing.price());
    assert!(SpoilerType::MediumWing.price() < SpoilerType::GtWing.price());
}

#[test]
fn rim_price_progression() {
    assert_eq!(RimType::Stock.price(), 0);
    assert!(RimType::Alloy.price() < RimType::MultiSpoke.price());
    assert!(RimType::MultiSpoke.price() < RimType::DeepDish.price());
    assert!(RimType::DeepDish.price() < RimType::CarbonFiber.price());
}

#[test]
fn visual_mods_default_is_stock() {
    let vm = VehicleVisualMods::default();
    assert_eq!(vm.paint, PaintColor::Stock);
    assert_eq!(vm.tint, WindowTint::None);
    assert_eq!(vm.spoiler, SpoilerType::None);
    assert_eq!(vm.rims, RimType::Stock);
    assert_eq!(vm.total_value(), 0);
}

#[test]
fn visual_mods_total_value() {
    let vm = VehicleVisualMods {
        paint: PaintColor::CrimsonRed,   // 3000
        tint: WindowTint::Dark,          // 3500
        spoiler: SpoilerType::GtWing,    // 10000
        rims: RimType::CarbonFiber,      // 15000
    };
    assert_eq!(vm.total_value(), 31_500);
}

#[test]
fn visual_mod_purchase_price() {
    let p = VisualModPurchase::Paint(PaintColor::SunsetOrange);
    assert_eq!(p.price(), 3_000);
    assert!(p.name().contains("日落橙"));
}
