//! 傷害系統
//!
//! 處理傷害計算、死亡邏輯等。
#![allow(dead_code)]


use bevy::ecs::system::SystemParam;
use bevy::math::EulerRot;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use super::components::*;
use super::health::*;
use super::killcam::{KillCamState, KillCamTrigger};
use super::ragdoll::{convert_to_skeletal_ragdoll, BodyPart};
use super::visuals::*;
use crate::ai::{AiBehavior, AiCombat, AiMovement, AiPerception, CoverPoint, CoverSeeker};
use crate::audio::{play_hit_sound, AudioManager, WeaponSounds};
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::ui::{
    trigger_damage_indicator, ChineseFont, DamageIndicatorState, FloatingDamageNumber,
    FloatingDamageTracker, NotificationQueue,
};
use crate::economy::CashPickup;
use crate::wanted::{CrimeEvent, PoliceOfficer};

/// 傷害系統資源參數包（解決 Bevy 16 參數限制）
#[derive(SystemParam)]
pub struct DamageSystemResources<'w> {
    pub weapon_sounds: Option<Res<'w, WeaponSounds>>,
    pub audio_manager: Res<'w, AudioManager>,
    pub time: Res<'w, Time>,
    pub notifications: ResMut<'w, NotificationQueue>,
    pub combat_state: ResMut<'w, CombatState>,
    pub damage_indicator: ResMut<'w, DamageIndicatorState>,
    pub damage_tracker: ResMut<'w, FloatingDamageTracker>,
    pub font: Option<Res<'w, ChineseFont>>,
}

/// 死亡處理系統資源參數包
#[derive(SystemParam)]
pub struct DeathSystemResources<'w> {
    pub notifications: ResMut<'w, NotificationQueue>,
    pub respawn_state: ResMut<'w, RespawnState>,
    pub ragdoll_tracker: ResMut<'w, RagdollTracker>,
    pub killcam: ResMut<'w, KillCamState>,
    pub blood_visuals: Option<Res<'w, BloodVisuals>>,
    pub time: Res<'w, Time>,
}

/// 死亡處理系統查詢參數包
#[derive(SystemParam)]
pub struct DeathSystemQueries<'w, 's> {
    pub player: Query<'w, 's, (Entity, &'static Transform), With<Player>>,
    pub enemies: Query<'w, 's, (Entity, &'static Transform), (With<Enemy>, Without<Ragdoll>)>,
    pub all_enemies: Query<'w, 's, Entity, (With<Enemy>, Without<Ragdoll>)>,
    pub pedestrians: Query<
        'w,
        's,
        (Entity, &'static Transform, &'static Children),
        (
            With<Pedestrian>,
            Without<Player>,
            Without<Enemy>,
        ),
    >,
    pub police: Query<
        'w,
        's,
        &'static Transform,
        (With<PoliceOfficer>, Without<Player>, Without<Enemy>),
    >,
    pub body_parts: Query<
        'w,
        's,
        (
            &'static BodyPart,
            &'static Transform,
            &'static Mesh3d,
            &'static MeshMaterial3d<StandardMaterial>,
        ),
    >,
}

/// 玩家重生狀態
#[derive(Resource, Default)]
pub struct RespawnState {
    pub is_dead: bool,
    pub respawn_timer: f32,
    pub death_position: Vec3,
}

/// 重生位置（西門町漢中街起點）
pub const RESPAWN_POSITION: Vec3 = Vec3::new(5.0, 0.7, -5.0);

// ============================================================================
// 傷害系統常數
// ============================================================================
/// 命中標記顯示時長（秒）
const HIT_MARKER_DURATION: f32 = 0.2;
/// 浮動傷害數字頭頂偏移
const FLOATING_DAMAGE_HEAD_OFFSET: f32 = 1.8;
/// 預設受傷位置 Y 偏移
const DEFAULT_HIT_POSITION_Y_OFFSET: f32 = 1.2;
/// 爆頭高度閾值（相對於敵人位置，約肩膀以上）
const HEADSHOT_HEIGHT_THRESHOLD: f32 = 0.85;
/// 胸口高度（衝量應用點）
const CHEST_HEIGHT: f32 = 1.0;
/// 重生計時器時長（秒）
const RESPAWN_TIMER_DURATION: f32 = 3.0;

// ============================================================================
// 敵人掉落常數
// ============================================================================
/// 敵人死亡掉落金額最小值
const ENEMY_DROP_MIN: i32 = 50;
/// 敵人死亡掉落金額最大值
const ENEMY_DROP_MAX: i32 = 200;

// ============================================================================
// 衝量強度常數
// ============================================================================
/// 子彈衝量強度
const IMPULSE_BULLET: f32 = 350.0;
/// 爆炸衝量強度
const IMPULSE_EXPLOSION: f32 = 800.0;
/// 車輛撞擊衝量強度
const IMPULSE_VEHICLE: f32 = 1000.0;
/// 近戰衝量強度
const IMPULSE_MELEE: f32 = 200.0;
/// 墜落衝量強度
const IMPULSE_FALL: f32 = 100.0;
/// 火焰衝量強度
const IMPULSE_FIRE: f32 = 150.0;
/// 環境傷害衝量強度
const IMPULSE_ENVIRONMENT: f32 = 200.0;

// ============================================================================
// 傾斜強度常數
// ============================================================================
/// 子彈傾斜強度
const TILT_BULLET: f32 = 120.0;
/// 爆炸傾斜強度
const TILT_EXPLOSION: f32 = 200.0;
/// 車輛傾斜強度
const TILT_VEHICLE: f32 = 300.0;
/// 近戰傾斜強度
const TILT_MELEE: f32 = 80.0;
/// 預設傾斜強度
const TILT_DEFAULT: f32 = 100.0;

// ============================================================================
// 布娃娃物理常數
// ============================================================================
/// 布娃娃重力縮放
const RAGDOLL_GRAVITY_SCALE: f32 = 2.0;
/// 布娃娃向上推力因子
const RAGDOLL_UPWARD_PUSH_FACTOR: f32 = 0.3;
/// 布娃娃線性阻尼
const RAGDOLL_LINEAR_DAMPING: f32 = 0.3;
/// 布娃娃角阻尼
const RAGDOLL_ANGULAR_DAMPING: f32 = 0.8;
/// 布娃娃靜止速度閾值
const RAGDOLL_SETTLE_SPEED_THRESHOLD: f32 = 0.5;
/// 布娃娃靜止時間閾值
const RAGDOLL_SETTLE_TIME_THRESHOLD: f32 = 1.0;
/// 布娃娃淡出開始前的時間（距離最大生命週期）
const RAGDOLL_FADE_OFFSET: f32 = 1.5;
/// 布娃娃閃爍基礎速率
const RAGDOLL_BLINK_BASE_RATE: f32 = 4.0;
/// 布娃娃閃爍最大加速
const RAGDOLL_BLINK_ACCELERATION: f32 = 12.0;

// ============================================================================
// 血液粒子常數
// ============================================================================
/// 血液粒子最小速度
const BLOOD_PARTICLE_MIN_SPEED: f32 = 4.0;
/// 血液粒子最大速度
const BLOOD_PARTICLE_MAX_SPEED: f32 = 10.0;
/// 血液粒子最小生命週期
const BLOOD_PARTICLE_MIN_LIFETIME: f32 = 0.5;
/// 血液粒子最大生命週期
const BLOOD_PARTICLE_MAX_LIFETIME: f32 = 1.0;
/// 血液粒子最小縮放
const BLOOD_PARTICLE_MIN_SCALE: f32 = 0.03;
/// 血液粒子最大縮放
const BLOOD_PARTICLE_MAX_SCALE: f32 = 0.06;
/// 粒子觸地高度閾值
const PARTICLE_GROUND_HEIGHT: f32 = 0.05;
/// 粒子觸地後生命週期加速倍率
const PARTICLE_GROUND_LIFETIME_ACCEL: f32 = 3.0;
/// 粒子最小縮放比例
const PARTICLE_MIN_SCALE_RATIO: f32 = 0.3;
/// 粒子基礎縮放
const PARTICLE_BASE_SCALE: f32 = 0.05;

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
    transform_query: &Query<&Transform>,
) {
    let Some(ref mut reaction) = hit_reaction else {
        return;
    };
    let hit_direction = calculate_hit_direction(attacker, target, transform_query);
    reaction.trigger(damage_dealt, hit_direction, is_headshot);
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
        return Some(hit_pos + Vec3::Y * 0.3);
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

// ============================================================================
// 死亡系統輔助函數
// ============================================================================
/// 處理玩家死亡，返回 true 表示是玩家死亡事件（應跳過後續處理）
#[inline]
fn handle_player_death(
    entity: Entity,
    player_query: &Query<(Entity, &Transform), With<Player>>,
    respawn_state: &mut RespawnState,
    notifications: &mut NotificationQueue,
) -> bool {
    let Ok((_, transform)) = player_query.get(entity) else {
        return false;
    };

    if respawn_state.is_dead {
        return true;
    }

    notifications.error("💀 你死了！3 秒後重生...");
    respawn_state.is_dead = true;
    respawn_state.respawn_timer = RESPAWN_TIMER_DURATION;
    respawn_state.death_position = transform.translation;
    true
}

/// 處理警察死亡犯罪事件
#[inline]
fn handle_police_death_crime(
    entity: Entity,
    killer: Option<Entity>,
    player_entity: Option<Entity>,
    police_query: &Query<&Transform, (With<PoliceOfficer>, Without<Player>, Without<Enemy>)>,
    crime_events: &mut MessageWriter<CrimeEvent>,
    notifications: &mut NotificationQueue,
) {
    let Ok(police_transform) = police_query.get(entity) else {
        return;
    };
    if killer != player_entity {
        return;
    }

    crime_events.write(CrimeEvent::PoliceKilled {
        victim: entity,
        position: police_transform.translation,
    });
    notifications.warning("⚠️ 擊殺警察！通緝等級大幅上升！");
}

/// 處理行人死亡犯罪事件
#[inline]
fn handle_pedestrian_death_crime(
    entity: Entity,
    killer: Option<Entity>,
    player_entity: Option<Entity>,
    pedestrian_query: &Query<
        (Entity, &Transform, &Children),
        (With<Pedestrian>, Without<Player>, Without<Enemy>),
    >,
    crime_events: &mut MessageWriter<CrimeEvent>,
) {
    let Ok((_, ped_transform, _)) = pedestrian_query.get(entity) else {
        return;
    };
    if killer != player_entity {
        return;
    }

    crime_events.write(CrimeEvent::Murder {
        victim: entity,
        position: ped_transform.translation,
    });
}

/// 判斷 Kill Cam 觸發類型
#[inline]
fn determine_killcam_trigger(
    event: &DeathEvent,
    enemy_pos: Vec3,
    remaining_enemies: usize,
    killcam: &KillCamState,
) -> Option<KillCamTrigger> {
    if event.cause != DamageSource::Bullet {
        return None;
    }

    // 檢查擊殺條件（優先順序：爆頭 > 最後敵人 > 連殺）
    let is_headshot = event
        .hit_position
        .map(|p| p.y > enemy_pos.y + HEADSHOT_HEIGHT_THRESHOLD)
        .unwrap_or(false);

    if is_headshot {
        Some(KillCamTrigger::Headshot)
    } else if remaining_enemies == 0 {
        Some(KillCamTrigger::LastEnemy)
    } else if killcam.should_trigger_multi_kill() {
        Some(KillCamTrigger::MultiKill(killcam.get_kill_streak()))
    } else {
        None
    }
}

/// 處理 Kill Cam 邏輯
#[inline]
fn handle_killcam(
    event: &DeathEvent,
    enemy_pos: Vec3,
    player_entity: Option<Entity>,
    all_enemies_query: &Query<Entity, (With<Enemy>, Without<Ragdoll>)>,
    killcam: &mut KillCamState,
    current_time: f32,
) {
    if event.killer != player_entity {
        return;
    }

    killcam.record_kill(current_time);

    let remaining_enemies = all_enemies_query.iter().count().saturating_sub(1);
    let trigger_type = determine_killcam_trigger(event, enemy_pos, remaining_enemies, killcam);

    if let Some(trigger) = trigger_type {
        killcam.trigger(trigger, event.entity, enemy_pos, current_time);
    }
}

/// 取得傷害來源的衝量強度
#[inline]
fn get_impulse_strength(cause: DamageSource) -> f32 {
    match cause {
        DamageSource::Bullet => IMPULSE_BULLET,
        DamageSource::Explosion => IMPULSE_EXPLOSION,
        DamageSource::Vehicle => IMPULSE_VEHICLE,
        DamageSource::Melee => IMPULSE_MELEE,
        DamageSource::Fall => IMPULSE_FALL,
        DamageSource::Fire => IMPULSE_FIRE,
        DamageSource::Environment => IMPULSE_ENVIRONMENT,
    }
}

/// 取得傷害來源的傾斜強度
#[inline]
fn get_tilt_strength(cause: DamageSource) -> f32 {
    match cause {
        DamageSource::Bullet => TILT_BULLET,
        DamageSource::Explosion => TILT_EXPLOSION,
        DamageSource::Vehicle => TILT_VEHICLE,
        DamageSource::Melee => TILT_MELEE,
        _ => TILT_DEFAULT,
    }
}

/// 管理屍體數量限制
#[inline]
fn manage_ragdoll_limit(
    commands: &mut Commands,
    ragdoll_tracker: &mut RagdollTracker,
    new_entity: Entity,
    current_time: f32,
) {
    // max_count = 0 表示不保留任何屍體
    if ragdoll_tracker.max_count == 0 {
        return;
    }
    // 超過限制時移除最舊的屍體 - O(1) 操作
    while ragdoll_tracker.ragdolls.len() >= ragdoll_tracker.max_count {
        if let Some((oldest_entity, _)) = ragdoll_tracker.ragdolls.front().copied() {
            if let Ok(mut entity_commands) = commands.get_entity(oldest_entity) {
                entity_commands.despawn();
            }
            ragdoll_tracker.ragdolls.pop_front();
        } else {
            break;
        }
    }
    ragdoll_tracker.ragdolls.push_back((new_entity, current_time));
}

/// 設置布娃娃物理組件
fn setup_ragdoll_physics(
    commands: &mut Commands,
    entity: Entity,
    impulse_dir: Vec3,
    impulse_strength: f32,
    tilt_strength: f32,
) {
    let tilt_axis = Vec3::new(impulse_dir.z, 0.0, -impulse_dir.x).normalize_or_zero();

    let Ok(mut entity_commands) = commands.get_entity(entity) else {
        return;
    };

    entity_commands
        .insert(Ragdoll::with_impulse(impulse_dir, impulse_strength))
        .remove::<KinematicCharacterController>()
        .remove::<AiBehavior>()
        .remove::<AiMovement>()
        .remove::<AiPerception>()
        .remove::<AiCombat>()
        .insert(RigidBody::Dynamic)
        .insert(GravityScale(RAGDOLL_GRAVITY_SCALE))
        .insert(Velocity::default())
        .insert(ExternalImpulse {
            impulse: Vec3::new(
                impulse_dir.x * impulse_strength,
                impulse_strength * RAGDOLL_UPWARD_PUSH_FACTOR,
                impulse_dir.z * impulse_strength,
            ),
            torque_impulse: tilt_axis * tilt_strength,
        })
        .insert(Damping {
            linear_damping: RAGDOLL_LINEAR_DAMPING,
            angular_damping: RAGDOLL_ANGULAR_DAMPING,
        })
        .insert(CollisionGroups::new(Group::GROUP_10, Group::GROUP_1));
}

/// 處理敵人死亡效果
fn handle_enemy_death(
    commands: &mut Commands,
    event: &DeathEvent,
    enemy_transform: &Transform,
    player_entity: Option<Entity>,
    all_enemies_query: &Query<Entity, (With<Enemy>, Without<Ragdoll>)>,
    killcam: &mut KillCamState,
    ragdoll_tracker: &mut RagdollTracker,
    blood_visuals: &Option<Res<BloodVisuals>>,
    current_time: f32,
) {
    let enemy_pos = enemy_transform.translation;

    // Kill Cam 邏輯
    handle_killcam(
        event,
        enemy_pos,
        player_entity,
        all_enemies_query,
        killcam,
        current_time,
    );

    // 計算物理參數
    let impulse_dir = event
        .hit_direction
        .unwrap_or(Vec3::new(0.0, 0.2, -1.0).normalize());
    let impulse_strength = get_impulse_strength(event.cause);
    let tilt_strength = get_tilt_strength(event.cause);

    // 屍體數量管理
    manage_ragdoll_limit(commands, ragdoll_tracker, event.entity, current_time);

    // 生成血液粒子
    let blood_pos = enemy_pos + Vec3::Y * CHEST_HEIGHT;
    if let Some(ref blood) = blood_visuals {
        spawn_blood_particles(commands, blood_pos, impulse_dir, blood);
    }

    // 掉落金錢（隨機金額）
    let mut rng = rand::rng();
    let drop_amount = rng.random_range(ENEMY_DROP_MIN..=ENEMY_DROP_MAX);
    commands.spawn((
        Name::new("CashDrop"),
        Transform::from_translation(enemy_pos + Vec3::Y * 0.5),
        GlobalTransform::default(),
        CashPickup::new(drop_amount),
    ));

    // 設置布娃娃物理
    setup_ragdoll_physics(
        commands,
        event.entity,
        impulse_dir,
        impulse_strength,
        tilt_strength,
    );
}

/// 死亡處理系統
pub fn death_system(
    mut death_events: MessageReader<DeathEvent>,
    mut commands: Commands,
    queries: DeathSystemQueries,
    mut res: DeathSystemResources,
    mut crime_events: MessageWriter<CrimeEvent>,
) {
    let current_time = res.time.elapsed_secs();
    let player_entity = queries.player.single().ok().map(|(e, _)| e);

    for event in death_events.read() {
        // 玩家死亡
        if handle_player_death(
            event.entity,
            &queries.player,
            &mut res.respawn_state,
            &mut res.notifications,
        ) {
            continue;
        }

        // 犯罪事件
        handle_police_death_crime(
            event.entity,
            event.killer,
            player_entity,
            &queries.police,
            &mut crime_events,
            &mut res.notifications,
        );

        // 計算衝擊方向和強度
        let impulse_dir = event
            .hit_direction
            .unwrap_or(Vec3::new(0.0, 0.2, -1.0).normalize());
        let impulse_strength = get_impulse_strength(event.cause);

        // 行人死亡 - 骨骼布娃娃效果
        if let Ok((ped_entity, ped_transform, children)) = queries.pedestrians.get(event.entity) {
            // 生成血液粒子
            let blood_pos = ped_transform.translation + Vec3::Y * CHEST_HEIGHT;
            if let Some(ref blood) = res.blood_visuals {
                spawn_blood_particles(&mut commands, blood_pos, impulse_dir, blood);
            }

            // 轉換為骨骼布娃娃並追蹤數量
            if let Some(ragdoll_entity) = convert_to_skeletal_ragdoll(
                &mut commands,
                ped_entity,
                ped_transform,
                children,
                &queries.body_parts,
                impulse_dir,
                impulse_strength,
            ) {
                manage_ragdoll_limit(
                    &mut commands,
                    &mut res.ragdoll_tracker,
                    ragdoll_entity,
                    current_time,
                );
            }
            continue;
        }

        // 敵人死亡 - 傳統布娃娃效果（保持向後兼容）
        if let Ok((_, enemy_transform)) = queries.enemies.get(event.entity) {
            res.notifications.success("擊殺敵人！");
            handle_enemy_death(
                &mut commands,
                event,
                enemy_transform,
                player_entity,
                &queries.all_enemies,
                &mut res.killcam,
                &mut res.ragdoll_tracker,
                &res.blood_visuals,
                current_time,
            );
        }
    }
}

/// 生成血液粒子
fn spawn_blood_particles(
    commands: &mut Commands,
    position: Vec3,
    direction: Vec3,
    blood_visuals: &BloodVisuals,
) {
    let mut rng = rand::rng();
    let particle_count = 12;

    for _ in 0..particle_count {
        // 隨機散射方向
        let spread = Vec3::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(0.2..0.8),
            rng.random_range(-1.0..1.0),
        );
        // 粒子速度：沿衝擊方向 + 散射
        let velocity = (direction + spread).normalize()
            * rng.random_range(BLOOD_PARTICLE_MIN_SPEED..BLOOD_PARTICLE_MAX_SPEED);
        let max_lifetime =
            rng.random_range(BLOOD_PARTICLE_MIN_LIFETIME..BLOOD_PARTICLE_MAX_LIFETIME);
        let scale = rng.random_range(BLOOD_PARTICLE_MIN_SCALE..BLOOD_PARTICLE_MAX_SCALE);

        commands.spawn((
            Mesh3d(blood_visuals.particle_mesh.clone()),
            MeshMaterial3d(blood_visuals.particle_material.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(scale)),
            BloodParticle::new(velocity, max_lifetime),
        ));
    }
}

/// 玩家重生系統
pub fn player_respawn_system(
    time: Res<Time>,
    mut respawn_state: ResMut<RespawnState>,
    mut player_query: Query<(&mut Transform, &mut Health, Option<&mut Armor>), With<Player>>,
    mut notifications: ResMut<NotificationQueue>,
) {
    if !respawn_state.is_dead {
        return;
    }

    // 更新重生計時器
    respawn_state.respawn_timer -= time.delta_secs();

    if respawn_state.respawn_timer <= 0.0 {
        // 重生玩家
        for (mut transform, mut health, armor) in player_query.iter_mut() {
            // 重置位置
            transform.translation = RESPAWN_POSITION;

            // 重置生命值
            health.current = health.max;

            // 重置護甲
            if let Some(mut armor) = armor {
                armor.current = 0.0;
            }
        }

        respawn_state.is_dead = false;
        notifications.success("🔄 你已重生！");
    }
}

/// 生命回復系統（可選，給有回復能力的實體使用）
pub fn health_regeneration_system(time: Res<Time>, mut query: Query<&mut Health>) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    for mut health in query.iter_mut() {
        if health.regeneration <= 0.0 || health.is_dead() || health.is_full() {
            continue;
        }

        // 檢查是否過了回復延遲
        let time_since_damage = current_time - health.last_damage_time;
        if time_since_damage < health.regen_delay {
            continue;
        }

        // 回復生命（先讀取再修改，避免借用衝突）
        let regen_amount = health.regeneration * dt;
        health.heal(regen_amount);
    }
}

/// 布娃娃更新系統
/// 處理布娃娃物理狀態更新和延遲移除
pub fn ragdoll_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut ragdoll_query: Query<(Entity, &mut Ragdoll, &mut Transform, Option<&Velocity>)>,
) {
    let dt = time.delta_secs();

    for (entity, mut ragdoll, mut transform, velocity) in ragdoll_query.iter_mut() {
        // 更新計時器
        ragdoll.lifetime += dt;

        // 確保布娃娃不會穿過地面
        if transform.translation.y < 0.0 {
            transform.translation.y = 0.0;
        }

        // 檢查是否需要停止（速度很低時）
        if let Some(vel) = velocity {
            let speed = vel.linvel.length();
            // 如果速度很低且已經過了一段時間，開始淡出
            if speed < RAGDOLL_SETTLE_SPEED_THRESHOLD
                && ragdoll.lifetime > RAGDOLL_SETTLE_TIME_THRESHOLD
            {
                // 加速計時器
                ragdoll.lifetime += dt * 2.0;
            }
        }

        // 檢查是否超時
        if ragdoll.lifetime >= ragdoll.max_lifetime {
            // 移除布娃娃實體
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
        }
    }
}

// ============================================================================
// 布娃娃視覺輔助函數
// ============================================================================
/// 計算布娃娃閃爍可見性
#[inline]
fn calculate_ragdoll_blink_visibility(ragdoll: &Ragdoll) -> Option<bool> {
    let fade_start = ragdoll.max_lifetime - RAGDOLL_FADE_OFFSET;
    if ragdoll.lifetime <= fade_start {
        return None;
    }

    let fade_progress = (ragdoll.lifetime - fade_start) / RAGDOLL_FADE_OFFSET;
    let blink_rate = RAGDOLL_BLINK_BASE_RATE + fade_progress * RAGDOLL_BLINK_ACCELERATION;
    Some((ragdoll.lifetime * blink_rate).sin() > 0.0)
}

/// 設置子實體可見性
#[inline]
fn set_children_visibility(
    children: &Children,
    visible: bool,
    material_query: &mut Query<&mut Visibility>,
) {
    let visibility_value = if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for child in children.iter() {
        if let Ok(mut visibility) = material_query.get_mut(child) {
            *visibility = visibility_value;
        }
    }
}

/// 布娃娃視覺效果系統
/// 處理布娃娃的視覺淡出效果
pub fn ragdoll_visual_system(
    ragdoll_query: Query<(&Ragdoll, &Children)>,
    mut material_query: Query<&mut Visibility>,
) {
    for (ragdoll, children) in ragdoll_query.iter() {
        let Some(visible) = calculate_ragdoll_blink_visibility(ragdoll) else {
            continue;
        };
        set_children_visibility(children, visible, &mut material_query);
    }
}

/// 血液粒子更新系統
/// 處理血液粒子的物理移動和生命週期
pub fn blood_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_query: Query<(Entity, &mut BloodParticle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    const GRAVITY: f32 = 15.0;

    for (entity, mut particle, mut transform) in particle_query.iter_mut() {
        // 更新生命時間
        particle.lifetime += dt;

        // 檢查是否過期
        if particle.lifetime >= particle.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 應用重力
        particle.velocity.y -= GRAVITY * dt;

        // 更新位置
        transform.translation += particle.velocity * dt;

        // 如果碰到地面，停止移動
        if transform.translation.y < PARTICLE_GROUND_HEIGHT {
            transform.translation.y = PARTICLE_GROUND_HEIGHT;
            particle.velocity = Vec3::ZERO;
            // 加速消失
            particle.lifetime += dt * PARTICLE_GROUND_LIFETIME_ACCEL;
        }

        // 根據生命週期縮小粒子
        let life_ratio = 1.0 - (particle.lifetime / particle.max_lifetime);
        let scale = life_ratio.max(PARTICLE_MIN_SCALE_RATIO);
        transform.scale = Vec3::splat(scale * PARTICLE_BASE_SCALE);
    }
}

// ============================================================================
// 浮動傷害數字系統
// ============================================================================

/// 傷害數字顏色常數
const DAMAGE_NUMBER_COLOR: Color = Color::WHITE;
const HEADSHOT_NUMBER_COLOR: Color = Color::srgb(1.0, 0.9, 0.0); // 金黃色
const CRITICAL_NUMBER_COLOR: Color = Color::srgb(1.0, 0.3, 0.1); // 橙紅色

/// 生成浮動傷害數字實體
fn spawn_floating_damage_number(
    commands: &mut Commands,
    damage: FloatingDamageNumber,
    font: &ChineseFont,
) {
    // 決定顏色
    let color = if damage.is_headshot {
        HEADSHOT_NUMBER_COLOR
    } else if damage.damage >= 50.0 {
        CRITICAL_NUMBER_COLOR // 高傷害用橙紅色
    } else {
        DAMAGE_NUMBER_COLOR
    };

    // 格式化傷害數字
    let text = if damage.is_headshot {
        format!("💀 {:.0}", damage.damage) // 爆頭加骷髏
    } else {
        format!("{:.0}", damage.damage)
    };

    // 計算初始位置（加上水平偏移）
    let position = damage.start_position + Vec3::new(damage.horizontal_offset, 0.0, 0.0);

    // 創建 Billboard 文字（世界空間，始終面向攝影機）
    commands.spawn((
        // Billboard 文字
        Text2d::new(text),
        TextFont {
            font: font.font.clone(),
            font_size: 24.0 * damage.initial_scale,
            ..default()
        },
        TextColor(color),
        // 世界空間 Transform
        Transform::from_translation(position).with_scale(Vec3::splat(0.02)), // 縮小到世界空間大小
        GlobalTransform::default(),
        // 浮動傷害組件
        damage,
        // Billboard 行為標記
        DamageNumberBillboard,
    ));
}

/// Billboard 標記（讓文字始終面向攝影機）
#[derive(Component)]
pub struct DamageNumberBillboard;

// ============================================================================
// 浮動傷害數字輔助函數
// ============================================================================
/// 計算 Billboard 旋轉
#[inline]
fn calculate_billboard_rotation(transform_pos: Vec3, camera_pos: Option<Vec3>) -> Option<Quat> {
    let cam_pos = camera_pos?;
    let direction = cam_pos - transform_pos;

    if direction.length_squared() <= 0.001 {
        return None;
    }

    Some(Quat::from_rotation_arc(Vec3::NEG_Z, direction.normalize()))
}

/// 取得傷害數字的基礎顏色
#[inline]
fn get_damage_number_color(is_headshot: bool, damage: f32) -> Color {
    if is_headshot {
        HEADSHOT_NUMBER_COLOR
    } else if damage >= 50.0 {
        CRITICAL_NUMBER_COLOR
    } else {
        DAMAGE_NUMBER_COLOR
    }
}

/// 浮動傷害數字更新系統
/// 處理上浮動畫、縮放變化和淡出效果
pub fn floating_damage_number_update_system(
    mut commands: Commands,
    time: Res<Time>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut damage_query: Query<
        (
            Entity,
            &mut FloatingDamageNumber,
            &mut Transform,
            &mut TextColor,
        ),
        Without<Camera3d>,
    >,
    mut damage_tracker: ResMut<FloatingDamageTracker>,
) {
    let dt = time.delta_secs();
    let camera_pos = camera_query.single().map(|t| t.translation).ok();

    for (entity, mut damage, mut transform, mut text_color) in damage_query.iter_mut() {
        damage.lifetime += dt;

        // 檢查是否過期
        if damage.lifetime >= damage.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                damage_tracker.active_count = damage_tracker.active_count.saturating_sub(1);
            }
            continue;
        }

        // 更新位置
        let y_offset = damage.y_offset();
        transform.translation =
            damage.start_position + Vec3::new(damage.horizontal_offset, y_offset, 0.0);

        // Billboard 效果
        if let Some(rotation) = calculate_billboard_rotation(transform.translation, camera_pos) {
            transform.rotation = rotation;
        }

        // 更新縮放
        transform.scale = Vec3::splat(damage.scale() * 0.02);

        // 更新顏色和透明度
        let base_color = get_damage_number_color(damage.is_headshot, damage.damage);
        text_color.0 = base_color.with_alpha(damage.alpha());
    }
}

// ============================================================================
// 受傷反應系統
// ============================================================================

/// 受傷反應更新系統
/// 每幀更新所有 HitReaction 組件的狀態
pub fn hit_reaction_update_system(time: Res<Time>, mut query: Query<&mut HitReaction>) {
    let delta = time.delta_secs();

    for mut reaction in query.iter_mut() {
        reaction.update(delta);
    }
}

/// 受傷反應視覺效果系統
/// 將 HitReaction 的視覺旋轉應用到實體的子視覺組件上
pub fn hit_reaction_visual_system(
    reaction_query: Query<(&HitReaction, &Children), Changed<HitReaction>>,
    mut transform_query: Query<&mut Transform, Without<HitReaction>>,
) {
    for (reaction, children) in reaction_query.iter() {
        if reaction.phase == HitReactionPhase::None {
            continue;
        }

        // 將視覺旋轉應用到第一個子實體（通常是模型）
        for child in children.iter() {
            if let Ok(mut transform) = transform_query.get_mut(child) {
                // 只修改 X 軸旋轉（後仰效果），保持其他旋轉
                let current_euler = transform.rotation.to_euler(EulerRot::XYZ);
                let target_euler = reaction.visual_rotation.to_euler(EulerRot::XYZ);
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    target_euler.0,  // 使用反應的 X 旋轉
                    current_euler.1, // 保持 Y 旋轉
                    current_euler.2, // 保持 Z 旋轉
                );
                break; // 只處理第一個子實體
            }
        }
    }
}

/// 受傷反應擊退系統
/// 將擊退速度應用到角色控制器
pub fn hit_reaction_knockback_system(
    time: Res<Time>,
    mut query: Query<(&HitReaction, &mut KinematicCharacterController)>,
) {
    let delta = time.delta_secs();

    for (reaction, mut controller) in query.iter_mut() {
        let knockback = reaction.get_knockback_velocity();
        if knockback.length_squared() > 0.001 {
            // 將擊退速度加到控制器的位移上
            let current_translation = controller.translation.unwrap_or(Vec3::ZERO);
            controller.translation = Some(current_translation + knockback * delta);
        }
    }
}

/// 應用擊退效果到 Transform（共用邏輯）
#[inline]
fn apply_knockback_to_transform(reaction: &HitReaction, transform: &mut Transform, delta: f32) {
    let knockback = reaction.get_knockback_velocity();
    if knockback.length_squared() <= 0.001 {
        return;
    }

    transform.translation += knockback * delta;
    // 確保不會掉到地面以下
    if transform.translation.y < 0.0 {
        transform.translation.y = 0.0;
    }
}

/// 敵人受傷反應擊退系統
pub fn enemy_hit_reaction_knockback_system(
    time: Res<Time>,
    mut query: Query<(&HitReaction, &mut Transform), (With<Enemy>, Without<Ragdoll>)>,
) {
    let delta = time.delta_secs();
    for (reaction, mut transform) in query.iter_mut() {
        apply_knockback_to_transform(reaction, &mut transform, delta);
    }
}

/// 行人受傷反應擊退系統
pub fn pedestrian_hit_reaction_knockback_system(
    time: Res<Time>,
    mut query: Query<(&HitReaction, &mut Transform), (With<Pedestrian>, Without<Ragdoll>)>,
) {
    let delta = time.delta_secs();
    for (reaction, mut transform) in query.iter_mut() {
        apply_knockback_to_transform(reaction, &mut transform, delta);
    }
}

// ============================================================================
// 護甲特效系統
// ============================================================================

/// 護甲破碎特效生成系統
pub fn armor_break_effect_system(
    mut commands: Commands,
    mut armor_events: MessageReader<ArmorBreakEvent>,
    visuals: Option<Res<ArmorEffectVisuals>>,
) {
    let Some(visuals) = visuals else { return };

    for event in armor_events.read() {
        let position = event.position;

        // 生成火花（每次護甲受擊都生成）
        spawn_armor_sparks(&mut commands, position, &visuals, 6);

        // 如果是完全破碎，生成更多碎片
        if event.is_full_break {
            spawn_armor_shards(&mut commands, position, &visuals, 8);
            spawn_armor_sparks(&mut commands, position, &visuals, 12);
        }
    }
}

/// 生成護甲火花
fn spawn_armor_sparks(
    commands: &mut Commands,
    position: Vec3,
    visuals: &ArmorEffectVisuals,
    count: u32,
) {
    let mut rng = rand::rng();

    for _ in 0..count {
        // 隨機散射方向
        let spread = Vec3::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(0.3..1.0),
            rng.random_range(-1.0..1.0),
        );
        let velocity = spread.normalize() * rng.random_range(3.0..8.0);
        let max_lifetime = rng.random_range(0.15..0.35);

        commands.spawn((
            Mesh3d(visuals.spark_mesh.clone()),
            MeshMaterial3d(visuals.spark_material.clone()),
            Transform::from_translation(position),
            ArmorSparkParticle::new(velocity, max_lifetime),
        ));
    }
}

/// 生成護甲碎片
fn spawn_armor_shards(
    commands: &mut Commands,
    position: Vec3,
    visuals: &ArmorEffectVisuals,
    count: u32,
) {
    let mut rng = rand::rng();

    for _ in 0..count {
        // 隨機散射方向
        let spread = Vec3::new(
            rng.random_range(-1.0..1.0),
            rng.random_range(0.2..0.8),
            rng.random_range(-1.0..1.0),
        );
        let velocity = spread.normalize() * rng.random_range(2.0..5.0);
        let angular_velocity = Vec3::new(
            rng.random_range(-10.0..10.0),
            rng.random_range(-10.0..10.0),
            rng.random_range(-10.0..10.0),
        );
        let max_lifetime = rng.random_range(0.8..1.5);
        let scale = rng.random_range(0.5..1.5);

        commands.spawn((
            Mesh3d(visuals.shard_mesh.clone()),
            MeshMaterial3d(visuals.shard_material.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(scale)),
            ArmorShardParticle::new(velocity, angular_velocity, max_lifetime),
        ));
    }
}

/// 護甲火花更新系統
pub fn armor_spark_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut spark_query: Query<(Entity, &mut ArmorSparkParticle, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (entity, mut spark, mut transform) in spark_query.iter_mut() {
        spark.lifetime += dt;

        if spark.lifetime >= spark.max_lifetime {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
            continue;
        }

        // 更新位置（快速衰減）
        spark.velocity *= 0.9;
        transform.translation += spark.velocity * dt;

        // 縮小並淡出
        let life_ratio = 1.0 - (spark.lifetime / spark.max_lifetime);
        transform.scale = Vec3::splat(life_ratio.max(0.1));
    }
}

/// 護甲碎片更新系統
pub fn armor_shard_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut shard_query: Query<(Entity, &mut ArmorShardParticle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    const GRAVITY: f32 = 12.0;

    for (entity, mut shard, mut transform) in shard_query.iter_mut() {
        shard.lifetime += dt;

        if shard.lifetime >= shard.max_lifetime {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
            continue;
        }

        // 應用重力
        shard.velocity.y -= GRAVITY * dt;

        // 更新位置
        transform.translation += shard.velocity * dt;

        // 更新旋轉
        let rotation_delta = Quat::from_euler(
            EulerRot::XYZ,
            shard.angular_velocity.x * dt,
            shard.angular_velocity.y * dt,
            shard.angular_velocity.z * dt,
        );
        transform.rotation = rotation_delta * transform.rotation;

        // 碰到地面停止
        if transform.translation.y < 0.02 {
            transform.translation.y = 0.02;
            shard.velocity = Vec3::ZERO;
            shard.angular_velocity *= 0.5;
            // 加速消失
            shard.lifetime += dt * 2.0;
        }

        // 後期淡出（縮小）
        if shard.lifetime > shard.max_lifetime * 0.7 {
            let fade_progress =
                (shard.lifetime - shard.max_lifetime * 0.7) / (shard.max_lifetime * 0.3);
            let scale = (1.0 - fade_progress).max(0.1);
            transform.scale = Vec3::splat(scale);
        }
    }
}
