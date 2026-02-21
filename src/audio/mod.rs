//! 音效系統模組
//!
//! 注意：部分功能為將來擴展預留

mod components;
mod integration;
mod systems;

pub use components::*;
pub use integration::*;
pub use systems::*;

use bevy::prelude::*;
use crate::core::AppState;

/// 音效插件
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .insert_resource(AudioManager::default())
            .init_resource::<FootstepTimer>()
            .init_resource::<AudioVehicleState>()
            .init_resource::<RadioManager>()
            .init_resource::<RadioPlaylists>()
            .init_resource::<PlayerGroundSurface>()
            // 啟動系統
            .add_systems(
                Startup,
                (
                    setup_audio,
                    setup_weapon_sounds,
                    setup_vehicle_sounds,
                    setup_player_sounds,
                    setup_ui_sounds,
                    setup_police_radio,
                    setup_npc_dialogue,
                ),
            )
            // 背景音樂（不受暫停影響）
            .add_systems(Update, update_background_music)
            // 引擎聲和環境音（暫停時跳過）
            .add_systems(
                Update,
                (
                    auto_attach_engine_sounds,
                    update_engine_sounds
                        .after(auto_attach_engine_sounds),
                    update_ambient_sounds,
                )
                    .run_if(in_state(AppState::InGame)),
            )
            // 事件驅動觸發音效
            .add_systems(
                Update,
                (
                    audio_wanted_level_system,
                    audio_mission_event_system,
                    audio_vehicle_enter_exit_system,
                    detect_ground_surface_system,
                    audio_footstep_system
                        .after(detect_ground_surface_system),
                    radio_input_system,
                    radio_fade_system
                        .after(radio_input_system),
                    radio_playback_system
                        .after(radio_fade_system),
                    radio_station_name_timer,
                    police_radio_chatter_system,
                    npc_dialogue_cooldown_system,
                    npc_dialogue_trigger_system
                        .after(npc_dialogue_cooldown_system),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
