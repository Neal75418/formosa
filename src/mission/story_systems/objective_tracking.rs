//! 目標追蹤、階段切換、失敗檢查

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]
#![allow(
    clippy::needless_pass_by_value,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use bevy::prelude::*;
use std::collections::HashMap;

use crate::economy::PlayerWallet;

use super::super::cutscene::CutsceneEvent;
use super::super::cutscene_systems::is_cutscene_active;
use super::super::dialogue::DialogueEvent;
use super::super::dialogue_systems::is_dialogue_active;
use super::super::economy::RespectManager;
use super::super::story_data::{
    FailCondition, MissionObjective, MissionPhase, ObjectiveType, StoryMissionId,
};
use super::super::story_manager::{StoryMissionDatabase, StoryMissionEvent, StoryMissionManager};
use super::super::trigger::MissionTargetEntity;
use super::super::unlocks::UnlockManager;

// ============================================================================
// 目標追蹤輔助函數
// ============================================================================

fn check_follow_target_complete(
    target_id: &str,
    mission_id: StoryMissionId,
    target_query: &Query<(&Transform, &MissionTargetEntity)>,
) -> bool {
    for (_transform, target_entity) in target_query {
        if target_entity.target_id == target_id && target_entity.mission_id == mission_id {
            let reached = !target_entity.waypoints.is_empty()
                && target_entity.current_waypoint >= target_entity.waypoints.len();
            return reached;
        }
    }
    false
}

fn check_escort_npc_complete(
    target_id: &str,
    mission_id: StoryMissionId,
    target_query: &Query<(&Transform, &MissionTargetEntity)>,
) -> bool {
    for (_transform, target_entity) in target_query {
        if target_entity.target_id == target_id
            && target_entity.mission_id == mission_id
            && target_entity.target_type == super::super::trigger::MissionTargetType::Escort
        {
            let reached = !target_entity.waypoints.is_empty()
                && target_entity.current_waypoint >= target_entity.waypoints.len();
            return reached;
        }
    }
    false
}

fn check_objective_complete(
    objective: &MissionObjective,
    player_pos: Vec3,
    phase_timer: f32,
    mission_id: StoryMissionId,
    target_query: &Query<(&Transform, &MissionTargetEntity)>,
) -> bool {
    match &objective.objective_type {
        ObjectiveType::ReachLocation(target_pos, radius) => {
            player_pos.distance_squared(*target_pos) <= radius * radius
        }
        ObjectiveType::KillCount(required) => objective.current_count >= *required,
        ObjectiveType::SurviveTime(duration) => phase_timer >= *duration,
        ObjectiveType::FollowTarget(target_id, _max_distance) => {
            check_follow_target_complete(target_id, mission_id, target_query)
        }
        ObjectiveType::EscortNpc(target_id) => {
            check_escort_npc_complete(target_id, mission_id, target_query)
        }
        _ => false,
    }
}

// ============================================================================
// 目標追蹤系統
// ============================================================================

pub fn mission_objective_tracking_system(
    mut manager: ResMut<StoryMissionManager>,
    player_query: Query<&Transform, With<crate::player::Player>>,
    target_query: Query<(&Transform, &MissionTargetEntity)>,
    mut events: MessageWriter<StoryMissionEvent>,
) {
    let Some(active) = &mut manager.current_mission else {
        return;
    };
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation;
    let mission_id = active.mission_id;
    let phase_timer = active.phase_timer;

    for (index, objective) in active.objectives.iter_mut().enumerate() {
        if objective.is_completed {
            continue;
        }

        let completed = check_objective_complete(
            objective,
            player_pos,
            phase_timer,
            mission_id,
            &target_query,
        );

        if completed {
            objective.is_completed = true;
            objective.current_count = objective.target_count;
            events.write(StoryMissionEvent::ObjectiveCompleted {
                mission_id,
                objective_index: index,
            });
        }
    }
}

// ============================================================================
// 階段系統
// ============================================================================

fn play_phase_start_events(
    phase: &MissionPhase,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
    cutscene_events: &mut MessageWriter<CutsceneEvent>,
) {
    if let Some(dialogue_id) = phase.start_dialogue {
        dialogue_events.write(DialogueEvent::Start {
            dialogue_id,
            participants: HashMap::new(),
        });
    }
    if let Some(cutscene_id) = phase.cutscene {
        cutscene_events.write(CutsceneEvent::Start(cutscene_id));
    }
}

fn cleanup_mission_entities(commands: &mut Commands, spawned_entities: Vec<Entity>) {
    for entity in spawned_entities {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }
}

pub fn mission_phase_system(
    mut commands: Commands,
    mut manager: ResMut<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    mut wallet: ResMut<PlayerWallet>,
    mut respect: ResMut<RespectManager>,
    mut unlocks: ResMut<UnlockManager>,
    time: Res<Time>,
    dialogue_state: Res<super::super::dialogue::DialogueState>,
    cutscene_state: Res<super::super::cutscene::CutsceneState>,
    mut dialogue_events: MessageWriter<DialogueEvent>,
    mut cutscene_events: MessageWriter<CutsceneEvent>,
    mut mission_events: MessageWriter<StoryMissionEvent>,
    player_query: Query<&Transform, With<crate::player::Player>>,
) {
    if is_dialogue_active(&dialogue_state) || is_cutscene_active(&cutscene_state) {
        return;
    }

    let Some(active) = &mut manager.current_mission else {
        return;
    };
    active.tick(time.delta_secs());
    if !active.is_phase_complete() {
        return;
    }

    let mission_id = active.mission_id;
    let current_phase = active.current_phase;
    let Some(mission) = database.get(mission_id) else {
        return;
    };

    if let Some(end_dialogue) = mission
        .get_phase(current_phase)
        .and_then(|p| p.end_dialogue)
    {
        dialogue_events.write(DialogueEvent::Start {
            dialogue_id: end_dialogue,
            participants: HashMap::new(),
        });
        return;
    }

    let next_phase_index = current_phase + 1;

    if let Some(next_phase) = mission.get_phase(next_phase_index) {
        // 階段轉換時自動保存檢查點
        let player_pos = player_query
            .single()
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);
        manager.create_checkpoint(player_pos, next_phase_index as u32);

        // 重新取得 active（create_checkpoint 借用了 manager）
        let Some(active) = manager.current_mission.as_mut() else {
            return;
        };
        active.advance_phase(next_phase);
        mission_events.write(StoryMissionEvent::PhaseChanged {
            mission_id,
            new_phase: next_phase_index as u32,
        });
        mission_events.write(StoryMissionEvent::CheckpointReached {
            mission_id,
            phase: next_phase_index as u32,
        });
        play_phase_start_events(next_phase, &mut dialogue_events, &mut cutscene_events);
        info!(
            "📋 任務 {} 進入階段 {} (檢查點已保存)",
            mission_id, next_phase_index
        );
    } else {
        let rewards = mission.rewards.clone();
        manager.grant_rewards(&rewards, &mut wallet, &mut respect, &mut unlocks);

        if let Some((completed_id, spawned_entities)) = manager.complete_current_mission(&database)
        {
            cleanup_mission_entities(&mut commands, spawned_entities);
            mission_events.write(StoryMissionEvent::Completed {
                mission_id: completed_id,
                rewards,
            });
            info!("📋 任務完成: {}", completed_id);
        }
    }
}

// ============================================================================
// 失敗檢查
// ============================================================================

fn check_fail_condition(
    condition: &FailCondition,
    player_health: Option<&crate::combat::Health>,
    phase_timer: f32,
    time_limit: Option<f32>,
) -> bool {
    match condition {
        FailCondition::PlayerDeath => player_health.is_some_and(|h| h.current <= 0.0),
        FailCondition::TimeExpired => time_limit.is_some_and(|limit| phase_timer >= limit),
        _ => false,
    }
}

pub fn mission_fail_check_system(
    mut commands: Commands,
    mut manager: ResMut<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    player_query: Query<&crate::combat::Health, With<crate::player::Player>>,
    mut events: MessageWriter<StoryMissionEvent>,
) {
    let Some(active) = &manager.current_mission else {
        return;
    };

    let mission_id = active.mission_id;
    let phase_timer = active.phase_timer;
    let Some(mission) = database.get(mission_id) else {
        return;
    };
    let Some(phase) = mission.get_phase(active.current_phase) else {
        return;
    };

    let player_health = player_query.single().ok();

    for fail_condition in &phase.fail_conditions {
        if !check_fail_condition(fail_condition, player_health, phase_timer, phase.time_limit) {
            continue;
        }

        let reason = fail_condition.clone();
        events.write(StoryMissionEvent::Failed {
            mission_id,
            reason: reason.clone(),
        });

        let entities_to_despawn = manager.fail_current_mission(reason);
        cleanup_mission_entities(&mut commands, entities_to_despawn);
        return;
    }
}

// ============================================================================
// 便利函數
// ============================================================================

pub fn update_kill_objective(
    manager: &mut StoryMissionManager,
    target_id: &str,
    events: &mut MessageWriter<StoryMissionEvent>,
) {
    let Some(active) = &mut manager.current_mission else {
        return;
    };
    let mission_id = active.mission_id;

    for (index, objective) in active.objectives.iter_mut().enumerate() {
        if objective.is_completed {
            continue;
        }

        let matches = match &objective.objective_type {
            ObjectiveType::KillTarget(id) => id == target_id,
            ObjectiveType::KillCount(_) => true,
            _ => false,
        };

        if matches {
            objective.increment();
            events.write(StoryMissionEvent::ObjectiveUpdated {
                mission_id,
                objective_index: index,
                progress: objective.current_count,
                total: objective.target_count,
            });

            if objective.is_completed {
                events.write(StoryMissionEvent::ObjectiveCompleted {
                    mission_id,
                    objective_index: index,
                });
            }
        }
    }
}

pub fn update_talk_objective(
    manager: &mut StoryMissionManager,
    npc_id: &str,
    events: &mut MessageWriter<StoryMissionEvent>,
) {
    let Some(active) = &mut manager.current_mission else {
        return;
    };
    let mission_id = active.mission_id;

    for (index, objective) in active.objectives.iter_mut().enumerate() {
        if objective.is_completed {
            continue;
        }

        if let ObjectiveType::TalkToNpc(target_id) = &objective.objective_type {
            if target_id == npc_id {
                objective.is_completed = true;
                objective.current_count = objective.target_count;

                events.write(StoryMissionEvent::ObjectiveCompleted {
                    mission_id,
                    objective_index: index,
                });
            }
        }
    }
}

pub fn get_current_mission_info(
    manager: &StoryMissionManager,
    database: &StoryMissionDatabase,
) -> Option<CurrentMissionInfo> {
    let active = manager.current_mission.as_ref()?;
    let mission = database.get(active.mission_id)?;
    let phase = mission.get_phase(active.current_phase)?;

    Some(CurrentMissionInfo {
        mission_id: active.mission_id,
        title: mission.title.clone(),
        phase_description: phase.description.clone(),
        objectives: active.objectives.clone(),
        time_remaining: phase
            .time_limit
            .map(|limit| (limit - active.phase_timer).max(0.0)),
    })
}

pub struct CurrentMissionInfo {
    pub mission_id: StoryMissionId,
    pub title: String,
    pub phase_description: String,
    pub objectives: Vec<MissionObjective>,
    pub time_remaining: Option<f32>,
}

// ============================================================================
// 檢查點重啟系統
// ============================================================================

/// 任務失敗後，若有檢查點，按 R 鍵從檢查點重新開始
pub fn checkpoint_retry_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut manager: ResMut<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    mut player_query: Query<&mut Transform, With<crate::player::Player>>,
    mut events: MessageWriter<StoryMissionEvent>,
    mut notifications: ResMut<crate::ui::NotificationQueue>,
) {
    // 只在沒有進行中任務且有檢查點時觸發
    if manager.current_mission.is_some() {
        return;
    }

    let Some(checkpoint) = manager.load_checkpoint() else {
        return;
    };

    let checkpoint_mission_id = checkpoint.mission_id;
    let checkpoint_phase = checkpoint.phase;

    // 顯示提示（每秒只提示一次，避免刷屏）
    // 用 R 鍵重試，用 Escape 放棄
    if keyboard.just_pressed(KeyCode::KeyR) {
        match manager.retry_from_checkpoint(&database) {
            Ok(position) => {
                // 傳送玩家到檢查點位置
                if let Ok(mut player_transform) = player_query.single_mut() {
                    player_transform.translation = position;
                }

                events.write(StoryMissionEvent::CheckpointReached {
                    mission_id: checkpoint_mission_id,
                    phase: checkpoint_phase,
                });

                notifications.info("從檢查點重新開始");
                info!(
                    "從檢查點重試: 任務 {}, 階段 {}",
                    checkpoint_mission_id, checkpoint_phase
                );
            }
            Err(e) => {
                notifications.warning(format!("無法從檢查點重試: {e}"));
                warn!("檢查點重試失敗: {}", e);
            }
        }
    } else if keyboard.just_pressed(KeyCode::Escape) {
        manager.clear_checkpoint();
        notifications.info("已放棄任務");
    }
}
