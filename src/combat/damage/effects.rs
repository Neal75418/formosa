//! 傷害視覺效果系統
//!
//! 血液粒子、浮動傷害數字、護甲破碎特效。

use bevy::math::EulerRot;
use bevy::prelude::*;
use rand::Rng;

use crate::combat::health::ArmorBreakEvent;
use crate::combat::visuals::*;
use crate::ui::{ChineseFont, FloatingDamageNumber, FloatingDamageTracker};

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
/// 血液粒子數量（每次受擊）
const BLOOD_PARTICLE_COUNT: usize = 12;

// ============================================================================
// 傷害數字常數
// ============================================================================
/// 高傷害判定閾值（影響傷害數字顏色）
const CRITICAL_DAMAGE_THRESHOLD: f32 = 50.0;
/// 傷害數字基礎字型大小
const DAMAGE_NUMBER_FONT_SIZE: f32 = 24.0;
/// 傷害數字世界空間縮放
const DAMAGE_NUMBER_WORLD_SCALE: f32 = 0.02;
/// 傷害數字顏色常數
const DAMAGE_NUMBER_COLOR: Color = Color::WHITE;
const HEADSHOT_NUMBER_COLOR: Color = Color::srgb(1.0, 0.9, 0.0); // 金黃色
const CRITICAL_NUMBER_COLOR: Color = Color::srgb(1.0, 0.3, 0.1); // 橙紅色

// ============================================================================
// 其他視覺常數
// ============================================================================
/// 火花速度衰減係數（每幀）
const SPARK_VELOCITY_DECAY: f32 = 0.9;

// ============================================================================
// 血液粒子
// ============================================================================

/// 生成血液粒子
pub(super) fn spawn_blood_particles(
    commands: &mut Commands,
    position: Vec3,
    direction: Vec3,
    blood_visuals: &BloodVisuals,
) {
    let mut rng = rand::rng();
    let particle_count = BLOOD_PARTICLE_COUNT;

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

/// 血液粒子更新系統
/// 處理血液粒子的物理移動和生命週期
pub fn blood_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_query: Query<(Entity, &mut BloodParticle, &mut Transform)>,
) {
    const GRAVITY: f32 = 15.0;
    let dt = time.delta_secs();

    for (entity, mut particle, mut transform) in &mut particle_query {
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
// 浮動傷害數字
// ============================================================================

/// Billboard 標記（讓文字始終面向攝影機）
#[derive(Component)]
pub(super) struct DamageNumberBillboard;

/// 生成浮動傷害數字實體
pub(super) fn spawn_floating_damage_number(
    commands: &mut Commands,
    damage: FloatingDamageNumber,
    font: &ChineseFont,
) {
    // 決定顏色
    let color = if damage.is_headshot {
        HEADSHOT_NUMBER_COLOR
    } else if damage.damage >= CRITICAL_DAMAGE_THRESHOLD {
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
            font_size: DAMAGE_NUMBER_FONT_SIZE * damage.initial_scale,
            ..default()
        },
        TextColor(color),
        // 世界空間 Transform
        Transform::from_translation(position).with_scale(Vec3::splat(DAMAGE_NUMBER_WORLD_SCALE)),
        GlobalTransform::default(),
        // 浮動傷害組件
        damage,
        // Billboard 行為標記
        DamageNumberBillboard,
    ));
}

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
    } else if damage >= CRITICAL_DAMAGE_THRESHOLD {
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

    for (entity, mut damage, mut transform, mut text_color) in &mut damage_query {
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
// 護甲特效
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

    for (entity, mut spark, mut transform) in &mut spark_query {
        spark.lifetime += dt;

        if spark.lifetime >= spark.max_lifetime {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
            continue;
        }

        // 更新位置（快速衰減）
        spark.velocity *= SPARK_VELOCITY_DECAY;
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
    const GRAVITY: f32 = 12.0;
    let dt = time.delta_secs();

    for (entity, mut shard, mut transform) in &mut shard_query {
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
