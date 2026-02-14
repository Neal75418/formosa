//! 行人系統
//!
//! 處理街道上的路人 NPC，包括生成、移動、和對槍聲的反應。

mod components;
mod pathfinding;
mod behavior;
mod panic;
mod systems;

pub use components::*;
#[allow(unused_imports)]
pub use pathfinding::*;
#[allow(unused_imports)]
pub use behavior::*;
pub use panic::*;
pub use systems::*;

use bevy::prelude::*;
use crate::core::{AppState, PedestrianSpatialHash, VehicleSpatialHash};

/// 行人系統插件
pub struct PedestrianPlugin;

impl Plugin for PedestrianPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<PedestrianConfig>()
            .init_resource::<PedestrianPaths>()
            .init_resource::<GunshotTracker>()
            .init_resource::<PanicWaveManager>()  // 恐慌波管理器
            .insert_resource(VehicleSpatialHash::new())  // 車輛空間哈希
            .insert_resource(PedestrianSpatialHash::new())  // 行人空間哈希（恐慌傳播優化）
            // 設置系統
            .add_systems(Startup, (
                setup_pedestrian_visuals,
                setup_pedestrian_paths,
                setup_pathfinding_grid,
            ))
            // 更新系統 - 空間哈希更新（在碰撞/恐慌傳播前執行）
            .add_systems(Update, (
                update_vehicle_spatial_hash_system,
                update_pedestrian_spatial_hash_system,
            ).run_if(in_state(AppState::InGame)))
            // 更新系統 - 主要邏輯（暫停時跳過）
            .add_systems(Update, (
                pedestrian_spawn_system,
                pedestrian_movement_system,
                pedestrian_reaction_system,
                gunshot_tracking_system,
                pedestrian_despawn_system,
            ).chain().run_if(in_state(AppState::InGame)))
            // 更新系統 - A* 尋路（暫停時跳過）
            .add_systems(Update, (
                astar_path_calculation_system,
                astar_movement_system,
            ).run_if(in_state(AppState::InGame)))
            // 更新系統 - 日常行為（暫停時跳過）
            .add_systems(Update, (
                daily_behavior_init_system,
                daily_behavior_update_system,
                behavior_animation_system,
            ).run_if(in_state(AppState::InGame)))
            // 更新系統 - 動畫和碰撞（暫停時跳過）
            .add_systems(Update, (
                pedestrian_walking_animation_system,
                pedestrian_vehicle_collision_system,
                pedestrian_hit_response_system,
            ).run_if(in_state(AppState::InGame)))
            // 更新系統 - GTA 5 風格報警系統（暫停時跳過）
            .add_systems(Update, (
                witness_crime_detection_system,
                witness_phone_call_system,
                bribe_witness_system,
                witness_visual_system,
                witness_icon_follow_system,
            ).chain().run_if(in_state(AppState::InGame)))
            // 更新系統 - GTA 5 風格群體恐慌傳播（暫停時跳過）
            .add_systems(Update, (
                gunshot_panic_trigger_system,      // 槍聲觸發恐慌波
                panic_wave_propagation_system,    // 恐慌波傳播
                pedestrian_scream_system,         // 行人尖叫傳播
                panic_flee_direction_system,      // 恐慌逃跑方向
            ).chain().run_if(in_state(AppState::InGame)));
    }
}
