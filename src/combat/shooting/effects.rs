//! 射擊視覺效果系統
//!
//! 槍口閃光、彈道軌跡、擊中特效、揮拳動畫、武器模型、流血傷害、持槍姿勢。

use bevy::ecs::hierarchy::ChildOf;
use bevy::math::EulerRot;
use bevy::prelude::*;

use crate::combat::components::*;
use crate::combat::health::{BleedEffect, DamageEvent, DamageSource};
use crate::combat::visuals::*;
use crate::combat::weapon::*;
use crate::core::{ease_in_out_quad, ease_out_cubic, ease_out_quad, GameState};
use crate::player::Player;

// ============================================================================
// 彈道視覺常數
// ============================================================================
/// 最小彈道軌跡長度（低於此值不生成拖尾）
const MIN_TRACER_LENGTH: f32 = 0.1;
/// 槍口閃光存活時間（秒）
const MUZZLE_FLASH_LIFETIME: f32 = 0.05;
/// 擊中特效存活時間（秒）
const IMPACT_EFFECT_LIFETIME: f32 = 0.15;

// --- 擊中特效動畫 ---
/// 擊中特效膨脹階段比例（前 30%）
const IMPACT_EXPAND_PHASE: f32 = 0.3;
/// 擊中特效膨脹速率
const IMPACT_EXPAND_RATE: f32 = 1.67;
/// 擊中特效最大縮放
const IMPACT_MAX_SCALE: f32 = 1.5;
/// 擊中特效縮小階段比例（後 70%）
const IMPACT_SHRINK_PHASE: f32 = 0.7;

// ============================================================================
// Lifetime Trait 用於統一特效消失邏輯
// ============================================================================

/// 具有生命週期的組件 trait
trait HasLifetime {
    fn lifetime(&self) -> f32;
    fn lifetime_mut(&mut self) -> &mut f32;
}

impl HasLifetime for MuzzleFlash {
    fn lifetime(&self) -> f32 {
        self.lifetime
    }
    fn lifetime_mut(&mut self) -> &mut f32 {
        &mut self.lifetime
    }
}

impl HasLifetime for BulletTracer {
    fn lifetime(&self) -> f32 {
        self.lifetime
    }
    fn lifetime_mut(&mut self) -> &mut f32 {
        &mut self.lifetime
    }
}

/// 更新 lifetime 並檢查是否應該 despawn
#[inline]
fn update_lifetime_and_check_despawn<T: HasLifetime>(component: &mut T, dt: f32) -> bool {
    *component.lifetime_mut() -= dt;
    component.lifetime() <= 0.0
}

// ============================================================================
// 彈道與槍口特效
// ============================================================================

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

    if length < MIN_TRACER_LENGTH {
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

/// 生成槍口閃光（公開供其他模組使用）
pub fn spawn_muzzle_flash(commands: &mut Commands, visuals: &CombatVisuals, position: Vec3) {
    // 共用 mesh 和 material
    commands.spawn((
        Mesh3d(visuals.muzzle_mesh.clone()),
        MeshMaterial3d(visuals.muzzle_material.clone()),
        Transform::from_translation(position),
        MuzzleFlash { lifetime: MUZZLE_FLASH_LIFETIME },
    ));
}

/// 生成擊中特效（火花）
pub(super) fn spawn_impact_effect(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    position: Vec3,
) {
    let lifetime = IMPACT_EFFECT_LIFETIME;
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
        let scale = if progress < IMPACT_EXPAND_PHASE {
            // 前 30%：快速膨脹到 1.5 倍
            1.0 + progress * IMPACT_EXPAND_RATE
        } else {
            // 後 70%：從 1.5 倍縮小消失
            let shrink_progress = (progress - IMPACT_EXPAND_PHASE) / IMPACT_SHRINK_PHASE;
            IMPACT_MAX_SCALE * (1.0 - shrink_progress)
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
pub fn punch_animation_trigger_system(
    mut commands: Commands,
    input: Res<ShootingInput>,
    game_state: Res<GameState>,
    respawn_state: Res<crate::combat::RespawnState>,
    player_query: Query<(&WeaponInventory, &Children), With<Player>>,
    arm_query: Query<(Entity, &PlayerArm), Without<PunchAnimation>>,
) {
    // 死亡或在車上時不觸發
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    // 檢查是否按下攻擊鍵
    if !input.is_fire_pressed {
        return;
    }

    // 檢查玩家當前武器是否是近戰武器
    let Ok((inventory, children)) = player_query.single() else {
        return;
    };

    let Some(weapon) = inventory.current_weapon() else {
        return;
    };

    if !weapon.stats.weapon_type.is_melee() {
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
                commands
                    .entity(arm_entity)
                    .insert(PunchAnimation::default());
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

        // 更新階段
        anim.update_phase();

        let (wind_up_end, strike_end, duration) = anim.phase_times();
        let t = anim.timer;

        // 只處理右手臂的動畫
        if !arm.is_right {
            continue;
        }

        // 自然上勾拳動畫
        match anim.phase {
            PunchPhase::WindUp => {
                // 蓄力：手臂向下、向後、向外側收（準備上勾）
                let phase_progress = t / wind_up_end;
                let ease = ease_out_quad(phase_progress);

                let rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    0.5 * ease,  // X: 手臂向下/向後（正值）
                    -0.3 * ease, // Y: 向外側旋轉
                    0.2 * ease,  // Z: 稍微內傾
                );

                let offset = Vec3::new(
                    -0.08 * ease, // X: 往外側
                    -0.12 * ease, // Y: 向下沉（蓄力）
                    -0.1 * ease,  // Z: 往後
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
                    current_x,          // X: 從下往上揮
                    current_y,          // Y: 從外側到前方（弧線）
                    0.2 * (1.0 - ease), // Z: 從傾斜到直
                );

                // 位置：弧線軌跡（從下後外 → 上前中）
                let arc = (phase_progress * std::f32::consts::PI).sin();
                let offset = Vec3::new(
                    -0.08 + 0.13 * ease + 0.05 * arc, // X: 從外側繞回中間
                    -0.12 + 0.42 * ease,              // Y: 從下往上（-0.12 → +0.3）
                    -0.1 + 0.4 * ease,                // Z: 向前伸出
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
                    -1.0, // 手臂向上的終點
                    0.2,  // 在前方
                    0.0,
                );
                let strike_offset = Vec3::new(0.05, 0.3, 0.3);

                transform.translation =
                    (arm.rest_position + strike_offset).lerp(arm.rest_position, ease);
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
    let Some(visuals) = weapon_visuals else {
        return;
    };

    for (hand_entity, hand) in hand_query.iter() {
        // 只為右手生成武器
        if !hand.is_right {
            continue;
        }

        // 為每種武器類型生成模型
        // 使用 ChildOf 直接設定父子關係，可能避免 B0004 警告
        for weapon_type in [
            WeaponType::Staff,
            WeaponType::Knife,
            WeaponType::Pistol,
            WeaponType::SMG,
            WeaponType::Shotgun,
            WeaponType::Rifle,
        ] {
            let Some(weapon_data) = visuals.get(weapon_type) else {
                continue;
            };

            // 先生成武器根實體，使用 ChildOf 設定父實體
            let weapon_root = commands
                .spawn((
                    Transform::from_translation(weapon_data.hand_offset)
                        .with_rotation(weapon_data.hand_rotation),
                    GlobalTransform::default(),
                    Visibility::Hidden, // 預設隱藏
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    WeaponModel { weapon_type },
                    Name::new(format!("Weapon_{:?}", weapon_type)),
                    ChildOf(hand_entity), // 直接設定父實體
                ))
                .id();

            // 生成武器部件作為武器根的子實體
            for part in &weapon_data.parts {
                commands.spawn((
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(part.material.clone()),
                    part.transform,
                    GlobalTransform::default(),
                    ChildOf(weapon_root), // 直接設定父實體
                ));
            }
        }
    }
}

/// 更新武器模型可見性（共用邏輯）
fn update_weapon_visibility(
    current_type: WeaponType,
    weapon_model_query: &mut Query<(&WeaponModel, &mut Visibility)>,
) {
    for (model, mut visibility) in weapon_model_query.iter_mut() {
        *visibility = if model.weapon_type == current_type {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// 根據當前武器更新武器模型可見性
pub fn weapon_visibility_system(
    player_query: Query<&WeaponInventory, (With<Player>, Changed<WeaponInventory>)>,
    mut weapon_model_query: Query<(&WeaponModel, &mut Visibility)>,
) {
    let Ok(inventory) = player_query.single() else {
        return;
    };
    let Some(current_weapon) = inventory.current_weapon() else {
        return;
    };

    update_weapon_visibility(current_weapon.stats.weapon_type, &mut weapon_model_query);
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

    let Ok(inventory) = player_query.single() else {
        return;
    };
    let Some(current_weapon) = inventory.current_weapon() else {
        return;
    };

    // 檢查是否有武器模型存在
    if weapon_model_query.iter().next().is_none() {
        return;
    }

    update_weapon_visibility(current_weapon.stats.weapon_type, &mut weapon_model_query);
    *initialized = true;
}

// ============================================================================
// 流血傷害系統
// ============================================================================

/// 流血傷害系統
/// 處理刀傷導致的持續傷害
pub fn bleed_damage_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut BleedEffect)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let dt = time.delta_secs();
    const BLEED_TICK_INTERVAL: f32 = 1.0; // 每秒造成一次傷害

    for (entity, mut bleed) in query.iter_mut() {
        bleed.remaining_time -= dt;
        bleed.tick_timer += dt;

        // 每秒造成一次傷害
        if bleed.tick_timer >= BLEED_TICK_INTERVAL {
            bleed.tick_timer -= BLEED_TICK_INTERVAL;

            // 發送流血傷害事件（使用 Melee 類型以正確歸類）
            let mut event = DamageEvent::new(entity, bleed.damage_per_second, DamageSource::Melee);
            if let Some(source) = bleed.source {
                event = event.with_attacker(source);
            }
            damage_events.write(event);
        }

        // 流血結束，移除組件
        if bleed.is_finished() {
            commands.entity(entity).remove::<BleedEffect>();
        }
    }
}

// ============================================================================
// 持槍姿勢系統
// ============================================================================

/// 持槍姿勢系統 - 當持有槍械時調整手臂位置
pub fn holding_pose_system(
    player_query: Query<(&WeaponInventory, &Children), With<Player>>,
    mut arm_query: Query<(&PlayerArm, &mut Transform), Without<PunchAnimation>>,
    input: Res<ShootingInput>,
) {
    let Ok((inventory, children)) = player_query.single() else {
        return;
    };
    let Some(weapon) = inventory.current_weapon() else {
        return;
    };

    // 使用 ShootingInput 中的 is_aim_pressed，確保系統順序正確
    let is_aiming = input.is_aim_pressed;
    let is_melee = weapon.stats.weapon_type.is_melee();

    for child in children.iter() {
        let Ok((arm, mut transform)) = arm_query.get_mut(child) else {
            continue;
        };

        if arm.is_right {
            // 右手臂 - 主要持槍手
            if is_melee {
                // 近戰武器模式：恢復原位（或使用特定姿勢）
                transform.translation = arm.rest_position;
                transform.rotation = arm.rest_rotation;
            } else if is_aiming {
                // 瞄準姿勢：手臂向前伸直，抬槍瞄準
                let aim_rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    1.4, // X: 接近水平（稍微抬起）
                    0.0,
                    0.0, // Z: 直的
                );
                let aim_offset = Vec3::new(
                    -0.05, // 往身體中心靠一點
                    -0.15, // 手臂水平後的高度調整
                    0.35,  // 向前伸
                );
                transform.translation = arm.rest_position + aim_offset;
                transform.rotation = aim_rotation;
            } else {
                // 待機持槍姿勢：手臂微彎，槍口朝下前方
                let hold_rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    0.8, // X: 稍微向前（約45度）
                    0.0,
                    0.1, // Z: 稍微外傾
                );
                let hold_offset = Vec3::new(
                    -0.02, // 往身體靠一點
                    -0.08, // 稍微下降
                    0.12,  // 稍微向前
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
                    1.3, // X: 接近水平
                    0.0,
                    -0.1, // Z: 稍微內傾
                );
                let support_offset = Vec3::new(
                    -0.12, // 往中間移動
                    -0.12, // 手臂水平後的高度調整
                    0.38,  // 向前伸（比右手更前，支撐護木）
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
