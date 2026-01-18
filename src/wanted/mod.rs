//! 警察通緝系統
//!
//! GTA 風格的 1-5 星通緝機制，包含：
//! - 犯罪行為追蹤
//! - 通緝等級管理
//! - 警察 NPC 生成與追捕
//! - 通緝等級消退

mod components;
mod events;
mod systems;

pub use components::*;
pub use events::*;
pub use systems::*;

use bevy::prelude::*;
use crate::ui::UiState;

/// 警察通緝系統插件
pub struct WantedPlugin;

impl Plugin for WantedPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<WantedLevel>()
            .init_resource::<PoliceConfig>()
            // 事件（Bevy 0.17: add_message）
            .add_message::<CrimeEvent>()
            .add_message::<WantedLevelChanged>()
            .add_message::<WitnessReport>()
            // 設置系統
            .add_systems(Startup, setup_police_visuals)
            // 更新系統 - 犯罪處理（暫停時跳過）
            .add_systems(Update, (
                process_crime_events,
                process_witness_reports,
                wanted_cooldown_system,
            ).chain().run_if(|ui: Res<UiState>| !ui.paused))
            // 更新系統 - 警察管理（暫停時跳過）
            .add_systems(Update, (
                spawn_police_system,
                police_ai_system,
                police_radio_call_system,  // 無線電呼叫系統
                police_combat_system,
                despawn_police_system,
            ).run_if(|ui: Res<UiState>| !ui.paused))
            // 更新系統 - UI（不受暫停影響，保持顯示）
            .add_systems(Update, update_wanted_hud);
    }
}
