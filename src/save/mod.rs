//! 存檔系統模組
//!
//! 處理遊戲進度保存與讀取

mod components;
mod systems;

#[cfg(test)]
mod tests;

pub use components::*;
pub use systems::*;

use bevy::prelude::*;

/// 存檔系統插件
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<SaveManager>()
            .init_resource::<SaveTaskTracker>()
            // 事件
            .add_message::<SaveGameEvent>()
            .add_message::<LoadGameEvent>()
            .add_message::<AutoSaveEvent>()
            // 系統
            .add_systems(Update, (
                handle_save_input,
                handle_save_events,
                handle_load_events,
                handle_auto_save,
                // 非同步任務輪詢
                poll_save_task,
                poll_load_task,
                apply_pending_load_data,
            ).chain());
    }
}
