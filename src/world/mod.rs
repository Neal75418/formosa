//! 世界系統模組

mod buildings;
mod characters;
mod components;
mod constants;
mod destructible;
mod interior;
mod random_events;
mod roads;
mod setup;
mod street_furniture;
mod time_weather;

#[cfg(all(debug_assertions, feature = "dev_tools"))]
mod entity_naming;

// 公開 API re-exports (允許外部使用 crate::world::* 存取)
#[allow(unused_imports)]
pub use buildings::*;
#[allow(unused_imports)]
pub use characters::*;
pub use components::*;
pub use constants::*;
pub use destructible::*;
pub use interior::*;
pub use random_events::*;
#[allow(unused_imports)]
pub use roads::*;
pub use setup::*;
#[allow(unused_imports)]
pub use street_furniture::*;
pub use time_weather::*;

#[cfg(all(debug_assertions, feature = "dev_tools"))]
pub use entity_naming::*;

use bevy::prelude::*;
use crate::core::{AppState, GameSet, InteractionSet};

/// 實體命名計時器（每秒執行一次，僅 Debug 模式）
#[cfg(all(debug_assertions, feature = "dev_tools"))]
#[derive(Resource)]
pub struct EntityNamingTimer {
    timer: Timer,
}

#[cfg(all(debug_assertions, feature = "dev_tools"))]
impl Default for EntityNamingTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

/// 世界系統插件
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            // Message
            .add_message::<RandomEventTriggered>()
            .add_message::<RandomEventCompleted>()
            .add_message::<EnvironmentDamageEvent>()
            // Startup
            .add_systems(Startup, setup_world)
            .add_systems(Startup, setup_random_events)
            .add_systems(Startup, setup_destructible_visuals)
            // Update（時間/光照）
            .add_systems(Update, (
                update_world_time,
                update_lighting,
                sun_moon_rotation_system,
                update_neon_signs,
                update_building_windows,
            ).in_set(GameSet::World))
            // Update（天氣：輸入和視覺）
            .add_systems(Update, (
                weather_input_system,
                update_sky_color,
                update_fog_effect,
            ).in_set(GameSet::World))
            // Update（隨機事件）
            .add_systems(Update, (
                random_event_spawn_system,
                random_event_update_system,
                handle_event_completed_system,
                event_notification_system,
                event_marker_system,
            )
                .in_set(GameSet::World)
                .run_if(in_state(AppState::InGame)))
            // Update（天氣粒子/動態效果）
            .add_systems(Update, (
                update_weather_transition,
                spawn_rain_drops,
                update_rain_drops,
                cleanup_rain,
                spawn_rain_puddles,
                update_rain_puddles,
                update_lightning,
                lightning_visual_effect,
            )
                .in_set(GameSet::World)
                .run_if(in_state(AppState::InGame)))
            // Update（室內系統）
            .add_systems(Update, (
                interior_proximity_system,
                interior_enter_system.in_set(InteractionSet::Interior),
                interior_hiding_system,
                door_animation_system,
            )
                .in_set(GameSet::World)
                .run_if(in_state(AppState::InGame)))
            // Update（可破壞環境）
            .add_systems(Update, (
                vehicle_destructible_collision_system,
                combat_destructible_damage_system,
                handle_environment_damage_system,
                debris_update_system,
                destruction_particle_update_system,
            )
                .in_set(GameSet::World)
                .run_if(in_state(AppState::InGame)));

        // === 實體命名系統（僅 Debug 模式，每秒執行一次即可）===
        #[cfg(all(debug_assertions, feature = "dev_tools"))]
        {
            app
                .init_resource::<EntityNamingTimer>()
                .add_systems(Update, (
                    update_entity_naming_timer,
                    (
                        name_player_entities,
                        name_vehicle_entities,
                        name_police_entities,
                        name_pedestrian_entities,
                        name_police_car_entities,
                        name_building_entities,
                    ).run_if(|timer: Res<EntityNamingTimer>| timer.timer.just_finished()),
                ).chain().run_if(in_state(AppState::InGame)));
        }
    }
}
