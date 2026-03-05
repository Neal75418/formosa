//! AI 攻擊系統（射擊、近戰、精度計算）

use bevy::prelude::*;
use rand::Rng;

use super::{AiBehavior, AiCombat, AiConfig, AiPerception, AiState};
use crate::combat::{
    spawn_bullet_tracer, CombatVisuals, DamageEvent, DamageSource, Enemy, EnemyArm,
    EnemyPunchAnimation, MuzzleFlash, PunchAnimatable, PunchPhase, TracerStyle, Weapon,
    MELEE_DAMAGE,
};
use crate::core::{ease_in_out_quad, ease_out_cubic, ease_out_quad};
use crate::player::Player;

/// 近戰攻擊冷卻時間（秒）
const MELEE_FIRE_COOLDOWN: f32 = 0.5;

// ============================================================================
// 攻擊系統
// ============================================================================

// ============================================================================
// 攻擊系統輔助函數
// ============================================================================
/// 更新武器冷卻和換彈狀態
/// 返回 true 表示正在換彈，應跳過攻擊
#[inline]
fn update_weapon_state(weapon: &mut Weapon, dt: f32) -> bool {
    weapon.tick_cooldown(dt);
    weapon.tick_reload(dt)
}

/// 檢查是否滿足攻擊前置條件
#[inline]
fn check_attack_preconditions(behavior: &AiBehavior, perception: &AiPerception) -> bool {
    !behavior.is_spawn_protected() && behavior.state == AiState::Attack && perception.can_see_target
}

/// 執行近戰攻擊
/// 返回 true 表示觸發了近戰攻擊
#[inline]
fn execute_melee_attack(
    commands: &mut Commands,
    children: &Children,
    arm_query: &Query<(Entity, &EnemyArm), Without<EnemyPunchAnimation>>,
    player_entity: Entity,
    enemy_entity: Entity,
    weapon: &mut Weapon,
) -> bool {
    if weapon.is_cooling_down() {
        return false;
    }

    // 找到右手臂觸發揮拳動畫
    for child in children.iter() {
        if let Ok((arm_entity, arm)) = arm_query.get(child) {
            if arm.is_right {
                commands
                    .entity(arm_entity)
                    .insert(EnemyPunchAnimation::with_target(
                        player_entity,
                        enemy_entity,
                    ));
                break;
            }
        }
    }

    weapon.set_fire_cooldown(MELEE_FIRE_COOLDOWN);
    true
}

/// 計算射擊精度（含距離衰減）
#[inline]
fn calculate_effective_accuracy(
    config: &AiConfig,
    base_accuracy: f32,
    weapon_range: f32,
    target_distance: f32,
) -> f32 {
    let half_range = weapon_range * 0.5;
    let range_penalty = if target_distance > half_range {
        let over_range = (target_distance - half_range) / half_range;
        over_range.clamp(0.0, config.max_range_penalty)
    } else {
        0.0
    };
    (base_accuracy - range_penalty).max(config.min_accuracy)
}

/// 計算彈道終點（命中或未命中）
#[inline]
fn calculate_tracer_end(
    config: &AiConfig,
    hit_roll: f32,
    effective_accuracy: f32,
    player_pos: Vec3,
    player_entity: Entity,
    enemy_entity: Entity,
    muzzle_pos: Vec3,
    damage: f32,
    damage_events: &mut MessageWriter<DamageEvent>,
) -> Vec3 {
    if hit_roll <= effective_accuracy {
        // 命中
        damage_events.write(
            DamageEvent::new(player_entity, damage, DamageSource::Bullet)
                .with_attacker(enemy_entity)
                .with_position(muzzle_pos),
        );
        player_pos + Vec3::Y * 1.0
    } else {
        // 未命中 - 偏移到玩家附近
        let mut rng = rand::rng();
        let miss_offset = Vec3::new(
            rng.random_range(-config.miss_spread_x..config.miss_spread_x),
            rng.random_range(config.miss_spread_y_min..config.miss_spread_y_max),
            rng.random_range(-config.miss_spread_z..config.miss_spread_z),
        );
        player_pos + Vec3::Y * config.player_body_height + miss_offset
    }
}

/// 生成槍口特效和子彈拖尾
#[inline]
fn spawn_muzzle_effects(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    muzzle_pos: Vec3,
    tracer_end: Vec3,
) {
    // 槍口閃光
    commands.spawn((
        Mesh3d(visuals.muzzle_mesh.clone()),
        MeshMaterial3d(visuals.muzzle_material.clone()),
        Transform::from_translation(muzzle_pos),
        MuzzleFlash { lifetime: 0.05 },
    ));

    // 子彈拖尾
    spawn_bullet_tracer(
        commands,
        visuals,
        muzzle_pos,
        tracer_end,
        TracerStyle::Rifle,
    );
}

/// 執行遠程攻擊的射擊邏輯
/// 返回 true 表示成功開火
#[inline]
#[allow(clippy::too_many_arguments, clippy::ref_option)]
fn execute_ranged_attack(
    config: &AiConfig,
    commands: &mut Commands,
    visuals: &Option<Res<CombatVisuals>>,
    transform: &Transform,
    combat: &mut AiCombat,
    weapon: &mut Weapon,
    player_pos: Vec3,
    player_entity: Entity,
    enemy_entity: Entity,
    target_distance: f32,
    damage_events: &mut MessageWriter<DamageEvent>,
) -> bool {
    // 檢查是否需要換彈
    if weapon.needs_reload() {
        weapon.start_reload(1.0); // AI 無技能加成
        return false;
    }

    // 檢查是否可以開火
    let should_fire = combat.can_attack() || combat.should_fire_next();
    if !should_fire || !weapon.can_fire() {
        return false;
    }

    // 計算槍口位置
    let forward = transform.forward();
    let muzzle_pos = transform.translation
        + forward.as_vec3() * config.muzzle_forward_offset
        + Vec3::new(0.0, config.muzzle_height_offset, 0.0);

    // 計算精度和彈道終點
    let mut rng = rand::rng();
    let hit_roll: f32 = rng.random();
    let effective_accuracy = calculate_effective_accuracy(
        config,
        combat.accuracy,
        weapon.effective_range(),
        target_distance,
    );
    let tracer_end = calculate_tracer_end(
        config,
        hit_roll,
        effective_accuracy,
        player_pos,
        player_entity,
        enemy_entity,
        muzzle_pos,
        weapon.base_damage(),
        damage_events,
    );

    // 生成特效
    if let Some(ref vis) = visuals {
        spawn_muzzle_effects(commands, vis, muzzle_pos, tracer_end);
    }

    // 消耗彈藥並更新狀態
    weapon.consume_ammo();
    weapon.reset_fire_cooldown();
    combat.fire_once();
    true
}

/// AI 攻擊系統：向玩家開火或近戰攻擊
#[allow(clippy::too_many_arguments)]
pub fn ai_attack_system(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<AiConfig>,
    visuals: Option<Res<CombatVisuals>>,
    mut enemy_query: Query<
        (
            Entity,
            &Transform,
            &AiBehavior,
            &AiPerception,
            &mut AiCombat,
            &mut Weapon,
            &Children,
        ),
        With<Enemy>,
    >,
    player_query: Query<(Entity, &Transform), With<Player>>,
    arm_query: Query<(Entity, &EnemyArm), Without<EnemyPunchAnimation>>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let dt = time.delta_secs();

    // 取得玩家位置（用於子彈拖尾終點）
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (enemy_entity, transform, behavior, perception, mut combat, mut weapon, children) in
        &mut enemy_query
    {
        // 更新冷卻
        combat.tick(dt);

        // 更新武器狀態（換彈中跳過）
        if update_weapon_state(&mut weapon, dt) {
            continue;
        }

        // 檢查攻擊前置條件
        if !check_attack_preconditions(behavior, perception) {
            continue;
        }

        // 計算與目標的距離（平方）
        let target_distance_sq = behavior
            .last_known_target_pos
            .map_or(f32::MAX, |pos| transform.translation.distance_squared(pos));

        // 判斷攻擊類型
        // MELEE_ATTACK_RANGE_SQ = 6.25
        if target_distance_sq <= 6.25 {
            // 近戰攻擊
            execute_melee_attack(
                &mut commands,
                children,
                &arm_query,
                player_entity,
                enemy_entity,
                &mut weapon,
            );
        } else {
            let target_distance = target_distance_sq.sqrt();
            // 遠程攻擊
            execute_ranged_attack(
                &config,
                &mut commands,
                &visuals,
                transform,
                &mut combat,
                &mut weapon,
                player_pos,
                player_entity,
                enemy_entity,
                target_distance,
                &mut damage_events,
            );
        }
    }
}

// ============================================================================
// 敵人揮拳動畫系統
// ============================================================================

// ============================================================================
// 揮拳動畫輔助函數
// ============================================================================
/// 應用蓄力階段動畫
#[inline]
fn apply_wind_up_animation(transform: &mut Transform, arm: &EnemyArm, t: f32, wind_up_end: f32) {
    let phase_progress = t / wind_up_end;
    let ease = ease_out_quad(phase_progress);
    let rest_z = arm.rest_rotation.to_euler(EulerRot::XYZ).2;

    transform.rotation = Quat::from_euler(EulerRot::XYZ, -0.3 * ease, 0.0, rest_z + 0.3 * ease);
}

/// 應用出拳階段動畫
#[inline]
fn apply_strike_animation(
    transform: &mut Transform,
    arm: &EnemyArm,
    t: f32,
    wind_up_end: f32,
    strike_end: f32,
) {
    let phase_t = t - wind_up_end;
    let phase_duration = strike_end - wind_up_end;
    let phase_progress = phase_t / phase_duration;
    let ease = ease_out_cubic(phase_progress);
    let rest_z = arm.rest_rotation.to_euler(EulerRot::XYZ).2;

    let rotation = Quat::from_euler(EulerRot::XYZ, 1.4 * ease, 0.0, rest_z * (1.0 - ease));

    transform.translation = arm.rest_position + Vec3::new(0.0, 0.0, 0.4 * ease);
    transform.rotation = rotation;
}

/// 應用收回階段動畫
#[inline]
fn apply_return_animation(
    transform: &mut Transform,
    arm: &EnemyArm,
    t: f32,
    strike_end: f32,
    duration: f32,
) {
    let phase_t = t - strike_end;
    let phase_duration = duration - strike_end;
    let phase_progress = phase_t / phase_duration;
    let ease = ease_in_out_quad(phase_progress);

    let strike_rotation = Quat::from_euler(EulerRot::XYZ, 1.4, 0.0, 0.0);
    let strike_offset = Vec3::new(0.0, 0.0, 0.4);

    transform.translation = (arm.rest_position + strike_offset).lerp(arm.rest_position, ease);
    transform.rotation = strike_rotation.slerp(arm.rest_rotation, ease);
}

/// 敵人揮拳動畫更新系統
/// 處理手臂動畫的三個階段：WindUp → Strike → Return
/// 在 Strike 階段發送傷害事件
pub fn enemy_punch_animation_system(
    time: Res<Time>,
    mut commands: Commands,
    mut arm_query: Query<(Entity, &EnemyArm, &mut Transform, &mut EnemyPunchAnimation)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let dt = time.delta_secs();

    for (entity, arm, mut transform, mut anim) in &mut arm_query {
        anim.timer += dt;

        // 更新階段
        anim.update_phase();

        let (wind_up_end, strike_end, duration) = anim.phase_times();
        let t = anim.timer;

        // 進入 Strike 階段時發送傷害事件
        if anim.phase == PunchPhase::Strike && !anim.has_damage_dealt {
            if let (Some(target), Some(attacker)) = (anim.target, anim.attacker) {
                damage_events.write(
                    DamageEvent::new(target, MELEE_DAMAGE, DamageSource::Melee)
                        .with_attacker(attacker),
                );
                anim.has_damage_dealt = true;
            }
        }

        // 只處理右手臂的動畫
        if !arm.is_right {
            continue;
        }

        // 應用階段動畫
        match anim.phase {
            PunchPhase::WindUp => apply_wind_up_animation(&mut transform, arm, t, wind_up_end),
            PunchPhase::Strike => {
                apply_strike_animation(&mut transform, arm, t, wind_up_end, strike_end);
            }
            PunchPhase::Return => {
                apply_return_animation(&mut transform, arm, t, strike_end, duration);
            }
        }

        // 動畫結束，移除組件
        if anim.is_finished() {
            transform.translation = arm.rest_position;
            transform.rotation = arm.rest_rotation;
            commands.entity(entity).remove::<EnemyPunchAnimation>();
        }
    }
}
