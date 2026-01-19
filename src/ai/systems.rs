//! AI 系統
//!
//! 處理敵人感知、決策、移動、攻擊行為。

// Bevy 系統需要 Res<T> 按值傳遞
#![allow(clippy::needless_pass_by_value)]
#![allow(dead_code)] // Phase 5+ 預留功能

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use crate::combat::{
    DamageEvent, DamageSource, DeathEvent, Enemy, EnemyType,
    Health, Weapon, Damageable, EnemyArm, EnemyPunchAnimation, PunchPhase,
    spawn_bullet_tracer, TracerStyle, Ragdoll, HitReaction,
};
use crate::core::{COLLISION_GROUP_CHARACTER, COLLISION_GROUP_VEHICLE, COLLISION_GROUP_STATIC};
use crate::player::Player;
use super::{
    AiBehavior, AiCombat, AiMovement, AiPerception, AiState,
    AiUpdateTimer, EnemySpawnTimer, PatrolPath, CoverSeeker, CoverPoint,
    SquadMember, SquadRole, SquadManager, calculate_flank_position,
};

// ============================================================================
// AI 系統常數
// ============================================================================

// === 感知相關 ===
/// AI 眼睛高度（發射視線的起點）
const AI_EYE_HEIGHT: f32 = 1.5;
/// 玩家身體中心高度（視線目標）
const PLAYER_BODY_HEIGHT: f32 = 1.0;
/// 視線遮擋容差（95% 距離內無遮擋視為可見）
const LINE_OF_SIGHT_TOLERANCE: f32 = 0.95;

// === 天氣影響視野 ===
/// 晴天視野乘數
const WEATHER_CLEAR_SIGHT: f32 = 1.0;
/// 陰天視野乘數
const WEATHER_CLOUDY_SIGHT: f32 = 0.95;
/// 雨天基礎視野乘數
const WEATHER_RAINY_SIGHT_BASE: f32 = 0.8;
/// 雨天強度衰減
const WEATHER_RAINY_SIGHT_DECAY: f32 = 0.2;
/// 霧天基礎視野乘數
const WEATHER_FOGGY_SIGHT_BASE: f32 = 0.5;
/// 霧天強度衰減
const WEATHER_FOGGY_SIGHT_DECAY: f32 = 0.2;

// === 行為閾值 ===
/// 逃跑時的移動距離
const FLEE_DISTANCE: f32 = 30.0;
/// 警戒距離（保持安全距離）
const ALERT_DISTANCE: f32 = 40.0;
/// 巡邏待機計時器閾值
const PATROL_IDLE_THRESHOLD: f32 = 3.0;
/// 警戒狀態超時（秒）
const ALERT_TIMEOUT: f32 = 5.0;
/// 失去目標超時（秒）
const LOSE_TARGET_TIMEOUT: f32 = 5.0;
/// 低血量閾值（觸發撤退）
const LOW_HEALTH_THRESHOLD: f32 = 0.7;

// === 射擊精度 ===
/// 最小射擊精度
const MIN_ACCURACY: f32 = 0.1;
/// 最大距離懲罰
const MAX_RANGE_PENALTY: f32 = 0.5;
/// 射擊散佈範圍 X
const MISS_SPREAD_X: f32 = 2.0;
/// 射擊散佈範圍 Y (上/下)
const MISS_SPREAD_Y_MIN: f32 = -1.0;
const MISS_SPREAD_Y_MAX: f32 = 1.5;
/// 射擊散佈範圍 Z
const MISS_SPREAD_Z: f32 = 2.0;

// === 生成相關 ===
/// 最小生成距離
const MIN_SPAWN_DISTANCE: f32 = 45.0;
/// 最小生成距離備用（防止過近）
const MIN_SPAWN_RADIUS_BUFFER: f32 = 5.0;

// === 槍口位置 ===
/// 槍口前方偏移
const MUZZLE_FORWARD_OFFSET: f32 = 0.5;
/// 槍口高度偏移
const MUZZLE_HEIGHT_OFFSET: f32 = 0.3;

// === 小隊角色分配閾值 ===
/// Gangster 衝鋒者機率
const GANGSTER_RUSHER_THRESHOLD: f32 = 0.5;
/// Gangster 側翼者機率（累積）
const GANGSTER_FLANKER_THRESHOLD: f32 = 0.9;
/// Thug 衝鋒者機率
const THUG_RUSHER_THRESHOLD: f32 = 0.7;

// === 距離平方常數 (效能優化：避免 sqrt) ===
/// 警戒距離平方 (40.0²)
const ALERT_DISTANCE_SQ: f32 = 1600.0;
/// 掩體到達距離平方 (1.5²)
const COVER_ARRIVAL_SQ: f32 = 2.25;
/// 包抄到達距離平方 (2.0²)
const FLANK_ARRIVAL_SQ: f32 = 4.0;

// ============================================================================
// 感知系統
// ============================================================================

/// AI 感知系統：檢測玩家位置
/// GTA 5 風格：60° FOV + 視線遮擋檢測 + 天氣影響
pub fn ai_perception_system(
    time: Res<Time>,
    mut timer: ResMut<AiUpdateTimer>,
    weather: Res<crate::core::WeatherState>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    mut enemy_query: Query<(
        Entity,
        &Transform,
        &mut AiPerception,
        &mut AiBehavior,
    ), With<Enemy>>,
    rapier_context: ReadRapierContext,
) {
    timer.perception_timer.tick(time.delta());
    if !timer.perception_timer.just_finished() {
        return;
    }

    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let current_time = time.elapsed_secs();

    // 取得物理世界
    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    // === GTA5 風格：天氣影響 AI 感知 ===
    let weather_sight_multiplier = match weather.weather_type {
        crate::core::WeatherType::Clear => WEATHER_CLEAR_SIGHT,
        crate::core::WeatherType::Cloudy => WEATHER_CLOUDY_SIGHT,
        crate::core::WeatherType::Rainy => WEATHER_RAINY_SIGHT_BASE - weather.intensity * WEATHER_RAINY_SIGHT_DECAY,
        crate::core::WeatherType::Foggy => WEATHER_FOGGY_SIGHT_BASE - weather.intensity * WEATHER_FOGGY_SIGHT_DECAY,
    };

    for (enemy_entity, transform, mut perception, mut behavior) in &mut enemy_query {
        let my_pos = transform.translation;
        let my_forward = transform.forward().as_vec3();

        // 重置感知狀態
        perception.can_see_target = false;

        // 1. 檢查距離（根據天氣調整感知範圍）- 使用 distance_squared 避免 sqrt
        let effective_sight_range = perception.sight_range * weather_sight_multiplier;
        let effective_sight_range_sq = effective_sight_range * effective_sight_range;
        if my_pos.distance_squared(player_pos) > effective_sight_range_sq {
            continue;
        }

        // 2. 檢查 FOV（60° 視野錐）
        if !perception.is_in_fov(my_pos, my_forward, player_pos) {
            // 不在視野內 - 玩家可以從背後偷襲
            continue;
        }

        // 3. 檢查視線遮擋（Raycast）
        let ray_origin = my_pos + Vec3::Y * AI_EYE_HEIGHT;
        let ray_target = player_pos + Vec3::Y * PLAYER_BODY_HEIGHT;
        let ray_dir = (ray_target - ray_origin).normalize_or_zero();
        let max_distance = ray_origin.distance(ray_target);

        // 設定碰撞過濾：排除自己，只檢測靜態物體和車輛
        let filter = QueryFilter::default()
            .exclude_rigid_body(enemy_entity)
            .groups(CollisionGroups::new(
                Group::ALL,
                COLLISION_GROUP_STATIC | COLLISION_GROUP_VEHICLE,
            ));

        // 執行射線檢測
        let has_line_of_sight = if let Some((_hit_entity, toi)) = rapier.cast_ray(
            ray_origin,
            ray_dir,
            max_distance,
            true,
            filter,
        ) {
            // 如果射線打到的距離小於玩家距離，表示有遮擋
            toi >= max_distance * LINE_OF_SIGHT_TOLERANCE
        } else {
            // 沒有打到任何東西，視線通暢
            true
        };

        if has_line_of_sight {
            perception.can_see_target = true;
            behavior.see_target(player_entity, player_pos, current_time);
        }
    }
}

// ============================================================================
// 決策系統
// ============================================================================

/// AI 決策系統：根據感知更新狀態
/// 每幀執行，確保即時響應
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_lines)]
pub fn ai_decision_system(
    time: Res<Time>,
    mut enemy_query: Query<(
        &Transform,
        &Health,
        &AiPerception,
        &AiCombat,
        &mut AiBehavior,
        &mut AiMovement,
        Option<&PatrolPath>,
    ), With<Enemy>>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    for (transform, health, perception, combat, mut behavior, mut movement, patrol) in &mut enemy_query {
        let my_pos = transform.translation;

        // 更新狀態計時器
        behavior.tick(dt);

        // 檢查是否應該逃跑
        if health.percentage() <= behavior.flee_threshold && !behavior.is_fleeing {
            behavior.is_fleeing = true;
            behavior.set_state(AiState::Flee, current_time);
            movement.is_running = true;
            if let Some(target_pos) = behavior.last_known_target_pos {
                let flee_dir = (my_pos - target_pos).normalize_or_zero();
                movement.move_target = Some(my_pos + flee_dir * FLEE_DISTANCE);
            }
            continue;
        }

        // 逃跑狀態持續
        if behavior.is_fleeing {
            if let Some(target_pos) = behavior.last_known_target_pos {
                let distance_from_threat_sq = my_pos.distance_squared(target_pos);

                // 逃離超過安全距離後停止逃跑，進入警戒狀態
                if distance_from_threat_sq > ALERT_DISTANCE_SQ {
                    behavior.is_fleeing = false;
                    behavior.set_state(AiState::Alert, current_time);
                    movement.is_running = false;
                    movement.move_target = None;
                } else {
                    // 繼續逃跑
                    let flee_dir = (my_pos - target_pos).normalize_or_zero();
                    movement.move_target = Some(my_pos + flee_dir * FLEE_DISTANCE);
                }
            } else {
                // 沒有目標位置，停止逃跑
                behavior.is_fleeing = false;
                behavior.set_state(AiState::Idle, current_time);
                movement.is_running = false;
                movement.move_target = None;
            }
            continue;
        }

        // 正常狀態機轉換
        match behavior.state {
            AiState::Idle => {
                if perception.can_see_target {
                    // 看到玩家 → 追逐，並立即設定移動目標
                    behavior.set_state(AiState::Chase, current_time);
                    movement.is_running = true;
                    movement.move_target = behavior.last_known_target_pos;
                } else if perception.heard_noise {
                    behavior.set_state(AiState::Alert, current_time);
                    movement.move_target = perception.noise_position;
                } else if patrol.is_some() && behavior.state_timer > PATROL_IDLE_THRESHOLD {
                    behavior.set_state(AiState::Patrol, current_time);
                }
            }

            AiState::Patrol => {
                if perception.can_see_target {
                    behavior.set_state(AiState::Chase, current_time);
                    movement.is_running = true;
                    movement.move_target = behavior.last_known_target_pos;
                } else if perception.heard_noise {
                    behavior.set_state(AiState::Alert, current_time);
                    movement.move_target = perception.noise_position;
                }
            }

            AiState::Alert => {
                if perception.can_see_target {
                    behavior.set_state(AiState::Chase, current_time);
                    movement.is_running = true;
                    movement.move_target = behavior.last_known_target_pos;
                } else if behavior.state_timer > ALERT_TIMEOUT {
                    if patrol.is_some() {
                        behavior.set_state(AiState::Patrol, current_time);
                    } else {
                        behavior.set_state(AiState::Idle, current_time);
                    }
                }
            }

            AiState::Chase => {
                // 持續更新移動目標
                if let Some(target_pos) = behavior.last_known_target_pos {
                    movement.move_target = Some(target_pos);

                    // 在攻擊範圍內且能看到 → 攻擊
                    if combat.is_in_range(my_pos, target_pos) && perception.can_see_target {
                        behavior.set_state(AiState::Attack, current_time);
                        movement.is_running = false;
                        movement.move_target = None; // 攻擊時停止移動
                    }
                }

                // 失去目標超過閾值時間
                if behavior.lose_target(current_time, LOSE_TARGET_TIMEOUT) {
                    behavior.set_state(AiState::Alert, current_time);
                    movement.is_running = false;
                    movement.move_target = behavior.last_known_target_pos;
                }
            }

            AiState::Attack => {
                // 攻擊時面向目標但不移動
                if let Some(target_pos) = behavior.last_known_target_pos {
                    if !combat.is_in_range(my_pos, target_pos) || !perception.can_see_target {
                        // 目標離開攻擊範圍 → 追逐
                        behavior.set_state(AiState::Chase, current_time);
                        movement.is_running = true;
                        movement.move_target = Some(target_pos);
                    }
                } else {
                    behavior.set_state(AiState::Alert, current_time);
                }
            }

            AiState::Flee => {
                // 逃跑狀態在上面已處理
            }

            AiState::TakingCover => {
                // 在掩體後：等待並週期性探出射擊
                // 實際邏輯在 ai_cover_system 中處理
                if let Some(target_pos) = behavior.last_known_target_pos {
                    // 如果目標離開視野且血量恢復，可以離開掩體
                    if !perception.can_see_target && behavior.state_timer > ALERT_TIMEOUT {
                        behavior.set_state(AiState::Alert, current_time);
                    } else if health.percentage() > LOW_HEALTH_THRESHOLD {
                        // 血量恢復到 70% 以上，重新進攻
                        behavior.set_state(AiState::Chase, current_time);
                        movement.is_running = true;
                        movement.move_target = Some(target_pos);
                    }
                }
            }
        }
    }
}

// ============================================================================
// 移動系統
// ============================================================================

/// 重力常數
const GRAVITY: f32 = -9.8;

/// AI 移動系統：移動到目標位置
#[allow(clippy::type_complexity)]
pub fn ai_movement_system(
    time: Res<Time>,
    mut enemy_query: Query<(
        &mut Transform,
        &AiBehavior,
        &mut AiMovement,
        &mut KinematicCharacterController,
        Option<&KinematicCharacterControllerOutput>,
        Option<&mut PatrolPath>,
    ), With<Enemy>>,
) {
    let dt = time.delta_secs();

    for (mut transform, behavior, mut movement, mut controller, output, patrol) in &mut enemy_query {
        // 檢查是否在地面（用於重力計算）
        let is_grounded = output.is_some_and(|o| o.grounded);
        let gravity_velocity = if is_grounded { 0.0 } else { GRAVITY * dt };

        // 攻擊狀態：不移動，但面向目標（仍需重力）
        if behavior.state == AiState::Attack {
            controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
            if let Some(target_pos) = behavior.last_known_target_pos {
                let direction = (target_pos - transform.translation).normalize_or_zero();
                let flat_direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
                if flat_direction.length_squared() > 0.01 {
                    let look_target = transform.translation + flat_direction;
                    transform.look_at(look_target, Vec3::Y);
                }
            }
            continue;
        }

        // 閒置狀態不移動（仍需重力）
        if behavior.state == AiState::Idle {
            controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
            continue;
        }

        // 巡邏狀態：使用巡邏路徑
        if behavior.state == AiState::Patrol {
            if let Some(mut patrol_path) = patrol {
                // 處理等待
                if patrol_path.wait_timer > 0.0 {
                    patrol_path.wait_timer -= dt;
                    controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
                    continue;
                }

                // 取得當前巡邏點
                if let Some(waypoint) = patrol_path.current_waypoint() {
                    movement.move_target = Some(waypoint);
                    movement.is_running = false;

                    // 檢查是否到達
                    if movement.has_arrived(transform.translation) {
                        patrol_path.wait_timer = patrol_path.wait_time;
                        patrol_path.advance();
                    }
                }
            }
        }

        // 執行移動（包含重力）
        if let Some(target) = movement.move_target {
            let my_pos = transform.translation;
            let direction = (target - my_pos).normalize_or_zero();

            // 只在 XZ 平面移動
            let flat_direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();

            if flat_direction.length_squared() > 0.01 {
                // 使用 KinematicCharacterController 移動 + 重力
                let speed = movement.current_speed();
                let horizontal = flat_direction * speed * dt;
                controller.translation = Some(Vec3::new(horizontal.x, gravity_velocity, horizontal.z));

                // 面向移動方向
                let look_target = transform.translation + flat_direction;
                transform.look_at(look_target, Vec3::Y);
            } else {
                controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
            }
        } else {
            controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
        }
    }
}

// ============================================================================
// 攻擊系統
// ============================================================================

/// 近戰攻擊範圍（公尺）- 與玩家拳頭範圍一致
const MELEE_ATTACK_RANGE: f32 = 2.5;
/// 近戰傷害
const MELEE_DAMAGE: f32 = 15.0;
/// 近戰冷卻時間（秒）
const MELEE_COOLDOWN: f32 = 0.5;

/// AI 攻擊系統：向玩家開火或近戰攻擊
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn ai_attack_system(
    mut commands: Commands,
    time: Res<Time>,
    visuals: Option<Res<crate::combat::CombatVisuals>>,
    mut enemy_query: Query<(
        Entity,
        &Transform,
        &AiBehavior,
        &AiPerception,
        &mut AiCombat,
        &mut Weapon,
        &Children,
    ), With<Enemy>>,
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

    for (enemy_entity, transform, behavior, perception, mut combat, mut weapon, children) in &mut enemy_query {
        // 更新冷卻
        combat.tick(dt);

        // 更新武器冷卻和換彈
        if weapon.fire_cooldown > 0.0 {
            weapon.fire_cooldown -= dt;
        }
        if weapon.is_reloading {
            weapon.reload_timer -= dt;
            if weapon.reload_timer <= 0.0 {
                weapon.finish_reload();
            }
            continue; // 換彈中不能攻擊
        }

        // 生成保護期內不攻擊（給玩家反應時間）
        if behavior.is_spawn_protected() {
            continue;
        }

        // 只在攻擊狀態才開火
        if behavior.state != AiState::Attack {
            continue;
        }

        // 必須能看到目標
        if !perception.can_see_target {
            continue;
        }

        // 計算與目標的距離
        let target_distance = if let Some(target_pos) = behavior.last_known_target_pos {
            transform.translation.distance(target_pos)
        } else {
            f32::MAX
        };

        // 判斷是否使用近戰攻擊
        let use_melee = target_distance <= MELEE_ATTACK_RANGE;

        if use_melee {
            // === 近戰攻擊 ===
            // 檢查武器冷卻（近戰也用 weapon.fire_cooldown）
            if weapon.fire_cooldown > 0.0 {
                continue;
            }

            // 觸發揮拳動畫（找到右手臂）
            // 傷害事件將在動畫 Strike 階段發送
            for child in children.iter() {
                // 搜索直接子實體中的手臂
                if let Ok((arm_entity, arm)) = arm_query.get(child) {
                    if arm.is_right {
                        commands.entity(arm_entity).insert(
                            EnemyPunchAnimation::with_target(player_entity, enemy_entity)
                        );
                        break;
                    }
                }
            }

            // 設置近戰冷卻
            weapon.fire_cooldown = MELEE_COOLDOWN;
        } else {
            // === 遠程攻擊 ===
            // 檢查彈藥（自動換彈）
            if weapon.needs_reload() {
                weapon.start_reload();
                continue;
            }

            // 檢查是否可以開火
            let should_fire = combat.can_attack() || combat.should_fire_next();
            if !should_fire || !weapon.can_fire() {
                continue;
            }

            // 計算槍口位置（敵人手部前方）
            let forward = transform.forward();
            let muzzle_pos = transform.translation + forward.as_vec3() * MUZZLE_FORWARD_OFFSET + Vec3::new(0.0, MUZZLE_HEIGHT_OFFSET, 0.0);

            // 計算命中（簡化：基於精度和距離）
            let mut rng = rand::rng();
            let hit_roll: f32 = rng.random();

            // 距離衰減：超過有效射程一半後精度開始下降
            let half_range = weapon.stats.range * 0.5;
            let range_penalty = if target_distance > half_range {
                let over_range = (target_distance - half_range) / half_range;
                over_range.clamp(0.0, MAX_RANGE_PENALTY)
            } else {
                0.0
            };
            let effective_accuracy = (combat.accuracy - range_penalty).max(MIN_ACCURACY);

            // 計算彈道終點
            let tracer_end = if hit_roll <= effective_accuracy {
                // 命中！發送傷害事件
                let damage = weapon.stats.damage;
                damage_events.write(
                    DamageEvent::new(player_entity, damage, DamageSource::Bullet)
                        .with_attacker(enemy_entity)
                        .with_position(muzzle_pos)
                );
                // 命中時彈道直指玩家身體中心
                player_pos + Vec3::Y * 1.0
            } else {
                // 未命中時彈道偏移到玩家附近
                let miss_offset = Vec3::new(
                    rng.random_range(-MISS_SPREAD_X..MISS_SPREAD_X),
                    rng.random_range(MISS_SPREAD_Y_MIN..MISS_SPREAD_Y_MAX),
                    rng.random_range(-MISS_SPREAD_Z..MISS_SPREAD_Z),
                );
                player_pos + Vec3::Y * PLAYER_BODY_HEIGHT + miss_offset
            };

            // 生成槍口閃光和子彈拖尾
            if let Some(ref vis) = visuals {
                // 槍口閃光
                commands.spawn((
                    Mesh3d(vis.muzzle_mesh.clone()),
                    MeshMaterial3d(vis.muzzle_material.clone()),
                    Transform::from_translation(muzzle_pos),
                    crate::combat::MuzzleFlash { lifetime: 0.05 },
                ));

                // 子彈拖尾（使用步槍風格）
                spawn_bullet_tracer(&mut commands, vis, muzzle_pos, tracer_end, TracerStyle::Rifle);
            }

            // 消耗彈藥
            weapon.consume_ammo();
            weapon.fire_cooldown = weapon.stats.fire_rate;

            // 更新連射狀態
            combat.fire_once();
        }
    }
}

// ============================================================================
// 敵人生成系統
// ============================================================================

/// 敵人生成系統
pub fn enemy_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<EnemySpawnTimer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query: Query<&Transform, With<Player>>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }

    // 檢查敵人數量上限
    let current_count = enemy_query.iter().count();
    if current_count >= timer.max_enemies {
        return;
    }

    // 取得玩家位置
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 隨機敵人類型（先決定類型，再計算高度）
    let mut rng = rand::rng();
    let enemy_type = match rng.random_range(0..10) {
        0..=6 => EnemyType::Gangster,  // 70%
        7..=8 => EnemyType::Thug,       // 20%
        _ => EnemyType::Boss,           // 10%
    };

    // 隨機生成位置（在玩家周圍，但在攻擊範圍外）
    // 最小距離 45m，確保敵人生成在攻擊範圍（30m）之外
    // 玩家需要先看到敵人，敵人才會靠近攻擊
    let angle: f32 = rng.random::<f32>() * std::f32::consts::TAU;
    let min_spawn_distance: f32 = MIN_SPAWN_DISTANCE;
    let distance: f32 = min_spawn_distance + rng.random::<f32>() * (timer.spawn_radius - min_spawn_distance).max(5.0);

    // 計算正確的生成高度（碰撞體中心高度 = half_height + radius）
    // 新的碰撞體參數：Gangster (0.45, 0.25), Thug (0.50, 0.28), Boss (0.55, 0.30)
    let spawn_height = match enemy_type {
        EnemyType::Gangster => 0.45 + 0.25,  // 0.70
        EnemyType::Thug => 0.50 + 0.28,      // 0.78
        EnemyType::Boss => 0.55 + 0.30,      // 0.85
    };

    let spawn_pos = Vec3::new(
        player_pos.x + angle.cos() * distance,
        spawn_height,
        player_pos.z + angle.sin() * distance,
    );

    // 生成敵人
    spawn_enemy(&mut commands, &mut meshes, &mut materials, spawn_pos, enemy_type, &mut rng);
}

/// 生成單個敵人（人形模型 - 有關節的完整人體）
fn spawn_enemy(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    enemy_type: EnemyType,
    rng: &mut rand::prelude::ThreadRng,
) {
    // === 根據敵人類型定義外觀 ===
    let appearance = get_enemy_appearance(enemy_type, materials);

    // 敵人尺寸（碰撞體）
    let (collider_half_height, collider_radius) = match enemy_type {
        EnemyType::Gangster => (0.45, 0.25),
        EnemyType::Thug => (0.50, 0.28),
        EnemyType::Boss => (0.55, 0.30),
    };

    // 身體比例縮放因子
    let scale = match enemy_type {
        EnemyType::Gangster => 1.0,
        EnemyType::Thug => 1.1,    // 打手更壯
        EnemyType::Boss => 1.05,   // Boss 略高
    };

    // 分批插入組件以避免 tuple 大小限制
    let entity = commands.spawn((
        Name::new(format!("Enemy_{enemy_type:?}")),
        Enemy { enemy_type },
        Damageable,
        Health::new(enemy_type.health()),
        Weapon::new(enemy_type.weapon()),
        HitReaction::default(),  // 受傷反應
    )).id();

    // 隨機分配小隊角色（根據敵人類型調整權重）
    let squad_role = {
        let role_roll: f32 = rng.random();
        match enemy_type {
            EnemyType::Gangster => {
                // 小混混：50% 突擊, 40% 側翼, 10% 壓制
                if role_roll < GANGSTER_RUSHER_THRESHOLD { SquadRole::Rusher }
                else if role_roll < GANGSTER_FLANKER_THRESHOLD { SquadRole::Flanker }
                else { SquadRole::Suppressor }
            }
            EnemyType::Thug => {
                // 打手：70% 突擊, 20% 側翼, 10% 壓制
                if role_roll < THUG_RUSHER_THRESHOLD { SquadRole::Rusher }
                else if role_roll < GANGSTER_FLANKER_THRESHOLD { SquadRole::Flanker }
                else { SquadRole::Suppressor }
            }
            EnemyType::Boss => {
                // Boss：30% 隊長, 30% 壓制, 40% 側翼
                if role_roll < 0.3 { SquadRole::Leader }
                else if role_roll < 0.6 { SquadRole::Suppressor }
                else { SquadRole::Flanker }
            }
        }
    };

    // AI 組件
    commands.entity(entity).insert((
        AiBehavior::default(),
        AiPerception::default().with_range(30.0, 50.0),
        AiMovement {
            walk_speed: 3.0,
            run_speed: 6.0,
            ..default()
        },
        AiCombat {
            attack_range: enemy_type.weapon().range * 0.6,
            accuracy: match enemy_type {
                EnemyType::Gangster => 0.4,
                EnemyType::Thug => 0.55,
                EnemyType::Boss => 0.7,
            },
            ..default()
        },
        CoverSeeker::default(),  // 掩體尋找
        SquadMember::with_role(squad_role),  // 小隊角色
    ));

    // 物理和視覺
    commands.entity(entity).insert((
        Collider::capsule_y(collider_half_height, collider_radius),
        RigidBody::KinematicPositionBased,  // 敵人使用運動學剛體
        KinematicCharacterController::default(),
        CollisionGroups::new(
            COLLISION_GROUP_CHARACTER,
            COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
        ),  // 敵人與角色、載具、靜態物碰撞
        Transform::from_translation(position),
        GlobalTransform::default(),  // 必須有此組件，否則子實體會觸發 B0004 警告
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    // 添加子實體（完整人形視覺網格）
    commands.entity(entity).with_children(|parent| {
        spawn_humanoid_mesh(parent, meshes, &appearance, scale, collider_half_height);
    });
}

/// 敵人外觀數據
struct EnemyAppearance {
    skin: Handle<StandardMaterial>,
    shirt: Handle<StandardMaterial>,
    pants: Handle<StandardMaterial>,
    shoes: Handle<StandardMaterial>,
    hair: Handle<StandardMaterial>,
    eye_white: Handle<StandardMaterial>,
    eye_iris: Handle<StandardMaterial>,
    lip: Handle<StandardMaterial>,
    hair_style: HairStyle,
    has_beard: bool,
}

/// 髮型類型
#[derive(Clone, Copy)]
enum HairStyle {
    ShortSpiky,    // 小混混：短刺頭
    Bald,          // 打手：光頭
    SlickedBack,   // Boss：油頭後梳
}

/// 根據敵人類型獲取外觀
fn get_enemy_appearance(
    enemy_type: EnemyType,
    materials: &mut Assets<StandardMaterial>,
) -> EnemyAppearance {
    let (skin_color, shirt_color, pants_color, shoe_color, hair_color, hair_style, has_beard) = match enemy_type {
        EnemyType::Gangster => (
            Color::srgb(0.87, 0.72, 0.62),  // 淺膚色
            Color::srgb(0.15, 0.15, 0.2),   // 深灰 T 恤
            Color::srgb(0.2, 0.22, 0.3),    // 牛仔褲藍
            Color::srgb(0.9, 0.9, 0.95),    // 白色球鞋
            Color::srgb(0.15, 0.12, 0.08),  // 深棕髮
            HairStyle::ShortSpiky,
            false,
        ),
        EnemyType::Thug => (
            Color::srgb(0.75, 0.58, 0.45),  // 較深膚色
            Color::srgb(0.08, 0.08, 0.08),  // 黑色背心
            Color::srgb(0.25, 0.2, 0.15),   // 卡其褲
            Color::srgb(0.12, 0.12, 0.12),  // 黑色靴子
            Color::srgb(0.1, 0.08, 0.06),   // 黑髮（光頭用）
            HairStyle::Bald,
            true,  // 有鬍子
        ),
        EnemyType::Boss => (
            Color::srgb(0.82, 0.68, 0.58),  // 中等膚色
            Color::srgb(0.1, 0.1, 0.12),    // 黑色西裝
            Color::srgb(0.08, 0.08, 0.1),   // 黑色西褲
            Color::srgb(0.2, 0.12, 0.08),   // 棕色皮鞋
            Color::srgb(0.05, 0.05, 0.05),  // 黑髮
            HairStyle::SlickedBack,
            false,
        ),
    };

    EnemyAppearance {
        skin: materials.add(StandardMaterial {
            base_color: skin_color,
            perceptual_roughness: 0.6,
            ..default()
        }),
        shirt: materials.add(StandardMaterial {
            base_color: shirt_color,
            perceptual_roughness: 0.8,
            ..default()
        }),
        pants: materials.add(StandardMaterial {
            base_color: pants_color,
            perceptual_roughness: 0.7,
            ..default()
        }),
        shoes: materials.add(StandardMaterial {
            base_color: shoe_color,
            perceptual_roughness: 0.5,
            ..default()
        }),
        hair: materials.add(StandardMaterial {
            base_color: hair_color,
            perceptual_roughness: 0.9,
            ..default()
        }),
        eye_white: materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.95, 0.95),
            ..default()
        }),
        eye_iris: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.15, 0.1),
            ..default()
        }),
        lip: materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.45, 0.4),
            perceptual_roughness: 0.4,
            ..default()
        }),
        hair_style,
        has_beard,
    }
}

/// 生成完整人形網格（有關節）
fn spawn_humanoid_mesh(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    app: &EnemyAppearance,
    scale: f32,
    half_height: f32,
) {
    // === 身體比例常數（以碰撞體中心為原點）===
    let head_y = half_height + 0.12 * scale;
    let neck_y = half_height - 0.02 * scale;
    let chest_y = 0.15 * scale;
    let waist_y = -0.05 * scale;
    let hip_y = -0.18 * scale;

    // === 頭部 ===
    spawn_head(parent, meshes, app, head_y, scale);

    // === 脖子 ===
    parent.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.04 * scale, 0.08 * scale))),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(0.0, neck_y, 0.0),
    ));

    // === 軀幹（胸部 + 腰部 + 臀部）===
    // 胸部
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.28 * scale, 0.2 * scale, 0.14 * scale))),
        MeshMaterial3d(app.shirt.clone()),
        Transform::from_xyz(0.0, chest_y, 0.0),
    ));
    // 腰部（較窄）
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.22 * scale, 0.1 * scale, 0.12 * scale))),
        MeshMaterial3d(app.shirt.clone()),
        Transform::from_xyz(0.0, waist_y, 0.0),
    ));
    // 臀部/髖部
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.26 * scale, 0.1 * scale, 0.14 * scale))),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(0.0, hip_y, 0.0),
    ));

    // === 手臂（上臂 + 肘關節 + 前臂 + 手）===
    spawn_arm(parent, meshes, app, scale, chest_y, true);   // 左臂
    spawn_arm(parent, meshes, app, scale, chest_y, false);  // 右臂

    // === 腿部（大腿 + 膝關節 + 小腿 + 腳踝 + 腳掌）===
    spawn_leg(parent, meshes, app, scale, hip_y, true);   // 左腿
    spawn_leg(parent, meshes, app, scale, hip_y, false);  // 右腿
}

/// 生成頭部（含臉部細節和髮型）
#[allow(clippy::too_many_lines)]
fn spawn_head(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    app: &EnemyAppearance,
    head_y: f32,
    scale: f32,
) {
    let head_radius = 0.1 * scale;

    // 頭部主體（略扁的球體）
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(head_radius))),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(0.0, head_y, 0.0)
            .with_scale(Vec3::new(0.95, 1.0, 0.9)),
    ));

    // === 臉部細節 ===
    // 眼睛（左）
    let eye_y = head_y + 0.015 * scale;
    let eye_z = head_radius * 0.85;
    let eye_spacing = 0.035 * scale;

    // 眼白
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.018 * scale))),
        MeshMaterial3d(app.eye_white.clone()),
        Transform::from_xyz(eye_spacing, eye_y, eye_z)
            .with_scale(Vec3::new(1.2, 0.8, 0.5)),
    ));
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.018 * scale))),
        MeshMaterial3d(app.eye_white.clone()),
        Transform::from_xyz(-eye_spacing, eye_y, eye_z)
            .with_scale(Vec3::new(1.2, 0.8, 0.5)),
    ));

    // 瞳孔
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.008 * scale))),
        MeshMaterial3d(app.eye_iris.clone()),
        Transform::from_xyz(eye_spacing, eye_y, eye_z + 0.008),
    ));
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.008 * scale))),
        MeshMaterial3d(app.eye_iris.clone()),
        Transform::from_xyz(-eye_spacing, eye_y, eye_z + 0.008),
    ));

    // 眉毛
    let brow_mat = app.hair.clone();
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.03 * scale, 0.008 * scale, 0.01 * scale))),
        MeshMaterial3d(brow_mat.clone()),
        Transform::from_xyz(eye_spacing, eye_y + 0.025 * scale, eye_z - 0.005),
    ));
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.03 * scale, 0.008 * scale, 0.01 * scale))),
        MeshMaterial3d(brow_mat),
        Transform::from_xyz(-eye_spacing, eye_y + 0.025 * scale, eye_z - 0.005),
    ));

    // 鼻子
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.02 * scale, 0.035 * scale, 0.025 * scale))),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(0.0, head_y - 0.01 * scale, eye_z + 0.01),
    ));

    // 嘴巴
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.04 * scale, 0.012 * scale, 0.015 * scale))),
        MeshMaterial3d(app.lip.clone()),
        Transform::from_xyz(0.0, head_y - 0.045 * scale, eye_z - 0.01),
    ));

    // 耳朵
    let ear_y = head_y;
    let ear_x = head_radius * 0.9;
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.025 * scale))),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(ear_x, ear_y, 0.0)
            .with_scale(Vec3::new(0.4, 1.0, 0.7)),
    ));
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.025 * scale))),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(-ear_x, ear_y, 0.0)
            .with_scale(Vec3::new(0.4, 1.0, 0.7)),
    ));

    // === 髮型（根據類型變化）===
    match app.hair_style {
        HairStyle::ShortSpiky => {
            // 短刺頭：多個小尖刺
            #[allow(clippy::cast_precision_loss)]
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let spike_x = angle.cos() * head_radius * 0.6;
                let spike_z = angle.sin() * head_radius * 0.6 - 0.02;
                parent.spawn((
                    Mesh3d(meshes.add(Capsule3d::new(0.015 * scale, 0.04 * scale))),
                    MeshMaterial3d(app.hair.clone()),
                    Transform::from_xyz(spike_x, head_y + head_radius * 0.7, spike_z)
                        .with_rotation(Quat::from_rotation_x(-0.3 + angle.sin() * 0.2)),
                ));
            }
        }
        HairStyle::Bald => {
            // 光頭：只有一點點陰影/刺青
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(head_radius * 1.01))),
                MeshMaterial3d(app.hair.clone()),
                Transform::from_xyz(0.0, head_y + head_radius * 0.3, -head_radius * 0.3)
                    .with_scale(Vec3::new(0.5, 0.2, 0.5)),
            ));
            // 鬍子
            if app.has_beard {
                parent.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.06 * scale, 0.04 * scale, 0.02 * scale))),
                    MeshMaterial3d(app.hair.clone()),
                    Transform::from_xyz(0.0, head_y - 0.06 * scale, eye_z - 0.02),
                ));
            }
        }
        HairStyle::SlickedBack => {
            // 油頭後梳：光滑的髮型
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(head_radius * 1.08))),
                MeshMaterial3d(app.hair.clone()),
                Transform::from_xyz(0.0, head_y + head_radius * 0.15, -head_radius * 0.2)
                    .with_scale(Vec3::new(1.0, 0.5, 1.2)),
            ));
            // 側面髮際線
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(head_radius * 2.1, 0.02 * scale, head_radius * 0.8))),
                MeshMaterial3d(app.hair.clone()),
                Transform::from_xyz(0.0, head_y + head_radius * 0.6, -head_radius * 0.3),
            ));
        }
    }
}

/// 生成手臂（有關節，帶有 `EnemyArm` 組件以支援揮拳動畫）
/// 比例：手指到大腿中段
fn spawn_arm(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    app: &EnemyAppearance,
    scale: f32,
    chest_y: f32,
    is_left: bool,
) {
    let side = if is_left { 1.0 } else { -1.0 };
    let shoulder_x = 0.15 * scale * side;
    let arm_tilt = 0.12 * side;  // 手臂自然下垂角度

    // 肩關節位置
    let shoulder_y = chest_y + 0.06 * scale;

    // 計算手臂整體的靜止位置和旋轉
    let rest_position = Vec3::new(shoulder_x, shoulder_y, 0.0);
    let rest_rotation = Quat::from_rotation_z(arm_tilt);

    // 創建手臂根實體（帶有 EnemyArm 組件）
    parent.spawn((
        Transform::from_translation(rest_position).with_rotation(rest_rotation),
        GlobalTransform::default(),  // 必須有此組件，否則 mesh 子實體會觸發 B0004 警告
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        if is_left {
            EnemyArm::left(rest_position, rest_rotation)
        } else {
            EnemyArm::right(rest_position, rest_rotation)
        },
        Name::new(if is_left { "LeftArm" } else { "RightArm" }),
    )).with_children(|arm| {
        // 肩關節（球形）- 相對於手臂根
        arm.spawn((
            Mesh3d(meshes.add(Sphere::new(0.038 * scale))),
            MeshMaterial3d(app.shirt.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));

        // 上臂（縮短：手指到大腿中段）
        let upper_arm_len = 0.10 * scale;
        arm.spawn((
            Mesh3d(meshes.add(Capsule3d::new(0.030 * scale, upper_arm_len))),
            MeshMaterial3d(app.shirt.clone()),
            Transform::from_xyz(0.0, -upper_arm_len, 0.0),
        ));

        // 肘關節（球形）
        let elbow_y = -upper_arm_len * 2.0 - 0.015 * scale;
        arm.spawn((
            Mesh3d(meshes.add(Sphere::new(0.026 * scale))),
            MeshMaterial3d(app.skin.clone()),
            Transform::from_xyz(0.0, elbow_y, 0.0),
        ));

        // 前臂（縮短）
        let forearm_len = 0.08 * scale;
        let forearm_y = elbow_y - forearm_len;
        arm.spawn((
            Mesh3d(meshes.add(Capsule3d::new(0.024 * scale, forearm_len))),
            MeshMaterial3d(app.skin.clone()),
            Transform::from_xyz(0.0, forearm_y, 0.0),
        ));

        // 手腕
        let wrist_y = forearm_y - forearm_len;
        arm.spawn((
            Mesh3d(meshes.add(Sphere::new(0.018 * scale))),
            MeshMaterial3d(app.skin.clone()),
            Transform::from_xyz(0.0, wrist_y, 0.0),
        ));

        // 手掌
        arm.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.038 * scale, 0.055 * scale, 0.018 * scale))),
            MeshMaterial3d(app.skin.clone()),
            Transform::from_xyz(0.0, wrist_y - 0.038 * scale, 0.0),
        ));

        // 手指（簡化為一組）
        arm.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.032 * scale, 0.035 * scale, 0.014 * scale))),
            MeshMaterial3d(app.skin.clone()),
            Transform::from_xyz(0.0, wrist_y - 0.08 * scale, 0.0),
        ));
    });
}

/// 生成腿部（有關節）
/// 比例修正：腿部總長度應在碰撞體範圍內（約 0.52 從臀部到腳底）
fn spawn_leg(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut Assets<Mesh>,
    app: &EnemyAppearance,
    scale: f32,
    hip_y: f32,
    is_left: bool,
) {
    let side = if is_left { 1.0 } else { -1.0 };
    let hip_x = 0.07 * scale * side;

    // 髖關節（球形）
    let joint_y = hip_y - 0.03 * scale;
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.045 * scale))),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(hip_x, joint_y, 0.0),
    ));

    // 大腿（縮短）
    let thigh_len = 0.11 * scale;
    let thigh_y = joint_y - thigh_len;
    parent.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.045 * scale, thigh_len))),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(hip_x, thigh_y, 0.0),
    ));

    // 膝關節（球形）
    let knee_y = thigh_y - thigh_len - 0.015 * scale;
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.038 * scale))),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(hip_x, knee_y, 0.0),
    ));

    // 小腿（縮短）
    let shin_len = 0.10 * scale;
    let shin_y = knee_y - shin_len;
    parent.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.034 * scale, shin_len))),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(hip_x, shin_y, 0.0),
    ));

    // 腳踝
    let ankle_y = shin_y - shin_len - 0.015 * scale;
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.028 * scale))),
        MeshMaterial3d(app.shoes.clone()),
        Transform::from_xyz(hip_x, ankle_y, 0.0),
    ));

    // 腳掌（鞋子）
    let foot_y = ankle_y - 0.02 * scale;
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.055 * scale, 0.035 * scale, 0.10 * scale))),
        MeshMaterial3d(app.shoes.clone()),
        Transform::from_xyz(hip_x, foot_y, 0.02 * scale),
    ));

    // 鞋頭（腳趾部分）
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.028 * scale))),
        MeshMaterial3d(app.shoes.clone()),
        Transform::from_xyz(hip_x, foot_y, 0.065 * scale)
            .with_scale(Vec3::new(1.0, 0.7, 1.2)),
    ));
}

// ============================================================================
// 敵人揮拳動畫系統
// ============================================================================

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
        // 更新計時器
        anim.timer += dt;

        let (wind_up_end, strike_end, duration) = anim.phase_times();
        let t = anim.timer;

        // 更新階段
        if t < wind_up_end {
            anim.phase = PunchPhase::WindUp;
        } else if t < strike_end {
            anim.phase = PunchPhase::Strike;
        } else if t < duration {
            anim.phase = PunchPhase::Return;
        }

        // 進入 Strike 階段時發送傷害事件
        if anim.phase == PunchPhase::Strike && !anim.damage_dealt {
            if let (Some(target), Some(attacker)) = (anim.target, anim.attacker) {
                damage_events.write(
                    DamageEvent::new(target, MELEE_DAMAGE, DamageSource::Melee)
                        .with_attacker(attacker)
                );
                anim.damage_dealt = true;
            }
        }

        // 只處理右手臂的動畫
        if !arm.is_right {
            continue;
        }

        // 揮拳動畫（相對於手臂根位置）
        match anim.phase {
            PunchPhase::WindUp => {
                // 蓄力：手臂向後收
                let phase_progress = t / wind_up_end;
                let ease = ease_out_quad(phase_progress);

                // 旋轉手臂向後收
                let rotation = Quat::from_euler(
                    bevy::math::EulerRot::XYZ,
                    -0.3 * ease,     // X: 稍微抬起
                    0.0,
                    arm.rest_rotation.to_euler(bevy::math::EulerRot::XYZ).2 + 0.3 * ease
                );

                transform.rotation = rotation;
            }
            PunchPhase::Strike => {
                // 出拳：手臂快速向前伸直
                let phase_t = t - wind_up_end;
                let phase_duration = strike_end - wind_up_end;
                let phase_progress = phase_t / phase_duration;
                let ease = ease_out_cubic(phase_progress);

                // 手臂旋轉成水平向前
                let rotation = Quat::from_euler(
                    bevy::math::EulerRot::XYZ,
                    1.4 * ease,  // X: 從垂直轉到水平
                    0.0,
                    arm.rest_rotation.to_euler(bevy::math::EulerRot::XYZ).2 * (1.0 - ease)
                );

                // 位置向前伸出
                let offset = Vec3::new(0.0, 0.0, 0.4 * ease);
                transform.translation = arm.rest_position + offset;
                transform.rotation = rotation;
            }
            PunchPhase::Return => {
                // 收回：快速回到原位
                let phase_t = t - strike_end;
                let phase_duration = duration - strike_end;
                let phase_progress = phase_t / phase_duration;
                let ease = ease_in_out_quad(phase_progress);

                // 從出拳終點插值回原位
                let strike_rotation = Quat::from_euler(
                    bevy::math::EulerRot::XYZ,
                    1.4,
                    0.0,
                    0.0
                );
                let strike_offset = Vec3::new(0.0, 0.0, 0.4);

                transform.translation = (arm.rest_position + strike_offset).lerp(arm.rest_position, ease);
                transform.rotation = strike_rotation.slerp(arm.rest_rotation, ease);
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

// === 緩動函數 ===

fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

// ============================================================================
// 敵人死亡系統
// ============================================================================

/// 敵人死亡處理系統
/// 注意：布娃娃系統已在 combat/damage.rs 處理敵人死亡
/// 此系統僅作為後備，處理任何未被布娃娃系統處理的死亡事件
pub fn enemy_death_system(
    mut commands: Commands,
    mut death_events: MessageReader<DeathEvent>,
    // 排除已有 Ragdoll 組件的敵人（由布娃娃系統處理）
    enemy_query: Query<Entity, (With<Enemy>, Without<Ragdoll>)>,
) {
    for event in death_events.read() {
        // 檢查是否為敵人（且沒有 Ragdoll 組件）
        if enemy_query.get(event.entity).is_ok() {
            // 移除敵人實體及其子實體（視覺網格）
            // Bevy 0.17: despawn() 預設會移除子實體
            if let Ok(mut entity_commands) = commands.get_entity(event.entity) {
                entity_commands.despawn();
            }
            // TODO: 掉落物品、經驗值
        }
    }
}

/// 掩體釋放系統
/// 當敵人死亡或變成布娃娃時，釋放其佔用的掩體
/// 也處理掩體佔用者實體不存在的清理
///
/// 優化版本：只在有死亡事件時執行完整清理
pub fn cover_release_system(
    mut death_events: MessageReader<DeathEvent>,
    enemy_query: Query<&CoverSeeker, With<Enemy>>,
    mut cover_query: Query<&mut CoverPoint>,
) {
    let mut had_deaths = false;

    // 處理死亡事件，釋放死亡敵人佔用的掩體
    for event in death_events.read() {
        had_deaths = true;

        // 使用 let-else 模式減少嵌套
        let Ok(seeker) = enemy_query.get(event.entity) else { continue };
        let Some(cover_entity) = seeker.target_cover else { continue };
        let Ok(mut cover) = cover_query.get_mut(cover_entity) else { continue };
        cover.release();
    }

    // 只在有死亡事件時才執行完整清理
    // 這樣可以避免每幀都遍歷所有掩體
    if !had_deaths {
        return;
    }

    // 清理：釋放佔用者已不存在的掩體
    // 這處理了死亡實體在查詢前已被移除的情況
    for mut cover in cover_query.iter_mut() {
        // 跳過未被佔用的掩體
        if !cover.occupied { continue; }
        // 檢查佔用者是否仍存在
        let Some(occupant) = cover.occupant else { continue };
        if enemy_query.get(occupant).is_err() {
            cover.release();
        }
    }
}

// ============================================================================
// 掩體系統 (GTA 5 風格)
// ============================================================================

/// AI 掩體尋找系統
/// 當 AI 血量低時，尋找附近的掩體並移動過去
#[allow(clippy::type_complexity)]
pub fn ai_cover_system(
    time: Res<Time>,
    mut enemy_query: Query<(
        Entity,
        &Transform,
        &Health,
        &mut AiBehavior,
        &mut AiMovement,
        &mut CoverSeeker,
    ), (With<Enemy>, Without<Ragdoll>)>,
    mut cover_query: Query<(Entity, &Transform, &mut CoverPoint)>,
    player_query: Query<&Transform, With<Player>>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    let player_pos = match player_query.single() {
        Ok(t) => t.translation,
        Err(_) => return,
    };

    for (enemy_entity, transform, health, mut behavior, mut movement, mut seeker) in &mut enemy_query {
        let my_pos = transform.translation;
        let health_percent = health.percentage();

        // 更新掩體計時器
        seeker.tick(dt);

        // 如果正在掩體中
        if seeker.is_in_cover {
            // 處理探出射擊
            if seeker.is_peeking {
                // 探出時可以攻擊
                if behavior.state != AiState::Attack {
                    behavior.set_state(AiState::Attack, current_time);
                }
                // 探出 0.5 秒後縮回
                if seeker.peek_timer <= seeker.peek_interval - 0.5 {
                    seeker.end_peek();
                    behavior.set_state(AiState::TakingCover, current_time);
                }
            }
            continue;
        }

        // 檢查是否應該尋找掩體
        if seeker.should_seek_cover(health_percent) && behavior.state != AiState::Flee {
            // 尋找最近的可用掩體
            let mut best_cover: Option<(Entity, Vec3, f32)> = None;

            for (cover_entity, cover_transform, cover) in cover_query.iter() {
                if !cover.is_available() {
                    continue;
                }

                let cover_pos = cover_transform.translation;
                let distance_sq = my_pos.distance_squared(cover_pos);
                let max_cover_distance_sq = seeker.max_cover_distance * seeker.max_cover_distance;

                // 檢查距離是否在範圍內 (使用 distance_squared 避免 sqrt)
                if distance_sq > max_cover_distance_sq {
                    continue;
                }

                // 檢查掩體是否能遮擋玩家
                if !cover.is_covered_from(cover_pos, cover_pos - cover.cover_direction * 0.5, player_pos) {
                    continue;
                }

                // 選擇最近的掩體 (使用 distance_squared)
                let is_closer = best_cover.map_or(true, |(_, _, d)| distance_sq < d);
                if is_closer {
                    best_cover = Some((cover_entity, cover_pos, distance_sq));
                }
            }

            // 找到掩體，移動過去
            if let Some((cover_entity, cover_pos, _)) = best_cover {
                // 再次檢查掩體是否有效
                let Ok((_, _, cover)) = cover_query.get(cover_entity) else { continue };
                seeker.target_cover = Some(cover_entity);
                behavior.set_state(AiState::TakingCover, current_time);
                movement.is_running = true;
                // 移動到掩體後方
                let behind_cover = cover_pos - cover.cover_direction * 0.8;
                movement.move_target = Some(behind_cover);

                // 佔用掩體
                if let Ok((_, _, mut cover_mut)) = cover_query.get_mut(cover_entity) {
                    cover_mut.occupy(enemy_entity);
                }
            }
        }

        // 檢查是否到達掩體 (使用 distance_squared 避免 sqrt)
        if let Some(cover_entity) = seeker.target_cover {
            if let Ok((_, cover_transform, _)) = cover_query.get(cover_entity) {
                let cover_pos = cover_transform.translation;
                if my_pos.distance_squared(cover_pos) < COVER_ARRIVAL_SQ {
                    // 到達掩體
                    seeker.enter_cover(cover_entity);
                    movement.is_running = false;
                    movement.move_target = None;
                }
            }
        }
    }
}

// 掩體傷害減免已整合到 combat/damage.rs 的 damage_system 中

// ============================================================================
// 小隊協調系統 (GTA 5 風格包抄戰術)
// ============================================================================

/// 小隊協調系統
/// 協調同一小隊的敵人進行包抄戰術
#[allow(clippy::type_complexity)]
pub fn squad_coordination_system(
    time: Res<Time>,
    mut squad_manager: ResMut<SquadManager>,
    player_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<(
        Entity,
        &Transform,
        &mut AiBehavior,
        &mut AiMovement,
        &mut SquadMember,
    ), (With<Enemy>, Without<Ragdoll>)>,
) {
    let dt = time.delta_secs();

    // 先更新所有成員的計時器（只執行一次，避免重複呼叫）
    for (_, _, _, _, mut member) in &mut enemy_query {
        member.tick(dt);
    }

    // 更新協調計時器
    squad_manager.coordination_timer.tick(time.delta());
    if !squad_manager.coordination_timer.just_finished() {
        // 協調邏輯有冷卻時間，跳過本幀的包抄計算
        return;
    }

    let current_time = time.elapsed_secs();

    // 取得玩家位置
    let player_pos = match player_query.single() {
        Ok(t) => t.translation,
        Err(_) => return,
    };

    // 收集所有敵人位置（用於包抄計算）
    // 注意：這裡只收集一次，後面使用索引排除自己而非重新過濾
    let ally_positions: Vec<Vec3> = enemy_query
        .iter()
        .map(|(_, t, _, _, _)| t.translation)
        .collect();

    // 處理每個敵人的包抄行為
    for (_entity, transform, mut behavior, mut movement, mut member) in &mut enemy_query {
        let my_pos = transform.translation;

        // 只有在追逐或攻擊狀態時才執行包抄
        if behavior.state != AiState::Chase && behavior.state != AiState::Attack {
            // 如果正在包抄但狀態改變，結束包抄
            if member.is_flanking {
                member.end_flank();
            }
            continue;
        }

        // 檢查是否可以開始新的包抄（Flanker 角色專用）
        if member.role == SquadRole::Flanker && member.can_flank() {
            // 計算包抄位置
            // 注意：過濾自己的位置，避免把自己算入隊友
            // 效能備註：對於少量敵人（< 20）這是可接受的
            let other_positions: Vec<Vec3> = ally_positions
                .iter()
                .filter(|p| p.distance(my_pos) > 0.5) // 排除自己（使用距離而非 Entity 比較）
                .copied()
                .collect();

            let flank_pos = calculate_flank_position(
                my_pos,
                player_pos,
                member.role,
                &other_positions,
                member.min_ally_distance,
            );

            // 開始包抄
            member.start_flank(flank_pos);
            movement.move_target = Some(flank_pos);
            movement.is_running = true;
        }

        // 如果正在包抄，更新移動目標
        if member.is_flanking {
            if let Some(flank_target) = member.flank_target {
                // 檢查是否到達包抄位置 (使用 distance_squared 避免 sqrt)
                if my_pos.distance_squared(flank_target) < FLANK_ARRIVAL_SQ {
                    // 到達包抄位置，結束包抄，準備攻擊
                    member.end_flank();
                    // 切換到攻擊狀態
                    if behavior.state == AiState::Chase {
                        behavior.set_state(AiState::Attack, current_time);
                        movement.is_running = false;
                    }
                } else {
                    // 繼續向包抄位置移動
                    movement.move_target = Some(flank_target);
                }
            }
        }

        // 根據角色調整行為
        match member.role {
            SquadRole::Rusher => {
                // 突擊者：直接衝向目標
                if !member.is_flanking && behavior.state == AiState::Chase {
                    movement.move_target = behavior.last_known_target_pos;
                    movement.is_running = true;
                }
            }
            SquadRole::Suppressor => {
                // 壓制者：保持距離，不主動接近
                if let Some(target_pos) = behavior.last_known_target_pos {
                    let distance = my_pos.distance(target_pos);
                    let ideal_dist = member.role.ideal_attack_distance();

                    if distance < ideal_dist - 2.0 {
                        // 太近了，後退
                        let retreat_dir = (my_pos - target_pos).normalize_or_zero();
                        movement.move_target = Some(my_pos + retreat_dir * 5.0);
                        movement.is_running = false;
                    } else if distance > ideal_dist + 5.0 {
                        // 太遠了，靠近一點
                        movement.move_target = Some(target_pos);
                        movement.is_running = false;
                    } else {
                        // 距離合適，停止移動準備射擊
                        movement.move_target = None;
                    }
                }
            }
            SquadRole::Leader => {
                // 隊長：中距離觀察，不衝太前面
                if let Some(target_pos) = behavior.last_known_target_pos {
                    let distance = my_pos.distance(target_pos);
                    let ideal_dist = member.role.ideal_attack_distance();

                    if distance < ideal_dist - 3.0 {
                        // 太近了，稍微後退
                        let retreat_dir = (my_pos - target_pos).normalize_or_zero();
                        movement.move_target = Some(my_pos + retreat_dir * 3.0);
                        movement.is_running = false;
                    }
                }
            }
            SquadRole::Flanker => {
                // 側翼者：由上面的包抄邏輯處理
            }
        }
    }
}
