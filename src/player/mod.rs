//! 玩家系統模組
//!
//! 注意：部分玩家屬性為將來擴展預留

#![allow(dead_code)]

mod climb;
mod components;
mod config;
mod systems;

pub use climb::*;
pub use components::*;
pub use config::*;
pub use systems::*;

use crate::core::{AppState, GameSet, InteractionSet};
use bevy::prelude::*;

/// 玩家系統插件
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerConfig>()
            .init_resource::<DoubleTapTracker>()
            .init_resource::<VehicleTransitionState>()
            .add_systems(
                Update,
                (
                    player_input,
                    dodge_detection_system,
                    dodge_state_update_system,
                    climb_detection_system.after(dodge_state_update_system),
                    player_movement.after(climb_detection_system),
                    climb_animation_system.after(player_movement),
                    dodge_movement_system.after(player_movement),
                    player_jump
                        .after(climb_animation_system)
                        .after(dodge_movement_system),
                    enter_exit_vehicle.in_set(InteractionSet::Vehicle),
                    vehicle_transition_animation_system.after(enter_exit_vehicle),
                )
                    .in_set(GameSet::Player)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
