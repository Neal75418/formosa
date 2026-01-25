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

pub use combat::*;
pub use components::*;
pub use config::*;
pub use cover::*;
pub use decision::*;
pub use lifecycle::*;
pub use movement::*;
pub use perception::*;
pub use squad::*;

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
            .init_resource::<AiUpdateTimer>()
            .init_resource::<DebugSettings>()
            .init_resource::<SquadManager>()
            // 系統
            .add_systems(
                Update,
                (
                    // Debug 切換（F3）
                    debug_toggle_system,
                    // AI 感知（每 0.1 秒）
                    ai_perception_system,
                    // AI 決策（每 0.2 秒）
                    ai_decision_system,
                    // AI 掩體系統
                    ai_cover_system,
                    // 掩體釋放（死亡時清理）
                    cover_release_system,
                    // AI 小隊協調（包抄戰術）
                    squad_coordination_system,
                    // AI 移動
                    ai_movement_system,
                    // AI 攻擊（包含近戰）
                    ai_attack_system,
                    // 敵人揮拳動畫
                    enemy_punch_animation_system,
                    // 敵人生成
                    enemy_spawn_system,
                    // 敵人死亡處理
                    enemy_death_system,
                    // Debug 視覺化（Gizmo）
                    draw_ai_debug_gizmos,
                )
                    .chain()
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
    enemy_query: Query<(&Transform, &AiPerception), With<crate::combat::Enemy>>,
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
