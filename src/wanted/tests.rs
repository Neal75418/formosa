//! 通緝系統單元測試

use super::components::*;
use bevy::prelude::*;

// ============================================================================
// 測試輔助函數
// ============================================================================

fn create_wanted_level() -> WantedLevel {
    WantedLevel::default()
}

fn wanted_level_with_heat(heat: f32) -> WantedLevel {
    let mut level = WantedLevel::default();
    level.heat = heat;
    level.stars = level.calculate_stars();
    level
}

/// 驗證切片中所有變體相等性（自身相等，其他不相等）
fn assert_variants_distinct<T: PartialEq + std::fmt::Debug>(variants: &[T]) {
    for (i, v1) in variants.iter().enumerate() {
        for (j, v2) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(v1, v2);
            } else {
                assert_ne!(v1, v2);
            }
        }
    }
}

// ============================================================================
// WantedLevel 測試
// ============================================================================

#[test]
fn test_wanted_level_default() {
    let level = create_wanted_level();
    assert_eq!(level.stars, 0);
    assert_eq!(level.heat, 0.0);
    assert_eq!(level.police_count, 0);
    assert_eq!(level.cooldown_timer, 0.0);
    assert!(!level.player_visible);
    assert!(level.player_last_seen_pos.is_none());
    assert!(level.search_center.is_none());
}

#[test]
fn test_calculate_stars_boundaries() {
    // 測試各星級邊界
    assert_eq!(wanted_level_with_heat(0.0).calculate_stars(), 0);
    assert_eq!(wanted_level_with_heat(19.0).calculate_stars(), 0);
    assert_eq!(wanted_level_with_heat(20.0).calculate_stars(), 1);
    assert_eq!(wanted_level_with_heat(39.0).calculate_stars(), 1);
    assert_eq!(wanted_level_with_heat(40.0).calculate_stars(), 2);
    assert_eq!(wanted_level_with_heat(59.0).calculate_stars(), 2);
    assert_eq!(wanted_level_with_heat(60.0).calculate_stars(), 3);
    assert_eq!(wanted_level_with_heat(79.0).calculate_stars(), 3);
    assert_eq!(wanted_level_with_heat(80.0).calculate_stars(), 4);
    assert_eq!(wanted_level_with_heat(99.0).calculate_stars(), 4);
    assert_eq!(wanted_level_with_heat(100.0).calculate_stars(), 5);
}

#[test]
fn test_add_heat() {
    let mut level = create_wanted_level();

    level.add_heat(25.0);
    assert_eq!(level.heat, 25.0);
    assert_eq!(level.stars, 1);

    level.add_heat(20.0);
    assert_eq!(level.heat, 45.0);
    assert_eq!(level.stars, 2);
}

#[test]
fn test_add_heat_capped_at_100() {
    let mut level = create_wanted_level();

    level.add_heat(150.0);
    assert_eq!(level.heat, 100.0);
    assert_eq!(level.stars, 5);
}

#[test]
fn test_reduce_heat() {
    let mut level = wanted_level_with_heat(50.0);

    level.reduce_heat(20.0);
    assert_eq!(level.heat, 30.0);
    assert_eq!(level.stars, 1);
}

#[test]
fn test_reduce_heat_minimum_zero() {
    let mut level = wanted_level_with_heat(10.0);

    level.reduce_heat(50.0);
    assert_eq!(level.heat, 0.0);
    assert_eq!(level.stars, 0);
}

#[test]
fn test_target_police_count() {
    assert_eq!(wanted_level_with_heat(0.0).target_police_count(), 0);
    assert_eq!(wanted_level_with_heat(20.0).target_police_count(), 2);
    assert_eq!(wanted_level_with_heat(40.0).target_police_count(), 4);
    assert_eq!(wanted_level_with_heat(60.0).target_police_count(), 6);
    assert_eq!(wanted_level_with_heat(80.0).target_police_count(), 8);
    assert_eq!(wanted_level_with_heat(100.0).target_police_count(), 10);
}

#[test]
fn test_cooldown_duration() {
    assert_eq!(wanted_level_with_heat(0.0).cooldown_duration(), 5.0);
    assert_eq!(wanted_level_with_heat(20.0).cooldown_duration(), 10.0);
    assert_eq!(wanted_level_with_heat(40.0).cooldown_duration(), 15.0);
    assert_eq!(wanted_level_with_heat(60.0).cooldown_duration(), 20.0);
    assert_eq!(wanted_level_with_heat(80.0).cooldown_duration(), 40.0);  // 4星：提高消退時間
    assert_eq!(wanted_level_with_heat(100.0).cooldown_duration(), 60.0); // 5星：提高消退時間
}

// ============================================================================
// PoliceConfig 測試
// ============================================================================

#[test]
fn test_police_config_default() {
    let config = PoliceConfig::default();

    assert_eq!(config.spawn_interval, 3.0);
    assert_eq!(config.spawn_distance_min, 30.0);
    assert_eq!(config.spawn_distance_max, 50.0);
    assert_eq!(config.despawn_distance, 80.0);
    assert_eq!(config.vision_range, 40.0);
    assert_eq!(config.attack_range, 25.0);
    assert_eq!(config.walk_speed, 3.0);
    assert_eq!(config.run_speed, 6.0);
    assert_eq!(config.damage, 15.0);
    assert_eq!(config.attack_cooldown, 1.5);
    assert_eq!(config.base_hit_chance, 0.28);
}

#[test]
fn test_police_config_vision_fov() {
    let config = PoliceConfig::default();
    // 60 度視野角
    let expected_fov = std::f32::consts::PI / 3.0;
    assert!((config.vision_fov - expected_fov).abs() < 0.001);
}

// ============================================================================
// PoliceOfficer 測試
// ============================================================================

#[test]
fn test_police_officer_default() {
    let officer = PoliceOfficer::default();

    assert_eq!(officer.state, PoliceState::Patrolling);
    assert!(officer.patrol_route.is_empty());
    assert_eq!(officer.patrol_index, 0);
    assert!(!officer.target_player);
    assert_eq!(officer.search_timer, 0.0);
    assert_eq!(officer.attack_cooldown, 0.0);
    assert_eq!(officer.officer_type, PoliceType::Patrol);
    assert_eq!(officer.radio_cooldown, 0.0);
    assert!(!officer.radio_alerted);
    assert!(officer.radio_alert_position.is_none());
}

// ============================================================================
// PoliceState 測試
// ============================================================================

#[test]
fn test_police_state_default() {
    let state = PoliceState::default();
    assert_eq!(state, PoliceState::Patrolling);
}

#[test]
fn test_police_state_variants() {
    let states = [
        PoliceState::Patrolling,
        PoliceState::Alerted,
        PoliceState::Pursuing,
        PoliceState::Searching,
        PoliceState::Engaging,
        PoliceState::Returning,
    ];

    assert_variants_distinct(&states);
}

// ============================================================================
// PoliceType 測試
// ============================================================================

#[test]
fn test_police_type_default() {
    let ptype = PoliceType::default();
    assert_eq!(ptype, PoliceType::Patrol);
}

#[test]
fn test_police_type_variants() {
    let types = [
        PoliceType::Patrol,
        PoliceType::Swat,
        PoliceType::Vehicular,
    ];

    assert_variants_distinct(&types);
}

// ============================================================================
// SearchZone 測試
// ============================================================================

#[test]
fn test_search_zone_creation() {
    let zone = SearchZone {
        center: Vec3::new(10.0, 0.0, 20.0),
        radius: 50.0,
        lifetime: 30.0,
    };

    assert_eq!(zone.center, Vec3::new(10.0, 0.0, 20.0));
    assert_eq!(zone.radius, 50.0);
    assert_eq!(zone.lifetime, 30.0);
}

// ============================================================================
// WantedStar 測試
// ============================================================================

#[test]
fn test_wanted_star_creation() {
    for i in 0..5 {
        let star = WantedStar { index: i };
        assert_eq!(star.index, i);
    }
}

// ============================================================================
// 整合情境測試
// ============================================================================

#[test]
fn test_wanted_level_crime_escalation() {
    let mut level = create_wanted_level();

    // 輕微犯罪（小偷）
    level.add_heat(25.0);
    assert_eq!(level.stars, 1);
    assert_eq!(level.target_police_count(), 2);

    // 攻擊平民
    level.add_heat(20.0);
    assert_eq!(level.stars, 2);
    assert_eq!(level.target_police_count(), 4);

    // 攻擊警察
    level.add_heat(25.0);
    assert_eq!(level.stars, 3);
    assert_eq!(level.target_police_count(), 6);

    // 大規模破壞
    level.add_heat(20.0);
    assert_eq!(level.stars, 4);
    assert_eq!(level.target_police_count(), 8);

    // 極端暴力
    level.add_heat(20.0);
    assert_eq!(level.stars, 5);
    assert_eq!(level.target_police_count(), 10);
}

#[test]
fn test_wanted_level_cooldown_escape() {
    let mut level = wanted_level_with_heat(80.0); // 4 星
    assert_eq!(level.stars, 4);

    // 模擬逃脫過程中熱度降低
    level.reduce_heat(25.0);
    assert_eq!(level.stars, 2);

    level.reduce_heat(25.0);
    assert_eq!(level.stars, 1);

    level.reduce_heat(25.0);
    assert_eq!(level.stars, 0);
}

#[test]
fn test_heat_precision() {
    let mut level = create_wanted_level();

    // 測試小數精度
    level.add_heat(19.5);
    assert_eq!(level.stars, 0);

    level.add_heat(0.5);
    assert_eq!(level.stars, 1);
}

#[test]
fn test_police_officer_state_transitions() {
    let mut officer = PoliceOfficer::default();

    // 巡邏 -> 警覺
    assert_eq!(officer.state, PoliceState::Patrolling);
    officer.state = PoliceState::Alerted;
    assert_eq!(officer.state, PoliceState::Alerted);

    // 警覺 -> 追捕
    officer.state = PoliceState::Pursuing;
    officer.target_player = true;
    assert_eq!(officer.state, PoliceState::Pursuing);
    assert!(officer.target_player);

    // 追捕 -> 戰鬥
    officer.state = PoliceState::Engaging;
    assert_eq!(officer.state, PoliceState::Engaging);

    // 失去視線 -> 搜索
    officer.state = PoliceState::Searching;
    officer.search_timer = 10.0;
    assert_eq!(officer.state, PoliceState::Searching);
    assert_eq!(officer.search_timer, 10.0);

    // 搜索超時 -> 返回
    officer.state = PoliceState::Returning;
    officer.target_player = false;
    assert_eq!(officer.state, PoliceState::Returning);
    assert!(!officer.target_player);
}

#[test]
fn test_radio_alert_system() {
    let mut officer = PoliceOfficer::default();

    // 初始狀態
    assert!(!officer.radio_alerted);
    assert!(officer.radio_alert_position.is_none());

    // 收到無線電通報
    officer.radio_alerted = true;
    officer.radio_alert_position = Some(Vec3::new(100.0, 0.0, 200.0));

    assert!(officer.radio_alerted);
    assert_eq!(
        officer.radio_alert_position,
        Some(Vec3::new(100.0, 0.0, 200.0))
    );
}
