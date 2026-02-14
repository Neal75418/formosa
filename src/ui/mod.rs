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
pub use crosshair::*;
pub use damage_indicator::*;
pub use delivery_app::*;
pub use enemy_health_bars::*;
pub use gps_navigation::*;
pub use hud::*;
pub use init::*;
pub use interaction_prompt::*;
pub use minimap::*;
pub use notification::*;
pub use pause_menu::*;
pub use phone::*;
pub use save_slot_ui::*;
pub use story_mission_hud::*;
pub use systems::*;
pub use weapon_wheel::*;
pub use weather_hud::*;

#[cfg(all(debug_assertions, feature = "dev_tools"))]
pub use fps_counter::*;

use bevy::ecs::schedule::SystemCondition;
use bevy::prelude::*;
use crate::core::{AppState, GameSet};

/// UI 系統插件
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let ui_active = in_state(AppState::InGame).or(in_state(AppState::Paused));

        app
            // 存檔槽 UI 資源
            .init_resource::<SaveSlotUiState>()
            .init_resource::<PhoneUiState>()
            // 暫停狀態進出
            .add_systems(OnEnter(AppState::Paused), on_enter_pause)
            .add_systems(OnExit(AppState::Paused), on_exit_pause)
            // Startup - UI Scale 最先初始化
            .add_systems(Startup, setup_ui_scale)
            .add_systems(Startup, setup_chinese_font.after(setup_ui_scale))
            .add_systems(Startup, (
                setup_ui,
                setup_delivery_app,
                setup_notification_ui,
                setup_crosshair,
                setup_damage_indicator,
                setup_weather_hud,
                setup_weapon_wheel,
                setup_interaction_prompt,
                setup_gps_ui,
                setup_story_mission_hud,
                setup_phone_ui,
            ).after(setup_chinese_font))
            // Update（核心 + UI 第一組）
            .add_systems(Update, (
                toggle_pause,
                button_hover_effect,
                animate_button_scale.after(button_hover_effect),
                toggle_map,
                toggle_delivery_app,
                update_delivery_app,
                update_ui,
                update_hud,
                update_mission_ui,
                update_minimap,
                minimap_zoom_control,
                update_fullmap,
            )
                .in_set(GameSet::Ui)
                .run_if(ui_active.clone()))
            // Update（UI 第二組）
            .add_systems(Update, (
                setup_world_name_tags,
                update_world_name_tags,
                cleanup_orphaned_world_tags,
                update_notifications,
                update_crosshair,
                update_hit_marker,
                update_ammo_display,
                update_ammo_visual_grid,
                update_weapon_switch_animation.after(update_ammo_display),
                setup_enemy_health_bars,
                update_enemy_health_bars,
                cleanup_enemy_health_bars,
                update_damage_indicator,
                update_hud_animations,
                update_crosshair_dynamics,
            )
                .in_set(GameSet::Ui)
                .run_if(ui_active.clone()))
            // 武器輪盤
            .add_systems(Update, (
                weapon_wheel_input_system,
                weapon_wheel_update_system,
                weapon_wheel_icon_update_system,
            )
                .in_set(GameSet::Ui)
                .run_if(ui_active.clone()))
            // 互動提示
            .add_systems(Update, (
                update_interaction_prompt_state,
                update_interaction_prompt_ui,
            )
                .in_set(GameSet::Ui)
                .run_if(ui_active.clone()))
            // GPS 導航
            .add_systems(Update, (
                update_gps_navigation,
                update_minimap_gps_marker,
                gps_mission_integration,
                update_gps_turn_indicator.after(update_gps_navigation),
            )
                .in_set(GameSet::Ui)
                .run_if(ui_active.clone()))
            // 天氣 HUD
            .add_systems(Update, update_weather_hud.in_set(GameSet::Ui).run_if(ui_active.clone()))
            // 劇情任務 HUD
            .add_systems(Update, update_story_mission_hud.in_set(GameSet::Ui).run_if(ui_active.clone()))
            // 存檔槽 UI
            .add_systems(Update, (
                save_slot_input_system,
                save_slot_tab_system,
                save_slot_refresh_system,
                save_slot_click_system,
                save_slot_hover_system,
            ).in_set(GameSet::Ui).run_if(ui_active.clone()))
            // 手機 UI
            .add_systems(Update, (
                phone_input_system,
                phone_visibility_system.after(phone_input_system),
                phone_icon_highlight_system.after(phone_input_system),
                phone_content_system.after(phone_input_system),
            ).in_set(GameSet::Ui).run_if(ui_active))
            // UI Scale 動態更新（視窗大小改變時）
            .add_systems(Update, update_ui_scale);

        // === FPS 計數器（僅 Debug 模式）===
        #[cfg(all(debug_assertions, feature = "dev_tools"))]
        {
            app.add_systems(Startup, setup_fps_counter)
                .add_systems(Update, update_fps_counter);
        }
    }
}
