//! UI 系統模組
//!
//! 注意：部分 UI 組件為將來擴展預留

mod components;
mod constants;
mod crosshair;
mod damage_indicator;
mod delivery_app;
mod enemy_health_bars;
mod gps_navigation;
mod hud;
mod init;
mod interaction_prompt;
mod minimap;
mod notification;
mod pause_menu;
mod phone;
mod phone_apps;
mod save_slot_ui;
mod story_mission_hud;
mod setup_hud;
mod setup_map;
mod setup_menu;
mod systems;
mod weapon_wheel;
mod weather_hud;

#[cfg(all(debug_assertions, feature = "dev_tools"))]
mod fps_counter;

#[cfg(test)]
mod tests;

pub use components::*;
pub use damage_indicator::*;
pub use init::*;
pub use notification::*;
pub use systems::*;

#[cfg(all(debug_assertions, feature = "dev_tools"))]
pub use fps_counter::*;

use bevy::ecs::schedule::SystemCondition;
use bevy::prelude::*;
use crate::core::{AppState, GameSet};

/// 子 Plugin 共用：Startup 系統集（在字型初始化後執行）
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct UiSetup;

/// 子 Plugin 共用：Update 系統集（含 GameSet::Ui + InGame/Paused 條件）
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct UiActive;

/// UI 系統插件
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // 共用 SystemSet 配置
        app.configure_sets(Startup, UiSetup.after(setup_chinese_font))
            .configure_sets(
                Update,
                UiActive
                    .in_set(GameSet::Ui)
                    .run_if(in_state(AppState::InGame).or(in_state(AppState::Paused))),
            );

        // 共用基礎建設
        app.init_resource::<SaveSlotUiState>()
            .init_resource::<PhoneUiState>()
            .add_systems(Startup, setup_ui_scale)
            .add_systems(Startup, setup_chinese_font.after(setup_ui_scale))
            .add_systems(Startup, setup_ui.in_set(UiSetup))
            .add_systems(Update, update_ui_scale);

        // 子 Plugin
        app.add_plugins((
            hud::HudPlugin,
            weather_hud::WeatherHudPlugin,
            story_mission_hud::StoryMissionHudPlugin,
            minimap::MinimapPlugin,
            crosshair::CrosshairPlugin,
            enemy_health_bars::EnemyHealthBarPlugin,
            DamageIndicatorPlugin,
            weapon_wheel::WeaponWheelPlugin,
            delivery_app::DeliveryAppPlugin,
            gps_navigation::GpsNavigationPlugin,
            NotificationPlugin,
            pause_menu::PauseMenuPlugin,
            interaction_prompt::InteractionPromptPlugin,
            save_slot_ui::SaveSlotPlugin,
            phone::PhonePlugin,
        ));

        #[cfg(all(debug_assertions, feature = "dev_tools"))]
        {
            app.add_systems(Startup, setup_fps_counter)
                .add_systems(Update, update_fps_counter);
        }
    }
}
