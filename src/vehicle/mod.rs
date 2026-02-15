//! 載具系統模組
//!
//! 注意：部分載具屬性為將來擴展預留

mod components;
mod config;
mod effects;
mod modifications;
mod npc_ai;
mod spawning;
mod systems;
mod vehicle_physics;
mod theft;
mod theft_ui;
mod traffic_lights;
mod vehicle_damage;
pub mod watercraft;

pub use components::*;
pub use config::*;
pub use effects::*;
pub use modifications::*;
pub use npc_ai::*;
pub use spawning::*;
pub use systems::*;
pub use vehicle_physics::*;
pub use theft::*;
pub use theft_ui::*;
pub use traffic_lights::*;
pub use vehicle_damage::*;
#[allow(unused_imports)]
pub use watercraft::*;

use crate::core::{AppState, GameSet};
use crate::world;
use bevy::prelude::*;

/// 載具系統插件
pub struct VehiclePlugin;

impl Plugin for VehiclePlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
            .init_resource::<VehicleConfig>()
            .init_resource::<WaveParams>()
            // Message
            .add_message::<TheftEvent>()
            .add_message::<PurchaseModificationEvent>()
            .add_message::<PurchaseNitroEvent>()
            .add_message::<ModificationCompleteEvent>()
            .add_message::<PurchaseVisualModEvent>()
            // Startup
            .add_systems(Startup, setup_vehicle_effects)
            .add_systems(Startup, setup_vehicle_damage_effects)
            .add_systems(Startup, setup_traffic_lights)
            .add_systems(
                Startup,
                spawn_world_traffic_lights.after(setup_traffic_lights),
            )
            .add_systems(Startup, setup_theft_visuals)
            .add_systems(Startup, spawn_initial_traffic.after(world::setup_world))
            // Update
            .add_systems(
                Update,
                (
                    vehicle_input,
                    (
                        vehicle_weather_system,
                        vehicle_acceleration_system,
                        vehicle_steering_system,
                        vehicle_drift_system,
                        vehicle_suspension_system,
                        vehicle_physics_integration_system,
                    )
                        .chain(),
                    motorcycle_crash_system.after(vehicle_suspension_system),
                    update_vehicle_visuals.after(vehicle_physics_integration_system),
                    npc_vehicle_ai,
                    npc_vehicle_motion_system.after(npc_vehicle_ai),
                )
                    .in_set(GameSet::Vehicle)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    drift_smoke_spawn_system,
                    drift_smoke_update_system,
                    tire_track_spawn_system,
                    tire_track_update_system,
                    nitro_flame_spawn_system,
                    nitro_flame_update_system,
                )
                    .in_set(GameSet::Vehicle)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    vehicle_collision_damage_system,
                    vehicle_fire_system,
                    vehicle_damage_effect_system,
                    vehicle_explosion_system,
                    vehicle_damage_particle_update_system,
                    vehicle_damage_event_system,
                    door_animation_system,
                    door_input_system,
                    collision_window_damage_system,
                    bullet_window_damage_system,
                    body_part_visual_damage_system,
                )
                    .in_set(GameSet::Vehicle)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    theft_input_system,
                    theft_progress_system,
                    vehicle_alarm_system,
                    owner_reaction_system,
                    glass_shard_update_system,
                    hotwire_spark_update_system,
                    theft_ui_system,
                )
                    .in_set(GameSet::Vehicle)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    purchase_modification_system,
                    purchase_nitro_system,
                    nitro_boost_system,
                    purchase_visual_mod_system,
                )
                    .in_set(GameSet::Vehicle)
                    .run_if(in_state(AppState::InGame)),
            )
            // 水上載具
            .add_systems(
                Update,
                (
                    watercraft_buoyancy_system,
                    watercraft_movement_system,
                )
                    .in_set(GameSet::Vehicle)
                    .run_if(in_state(AppState::InGame)),
            )
            // 紅綠燈系統（不受暫停影響）
            .add_systems(Update, traffic_light_cycle_system);
    }
}
