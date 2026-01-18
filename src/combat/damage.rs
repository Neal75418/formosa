//! 傷害系統
//!
//! 處理傷害計算、死亡邏輯等。

#![allow(dead_code)] // Phase 2+ 功能預留

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::math::EulerRot;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use super::components::*;
use super::killcam::{KillCamState, KillCamTrigger};
use crate::player::Player;
use crate::ui::{NotificationQueue, DamageIndicatorState, trigger_damage_indicator, FloatingDamageNumber, FloatingDamageTracker, ChineseFont};
use crate::audio::{AudioManager, WeaponSounds, play_hit_sound};
use crate::ai::{AiBehavior, AiMovement, AiPerception, AiCombat, CoverSeeker, CoverPoint};
use crate::pedestrian::Pedestrian;
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

/// 玩家重生狀態
#[derive(Resource, Default)]
pub struct RespawnState {
    pub is_dead: bool,
    pub respawn_timer: f32,
    pub death_position: Vec3,
}

/// 重生位置（西門町漢中街起點）
pub const RESPAWN_POSITION: Vec3 = Vec3::new(5.0, 0.7, -5.0);

// === 傷害系統常數 ===
/// 命中標記顯示時長（秒）
const HIT_MARKER_DURATION: f32 = 0.2;
/// 浮動傷害數字頭頂偏移
const FLOATING_DAMAGE_HEAD_OFFSET: f32 = 1.8;
/// 預設受傷位置 Y 偏移
const DEFAULT_HIT_POSITION_Y_OFFSET: f32 = 1.2;
/// 爆頭高度閾值（相對於敵人位置）
const HEADSHOT_HEIGHT_THRESHOLD: f32 = 1.5;
/// 胸口高度（衝量應用點）
const CHEST_HEIGHT: f32 = 1.0;
/// 重生計時器時長（秒）
const RESPAWN_TIMER_DURATION: f32 = 3.0;

// === 衝量強度常數 ===
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

// === 傾斜強度常數 ===
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

// === 布娃娃物理常數 ===
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

// === 血液粒子常數 ===
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

/// 傷害處理系統
#[allow(clippy::too_many_arguments)]
pub fn damage_system(
    mut damage_events: MessageReader<DamageEvent>,
    mut death_events: MessageWriter<DeathEvent>,
    mut armor_break_events: MessageWriter<ArmorBreakEvent>,
    mut commands: Commands,
    // 合併查詢：Health + Armor + CoverSeeker + HitReaction (同一實體)
    mut health_query: Query<(&mut Health, Option<&mut Armor>, Option<&CoverSeeker>, Option<&mut HitReaction>)>,
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
        // 取得目標的生命值、護甲、掩體狀態、受傷反應（合併查詢）
        let Ok((mut health, mut armor, cover_seeker, mut hit_reaction)) = health_query.get_mut(event.target) else {
            continue;
        };

        // === 掩體傷害減免 ===
        let mut actual_damage = event.amount;
        if let Some(seeker) = cover_seeker {
            if seeker.is_in_cover && !seeker.is_peeking {
                if let Some(cover_entity) = seeker.target_cover {
                    if let Ok(cover) = cover_point_query.get(cover_entity) {
                        actual_damage *= 1.0 - cover.damage_reduction;
                    }
                }
            }
        }

        // 計算實際傷害（護甲吸收）
        let mut armor_was_hit = false;
        let mut armor_was_broken = false;
        if let Some(ref mut armor) = armor {
            let armor_before = armor.current;
            actual_damage = armor.absorb_damage(actual_damage);

            // 檢查護甲是否受擊或破碎
            if armor_before > 0.0 {
                armor_was_hit = true;
                if armor.current <= 0.0 {
                    armor_was_broken = true;
                }
            }
        }

        // 發送護甲破碎事件（如果有護甲被擊中）
        if armor_was_hit {
            if let Ok(target_transform) = transform_query.get(event.target) {
                let hit_pos = event.hit_position.unwrap_or(target_transform.translation + Vec3::Y * 1.2);
                armor_break_events.write(ArmorBreakEvent {
                    entity: event.target,
                    position: hit_pos,
                    is_full_break: armor_was_broken,
                });
            }
        }

        // 扣血
        let damage_dealt = health.take_damage(actual_damage, current_time);

        // === 觸發受傷反應 ===
        if let Some(ref mut hit_reaction) = hit_reaction {
            // 計算擊中方向
            let hit_direction = if let Some(attacker) = event.attacker {
                if let (Ok(target_transform), Ok(attacker_transform)) =
                    (transform_query.get(event.target), transform_query.get(attacker))
                {
                    (target_transform.translation - attacker_transform.translation).normalize_or_zero()
                } else {
                    Vec3::NEG_Z
                }
            } else {
                Vec3::NEG_Z
            };

            hit_reaction.trigger(damage_dealt, hit_direction, event.is_headshot);
        }

        // 如果是玩家，顯示傷害提示並觸發受傷指示器
        if player_query.get(event.target).is_ok() {
            res.notifications.warning(format!("-{:.0} HP", damage_dealt));
            trigger_damage_indicator(&mut res.damage_indicator, damage_dealt);
        }

        // 如果玩家攻擊敵人，顯示命中標記並播放命中音效
        if let Some(player) = player_entity {
            if event.attacker == Some(player) && enemy_query.get(event.target).is_ok() {
                res.combat_state.hit_marker_timer = HIT_MARKER_DURATION;
                res.combat_state.hit_marker_headshot = event.is_headshot;
                // 播放命中音效
                if let Some(ref sounds) = res.weapon_sounds {
                    play_hit_sound(&mut commands, sounds, &res.audio_manager, event.is_headshot);
                }

                // === GTA 5 風格浮動傷害數字 ===
                if res.damage_tracker.active_count < res.damage_tracker.max_count {
                    // 取得傷害位置（使用擊中位置或敵人位置）
                    let damage_pos = if let Some(hit_pos) = event.hit_position {
                        hit_pos + Vec3::Y * 0.3
                    } else if let Ok(enemy_transform) = transform_query.get(event.target) {
                        enemy_transform.translation + Vec3::Y * FLOATING_DAMAGE_HEAD_OFFSET
                    } else {
                        continue;
                    };

                    // 生成浮動傷害數字
                    let offset = res.damage_tracker.next_offset();
                    let floating_damage = FloatingDamageNumber::new(damage_pos, damage_dealt, event.is_headshot)
                        .with_offset(offset);

                    // 創建世界空間文字實體
                    if let Some(ref chinese_font) = res.font {
                        spawn_floating_damage_number(&mut commands, floating_damage, chinese_font);
                        res.damage_tracker.active_count += 1;
                    }
                }
            }
        }

        // 檢查死亡
        if health.is_dead() {
            // 計算擊中方向（從攻擊者指向目標）
            let hit_direction = if let (Some(attacker), Ok(target_transform)) =
                (event.attacker, transform_query.get(event.target))
            {
                if let Ok(attacker_transform) = transform_query.get(attacker) {
                    // 從攻擊者指向目標的水平方向
                    let dir = target_transform.translation - attacker_transform.translation;
                    Some(Vec3::new(dir.x, 0.3, dir.z).normalize_or_zero())
                } else {
                    // 無法取得攻擊者位置，使用預設方向
                    Some(Vec3::new(0.0, 0.2, -1.0).normalize())
                }
            } else {
                // 無攻擊者或無法取得位置
                Some(Vec3::new(0.0, 0.2, -1.0).normalize())
            };

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

/// 死亡處理系統
#[allow(clippy::too_many_arguments)]
pub fn death_system(
    mut death_events: MessageReader<DeathEvent>,
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<Player>>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<Ragdoll>)>,
    all_enemies_query: Query<Entity, (With<Enemy>, Without<Ragdoll>)>,
    // 行人和警察查詢（用於犯罪事件）
    pedestrian_query: Query<&Transform, (With<Pedestrian>, Without<Player>, Without<Enemy>)>,
    police_query: Query<&Transform, (With<PoliceOfficer>, Without<Player>, Without<Enemy>)>,
    mut crime_events: MessageWriter<CrimeEvent>,
    mut notifications: ResMut<NotificationQueue>,
    mut respawn_state: ResMut<RespawnState>,
    mut ragdoll_tracker: ResMut<RagdollTracker>,
    mut killcam: ResMut<KillCamState>,
    blood_visuals: Option<Res<BloodVisuals>>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();
    let player_entity = player_query.single().ok().map(|(e, _)| e);

    for event in death_events.read() {
        // 檢查是否為玩家死亡
        if let Ok((_, transform)) = player_query.get(event.entity) {
            if !respawn_state.is_dead {
                notifications.error("💀 你死了！3 秒後重生...");
                respawn_state.is_dead = true;
                respawn_state.respawn_timer = RESPAWN_TIMER_DURATION;
                respawn_state.death_position = transform.translation;
            }
            continue;
        }

        // === 檢查是否為警察死亡 - 嚴重犯罪！ ===
        if let Ok(police_transform) = police_query.get(event.entity) {
            // 只有玩家擊殺警察才算犯罪
            if event.killer == player_entity {
                crime_events.write(CrimeEvent::PoliceKilled {
                    victim: event.entity,
                    position: police_transform.translation,
                });
                notifications.warning("⚠️ 擊殺警察！通緝等級大幅上升！");
            }
        }

        // === 檢查是否為行人死亡 - 謀殺 ===
        if let Ok(ped_transform) = pedestrian_query.get(event.entity) {
            // 只有玩家擊殺行人才算犯罪
            if event.killer == player_entity {
                crime_events.write(CrimeEvent::Murder {
                    victim: event.entity,
                    position: ped_transform.translation,
                });
            }
        }

        // 敵人死亡 - 啟動布娃娃效果
        if let Ok((_, enemy_transform)) = enemy_query.get(event.entity) {
            notifications.success("擊殺敵人！");

            // === Kill Cam 觸發邏輯 ===
            // 只有玩家擊殺敵人才觸發
            if event.killer == player_entity {
                // 記錄擊殺（用於連殺計數）
                killcam.record_kill(current_time);

                // 計算剩餘敵人數量（不包括當前被擊殺的）
                let remaining_enemies = all_enemies_query.iter().count().saturating_sub(1);
                let target_pos = enemy_transform.translation;

                // 判斷是否觸發 Kill Cam
                let trigger_type = if event.cause == DamageSource::Bullet {
                    // 檢查擊殺條件（優先順序：爆頭 > 最後敵人 > 連殺）
                    if event.hit_position.map(|p| p.y > enemy_transform.translation.y + 1.5).unwrap_or(false) {
                        // 爆頭擊殺（擊中位置高於肩膀）
                        Some(KillCamTrigger::Headshot)
                    } else if remaining_enemies == 0 {
                        // 最後一個敵人
                        Some(KillCamTrigger::LastEnemy)
                    } else if killcam.should_trigger_multi_kill() {
                        // 連殺 (3+)
                        Some(KillCamTrigger::MultiKill(killcam.get_kill_streak()))
                    } else {
                        None
                    }
                } else {
                    None
                };

                // 觸發 Kill Cam
                if let Some(trigger) = trigger_type {
                    killcam.trigger(trigger, event.entity, target_pos, current_time);
                }
            }

            // 取得敵人位置用於生成血液粒子
            let enemy_pos = enemy_transform.translation + Vec3::Y * CHEST_HEIGHT;

            // 計算衝擊力方向和強度
            let impulse_dir = event.hit_direction.unwrap_or(Vec3::new(0.0, 0.2, -1.0).normalize());

            // 根據傷害來源調整衝擊力
            let impulse_strength = match event.cause {
                DamageSource::Bullet => IMPULSE_BULLET,
                DamageSource::Explosion => IMPULSE_EXPLOSION,
                DamageSource::Vehicle => IMPULSE_VEHICLE,
                DamageSource::Melee => IMPULSE_MELEE,
                DamageSource::Fall => IMPULSE_FALL,
                DamageSource::Fire => IMPULSE_FIRE,
                DamageSource::Environment => IMPULSE_ENVIRONMENT,
            };

            // 計算傾倒軸（垂直於衝擊方向，讓身體往後倒）
            let tilt_axis = Vec3::new(impulse_dir.z, 0.0, -impulse_dir.x).normalize_or_zero();
            // 根據傷害來源調整傾倒力度
            let tilt_strength = match event.cause {
                DamageSource::Bullet => TILT_BULLET,
                DamageSource::Explosion => TILT_EXPLOSION,
                DamageSource::Vehicle => TILT_VEHICLE,
                DamageSource::Melee => TILT_MELEE,
                _ => TILT_DEFAULT,
            };

            // === 屍體數量限制 ===
            if ragdoll_tracker.ragdolls.len() >= ragdoll_tracker.max_count {
                // 移除最舊的屍體
                if let Some((oldest_entity, _)) = ragdoll_tracker.ragdolls.first().copied() {
                    if let Ok(mut entity_commands) = commands.get_entity(oldest_entity) {
                        entity_commands.despawn();
                    }
                    ragdoll_tracker.ragdolls.remove(0);
                }
            }
            // 追蹤新屍體
            ragdoll_tracker.ragdolls.push((event.entity, time.elapsed_secs()));

            // === 生成血液粒子 ===
            if let Some(ref blood) = blood_visuals {
                spawn_blood_particles(&mut commands, enemy_pos, impulse_dir, blood);
            }

            // 添加 Ragdoll 組件並移除 AI 相關組件
            if let Ok(mut entity_commands) = commands.get_entity(event.entity) {
                entity_commands
                    // 添加布娃娃組件
                    .insert(Ragdoll::with_impulse(impulse_dir, impulse_strength))
                    // 移除運動學控制器，改用動態物理
                    .remove::<KinematicCharacterController>()
                    // 移除 AI 組件（停止 AI 行為）
                    .remove::<AiBehavior>()
                    .remove::<AiMovement>()
                    .remove::<AiPerception>()
                    .remove::<AiCombat>()
                    // 切換到動態剛體
                    .insert(RigidBody::Dynamic)
                    // 添加重力（確保掉落）
                    .insert(GravityScale(RAGDOLL_GRAVITY_SCALE))
                    // 添加速度組件
                    .insert(Velocity::default())
                    // 添加外部衝擊力（增強傾倒效果）
                    .insert(ExternalImpulse {
                        // 衝擊力：水平方向 + 輕微向上
                        impulse: Vec3::new(
                            impulse_dir.x * impulse_strength,
                            impulse_strength * RAGDOLL_UPWARD_PUSH_FACTOR,
                            impulse_dir.z * impulse_strength,
                        ),
                        // 旋轉力矩：讓身體往後傾倒
                        torque_impulse: tilt_axis * tilt_strength,
                    })
                    // 添加阻尼讓物理更自然
                    .insert(Damping {
                        linear_damping: RAGDOLL_LINEAR_DAMPING,
                        angular_damping: RAGDOLL_ANGULAR_DAMPING,
                    })
                    // 屍體碰撞組：只與地面碰撞，不阻擋玩家
                    .insert(CollisionGroups::new(
                        Group::GROUP_10,  // 屍體專用組
                        Group::GROUP_1,   // 只與地面/靜態物碰撞
                    ));
            }

            // TODO: 掉落物品、經驗值等
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
        let velocity = (direction + spread).normalize() * rng.random_range(BLOOD_PARTICLE_MIN_SPEED..BLOOD_PARTICLE_MAX_SPEED);
        let max_lifetime = rng.random_range(BLOOD_PARTICLE_MIN_LIFETIME..BLOOD_PARTICLE_MAX_LIFETIME);
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
pub fn health_regeneration_system(
    time: Res<Time>,
    mut query: Query<&mut Health>,
) {
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
            if speed < RAGDOLL_SETTLE_SPEED_THRESHOLD && ragdoll.lifetime > RAGDOLL_SETTLE_TIME_THRESHOLD {
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

/// 布娃娃視覺效果系統
/// 處理布娃娃的視覺淡出效果
pub fn ragdoll_visual_system(
    ragdoll_query: Query<(&Ragdoll, &Children)>,
    mut material_query: Query<&mut Visibility>,
) {
    for (ragdoll, children) in ragdoll_query.iter() {
        // 在最後一段時間開始淡出（通過閃爍實現）
        let fade_start = ragdoll.max_lifetime - RAGDOLL_FADE_OFFSET;
        if ragdoll.lifetime > fade_start {
            let fade_progress = (ragdoll.lifetime - fade_start) / RAGDOLL_FADE_OFFSET;
            // 閃爍頻率隨時間增加
            let blink_rate = RAGDOLL_BLINK_BASE_RATE + fade_progress * RAGDOLL_BLINK_ACCELERATION;
            let visible = (ragdoll.lifetime * blink_rate).sin() > 0.0;

            // 遍歷所有子實體設置可見性
            for child in children.iter() {
                if let Ok(mut visibility) = material_query.get_mut(child) {
                    *visibility = if visible {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    };
                }
            }
        }
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
        if transform.translation.y < 0.05 {
            transform.translation.y = 0.05;
            particle.velocity = Vec3::ZERO;
            // 加速消失
            particle.lifetime += dt * 3.0;
        }

        // 根據生命週期縮小粒子
        let life_ratio = 1.0 - (particle.lifetime / particle.max_lifetime);
        let scale = life_ratio.max(0.3);
        transform.scale = Vec3::splat(scale * 0.05);
    }
}

// ============================================================================
// 浮動傷害數字系統
// ============================================================================

/// 傷害數字顏色常數
const DAMAGE_NUMBER_COLOR: Color = Color::WHITE;
const HEADSHOT_NUMBER_COLOR: Color = Color::srgb(1.0, 0.9, 0.0);  // 金黃色
const CRITICAL_NUMBER_COLOR: Color = Color::srgb(1.0, 0.3, 0.1);  // 橙紅色

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
        CRITICAL_NUMBER_COLOR  // 高傷害用橙紅色
    } else {
        DAMAGE_NUMBER_COLOR
    };

    // 格式化傷害數字
    let text = if damage.is_headshot {
        format!("💀 {:.0}", damage.damage)  // 爆頭加骷髏
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
        Transform::from_translation(position)
            .with_scale(Vec3::splat(0.02)),  // 縮小到世界空間大小
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

/// 浮動傷害數字更新系統
/// 處理上浮動畫、縮放變化和淡出效果
pub fn floating_damage_number_update_system(
    mut commands: Commands,
    time: Res<Time>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut damage_query: Query<(
        Entity,
        &mut FloatingDamageNumber,
        &mut Transform,
        &mut TextColor,
    ), Without<Camera3d>>,
    mut damage_tracker: ResMut<FloatingDamageTracker>,
) {
    let dt = time.delta_secs();

    // 取得攝影機位置（用於 Billboard 效果）
    let camera_pos = camera_query.single().map(|t| t.translation).ok();

    for (entity, mut damage, mut transform, mut text_color) in damage_query.iter_mut() {
        // 更新生命時間
        damage.lifetime += dt;

        // 檢查是否過期
        if damage.lifetime >= damage.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                damage_tracker.active_count = damage_tracker.active_count.saturating_sub(1);
            }
            continue;
        }

        // 更新位置（向上漂浮）
        let y_offset = damage.y_offset();
        transform.translation = damage.start_position
            + Vec3::new(damage.horizontal_offset, y_offset, 0.0);

        // Billboard 效果：讓文字面向攝影機
        if let Some(cam_pos) = camera_pos {
            let direction = cam_pos - transform.translation;
            if direction.length_squared() > 0.001 {
                let look_rotation = Quat::from_rotation_arc(
                    Vec3::NEG_Z,
                    direction.normalize(),
                );
                transform.rotation = look_rotation;
            }
        }

        // 更新縮放（彈出效果）
        let scale = damage.scale() * 0.02;  // 基礎縮放 0.02
        transform.scale = Vec3::splat(scale);

        // 更新透明度（淡出效果）
        let alpha = damage.alpha();
        let base_color = if damage.is_headshot {
            HEADSHOT_NUMBER_COLOR
        } else if damage.damage >= 50.0 {
            CRITICAL_NUMBER_COLOR
        } else {
            DAMAGE_NUMBER_COLOR
        };

        // 創建帶透明度的顏色
        text_color.0 = base_color.with_alpha(alpha);
    }
}

// ============================================================================
// 受傷反應系統
// ============================================================================

/// 受傷反應更新系統
/// 每幀更新所有 HitReaction 組件的狀態
pub fn hit_reaction_update_system(
    time: Res<Time>,
    mut query: Query<&mut HitReaction>,
) {
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
                    target_euler.0,      // 使用反應的 X 旋轉
                    current_euler.1,     // 保持 Y 旋轉
                    current_euler.2,     // 保持 Z 旋轉
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

/// 敵人受傷反應擊退系統（使用 AiMovement）
pub fn enemy_hit_reaction_knockback_system(
    time: Res<Time>,
    mut query: Query<(&HitReaction, &mut Transform), (With<Enemy>, Without<Ragdoll>)>,
) {
    let delta = time.delta_secs();

    for (reaction, mut transform) in query.iter_mut() {
        let knockback = reaction.get_knockback_velocity();
        if knockback.length_squared() > 0.001 {
            // 直接移動位置
            transform.translation += knockback * delta;
            // 確保不會掉到地面以下
            if transform.translation.y < 0.0 {
                transform.translation.y = 0.0;
            }
        }
    }
}

/// 行人受傷反應擊退系統
pub fn pedestrian_hit_reaction_knockback_system(
    time: Res<Time>,
    mut query: Query<(&HitReaction, &mut Transform), (With<Pedestrian>, Without<Ragdoll>)>,
) {
    let delta = time.delta_secs();

    for (reaction, mut transform) in query.iter_mut() {
        let knockback = reaction.get_knockback_velocity();
        if knockback.length_squared() > 0.001 {
            // 直接移動位置
            transform.translation += knockback * delta;
            // 確保不會掉到地面以下
            if transform.translation.y < 0.0 {
                transform.translation.y = 0.0;
            }
        }
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
            bevy::math::EulerRot::XYZ,
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
            let fade_progress = (shard.lifetime - shard.max_lifetime * 0.7) / (shard.max_lifetime * 0.3);
            let scale = (1.0 - fade_progress).max(0.1);
            transform.scale = Vec3::splat(scale);
        }
    }
}
