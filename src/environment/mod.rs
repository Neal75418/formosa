//! 環境互動系統
//!
//! 處理可破壞物件、玻璃窗、碎片效果等。

mod components;
mod systems;

pub use components::*;
pub use systems::*;

use crate::core::AppState;
use bevy::prelude::*;

/// 環境系統插件
pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<DestroyedObjectTracker>()
            // 事件
            .add_message::<DestructionEvent>()
            // 啟動 - 初始化視覺資源，然後生成可破壞物件
            .add_systems(
                Startup,
                (setup_destructible_visuals, setup_world_destructibles).chain(),
            )
            // 更新
            .add_systems(
                Update,
                (
                    destructible_damage_system,
                    destruction_effect_system,
                    debris_update_system,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
