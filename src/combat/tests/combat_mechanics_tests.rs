//! 戰鬥機制測試：近戰動畫、布娃娃、血液粒子、敵人類型、傷害事件、爆頭判定、受傷反應

use bevy::prelude::*;
use crate::combat::*;

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
