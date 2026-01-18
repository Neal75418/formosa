//! 劇情任務核心系統
//!
//! 處理任務觸發、目標追蹤、階段切換等核心邏輯

use bevy::prelude::*;

use super::cutscene::CutsceneEvent;
use super::cutscene_systems::is_cutscene_active;
use super::dialogue::DialogueEvent;
use super::dialogue_systems::is_dialogue_active;
use super::story_data::*;
use super::story_manager::*;
use super::trigger::{
    MissionTargetEntity, MissionTargetType, MissionTrigger, ObjectiveMarker, TriggerEvent,
    TriggerEventType, TriggerShape, TriggerType, DialogueTrigger, MissionNpc, TriggerVisual,
};

/// 劇情任務系統 Plugin
pub struct StoryMissionPlugin;

impl Plugin for StoryMissionPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<StoryMissionManager>()
            .init_resource::<StoryMissionDatabase>()
            // 事件
            .add_message::<StoryMissionEvent>()
            .add_message::<TriggerEvent>()
            // 啟動系統
            .add_systems(Startup, (
                setup_mission_trigger_visuals,
                setup_story_missions,
            ).chain())
            // 更新系統
            .add_systems(
                Update,
                (
                    update_total_play_time, // 每幀更新遊戲時間
                    spawn_mission_triggers, // 生成可用任務的觸發點
                    mission_trigger_system,
                    dialogue_trigger_system, // 對話觸發器
                    mission_npc_interaction_system,
                    mission_objective_tracking_system,
                    mission_phase_system,
                    mission_fail_check_system,
                    mission_event_handler,
                    objective_marker_system,
                    update_mission_trigger_visuals, // 更新觸發點視覺效果
                )
                    .chain(),
            );
    }
}

// ============================================================================
// 遊戲時間追蹤
// ============================================================================

/// 更新總遊戲時間
fn update_total_play_time(
    mut manager: ResMut<StoryMissionManager>,
    time: Res<Time>,
) {
    manager.total_play_time += time.delta_secs();
}

// ============================================================================
// 初始設置
// ============================================================================

/// 設置劇情任務
fn setup_story_missions(
    mut database: ResMut<StoryMissionDatabase>,
    mut manager: ResMut<StoryMissionManager>,
) {
    // 載入範例任務
    create_sample_missions(&mut database);

    // 解鎖第一個任務
    manager.unlock_mission(1);

    info!("劇情任務系統初始化完成，共 {} 個任務", database.total_count());
}

// ============================================================================
// 觸發系統
// ============================================================================

/// 觸發器追蹤狀態（用於 OnEnterDelayed、OnStay、OnExit）
#[derive(Default)]
struct TriggerTrackingState {
    /// 玩家是否在區域內
    was_inside: bool,
    /// 進入後的計時器
    timer: f32,
    /// 是否已觸發
    triggered: bool,
}

/// 任務觸發點系統（支援所有 TriggerType）
fn mission_trigger_system(
    mut trigger_query: Query<(Entity, &Transform, &mut MissionTrigger)>,
    player_query: Query<&Transform, With<crate::player::Player>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    manager: Res<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    mut events: MessageWriter<TriggerEvent>,
    mut mission_events: MessageWriter<StoryMissionEvent>,
    mut tracking: Local<std::collections::HashMap<Entity, TriggerTrackingState>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation;

    for (entity, transform, mut trigger) in &mut trigger_query {
        if !trigger.enabled {
            continue;
        }

        // 取得或初始化追蹤狀態
        let track = tracking.entry(entity).or_default();

        // 檢查旗標條件
        if let Some(flag) = &trigger.required_flag {
            if !manager.get_flag(flag) {
                continue;
            }
        }

        // 檢查玩家是否在觸發區域內
        let in_range = trigger.shape.contains(transform.translation, player_pos);
        let just_entered = in_range && !track.was_inside;
        let just_exited = !in_range && track.was_inside;

        // 更新追蹤狀態
        track.was_inside = in_range;

        // 如果已觸發且是 one_shot，跳過
        if trigger.triggered && trigger.one_shot {
            continue;
        }

        match trigger.trigger_type {
            TriggerType::OnEnter => {
                if just_entered {
                    trigger.triggered = trigger.one_shot;
                    events.write(TriggerEvent::PlayerEntered {
                        entity,
                        trigger_type: TriggerEventType::Mission(trigger.mission_id),
                    });
                    try_start_mission(trigger.mission_id, &database, &mut mission_events);
                }
            }
            TriggerType::OnInteract => {
                if in_range && keyboard.just_pressed(KeyCode::KeyF) {
                    trigger.triggered = trigger.one_shot;
                    events.write(TriggerEvent::PlayerInteracted {
                        entity,
                        trigger_type: TriggerEventType::Mission(trigger.mission_id),
                    });
                    try_start_mission(trigger.mission_id, &database, &mut mission_events);
                }
            }
            TriggerType::OnEnterDelayed { delay } => {
                if just_entered {
                    // 開始計時
                    track.timer = 0.0;
                    track.triggered = false;
                }

                if in_range && !track.triggered {
                    track.timer += time.delta_secs() * 1000.0; // 轉換為毫秒
                    if track.timer >= delay as f32 {
                        track.triggered = true;
                        trigger.triggered = trigger.one_shot;
                        events.write(TriggerEvent::PlayerEntered {
                            entity,
                            trigger_type: TriggerEventType::Mission(trigger.mission_id),
                        });
                        try_start_mission(trigger.mission_id, &database, &mut mission_events);
                        info!("延遲觸發: {} ms 後觸發任務 {}", delay, trigger.mission_id);
                    }
                }
            }
            TriggerType::OnExit => {
                if just_exited {
                    trigger.triggered = trigger.one_shot;
                    events.write(TriggerEvent::PlayerExited {
                        entity,
                        trigger_type: TriggerEventType::Mission(trigger.mission_id),
                    });
                    try_start_mission(trigger.mission_id, &database, &mut mission_events);
                    info!("離開觸發: 任務 {}", trigger.mission_id);
                }
            }
            TriggerType::OnStay { duration } => {
                if just_entered {
                    track.timer = 0.0;
                    track.triggered = false;
                }

                if in_range && !track.triggered {
                    track.timer += time.delta_secs() * 1000.0;
                    if track.timer >= duration as f32 {
                        track.triggered = true;
                        trigger.triggered = trigger.one_shot;
                        events.write(TriggerEvent::PlayerStayed {
                            entity,
                            trigger_type: TriggerEventType::Mission(trigger.mission_id),
                            duration: track.timer / 1000.0,
                        });
                        try_start_mission(trigger.mission_id, &database, &mut mission_events);
                        info!("停留觸發: 停留 {} ms 後觸發任務 {}", duration, trigger.mission_id);
                    }
                } else if !in_range {
                    // 離開區域重置計時
                    track.timer = 0.0;
                    track.triggered = false;
                }
            }
        }
    }
}

/// 對話觸發點系統
fn dialogue_trigger_system(
    mut trigger_query: Query<(Entity, &Transform, &mut DialogueTrigger)>,
    player_query: Query<&Transform, With<crate::player::Player>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    dialogue_state: Res<super::dialogue::DialogueState>,
    mut events: MessageWriter<TriggerEvent>,
    mut dialogue_events: MessageWriter<DialogueEvent>,
    mut tracking: Local<std::collections::HashMap<Entity, TriggerTrackingState>>,
) {
    // 如果已有對話進行中，不處理
    if super::dialogue_systems::is_dialogue_active(&dialogue_state) {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation;

    for (entity, transform, mut trigger) in &mut trigger_query {
        if !trigger.enabled {
            continue;
        }

        // 取得或初始化追蹤狀態
        let track = tracking.entry(entity).or_default();

        // 檢查玩家是否在觸發區域內
        let in_range = trigger.shape.contains(transform.translation, player_pos);
        let just_entered = in_range && !track.was_inside;
        let just_exited = !in_range && track.was_inside;

        // 更新追蹤狀態
        track.was_inside = in_range;

        // 如果已觸發且是 one_shot，跳過
        if trigger.triggered && trigger.one_shot {
            continue;
        }

        match trigger.trigger_type {
            TriggerType::OnEnter => {
                if just_entered {
                    trigger.triggered = trigger.one_shot;
                    events.write(TriggerEvent::PlayerEntered {
                        entity,
                        trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
                    });
                    start_dialogue_from_trigger(trigger.dialogue_id, &mut dialogue_events);
                }
            }
            TriggerType::OnInteract => {
                if in_range && keyboard.just_pressed(KeyCode::KeyF) {
                    trigger.triggered = trigger.one_shot;
                    events.write(TriggerEvent::PlayerInteracted {
                        entity,
                        trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
                    });
                    start_dialogue_from_trigger(trigger.dialogue_id, &mut dialogue_events);
                }
            }
            TriggerType::OnEnterDelayed { delay } => {
                if just_entered {
                    track.timer = 0.0;
                    track.triggered = false;
                }

                if in_range && !track.triggered {
                    track.timer += time.delta_secs() * 1000.0;
                    if track.timer >= delay as f32 {
                        track.triggered = true;
                        trigger.triggered = trigger.one_shot;
                        events.write(TriggerEvent::PlayerEntered {
                            entity,
                            trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
                        });
                        start_dialogue_from_trigger(trigger.dialogue_id, &mut dialogue_events);
                    }
                }
            }
            TriggerType::OnExit => {
                if just_exited {
                    trigger.triggered = trigger.one_shot;
                    events.write(TriggerEvent::PlayerExited {
                        entity,
                        trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
                    });
                    start_dialogue_from_trigger(trigger.dialogue_id, &mut dialogue_events);
                }
            }
            TriggerType::OnStay { duration } => {
                if just_entered {
                    track.timer = 0.0;
                    track.triggered = false;
                }

                if in_range && !track.triggered {
                    track.timer += time.delta_secs() * 1000.0;
                    if track.timer >= duration as f32 {
                        track.triggered = true;
                        trigger.triggered = trigger.one_shot;
                        events.write(TriggerEvent::PlayerStayed {
                            entity,
                            trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
                            duration: track.timer / 1000.0,
                        });
                        start_dialogue_from_trigger(trigger.dialogue_id, &mut dialogue_events);
                    }
                } else if !in_range {
                    track.timer = 0.0;
                    track.triggered = false;
                }
            }
        }
    }
}

/// 從觸發器開始對話
fn start_dialogue_from_trigger(
    dialogue_id: super::story_data::DialogueId,
    events: &mut MessageWriter<DialogueEvent>,
) {
    events.write(DialogueEvent::Start {
        dialogue_id,
        participants: std::collections::HashMap::new(),
    });
    info!("對話觸發: 開始對話 {}", dialogue_id);
}

/// 嘗試開始任務
fn try_start_mission(
    mission_id: StoryMissionId,
    database: &StoryMissionDatabase,
    events: &mut MessageWriter<StoryMissionEvent>,
) {
    if let Some(_mission) = database.get(mission_id) {
        events.write(StoryMissionEvent::Started(mission_id));
    }
}

/// NPC 互動系統
fn mission_npc_interaction_system(
    npc_query: Query<(Entity, &Transform, &MissionNpc)>,
    player_query: Query<&Transform, With<crate::player::Player>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    manager: Res<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    dialogue_state: Res<super::dialogue::DialogueState>,
    mut dialogue_events: MessageWriter<DialogueEvent>,
    mut mission_events: MessageWriter<StoryMissionEvent>,
) {
    // 如果已有對話進行中，不處理
    if is_dialogue_active(&dialogue_state) {
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

        // 檢查距離 (使用 distance_squared 避免 sqrt)
        let distance_sq = player_pos.distance_squared(transform.translation);
        let radius_sq = npc.interaction_radius * npc.interaction_radius;
        if distance_sq > radius_sq {
            continue;
        }

        // 按 F 互動
        if keyboard.just_pressed(KeyCode::KeyF) {
            // 優先檢查是否有任務可提供
            if let Some(mission_id) = npc.offers_mission {
                let status = manager.get_mission_status(mission_id);

                if status == StoryMissionStatus::Available {
                    // 開始任務
                    mission_events.write(StoryMissionEvent::Started(mission_id));

                    // 播放任務開場對話（如果有）
                    if let Some(mission) = database.get(mission_id) {
                        if let Some(first_phase) = mission.phases.first() {
                            if let Some(dialogue_id) = first_phase.start_dialogue {
                                dialogue_events.write(DialogueEvent::Start {
                                    dialogue_id,
                                    participants: std::collections::HashMap::new(),
                                });
                            }
                        }
                    }
                    return;
                }
            }

            // 否則播放閒聊對話
            if let Some(dialogue_id) = npc.idle_dialogue {
                dialogue_events.write(DialogueEvent::Start {
                    dialogue_id,
                    participants: std::collections::HashMap::new(),
                });
            }
        }
    }
}

// ============================================================================
// 目標追蹤系統
// ============================================================================

/// 任務目標追蹤系統
fn mission_objective_tracking_system(
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

    // 檢查每個目標
    for (index, objective) in active.objectives.iter_mut().enumerate() {
        if objective.is_completed {
            continue;
        }

        let completed = match &objective.objective_type {
            ObjectiveType::ReachLocation(target_pos, radius) => {
                // 使用 distance_squared 避免 sqrt
                player_pos.distance_squared(*target_pos) <= radius * radius
            }
            ObjectiveType::KillCount(required) => {
                objective.current_count >= *required
            }
            ObjectiveType::SurviveTime(duration) => {
                active.phase_timer >= *duration
            }
            ObjectiveType::FollowTarget(target_id, max_distance) => {
                // 查找目標實體並檢查玩家是否保持在範圍內
                let mut target_reached_destination = false;
                for (target_transform, target_entity) in &target_query {
                    if &target_entity.target_id == target_id && target_entity.mission_id == mission_id {
                        let distance = player_pos.distance(target_transform.translation);
                        // 如果目標已到達終點（路徑走完），任務完成
                        if !target_entity.waypoints.is_empty()
                            && target_entity.current_waypoint >= target_entity.waypoints.len()
                        {
                            target_reached_destination = true;
                        }
                        // 注意：如果玩家距離太遠，會由 mission_fail_check_system 處理失敗
                        break;
                    }
                }
                target_reached_destination
            }
            ObjectiveType::EscortNpc(target_id) => {
                // 護送任務：檢查 NPC 是否到達目的地
                let mut escort_complete = false;
                for (_target_transform, target_entity) in &target_query {
                    if &target_entity.target_id == target_id
                        && target_entity.mission_id == mission_id
                        && target_entity.target_type == MissionTargetType::Escort
                    {
                        // NPC 到達終點
                        if !target_entity.waypoints.is_empty()
                            && target_entity.current_waypoint >= target_entity.waypoints.len()
                        {
                            escort_complete = true;
                        }
                        break;
                    }
                }
                escort_complete
            }
            // 其他目標類型由專門的系統更新
            _ => false,
        };

        if completed && !objective.is_completed {
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

/// 任務階段切換系統
fn mission_phase_system(
    mut commands: Commands,
    mut manager: ResMut<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    time: Res<Time>,
    dialogue_state: Res<super::dialogue::DialogueState>,
    cutscene_state: Res<super::cutscene::CutsceneState>,
    mut dialogue_events: MessageWriter<DialogueEvent>,
    mut cutscene_events: MessageWriter<CutsceneEvent>,
    mut mission_events: MessageWriter<StoryMissionEvent>,
) {
    // 如果有對話或過場進行中，不切換階段
    if is_dialogue_active(&dialogue_state) || is_cutscene_active(&cutscene_state) {
        return;
    }

    let Some(active) = &mut manager.current_mission else {
        return;
    };

    // 更新計時器
    active.tick(time.delta_secs());

    // 檢查當前階段是否完成
    if !active.is_phase_complete() {
        return;
    }

    let mission_id = active.mission_id;
    let current_phase = active.current_phase;

    let Some(mission) = database.get(mission_id) else {
        return;
    };

    // 播放階段完成過場動畫
    if let Some(phase) = mission.get_phase(current_phase) {
        if let Some(end_dialogue) = phase.end_dialogue {
            dialogue_events.write(DialogueEvent::Start {
                dialogue_id: end_dialogue,
                participants: std::collections::HashMap::new(),
            });
            return; // 等待對話完成
        }
    }

    // 嘗試前進到下一階段
    let next_phase_index = current_phase + 1;

    if let Some(next_phase) = mission.get_phase(next_phase_index) {
        active.advance_phase(next_phase);

        mission_events.write(StoryMissionEvent::PhaseChanged {
            mission_id,
            new_phase: next_phase_index as u32,
        });

        // 播放新階段開場對話
        if let Some(dialogue_id) = next_phase.start_dialogue {
            dialogue_events.write(DialogueEvent::Start {
                dialogue_id,
                participants: std::collections::HashMap::new(),
            });
        }

        // 播放新階段過場動畫
        if let Some(cutscene_id) = next_phase.cutscene {
            cutscene_events.write(CutsceneEvent::Start(cutscene_id));
        }

        info!("任務 {} 進入階段 {}", mission_id, next_phase_index);
    } else {
        // 任務完成
        let rewards = mission.rewards.clone();

        // 發放獎勵
        manager.grant_rewards(&rewards);

        // 清除當前任務並清理生成的實體
        if let Some((completed_id, spawned_entities)) = manager.complete_current_mission() {
            // 清理任務生成的實體
            for entity in spawned_entities {
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.despawn();
                }
            }

            mission_events.write(StoryMissionEvent::Completed {
                mission_id: completed_id,
                rewards,
            });

            info!("任務完成: {}", completed_id);
        }
    }
}

// ============================================================================
// 失敗檢查系統
// ============================================================================

/// 任務失敗檢查系統
fn mission_fail_check_system(
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
    let current_phase = active.current_phase;

    let Some(mission) = database.get(mission_id) else {
        return;
    };

    let Some(phase) = mission.get_phase(current_phase) else {
        return;
    };

    // 檢查每個失敗條件
    for fail_condition in &phase.fail_conditions {
        let failed = match fail_condition {
            FailCondition::PlayerDeath => {
                if let Ok(health) = player_query.single() {
                    health.current <= 0.0
                } else {
                    false
                }
            }
            FailCondition::TimeExpired => {
                if let Some(limit) = phase.time_limit {
                    active.phase_timer >= limit
                } else {
                    false
                }
            }
            // 其他失敗條件
            _ => false,
        };

        if failed {
            let reason = fail_condition.clone();
            events.write(StoryMissionEvent::Failed {
                mission_id,
                reason: reason.clone(),
            });

            // 清理任務生成的實體
            let entities_to_despawn = manager.fail_current_mission(reason);
            for entity in entities_to_despawn {
                if let Ok(mut cmd) = commands.get_entity(entity) {
                    cmd.despawn();
                }
            }
            return;
        }
    }
}

// ============================================================================
// 事件處理
// ============================================================================

/// 任務事件處理系統
fn mission_event_handler(
    mut events: MessageReader<StoryMissionEvent>,
    mut manager: ResMut<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
) {
    for event in events.read() {
        match event {
            StoryMissionEvent::Started(mission_id) => {
                if let Some(mission) = database.get(*mission_id) {
                    if let Err(e) = manager.start_mission(mission) {
                        warn!("無法開始任務 {}: {}", mission_id, e);
                    } else {
                        info!("任務開始: {} - {}", mission_id, mission.title);
                    }
                }
            }
            StoryMissionEvent::MissionUnlocked(mission_id) => {
                manager.unlock_mission(*mission_id);
                info!("任務解鎖: {}", mission_id);
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

// ============================================================================
// 標記系統
// ============================================================================

/// 目標標記動畫系統
fn objective_marker_system(
    mut marker_query: Query<(&mut ObjectiveMarker, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut marker, mut transform) in &mut marker_query {
        // 脈衝動畫
        marker.pulse_phase += time.delta_secs() * 2.0;
        if marker.pulse_phase > std::f32::consts::TAU {
            marker.pulse_phase -= std::f32::consts::TAU;
        }

        // 上下浮動
        let offset = marker.pulse_phase.sin() * 0.3;
        transform.translation.y = marker.height_offset + offset;
    }
}

// ============================================================================
// 便利函數
// ============================================================================

/// 更新擊殺目標計數
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

/// 更新對話目標
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

/// 取得當前任務資訊（用於 UI 顯示）
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
        time_remaining: phase.time_limit.map(|limit| (limit - active.phase_timer).max(0.0)),
    })
}

/// 當前任務資訊
pub struct CurrentMissionInfo {
    pub mission_id: StoryMissionId,
    pub title: String,
    pub phase_description: String,
    pub objectives: Vec<MissionObjective>,
    pub time_remaining: Option<f32>,
}

// ============================================================================
// 任務觸發點生成系統
// ============================================================================

// --- 觸發點視覺效果常數 ---
/// 觸發點高度偏移（地面以上）
const TRIGGER_HEIGHT_OFFSET: f32 = 4.0;
/// 脈衝動畫速度
const TRIGGER_PULSE_SPEED: f32 = 2.0;
/// 上下浮動幅度
const TRIGGER_FLOAT_AMPLITUDE: f32 = 0.3;
/// 縮放脈衝幅度
const TRIGGER_SCALE_AMPLITUDE: f32 = 0.1;

/// 世界任務觸發點標記（用於識別由系統生成的觸發點）
#[derive(Component)]
pub struct WorldMissionTrigger {
    /// 關聯的任務 ID
    pub mission_id: StoryMissionId,
    /// 基礎 Y 位置（用於動畫）
    pub base_y: f32,
}

/// 任務觸發點視覺資源
#[derive(Resource)]
pub struct MissionTriggerVisuals {
    /// 黃色圓柱 mesh（任務標記）
    pub marker_mesh: Handle<Mesh>,
    /// 黃色材質
    pub marker_material: Handle<StandardMaterial>,
    /// 脈衝光環 mesh
    pub ring_mesh: Handle<Mesh>,
    /// 半透明黃色材質
    pub ring_material: Handle<StandardMaterial>,
}

/// 任務標記光環組件（用於旋轉動畫）
#[derive(Component)]
pub struct MissionMarkerRing {
    pub rotation_speed: f32,
}

impl Default for MissionMarkerRing {
    fn default() -> Self {
        Self { rotation_speed: 1.0 }
    }
}

/// 設置任務觸發點視覺資源
pub fn setup_mission_trigger_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let visuals = MissionTriggerVisuals {
        // 黃色圓柱標記（像 GTA 的任務標記）
        marker_mesh: meshes.add(Cylinder::new(0.8, 8.0)),
        marker_material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.9, 0.0, 0.6),
            emissive: bevy::color::LinearRgba::new(1.0, 0.8, 0.0, 1.0),
            alpha_mode: bevy::prelude::AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        // 旋轉光環
        ring_mesh: meshes.add(Torus::new(1.5, 0.15)),
        ring_material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.9, 0.0, 0.4),
            emissive: bevy::color::LinearRgba::new(1.0, 0.7, 0.0, 1.0),
            alpha_mode: bevy::prelude::AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
    };
    commands.insert_resource(visuals);
}

/// 生成可用任務的觸發點
pub fn spawn_mission_triggers(
    mut commands: Commands,
    manager: Res<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    visuals: Res<MissionTriggerVisuals>,
    existing_triggers: Query<(Entity, &WorldMissionTrigger)>,
    mut mission_events: MessageReader<StoryMissionEvent>,
) {
    // 收集需要移除觸發點的任務 ID（避免重複遍歷）
    let mut missions_to_remove = std::collections::HashSet::new();
    for event in mission_events.read() {
        match event {
            StoryMissionEvent::Started(mission_id)
            | StoryMissionEvent::Completed { mission_id, .. } => {
                missions_to_remove.insert(*mission_id);
            }
            _ => {}
        }
    }

    // 單次遍歷移除所有需要清理的觸發點
    if !missions_to_remove.is_empty() {
        for (entity, trigger) in &existing_triggers {
            if missions_to_remove.contains(&trigger.mission_id) {
                commands.entity(entity).despawn();
                info!("移除任務 {} 的觸發點", trigger.mission_id);
            }
        }
    }

    // 檢查哪些可用任務還沒有觸發點
    let available_missions = manager.get_available_missions();

    for mission_id in available_missions {
        // 檢查是否已有觸發點
        let already_exists = existing_triggers
            .iter()
            .any(|(_, t)| t.mission_id == mission_id);

        if already_exists {
            continue;
        }

        // 取得任務資料
        let Some(mission) = database.get(mission_id) else {
            continue;
        };

        // 需要有觸發位置
        let Some(trigger_pos) = mission.trigger_location else {
            continue;
        };

        // 生成任務觸發點
        spawn_mission_trigger_marker(
            &mut commands,
            &visuals,
            mission_id,
            trigger_pos,
            mission.trigger_radius,
            &mission.title,
        );

        info!(
            "生成任務觸發點: {} - {} 在 {:?}",
            mission_id, mission.title, trigger_pos
        );
    }
}

/// 生成單個任務觸發點標記
fn spawn_mission_trigger_marker(
    commands: &mut Commands,
    visuals: &MissionTriggerVisuals,
    mission_id: StoryMissionId,
    position: Vec3,
    radius: f32,
    title: &str,
) {
    // 計算基礎 Y 位置
    let base_y = position.y + TRIGGER_HEIGHT_OFFSET;

    // 主標記實體（黃色光柱）
    commands
        .spawn((
            Mesh3d(visuals.marker_mesh.clone()),
            MeshMaterial3d(visuals.marker_material.clone()),
            Transform::from_translation(position + Vec3::Y * TRIGGER_HEIGHT_OFFSET),
            WorldMissionTrigger { mission_id, base_y },
            MissionTrigger::new(mission_id)
                .with_shape(TriggerShape::Circle(radius))
                .with_trigger_type(TriggerType::OnInteract)
                .with_prompt(format!("按 F 開始任務: {}", title))
                .with_mission_name(title.to_string()),
            TriggerVisual::default(),
            Name::new(format!("MissionTrigger_{}", mission_id)),
        ))
        .with_children(|parent| {
            // 底部旋轉光環
            parent.spawn((
                Mesh3d(visuals.ring_mesh.clone()),
                MeshMaterial3d(visuals.ring_material.clone()),
                Transform::from_translation(Vec3::Y * -TRIGGER_HEIGHT_OFFSET)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                MissionMarkerRing::default(),
                Name::new(format!("MissionTriggerRing_{}", mission_id)),
            ));
        });
}

/// 更新任務觸發點視覺效果（脈衝動畫）
pub fn update_mission_trigger_visuals(
    mut trigger_query: Query<(&mut Transform, &WorldMissionTrigger)>,
    mut ring_query: Query<(&mut Transform, &MissionMarkerRing), Without<WorldMissionTrigger>>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
    let dt = time.delta_secs();

    // 更新主標記脈衝
    for (mut transform, trigger) in &mut trigger_query {
        // 上下浮動（使用儲存的基礎 Y 位置）
        let phase = elapsed * TRIGGER_PULSE_SPEED;
        let offset = phase.sin() * TRIGGER_FLOAT_AMPLITUDE;
        transform.translation.y = trigger.base_y + offset;

        // 縮放脈衝
        let scale_pulse = 1.0 + phase.sin().abs() * TRIGGER_SCALE_AMPLITUDE;
        transform.scale = Vec3::splat(scale_pulse);
    }

    // 更新光環旋轉（使用組件中的 rotation_speed）
    for (mut transform, ring) in &mut ring_query {
        transform.rotate_y(dt * ring.rotation_speed);
    }
}
