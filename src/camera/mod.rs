//! 攝影機系統模組

pub(crate) mod constants;
mod systems;

#[cfg(test)]
mod tests;

pub use systems::*;

use bevy::prelude::*;
use crate::core::AppState;

/// 攝影機插件
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<crate::core::CameraSettings>()
            .init_resource::<crate::core::CameraShake>()
            .init_resource::<crate::core::CinematicState>()
            .add_systems(Startup, setup_cinematic_letterbox)
            .add_systems(
                Update,
                (
                    camera_input,
                    camera_auto_follow
                        .after(camera_input)
                        .after(crate::player::player_movement),
                    camera_follow
                        .after(camera_auto_follow)
                        .after(crate::vehicle::vehicle_physics_integration_system),
                    dynamic_fov_system
                        .after(camera_input),
                    recoil_and_shake_update_system,
                    cinematic_camera_system,
                    cinematic_letterbox_system
                        .after(cinematic_camera_system),
                    cinematic_hud_toggle_system
                        .after(cinematic_letterbox_system),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
