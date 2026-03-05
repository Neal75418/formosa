//! 觸發事件與 NPC 互動處理

use bevy::prelude::*;
use std::collections::HashMap;

use crate::core::{InteractionState, WorldTime};
use crate::economy::PlayerWallet;

use super::super::dialogue::DialogueEvent;
use super::super::dialogue_systems::is_dialogue_active;
use super::super::economy::RespectManager;
use super::super::story_data::{DialogueId, StoryMissionId, StoryMissionStatus};
use super::super::story_manager::{StoryMissionDatabase, StoryMissionEvent, StoryMissionManager};
use super::super::trigger::{MissionNpc, Trigger, TriggerEvent, TriggerEventType};
use super::super::unlocks::UnlockManager;

// ============================================================================
// 觸發事件處理
// ============================================================================

/// 處理任務觸發事件
pub fn mission_trigger_event_handler(
    mut events: MessageReader<TriggerEvent>,
    mut mission_events: MessageWriter<StoryMissionEvent>,
    database: Res<StoryMissionDatabase>,
    mut manager: ResMut<StoryMissionManager>,
    wallet: Res<PlayerWallet>,
    respect: Res<RespectManager>,
    unlocks: Res<UnlockManager>,
    world_time: Res<WorldTime>,
    trigger_query: Query<&Trigger>,
) {
    for event in events.read() {
        if let Some((entity, mission_id)) = extract_mission_trigger_event(event) {
            if !check_trigger_flags(*entity, &trigger_query, &manager) {
                continue;
            }

            try_start_mission(
                *mission_id,
                &database,
                &mut mission_events,
                &mut manager,
                &wallet,
                &respect,
                &unlocks,
                &world_time,
            );
        }
    }
}

fn extract_mission_trigger_event(event: &TriggerEvent) -> Option<(&Entity, &StoryMissionId)> {
    match event {
        TriggerEvent::PlayerEntered {
            entity,
            trigger_type,
        }
        | TriggerEvent::PlayerInteracted {
            entity,
            trigger_type,
        }
        | TriggerEvent::PlayerStayed {
            entity,
            trigger_type,
            ..
        } => {
            if let TriggerEventType::Mission(mission_id) = trigger_type {
                Some((entity, mission_id))
            } else {
                None
            }
        }
        TriggerEvent::PlayerExited { .. } => None,
    }
}

fn check_trigger_flags(
    entity: Entity,
    trigger_query: &Query<&Trigger>,
    manager: &StoryMissionManager,
) -> bool {
    if let Ok(trigger) = trigger_query.get(entity) {
        if let Some(flag) = &trigger.required_flag {
            if !manager.get_flag(flag) {
                return false;
            }
        }
    }
    true
}

/// 處理對話觸發事件
pub fn dialogue_trigger_event_handler(
    mut events: MessageReader<TriggerEvent>,
    mut dialogue_events: MessageWriter<DialogueEvent>,
) {
    for event in events.read() {
        match event {
            TriggerEvent::PlayerEntered { trigger_type, .. }
            | TriggerEvent::PlayerInteracted { trigger_type, .. } => {
                if let TriggerEventType::Dialogue(id) = trigger_type {
                    start_dialogue_from_trigger(*id, &mut dialogue_events);
                }
            }
            _ => {}
        }
    }
}

fn start_dialogue_from_trigger(dialogue_id: DialogueId, events: &mut MessageWriter<DialogueEvent>) {
    events.write(DialogueEvent::Start {
        dialogue_id,
        participants: HashMap::new(),
    });
    info!("對話觸發: 開始對話 {}", dialogue_id);
}

fn try_start_mission(
    mission_id: StoryMissionId,
    database: &StoryMissionDatabase,
    events: &mut MessageWriter<StoryMissionEvent>,
    manager: &mut StoryMissionManager,
    wallet: &PlayerWallet,
    respect: &RespectManager,
    unlocks: &UnlockManager,
    world_time: &WorldTime,
) {
    if let Some(mission) = database.get(mission_id) {
        if manager
            .start_mission(mission, wallet, respect, unlocks, world_time)
            .is_ok()
        {
            events.write(StoryMissionEvent::Started(mission_id));
        }
    }
}

// ============================================================================
// NPC 互動
// ============================================================================

fn try_start_npc_mission(
    mission_id: StoryMissionId,
    manager: &mut StoryMissionManager,
    database: &StoryMissionDatabase,
    wallet: &PlayerWallet,
    respect: &RespectManager,
    unlocks: &UnlockManager,
    world_time: &WorldTime,
    mission_events: &mut MessageWriter<StoryMissionEvent>,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
) -> bool {
    let status = manager.get_mission_status(mission_id);
    if status != StoryMissionStatus::Available {
        return false;
    }

    if let Some(mission) = database.get(mission_id) {
        if manager
            .start_mission(mission, wallet, respect, unlocks, world_time)
            .is_ok()
        {
            mission_events.write(StoryMissionEvent::Started(mission_id));
        } else {
            return false;
        }
    } else {
        return false;
    }

    if let Some(dialogue_id) = get_mission_start_dialogue(mission_id, database) {
        dialogue_events.write(DialogueEvent::Start {
            dialogue_id,
            participants: HashMap::new(),
        });
        trace!("任務 {:?} 開場對話已觸發: {:?}", mission_id, dialogue_id);
    } else {
        trace!("任務 {:?} 無開場對話，直接開始", mission_id);
    }
    true
}

fn get_mission_start_dialogue(
    mission_id: StoryMissionId,
    database: &StoryMissionDatabase,
) -> Option<DialogueId> {
    let mission = database.get(mission_id)?;
    let first_phase = mission.phases.first()?;
    first_phase.start_dialogue
}

fn play_npc_idle_dialogue(
    dialogue_id: DialogueId,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
) {
    dialogue_events.write(DialogueEvent::Start {
        dialogue_id,
        participants: HashMap::new(),
    });
}

/// NPC 互動系統
pub fn mission_npc_interaction_system(
    npc_query: Query<(Entity, &Transform, &MissionNpc)>,
    player_query: Query<&Transform, With<crate::player::Player>>,
    mut interaction: ResMut<InteractionState>,
    mut manager: ResMut<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    wallet: Res<PlayerWallet>,
    respect: Res<RespectManager>,
    unlocks: Res<UnlockManager>,
    world_time: Res<WorldTime>,
    dialogue_state: Res<super::super::dialogue::DialogueState>,
    mut dialogue_events: MessageWriter<DialogueEvent>,
    mut mission_events: MessageWriter<StoryMissionEvent>,
) {
    if is_dialogue_active(&dialogue_state) {
        return;
    }
    if !interaction.can_interact() {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (_entity, transform, npc) in &npc_query {
        if !npc.can_interact {
            continue;
        }

        let distance_sq = player_pos.distance_squared(transform.translation);
        let radius_sq = npc.interaction_radius * npc.interaction_radius;
        if distance_sq > radius_sq {
            continue;
        }

        if let Some(mission_id) = npc.offers_mission {
            if try_start_npc_mission(
                mission_id,
                &mut manager,
                &database,
                &wallet,
                &respect,
                &unlocks,
                &world_time,
                &mut mission_events,
                &mut dialogue_events,
            ) {
                interaction.consume();
                return;
            }
        }

        if let Some(dialogue_id) = npc.idle_dialogue {
            play_npc_idle_dialogue(dialogue_id, &mut dialogue_events);
            interaction.consume();
            return;
        }
    }
}
