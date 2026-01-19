//! 對話系統邏輯
//!
//! 處理對話顯示、打字效果、選項選擇等

use bevy::prelude::*;

use super::dialogue::*;
use super::story_data::DialogueId;
use super::story_manager::StoryMissionManager;

// ============================================================================
// 常數定義
// ============================================================================

/// 範例 NPC：老王的 ID
const NPC_LAO_WANG: u32 = 100;

/// 範例對話 ID
const DIALOGUE_ID_MISSION1: DialogueId = 1;
const DIALOGUE_ID_MISSION2: DialogueId = 2;

/// 對話動作事件（用於跨系統觸發）
#[derive(Message, Debug, Clone)]
pub enum DialogueActionEvent {
    /// 攝影機聚焦（GTA 風格對話特寫）
    CameraFocus { target: Vec3, duration: f32 },
    /// 播放動畫
    PlayAnimation { target_npc: u32, animation: String },
    /// 生成特效
    SpawnEffect { effect: String, position: Vec3 },
    /// 播放音效
    PlaySound(String),
    /// 設置天氣
    SetWeather(String),
    /// 設置時間
    SetTime(f32),
}

/// 對話系統 Plugin
pub struct DialogueSystemPlugin;

impl Plugin for DialogueSystemPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<DialogueState>()
            .init_resource::<DialogueDatabase>()
            .init_resource::<DialogueSettings>()
            .init_resource::<DialogueActionState>()
            // 事件
            .add_message::<DialogueEvent>()
            .add_message::<DialogueActionEvent>()
            // 啟動系統
            .add_systems(Startup, setup_sample_dialogues)
            // 更新系統
            .add_systems(
                Update,
                (
                    dialogue_event_handler,
                    dialogue_action_trigger_system, // 觸發節點動作
                    dialogue_action_executor_system, // 執行動作（攝影機等）
                    dialogue_typing_system,
                    dialogue_input_system,
                    dialogue_auto_advance_system,
                    dialogue_history_system, // 記錄對話歷史
                )
                    .chain(),
            );
    }
}

/// 設置範例對話
fn setup_sample_dialogues(mut database: ResMut<DialogueDatabase>) {
    // 註冊範例對話 1（任務 1 的開場對話）
    let dialogue1 = super::dialogue::create_sample_dialogue();
    database.register_dialogue(dialogue1);

    // 註冊範例對話 2（任務 2 的開場對話）
    let dialogue2 = create_mission2_dialogue();
    database.register_dialogue(dialogue2);

    // 註冊 NPC 資料
    database.register_npc(NpcDialogueData {
        id: NPC_LAO_WANG,
        name: "老王".to_string(),
        portrait: String::new(), // 暫無頭像
        voice_style: None,
    });

    info!("對話系統初始化完成，共 {} 個對話", database.dialogue_count());
}

/// 創建任務 2 的對話
fn create_mission2_dialogue() -> DialogueTree {
    let mut tree = DialogueTree::new(DIALOGUE_ID_MISSION2, "任務：收債");
    let speaker = DialogueSpeaker::Npc(NPC_LAO_WANG);

    // 節點 0：任務簡報
    tree.add_node(
        DialogueNode::new(0, speaker.clone(), "好，來談正事。")
            .with_emotion(SpeakerEmotion::Serious)
            .then(1),
    );

    // 節點 1：任務說明
    tree.add_node(
        DialogueNode::new(1, speaker.clone(), "工業區有個叫阿強的傢伙欠我一大筆錢。我需要你去「提醒」他該還錢了。")
            .with_emotion(SpeakerEmotion::Serious)
            .then(2),
    );

    // 節點 2：任務目標
    tree.add_node(
        DialogueNode::new(2, speaker.clone(), "他躲在倉庫裡，身邊有幾個打手。先解決掉他們，然後把阿強帶到我這來。")
            .with_emotion(SpeakerEmotion::Angry)
            .with_choice(DialogueChoice::simple("沒問題", 3))
            .with_choice(DialogueChoice::simple("報酬呢？", 4)),
    );

    // 節點 3：接受任務
    tree.add_node(
        DialogueNode::new(3, speaker.clone(), "好，地點我發到你手機了。記住，我要活的。")
            .with_emotion(SpeakerEmotion::Neutral)
            .with_choice(DialogueChoice::end("出發")),
    );

    // 節點 4：談報酬
    tree.add_node(
        DialogueNode::new(4, speaker, "事成之後，500 塊現金，加上我欠你一個人情。這個人情在這座島上可值不少。")
            .with_emotion(SpeakerEmotion::Smirk)
            .with_choice(DialogueChoice::simple("成交", 3)),
    );

    tree
}

/// 對話動作執行狀態
#[derive(Resource, Default)]
pub struct DialogueActionState {
    /// 當前攝影機聚焦目標
    pub camera_focus: Option<CameraFocusData>,
    /// 等待計時器
    pub wait_timer: f32,
    /// 是否正在等待
    pub is_waiting: bool,
    /// 上一個處理的節點 ID（用於檢測節點變化）
    last_processed_node: Option<(DialogueId, u32)>,
}

/// 攝影機聚焦資料
#[derive(Clone)]
pub struct CameraFocusData {
    /// 目標位置
    pub target: Vec3,
    /// 持續時間
    pub duration: f32,
    /// 已經過時間
    pub elapsed: f32,
    /// 原始攝影機位置
    pub original_position: Option<Vec3>,
}

/// 處理對話事件
pub fn dialogue_event_handler(
    mut events: MessageReader<DialogueEvent>,
    mut dialogue_state: ResMut<DialogueState>,
    database: Res<DialogueDatabase>,
    time: Res<Time>,
) {
    for event in events.read() {
        match event {
            DialogueEvent::Start { dialogue_id, participants } => {
                if let Some(tree) = database.get_dialogue(*dialogue_id) {
                    let mut active = ActiveDialogue::new(*dialogue_id, tree.start_node);
                    active.start_time = time.elapsed_secs();

                    // 添加參與者
                    for (key, value) in participants {
                        active.participants.insert(key.clone(), value.clone());
                    }

                    dialogue_state.active_dialogue = Some(active);
                    info!("對話開始: {} (ID: {})", tree.name, dialogue_id);
                } else {
                    warn!("找不到對話樹: {}", dialogue_id);
                }
            }
            DialogueEvent::GoToNode(node_id) => {
                if let Some(active) = &mut dialogue_state.active_dialogue {
                    active.current_node = *node_id;
                    active.typing_progress = 0.0;
                    active.typing_complete = false;
                    active.selected_choice = None;
                }
            }
            DialogueEvent::SelectChoice(choice_index) => {
                if let Some(active) = &mut dialogue_state.active_dialogue {
                    active.selected_choice = Some(*choice_index);
                }
            }
            DialogueEvent::SkipTyping => {
                if let Some(active) = &mut dialogue_state.active_dialogue {
                    active.typing_progress = 1.0;
                    active.typing_complete = true;
                }
            }
            DialogueEvent::End => {
                if let Some(active) = dialogue_state.active_dialogue.take() {
                    info!("對話結束: {}", active.dialogue_id);
                }
            }
            DialogueEvent::Completed(_dialogue_id) => {
                // 由其他系統處理完成回調
            }
        }
    }
}

/// 打字機效果系統
pub fn dialogue_typing_system(
    mut dialogue_state: ResMut<DialogueState>,
    database: Res<DialogueDatabase>,
    settings: Res<DialogueSettings>,
    time: Res<Time>,
) {
    let Some(active) = &mut dialogue_state.active_dialogue else {
        return;
    };

    if active.typing_complete {
        return;
    }

    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 計算打字速度
    let typing_speed = if node.typing_speed > 0.0 {
        node.typing_speed
    } else {
        settings.default_typing_speed
    };

    // 計算文字總長度
    let text = substitute_variables(&node.text, &active.participants);
    let total_chars = text.chars().count() as f32;

    if total_chars == 0.0 || typing_speed == 0.0 {
        active.typing_progress = 1.0;
        active.typing_complete = true;
        return;
    }

    // 更新進度
    let chars_per_second = typing_speed;
    let progress_per_second = chars_per_second / total_chars;
    active.typing_progress += progress_per_second * time.delta_secs();

    if active.typing_progress >= 1.0 {
        active.typing_progress = 1.0;
        active.typing_complete = true;
    }
}

/// 對話輸入處理系統
pub fn dialogue_input_system(
    dialogue_state: ResMut<DialogueState>,
    mut story_manager: ResMut<StoryMissionManager>,
    database: Res<DialogueDatabase>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut events: MessageWriter<DialogueEvent>,
) {
    let Some(active) = &dialogue_state.active_dialogue else {
        return;
    };

    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 空白鍵或滑鼠左鍵繼續
    let advance_pressed = keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::Enter)
        || mouse.just_pressed(MouseButton::Left);

    if advance_pressed {
        if !active.typing_complete {
            // 跳過打字效果
            events.write(DialogueEvent::SkipTyping);
        } else if node.choices.is_empty() {
            // 沒有選項，自動跳到下一節點或結束
            if let Some(next_node) = node.next_node {
                events.write(DialogueEvent::GoToNode(next_node));
            } else {
                // 對話結束
                process_dialogue_consequences(&tree.on_complete, &mut story_manager);
                events.write(DialogueEvent::Completed(active.dialogue_id));
                events.write(DialogueEvent::End);
            }
        }
    }

    // 數字鍵選擇選項
    if active.typing_complete && !node.choices.is_empty() {
        let valid_choices = get_valid_choices(node, &story_manager);

        for (index, _choice) in valid_choices.iter().enumerate() {
            let key = match index {
                0 => Some(KeyCode::Digit1),
                1 => Some(KeyCode::Digit2),
                2 => Some(KeyCode::Digit3),
                3 => Some(KeyCode::Digit4),
                _ => None,
            };

            if let Some(key) = key {
                if keyboard.just_pressed(key) {
                    select_choice(
                        index,
                        &valid_choices,
                        &mut story_manager,
                        &mut events,
                        tree,
                        active.dialogue_id,
                    );
                    break;
                }
            }
        }
    }
}

/// 自動前進系統（用於無選項節點）
pub fn dialogue_auto_advance_system(
    dialogue_state: ResMut<DialogueState>,
    database: Res<DialogueDatabase>,
    settings: Res<DialogueSettings>,
    time: Res<Time>,
    mut events: MessageWriter<DialogueEvent>,
    mut auto_advance_timer: Local<f32>,
) {
    let Some(active) = &dialogue_state.active_dialogue else {
        *auto_advance_timer = 0.0;
        return;
    };

    if !active.typing_complete {
        *auto_advance_timer = 0.0;
        return;
    }

    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 只有在沒有選項且有設置自動前進時才自動前進
    if node.choices.is_empty() && node.auto_advance_delay > 0.0 {
        *auto_advance_timer += time.delta_secs();

        if *auto_advance_timer >= node.auto_advance_delay {
            *auto_advance_timer = 0.0;

            if let Some(next_node) = node.next_node {
                events.write(DialogueEvent::GoToNode(next_node));
            } else {
                events.write(DialogueEvent::End);
            }
        }
    } else if settings.auto_advance && node.choices.is_empty() {
        // 全局自動前進設置
        *auto_advance_timer += time.delta_secs();

        if *auto_advance_timer >= settings.auto_advance_delay {
            *auto_advance_timer = 0.0;

            if let Some(next_node) = node.next_node {
                events.write(DialogueEvent::GoToNode(next_node));
            } else {
                events.write(DialogueEvent::End);
            }
        }
    }
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 替換文字中的變數
pub fn substitute_variables(text: &str, participants: &std::collections::HashMap<String, String>) -> String {
    let mut result = text.to_string();

    for (key, value) in participants {
        let pattern = format!("{{{}}}", key);
        result = result.replace(&pattern, value);
    }

    result
}

/// 取得當前顯示的文字（根據打字進度）
pub fn get_displayed_text(text: &str, progress: f32, participants: &std::collections::HashMap<String, String>) -> String {
    let full_text = substitute_variables(text, participants);
    let total_chars = full_text.chars().count();
    let visible_chars = (total_chars as f32 * progress).round() as usize;

    full_text.chars().take(visible_chars).collect()
}

/// 取得有效的選項（根據條件過濾）
pub fn get_valid_choices<'a>(
    node: &'a DialogueNode,
    story_manager: &StoryMissionManager,
) -> Vec<&'a DialogueChoice> {
    node.choices
        .iter()
        .filter(|choice| {
            if let Some(condition) = &choice.condition {
                check_dialogue_condition(condition, story_manager)
            } else {
                true
            }
        })
        .collect()
}

/// 檢查對話條件
pub fn check_dialogue_condition(
    condition: &DialogueCondition,
    story_manager: &StoryMissionManager,
) -> bool {
    match condition {
        DialogueCondition::HasFlag(flag) => story_manager.get_flag(flag),
        DialogueCondition::NotHasFlag(flag) => !story_manager.get_flag(flag),
        DialogueCondition::HasMoney(amount) => story_manager.player_money >= *amount,
        DialogueCondition::HasItem { item_id, count: _ } => {
            story_manager.is_item_unlocked(item_id)
        }
        DialogueCondition::MissionCompleted(mission_id) => {
            story_manager.get_mission_status(*mission_id)
                == super::story_data::StoryMissionStatus::Completed
        }
        DialogueCondition::RelationshipMin { npc_id, min } => {
            story_manager.get_relationship(*npc_id) >= *min
        }
        DialogueCondition::All(conditions) => {
            conditions.iter().all(|c| check_dialogue_condition(c, story_manager))
        }
        DialogueCondition::Any(conditions) => {
            conditions.iter().any(|c| check_dialogue_condition(c, story_manager))
        }
    }
}

/// 選擇對話選項
fn select_choice(
    choice_index: usize,
    valid_choices: &[&DialogueChoice],
    story_manager: &mut StoryMissionManager,
    events: &mut MessageWriter<DialogueEvent>,
    tree: &DialogueTree,
    dialogue_id: DialogueId,
) {
    if let Some(choice) = valid_choices.get(choice_index) {
        // 處理選項後果
        process_dialogue_consequences(&choice.consequences, story_manager);

        // 跳轉或結束
        if choice.ends_dialogue {
            process_dialogue_consequences(&tree.on_complete, story_manager);
            events.write(DialogueEvent::Completed(dialogue_id));
            events.write(DialogueEvent::End);
        } else if let Some(next_node) = choice.next_node {
            events.write(DialogueEvent::GoToNode(next_node));
        }
    }
}

/// 處理對話後果
pub fn process_dialogue_consequences(
    consequences: &[DialogueConsequence],
    story_manager: &mut StoryMissionManager,
) {
    for consequence in consequences {
        match consequence {
            DialogueConsequence::SetStoryFlag { flag, value } => {
                story_manager.set_flag(flag.clone(), *value);
            }
            DialogueConsequence::ChangeRelationship { npc_id, delta } => {
                story_manager.change_relationship(*npc_id, *delta);
            }
            DialogueConsequence::GiveMoney(amount) => {
                story_manager.add_money(*amount);
            }
            DialogueConsequence::GiveItem { item_id, count: _ } => {
                story_manager.unlock_item(item_id.clone());
            }
            DialogueConsequence::UnlockMission(mission_id) => {
                story_manager.unlock_mission(*mission_id);
            }
            DialogueConsequence::FailMission => {
                story_manager.fail_current_mission(super::story_data::FailCondition::PlayerDeath);
            }
            DialogueConsequence::SkipPhase => {
                // 由任務系統處理
            }
            DialogueConsequence::TriggerCombat => {
                // 由戰鬥系統處理
            }
            DialogueConsequence::PlaySound(_sound) => {
                // 由音效系統處理
            }
            DialogueConsequence::CustomEvent(_event) => {
                // 由自定義系統處理
            }
        }
    }
}

/// 開始對話的便利函數
pub fn start_dialogue(
    dialogue_id: DialogueId,
    events: &mut MessageWriter<DialogueEvent>,
) {
    events.write(DialogueEvent::Start {
        dialogue_id,
        participants: std::collections::HashMap::new(),
    });
}

/// 開始對話（帶參與者）的便利函數
pub fn start_dialogue_with_participants(
    dialogue_id: DialogueId,
    participants: std::collections::HashMap<String, String>,
    events: &mut MessageWriter<DialogueEvent>,
) {
    events.write(DialogueEvent::Start {
        dialogue_id,
        participants,
    });
}

/// 檢查是否有對話進行中
pub fn is_dialogue_active(dialogue_state: &DialogueState) -> bool {
    dialogue_state.active_dialogue.is_some()
}

/// 取得當前對話節點
pub fn get_current_dialogue_node<'a>(
    dialogue_state: &DialogueState,
    database: &'a DialogueDatabase,
) -> Option<&'a DialogueNode> {
    let active = dialogue_state.active_dialogue.as_ref()?;
    let tree = database.get_dialogue(active.dialogue_id)?;
    tree.get_node(active.current_node)
}

// ============================================================================
// 對話動作系統（GTA 5 風格）
// ============================================================================

/// 觸發對話節點動作
fn dialogue_action_trigger_system(
    dialogue_state: Res<DialogueState>,
    database: Res<DialogueDatabase>,
    mut action_state: ResMut<DialogueActionState>,
    mut action_events: MessageWriter<DialogueActionEvent>,
) {
    let Some(active) = &dialogue_state.active_dialogue else {
        // 對話結束，清理狀態
        action_state.last_processed_node = None;
        action_state.camera_focus = None;
        action_state.is_waiting = false;
        return;
    };

    let current_key = (active.dialogue_id, active.current_node);

    // 檢查是否已處理過這個節點
    if action_state.last_processed_node == Some(current_key) {
        return;
    }

    // 標記為已處理
    action_state.last_processed_node = Some(current_key);

    // 取得節點
    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 執行節點的所有動作
    for action in &node.actions {
        match action {
            DialogueAction::None => {}
            DialogueAction::CameraFocus { target, duration } => {
                action_events.write(DialogueActionEvent::CameraFocus {
                    target: *target,
                    duration: *duration,
                });
                // 設置攝影機聚焦狀態
                action_state.camera_focus = Some(CameraFocusData {
                    target: *target,
                    duration: *duration,
                    elapsed: 0.0,
                    original_position: None,
                });
                info!("對話動作: 攝影機聚焦 {:?}", target);
            }
            DialogueAction::PlayAnimation { target, animation } => {
                action_events.write(DialogueActionEvent::PlayAnimation {
                    target_npc: *target,
                    animation: animation.clone(),
                });
                info!("對話動作: 播放動畫 {} on NPC {}", animation, target);
            }
            DialogueAction::SpawnEffect { effect, position } => {
                action_events.write(DialogueActionEvent::SpawnEffect {
                    effect: effect.clone(),
                    position: *position,
                });
                info!("對話動作: 生成特效 {} at {:?}", effect, position);
            }
            DialogueAction::PlaySound(sound) => {
                action_events.write(DialogueActionEvent::PlaySound(sound.clone()));
                info!("對話動作: 播放音效 {}", sound);
            }
            DialogueAction::Wait(duration) => {
                action_state.is_waiting = true;
                action_state.wait_timer = *duration;
                info!("對話動作: 等待 {} 秒", duration);
            }
            DialogueAction::SetWeather(weather) => {
                action_events.write(DialogueActionEvent::SetWeather(weather.clone()));
                info!("對話動作: 設置天氣 {}", weather);
            }
            DialogueAction::SetTime(time) => {
                action_events.write(DialogueActionEvent::SetTime(*time));
                info!("對話動作: 設置時間 {}", time);
            }
        }
    }
}

/// 執行對話動作（攝影機聚焦等）
fn dialogue_action_executor_system(
    mut action_state: ResMut<DialogueActionState>,
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
) {
    // 處理等待計時器
    if action_state.is_waiting {
        action_state.wait_timer -= time.delta_secs();
        if action_state.wait_timer <= 0.0 {
            action_state.is_waiting = false;
            action_state.wait_timer = 0.0;
        }
    }

    // 處理攝影機聚焦（GTA 5 風格平滑過渡）
    if let Some(focus) = &mut action_state.camera_focus {
        focus.elapsed += time.delta_secs();

        for mut camera_transform in &mut camera_query {
            // 記錄原始位置
            if focus.original_position.is_none() {
                focus.original_position = Some(camera_transform.translation);
            }

            let progress = (focus.elapsed / focus.duration).clamp(0.0, 1.0);

            // 使用 ease-out 曲線平滑過渡（GTA 風格）
            let eased_progress = 1.0 - (1.0 - progress).powi(3);

            if let Some(original) = focus.original_position {
                // 計算聚焦位置（在目標前方偏移以獲得好的視角）
                let focus_offset = Vec3::new(0.0, 2.0, 5.0);
                let focus_position = focus.target + focus_offset;

                // 插值攝影機位置
                camera_transform.translation = original.lerp(focus_position, eased_progress);

                // 讓攝影機看向目標
                camera_transform.look_at(focus.target + Vec3::Y * 1.5, Vec3::Y);
            }
        }

        // 聚焦完成後清理
        if focus.elapsed >= focus.duration {
            action_state.camera_focus = None;
        }
    }
}

/// 對話歷史記錄系統
fn dialogue_history_system(
    mut dialogue_state: ResMut<DialogueState>,
    database: Res<DialogueDatabase>,
    time: Res<Time>,
    mut last_recorded_node: Local<Option<(DialogueId, u32)>>,
) {
    let Some(active) = &dialogue_state.active_dialogue else {
        *last_recorded_node = None;
        return;
    };

    // 只有打字完成時才記錄
    if !active.typing_complete {
        return;
    }

    let current_key = (active.dialogue_id, active.current_node);

    // 已記錄過這個節點
    if *last_recorded_node == Some(current_key) {
        return;
    }

    *last_recorded_node = Some(current_key);

    // 取得節點
    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 取得說話者名稱
    let speaker_name = match &node.speaker_name {
        Some(name) => name.clone(),
        None => match node.speaker {
            DialogueSpeaker::Player => "玩家".to_string(),
            DialogueSpeaker::Npc(npc_id) => {
                database.get_npc(npc_id)
                    .map(|npc| npc.name.clone())
                    .unwrap_or_else(|| format!("NPC {}", npc_id))
            }
            DialogueSpeaker::Narrator => "旁白".to_string(),
            DialogueSpeaker::Radio => "電台".to_string(),
            DialogueSpeaker::System => "系統".to_string(),
        },
    };

    // 添加到歷史記錄
    let text = substitute_variables(&node.text, &active.participants);
    let entry = DialogueHistoryEntry {
        speaker_name,
        text,
        timestamp: time.elapsed_secs(),
    };

    dialogue_state.history.push(entry);

    // 限制歷史記錄數量
    let max_history = if dialogue_state.max_history > 0 {
        dialogue_state.max_history
    } else {
        50 // 預設最大 50 條
    };

    while dialogue_state.history.len() > max_history {
        dialogue_state.history.remove(0);
    }
}
