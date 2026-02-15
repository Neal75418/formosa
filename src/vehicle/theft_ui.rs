//! 偷車視覺效果與 UI
//!
//! 玻璃碎片動畫、熱線火花、進度條 UI

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

use super::theft::*;
use crate::player::Player;

// ============================================================================
// 視覺效果常數
// ============================================================================

/// 玻璃碎片旋轉速度 X
const SHARD_ROTATION_SPEED_X: f32 = 5.0;
/// 玻璃碎片旋轉速度 Z
const SHARD_ROTATION_SPEED_Z: f32 = 3.0;
/// 玻璃碎片基礎生命週期
const SHARD_BASE_LIFETIME: f32 = 2.0;
/// 火花閃爍速率
const SPARK_FLICKER_RATE: f32 = 10.0;
/// 重力加速度
const GRAVITY: f32 = 9.8;

// UI 常數
/// 進度條左側位置（百分比）
const THEFT_UI_LEFT: f32 = 40.0;
/// 進度條頂部位置（百分比）
const THEFT_UI_TOP: f32 = 60.0;
/// 進度條寬度（百分比）
const THEFT_UI_WIDTH: f32 = 20.0;
/// 進度條高度（像素）
const THEFT_UI_HEIGHT: f32 = 20.0;
/// 進度條邊框寬度（像素）
const THEFT_UI_BORDER: f32 = 2.0;

// ============================================================================
// 視覺效果系統
// ============================================================================

/// 玻璃碎片更新系統
pub fn glass_shard_update_system(
    mut commands: Commands,
    mut shard_query: Query<(Entity, &mut GlassShard, &mut Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut shard, mut transform) in &mut shard_query {
        shard.lifetime -= dt;

        if shard.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 移動
        transform.translation += shard.velocity * dt;

        // 重力
        shard.velocity.y -= GRAVITY * dt;

        // 旋轉
        transform.rotate_x(dt * SHARD_ROTATION_SPEED_X);
        transform.rotate_z(dt * SHARD_ROTATION_SPEED_Z);

        // 縮小
        let scale = (shard.lifetime / SHARD_BASE_LIFETIME).min(1.0);
        transform.scale = Vec3::splat(scale * 0.5 + 0.5);
    }
}

/// 熱線火花更新系統
pub fn hotwire_spark_update_system(
    mut commands: Commands,
    mut spark_query: Query<(Entity, &mut HotwireSpark, &mut Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut spark, mut transform) in &mut spark_query {
        spark.lifetime -= dt;

        if spark.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 閃爍縮放
        let scale = (spark.lifetime * SPARK_FLICKER_RATE).sin().abs() * 0.5 + 0.5;
        transform.scale = Vec3::splat(scale * 0.1);
    }
}

// ============================================================================
// UI 系統
// ============================================================================

/// 偷車進度 UI 更新系統
pub fn theft_ui_system(
    mut commands: Commands,
    player_query: Query<&PlayerTheftState, With<Player>>,
    ui_query: Query<(Entity, &Children), With<TheftProgressUI>>,
    mut bar_query: Query<&mut Node, Without<TheftProgressUI>>,
) {
    let Ok(theft_state) = player_query.single() else {
        // 移除 UI
        for (entity, _) in &ui_query {
            commands.entity(entity).despawn();
        }
        return;
    };

    if theft_state.is_stealing() {
        // 創建或更新進度條
        if ui_query.is_empty() {
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(THEFT_UI_LEFT),
                    top: Val::Percent(THEFT_UI_TOP),
                    width: Val::Percent(THEFT_UI_WIDTH),
                    height: Val::Px(THEFT_UI_HEIGHT),
                    border: UiRect::all(Val::Px(THEFT_UI_BORDER)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                BorderColor::all(Color::WHITE),
                TheftProgressUI,
            )).with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 0.5, 0.0)),
                    Name::new("ProgressBar"),
                ));
            });
        } else {
            // 更新進度條寬度
            for (_, children) in &ui_query {
                for child in children.iter() {
                    if let Ok(mut node) = bar_query.get_mut(child) {
                        node.width = Val::Percent(theft_state.progress * 100.0);
                    }
                }
            }
        }
    } else {
        // 移除 UI
        for (entity, _) in &ui_query {
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// 生成輔助函數
// ============================================================================

/// 生成玻璃碎片
pub fn spawn_glass_shards(
    commands: &mut Commands,
    visuals: &TheftVisuals,
    position: Vec3,
) {
    for _ in 0..12 {
        let velocity = Vec3::new(
            (rand::random::<f32>() - 0.5) * 4.0,
            rand::random::<f32>() * 3.0 + 1.0,
            (rand::random::<f32>() - 0.5) * 4.0,
        );

        let shard_pos = position + Vec3::new(
            (rand::random::<f32>() - 0.5) * 0.3,
            1.0 + rand::random::<f32>() * 0.5,
            (rand::random::<f32>() - 0.5) * 0.3,
        );

        commands.spawn((
            Mesh3d(visuals.glass_shard_mesh.clone()),
            MeshMaterial3d(visuals.glass_material.clone()),
            Transform::from_translation(shard_pos)
                .with_rotation(random_rotation())
                .with_scale(Vec3::splat(0.5 + rand::random::<f32>() * 0.5)),
            GlassShard {
                lifetime: 2.0 + rand::random::<f32>() * 1.0,
                velocity,
            },
        ));
    }
}

/// 生成熱線火花
pub fn spawn_hotwire_sparks(
    commands: &mut Commands,
    visuals: &TheftVisuals,
    position: Vec3,
) {
    for _ in 0..5 {
        let spark_pos = position + Vec3::new(
            (rand::random::<f32>() - 0.5) * 0.2,
            0.8,
            (rand::random::<f32>() - 0.5) * 0.2,
        );

        commands.spawn((
            Mesh3d(visuals.glass_shard_mesh.clone()),
            MeshMaterial3d(visuals.spark_material.clone()),
            Transform::from_translation(spark_pos)
                .with_scale(Vec3::splat(0.1)),
            HotwireSpark {
                lifetime: 0.2 + rand::random::<f32>() * 0.2,
            },
        ));
    }
}
