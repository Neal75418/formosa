//! 受傷指示器 UI 系統
//!
//! 顯示 GTA 風格的螢幕邊緣暈影傷害指示

use bevy::prelude::*;

use super::components::{DamageEdge, DamageIndicator, DamageIndicatorEdge, DamageIndicatorState};
use super::constants::{DAMAGE_EDGE_WIDTH, DAMAGE_FADE_RATE, DAMAGE_INDICATOR_COLOR, DAMAGE_INDICATOR_MAX_ALPHA};

/// 設置受傷指示器 UI（螢幕邊緣暈影）
pub fn setup_damage_indicator(mut commands: Commands) {
    // 受傷指示器容器（全螢幕）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            // 使用 PickingBehavior::IGNORE 讓點擊穿透
            Visibility::Hidden,
            DamageIndicator,
        ))
        .with_children(|parent| {
            // 頂部邊緣
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Px(DAMAGE_EDGE_WIDTH),
                    ..default()
                },
                BackgroundColor(DAMAGE_INDICATOR_COLOR),
                DamageIndicatorEdge {
                    edge: DamageEdge::Top,
                },
            ));

            // 底部邊緣
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Px(DAMAGE_EDGE_WIDTH),
                    ..default()
                },
                BackgroundColor(DAMAGE_INDICATOR_COLOR),
                DamageIndicatorEdge {
                    edge: DamageEdge::Bottom,
                },
            ));

            // 左側邊緣
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Px(DAMAGE_EDGE_WIDTH),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(DAMAGE_INDICATOR_COLOR),
                DamageIndicatorEdge {
                    edge: DamageEdge::Left,
                },
            ));

            // 右側邊緣
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    right: Val::Px(0.0),
                    width: Val::Px(DAMAGE_EDGE_WIDTH),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(DAMAGE_INDICATOR_COLOR),
                DamageIndicatorEdge {
                    edge: DamageEdge::Right,
                },
            ));

            // 中心暈影（額外的傷害效果）
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(30.0),
                    left: Val::Percent(30.0),
                    width: Val::Percent(40.0),
                    height: Val::Percent(40.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.4, 0.0, 0.0, 0.0)),
                BorderRadius::all(Val::Percent(50.0)),
            ));
        });
}

/// 更新受傷指示器（根據傷害強度和方向）
pub fn update_damage_indicator(
    time: Res<Time>,
    mut damage_state: ResMut<DamageIndicatorState>,
    mut indicator_query: Query<&mut Visibility, With<DamageIndicator>>,
    mut edge_query: Query<(&DamageIndicatorEdge, &mut BackgroundColor)>,
) {
    fade_damage_intensity(&time, &mut damage_state);

    let should_show = damage_state.intensity > 0.01;
    update_indicator_visibility(&mut indicator_query, should_show);

    if should_show {
        update_indicator_edges(&mut edge_query, &damage_state);
    }
}

fn fade_damage_intensity(time: &Time, damage_state: &mut DamageIndicatorState) {
    if damage_state.intensity > 0.0 {
        damage_state.intensity -= DAMAGE_FADE_RATE * time.delta_secs();
        damage_state.intensity = damage_state.intensity.max(0.0);
    }
}

fn update_indicator_visibility(
    indicator_query: &mut Query<&mut Visibility, With<DamageIndicator>>,
    should_show: bool,
) {
    for mut visibility in indicator_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_indicator_edges(
    edge_query: &mut Query<(&DamageIndicatorEdge, &mut BackgroundColor)>,
    damage_state: &DamageIndicatorState,
) {
    let base_alpha = damage_state.intensity * DAMAGE_INDICATOR_MAX_ALPHA;

    for (edge, mut bg) in edge_query.iter_mut() {
        let alpha = calculate_edge_alpha(edge.edge, damage_state.damage_direction, base_alpha);
        *bg = BackgroundColor(Color::srgba(0.6, 0.0, 0.0, alpha));
    }
}

fn calculate_edge_alpha(edge: DamageEdge, direction: Option<Vec2>, base_alpha: f32) -> f32 {
    if let Some(dir) = direction {
        // 根據傷害方向計算每個邊的強度
        // dir 是從玩家指向攻擊者的方向（螢幕座標系）
        let edge_factor = match edge {
            DamageEdge::Top => (-dir.y).max(0.0), // 傷害從上方來 (dir.y < 0)
            DamageEdge::Bottom => dir.y.max(0.0), // 傷害從下方來 (dir.y > 0)
            DamageEdge::Left => (-dir.x).max(0.0), // 傷害從左方來 (dir.x < 0)
            DamageEdge::Right => dir.x.max(0.0),  // 傷害從右方來 (dir.x > 0)
        };
        // 保留最小 0.15 的基礎強度，加上方向性的額外強度
        base_alpha * (0.15 + edge_factor * 0.85)
    } else {
        // 無方向時，所有邊緣均勻顯示
        base_alpha
    }
}

/// 觸發受傷指示器（在傷害系統中調用）
///
/// # 參數
/// - `damage_state`: 傷害指示器狀態
/// - `damage_amount`: 傷害量
/// - `direction`: 從玩家指向攻擊者的方向（世界座標 XZ 平面）
pub fn trigger_damage_indicator(
    damage_state: &mut DamageIndicatorState,
    damage_amount: f32,
    direction: Option<Vec2>,
) {
    // 根據傷害量設置強度（最大 1.0）
    let intensity_boost = (damage_amount / 30.0).min(1.0);
    damage_state.intensity = (damage_state.intensity + intensity_boost).min(1.0);

    // 設置傷害方向（新傷害覆蓋舊方向）
    if direction.is_some() {
        damage_state.damage_direction = direction;
    }
}

pub(super) struct DamageIndicatorPlugin;

impl Plugin for DamageIndicatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_damage_indicator.in_set(super::UiSetup))
            .add_systems(Update, update_damage_indicator.in_set(super::UiActive));
    }
}
