//! 劇情任務核心系統
//!
//! 處理任務觸發、目標追蹤、階段切換等核心邏輯
//!
//! 子模組：
//! - `trigger_handler` - 觸發事件與 NPC 互動
//! - `objective_tracking` - 目標追蹤、階段切換、失敗檢查
//! - `mission_markers` - 任務觸發點視覺效果

mod trigger_handler;
mod objective_tracking;
mod mission_markers;

use bevy::prelude::*;

use super::story_manager::*;
use super::trigger::{
    trigger_system, ObjectiveMarker,
};
use crate::core::InteractionSet;

pub use mission_markers::{
    setup_mission_trigger_visuals, spawn_mission_triggers, update_mission_trigger_visuals,
};
pub use objective_tracking::get_current_mission_info;

/// 劇情任務系統 Plugin
pub struct StoryMissionPlugin;

impl Plugin for StoryMissionPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<StoryMissionManager>()
            .init_resource::<super::economy::RespectManager>()
            .init_resource::<super::relationship::RelationshipManager>()
            .init_resource::<super::unlocks::UnlockManager>()
            .init_resource::<StoryMissionDatabase>()
            .add_message::<StoryMissionEvent>()
            .add_message::<super::trigger::TriggerEvent>()
            .add_systems(
                Startup,
                (setup_mission_trigger_visuals, setup_story_missions).chain(),
            )
            .add_systems(
                Update,
                (
                    update_total_play_time,
                    spawn_mission_triggers,
                    objective_marker_system,
                    update_mission_trigger_visuals,
                    trigger_system,
                    (
                        trigger_handler::mission_trigger_event_handler,
                        trigger_handler::dialogue_trigger_event_handler,
                    )
                        .after(trigger_system),
                    trigger_handler::mission_npc_interaction_system,
                    objective_tracking::mission_objective_tracking_system,
                    objective_tracking::mission_phase_system.after(objective_tracking::mission_objective_tracking_system),
                    objective_tracking::mission_fail_check_system.after(objective_tracking::mission_phase_system),
                    mission_event_handler,
                    objective_tracking::checkpoint_retry_system,
                )
                    .in_set(InteractionSet::Mission),
            );
    }
}

// ============================================================================
// 基本系統
// ============================================================================

fn update_total_play_time(mut manager: ResMut<StoryMissionManager>, time: Res<Time>) {
    manager.total_play_time += time.delta_secs();
}

fn setup_story_missions(
    mut database: ResMut<StoryMissionDatabase>,
    mut manager: ResMut<StoryMissionManager>,
) {
    create_sample_missions(&mut database);

    // 註冊 Strangers & Freaks 支線任務
    super::side_missions::register_side_missions(&mut database);

    // 解鎖主線與支線
    manager.unlock_mission(1);
    // 支線任務預設解鎖（不需主線前置）
    for id in 100..=105 {
        manager.unlock_mission(id);
    }

    info!("任務系統初始化完成，共 {} 個任務（含支線）", database.total_count());
}

fn mission_event_handler(
    mut events: MessageReader<StoryMissionEvent>,
    mut manager: ResMut<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    wallet: Res<crate::economy::PlayerWallet>,
    respect: Res<super::economy::RespectManager>,
    unlocks: Res<super::unlocks::UnlockManager>,
) {
    for event in events.read() {
        match event {
            StoryMissionEvent::Started(mission_id) => {
                if let Some(mission) = database.get(*mission_id) {
                    if let Err(e) = manager.start_mission(mission, &wallet, &respect, &unlocks) {
                        warn!("無法開始任務 {}: {}", mission_id, e);
                    } else {
                        info!("📋 任務開始: {} - {}", mission_id, mission.title);
                    }
                }
            }
            StoryMissionEvent::MissionUnlocked(mission_id) => {
                manager.unlock_mission(*mission_id);
                info!("📋 任務解鎖: {}", mission_id);
            }
            StoryMissionEvent::MoneyChanged { old, new } => {
                info!("金錢變化: {} -> {}", old, new);
            }
            StoryMissionEvent::RespectChanged { old, new } => {
                info!("聲望變化: {} -> {}", old, new);
            }
            _ => {}
        }
    }
}

fn objective_marker_system(
    mut marker_query: Query<(&mut ObjectiveMarker, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut marker, mut transform) in &mut marker_query {
        marker.pulse_phase += time.delta_secs() * 2.0;
        if marker.pulse_phase > std::f32::consts::TAU {
            marker.pulse_phase -= std::f32::consts::TAU;
        }
        let offset = marker.pulse_phase.sin() * 0.3;
        transform.translation.y = marker.height_offset + offset;
    }
}
