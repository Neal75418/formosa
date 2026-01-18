//! 射擊系統
//!
//! 處理射擊輸入、武器發射、子彈移動等邏輯。

use bevy::prelude::*;
use bevy::math::EulerRot;
use bevy::ecs::hierarchy::ChildOf;
use bevy_rapier3d::prelude::*;

/// 將 Rapier 的 Real 類型 (f32) 轉換為 f32
/// 注意：bevy_rapier3d 0.32 的 Real 就是 f32，所以直接返回
#[inline]
fn real_to_f32(r: bevy_rapier3d::prelude::Real) -> f32 {
    r
}

use super::components::{*, check_headshot, HEADSHOT_MULTIPLIER};
use crate::player::Player;
use crate::core::{CameraSettings, GameState, RecoilState, CameraShake};
use crate::ui::NotificationQueue;
use crate::audio::{AudioManager, WeaponSounds, play_weapon_fire_sound, play_reload_sound, play_weapon_switch_sound};
use super::RespawnState;

// === Lifetime Trait 用於統一特效消失邏輯 ===

/// 具有生命週期的組件 trait
trait HasLifetime {
    fn lifetime(&self) -> f32;
    fn lifetime_mut(&mut self) -> &mut f32;
}

impl HasLifetime for MuzzleFlash {
    fn lifetime(&self) -> f32 { self.lifetime }
    fn lifetime_mut(&mut self) -> &mut f32 { &mut self.lifetime }
}

impl HasLifetime for BulletTracer {
    fn lifetime(&self) -> f32 { self.lifetime }
    fn lifetime_mut(&mut self) -> &mut f32 { &mut self.lifetime }
}

/// 更新 lifetime 並檢查是否應該 despawn
#[inline]
fn update_lifetime_and_check_despawn<T: HasLifetime>(component: &mut T, dt: f32) -> bool {
    *component.lifetime_mut() -= dt;
    component.lifetime() <= 0.0
}

/// 射擊輸入收集系統
pub fn shooting_input_system(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut input: ResMut<ShootingInput>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    player_query: Query<&WeaponInventory, With<Player>>,
) {
    // 死亡時或在車上時不處理射擊輸入
    if respawn_state.is_dead || game_state.player_in_vehicle {
        input.fire_pressed = false;
        input.fire_held = false;
        input.aim_pressed = false;
        input.reload_pressed = false;
        input.weapon_switch = None;
        input.mouse_wheel = 0.0;
        return;
    }

    // 檢查當前武器是否為拳頭
    let is_fist = player_query
        .single()
        .ok()
        .and_then(|inv| inv.current_weapon())
        .map(|w| w.stats.weapon_type == WeaponType::Fist)
        .unwrap_or(false);

    // 射擊：R 鍵（與 UI 提示一致）
    input.fire_pressed = keyboard.just_pressed(KeyCode::KeyR);
    input.fire_held = keyboard.pressed(KeyCode::KeyR);
    // 拳頭狀態下不啟用瞄準模式（沒有準星）
    input.aim_pressed = !is_fist && mouse.pressed(MouseButton::Right);
    // 換彈：T 鍵
    input.reload_pressed = keyboard.just_pressed(KeyCode::KeyT);

    // 武器切換 (1-4 數字鍵)
    input.weapon_switch = None;
    if keyboard.just_pressed(KeyCode::Digit1) {
        input.weapon_switch = Some(1);
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        input.weapon_switch = Some(2);
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        input.weapon_switch = Some(3);
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        input.weapon_switch = Some(4);
    }

    // 滑鼠滾輪切換武器
    input.mouse_wheel = 0.0;
    for event in mouse_wheel.read() {
        input.mouse_wheel += event.y;
    }
}

/// 武器冷卻計時系統
pub fn weapon_cooldown_system(
    time: Res<Time>,
    mut player_query: Query<&mut WeaponInventory, With<Player>>,
) {
    let dt = time.delta_secs();

    for mut inventory in player_query.iter_mut() {
        if let Some(weapon) = inventory.current_weapon_mut() {
            if weapon.fire_cooldown > 0.0 {
                weapon.fire_cooldown -= dt;
            }
        }
    }
}

/// 顯示武器切換通知
fn notify_weapon_switch(notifications: &mut NotificationQueue, inventory: &WeaponInventory) {
    if let Some(weapon) = inventory.current_weapon() {
        notifications.info(format!(
            "{} {}",
            weapon.stats.weapon_type.icon(),
            weapon.stats.weapon_type.name()
        ));
    }
}

/// 處理武器切換邏輯
fn handle_weapon_switch(
    input: &ShootingInput,
    inventory: &mut WeaponInventory,
    notifications: &mut NotificationQueue,
) -> bool {
    // 數字鍵切換（1-4）
    if let Some(slot) = input.weapon_switch {
        inventory.select_weapon(slot);
        notify_weapon_switch(notifications, inventory);
        return true;
    }
    false
}

/// 檢查是否應該切換武器
#[inline]
fn should_switch_weapon(input: &ShootingInput) -> bool {
    input.weapon_switch.is_some()
}

/// 切換武器時取消換彈
fn cancel_reload_on_switch(weapon: &mut Weapon, notifications: &mut NotificationQueue) {
    if weapon.is_reloading {
        weapon.cancel_reload();
        notifications.warning("換彈取消");
    }
}

/// 換彈系統
#[allow(clippy::too_many_arguments)]
pub fn reload_system(
    time: Res<Time>,
    input: Res<ShootingInput>,
    mut commands: Commands,
    weapon_sounds: Option<Res<WeaponSounds>>,
    audio_manager: Res<AudioManager>,
    mut player_query: Query<&mut WeaponInventory, With<Player>>,
    mut notifications: ResMut<NotificationQueue>,
) {
    let dt = time.delta_secs();

    for mut inventory in player_query.iter_mut() {
        // 切換武器前取消換彈
        if should_switch_weapon(&input) {
            if let Some(weapon) = inventory.current_weapon_mut() {
                cancel_reload_on_switch(weapon, &mut notifications);
            }
            // 處理武器切換
            handle_weapon_switch(&input, &mut inventory, &mut notifications);
            // 播放武器切換音效
            if let Some(ref sounds) = weapon_sounds {
                play_weapon_switch_sound(&mut commands, sounds, &audio_manager);
            }
            // 切換後跳過當前幀，避免立即更新新武器的換彈進度
            continue;
        }

        // 處理換彈
        let Some(weapon) = inventory.current_weapon_mut() else { continue; };

        // 檢查換彈完成
        if weapon.is_reloading {
            weapon.reload_timer -= dt;
            if weapon.reload_timer <= 0.0 {
                weapon.finish_reload();
                notifications.success("換彈完成");
                // 播放換彈完成音效
                if let Some(ref sounds) = weapon_sounds {
                    play_reload_sound(&mut commands, sounds, &audio_manager, true);
                }
            }
            continue;
        }

        // 處理換彈請求
        let started_reload = if input.reload_pressed && weapon.start_reload() {
            notifications.info("換彈中...");
            true
        } else if weapon.needs_reload() && weapon.start_reload() {
            notifications.warning("彈匣空了！換彈中...");
            true
        } else {
            false
        };

        // 播放換彈開始音效
        if started_reload {
            if let Some(ref sounds) = weapon_sounds {
                play_reload_sound(&mut commands, sounds, &audio_manager, false);
            }
        }
    }
}

// === 射擊系統輔助函數 ===

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
    ).normalize();

    // 計算攝影機位置（玩家後上方）
    let player_center = player_pos + Vec3::Y * 1.5;
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
        camera_pos + cam_forward * real_to_f32(toi)
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
    let base_pos = player_pos + char_vecs.up * 0.55;

    if weapon_type == WeaponType::Fist {
        // 拳頭：從手的位置出發
        base_pos + char_vecs.right * 0.25 + char_vecs.forward * 0.3
    } else if is_aiming {
        // 瞄準姿勢：槍口在身體前方中央偏右
        let hand_pos = base_pos
            + char_vecs.right * 0.15
            + char_vecs.forward * 0.45;
        hand_pos + char_vecs.forward * muzzle_offset.z + char_vecs.up * muzzle_offset.y
    } else {
        // 待機持槍姿勢：槍口朝下前方
        let hand_pos = base_pos
            + char_vecs.right * 0.22
            + char_vecs.forward * 0.25
            + char_vecs.up * (-0.1);
        let tilted_forward = (char_vecs.forward * 0.8 + char_vecs.up * (-0.2)).normalize();
        hand_pos + tilted_forward * muzzle_offset.z * 0.8
    }
}

/// 檢查武器是否應該發射
#[inline]
fn should_fire(input: &ShootingInput, weapon: &Weapon) -> bool {
    let trigger_pressed = if weapon.stats.is_automatic {
        input.fire_held
    } else {
        input.fire_pressed
    };
    trigger_pressed && weapon.can_fire()
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
    let inner_count = remaining.min(6);  // 內圈最多 6 顆
    let outer_count = remaining.saturating_sub(6);  // 剩餘的放外圈

    // 內圈（較準確，50% 散佈半徑）
    let inner_radius = spread_rad * 0.5;
    for i in 0..inner_count {
        let angle = (i as f32 / inner_count as f32) * TAU;
        // 加入少量隨機偏移使其更自然
        let jitter = (rand::random::<f32>() - 0.5) * 0.1;
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
        let jitter = (rand::random::<f32>() - 0.5) * 0.15;
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
        weapon.stats.spread * 0.5
    } else {
        weapon.stats.spread
    };

    // 霰彈槍使用環形散佈模式
    if weapon.stats.weapon_type == WeaponType::Shotgun {
        let pattern = generate_shotgun_pattern(weapon.stats.pellet_count, spread);
        for offset in pattern {
            fire_bullet_with_offset(
                commands,
                visuals,
                attacker,
                muzzle_pos,
                direction,
                weapon,
                offset,
                rapier,
                damage_events,
                damageable_query,
                transform_query,
            );
        }
    } else {
        // 其他武器使用隨機散佈
        for _ in 0..weapon.stats.pellet_count {
            fire_bullet(
                commands,
                visuals,
                attacker,
                muzzle_pos,
                direction,
                weapon,
                spread,
                rapier,
                damage_events,
                damageable_query,
                transform_query,
            );
        }
    }

    spawn_muzzle_flash(commands, visuals, muzzle_pos);
}

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
    let Some(visuals) = combat_visuals else { return; };
    let Ok(rapier) = rapier_context.single() else { return; };

    for (player_entity, player_transform, mut inventory) in player_query.iter_mut() {
        let Some(weapon) = inventory.current_weapon_mut() else { continue; };

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
        if weapon.stats.weapon_type == WeaponType::Fist {
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

            // 添加後座力（瞄準時減半）
            let recoil_mult = if combat_state.is_aiming { 0.5 } else { 1.0 };
            recoil_state.add_recoil(
                weapon.stats.recoil_vertical * recoil_mult,
                weapon.stats.recoil_horizontal * recoil_mult,
            );

            // 觸發攝影機震動（根據武器類型調整強度）
            let shake_intensity = match weapon.stats.weapon_type {
                WeaponType::Pistol => 0.02,
                WeaponType::SMG => 0.015,
                WeaponType::Shotgun => 0.05,
                WeaponType::Rifle => 0.025,
                WeaponType::Fist => 0.0,
            };
            if shake_intensity > 0.0 {
                camera_shake.trigger(shake_intensity, 0.08);
            }
        }

        // 播放槍聲
        if let Some(ref sounds) = weapon_sounds {
            play_weapon_fire_sound(&mut commands, sounds, &audio_manager, weapon.stats.weapon_type);
        }

        // 消耗彈藥並設置冷卻
        weapon.consume_ammo();
        weapon.fire_cooldown = weapon.stats.fire_rate;
        combat_state.last_shot_time = time.elapsed_secs();
        combat_state.crosshair_bloom += 0.2;
    }
}

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

    // 近戰範圍檢測
    if let Some((hit_entity, toi)) = rapier.cast_ray(
        origin,
        direction,
        weapon.stats.range as bevy_rapier3d::prelude::Real,
        true,
        filter,
    ) {
        let hit_pos = origin + direction * real_to_f32(toi);
        // 對所有命中目標發送傷害事件（讓接收系統自行過濾）
        // damageable_query 參數保留以供未來使用或移除
        let _ = damageable_query;
        damage_events.write(
            DamageEvent::new(hit_entity, weapon.stats.damage, DamageSource::Melee)
                .with_attacker(attacker)
                .with_position(hit_pos),
        );
    }

    // 近戰視覺效果（可選）
    let _ = commands;
}

/// 發射子彈（使用預設隨機散佈）
#[allow(clippy::too_many_arguments)]
fn fire_bullet(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    spread_degrees: f32,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) {
    // 計算隨機散射偏移
    let spread_rad = spread_degrees.to_radians();
    let spread_x = (rand::random::<f32>() - 0.5) * spread_rad;
    let spread_y = (rand::random::<f32>() - 0.5) * spread_rad;

    fire_bullet_with_offset(
        commands,
        visuals,
        attacker,
        origin,
        direction,
        weapon,
        Vec2::new(spread_x, spread_y),
        rapier,
        damage_events,
        damageable_query,
        transform_query,
    );
}

/// 發射子彈（使用指定散佈偏移）
#[allow(clippy::too_many_arguments)]
fn fire_bullet_with_offset(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    spread_offset: Vec2,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    transform_query: &Query<&Transform>,
) {
    let right = direction.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(direction).normalize_or_zero();

    let spread_dir = (direction + right * spread_offset.x + up * spread_offset.y).normalize();

    let filter = QueryFilter::default().exclude_collider(attacker);

    // 取得武器彈道風格
    let tracer_style = weapon.stats.weapon_type.tracer_style();

    // 使用 Raycast 檢測命中
    if let Some((hit_entity, toi)) = rapier.cast_ray(
        origin,
        spread_dir,
        weapon.stats.range as bevy_rapier3d::prelude::Real,
        true,
        filter,
    ) {
        let hit_pos = origin + spread_dir * real_to_f32(toi);
        let distance = real_to_f32(toi);

        // 生成子彈拖尾（使用武器專屬風格）
        spawn_bullet_tracer(commands, visuals, origin, hit_pos, tracer_style);

        // 計算距離傷害衰減（對所有目標都適用）
        let falloff_multiplier = weapon.stats.calculate_damage_falloff(distance);

        // 計算最終傷害和爆頭（僅對可受傷實體進行爆頭檢測）
        let (final_damage, is_headshot) = if damageable_query.get(hit_entity).is_ok() {
            let mut damage = weapon.stats.damage * falloff_multiplier;

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
            (weapon.stats.damage * falloff_multiplier, false)
        };

        // 對所有命中目標發送傷害事件（讓接收系統自行過濾）
        damage_events.write(
            DamageEvent::new(hit_entity, final_damage, DamageSource::Bullet)
                .with_attacker(attacker)
                .with_position(hit_pos)
                .with_headshot(is_headshot),
        );

        // 生成命中效果（火花）
        spawn_impact_effect(commands, visuals, hit_pos);
    } else {
        // 未命中，子彈飛到最大距離
        let end_pos = origin + spread_dir * weapon.stats.range;
        spawn_bullet_tracer(commands, visuals, origin, end_pos, tracer_style);
    }
}

/// 生成子彈拖尾效果（根據武器類型使用不同風格）
/// 公開供 AI 系統使用
pub fn spawn_bullet_tracer(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    start: Vec3,
    end: Vec3,
    style: TracerStyle,
) {
    // 無彈道風格（近戰）則不生成
    if style == TracerStyle::None {
        return;
    }

    let direction = end - start;
    let length = direction.length();

    if length < 0.1 {
        return;
    }

    // 取得對應風格的彈道配置
    let Some(config) = visuals.get_tracer(style) else {
        return;
    };

    let mid = (start + end) / 2.0;
    let rotation = Quat::from_rotation_arc(Vec3::Y, direction.normalize());

    // 使用武器專屬的 mesh 和 material
    commands.spawn((
        Mesh3d(config.mesh.clone()),
        MeshMaterial3d(config.material.clone()),
        Transform::from_translation(mid)
            .with_rotation(rotation)
            .with_scale(Vec3::new(config.thickness, length, config.thickness)),
        BulletTracer {
            start_pos: start,
            end_pos: end,
            lifetime: config.lifetime,
        },
    ));
}

/// 生成槍口閃光
fn spawn_muzzle_flash(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    position: Vec3,
) {
    // 共用 mesh 和 material
    commands.spawn((
        Mesh3d(visuals.muzzle_mesh.clone()),
        MeshMaterial3d(visuals.muzzle_material.clone()),
        Transform::from_translation(position),
        MuzzleFlash { lifetime: 0.05 },
    ));
}

/// 生成擊中特效（火花）
fn spawn_impact_effect(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    position: Vec3,
) {
    let lifetime = 0.15;
    commands.spawn((
        Mesh3d(visuals.impact_mesh.clone()),
        MeshMaterial3d(visuals.impact_material.clone()),
        Transform::from_translation(position),
        ImpactEffect {
            lifetime,
            max_lifetime: lifetime,
        },
    ));
}

/// 槍口閃光消失系統
pub fn muzzle_flash_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut MuzzleFlash)>,
) {
    let dt = time.delta_secs();
    for (entity, mut flash) in query.iter_mut() {
        if update_lifetime_and_check_despawn(&mut *flash, dt) {
            commands.entity(entity).despawn();
        }
    }
}

/// 子彈拖尾消失系統
pub fn bullet_tracer_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut BulletTracer)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tracer) in query.iter_mut() {
        if update_lifetime_and_check_despawn(&mut *tracer, dt) {
            commands.entity(entity).despawn();
        }
    }
}

/// 擊中特效消失系統（含縮放動畫）
pub fn impact_effect_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut ImpactEffect, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (entity, mut effect, mut transform) in query.iter_mut() {
        effect.lifetime -= dt;

        // 縮放動畫：先快速膨脹，再慢慢消失
        let progress = if effect.max_lifetime > 0.0 {
            (1.0 - effect.lifetime / effect.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0 // 預設為已完成
        };
        let scale = if progress < 0.3 {
            // 前 30%：快速膨脹到 1.5 倍
            1.0 + progress * 1.67
        } else {
            // 後 70%：從 1.5 倍縮小消失
            let shrink_progress = (progress - 0.3) / 0.7;
            1.5 * (1.0 - shrink_progress)
        };
        transform.scale = Vec3::splat(scale.max(0.0));

        if effect.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// 揮拳動畫系統
// ============================================================================

/// 揮拳動畫觸發系統
/// 當使用拳頭攻擊時，為右臂添加 PunchAnimation 組件
pub fn punch_animation_trigger_system(
    mut commands: Commands,
    input: Res<ShootingInput>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    player_query: Query<(&WeaponInventory, &Children), With<Player>>,
    arm_query: Query<(Entity, &PlayerArm), Without<PunchAnimation>>,
) {
    // 死亡或在車上時不觸發
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    // 檢查是否按下攻擊鍵
    if !input.fire_pressed {
        return;
    }

    // 檢查玩家當前武器是否是拳頭
    let Ok((inventory, children)) = player_query.single() else {
        return;
    };

    let Some(weapon) = inventory.current_weapon() else {
        return;
    };

    if weapon.stats.weapon_type != WeaponType::Fist {
        return;
    }

    // 檢查是否可以攻擊（冷卻時間）
    if !weapon.can_fire() {
        return;
    }

    // 找到右手臂並添加揮拳動畫
    for child in children.iter() {
        if let Ok((arm_entity, arm)) = arm_query.get(child) {
            if arm.is_right {
                commands.entity(arm_entity).insert(PunchAnimation::default());
                break;
            }
        }
    }
}

/// 揮拳動畫更新系統
/// 處理手臂動畫的三個階段：WindUp → Strike → Return
/// 模擬自然的上勾拳：手臂從下方沿弧線向上揮出
pub fn punch_animation_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut arm_query: Query<(Entity, &PlayerArm, &mut Transform, &mut PunchAnimation)>,
) {
    let dt = time.delta_secs();

    for (entity, arm, mut transform, mut anim) in arm_query.iter_mut() {
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

        // 只處理右手臂的動畫
        if !arm.is_right {
            continue;
        }

        // 自然上勾拳動畫
        // 手臂從下方沿弧線向上+向前揮出
        match anim.phase {
            PunchPhase::WindUp => {
                // 蓄力：手臂向下、向後、向外側收（準備上勾）
                let phase_progress = t / wind_up_end;
                let ease = ease_out_quad(phase_progress);

                // 手臂向下+向後+向外側
                let rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    0.5 * ease,       // X: 手臂向下/向後（正值）
                    -0.3 * ease,      // Y: 向外側旋轉
                    0.2 * ease        // Z: 稍微內傾
                );

                // 位置往下、往後、往外收
                let offset = Vec3::new(
                    -0.08 * ease,   // X: 往外側
                    -0.12 * ease,   // Y: 向下沉（蓄力）
                    -0.1 * ease     // Z: 往後
                );

                transform.translation = arm.rest_position + offset;
                transform.rotation = rotation;
            }
            PunchPhase::Strike => {
                // 出拳：從下方沿弧線向上+向前揮出（上勾拳）
                let phase_t = t - wind_up_end;
                let phase_duration = strike_end - wind_up_end;
                let phase_progress = phase_t / phase_duration;
                let ease = ease_out_cubic(phase_progress);

                // 旋轉：從下方（0.5）揮到上方（-1.0）
                // X 軸：正值=向下，負值=向上
                let start_x = 0.5;
                let end_x = -1.0;
                let current_x = start_x + (end_x - start_x) * ease;

                // Y 軸：從外側（-0.3）揮到前方（+0.2），創造弧線
                let start_y = -0.3;
                let end_y = 0.2;
                let current_y = start_y + (end_y - start_y) * ease;

                let rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    current_x,                 // X: 從下往上揮
                    current_y,                 // Y: 從外側到前方（弧線）
                    0.2 * (1.0 - ease)         // Z: 從傾斜到直
                );

                // 位置：弧線軌跡（從下後外 → 上前中）
                let arc = (phase_progress * std::f32::consts::PI).sin();
                let offset = Vec3::new(
                    -0.08 + 0.13 * ease + 0.05 * arc,  // X: 從外側繞回中間
                    -0.12 + 0.42 * ease,               // Y: 從下往上（-0.12 → +0.3）
                    -0.1 + 0.4 * ease                  // Z: 向前伸出
                );

                transform.translation = arm.rest_position + offset;
                transform.rotation = rotation;
            }
            PunchPhase::Return => {
                // 收回：快速回到原位
                let phase_t = t - strike_end;
                let phase_duration = duration - strike_end;
                let phase_progress = phase_t / phase_duration;
                let ease = ease_in_out_quad(phase_progress);

                // 從上勾拳終點插值回原位
                let strike_rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    -1.0,   // 手臂向上的終點
                    0.2,    // 在前方
                    0.0
                );
                let strike_offset = Vec3::new(0.05, 0.3, 0.3);

                transform.translation = (arm.rest_position + strike_offset).lerp(arm.rest_position, ease);
                transform.rotation = strike_rotation.slerp(arm.rest_rotation, ease);
            }
        }

        // 動畫結束，移除組件
        if anim.is_finished() {
            // 確保回到原位
            transform.translation = arm.rest_position;
            transform.rotation = arm.rest_rotation;
            commands.entity(entity).remove::<PunchAnimation>();
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
// 武器模型系統
// ============================================================================

/// 為玩家生成所有武器模型（附加到右手）
/// 這是一個一次性系統，在玩家生成後執行
pub fn spawn_player_weapons(
    mut commands: Commands,
    weapon_visuals: Option<Res<WeaponVisuals>>,
    hand_query: Query<(Entity, &PlayerHand), Added<PlayerHand>>,
) {
    let Some(visuals) = weapon_visuals else { return; };

    for (hand_entity, hand) in hand_query.iter() {
        // 只為右手生成武器
        if !hand.is_right {
            continue;
        }

        // 為每種武器類型生成模型
        // 使用 ChildOf 直接設定父子關係，可能避免 B0004 警告
        for weapon_type in [WeaponType::Pistol, WeaponType::SMG, WeaponType::Shotgun, WeaponType::Rifle] {
            let Some(weapon_data) = visuals.get(weapon_type) else { continue; };

            // 先生成武器根實體，使用 ChildOf 設定父實體
            let weapon_root = commands.spawn((
                Transform::from_translation(weapon_data.hand_offset)
                    .with_rotation(weapon_data.hand_rotation),
                GlobalTransform::default(),
                Visibility::Hidden,  // 預設隱藏
                InheritedVisibility::default(),
                ViewVisibility::default(),
                WeaponModel { weapon_type },
                Name::new(format!("Weapon_{:?}", weapon_type)),
                ChildOf(hand_entity),  // 直接設定父實體
            )).id();

            // 生成武器部件作為武器根的子實體
            for part in &weapon_data.parts {
                commands.spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(part.material.clone()),
                    part.transform,
                    GlobalTransform::default(),
                    ChildOf(weapon_root),  // 直接設定父實體
                ));
            }
        }
    }
}

/// 根據當前武器更新武器模型可見性
pub fn weapon_visibility_system(
    player_query: Query<&WeaponInventory, (With<Player>, Changed<WeaponInventory>)>,
    mut weapon_model_query: Query<(&WeaponModel, &mut Visibility)>,
) {
    let Ok(inventory) = player_query.single() else { return; };
    let Some(current_weapon) = inventory.current_weapon() else { return; };
    let current_type = current_weapon.stats.weapon_type;

    for (model, mut visibility) in weapon_model_query.iter_mut() {
        *visibility = if model.weapon_type == current_type {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// 強制更新武器可見性（用於初始化）
pub fn weapon_visibility_init_system(
    player_query: Query<&WeaponInventory, With<Player>>,
    mut weapon_model_query: Query<(&WeaponModel, &mut Visibility)>,
    mut initialized: Local<bool>,
) {
    if *initialized {
        return;
    }

    let Ok(inventory) = player_query.single() else { return; };
    let Some(current_weapon) = inventory.current_weapon() else { return; };
    let current_type = current_weapon.stats.weapon_type;

    let mut found_any = false;
    for (model, mut visibility) in weapon_model_query.iter_mut() {
        found_any = true;
        *visibility = if model.weapon_type == current_type {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if found_any {
        *initialized = true;
    }
}

/// 持槍姿勢系統 - 當持有槍械時調整手臂位置
pub fn holding_pose_system(
    player_query: Query<(&WeaponInventory, &Children), With<Player>>,
    mut arm_query: Query<(&PlayerArm, &mut Transform), Without<PunchAnimation>>,
    input: Res<ShootingInput>,
) {
    let Ok((inventory, children)) = player_query.single() else { return; };
    let Some(weapon) = inventory.current_weapon() else { return; };

    // 使用 ShootingInput 中的 aim_pressed，確保系統順序正確
    let is_aiming = input.aim_pressed;
    let is_fist = weapon.stats.weapon_type == WeaponType::Fist;

    for child in children.iter() {
        let Ok((arm, mut transform)) = arm_query.get_mut(child) else { continue; };

        if arm.is_right {
            // 右手臂 - 主要持槍手
            if is_fist {
                // 拳頭模式：恢復原位
                transform.translation = arm.rest_position;
                transform.rotation = arm.rest_rotation;
            } else if is_aiming {
                // 瞄準姿勢：手臂向前伸直，抬槍瞄準
                let aim_rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    1.4,   // X: 接近水平（稍微抬起）
                    0.0,
                    0.0    // Z: 直的
                );
                let aim_offset = Vec3::new(
                    -0.05,  // 往身體中心靠一點
                    -0.15,  // 手臂水平後的高度調整
                    0.35    // 向前伸
                );
                transform.translation = arm.rest_position + aim_offset;
                transform.rotation = aim_rotation;
            } else {
                // 待機持槍姿勢：手臂微彎，槍口朝下前方
                let hold_rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    0.8,   // X: 稍微向前（約45度）
                    0.0,
                    0.1    // Z: 稍微外傾
                );
                let hold_offset = Vec3::new(
                    -0.02,  // 往身體靠一點
                    -0.08,  // 稍微下降
                    0.12    // 稍微向前
                );
                transform.translation = arm.rest_position + hold_offset;
                transform.rotation = hold_rotation;
            }
        } else {
            // 左手臂 - 長槍支撐手或自然垂放
            let needs_two_hands = matches!(
                weapon.stats.weapon_type,
                WeaponType::SMG | WeaponType::Shotgun | WeaponType::Rifle
            );

            if needs_two_hands && is_aiming {
                // 雙手持槍瞄準姿勢：左手支撐護木
                let support_rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    1.3,    // X: 接近水平
                    0.0,
                    -0.1    // Z: 稍微內傾
                );
                let support_offset = Vec3::new(
                    -0.12,  // 往中間移動
                    -0.12,  // 手臂水平後的高度調整
                    0.38    // 向前伸（比右手更前，支撐護木）
                );
                transform.translation = arm.rest_position + support_offset;
                transform.rotation = support_rotation;
            } else {
                // 非瞄準、單手武器、或拳頭：左手恢復原位
                transform.translation = arm.rest_position;
                transform.rotation = arm.rest_rotation;
            }
        }
    }
}
