//! 玩家系統模組
//!
//! 注意：部分玩家屬性為將來擴展預留

pub mod character_switch;
mod climb;
mod components;
mod config;
pub mod skills;
mod systems;
mod vehicle_transition;

#[allow(unused_imports)]
pub use character_switch::*;
pub use climb::*;
pub use components::*;
pub use config::*;
pub use skills::*;
pub use systems::*;
pub use vehicle_transition::*;

use crate::core::{AppState, GameSet, InteractionSet};
use bevy::prelude::*;

/// 玩家系統插件
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerConfig>()
            .init_resource::<DoubleTapTracker>()
            .init_resource::<VehicleTransitionState>()
            .init_resource::<StealthState>()
            .init_resource::<PlayerSkills>()
            .init_resource::<CharacterManager>()
            .add_systems(
                Update,
                (
                    player_input,
                    stamina_system.after(player_input),
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
                    stealth_noise_system.after(player_input),
                    // 技能成長系統
                    skills::driving_skill_system,
                    skills::stamina_skill_system,
                    skills::stealth_skill_system,
                    character_switch::character_switch_cooldown_system,
                )
                    .in_set(GameSet::Player)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
