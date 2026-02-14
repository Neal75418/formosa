//! 傷害計算系統
//!
//! 處理傷害事件：掩體減免、護甲吸收、扣血、觸發死亡。

use bevy::prelude::*;

use super::effects::spawn_floating_damage_number;
use super::DamageSystemResources;
use crate::combat::components::*;
use crate::combat::health::*;
use crate::ai::{CoverPoint, CoverSeeker};
use crate::audio::{play_hit_sound, AudioManager, WeaponSounds};
use crate::player::Player;
use crate::ui::{
    trigger_damage_indicator, ChineseFont, DamageIndicatorState, FloatingDamageNumber,
    FloatingDamageTracker, NotificationQueue,
};

// ============================================================================
// 傷害計算常數
// ============================================================================
/// 命中標記顯示時長（秒）
const HIT_MARKER_DURATION: f32 = 0.2;
/// 浮動傷害數字頭頂偏移
const FLOATING_DAMAGE_HEAD_OFFSET: f32 = 1.8;
/// 預設受傷位置 Y 偏移
const DEFAULT_HIT_POSITION_Y_OFFSET: f32 = 1.2;
/// 命中位置浮動偏移（Y 軸）
const FLOATING_DAMAGE_HIT_OFFSET: f32 = 0.3;

// ============================================================================
// 傷害系統輔助函數
// ============================================================================

/// 計算掩體傷害減免（含攻擊方向檢測）
#[inline]
fn calculate_cover_reduction(
    seeker: Option<&CoverSeeker>,
    cover_point_query: &Query<&CoverPoint>,
    attacker: Option<Entity>,
    target_pos: Vec3,
    transform_query: &Query<&Transform>,
) -> f32 {
    let Some(seeker) = seeker else { return 0.0 };
    if !seeker.is_in_cover || seeker.is_peeking {
        return 0.0;
    }
    let Some(cover_entity) = seeker.target_cover else {
        return 0.0;
    };
    let Ok(cover) = cover_point_query.get(cover_entity) else {
        return 0.0;
    };

    // 檢查攻擊方向是否與掩體保護方向一致
    if let Some(attacker_transform) = attacker.and_then(|e| transform_query.get(e).ok()) {
        let attack_dir = target_pos - attacker_transform.translation;
        let attack_dir_2d = Vec3::new(attack_dir.x, 0.0, attack_dir.z).normalize_or_zero();
        let cover_dir_2d = Vec3::new(cover.cover_direction.x, 0.0, cover.cover_direction.z)
            .normalize_or_zero();

        // 如果攻擊從掩體背面來（夾角 > 90°），掩體無效
        if attack_dir_2d.dot(cover_dir_2d) < 0.0 {
            return 0.0;
        }
    }

    cover.damage_reduction
}

/// 護甲吸收結果
struct ArmorAbsorptionResult {
    damage_after_armor: f32,
    was_hit: bool,
    was_broken: bool,
}

/// 處理護甲傷害吸收
#[inline]
fn process_armor_absorption(armor: &mut Option<Mut<Armor>>, damage: f32) -> ArmorAbsorptionResult {
    let Some(ref mut armor) = armor else {
        return ArmorAbsorptionResult {
            damage_after_armor: damage,
            was_hit: false,
            was_broken: false,
        };
    };

    let armor_before = armor.current;
    let damage_after = armor.absorb_damage(damage);

    ArmorAbsorptionResult {
        damage_after_armor: damage_after,
        was_hit: armor_before > 0.0,
        was_broken: armor_before > 0.0 && armor.current <= 0.0,
    }
}

/// 發送護甲破碎事件
#[inline]
fn send_armor_break_event(
    armor_result: &ArmorAbsorptionResult,
    target: Entity,
    hit_position: Option<Vec3>,
    transform_query: &Query<&Transform>,
    armor_break_events: &mut MessageWriter<ArmorBreakEvent>,
) {
    if !armor_result.was_hit {
        return;
    }
    let Ok(target_transform) = transform_query.get(target) else {
        return;
    };

    let hit_pos = hit_position
        .unwrap_or(target_transform.translation + Vec3::Y * DEFAULT_HIT_POSITION_Y_OFFSET);
    armor_break_events.write(ArmorBreakEvent {
        entity: target,
        position: hit_pos,
        is_full_break: armor_result.was_broken,
    });
}

/// 計算擊中方向
#[inline]
fn calculate_hit_direction(
    attacker: Option<Entity>,
    target: Entity,
    transform_query: &Query<&Transform>,
) -> Vec3 {
    let Some(attacker) = attacker else {
        return Vec3::NEG_Z;
    };
    let Ok(target_transform) = transform_query.get(target) else {
        return Vec3::NEG_Z;
    };
    let Ok(attacker_transform) = transform_query.get(attacker) else {
        return Vec3::NEG_Z;
    };

    (target_transform.translation - attacker_transform.translation).normalize_or_zero()
}

/// 觸發受傷反應
#[inline]
fn trigger_hit_reaction(
    hit_reaction: &mut Option<Mut<HitReaction>>,
    damage_dealt: f32,
    attacker: Option<Entity>,
    target: Entity,
    is_headshot: bool,
    force_knockback: bool,
    transform_query: &Query<&Transform>,
) {
    let Some(ref mut reaction) = hit_reaction else {
        return;
    };
    let hit_direction = calculate_hit_direction(attacker, target, transform_query);
    /// 連擊終結技的最低擊退傷害值（確保超過 Knockback 門檻 40.0）
    const FINISHER_KNOCKBACK_FORCE: f32 = 50.0;

    if force_knockback {
        // 連擊終結技：取實際傷害與最低擊退力的較大值
        reaction.trigger(damage_dealt.max(FINISHER_KNOCKBACK_FORCE), hit_direction, false);
    } else {
        reaction.trigger(damage_dealt, hit_direction, is_headshot);
    }
}

/// 處理玩家受傷通知
#[inline]
fn handle_player_damage_notification(
    target: Entity,
    damage_dealt: f32,
    attacker: Option<Entity>,
    player_query: &Query<Entity, With<Player>>,
    transform_query: &Query<&Transform>,
    notifications: &mut NotificationQueue,
    damage_indicator: &mut DamageIndicatorState,
) {
    if player_query.get(target).is_err() {
        return;
    }
    notifications.warning(format!("-{:.0} HP", damage_dealt));

    // 計算傷害方向（從玩家指向攻擊者）
    let direction = calculate_damage_direction(target, attacker, transform_query);
    trigger_damage_indicator(damage_indicator, damage_dealt, direction);
}

/// 計算傷害方向（用於 UI 指示器）
/// 返回從玩家指向攻擊者的 2D 方向（XZ 平面投影到螢幕座標）
#[inline]
fn calculate_damage_direction(
    target: Entity,
    attacker: Option<Entity>,
    transform_query: &Query<&Transform>,
) -> Option<Vec2> {
    let attacker_entity = attacker?;
    let target_transform = transform_query.get(target).ok()?;
    let attacker_transform = transform_query.get(attacker_entity).ok()?;

    // 世界座標方向（XZ 平面）
    let world_dir = attacker_transform.translation - target_transform.translation;
    let world_dir_2d = Vec2::new(world_dir.x, world_dir.z);

    // 距離太近時無法判斷方向（避免除以零和不穩定的方向）
    // 閾值 0.25 = 0.5m 距離的平方，近戰/爆炸時返回 None
    const MIN_DIRECTION_DISTANCE_SQ: f32 = 0.25;
    if world_dir_2d.length_squared() < MIN_DIRECTION_DISTANCE_SQ {
        return None;
    }

    // 轉換為相對於玩家朝向的方向
    // 玩家 forward 是 -Z，所以要相對於玩家的旋轉來計算
    let player_forward = target_transform.forward();
    let player_right = target_transform.right();

    // 投影到玩家的前後左右
    let forward_component = world_dir.x * player_forward.x + world_dir.z * player_forward.z;
    let right_component = world_dir.x * player_right.x + world_dir.z * player_right.z;

    // 螢幕座標：X 正向是右，Y 正向是上
    // forward（前方）= 上，right（右方）= 右
    let screen_dir = Vec2::new(right_component, -forward_component).normalize_or_zero();

    Some(screen_dir)
}

/// 處理命中標記和音效
#[inline]
fn handle_hit_marker_and_sound(
    is_headshot: bool,
    combat_state: &mut CombatState,
    commands: &mut Commands,
    weapon_sounds: &Option<Res<WeaponSounds>>,
    audio_manager: &AudioManager,
) {
    combat_state.hit_marker_timer = HIT_MARKER_DURATION;
    combat_state.hit_marker_headshot = is_headshot;

    if let Some(ref sounds) = weapon_sounds {
        play_hit_sound(commands, sounds, audio_manager, is_headshot);
    }
}

/// 計算浮動傷害數字位置
#[inline]
fn get_floating_damage_position(
    hit_position: Option<Vec3>,
    target: Entity,
    transform_query: &Query<&Transform>,
) -> Option<Vec3> {
    if let Some(hit_pos) = hit_position {
        return Some(hit_pos + Vec3::Y * FLOATING_DAMAGE_HIT_OFFSET);
    }
    transform_query
        .get(target)
        .ok()
        .map(|t| t.translation + Vec3::Y * FLOATING_DAMAGE_HEAD_OFFSET)
}

/// 生成浮動傷害數字
#[inline]
fn spawn_floating_damage_if_possible(
    commands: &mut Commands,
    damage_pos: Vec3,
    damage_dealt: f32,
    is_headshot: bool,
    damage_tracker: &mut FloatingDamageTracker,
    font: &Option<Res<ChineseFont>>,
) {
    if damage_tracker.active_count >= damage_tracker.max_count {
        return;
    }
    let Some(ref chinese_font) = font else { return };

    let offset = damage_tracker.next_offset();
    let floating_damage =
        FloatingDamageNumber::new(damage_pos, damage_dealt, is_headshot).with_offset(offset);

    spawn_floating_damage_number(commands, floating_damage, chinese_font);
    damage_tracker.active_count += 1;
}

/// 處理玩家攻擊敵人的效果（命中標記、音效、浮動傷害）
fn handle_player_hit_enemy(
    event: &DamageEvent,
    damage_dealt: f32,
    player_entity: Option<Entity>,
    enemy_query: &Query<Entity, With<Enemy>>,
    transform_query: &Query<&Transform>,
    res: &mut DamageSystemResources,
    commands: &mut Commands,
) {
    let Some(player) = player_entity else { return };
    if event.attacker != Some(player) || enemy_query.get(event.target).is_err() {
        return;
    }

    handle_hit_marker_and_sound(
        event.is_headshot,
        &mut res.combat_state,
        commands,
        &res.weapon_sounds,
        &res.audio_manager,
    );

    let Some(damage_pos) =
        get_floating_damage_position(event.hit_position, event.target, transform_query)
    else {
        return;
    };

    spawn_floating_damage_if_possible(
        commands,
        damage_pos,
        damage_dealt,
        event.is_headshot,
        &mut res.damage_tracker,
        &res.font,
    );
}

/// 計算死亡時的擊中方向
#[inline]
fn calculate_death_hit_direction(
    attacker: Option<Entity>,
    target: Entity,
    transform_query: &Query<&Transform>,
) -> Option<Vec3> {
    let default_dir = Some(Vec3::new(0.0, 0.2, -1.0).normalize());

    let Some(attacker) = attacker else {
        return default_dir;
    };
    let Ok(target_transform) = transform_query.get(target) else {
        return default_dir;
    };
    let Ok(attacker_transform) = transform_query.get(attacker) else {
        return default_dir;
    };

    let dir = target_transform.translation - attacker_transform.translation;
    Some(Vec3::new(dir.x, 0.3, dir.z).normalize_or_zero())
}

/// 傷害處理系統
#[allow(clippy::too_many_arguments)]
pub fn damage_system(
    mut damage_events: MessageReader<DamageEvent>,
    mut death_events: MessageWriter<DeathEvent>,
    mut armor_break_events: MessageWriter<ArmorBreakEvent>,
    mut commands: Commands,
    // 合併查詢：Health + Armor + CoverSeeker + HitReaction (同一實體)
    mut health_query: Query<(
        &mut Health,
        Option<&mut Armor>,
        Option<&CoverSeeker>,
        Option<&mut HitReaction>,
    )>,
    cover_point_query: Query<&CoverPoint>,
    player_query: Query<Entity, With<Player>>,
    enemy_query: Query<Entity, With<Enemy>>,
    transform_query: Query<&Transform>,
    // 資源參數包（解決 Bevy 16 參數限制）
    mut res: DamageSystemResources,
) {
    let current_time = res.time.elapsed_secs();
    let player_entity = player_query.single().ok();

    for event in damage_events.read() {
        let Ok((mut health, mut armor, cover_seeker, mut hit_reaction)) =
            health_query.get_mut(event.target)
        else {
            continue;
        };

        // 計算掩體傷害減免（含攻擊方向檢測）
        let target_pos = transform_query
            .get(event.target)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);
        let cover_reduction = calculate_cover_reduction(
            cover_seeker,
            &cover_point_query,
            event.attacker,
            target_pos,
            &transform_query,
        );
        let actual_damage = event.amount * (1.0 - cover_reduction);

        // 處理護甲吸收
        let armor_result = process_armor_absorption(&mut armor, actual_damage);
        send_armor_break_event(
            &armor_result,
            event.target,
            event.hit_position,
            &transform_query,
            &mut armor_break_events,
        );

        // 扣血
        let damage_dealt = health.take_damage(armor_result.damage_after_armor, current_time);

        // 觸發受傷反應
        trigger_hit_reaction(
            &mut hit_reaction,
            damage_dealt,
            event.attacker,
            event.target,
            event.is_headshot,
            event.force_knockback,
            &transform_query,
        );

        // 玩家受傷通知（含方向指示器）
        handle_player_damage_notification(
            event.target,
            damage_dealt,
            event.attacker,
            &player_query,
            &transform_query,
            &mut res.notifications,
            &mut res.damage_indicator,
        );

        // 玩家攻擊敵人的效果
        handle_player_hit_enemy(
            event,
            damage_dealt,
            player_entity,
            &enemy_query,
            &transform_query,
            &mut res,
            &mut commands,
        );

        // 檢查死亡
        if health.is_dead() {
            let hit_direction =
                calculate_death_hit_direction(event.attacker, event.target, &transform_query);
            death_events.write(DeathEvent {
                entity: event.target,
                killer: event.attacker,
                cause: event.source,
                hit_position: event.hit_position,
                hit_direction,
            });
        }
    }
}
