use super::*;

// --- VehicleDamageState ---

#[test]
fn damage_state_from_health_percent_boundaries() {
    assert_eq!(
        VehicleDamageState::from_health_percent(1.0),
        VehicleDamageState::Pristine
    );
    assert_eq!(
        VehicleDamageState::from_health_percent(1.5),
        VehicleDamageState::Pristine
    );
    assert_eq!(
        VehicleDamageState::from_health_percent(0.75),
        VehicleDamageState::Light
    );
    assert_eq!(
        VehicleDamageState::from_health_percent(0.50),
        VehicleDamageState::Moderate
    );
    assert_eq!(
        VehicleDamageState::from_health_percent(0.25),
        VehicleDamageState::Heavy
    );
    assert_eq!(
        VehicleDamageState::from_health_percent(0.10),
        VehicleDamageState::Critical
    );
    assert_eq!(
        VehicleDamageState::from_health_percent(0.0),
        VehicleDamageState::Destroyed
    );
}

// --- VehicleHealth ---

#[test]
fn health_new_sets_max_and_current() {
    let h = VehicleHealth::new(500.0);
    assert_eq!(h.current, 500.0);
    assert_eq!(h.max, 500.0);
    assert_eq!(h.percentage(), 1.0);
    assert!(!h.is_destroyed());
}

#[test]
fn health_for_vehicle_type_correct_values() {
    assert_eq!(
        VehicleHealth::for_vehicle_type(VehicleType::Scooter).max,
        500.0
    );
    assert_eq!(
        VehicleHealth::for_vehicle_type(VehicleType::Car).max,
        1000.0
    );
    assert_eq!(
        VehicleHealth::for_vehicle_type(VehicleType::Taxi).max,
        1200.0
    );
    assert_eq!(
        VehicleHealth::for_vehicle_type(VehicleType::Bus).max,
        2000.0
    );
}

#[test]
fn health_take_damage_reduces_hp() {
    let mut h = VehicleHealth::new(100.0);
    let actual = h.take_damage(30.0, 1.0);
    assert!((actual - 30.0).abs() < f32::EPSILON);
    assert!((h.current - 70.0).abs() < f32::EPSILON);
    assert!((h.percentage() - 0.7).abs() < 0.01);
}

#[test]
fn health_take_damage_clamps_to_zero() {
    let mut h = VehicleHealth::new(50.0);
    let actual = h.take_damage(200.0, 1.0);
    assert!((actual - 50.0).abs() < f32::EPSILON);
    assert_eq!(h.current, 0.0);
    assert_eq!(h.damage_state, VehicleDamageState::Destroyed);
}

#[test]
fn health_take_damage_invulnerable_returns_zero() {
    let mut h = VehicleHealth::new(100.0);
    h.is_invulnerable = true;
    let actual = h.take_damage(50.0, 1.0);
    assert_eq!(actual, 0.0);
    assert_eq!(h.current, 100.0);
}

#[test]
fn health_take_damage_critical_triggers_fire() {
    let mut h = VehicleHealth::new(100.0);
    h.take_damage(80.0, 1.0);
    assert!(h.is_on_fire);
    assert_eq!(h.damage_state, VehicleDamageState::Critical);
}

#[test]
fn health_repair_increases_hp() {
    let mut h = VehicleHealth::new(100.0);
    h.take_damage(60.0, 1.0);
    h.repair(30.0);
    assert!((h.current - 70.0).abs() < f32::EPSILON);
}

#[test]
fn health_repair_clamps_to_max() {
    let mut h = VehicleHealth::new(100.0);
    h.take_damage(20.0, 1.0);
    h.repair(500.0);
    assert_eq!(h.current, 100.0);
}

#[test]
fn health_repair_extinguishes_fire_above_threshold() {
    let mut h = VehicleHealth::new(100.0);
    h.take_damage(85.0, 1.0);
    assert!(h.is_on_fire);
    h.repair(60.0);
    assert!(!h.is_on_fire);
    assert!(h.percentage() > 0.3);
}

#[test]
fn health_repair_does_nothing_when_destroyed() {
    let mut h = VehicleHealth::new(100.0);
    h.take_damage(100.0, 1.0);
    assert!(h.is_destroyed());
    h.repair(50.0);
    assert_eq!(h.current, 0.0);
}

#[test]
fn health_full_repair_restores_everything() {
    let mut h = VehicleHealth::new(100.0);
    h.take_damage(80.0, 1.0);
    h.full_repair();
    assert_eq!(h.current, 100.0);
    assert_eq!(h.damage_state, VehicleDamageState::Pristine);
    assert!(!h.is_on_fire);
}

#[test]
fn health_apply_armor_upgrade_preserves_ratio() {
    let mut h = VehicleHealth::new(100.0);
    h.take_damage(50.0, 1.0);
    assert!((h.percentage() - 0.5).abs() < 0.01);
    h.apply_armor_upgrade(1.5);
    assert!((h.max - 150.0).abs() < f32::EPSILON);
    assert!((h.percentage() - 0.5).abs() < 0.01);
}

#[test]
fn health_tick_fire_countdown_explodes() {
    let mut h = VehicleHealth::new(1000.0);
    h.is_on_fire = true;
    h.fire_timer = 1.0;
    assert!(h.tick_fire(1.5));
    assert_eq!(h.damage_state, VehicleDamageState::Destroyed);
}

#[test]
fn health_tick_fire_burn_damage() {
    let mut h = VehicleHealth::new(100.0);
    h.is_on_fire = true;
    h.fire_timer = 10.0;
    h.tick_fire(1.0);
    assert!((h.current - 80.0).abs() < f32::EPSILON);
}

#[test]
fn health_tick_fire_not_on_fire_returns_false() {
    let mut h = VehicleHealth::new(100.0);
    assert!(!h.tick_fire(1.0));
    assert_eq!(h.current, 100.0);
}

// --- TireDamage ---

#[test]
fn tire_pop_and_count() {
    let mut td = TireDamage::default();
    assert_eq!(td.flat_count(), 0);
    td.pop_tire(0);
    assert_eq!(td.flat_count(), 1);
    assert!(td.has_front_flat());
    assert!(!td.has_rear_flat());
}

#[test]
fn tire_pop_rear_detected() {
    let mut td = TireDamage::default();
    td.pop_tire(2);
    assert!(!td.has_front_flat());
    assert!(td.has_rear_flat());
}

#[test]
fn tire_repair_single() {
    let mut td = TireDamage::default();
    td.pop_tire(1);
    td.pop_tire(3);
    assert_eq!(td.flat_count(), 2);
    td.repair_tire(1);
    assert_eq!(td.flat_count(), 1);
}

#[test]
fn tire_repair_all_resets() {
    let mut td = TireDamage::default();
    td.pop_tire(0);
    td.pop_tire(1);
    td.pop_tire(2);
    td.repair_all();
    assert_eq!(td.flat_count(), 0);
    assert_eq!(td.handling_penalty, 0.0);
    assert_eq!(td.speed_penalty, 0.0);
}

#[test]
fn tire_penalties_scale_with_count() {
    let mut td = TireDamage::default();
    td.pop_tire(0);
    assert!((td.handling_penalty - 0.15).abs() < f32::EPSILON);
    assert!((td.speed_penalty - 0.10).abs() < f32::EPSILON);
    td.pop_tire(1);
    assert!((td.handling_penalty - 0.30).abs() < f32::EPSILON);
    assert!((td.speed_penalty - 0.20).abs() < f32::EPSILON);
}

#[test]
fn tire_out_of_bounds_ignored() {
    let mut td = TireDamage::default();
    td.pop_tire(99);
    assert_eq!(td.flat_count(), 0);
}

// --- DoorWindowState ---

#[test]
fn door_default_all_closed() {
    let dw = DoorWindowState::default();
    for door in &dw.doors {
        assert_eq!(*door, DoorState::Closed);
    }
    for window in &dw.windows {
        assert_eq!(*window, WindowState::Intact);
    }
    assert_eq!(dw.drag_penalty, 0.0);
}

#[test]
fn door_toggle_opens_and_closes() {
    let mut dw = DoorWindowState::default();

    // 開門
    dw.toggle_door(DOOR_FRONT_LEFT);
    assert!(matches!(dw.doors[DOOR_FRONT_LEFT], DoorState::Opening(_)));

    // 完成開門動畫
    dw.tick_doors(1.0);
    assert_eq!(dw.doors[DOOR_FRONT_LEFT], DoorState::Open);

    // 關門
    dw.toggle_door(DOOR_FRONT_LEFT);
    assert!(matches!(dw.doors[DOOR_FRONT_LEFT], DoorState::Closing(_)));

    // 完成關門動畫
    dw.tick_doors(1.0);
    assert_eq!(dw.doors[DOOR_FRONT_LEFT], DoorState::Closed);
}

#[test]
fn door_animation_progress() {
    let mut dw = DoorWindowState::default();
    dw.toggle_door(0);

    // 半開
    let animating = dw.tick_doors(DOOR_ANIMATION_DURATION / 2.0);
    assert!(animating);
    if let DoorState::Opening(p) = dw.doors[0] {
        assert!((p - 0.5).abs() < 0.01);
    } else {
        panic!("應為 Opening 狀態");
    }
}

#[test]
fn door_drag_penalty_per_open_door() {
    let mut dw = DoorWindowState::default();
    dw.doors[0] = DoorState::Open;
    dw.doors[1] = DoorState::Open;
    dw.tick_doors(0.0); // 觸發 update_drag_penalty
    assert!((dw.drag_penalty - 0.10).abs() < f32::EPSILON);
}

#[test]
fn door_high_speed_breaks_open_doors() {
    let mut dw = DoorWindowState::default();
    dw.doors[0] = DoorState::Open;
    dw.doors[1] = DoorState::Closed;
    dw.check_high_speed_door_break(25.0);
    assert_eq!(dw.doors[0], DoorState::Broken);
    assert_eq!(dw.doors[1], DoorState::Closed); // 關門不受影響
}

#[test]
fn door_break_does_not_affect_closed() {
    let mut dw = DoorWindowState::default();
    dw.check_high_speed_door_break(25.0);
    for door in &dw.doors {
        assert_eq!(*door, DoorState::Closed);
    }
}

#[test]
fn window_break_and_crack() {
    let mut dw = DoorWindowState::default();
    dw.crack_window(0);
    assert_eq!(dw.windows[0], WindowState::Cracked);
    assert_eq!(dw.intact_window_count(), 3);

    dw.break_window(1);
    assert_eq!(dw.windows[1], WindowState::Broken);
    assert_eq!(dw.broken_window_count(), 1);
}

#[test]
fn window_crack_only_affects_intact() {
    let mut dw = DoorWindowState::default();
    dw.windows[0] = WindowState::Cracked;
    dw.crack_window(0); // 已裂，不應降級
    assert_eq!(dw.windows[0], WindowState::Cracked);
}

#[test]
fn door_window_out_of_bounds_ignored() {
    let mut dw = DoorWindowState::default();
    dw.toggle_door(99);
    dw.break_window(99);
    dw.crack_window(99);
    dw.break_door(99);
    assert_eq!(dw.broken_door_count(), 0);
    assert_eq!(dw.broken_window_count(), 0);
}

#[test]
fn door_window_repair_all() {
    let mut dw = DoorWindowState::default();
    dw.doors[0] = DoorState::Broken;
    dw.doors[1] = DoorState::Open;
    dw.windows[2] = WindowState::Broken;
    dw.windows[3] = WindowState::Cracked;
    dw.drag_penalty = 0.1;

    dw.repair_all();
    assert_eq!(dw.broken_door_count(), 0);
    assert_eq!(dw.broken_window_count(), 0);
    assert_eq!(dw.intact_window_count(), 4);
    assert_eq!(dw.drag_penalty, 0.0);
}

#[test]
fn door_angle_values() {
    assert_eq!(DoorState::Closed.angle(), 0.0);
    assert!((DoorState::Open.angle() - DOOR_MAX_ANGLE).abs() < f32::EPSILON);
    assert_eq!(DoorState::Broken.angle(), 0.0);
    assert!((DoorState::Opening(0.5).angle() - DOOR_MAX_ANGLE * 0.5).abs() < f32::EPSILON);
}

#[test]
fn door_broken_cannot_toggle() {
    let mut dw = DoorWindowState::default();
    dw.doors[0] = DoorState::Broken;
    dw.toggle_door(0);
    assert_eq!(dw.doors[0], DoorState::Broken); // 不應變化
}

// --- BodyPartDamage ---

#[test]
fn body_part_default_all_intact() {
    let bp = BodyPartDamage::default();
    for state in &bp.states {
        assert_eq!(*state, BodyPartState::Intact);
    }
    for dmg in &bp.damage {
        assert_eq!(*dmg, 0.0);
    }
}

#[test]
fn body_part_state_from_damage_thresholds() {
    assert_eq!(BodyPartState::from_damage(0.0), BodyPartState::Intact);
    assert_eq!(BodyPartState::from_damage(49.0), BodyPartState::Intact);
    assert_eq!(BodyPartState::from_damage(50.0), BodyPartState::Scratched);
    assert_eq!(BodyPartState::from_damage(149.0), BodyPartState::Scratched);
    assert_eq!(BodyPartState::from_damage(150.0), BodyPartState::Dented);
    assert_eq!(BodyPartState::from_damage(299.0), BodyPartState::Dented);
    assert_eq!(BodyPartState::from_damage(300.0), BodyPartState::Crushed);
}

#[test]
fn body_part_severity_ordering() {
    assert!(BodyPartState::Intact.severity() < BodyPartState::Scratched.severity());
    assert!(BodyPartState::Scratched.severity() < BodyPartState::Dented.severity());
    assert!(BodyPartState::Dented.severity() < BodyPartState::Crushed.severity());
}

#[test]
fn body_part_color_darken_factor_increases() {
    let f0 = BodyPartState::Intact.color_darken_factor();
    let f1 = BodyPartState::Scratched.color_darken_factor();
    let f2 = BodyPartState::Dented.color_darken_factor();
    let f3 = BodyPartState::Crushed.color_darken_factor();
    assert!(f0 < f1);
    assert!(f1 < f2);
    assert!(f2 < f3);
    assert!(f3 <= 1.0);
}

#[test]
fn body_part_apply_damage_updates_state() {
    let mut bp = BodyPartDamage::default();
    bp.apply_damage(BODY_HOOD, 100.0);
    assert_eq!(bp.states[BODY_HOOD], BodyPartState::Scratched);
    bp.apply_damage(BODY_HOOD, 100.0); // 累積到 200
    assert_eq!(bp.states[BODY_HOOD], BodyPartState::Dented);
    bp.apply_damage(BODY_HOOD, 150.0); // 累積到 350
    assert_eq!(bp.states[BODY_HOOD], BodyPartState::Crushed);
}

#[test]
fn body_part_apply_damage_out_of_bounds() {
    let mut bp = BodyPartDamage::default();
    bp.apply_damage(99, 100.0); // 不應 panic
    assert_eq!(bp.damaged_count(), 0);
}

#[test]
fn body_part_directional_damage_front() {
    let mut bp = BodyPartDamage::default();
    // 車頭方向碰撞（+Z）
    bp.apply_directional_damage(Vec3::new(0.0, 0.0, 1.0), 200.0);
    // 前保險桿應受 60% = 120 → Scratched
    assert_eq!(bp.states[BODY_FRONT_BUMPER], BodyPartState::Scratched);
    // 引擎蓋應受 40% = 80 → Scratched
    assert_eq!(bp.states[BODY_HOOD], BodyPartState::Scratched);
}

#[test]
fn body_part_directional_damage_rear() {
    let mut bp = BodyPartDamage::default();
    // 車尾方向碰撞（-Z）
    bp.apply_directional_damage(Vec3::new(0.0, 0.0, -1.0), 200.0);
    // 後保險桿應受 100% = 200 → Dented
    assert_eq!(bp.states[BODY_REAR_BUMPER], BodyPartState::Dented);
}

#[test]
fn body_part_directional_damage_left_side() {
    let mut bp = BodyPartDamage::default();
    // 左側碰撞（+X）
    bp.apply_directional_damage(Vec3::new(1.0, 0.0, 0.0), 300.0);
    // 左側板受 60% = 180 → Dented
    assert_eq!(bp.states[BODY_LEFT_PANEL], BodyPartState::Dented);
}

#[test]
fn body_part_directional_damage_top() {
    let mut bp = BodyPartDamage::default();
    // 上方碰撞（翻車）
    bp.apply_directional_damage(Vec3::new(0.0, 1.0, 0.0), 400.0);
    // 車頂受 100% = 400 → Crushed
    assert_eq!(bp.states[BODY_ROOF], BodyPartState::Crushed);
}

#[test]
fn body_part_worst_state() {
    let mut bp = BodyPartDamage::default();
    assert_eq!(bp.worst_state(), BodyPartState::Intact);
    bp.apply_damage(BODY_HOOD, 200.0);
    assert_eq!(bp.worst_state(), BodyPartState::Dented);
    bp.apply_damage(BODY_ROOF, 500.0);
    assert_eq!(bp.worst_state(), BodyPartState::Crushed);
}

#[test]
fn body_part_average_darken_factor() {
    let bp = BodyPartDamage::default();
    assert_eq!(bp.average_darken_factor(), 0.0);

    let mut bp2 = BodyPartDamage::default();
    // 所有部位 Crushed（darken = 0.55 each）
    for i in 0..BODY_PART_COUNT {
        bp2.apply_damage(i, 500.0);
    }
    assert!((bp2.average_darken_factor() - 0.55).abs() < 0.01);
}

#[test]
fn body_part_repair_all() {
    let mut bp = BodyPartDamage::default();
    bp.apply_damage(BODY_HOOD, 300.0);
    bp.apply_damage(BODY_LEFT_PANEL, 200.0);
    bp.repair_all();
    assert_eq!(bp.damaged_count(), 0);
    assert_eq!(bp.average_darken_factor(), 0.0);
}

#[test]
fn body_part_damaged_count() {
    let mut bp = BodyPartDamage::default();
    assert_eq!(bp.damaged_count(), 0);
    bp.apply_damage(BODY_HOOD, 100.0);
    bp.apply_damage(BODY_ROOF, 100.0);
    assert_eq!(bp.damaged_count(), 2);
}
