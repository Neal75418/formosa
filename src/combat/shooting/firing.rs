//! 武器發射系統
//!
//! 處理武器發射邏輯：瞄準計算、彈道射線、近戰攻擊、霰彈散佈。

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::auto_aim::apply_lock_on_aim_assist;
use super::effects::{spawn_bullet_tracer, spawn_impact_effect, spawn_muzzle_flash};
use crate::ai::AiBehavior;
use crate::audio::{play_weapon_fire_sound, AudioManager, WeaponSounds};
use crate::combat::components::*;
use crate::combat::health::{
    check_headshot, DamageEvent, DamageSource, Damageable, HEADSHOT_MULTIPLIER,
};
use crate::combat::visuals::*;
use crate::combat::weapon::*;
use crate::core::{rapier_real_to_f32, CameraSettings, CameraShake, RecoilState};
use crate::player::skills::{award_shooting_xp, award_stealth_kill_xp};
use crate::player::{Player, PlayerSkills, StealthState, STEALTH_KILL_MULTIPLIER};

/// 武器音效資源（合併為 SystemParam 以減少系統參數數量）
#[derive(SystemParam)]
pub(crate) struct FireWeaponAudio<'w> {
    weapon_sounds: Option<Res<'w, WeaponSounds>>,
    audio_manager: Res<'w, AudioManager>,
}

/// 近戰戰鬥上下文（鎖定 + 連擊 + 格擋 + 潛行狀態 + 技能 + 隱匿擊殺，合併為 SystemParam）
#[derive(SystemParam)]
pub(crate) struct MeleeCombatContext<'w, 's> {
    pub lock_on: Res<'w, LockOnState>,
    pub combo: ResMut<'w, MeleeComboState>,
    pub block: ResMut<'w, BlockState>,
    pub stealth: Res<'w, StealthState>,
    pub skills: ResMut<'w, PlayerSkills>, // ResMut 以支持 XP 獎勵
    pub ai_query: Query<'w, 's, &'static AiBehavior>,
    pub takedown: ResMut<'w, StealthTakedownState>,
}

// ============================================================================
// 射擊常數
// ============================================================================

// --- 攝影機/瞄準 ---
/// 玩家中心高度（攝影機射線起點）
const PLAYER_CENTER_HEIGHT: f32 = 1.5;
/// 武器基礎高度偏移（相對玩家位置）
const WEAPON_BASE_HEIGHT: f32 = 0.55;

// --- 攝影機震動 ---
/// 攝影機震動持續時間（秒）
const CAMERA_SHAKE_DURATION: f32 = 0.08;
/// 手槍震動強度
const PISTOL_SHAKE_INTENSITY: f32 = 0.02;
/// 衝鋒槍震動強度
const SMG_SHAKE_INTENSITY: f32 = 0.015;
/// 霰彈槍震動強度
const SHOTGUN_SHAKE_INTENSITY: f32 = 0.05;
/// 步槍震動強度
const RIFLE_SHAKE_INTENSITY: f32 = 0.025;
/// 狙擊槍震動強度
const SNIPER_SHAKE_INTENSITY: f32 = 0.06;
/// 火箭筒震動強度
const RPG_SHAKE_INTENSITY: f32 = 0.08;

// --- RPG 投射物 ---
/// RPG 爆炸傷害
const RPG_EXPLOSION_DAMAGE: f32 = 200.0;
/// RPG 爆炸半徑
const RPG_EXPLOSION_RADIUS: f32 = 10.0;
/// RPG 投射物最大存活時間（秒）
const RPG_MAX_LIFETIME: f32 = 5.0;

// --- 散佈/瞄準 ---
/// 瞄準時後座力倍率（減少 50%）
const AIM_RECOIL_MULTIPLIER: f32 = 0.5;
/// 瞄準時散佈倍率
const AIM_SPREAD_MULTIPLIER: f32 = 0.5;
/// 準星擴散增量（每次射擊）
const CROSSHAIR_BLOOM_PER_SHOT: f32 = 0.2;

// --- 霰彈槍散佈 ---
/// 霰彈槍內圈半徑比例（佔總散佈的 50%）
const SHOTGUN_INNER_RADIUS_RATIO: f32 = 0.5;
/// 霰彈槍內圈隨機偏移量
const SHOTGUN_INNER_JITTER: f32 = 0.1;
/// 霰彈槍外圈隨機偏移量
const SHOTGUN_OUTER_JITTER: f32 = 0.15;
/// 霰彈槍內圈最大彈丸數
const SHOTGUN_INNER_MAX_PELLETS: u32 = 6;

use crate::combat::explosives::{ExplosionEvent, ExplosiveType};

use super::melee::fire_melee;

// ============================================================================
// RPG 投射物
// ============================================================================

/// RPG 火箭投射物組件
#[derive(Component)]
pub struct RpgProjectile {
    /// 飛行方向
    pub direction: Vec3,
    /// 飛行速度（m/s）
    pub speed: f32,
    /// 直擊傷害
    pub direct_damage: f32,
    /// 爆炸傷害
    pub explosion_damage: f32,
    /// 爆炸半徑
    pub explosion_radius: f32,
    /// 已存活時間
    pub lifetime: f32,
    /// 最大存活時間
    pub max_lifetime: f32,
    /// 發射者 Entity
    pub owner: Entity,
}

/// RPG 投射物更新系統：移動、碰撞偵測、觸發爆炸
pub fn rpg_projectile_update_system(
    mut commands: Commands,
    time: Res<Time>,
    rapier_context: ReadRapierContext,
    mut query: Query<(Entity, &mut Transform, &mut RpgProjectile)>,
    mut explosion_events: MessageWriter<ExplosionEvent>,
    mut damage_events: MessageWriter<DamageEvent>,
    damageable_query: Query<Entity, With<Damageable>>,
) {
    let Ok(rapier) = rapier_context.single() else {
        return;
    };
    let dt = time.delta_secs();

    for (entity, mut transform, mut projectile) in &mut query {
        projectile.lifetime += dt;

        // 超時：空中爆炸
        if projectile.lifetime >= projectile.max_lifetime {
            explosion_events.write(ExplosionEvent {
                position: transform.translation,
                radius: projectile.explosion_radius,
                max_damage: projectile.explosion_damage,
                explosive_type: ExplosiveType::Rocket,
                source: Some(projectile.owner),
            });
            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.despawn();
            }
            continue;
        }

        // 移動
        let movement = projectile.direction * projectile.speed * dt;
        let ray_distance = movement.length();

        // 碰撞偵測（本幀移動距離內的 raycast）
        let filter = QueryFilter::default().exclude_collider(projectile.owner);
        if let Some((hit_entity, toi)) = rapier.cast_ray(
            transform.translation,
            projectile.direction,
            ray_distance as bevy_rapier3d::prelude::Real,
            true,
            filter,
        ) {
            // 碰撞點
            let hit_point = transform.translation + projectile.direction * rapier_real_to_f32(toi);

            // 直擊傷害（如果命中可受傷實體）
            if damageable_query.get(hit_entity).is_ok() {
                damage_events.write(
                    DamageEvent::new(hit_entity, projectile.direct_damage, DamageSource::Bullet)
                        .with_attacker(projectile.owner)
                        .with_position(hit_point),
                );
            }

            // 爆炸
            explosion_events.write(ExplosionEvent {
                position: hit_point,
                radius: projectile.explosion_radius,
                max_damage: projectile.explosion_damage,
                explosive_type: ExplosiveType::Rocket,
                source: Some(projectile.owner),
            });

            if let Ok(mut ec) = commands.get_entity(entity) {
                ec.despawn();
            }
            continue;
        }

        // 正常移動
        transform.translation += movement;
    }
}

// ============================================================================
// 輔助結構與函數
// ============================================================================

/// 角色方向向量
struct CharacterVectors {
    forward: Vec3,
    right: Vec3,
    up: Vec3,
}

impl CharacterVectors {
    fn from_yaw(yaw: f32) -> Self {
        Self {
            forward: Vec3::new(-yaw.sin(), 0.0, -yaw.cos()),
            right: Vec3::new(-yaw.cos(), 0.0, yaw.sin()),
            up: Vec3::Y,
        }
    }
}

/// 計算攝影機瞄準點（第三人稱瞄準修正）
fn calculate_aim_point(
    camera_settings: &CameraSettings,
    player_pos: Vec3,
    rapier: &RapierContext,
) -> Vec3 {
    // 從攝影機發射 raycast 找到瞄準點
    const MAX_AIM_DISTANCE: f32 = 500.0;

    let yaw = camera_settings.yaw;
    let pitch = camera_settings.pitch;
    let cam_distance = camera_settings.distance;

    // 攝影機前方向量
    let cam_forward = Vec3::new(
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    )
    .normalize();

    // 計算攝影機位置（玩家後上方）
    let player_center = player_pos + Vec3::Y * PLAYER_CENTER_HEIGHT;
    let cam_back = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let cam_up = Vec3::Y * pitch.sin().abs();
    let camera_pos = player_center + cam_back * cam_distance * pitch.cos() + cam_up * cam_distance;
    let aim_filter = QueryFilter::default();

    if let Some((_, toi)) = rapier.cast_ray(
        camera_pos,
        cam_forward,
        MAX_AIM_DISTANCE as bevy_rapier3d::prelude::Real,
        true,
        aim_filter,
    ) {
        camera_pos + cam_forward * rapier_real_to_f32(toi)
    } else {
        camera_pos + cam_forward * MAX_AIM_DISTANCE
    }
}

/// 計算槍口位置
fn calculate_muzzle_position(
    player_pos: Vec3,
    char_vecs: &CharacterVectors,
    weapon_type: WeaponType,
    is_aiming: bool,
    muzzle_offset: Vec3,
) -> Vec3 {
    let base_pos = player_pos + char_vecs.up * WEAPON_BASE_HEIGHT;

    if weapon_type == WeaponType::Fist {
        // 拳頭：從手的位置出發
        base_pos + char_vecs.right * 0.25 + char_vecs.forward * 0.3
    } else if is_aiming {
        // 瞄準姿勢：槍口在身體前方中央偏右
        let hand_pos = base_pos + char_vecs.right * 0.15 + char_vecs.forward * 0.45;
        hand_pos + char_vecs.forward * muzzle_offset.z + char_vecs.up * muzzle_offset.y
    } else {
        // 待機持槍姿勢：槍口朝下前方
        let hand_pos =
            base_pos + char_vecs.right * 0.22 + char_vecs.forward * 0.25 + char_vecs.up * (-0.1);
        let tilted_forward = (char_vecs.forward * 0.8 + char_vecs.up * (-0.2)).normalize();
        hand_pos + tilted_forward * muzzle_offset.z * 0.8
    }
}

/// 檢查近戰攻擊是否為背後偷襲
///
/// 條件：攻擊者位於目標背後 120° 範圍內。
/// 回傳 true 表示攻擊來自目標視野外（背面）。
#[inline]
fn is_backstab(attacker_pos: Vec3, target_pos: Vec3, target_forward: Vec3) -> bool {
    let to_attacker = (attacker_pos - target_pos).normalize_or_zero();
    let forward_2d = Vec3::new(target_forward.x, 0.0, target_forward.z).normalize_or_zero();
    let attacker_2d = Vec3::new(to_attacker.x, 0.0, to_attacker.z).normalize_or_zero();
    // dot < -0.5 ≈ 角度 > 120°（背後）
    forward_2d.dot(attacker_2d) < -0.5
}

/// 檢查武器是否應該發射
#[inline]
fn should_fire(input: &ShootingInput, weapon: &Weapon) -> bool {
    let trigger_pressed = if weapon.stats.is_automatic {
        input.is_fire_held
    } else {
        input.is_fire_pressed
    };
    trigger_pressed && weapon.can_fire()
}

/// 取得武器類型對應的攝影機震動強度
#[inline]
fn get_camera_shake_intensity(weapon_type: WeaponType) -> f32 {
    match weapon_type {
        WeaponType::Pistol => PISTOL_SHAKE_INTENSITY,
        WeaponType::SMG => SMG_SHAKE_INTENSITY,
        WeaponType::Shotgun => SHOTGUN_SHAKE_INTENSITY,
        WeaponType::Rifle => RIFLE_SHAKE_INTENSITY,
        WeaponType::SniperRifle => SNIPER_SHAKE_INTENSITY,
        WeaponType::RPG => RPG_SHAKE_INTENSITY,
        WeaponType::Fist | WeaponType::Staff | WeaponType::Knife => 0.0,
    }
}

/// 應用遠程武器發射後效果（後座力、攝影機震動）
fn apply_ranged_fire_effects(
    weapon: &Weapon,
    is_aiming: bool,
    skill_recoil_multiplier: f32,
    recoil_state: &mut RecoilState,
    camera_shake: &mut CameraShake,
) {
    // 瞄準倍率與技能倍率疊加（相乘）
    let aim_mult = if is_aiming {
        AIM_RECOIL_MULTIPLIER
    } else {
        1.0
    };
    let final_recoil_mult = aim_mult * skill_recoil_multiplier;
    recoil_state.add_recoil(
        weapon.stats.recoil_vertical * final_recoil_mult,
        weapon.stats.recoil_horizontal * final_recoil_mult,
    );

    let shake_intensity = get_camera_shake_intensity(weapon.stats.weapon_type);
    if shake_intensity > 0.0 {
        camera_shake.trigger(shake_intensity, CAMERA_SHAKE_DURATION);
    }
}

/// 生成霰彈槍散佈模式（環形分布）
/// 返回每顆彈丸的偏移角度 (x, y)，單位為弧度
fn generate_shotgun_pattern(pellet_count: u32, base_spread: f32) -> Vec<Vec2> {
    use std::f32::consts::TAU;
    let mut pattern = Vec::with_capacity(pellet_count as usize);
    let spread_rad = base_spread.to_radians();

    if pellet_count == 0 {
        return pattern;
    }

    // 中心彈丸（最準確）
    pattern.push(Vec2::ZERO);

    if pellet_count == 1 {
        return pattern;
    }

    // 計算內外圈彈丸數量
    let remaining = pellet_count - 1;
    let inner_count = remaining.min(SHOTGUN_INNER_MAX_PELLETS); // 內圈最多 6 顆
    let outer_count = remaining.saturating_sub(SHOTGUN_INNER_MAX_PELLETS); // 剩餘的放外圈

    // 內圈（較準確，佔總散佈半徑的 SHOTGUN_INNER_RADIUS_RATIO）
    let inner_radius = spread_rad * SHOTGUN_INNER_RADIUS_RATIO;
    for i in 0..inner_count {
        let angle = (i as f32 / inner_count as f32) * TAU;
        // 加入少量隨機偏移使其更自然
        let jitter = (rand::random::<f32>() - 0.5) * SHOTGUN_INNER_JITTER;
        pattern.push(Vec2::new(
            angle.cos() * inner_radius * (1.0 + jitter),
            angle.sin() * inner_radius * (1.0 + jitter),
        ));
    }

    // 外圈（較散，100% 散佈半徑）
    let outer_radius = spread_rad;
    for i in 0..outer_count {
        // 外圈與內圈錯開，更均勻分布
        let angle = (i as f32 / outer_count.max(1) as f32) * TAU + TAU / 12.0;
        let jitter = (rand::random::<f32>() - 0.5) * SHOTGUN_OUTER_JITTER;
        pattern.push(Vec2::new(
            angle.cos() * outer_radius * (1.0 + jitter),
            angle.sin() * outer_radius * (1.0 + jitter),
        ));
    }

    pattern
}

/// 發射遠程武器（處理多彈丸和槍口閃光）
#[allow(clippy::too_many_arguments)]
fn fire_ranged_weapon(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    attacker: Entity,
    muzzle_pos: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    is_aiming: bool,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    skills: &mut PlayerSkills, // 用於 XP 獎勵
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) {
    let spread = if is_aiming {
        weapon.stats.spread * AIM_SPREAD_MULTIPLIER
    } else {
        weapon.stats.spread
    };

    let ctx = FireContext {
        visuals,
        attacker,
        origin: muzzle_pos,
        direction,
        weapon,
        rapier,
    };

    // 霰彈槍使用環形散佈模式
    if weapon.stats.weapon_type == WeaponType::Shotgun {
        let pattern = generate_shotgun_pattern(weapon.stats.pellet_count, spread);
        for offset in pattern {
            fire_bullet_with_offset(
                commands,
                &ctx,
                offset,
                damage_events,
                skills,
                damageable_query,
                transform_query,
            );
        }
    } else {
        // 其他武器使用隨機散佈
        for _ in 0..weapon.stats.pellet_count {
            fire_bullet(
                commands,
                &ctx,
                spread,
                damage_events,
                skills,
                damageable_query,
                transform_query,
            );
        }
    }

    spawn_muzzle_flash(commands, visuals, muzzle_pos);
}

// ============================================================================
// 發射武器主系統
// ============================================================================

/// 發射武器系統
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn fire_weapon_system(
    input: Res<ShootingInput>,
    time: Res<Time>,
    camera_settings: Res<CameraSettings>,
    mut melee_ctx: MeleeCombatContext,
    rapier_context: ReadRapierContext,
    mut commands: Commands,
    combat_visuals: Option<Res<CombatVisuals>>,
    weapon_visuals: Option<Res<WeaponVisuals>>,
    audio: FireWeaponAudio,
    mut player_query: Query<(Entity, &Transform, &mut WeaponInventory), With<Player>>,
    mut combat_state: ResMut<CombatState>,
    mut recoil_state: ResMut<RecoilState>,
    mut camera_shake: ResMut<CameraShake>,
    mut damage_events: MessageWriter<DamageEvent>,
    damageable_query: Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: Query<&Transform>,
) {
    let Some(visuals) = combat_visuals else {
        return;
    };
    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    // 隱匿擊殺進行中，禁止射擊
    if melee_ctx.takedown.phase != StealthTakedownPhase::None {
        return;
    }

    for (player_entity, player_transform, mut inventory) in &mut player_query {
        let Some(weapon) = inventory.current_weapon_mut() else {
            continue;
        };

        if !should_fire(&input, weapon) {
            continue;
        }

        let player_pos = player_transform.translation;
        let char_vecs = CharacterVectors::from_yaw(camera_settings.yaw);

        // 計算瞄準點（含自動瞄準吸附）
        let raw_aim_point = calculate_aim_point(&camera_settings, player_pos, &rapier);
        let aim_point =
            apply_lock_on_aim_assist(raw_aim_point, &melee_ctx.lock_on, &transform_query);

        let muzzle_offset = weapon_visuals
            .as_ref()
            .and_then(|wv| wv.get(weapon.stats.weapon_type))
            .map_or(Vec3::new(0.0, 0.0, 0.15), |wd| wd.muzzle_offset);

        let muzzle_pos = calculate_muzzle_position(
            player_pos,
            &char_vecs,
            weapon.stats.weapon_type,
            combat_state.is_aiming,
            muzzle_offset,
        );

        let direction = (aim_point - muzzle_pos).normalize();

        // 根據武器類型發射
        // 格擋中不可同時攻擊
        if input.is_block_pressed {
            continue;
        }

        if weapon.stats.weapon_type.is_melee() {
            // 反擊加成：精準格擋後的攻擊獲得 2x 傷害
            let counter_mult = melee_ctx.block.consume_counter();
            // 靜默擊殺檢測：蹲伏 + 目標未察覺 + 背後攻擊
            let mut stealth_target: Option<(Entity, Vec3)> = None;
            let stealth_mult = if melee_ctx.stealth.noise_level == crate::player::NoiseLevel::Silent
            {
                // 射線前方找到的目標是否滿足靜默擊殺條件
                let filter = QueryFilter::default().exclude_collider(player_entity);
                if let Some((target_entity, _)) = rapier.cast_ray(
                    muzzle_pos,
                    direction,
                    weapon.stats.range as bevy_rapier3d::prelude::Real,
                    true,
                    filter,
                ) {
                    let target_unaware = melee_ctx
                        .ai_query
                        .get(target_entity)
                        .is_ok_and(AiBehavior::is_unaware);
                    let from_behind = transform_query.get(target_entity).is_ok_and(|t| {
                        is_backstab(player_pos, t.translation, t.forward().as_vec3())
                    });
                    if target_unaware && from_behind {
                        let target_pos = transform_query
                            .get(target_entity)
                            .map(|t| t.translation)
                            .unwrap_or(muzzle_pos);
                        stealth_target = Some((target_entity, target_pos));
                        STEALTH_KILL_MULTIPLIER
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            } else {
                1.0
            };

            // 隱匿擊殺：觸發專屬 takedown 流程而非直接傷害
            if let Some((target_entity, target_pos)) = stealth_target {
                melee_ctx.takedown.phase = StealthTakedownPhase::Approaching;
                melee_ctx.takedown.progress = 0.0;
                melee_ctx.takedown.target = Some(target_entity);
                melee_ctx.takedown.start_position = player_pos;
                melee_ctx.takedown.target_position = target_pos;
                melee_ctx.takedown.pending_damage = weapon.stats.damage * STEALTH_KILL_MULTIPLIER;
                weapon.reset_fire_cooldown();
                continue; // 跳過普通近戰，進入 takedown 流程
            }

            let combo_mult = melee_ctx.combo.damage_multiplier() * counter_mult * stealth_mult;
            let is_finisher = melee_ctx.combo.current_step.is_finisher() || stealth_mult > 1.0; // 靜默擊殺也觸發擊退
            let hit = fire_melee(
                &mut commands,
                player_entity,
                muzzle_pos,
                direction,
                weapon,
                &rapier,
                &mut damage_events,
                &damageable_query,
                combo_mult,
                is_finisher,
            );
            if hit {
                let current_time = time.elapsed_secs();
                melee_ctx.combo.register_hit(current_time);
                // 靜默擊殺：強化攝影機震動 + 潛行 XP 獎勵
                if stealth_mult > 1.0 {
                    camera_shake.trigger(0.08, 0.25);
                    award_stealth_kill_xp(&mut melee_ctx.skills);
                }
                // 反擊成功附加攝影機震動
                if counter_mult > 1.0 {
                    camera_shake.trigger(0.06, 0.2);
                }
                // 終結技附加攝影機震動
                if is_finisher && stealth_mult <= 1.0 {
                    camera_shake.trigger(0.04, 0.15);
                }
            }
        } else if weapon.stats.weapon_type == WeaponType::RPG {
            // RPG：生成投射物而非 hitscan
            commands.spawn((
                Transform::from_translation(muzzle_pos).looking_to(direction, Vec3::Y),
                GlobalTransform::default(),
                RpgProjectile {
                    direction,
                    speed: weapon.stats.bullet_speed,
                    direct_damage: weapon.stats.damage,
                    explosion_damage: RPG_EXPLOSION_DAMAGE,
                    explosion_radius: RPG_EXPLOSION_RADIUS,
                    lifetime: 0.0,
                    max_lifetime: RPG_MAX_LIFETIME,
                    owner: player_entity,
                },
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ));
            // 發射煙霧曳光（重用 muzzle flash）
            spawn_muzzle_flash(&mut commands, &visuals, muzzle_pos);
            apply_ranged_fire_effects(
                weapon,
                combat_state.is_aiming,
                melee_ctx.skills.recoil_multiplier(),
                &mut recoil_state,
                &mut camera_shake,
            );
        } else {
            fire_ranged_weapon(
                &mut commands,
                &visuals,
                player_entity,
                muzzle_pos,
                direction,
                weapon,
                combat_state.is_aiming,
                &rapier,
                &mut damage_events,
                &mut melee_ctx.skills,
                &damageable_query,
                &transform_query,
            );
            apply_ranged_fire_effects(
                weapon,
                combat_state.is_aiming,
                melee_ctx.skills.recoil_multiplier(),
                &mut recoil_state,
                &mut camera_shake,
            );
        }

        // 播放槍聲
        if let Some(ref sounds) = audio.weapon_sounds {
            play_weapon_fire_sound(
                &mut commands,
                sounds,
                &audio.audio_manager,
                weapon.stats.weapon_type,
            );
        }

        // 消耗彈藥並設置冷卻
        weapon.consume_ammo();
        weapon.fire_cooldown = weapon.stats.fire_rate;
        combat_state.last_shot_time = time.elapsed_secs();
        combat_state.crosshair_bloom += CROSSHAIR_BLOOM_PER_SHOT;
    }
}

// ============================================================================
// 子彈發射
// ============================================================================

/// 射擊上下文（將共用參數打包，減少參數傳遞）
struct FireContext<'a> {
    visuals: &'a CombatVisuals,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &'a Weapon,
    rapier: &'a RapierContext<'a>,
}

/// 發射子彈（使用預設隨機散佈）
fn fire_bullet(
    commands: &mut Commands,
    ctx: &FireContext,
    spread_degrees: f32,
    damage_events: &mut MessageWriter<DamageEvent>,
    skills: &mut PlayerSkills, // 用於 XP 獎勵
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) {
    let spread_rad = spread_degrees.to_radians();
    let spread_x = (rand::random::<f32>() - 0.5) * 2.0 * spread_rad;
    let spread_y = (rand::random::<f32>() - 0.5) * 2.0 * spread_rad;

    fire_bullet_with_offset(
        commands,
        ctx,
        Vec2::new(spread_x, spread_y),
        damage_events,
        skills,
        damageable_query,
        transform_query,
    );
}

/// 發射子彈（使用指定散佈偏移）
fn fire_bullet_with_offset(
    commands: &mut Commands,
    ctx: &FireContext,
    spread_offset: Vec2,
    damage_events: &mut MessageWriter<DamageEvent>,
    skills: &mut PlayerSkills, // 用於 XP 獎勵
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) {
    let right = ctx.direction.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(ctx.direction).normalize_or_zero();

    let spread_dir = (ctx.direction + right * spread_offset.x + up * spread_offset.y).normalize();

    let filter = QueryFilter::default().exclude_collider(ctx.attacker);

    // 取得武器彈道風格
    let tracer_style = ctx.weapon.stats.weapon_type.tracer_style();

    let max_toi = ctx.weapon.stats.range as bevy_rapier3d::prelude::Real;
    let penetration = ctx.weapon.stats.penetration;

    if penetration == 0 {
        // === 無穿透：使用原有的 cast_ray（效能最佳）===
        fire_bullet_single_hit(
            commands,
            ctx,
            spread_dir,
            max_toi,
            filter,
            tracer_style,
            damage_events,
            skills,
            damageable_query,
            transform_query,
        );
    } else {
        // === 有穿透：收集射線路徑上所有命中 ===
        fire_bullet_penetrating(
            commands,
            ctx,
            spread_dir,
            max_toi,
            filter,
            tracer_style,
            penetration,
            damage_events,
            skills,
            damageable_query,
            transform_query,
        );
    }
}

/// 無穿透射擊（單目標，原有邏輯）
fn fire_bullet_single_hit(
    commands: &mut Commands,
    ctx: &FireContext,
    spread_dir: Vec3,
    max_toi: bevy_rapier3d::prelude::Real,
    filter: QueryFilter,
    tracer_style: TracerStyle,
    damage_events: &mut MessageWriter<DamageEvent>,
    skills: &mut PlayerSkills, // 用於 XP 獎勵
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) {
    if let Some((hit_entity, toi)) = ctx
        .rapier
        .cast_ray(ctx.origin, spread_dir, max_toi, true, filter)
    {
        let hit_pos = ctx.origin + spread_dir * rapier_real_to_f32(toi);
        let distance = rapier_real_to_f32(toi);

        spawn_bullet_tracer(commands, ctx.visuals, ctx.origin, hit_pos, tracer_style);

        let falloff_multiplier = ctx.weapon.stats.calculate_damage_falloff(distance);

        let (final_damage, is_headshot) = calculate_hit_damage(
            ctx,
            hit_entity,
            hit_pos,
            distance,
            falloff_multiplier,
            1.0,
            damageable_query,
            transform_query,
        );

        damage_events.write(
            DamageEvent::new(hit_entity, final_damage, DamageSource::Bullet)
                .with_attacker(ctx.attacker)
                .with_position(hit_pos)
                .with_headshot(is_headshot),
        );

        // 射擊 XP 獎勵
        award_shooting_xp(skills, is_headshot);

        spawn_impact_effect(commands, ctx.visuals, hit_pos);
    } else {
        let end_pos = ctx.origin + spread_dir * ctx.weapon.stats.range;
        spawn_bullet_tracer(commands, ctx.visuals, ctx.origin, end_pos, tracer_style);
    }
}

/// 穿透射擊（多目標）
fn fire_bullet_penetrating(
    commands: &mut Commands,
    ctx: &FireContext,
    spread_dir: Vec3,
    max_toi: bevy_rapier3d::prelude::Real,
    filter: QueryFilter,
    tracer_style: TracerStyle,
    penetration: u8,
    damage_events: &mut MessageWriter<DamageEvent>,
    skills: &mut PlayerSkills, // 用於 XP 獎勵
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) {
    // 收集射線路徑上所有命中（最多 penetration + 1 個目標）
    let max_hits = (penetration as usize) + 1;
    let mut hits: Vec<(Entity, f32)> = Vec::with_capacity(max_hits);

    ctx.rapier.intersect_ray(
        ctx.origin,
        spread_dir,
        max_toi,
        true,
        filter,
        |entity, intersection| {
            hits.push((entity, rapier_real_to_f32(intersection.time_of_impact)));
            hits.len() < max_hits
        },
    );

    if hits.is_empty() {
        let end_pos = ctx.origin + spread_dir * ctx.weapon.stats.range;
        spawn_bullet_tracer(commands, ctx.visuals, ctx.origin, end_pos, tracer_style);
        return;
    }

    // 按距離排序（確保穿透順序正確）
    hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // 子彈拖尾到最遠命中點
    let Some(farthest_hit) = hits.last() else {
        return;
    };
    let farthest_hit_pos = ctx.origin + spread_dir * farthest_hit.1;
    spawn_bullet_tracer(
        commands,
        ctx.visuals,
        ctx.origin,
        farthest_hit_pos,
        tracer_style,
    );

    // 對每個命中目標造成傷害，逐層衰減
    let penetration_falloff = ctx.weapon.stats.penetration_falloff;
    for (layer, &(hit_entity, distance)) in hits.iter().enumerate() {
        let hit_pos = ctx.origin + spread_dir * distance;
        let falloff_multiplier = ctx.weapon.stats.calculate_damage_falloff(distance);
        let penetration_multiplier = penetration_falloff.powi(layer as i32);

        let (final_damage, is_headshot) = calculate_hit_damage(
            ctx,
            hit_entity,
            hit_pos,
            distance,
            falloff_multiplier,
            penetration_multiplier,
            damageable_query,
            transform_query,
        );

        damage_events.write(
            DamageEvent::new(hit_entity, final_damage, DamageSource::Bullet)
                .with_attacker(ctx.attacker)
                .with_position(hit_pos)
                .with_headshot(is_headshot),
        );

        // 射擊 XP 獎勵
        award_shooting_xp(skills, is_headshot);

        spawn_impact_effect(commands, ctx.visuals, hit_pos);
    }
}

/// 計算單次命中傷害（含距離衰減、穿透衰減、爆頭檢測）
fn calculate_hit_damage(
    ctx: &FireContext,
    hit_entity: Entity,
    hit_pos: Vec3,
    _distance: f32,
    falloff_multiplier: f32,
    penetration_multiplier: f32,
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) -> (f32, bool) {
    if damageable_query.get(hit_entity).is_ok() {
        let mut damage = ctx.weapon.stats.damage * falloff_multiplier * penetration_multiplier;

        let headshot = if let Ok(target_transform) = transform_query.get(hit_entity) {
            check_headshot(hit_pos, target_transform.translation.y)
        } else {
            false
        };

        if headshot {
            damage *= HEADSHOT_MULTIPLIER;
        }
        (damage, headshot)
    } else {
        (
            ctx.weapon.stats.damage * falloff_multiplier * penetration_multiplier,
            false,
        )
    }
}

// ============================================================================
// 隱匿擊殺系統
// ============================================================================

/// 隱匿擊殺進行系統
///
/// 依序經過三個階段（Approaching → Executing → Completing），
/// 在 Executing 階段對鎖定目標施加致命傷害。
/// 全程保持 Silent 噪音等級，不驚動周圍 NPC。
pub fn stealth_takedown_system(
    time: Res<Time>,
    mut takedown: ResMut<StealthTakedownState>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut camera_shake: ResMut<CameraShake>,
    mut skills: ResMut<PlayerSkills>,
    damageable_query: Query<Entity, With<Damageable>>,
) {
    if takedown.phase == StealthTakedownPhase::None {
        return;
    }

    let dt = time.delta_secs();
    takedown.progress += dt;

    match takedown.phase {
        StealthTakedownPhase::None => {}
        StealthTakedownPhase::Approaching => {
            if takedown.progress >= TAKEDOWN_APPROACH_DURATION {
                takedown.phase = StealthTakedownPhase::Executing;
                takedown.progress = 0.0;
            }
        }
        StealthTakedownPhase::Executing => {
            // 在 Executing 階段施加傷害（flag 確保只觸發一次）
            if !takedown.damage_applied {
                takedown.damage_applied = true;
                if let Some(target) = takedown.target {
                    if damageable_query.get(target).is_ok() {
                        let mut event =
                            DamageEvent::new(target, takedown.pending_damage, DamageSource::Melee)
                                .with_position(takedown.target_position);
                        event.force_knockback = true;
                        damage_events.write(event);
                    }
                }
                // 隱匿擊殺攝影機特寫震動
                camera_shake.trigger(0.1, 0.3);
            }
            if takedown.progress >= TAKEDOWN_EXECUTE_DURATION {
                takedown.phase = StealthTakedownPhase::Completing;
                takedown.progress = 0.0;
            }
        }
        StealthTakedownPhase::Completing => {
            // 完成階段獎勵 XP（flag 確保只觸發一次）
            if !takedown.xp_awarded {
                takedown.xp_awarded = true;
                award_stealth_kill_xp(&mut skills);
            }
            if takedown.progress >= TAKEDOWN_COMPLETE_DURATION {
                // 重置 takedown 狀態
                takedown.phase = StealthTakedownPhase::None;
                takedown.progress = 0.0;
                takedown.target = None;
                takedown.damage_applied = false;
                takedown.xp_awarded = false;
            }
        }
    }
}
