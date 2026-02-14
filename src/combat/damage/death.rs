//! 死亡與重生系統
//!
//! 處理死亡事件、玩家重生、布娃娃物理、生命回復。

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use super::effects::spawn_blood_particles;
use super::{DeathSystemQueries, DeathSystemResources, RespawnState, RESPAWN_POSITION};
use crate::combat::components::*;
use crate::combat::health::*;
use crate::combat::killcam::{KillCamState, KillCamTrigger};
use crate::combat::ragdoll::convert_to_skeletal_ragdoll;
use crate::combat::visuals::*;
use crate::ai::{AiBehavior, AiCombat, AiMovement, AiPerception};
use crate::economy::CashPickup;
use crate::player::Player;
use crate::ui::NotificationQueue;
use crate::wanted::{CrimeEvent, PoliceOfficer};

// ============================================================================
// 死亡系統常數
// ============================================================================
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
/// 布娃娃靜止後加速倍率
const RAGDOLL_SETTLE_ACCEL: f32 = 2.0;
/// 布娃娃淡出開始前的時間（距離最大生命週期）
const RAGDOLL_FADE_OFFSET: f32 = 1.5;
/// 布娃娃閃爍基礎速率
const RAGDOLL_BLINK_BASE_RATE: f32 = 4.0;
/// 布娃娃閃爍最大加速
const RAGDOLL_BLINK_ACCELERATION: f32 = 12.0;

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

// ============================================================================
// 重生與回復系統
// ============================================================================

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

// ============================================================================
// 布娃娃系統
// ============================================================================

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
                ragdoll.lifetime += dt * RAGDOLL_SETTLE_ACCEL;
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
