//! 武器發射系統
//!
//! 處理武器發射邏輯：瞄準計算、彈道射線、近戰攻擊、霰彈散佈。

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::effects::{spawn_bullet_tracer, spawn_impact_effect, spawn_muzzle_flash};
use crate::combat::components::*;
use crate::combat::health::{
    check_headshot, BleedEffect, DamageEvent, DamageSource, Damageable, BLEED_CHANCE,
    HEADSHOT_MULTIPLIER,
};
use crate::combat::visuals::*;
use crate::combat::weapon::*;
use crate::audio::{play_weapon_fire_sound, AudioManager, WeaponSounds};
use crate::core::{rapier_real_to_f32, CameraSettings, CameraShake, RecoilState};
use crate::player::Player;

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

// --- 近戰 ---
/// 棍棒掃擊檢測步數
const STAFF_SWEEP_STEPS: usize = 5;

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

    // 從攝影機發射 raycast 找到瞄準點
    const MAX_AIM_DISTANCE: f32 = 500.0;
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
        WeaponType::Fist | WeaponType::Staff | WeaponType::Knife => 0.0,
    }
}

/// 應用遠程武器發射後效果（後座力、攝影機震動）
fn apply_ranged_fire_effects(
    weapon: &Weapon,
    is_aiming: bool,
    recoil_state: &mut RecoilState,
    camera_shake: &mut CameraShake,
) {
    let recoil_mult = if is_aiming { AIM_RECOIL_MULTIPLIER } else { 1.0 };
    recoil_state.add_recoil(
        weapon.stats.recoil_vertical * recoil_mult,
        weapon.stats.recoil_horizontal * recoil_mult,
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
            fire_bullet_with_offset(commands, &ctx, offset, damage_events, damageable_query, transform_query);
        }
    } else {
        // 其他武器使用隨機散佈
        for _ in 0..weapon.stats.pellet_count {
            fire_bullet(commands, &ctx, spread, damage_events, damageable_query, transform_query);
        }
    }

    spawn_muzzle_flash(commands, visuals, muzzle_pos);
}

// ============================================================================
// 發射武器主系統
// ============================================================================

/// 發射武器系統
#[allow(clippy::too_many_arguments)]
pub fn fire_weapon_system(
    input: Res<ShootingInput>,
    time: Res<Time>,
    camera_settings: Res<CameraSettings>,
    rapier_context: ReadRapierContext,
    mut commands: Commands,
    combat_visuals: Option<Res<CombatVisuals>>,
    weapon_visuals: Option<Res<WeaponVisuals>>,
    weapon_sounds: Option<Res<WeaponSounds>>,
    audio_manager: Res<AudioManager>,
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

    for (player_entity, player_transform, mut inventory) in player_query.iter_mut() {
        let Some(weapon) = inventory.current_weapon_mut() else {
            continue;
        };

        if !should_fire(&input, weapon) {
            continue;
        }

        let player_pos = player_transform.translation;
        let char_vecs = CharacterVectors::from_yaw(camera_settings.yaw);

        // 計算瞄準點和槍口位置
        let aim_point = calculate_aim_point(&camera_settings, player_pos, &rapier);

        let muzzle_offset = weapon_visuals
            .as_ref()
            .and_then(|wv| wv.get(weapon.stats.weapon_type))
            .map(|wd| wd.muzzle_offset)
            .unwrap_or(Vec3::new(0.0, 0.0, 0.15));

        let muzzle_pos = calculate_muzzle_position(
            player_pos,
            &char_vecs,
            weapon.stats.weapon_type,
            combat_state.is_aiming,
            muzzle_offset,
        );

        let direction = (aim_point - muzzle_pos).normalize();

        // 根據武器類型發射
        if weapon.stats.weapon_type.is_melee() {
            fire_melee(
                &mut commands,
                player_entity,
                muzzle_pos,
                direction,
                weapon,
                &rapier,
                &mut damage_events,
                &damageable_query,
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
                &damageable_query,
                &transform_query,
            );
            apply_ranged_fire_effects(
                weapon,
                combat_state.is_aiming,
                &mut recoil_state,
                &mut camera_shake,
            );
        }

        // 播放槍聲
        if let Some(ref sounds) = weapon_sounds {
            play_weapon_fire_sound(
                &mut commands,
                sounds,
                &audio_manager,
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
// 近戰攻擊
// ============================================================================

/// 近戰攻擊
#[allow(clippy::too_many_arguments)]
fn fire_melee(
    commands: &mut Commands,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
) {
    let filter = QueryFilter::default().exclude_collider(attacker);

    match weapon.stats.weapon_type {
        WeaponType::Staff => {
            // 棍棒：弧形掃擊，可命中多個目標
            fire_staff_sweep(
                commands,
                attacker,
                origin,
                direction,
                weapon,
                rapier,
                damage_events,
                filter,
            );
        }
        WeaponType::Knife => {
            // 刀：單目標，有機率觸發流血
            fire_knife_attack(
                commands,
                attacker,
                origin,
                direction,
                weapon,
                rapier,
                damage_events,
                filter,
            );
        }
        _ => {
            // 拳頭或其他近戰：單目標直線攻擊
            if let Some((hit_entity, toi)) = rapier.cast_ray(
                origin,
                direction,
                weapon.stats.range as bevy_rapier3d::prelude::Real,
                true,
                filter,
            ) {
                let hit_pos = origin + direction * rapier_real_to_f32(toi);
                damage_events.write(
                    DamageEvent::new(hit_entity, weapon.stats.damage, DamageSource::Melee)
                        .with_attacker(attacker)
                        .with_position(hit_pos),
                );
            }
        }
    }

    let _ = damageable_query; // 保留參數以供未來使用
}

/// 棍棒弧形掃擊攻擊
fn fire_staff_sweep(
    _commands: &mut Commands,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    filter: QueryFilter,
) {
    let sweep_angle = weapon.stats.spread.to_radians(); // 使用 spread 作為掃擊角度
    let mut hit_entities: Vec<Entity> = Vec::new();

    // 在弧形範圍內進行多次射線檢測
    for i in 0..STAFF_SWEEP_STEPS {
        let t = i as f32 / (STAFF_SWEEP_STEPS - 1) as f32;
        let angle = -sweep_angle / 2.0 + t * sweep_angle;

        // 繞 Y 軸旋轉方向向量
        let rotated_dir = Quat::from_rotation_y(angle) * direction;

        if let Some((hit_entity, toi)) = rapier.cast_ray(
            origin,
            rotated_dir,
            weapon.stats.range as bevy_rapier3d::prelude::Real,
            true,
            filter,
        ) {
            // 避免對同一目標重複造成傷害
            if !hit_entities.contains(&hit_entity) {
                hit_entities.push(hit_entity);

                let hit_pos = origin + rotated_dir * rapier_real_to_f32(toi);
                damage_events.write(
                    DamageEvent::new(hit_entity, weapon.stats.damage, DamageSource::Melee)
                        .with_attacker(attacker)
                        .with_position(hit_pos),
                );
            }
        }
    }
}

/// 刀攻擊（有流血效果）
fn fire_knife_attack(
    commands: &mut Commands,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    filter: QueryFilter,
) {
    if let Some((hit_entity, toi)) = rapier.cast_ray(
        origin,
        direction,
        weapon.stats.range as bevy_rapier3d::prelude::Real,
        true,
        filter,
    ) {
        let hit_pos = origin + direction * rapier_real_to_f32(toi);

        // 發送傷害事件
        damage_events.write(
            DamageEvent::new(hit_entity, weapon.stats.damage, DamageSource::Melee)
                .with_attacker(attacker)
                .with_position(hit_pos),
        );

        // 機率觸發流血效果
        if rand::random::<f32>() < BLEED_CHANCE {
            commands
                .entity(hit_entity)
                .insert(BleedEffect::new(attacker));
        }
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
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) {
    let right = ctx.direction.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(ctx.direction).normalize_or_zero();

    let spread_dir = (ctx.direction + right * spread_offset.x + up * spread_offset.y).normalize();

    let filter = QueryFilter::default().exclude_collider(ctx.attacker);

    // 取得武器彈道風格
    let tracer_style = ctx.weapon.stats.weapon_type.tracer_style();

    // 使用 Raycast 檢測命中
    if let Some((hit_entity, toi)) = ctx.rapier.cast_ray(
        ctx.origin,
        spread_dir,
        ctx.weapon.stats.range as bevy_rapier3d::prelude::Real,
        true,
        filter,
    ) {
        let hit_pos = ctx.origin + spread_dir * rapier_real_to_f32(toi);
        let distance = rapier_real_to_f32(toi);

        // 生成子彈拖尾（使用武器專屬風格）
        spawn_bullet_tracer(commands, ctx.visuals, ctx.origin, hit_pos, tracer_style);

        // 計算距離傷害衰減（對所有目標都適用）
        let falloff_multiplier = ctx.weapon.stats.calculate_damage_falloff(distance);

        // 計算最終傷害和爆頭（僅對可受傷實體進行爆頭檢測）
        let (final_damage, is_headshot) = if damageable_query.get(hit_entity).is_ok() {
            let mut damage = ctx.weapon.stats.damage * falloff_multiplier;

            // 檢查爆頭
            let headshot = if let Ok(target_transform) = transform_query.get(hit_entity) {
                let target_base_y = target_transform.translation.y;
                check_headshot(hit_pos, target_base_y)
            } else {
                false
            };

            // 爆頭加成
            if headshot {
                damage *= HEADSHOT_MULTIPLIER;
            }
            (damage, headshot)
        } else {
            // 對可破壞物件等其他目標，使用基礎傷害（無爆頭）
            (ctx.weapon.stats.damage * falloff_multiplier, false)
        };

        // 對所有命中目標發送傷害事件（讓接收系統自行過濾）
        damage_events.write(
            DamageEvent::new(hit_entity, final_damage, DamageSource::Bullet)
                .with_attacker(ctx.attacker)
                .with_position(hit_pos)
                .with_headshot(is_headshot),
        );

        // 生成命中效果（火花）
        spawn_impact_effect(commands, ctx.visuals, hit_pos);
    } else {
        // 未命中，子彈飛到最大距離
        let end_pos = ctx.origin + spread_dir * ctx.weapon.stats.range;
        spawn_bullet_tracer(commands, ctx.visuals, ctx.origin, end_pos, tracer_style);
    }
}
