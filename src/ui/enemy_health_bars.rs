//! 敵人血條 UI 系統
//!
//! 為敵人創建和更新 GTA 風格的血條 UI

use bevy::prelude::*;

use super::components::{
    EnemyHealthBar, EnemyHealthBarFill, EnemyHealthBarGlow, EnemyHealthBarHighlight,
};
use super::constants::{
    ENEMY_BAR_BG, ENEMY_BAR_BORDER, ENEMY_BAR_GLOW, ENEMY_BAR_HIGHLIGHT, ENEMY_HEALTH_FULL,
    ENEMY_HEALTH_LOW, ENEMY_HEALTH_MID,
};
use crate::combat::{Enemy, Health};

// ============================================================================
// 敵人血條尺寸常數
// ============================================================================
const ENEMY_HEALTH_BAR_WIDTH: f32 = 70.0;
const ENEMY_HEALTH_BAR_HEIGHT: f32 = 10.0;
const ENEMY_HEALTH_BAR_GLOW_PADDING: f32 = 3.0;

/// 為新生成的敵人創建血條 UI - GTA 風格
pub fn setup_enemy_health_bars(mut commands: Commands, new_enemies: Query<Entity, Added<Enemy>>) {
    for enemy_entity in &new_enemies {
        // 外發光層
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(ENEMY_HEALTH_BAR_WIDTH + ENEMY_HEALTH_BAR_GLOW_PADDING * 2.0),
                    height: Val::Px(ENEMY_HEALTH_BAR_HEIGHT + ENEMY_HEALTH_BAR_GLOW_PADDING * 2.0),
                    padding: UiRect::all(Val::Px(ENEMY_HEALTH_BAR_GLOW_PADDING)),
                    ..default()
                },
                BackgroundColor(ENEMY_BAR_GLOW),
                BorderRadius::all(Val::Px(6.0)),
                EnemyHealthBar { enemy_entity },
                EnemyHealthBarGlow { enemy_entity },
                Visibility::Hidden,
            ))
            .with_children(|glow| {
                // 邊框層
                glow.spawn((
                    Node {
                        width: Val::Px(ENEMY_HEALTH_BAR_WIDTH),
                        height: Val::Px(ENEMY_HEALTH_BAR_HEIGHT),
                        padding: UiRect::all(Val::Px(2.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(ENEMY_BAR_BORDER),
                    BorderColor::all(Color::srgba(0.3, 0.3, 0.35, 0.6)),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .with_children(|border| {
                    // 血條背景
                    border
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(ENEMY_BAR_BG),
                            BorderRadius::all(Val::Px(2.0)),
                        ))
                        .with_children(|bg| {
                            // 血條填充
                            bg.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(ENEMY_HEALTH_FULL),
                                BorderRadius::all(Val::Px(2.0)),
                                EnemyHealthBarFill { enemy_entity },
                            ))
                            .with_children(|fill| {
                                // 高光效果（頂部亮條）
                                fill.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Percent(100.0),
                                        height: Val::Px(3.0),
                                        top: Val::Px(0.0),
                                        left: Val::Px(0.0),
                                        ..default()
                                    },
                                    BackgroundColor(ENEMY_BAR_HIGHLIGHT),
                                    BorderRadius::top(Val::Px(2.0)),
                                    EnemyHealthBarHighlight { enemy_entity },
                                ));
                            });
                        });
                });
            });
    }
}

/// 根據血量百分比計算血條顏色
fn get_health_bar_color(percentage: f32) -> Color {
    if percentage > 0.6 {
        // 60%+ 綠色
        ENEMY_HEALTH_FULL
    } else if percentage > 0.3 {
        // 30-60% 黃色漸變到紅色
        let t = (percentage - 0.3) / 0.3; // 0.0 ~ 1.0
        Color::srgb(
            ENEMY_HEALTH_LOW.to_srgba().red * (1.0 - t) + ENEMY_HEALTH_MID.to_srgba().red * t,
            ENEMY_HEALTH_LOW.to_srgba().green * (1.0 - t) + ENEMY_HEALTH_MID.to_srgba().green * t,
            ENEMY_HEALTH_LOW.to_srgba().blue * (1.0 - t) + ENEMY_HEALTH_MID.to_srgba().blue * t,
        )
    } else {
        // 30% 以下紅色
        ENEMY_HEALTH_LOW
    }
}

/// 更新敵人血條位置和填充 - GTA 風格（含變色）
#[allow(clippy::type_complexity)]
pub fn update_enemy_health_bars(
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::camera::GameCamera>>,
    enemy_query: Query<(&GlobalTransform, &Health), With<Enemy>>,
    mut bar_query: Query<
        (&mut Node, &mut Visibility, &EnemyHealthBar),
        Without<EnemyHealthBarFill>,
    >,
    mut fill_query: Query<
        (&mut Node, &mut BackgroundColor, &EnemyHealthBarFill),
        Without<EnemyHealthBar>,
    >,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // 收集每個敵人的血量百分比（敵人數量少，Vec 線性搜尋比 HashMap 更快且免分配）
    let mut enemy_health_map: Vec<(Entity, f32)> = Vec::new();

    for (mut node, mut visibility, health_bar) in bar_query.iter_mut() {
        // 取得對應敵人的位置和血量
        let Ok((enemy_transform, health)) = enemy_query.get(health_bar.enemy_entity) else {
            // 敵人已不存在，隱藏血條
            *visibility = Visibility::Hidden;
            continue;
        };

        let percentage = health.percentage();
        enemy_health_map.push((health_bar.enemy_entity, percentage));

        // 血條位置：敵人頭頂上方
        let world_pos = enemy_transform.translation() + Vec3::new(0.0, 2.5, 0.0);

        // 世界座標轉螢幕座標
        let total_width = ENEMY_HEALTH_BAR_WIDTH + ENEMY_HEALTH_BAR_GLOW_PADDING * 2.0;
        let total_height = ENEMY_HEALTH_BAR_HEIGHT + ENEMY_HEALTH_BAR_GLOW_PADDING * 2.0;

        if let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_pos) {
            // 檢查是否在攝影機前方
            let forward = camera_transform.forward();
            let direction = (world_pos - camera_transform.translation()).normalize();
            let distance = world_pos.distance(camera_transform.translation());

            // 只在一定距離內且在攝影機前方顯示
            if forward.dot(direction) > 0.0 && distance < 50.0 {
                *visibility = Visibility::Visible;
                // 置中血條（考慮外發光層的額外尺寸）
                node.left = Val::Px(screen_pos.x - total_width / 2.0);
                node.top = Val::Px(screen_pos.y - total_height / 2.0);
            } else {
                *visibility = Visibility::Hidden;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    // 更新所有填充條的寬度和顏色
    for (mut fill_node, mut fill_bg, fill) in fill_query.iter_mut() {
        if let Some(&(_, percentage)) = enemy_health_map.iter().find(|(e, _)| *e == fill.enemy_entity) {
            fill_node.width = Val::Percent(percentage * 100.0);
            *fill_bg = BackgroundColor(get_health_bar_color(percentage));
        }
    }
}

/// 清理已死亡敵人的血條
pub fn cleanup_enemy_health_bars(
    mut commands: Commands,
    bar_query: Query<(Entity, &EnemyHealthBar)>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    for (bar_entity, health_bar) in &bar_query {
        // 如果敵人不存在了，移除血條（包含子實體）
        if enemy_query.get(health_bar.enemy_entity).is_err() {
            commands.entity(bar_entity).despawn();
        }
    }
}
