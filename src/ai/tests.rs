//! AI 系統單元測試

use super::*;
use super::decision::*;
use bevy::prelude::*;

// ============================================================================
// AiBehavior 測試
// ============================================================================

#[test]
fn test_ai_behavior_default() {
    let behavior = AiBehavior::default();
    assert_eq!(behavior.state, AiState::Idle);
    assert_eq!(behavior.state_timer, 0.0);
    assert!(behavior.target.is_none());
    assert!(behavior.last_known_target_pos.is_none());
    assert!(!behavior.is_fleeing);
    assert!(behavior.is_spawn_protected());
}

#[test]
fn test_ai_behavior_set_state() {
    let mut behavior = AiBehavior { state_timer: 5.0, ..AiBehavior::default() };

    behavior.set_state(AiState::Chase, 10.0);
    assert_eq!(behavior.state, AiState::Chase);
    assert_eq!(behavior.state_timer, 0.0); // 重置
    assert_eq!(behavior.last_state_change, 10.0);
}

#[test]
fn test_ai_behavior_set_state_same_state_no_reset() {
    let mut behavior = AiBehavior { state_timer: 5.0, ..AiBehavior::default() };

    behavior.set_state(AiState::Idle, 10.0); // 同狀態
    assert_eq!(behavior.state_timer, 5.0); // 不重置
}

#[test]
fn test_ai_behavior_tick() {
    let mut behavior = AiBehavior::default();
    behavior.tick(0.5);
    assert_eq!(behavior.state_timer, 0.5);

    behavior.tick(0.3);
    assert_eq!(behavior.state_timer, 0.8);
}

#[test]
fn test_ai_behavior_spawn_protection() {
    let mut behavior = AiBehavior::default();
    assert!(behavior.is_spawn_protected());

    behavior.tick(1.0);
    assert!(behavior.is_spawn_protected()); // 還在保護期

    behavior.tick(1.5); // 共 2.5 秒，超過 2.0 秒保護期
    assert!(!behavior.is_spawn_protected());
}

#[test]
fn test_ai_behavior_see_target() {
    let mut behavior = AiBehavior::default();
    let target = Entity::from_bits(42);
    let pos = Vec3::new(10.0, 0.0, 5.0);

    behavior.see_target(target, pos, 1.0);
    assert_eq!(behavior.target, Some(target));
    assert_eq!(behavior.last_known_target_pos, Some(pos));
    assert_eq!(behavior.last_seen_time, 1.0);
}

#[test]
fn test_ai_behavior_lose_target() {
    let mut behavior = AiBehavior { last_seen_time: 1.0, ..AiBehavior::default() };

    assert!(!behavior.lose_target(3.0, 5.0)); // 2s < 5s timeout
    assert!(behavior.lose_target(7.0, 5.0)); // 6s > 5s timeout
}

// ============================================================================
// AI 決策輔助函數測試
// ============================================================================

#[test]
fn test_handle_idle_to_chase_on_sight() {
    let config = AiConfig::default();
    let perception = AiPerception {
        can_see_target: true,
        has_heard_noise: false,
        noise_position: None,
        ..Default::default()
    };
    let mut behavior = AiBehavior::default();
    let mut movement = AiMovement::default();

    handle_idle_state(&config, &perception, &mut behavior, &mut movement, false, 1.0);

    assert_eq!(behavior.state, AiState::Chase);
    assert!(movement.is_running);
}

#[test]
fn test_handle_idle_to_alert_on_noise() {
    let config = AiConfig::default();
    let noise_pos = Vec3::new(10.0, 0.0, 0.0);
    let perception = AiPerception {
        can_see_target: false,
        has_heard_noise: true,
        noise_position: Some(noise_pos),
        ..Default::default()
    };
    let mut behavior = AiBehavior::default();
    let mut movement = AiMovement::default();

    handle_idle_state(&config, &perception, &mut behavior, &mut movement, false, 1.0);

    assert_eq!(behavior.state, AiState::Alert);
    assert_eq!(movement.move_target, Some(noise_pos));
}

#[test]
fn test_handle_idle_to_patrol_after_threshold() {
    let config = AiConfig::default();
    let perception = AiPerception::default();
    let mut behavior = AiBehavior { state_timer: config.patrol_idle_threshold + 1.0, ..AiBehavior::default() }; // 超過閾值
    let mut movement = AiMovement::default();

    handle_idle_state(&config, &perception, &mut behavior, &mut movement, true, 1.0);

    assert_eq!(behavior.state, AiState::Patrol);
}

#[test]
fn test_handle_idle_stays_idle_without_patrol() {
    let config = AiConfig::default();
    let perception = AiPerception::default();
    let mut behavior = AiBehavior { state_timer: config.patrol_idle_threshold + 1.0, ..AiBehavior::default() };
    let mut movement = AiMovement::default();

    handle_idle_state(&config, &perception, &mut behavior, &mut movement, false, 1.0);

    assert_eq!(behavior.state, AiState::Idle); // 沒有巡邏路徑，保持 Idle
}

#[test]
fn test_handle_patrol_to_chase_on_sight() {
    let perception = AiPerception {
        can_see_target: true,
        ..Default::default()
    };
    let mut behavior = AiBehavior::default();
    behavior.set_state(AiState::Patrol, 0.0);
    let mut movement = AiMovement::default();

    handle_patrol_state(&perception, &mut behavior, &mut movement, 1.0);

    assert_eq!(behavior.state, AiState::Chase);
}

#[test]
fn test_handle_alert_to_idle_on_timeout() {
    let config = AiConfig::default();
    let perception = AiPerception::default();
    let mut behavior = AiBehavior::default();
    behavior.set_state(AiState::Alert, 0.0);
    behavior.state_timer = config.alert_timeout + 1.0;
    let mut movement = AiMovement::default();

    handle_alert_state(&config, &perception, &mut behavior, &mut movement, false, 10.0);

    assert_eq!(behavior.state, AiState::Idle);
}

#[test]
fn test_handle_alert_to_patrol_on_timeout_with_patrol() {
    let config = AiConfig::default();
    let perception = AiPerception::default();
    let mut behavior = AiBehavior::default();
    behavior.set_state(AiState::Alert, 0.0);
    behavior.state_timer = config.alert_timeout + 1.0;
    let mut movement = AiMovement::default();

    handle_alert_state(&config, &perception, &mut behavior, &mut movement, true, 10.0);

    assert_eq!(behavior.state, AiState::Patrol);
}

#[test]
fn test_check_start_flee_triggers_below_threshold() {
    let config = AiConfig::default();
    let mut behavior = AiBehavior::default();
    let mut movement = AiMovement::default();
    behavior.last_known_target_pos = Some(Vec3::new(10.0, 0.0, 0.0));

    let result = check_start_flee(
        &config,
        0.1, // 10% 血量 < 20% 閾值
        &mut behavior,
        &mut movement,
        Vec3::ZERO,
        1.0,
    );

    assert!(result);
    assert!(behavior.is_fleeing);
    assert_eq!(behavior.state, AiState::Flee);
    assert!(movement.is_running);
    assert!(movement.move_target.is_some());
}

#[test]
fn test_check_start_flee_ignores_above_threshold() {
    let config = AiConfig::default();
    let mut behavior = AiBehavior::default();
    let mut movement = AiMovement::default();

    let result = check_start_flee(
        &config,
        0.5, // 50% > 20% 閾值
        &mut behavior,
        &mut movement,
        Vec3::ZERO,
        1.0,
    );

    assert!(!result);
    assert!(!behavior.is_fleeing);
    assert_eq!(behavior.state, AiState::Idle);
}

#[test]
fn test_handle_fleeing_state_stops_when_far() {
    let config = AiConfig::default();
    let mut behavior = AiBehavior { is_fleeing: true, last_known_target_pos: Some(Vec3::ZERO), ..AiBehavior::default() };
    let mut movement = AiMovement::default();

    // 遠離威脅超過 alert_distance (40m)
    let far_pos = Vec3::new(50.0, 0.0, 0.0);
    let result = handle_fleeing_state(&config, &mut behavior, &mut movement, far_pos, 1.0);

    assert!(result);
    assert!(!behavior.is_fleeing);
    assert_eq!(behavior.state, AiState::Alert);
}

#[test]
fn test_handle_fleeing_state_continues_when_close() {
    let config = AiConfig::default();
    let mut behavior = AiBehavior { is_fleeing: true, last_known_target_pos: Some(Vec3::ZERO), ..AiBehavior::default() };
    let mut movement = AiMovement::default();

    // 離威脅還很近 (10m < 40m)
    let close_pos = Vec3::new(10.0, 0.0, 0.0);
    let result = handle_fleeing_state(&config, &mut behavior, &mut movement, close_pos, 1.0);

    assert!(result);
    assert!(behavior.is_fleeing); // 繼續逃跑
    assert!(movement.move_target.is_some());
}

// ============================================================================
// AiPerception 測試
// ============================================================================

#[test]
fn test_ai_perception_is_in_fov() {
    let perception = AiPerception::default(); // 60 度視野
    let my_pos = Vec3::ZERO;
    let forward = Vec3::NEG_Z; // 面朝 -Z

    // 正前方
    assert!(perception.is_in_fov(my_pos, forward, Vec3::new(0.0, 0.0, -10.0)));
    // 稍微偏左（20 度內）
    assert!(perception.is_in_fov(my_pos, forward, Vec3::new(-3.6, 0.0, -10.0)));
    // 大幅偏左（超過 30 度）
    assert!(!perception.is_in_fov(my_pos, forward, Vec3::new(-10.0, 0.0, -5.0)));
    // 背後
    assert!(!perception.is_in_fov(my_pos, forward, Vec3::new(0.0, 0.0, 10.0)));
}

#[test]
fn test_ai_perception_is_in_sight_range() {
    let perception = AiPerception::default(); // 30m 視距
    let my_pos = Vec3::ZERO;

    assert!(perception.is_in_sight_range(my_pos, Vec3::new(20.0, 0.0, 0.0)));
    assert!(perception.is_in_sight_range(my_pos, Vec3::new(30.0, 0.0, 0.0)));
    assert!(!perception.is_in_sight_range(my_pos, Vec3::new(31.0, 0.0, 0.0)));
}

#[test]
fn test_ai_perception_is_in_hearing_range() {
    let perception = AiPerception::default(); // 50m 聽力
    let my_pos = Vec3::ZERO;

    assert!(perception.is_in_hearing_range(my_pos, Vec3::new(40.0, 0.0, 0.0)));
    assert!(perception.is_in_hearing_range(my_pos, Vec3::new(50.0, 0.0, 0.0)));
    assert!(!perception.is_in_hearing_range(my_pos, Vec3::new(51.0, 0.0, 0.0)));
}

// ============================================================================
// AiCombat 測試
// ============================================================================

#[test]
fn test_ai_combat_is_in_range() {
    let combat = AiCombat::default(); // attack_range = 20.0
    let my_pos = Vec3::ZERO;

    assert!(combat.is_in_range(my_pos, Vec3::new(15.0, 0.0, 0.0)));
    assert!(combat.is_in_range(my_pos, Vec3::new(20.0, 0.0, 0.0)));
    assert!(!combat.is_in_range(my_pos, Vec3::new(21.0, 0.0, 0.0)));
}

// ============================================================================
// AiState 轉換驗證測試（#19）
// ============================================================================

#[test]
fn test_ai_state_same_state_always_valid() {
    let states = [
        AiState::Idle, AiState::Patrol, AiState::Alert,
        AiState::Chase, AiState::Attack, AiState::Flee, AiState::TakingCover,
    ];
    for state in &states {
        assert!(state.can_transition_to(state));
    }
}

#[test]
fn test_ai_state_idle_transitions() {
    let idle = AiState::Idle;
    assert!(idle.can_transition_to(&AiState::Patrol));
    assert!(idle.can_transition_to(&AiState::Alert));
    assert!(idle.can_transition_to(&AiState::Chase));
    assert!(idle.can_transition_to(&AiState::Flee));
    assert!(!idle.can_transition_to(&AiState::Attack));
    assert!(!idle.can_transition_to(&AiState::TakingCover));
}

#[test]
fn test_ai_state_alert_is_hub() {
    let alert = AiState::Alert;
    assert!(alert.can_transition_to(&AiState::Idle));
    assert!(alert.can_transition_to(&AiState::Patrol));
    assert!(alert.can_transition_to(&AiState::Chase));
    assert!(alert.can_transition_to(&AiState::Attack));
    assert!(alert.can_transition_to(&AiState::Flee));
}

#[test]
fn test_ai_state_invalid_transitions() {
    assert!(!AiState::Flee.can_transition_to(&AiState::Attack));
    assert!(!AiState::Flee.can_transition_to(&AiState::Chase));
    assert!(!AiState::TakingCover.can_transition_to(&AiState::Idle));
    assert!(!AiState::TakingCover.can_transition_to(&AiState::Patrol));
    assert!(!AiState::Chase.can_transition_to(&AiState::Idle));
}
