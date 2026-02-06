//! AI 系統
//!
//! 處理敵人 AI 行為、感知、決策和攻擊。

mod combat;
mod components;
mod config;
mod cover;
mod decision;
mod lifecycle;
mod movement;
mod perception;
mod squad;

#[cfg(test)]
mod tests;

pub use combat::*;
pub use components::*;
pub use config::*;
pub use cover::*;
pub use decision::*;
pub use lifecycle::*;
pub use movement::*;
pub use perception::*;
pub use squad::*;

use crate::combat::Enemy;
use crate::core::{AppState, DebugSettings};
use bevy::prelude::*;

/// AI 系統插件
pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<AiConfig>()
            .init_resource::<EnemySpawnTimer>()
            // AiUpdateTimer 已改為系統本地計時器，避免資源競爭
            .init_resource::<DebugSettings>()
            .init_resource::<SquadManager>()
            // 系統 - 使用精確依賴而非全部串行化
            .add_systems(
                Update,
                (
                    // 獨立系統（可並行）
                    debug_toggle_system,
                    enemy_spawn_system,
                    enemy_death_system,
                    enemy_punch_animation_system,
                    draw_ai_debug_gizmos,
                    // AI 感知（每 0.1 秒）- 第一階段
                    ai_perception_system,
                    // AI 決策（每 0.2 秒）- 依賴感知
                    ai_decision_system.after(ai_perception_system),
                    // 依賴決策的系統 - 可並行執行
                    (
                        ai_cover_system,
                        ai_movement_system,
                        ai_attack_system,
                        squad_coordination_system,
                    )
                        .after(ai_decision_system),
                    // 掩體釋放 - 依賴決策
                    cover_release_system.after(ai_decision_system),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

/// Debug 切換系統（F3）
fn debug_toggle_system(keyboard: Res<ButtonInput<KeyCode>>, mut debug: ResMut<DebugSettings>) {
    if keyboard.just_pressed(KeyCode::F3) {
        debug.show_ai_ranges = !debug.show_ai_ranges;
    }
}

/// 繪製 AI Debug Gizmos（視野範圍、聽覺範圍）
fn draw_ai_debug_gizmos(
    debug: Res<DebugSettings>,
    enemy_query: Query<(&Transform, &AiPerception), With<Enemy>>,
    mut gizmos: Gizmos,
) {
    if !debug.show_ai_ranges {
        return;
    }

    for (transform, perception) in &enemy_query {
        let pos = transform.translation + Vec3::Y * 0.1; // 稍微抬高避免地面剪裁
        let dir = Dir3::Y;

        // 視野範圍（綠色圓圈）
        gizmos.circle(
            Isometry3d::new(pos, Quat::from_rotation_arc(Vec3::Z, *dir)),
            perception.sight_range,
            Color::srgba(0.0, 1.0, 0.0, 0.5),
        );

        // 聽覺範圍（藍色圓圈）
        gizmos.circle(
            Isometry3d::new(pos, Quat::from_rotation_arc(Vec3::Z, *dir)),
            perception.hearing_range,
            Color::srgba(0.0, 0.5, 1.0, 0.3),
        );
    }
}
