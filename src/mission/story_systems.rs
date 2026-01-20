//! 劇情任務核心系統
//!
//! 處理任務觸發、目標追蹤、階段切換等核心邏輯

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::f32::consts::{FRAC_PI_2, TAU};

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

/// 觸發器上下文（用於簡化參數傳遞）
struct TriggerContext {
    entity: Entity,
    in_range: bool,
    just_entered: bool,
    just_exited: bool,
}

impl TriggerContext {
    fn new(entity: Entity, trigger_pos: Vec3, player_pos: Vec3, shape: &TriggerShape, was_inside: bool) -> Self {
        let in_range = shape.contains(trigger_pos, player_pos);
        Self {
            entity,
            in_range,
            just_entered: in_range && !was_inside,
            just_exited: !in_range && was_inside,
        }
    }
}

// === 任務觸發類型處理輔助函數 ===

/// 處理 OnEnter 觸發
fn handle_mission_on_enter(
    ctx: &TriggerContext,
    trigger: &mut MissionTrigger,
    events: &mut MessageWriter<TriggerEvent>,
    mission_events: &mut MessageWriter<StoryMissionEvent>,
    database: &StoryMissionDatabase,
) {
    if !ctx.just_entered {
        return;
    }
    trigger.triggered = trigger.one_shot;
    events.write(TriggerEvent::PlayerEntered {
        entity: ctx.entity,
        trigger_type: TriggerEventType::Mission(trigger.mission_id),
    });
    try_start_mission(trigger.mission_id, database, mission_events);
}

/// 處理 OnInteract 觸發
fn handle_mission_on_interact(
    ctx: &TriggerContext,
    trigger: &mut MissionTrigger,
    keyboard: &ButtonInput<KeyCode>,
    events: &mut MessageWriter<TriggerEvent>,
    mission_events: &mut MessageWriter<StoryMissionEvent>,
    database: &StoryMissionDatabase,
) {
    if !ctx.in_range || !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    trigger.triggered = trigger.one_shot;
    events.write(TriggerEvent::PlayerInteracted {
        entity: ctx.entity,
        trigger_type: TriggerEventType::Mission(trigger.mission_id),
    });
    try_start_mission(trigger.mission_id, database, mission_events);
}

/// 處理 OnEnterDelayed 觸發
fn handle_mission_on_enter_delayed(
    ctx: &TriggerContext,
    trigger: &mut MissionTrigger,
    track: &mut TriggerTrackingState,
    delay: u32,
    delta_ms: f32,
    events: &mut MessageWriter<TriggerEvent>,
    mission_events: &mut MessageWriter<StoryMissionEvent>,
    database: &StoryMissionDatabase,
) {
    if ctx.just_entered {
        track.timer = 0.0;
        track.triggered = false;
    }
    if !ctx.in_range || track.triggered {
        return;
    }
    track.timer += delta_ms;
    if track.timer < delay as f32 {
        return;
    }
    track.triggered = true;
    trigger.triggered = trigger.one_shot;
    events.write(TriggerEvent::PlayerEntered {
        entity: ctx.entity,
        trigger_type: TriggerEventType::Mission(trigger.mission_id),
    });
    try_start_mission(trigger.mission_id, database, mission_events);
    info!("延遲觸發: {} ms 後觸發任務 {}", delay, trigger.mission_id);
}

/// 處理 OnExit 觸發
fn handle_mission_on_exit(
    ctx: &TriggerContext,
    trigger: &mut MissionTrigger,
    events: &mut MessageWriter<TriggerEvent>,
    mission_events: &mut MessageWriter<StoryMissionEvent>,
    database: &StoryMissionDatabase,
) {
    if !ctx.just_exited {
        return;
    }
    trigger.triggered = trigger.one_shot;
    events.write(TriggerEvent::PlayerExited {
        entity: ctx.entity,
        trigger_type: TriggerEventType::Mission(trigger.mission_id),
    });
    try_start_mission(trigger.mission_id, database, mission_events);
    info!("離開觸發: 任務 {}", trigger.mission_id);
}

/// 處理 OnStay 觸發
fn handle_mission_on_stay(
    ctx: &TriggerContext,
    trigger: &mut MissionTrigger,
    track: &mut TriggerTrackingState,
    duration: u32,
    delta_ms: f32,
    events: &mut MessageWriter<TriggerEvent>,
    mission_events: &mut MessageWriter<StoryMissionEvent>,
    database: &StoryMissionDatabase,
) {
    if ctx.just_entered {
        track.timer = 0.0;
        track.triggered = false;
    }
    if ctx.in_range && !track.triggered {
        track.timer += delta_ms;
        if track.timer >= duration as f32 {
            track.triggered = true;
            trigger.triggered = trigger.one_shot;
            events.write(TriggerEvent::PlayerStayed {
                entity: ctx.entity,
                trigger_type: TriggerEventType::Mission(trigger.mission_id),
                duration: track.timer / 1000.0,
            });
            try_start_mission(trigger.mission_id, database, mission_events);
            info!("停留觸發: 停留 {} ms 後觸發任務 {}", duration, trigger.mission_id);
        }
    } else if !ctx.in_range {
        track.timer = 0.0;
        track.triggered = false;
    }
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
    mut tracking: Local<HashMap<Entity, TriggerTrackingState>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let delta_ms = time.delta_secs() * 1000.0;

    for (entity, transform, mut trigger) in &mut trigger_query {
        if !trigger.enabled || (trigger.triggered && trigger.one_shot) {
            continue;
        }

        // 檢查旗標條件
        if let Some(flag) = &trigger.required_flag {
            if !manager.get_flag(flag) {
                continue;
            }
        }

        let track = tracking.entry(entity).or_default();
        let ctx = TriggerContext::new(entity, transform.translation, player_pos, &trigger.shape, track.was_inside);
        track.was_inside = ctx.in_range;

        match trigger.trigger_type {
            TriggerType::OnEnter => {
                handle_mission_on_enter(&ctx, &mut trigger, &mut events, &mut mission_events, &database);
            }
            TriggerType::OnInteract => {
                handle_mission_on_interact(&ctx, &mut trigger, &keyboard, &mut events, &mut mission_events, &database);
            }
            TriggerType::OnEnterDelayed { delay } => {
                handle_mission_on_enter_delayed(&ctx, &mut trigger, track, delay, delta_ms, &mut events, &mut mission_events, &database);
            }
            TriggerType::OnExit => {
                handle_mission_on_exit(&ctx, &mut trigger, &mut events, &mut mission_events, &database);
            }
            TriggerType::OnStay { duration } => {
                handle_mission_on_stay(&ctx, &mut trigger, track, duration, delta_ms, &mut events, &mut mission_events, &database);
            }
        }
    }
}

// === 對話觸發類型處理輔助函數 ===

/// 處理對話 OnEnter 觸發
fn handle_dialogue_on_enter(
    ctx: &TriggerContext,
    trigger: &mut DialogueTrigger,
    events: &mut MessageWriter<TriggerEvent>,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
) {
    if !ctx.just_entered {
        return;
    }
    trigger.triggered = trigger.one_shot;
    events.write(TriggerEvent::PlayerEntered {
        entity: ctx.entity,
        trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
    });
    start_dialogue_from_trigger(trigger.dialogue_id, dialogue_events);
}

/// 處理對話 OnInteract 觸發
fn handle_dialogue_on_interact(
    ctx: &TriggerContext,
    trigger: &mut DialogueTrigger,
    keyboard: &ButtonInput<KeyCode>,
    events: &mut MessageWriter<TriggerEvent>,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
) {
    if !ctx.in_range || !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    trigger.triggered = trigger.one_shot;
    events.write(TriggerEvent::PlayerInteracted {
        entity: ctx.entity,
        trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
    });
    start_dialogue_from_trigger(trigger.dialogue_id, dialogue_events);
}

/// 處理對話 OnEnterDelayed 觸發
fn handle_dialogue_on_enter_delayed(
    ctx: &TriggerContext,
    trigger: &mut DialogueTrigger,
    track: &mut TriggerTrackingState,
    delay: u32,
    delta_ms: f32,
    events: &mut MessageWriter<TriggerEvent>,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
) {
    if ctx.just_entered {
        track.timer = 0.0;
        track.triggered = false;
    }
    if !ctx.in_range || track.triggered {
        return;
    }
    track.timer += delta_ms;
    if track.timer < delay as f32 {
        return;
    }
    track.triggered = true;
    trigger.triggered = trigger.one_shot;
    events.write(TriggerEvent::PlayerEntered {
        entity: ctx.entity,
        trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
    });
    start_dialogue_from_trigger(trigger.dialogue_id, dialogue_events);
}

/// 處理對話 OnExit 觸發
fn handle_dialogue_on_exit(
    ctx: &TriggerContext,
    trigger: &mut DialogueTrigger,
    events: &mut MessageWriter<TriggerEvent>,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
) {
    if !ctx.just_exited {
        return;
    }
    trigger.triggered = trigger.one_shot;
    events.write(TriggerEvent::PlayerExited {
        entity: ctx.entity,
        trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
    });
    start_dialogue_from_trigger(trigger.dialogue_id, dialogue_events);
}

/// 處理對話 OnStay 觸發
fn handle_dialogue_on_stay(
    ctx: &TriggerContext,
    trigger: &mut DialogueTrigger,
    track: &mut TriggerTrackingState,
    duration: u32,
    delta_ms: f32,
    events: &mut MessageWriter<TriggerEvent>,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
) {
    if ctx.just_entered {
        track.timer = 0.0;
        track.triggered = false;
    }
    if ctx.in_range && !track.triggered {
        track.timer += delta_ms;
        if track.timer >= duration as f32 {
            track.triggered = true;
            trigger.triggered = trigger.one_shot;
            events.write(TriggerEvent::PlayerStayed {
                entity: ctx.entity,
                trigger_type: TriggerEventType::Dialogue(trigger.dialogue_id),
                duration: track.timer / 1000.0,
            });
            start_dialogue_from_trigger(trigger.dialogue_id, dialogue_events);
        }
    } else if !ctx.in_range {
        track.timer = 0.0;
        track.triggered = false;
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
    mut tracking: Local<HashMap<Entity, TriggerTrackingState>>,
) {
    if is_dialogue_active(&dialogue_state) {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let delta_ms = time.delta_secs() * 1000.0;

    for (entity, transform, mut trigger) in &mut trigger_query {
        if !trigger.enabled || (trigger.triggered && trigger.one_shot) {
            continue;
        }

        let track = tracking.entry(entity).or_default();
        let ctx = TriggerContext::new(entity, transform.translation, player_pos, &trigger.shape, track.was_inside);
        track.was_inside = ctx.in_range;

        match trigger.trigger_type {
            TriggerType::OnEnter => {
                handle_dialogue_on_enter(&ctx, &mut trigger, &mut events, &mut dialogue_events);
            }
            TriggerType::OnInteract => {
                handle_dialogue_on_interact(&ctx, &mut trigger, &keyboard, &mut events, &mut dialogue_events);
            }
            TriggerType::OnEnterDelayed { delay } => {
                handle_dialogue_on_enter_delayed(&ctx, &mut trigger, track, delay, delta_ms, &mut events, &mut dialogue_events);
            }
            TriggerType::OnExit => {
                handle_dialogue_on_exit(&ctx, &mut trigger, &mut events, &mut dialogue_events);
            }
            TriggerType::OnStay { duration } => {
                handle_dialogue_on_stay(&ctx, &mut trigger, track, duration, delta_ms, &mut events, &mut dialogue_events);
            }
        }
    }
}

/// 從觸發器開始對話
fn start_dialogue_from_trigger(
    dialogue_id: DialogueId,
    events: &mut MessageWriter<DialogueEvent>,
) {
    events.write(DialogueEvent::Start {
        dialogue_id,
        participants: HashMap::new(),
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

// === NPC 互動輔助函數 ===

/// 嘗試透過 NPC 開始任務
fn try_start_npc_mission(
    mission_id: StoryMissionId,
    manager: &StoryMissionManager,
    database: &StoryMissionDatabase,
    mission_events: &mut MessageWriter<StoryMissionEvent>,
    dialogue_events: &mut MessageWriter<DialogueEvent>,
) -> bool {
    let status = manager.get_mission_status(mission_id);
    if status != StoryMissionStatus::Available {
        return false;
    }

    mission_events.write(StoryMissionEvent::Started(mission_id));

    // 播放任務開場對話（如果有）
    if let Some(dialogue_id) = get_mission_start_dialogue(mission_id, database) {
        dialogue_events.write(DialogueEvent::Start {
            dialogue_id,
            participants: HashMap::new(),
        });
    }
    true
}

/// 取得任務開場對話 ID
fn get_mission_start_dialogue(
    mission_id: StoryMissionId,
    database: &StoryMissionDatabase,
) -> Option<DialogueId> {
    let mission = database.get(mission_id)?;
    let first_phase = mission.phases.first()?;
    first_phase.start_dialogue
}

/// 播放 NPC 閒聊對話
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

        let distance_sq = player_pos.distance_squared(transform.translation);
        let radius_sq = npc.interaction_radius * npc.interaction_radius;
        if distance_sq > radius_sq || !keyboard.just_pressed(KeyCode::KeyF) {
            continue;
        }

        // 優先嘗試開始任務
        if let Some(mission_id) = npc.offers_mission {
            if try_start_npc_mission(mission_id, &manager, &database, &mut mission_events, &mut dialogue_events) {
                return;
            }
        }

        // 否則播放閒聊對話
        if let Some(dialogue_id) = npc.idle_dialogue {
            play_npc_idle_dialogue(dialogue_id, &mut dialogue_events);
        }
    }
}

// ============================================================================
// 目標追蹤系統
// ============================================================================

// === 目標追蹤輔助函數 ===

/// 檢查跟隨目標是否完成（目標到達終點）
fn check_follow_target_complete(
    target_id: &str,
    mission_id: StoryMissionId,
    target_query: &Query<(&Transform, &MissionTargetEntity)>,
) -> bool {
    for (_transform, target_entity) in target_query {
        if &target_entity.target_id == target_id && target_entity.mission_id == mission_id {
            // 如果目標已到達終點（路徑走完），任務完成
            let reached = !target_entity.waypoints.is_empty()
                && target_entity.current_waypoint >= target_entity.waypoints.len();
            return reached;
        }
    }
    false
}

/// 檢查護送 NPC 是否完成（NPC 到達目的地）
fn check_escort_npc_complete(
    target_id: &str,
    mission_id: StoryMissionId,
    target_query: &Query<(&Transform, &MissionTargetEntity)>,
) -> bool {
    for (_transform, target_entity) in target_query {
        if &target_entity.target_id == target_id
            && target_entity.mission_id == mission_id
            && target_entity.target_type == MissionTargetType::Escort
        {
            let reached = !target_entity.waypoints.is_empty()
                && target_entity.current_waypoint >= target_entity.waypoints.len();
            return reached;
        }
    }
    false
}

/// 檢查目標是否完成
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
    let phase_timer = active.phase_timer;

    for (index, objective) in active.objectives.iter_mut().enumerate() {
        if objective.is_completed {
            continue;
        }

        let completed = check_objective_complete(objective, player_pos, phase_timer, mission_id, &target_query);

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

// === 階段系統輔助函數 ===

/// 播放階段開始的對話和過場動畫
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

/// 清理任務生成的實體
fn cleanup_mission_entities(commands: &mut Commands, spawned_entities: Vec<Entity>) {
    for entity in spawned_entities {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }
}

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

    // 播放階段完成對話（如果有）
    if let Some(end_dialogue) = mission.get_phase(current_phase).and_then(|p| p.end_dialogue) {
        dialogue_events.write(DialogueEvent::Start {
            dialogue_id: end_dialogue,
            participants: HashMap::new(),
        });
        return;
    }

    let next_phase_index = current_phase + 1;

    if let Some(next_phase) = mission.get_phase(next_phase_index) {
        // 前進到下一階段
        active.advance_phase(next_phase);
        mission_events.write(StoryMissionEvent::PhaseChanged {
            mission_id,
            new_phase: next_phase_index as u32,
        });
        play_phase_start_events(next_phase, &mut dialogue_events, &mut cutscene_events);
        info!("任務 {} 進入階段 {}", mission_id, next_phase_index);
    } else {
        // 任務完成
        let rewards = mission.rewards.clone();
        manager.grant_rewards(&rewards);

        if let Some((completed_id, spawned_entities)) = manager.complete_current_mission() {
            cleanup_mission_entities(&mut commands, spawned_entities);
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

// === 失敗檢查輔助函數 ===

/// 檢查是否滿足失敗條件
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
        if marker.pulse_phase > TAU {
            marker.pulse_phase -= TAU;
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
            emissive: LinearRgba::new(1.0, 0.8, 0.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        // 旋轉光環
        ring_mesh: meshes.add(Torus::new(1.5, 0.15)),
        ring_material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.9, 0.0, 0.4),
            emissive: LinearRgba::new(1.0, 0.7, 0.0, 1.0),
            alpha_mode: AlphaMode::Blend,
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
    let mut missions_to_remove = HashSet::new();
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
                    .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
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
