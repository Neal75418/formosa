//! 警察通緝系統
//!
//! GTA 風格的 1-5 星通緝機制，包含：
//! - 犯罪行為追蹤
//! - 通緝等級管理
//! - 警察 NPC 生成與追捕
//! - 通緝等級消退
//! - 警車追逐（2 星以上）

pub(crate) mod config;
mod components;
mod events;
mod systems;
mod police_vehicle;
mod roadblock;
mod arrest;
mod police_helicopter;

#[cfg(all(debug_assertions, feature = "dev_tools"))]
mod debug_viz; // Gizmos 除錯可視化（僅 Debug 模式）

#[cfg(test)]
mod tests;

pub use components::*;
pub use events::*;
pub use systems::*;
pub use police_vehicle::*;
pub use roadblock::*;
pub use arrest::*;
pub use police_helicopter::*;

#[cfg(all(debug_assertions, feature = "dev_tools"))]
pub use debug_viz::*;

use bevy::prelude::*;
use crate::core::{AppState, PoliceSpatialHash};

/// Debug 可視化開關（F3 切換）
#[cfg(all(debug_assertions, feature = "dev_tools"))]
#[derive(Resource, Default)]
pub struct DebugVisualizationState {
    pub enabled: bool,
}

/// 警察通緝系統插件
pub struct WantedPlugin;

impl Plugin for WantedPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<WantedLevel>()
            .init_resource::<PoliceConfig>()
            .insert_resource(PoliceSpatialHash::new());  // 警察空間哈希（視野檢測優化）

        // Debug 可視化資源（僅 Debug 模式）
        #[cfg(all(debug_assertions, feature = "dev_tools"))]
        {
            app.init_resource::<DebugVisualizationState>();
        }

        app
            // 事件（Bevy 0.17: add_message）
            .add_message::<CrimeEvent>()
            .add_message::<WantedLevelChanged>()
            .add_message::<WitnessReport>()
            .add_message::<ArrestEvent>()
            .add_message::<ArrestComplete>()
            // 設置系統
            .add_systems(Startup, setup_police_visuals)
            .add_systems(Startup, setup_police_car_visuals)
            .add_systems(Startup, setup_roadblock_visuals)
            .add_systems(Startup, setup_arrest_system)
            .add_systems(Startup, setup_helicopter_visuals)
            // 更新系統 - 空間哈希更新（在警察邏輯前執行）
            .add_systems(Update, update_police_spatial_hash_system
                .run_if(in_state(AppState::InGame)))
            // 更新系統 - 犯罪處理（暫停時跳過）
            .add_systems(Update, (
                process_crime_events,
                process_witness_reports,
                wanted_cooldown_system,
            ).chain().run_if(in_state(AppState::InGame)))
            // 更新系統 - 警察管理（暫停時跳過）
            .add_systems(Update, (
                spawn_police_system,
                police_ai_system,
                police_radio_call_system,  // 無線電呼叫系統
                police_combat_system,
                despawn_police_system,
            ).run_if(in_state(AppState::InGame)))
            // 更新系統 - 警車追逐（暫停時跳過）
            .add_systems(Update, (
                spawn_police_car_system,
                police_car_ai_system,
                police_car_collision_system,
                despawn_police_car_system,
                siren_light_system,
            ).run_if(in_state(AppState::InGame)))
            // 更新系統 - 路障（暫停時跳過）
            .add_systems(Update, (
                spawn_roadblock_system,
                roadblock_update_system,
                roadblock_collision_system,
                despawn_roadblock_system,
            ).run_if(in_state(AppState::InGame)))
            // 更新系統 - 警用直升機（5 星，暫停時跳過）
            .add_systems(Update, (
                spawn_helicopter_system,
                helicopter_ai_system,
                helicopter_movement_system,
                helicopter_combat_system,
                rotor_animation_system,
                spotlight_tracking_system,
                helicopter_damage_system,
                despawn_helicopter_system,
            ).run_if(in_state(AppState::InGame)))
            // 更新系統 - 投降/逮捕（暫停時跳過）
            .add_systems(Update, (
                player_surrender_input_system,
                police_arrest_system,
                handle_arrest_event_system,
                enemy_surrender_check_system,
                surrender_visual_system,
            ).chain().run_if(in_state(AppState::InGame)))
            // 更新系統 - UI（不受暫停影響，保持顯示）
            .add_systems(Update, (
                update_wanted_hud,
                wanted_level_change_animation,  // 通緝等級變化動畫
                surrender_ui_system,
                update_surrender_progress_bar,
            ));

        // === 🎨 除錯可視化（僅 Debug 模式）===
        #[cfg(all(debug_assertions, feature = "dev_tools"))]
        {
            app
                // F3 切換開關
                .add_systems(Update, toggle_debug_visualization)
                // 除錯繪製系統（僅在啟用時執行）
                .add_systems(Update, (
                    debug_police_vision,
                    debug_astar_paths,
                    debug_panic_propagation,
                ).run_if(in_state(AppState::InGame))
                 .run_if(|state: Res<DebugVisualizationState>| state.enabled));
        }
    }
}
