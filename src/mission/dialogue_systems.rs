//! 對話系統邏輯
//!
//! 處理對話顯示、打字效果、選項選擇等

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use std::collections::HashMap;

use super::dialogue::*;
use super::relationship::RelationshipManager;
use super::story_data::DialogueId;
use super::story_manager::StoryMissionManager;
use super::unlocks::UnlockManager;
use crate::economy::PlayerWallet;

// ============================================================================
// 常數定義
// ============================================================================

/// 範例 NPC：老王的 ID
const NPC_LAO_WANG: u32 = 100;

/// 範例對話 ID
const DIALOGUE_ID_MISSION2: DialogueId = 2;

use super::dialogue_actions::*;

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
                    dialogue_action_trigger_system,  // 觸發節點動作
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
    let dialogue1 = create_sample_dialogue();
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

    // 註冊支線任務對話（對話 ID 200-211，含分支選項影響好感度）
    super::side_dialogues::register_side_dialogues(&mut database);

    info!(
        "對話系統初始化完成，共 {} 個對話",
        database.dialogue_count()
    );
}

/// 創建任務 2 的對話
fn create_mission2_dialogue() -> DialogueTree {
    let mut tree = DialogueTree::new(DIALOGUE_ID_MISSION2, "任務：收債");
    let speaker = DialogueSpeaker::Npc(NPC_LAO_WANG);

    // 節點 0：任務簡報
    tree.add_node(
        DialogueNode::new(0, speaker, "好，來談正事。")
            .with_emotion(SpeakerEmotion::Serious)
            .then(1),
    );

    // 節點 1：任務說明
    tree.add_node(
        DialogueNode::new(
            1,
            speaker,
            "工業區有個叫阿強的傢伙欠我一大筆錢。我需要你去「提醒」他該還錢了。",
        )
        .with_emotion(SpeakerEmotion::Serious)
        .then(2),
    );

    // 節點 2：任務目標
    tree.add_node(
        DialogueNode::new(
            2,
            speaker,
            "他躲在倉庫裡，身邊有幾個打手。先解決掉他們，然後把阿強帶到我這來。",
        )
        .with_emotion(SpeakerEmotion::Angry)
        .with_choice(DialogueChoice::simple("沒問題", 3))
        .with_choice(DialogueChoice::simple("報酬呢？", 4)),
    );

    // 節點 3：接受任務
    tree.add_node(
        DialogueNode::new(
            3,
            speaker,
            "好，地點我發到你手機了。記住，我要活的。",
        )
        .with_emotion(SpeakerEmotion::Neutral)
        .with_choice(DialogueChoice::end("出發")),
    );

    // 節點 4：談報酬
    tree.add_node(
        DialogueNode::new(
            4,
            speaker,
            "事成之後，500 塊現金，加上我欠你一個人情。這個人情在這座島上可值不少。",
        )
        .with_emotion(SpeakerEmotion::Smirk)
        .with_choice(DialogueChoice::simple("成交", 3)),
    );

    tree
}

// ============================================================================
// 對話事件處理輔助函數
// ============================================================================
/// 處理對話開始事件
fn handle_dialogue_start(
    dialogue_id: DialogueId,
    participants: &HashMap<String, String>,
    dialogue_state: &mut DialogueState,
    database: &DialogueDatabase,
    elapsed_secs: f32,
) {
    let Some(tree) = database.get_dialogue(dialogue_id) else {
        warn!("找不到對話樹: {}", dialogue_id);
        return;
    };

    let mut active = ActiveDialogue::new(dialogue_id, tree.start_node);
    active.start_time = elapsed_secs;

    for (key, value) in participants.iter() {
        active
            .participants
            .insert(key.to_string(), value.to_string());
    }

    dialogue_state.active_dialogue = Some(active);
    info!("💬 對話開始: {} (ID: {})", tree.name, dialogue_id);
}

/// 處理跳轉節點事件
fn handle_go_to_node(active: &mut ActiveDialogue, node_id: u32) {
    active.current_node = node_id;
    active.typing_progress = 0.0;
    active.typing_complete = false;
    active.selected_choice = None;
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
            DialogueEvent::Start {
                dialogue_id,
                participants,
            } => {
                handle_dialogue_start(
                    *dialogue_id,
                    participants,
                    &mut dialogue_state,
                    &database,
                    time.elapsed_secs(),
                );
            }
            DialogueEvent::GoToNode(node_id) => {
                if let Some(active) = &mut dialogue_state.active_dialogue {
                    handle_go_to_node(active, *node_id);
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
            DialogueEvent::Completed(_) => {} // 由其他系統處理
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

// ============================================================================
// 對話輸入輔助函數
// ============================================================================
/// 取得選項對應的數字鍵
fn get_choice_key(index: usize) -> Option<KeyCode> {
    match index {
        0 => Some(KeyCode::Digit1),
        1 => Some(KeyCode::Digit2),
        2 => Some(KeyCode::Digit3),
        3 => Some(KeyCode::Digit4),
        _ => None,
    }
}

/// 處理對話前進（無選項時）
fn handle_dialogue_advance(
    node: &DialogueNode,
    tree: &DialogueTree,
    dialogue_id: DialogueId,
    story_manager: &mut StoryMissionManager,
    wallet: &mut PlayerWallet,
    unlocks: &mut UnlockManager,
    relationship: &mut RelationshipManager,
    events: &mut MessageWriter<DialogueEvent>,
) {
    if let Some(next_node) = node.next_node {
        events.write(DialogueEvent::GoToNode(next_node));
    } else {
        process_dialogue_consequences(
            &tree.on_complete,
            story_manager,
            wallet,
            unlocks,
            relationship,
        );
        events.write(DialogueEvent::Completed(dialogue_id));
        events.write(DialogueEvent::End);
    }
}

/// 處理數字鍵選擇選項
fn handle_choice_selection(
    keyboard: &ButtonInput<KeyCode>,
    valid_choices: &[&DialogueChoice],
    story_manager: &mut StoryMissionManager,
    wallet: &mut PlayerWallet,
    unlocks: &mut UnlockManager,
    relationship: &mut RelationshipManager,
    events: &mut MessageWriter<DialogueEvent>,
    tree: &DialogueTree,
    dialogue_id: DialogueId,
) {
    for (index, _) in valid_choices.iter().enumerate() {
        let Some(key) = get_choice_key(index) else {
            continue;
        };
        if keyboard.just_pressed(key) {
            select_choice(
                index,
                valid_choices,
                story_manager,
                wallet,
                unlocks,
                relationship,
                events,
                tree,
                dialogue_id,
            );
            return;
        }
    }
}

/// 對話輸入處理系統
pub fn dialogue_input_system(
    dialogue_state: ResMut<DialogueState>,
    mut story_manager: ResMut<StoryMissionManager>,
    mut wallet: ResMut<PlayerWallet>,
    mut unlocks: ResMut<UnlockManager>,
    mut relationship: ResMut<RelationshipManager>,
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

    let advance_pressed = keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::Enter)
        || mouse.just_pressed(MouseButton::Left);

    if advance_pressed {
        if !active.typing_complete {
            events.write(DialogueEvent::SkipTyping);
        } else if node.choices.is_empty() {
            handle_dialogue_advance(
                node,
                tree,
                active.dialogue_id,
                &mut story_manager,
                &mut wallet,
                &mut unlocks,
                &mut relationship,
                &mut events,
            );
        }
    }

    // 數字鍵選擇選項
    if active.typing_complete && !node.choices.is_empty() {
        let valid_choices =
            get_valid_choices(node, &story_manager, &wallet, &unlocks, &relationship);
        handle_choice_selection(
            &keyboard,
            &valid_choices,
            &mut story_manager,
            &mut wallet,
            &mut unlocks,
            &mut relationship,
            &mut events,
            tree,
            active.dialogue_id,
        );
    }
}

// ============================================================================
// 自動前進輔助函數
// ============================================================================
/// 執行自動前進（跳轉或結束對話）
fn perform_auto_advance(next_node: Option<u32>, events: &mut MessageWriter<DialogueEvent>) {
    match next_node {
        Some(node_id) => {
            events.write(DialogueEvent::GoToNode(node_id));
        }
        None => {
            events.write(DialogueEvent::End);
        }
    }
}

/// 檢查並處理自動前進計時器
fn try_auto_advance(
    timer: &mut f32,
    delay: f32,
    delta: f32,
    next_node: Option<u32>,
    events: &mut MessageWriter<DialogueEvent>,
) -> bool {
    *timer += delta;
    if *timer >= delay {
        *timer = 0.0;
        perform_auto_advance(next_node, events);
        true
    } else {
        false
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

    // 無選項時才考慮自動前進
    if !node.choices.is_empty() {
        return;
    }

    let delta = time.delta_secs();

    // 優先使用節點設置的延遲，否則使用全局設置
    if node.auto_advance_delay > 0.0 {
        try_auto_advance(
            &mut auto_advance_timer,
            node.auto_advance_delay,
            delta,
            node.next_node,
            &mut events,
        );
    } else if settings.auto_advance {
        try_auto_advance(
            &mut auto_advance_timer,
            settings.auto_advance_delay,
            delta,
            node.next_node,
            &mut events,
        );
    }
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 替換文字中的變數
pub fn substitute_variables(text: &str, participants: &HashMap<String, String>) -> String {
    let mut result = text.to_string();

    for (key, value) in participants {
        let pattern = format!("{{{}}}", key);
        result = result.replace(&pattern, value);
    }

    result
}

/// 取得當前顯示的文字（根據打字進度）
pub fn get_displayed_text(
    text: &str,
    progress: f32,
    participants: &HashMap<String, String>,
) -> String {
    let full_text = substitute_variables(text, participants);
    let total_chars = full_text.chars().count();
    let visible_chars = (total_chars as f32 * progress).round() as usize;

    full_text.chars().take(visible_chars).collect()
}

/// 取得有效的選項（根據條件過濾）
pub fn get_valid_choices<'a>(
    node: &'a DialogueNode,
    story_manager: &StoryMissionManager,
    wallet: &PlayerWallet,
    unlocks: &UnlockManager,
    relationship: &RelationshipManager,
) -> Vec<&'a DialogueChoice> {
    node.choices
        .iter()
        .filter(|choice| {
            if let Some(condition) = &choice.condition {
                check_dialogue_condition(condition, story_manager, wallet, unlocks, relationship)
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
    wallet: &PlayerWallet,
    unlocks: &UnlockManager,
    relationship: &RelationshipManager,
) -> bool {
    match condition {
        DialogueCondition::HasFlag(flag) => story_manager.get_flag(flag),
        DialogueCondition::NotHasFlag(flag) => !story_manager.get_flag(flag),
        DialogueCondition::HasMoney(amount) => wallet.cash >= *amount,
        DialogueCondition::HasItem { item_id, count: _ } => unlocks.is_item_unlocked(item_id),
        DialogueCondition::MissionCompleted(mission_id) => {
            story_manager.get_mission_status(*mission_id)
                == super::story_data::StoryMissionStatus::Completed
        }
        DialogueCondition::RelationshipMin { npc_id, min } => {
            relationship.get_relationship(*npc_id) >= *min
        }
        DialogueCondition::All(conditions) => conditions
            .iter()
            .all(|c| check_dialogue_condition(c, story_manager, wallet, unlocks, relationship)),
        DialogueCondition::Any(conditions) => conditions
            .iter()
            .any(|c| check_dialogue_condition(c, story_manager, wallet, unlocks, relationship)),
    }
}

/// 選擇對話選項
fn select_choice(
    choice_index: usize,
    valid_choices: &[&DialogueChoice],
    story_manager: &mut StoryMissionManager,
    wallet: &mut PlayerWallet,
    unlocks: &mut UnlockManager,
    relationship: &mut RelationshipManager,
    events: &mut MessageWriter<DialogueEvent>,
    tree: &DialogueTree,
    dialogue_id: DialogueId,
) {
    if let Some(choice) = valid_choices.get(choice_index) {
        // 處理選項後果
        process_dialogue_consequences(
            &choice.consequences,
            story_manager,
            wallet,
            unlocks,
            relationship,
        );

        // 跳轉或結束
        if choice.ends_dialogue {
            process_dialogue_consequences(
                &tree.on_complete,
                story_manager,
                wallet,
                unlocks,
                relationship,
            );
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
    wallet: &mut PlayerWallet,
    unlocks: &mut UnlockManager,
    relationship: &mut RelationshipManager,
) {
    for consequence in consequences {
        match consequence {
            DialogueConsequence::SetStoryFlag { flag, value } => {
                story_manager.set_flag(flag.clone(), *value);
            }
            DialogueConsequence::ChangeRelationship { npc_id, delta } => {
                relationship.change_relationship(*npc_id, *delta);
            }
            DialogueConsequence::GiveMoney(amount) => {
                wallet.add_cash(*amount);
            }
            DialogueConsequence::GiveItem { item_id, count: _ } => {
                unlocks.unlock_item(item_id.clone());
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
pub fn start_dialogue(dialogue_id: DialogueId, events: &mut MessageWriter<DialogueEvent>) {
    events.write(DialogueEvent::Start {
        dialogue_id,
        participants: HashMap::new(),
    });
}

/// 開始對話（帶參與者）的便利函數
pub fn start_dialogue_with_participants(
    dialogue_id: DialogueId,
    participants: HashMap<String, String>,
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
            DialogueSpeaker::Npc(npc_id) => database
                .get_npc(npc_id)
                .map(|npc| npc.name.clone())
                .unwrap_or_else(|| format!("NPC {}", npc_id)),
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

    dialogue_state.history.push_back(entry);

    // 限制歷史記錄數量 - O(1) 操作
    let max_history = if dialogue_state.max_history > 0 {
        dialogue_state.max_history
    } else {
        50 // 預設最大 50 條
    };

    while dialogue_state.history.len() > max_history {
        dialogue_state.history.pop_front();
    }
}
