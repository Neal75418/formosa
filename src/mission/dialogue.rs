//! 對話系統資料結構
//!
//! 定義對話樹、對話節點、選項分支等資料結構

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::story_data::{DialogueId, NpcId, StoryMissionId};

/// 對話說話者類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DialogueSpeaker {
    /// 玩家角色
    Player,
    /// 特定 NPC（用 NpcId 識別）
    Npc(NpcId),
    /// 旁白（無頭像）
    Narrator,
    /// 電台/電話（特殊 UI）
    Radio,
    /// 系統訊息
    System,
}

impl Default for DialogueSpeaker {
    fn default() -> Self {
        Self::Narrator
    }
}

/// 說話者情緒（影響頭像表情）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SpeakerEmotion {
    #[default]
    Neutral,
    Happy,
    Angry,
    Sad,
    Surprised,
    Afraid,
    Disgusted,
    Smirk,
    Serious,
    Thinking,
}

impl SpeakerEmotion {
    /// 取得情緒對應的頭像後綴
    pub fn portrait_suffix(&self) -> &'static str {
        match self {
            Self::Neutral => "",
            Self::Happy => "_happy",
            Self::Angry => "_angry",
            Self::Sad => "_sad",
            Self::Surprised => "_surprised",
            Self::Afraid => "_afraid",
            Self::Disgusted => "_disgusted",
            Self::Smirk => "_smirk",
            Self::Serious => "_serious",
            Self::Thinking => "_thinking",
        }
    }
}

/// 對話選項的後果類型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueConsequence {
    /// 設置劇情旗標
    SetStoryFlag { flag: String, value: bool },
    /// 改變好感度
    ChangeRelationship { npc_id: NpcId, delta: i32 },
    /// 給予金錢
    GiveMoney(i32),
    /// 給予物品
    GiveItem { item_id: String, count: u32 },
    /// 解鎖任務
    UnlockMission(StoryMissionId),
    /// 失敗當前任務
    FailMission,
    /// 跳過任務階段
    SkipPhase,
    /// 觸發戰鬥
    TriggerCombat,
    /// 播放音效
    PlaySound(String),
    /// 自定義事件
    CustomEvent(String),
}

/// 對話選項
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    /// 選項顯示文字
    pub text: String,
    /// 選擇此選項後跳轉到的節點 ID
    pub next_node: Option<u32>,
    /// 選擇此選項的條件（劇情旗標）
    pub condition: Option<DialogueCondition>,
    /// 選擇此選項的後果
    pub consequences: Vec<DialogueConsequence>,
    /// 是否為「結束對話」選項
    pub ends_dialogue: bool,
}

impl DialogueChoice {
    /// 創建簡單的選項（只跳轉）
    pub fn simple(text: impl Into<String>, next_node: u32) -> Self {
        Self {
            text: text.into(),
            next_node: Some(next_node),
            condition: None,
            consequences: Vec::new(),
            ends_dialogue: false,
        }
    }

    /// 創建結束對話的選項
    pub fn end(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            next_node: None,
            condition: None,
            consequences: Vec::new(),
            ends_dialogue: true,
        }
    }

    /// 添加條件
    pub fn with_condition(mut self, condition: DialogueCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    /// 添加後果
    pub fn with_consequence(mut self, consequence: DialogueConsequence) -> Self {
        self.consequences.push(consequence);
        self
    }
}

/// 對話條件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueCondition {
    /// 需要特定劇情旗標
    HasFlag(String),
    /// 需要旗標為 false
    NotHasFlag(String),
    /// 需要足夠金錢
    HasMoney(i32),
    /// 需要特定物品
    HasItem { item_id: String, count: u32 },
    /// 需要完成特定任務
    MissionCompleted(StoryMissionId),
    /// 好感度達到門檻
    RelationshipMin { npc_id: NpcId, min: i32 },
    /// 多個條件都滿足
    All(Vec<DialogueCondition>),
    /// 任一條件滿足
    Any(Vec<DialogueCondition>),
}

/// 對話節點動作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueAction {
    /// 無特殊動作
    None,
    /// 攝影機特寫某個位置
    CameraFocus { target: Vec3, duration: f32 },
    /// 播放動畫
    PlayAnimation { target: NpcId, animation: String },
    /// 生成特效
    SpawnEffect { effect: String, position: Vec3 },
    /// 播放音效
    PlaySound(String),
    /// 等待一段時間
    Wait(f32),
    /// 設置天氣
    SetWeather(String),
    /// 設置時間
    SetTime(f32),
}

/// 對話節點
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueNode {
    /// 節點 ID（在對話樹中唯一）
    pub id: u32,
    /// 說話者
    pub speaker: DialogueSpeaker,
    /// 說話者顯示名稱（覆蓋預設）
    pub speaker_name: Option<String>,
    /// 對話文字（支援 {player_name} 等變數）
    pub text: String,
    /// 說話者情緒
    pub emotion: SpeakerEmotion,
    /// 頭像路徑（覆蓋預設）
    pub portrait_override: Option<String>,
    /// 語音檔案路徑
    pub voice_clip: Option<String>,
    /// 文字顯示速度（每秒字數，0 = 立即顯示）
    pub typing_speed: f32,
    /// 對話選項（若為空則自動跳到下一節點）
    pub choices: Vec<DialogueChoice>,
    /// 自動跳轉到的下一節點（choices 為空時使用）
    pub next_node: Option<u32>,
    /// 自動跳轉前的等待時間
    pub auto_advance_delay: f32,
    /// 節點觸發的動作
    pub actions: Vec<DialogueAction>,
}

impl Default for DialogueNode {
    fn default() -> Self {
        Self {
            id: 0,
            speaker: DialogueSpeaker::Narrator,
            speaker_name: None,
            text: String::new(),
            emotion: SpeakerEmotion::Neutral,
            portrait_override: None,
            voice_clip: None,
            typing_speed: 30.0, // 每秒 30 字
            choices: Vec::new(),
            next_node: None,
            auto_advance_delay: 0.0,
            actions: Vec::new(),
        }
    }
}

impl DialogueNode {
    /// 創建簡單的對話節點
    pub fn new(id: u32, speaker: DialogueSpeaker, text: impl Into<String>) -> Self {
        Self {
            id,
            speaker,
            text: text.into(),
            ..Default::default()
        }
    }

    /// 設置情緒
    pub fn with_emotion(mut self, emotion: SpeakerEmotion) -> Self {
        self.emotion = emotion;
        self
    }

    /// 設置下一節點
    pub fn then(mut self, next_node: u32) -> Self {
        self.next_node = Some(next_node);
        self
    }

    /// 添加選項
    pub fn with_choice(mut self, choice: DialogueChoice) -> Self {
        self.choices.push(choice);
        self
    }

    /// 添加動作
    pub fn with_action(mut self, action: DialogueAction) -> Self {
        self.actions.push(action);
        self
    }

    /// 設置打字速度
    pub fn with_typing_speed(mut self, speed: f32) -> Self {
        self.typing_speed = speed;
        self
    }

    /// 設置自動跳轉延遲
    pub fn with_auto_advance(mut self, delay: f32) -> Self {
        self.auto_advance_delay = delay;
        self
    }
}

/// 完整的對話樹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTree {
    /// 對話樹 ID
    pub id: DialogueId,
    /// 對話樹名稱（用於編輯器）
    pub name: String,
    /// 起始節點 ID
    pub start_node: u32,
    /// 所有節點（以 ID 為 key）
    pub nodes: HashMap<u32, DialogueNode>,
    /// 對話結束後的回呼
    pub on_complete: Vec<DialogueConsequence>,
}

impl DialogueTree {
    /// 創建新的對話樹
    pub fn new(id: DialogueId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            start_node: 0,
            nodes: HashMap::new(),
            on_complete: Vec::new(),
        }
    }

    /// 添加節點
    pub fn add_node(&mut self, node: DialogueNode) {
        self.nodes.insert(node.id, node);
    }

    /// 取得節點
    pub fn get_node(&self, id: u32) -> Option<&DialogueNode> {
        self.nodes.get(&id)
    }

    /// 設置起始節點
    pub fn with_start_node(mut self, id: u32) -> Self {
        self.start_node = id;
        self
    }

    /// 添加完成回呼
    pub fn on_complete(mut self, consequence: DialogueConsequence) -> Self {
        self.on_complete.push(consequence);
        self
    }
}

/// 對話系統狀態資源
#[derive(Resource, Default)]
pub struct DialogueState {
    /// 當前進行中的對話樹
    pub active_dialogue: Option<ActiveDialogue>,
    /// 對話歷史記錄（用於回顧）
    pub history: Vec<DialogueHistoryEntry>,
    /// 最大歷史記錄數
    pub max_history: usize,
}

/// 進行中的對話
#[derive(Debug, Clone)]
pub struct ActiveDialogue {
    /// 對話樹 ID
    pub dialogue_id: DialogueId,
    /// 當前節點 ID
    pub current_node: u32,
    /// 打字機效果進度（0.0 - 1.0）
    pub typing_progress: f32,
    /// 是否已完成打字
    pub typing_complete: bool,
    /// 是否可跳過
    pub can_skip: bool,
    /// 已選擇的選項索引（若已選擇）
    pub selected_choice: Option<usize>,
    /// 對話開始時間
    pub start_time: f32,
    /// 對話參與者（用於替換變數）
    pub participants: HashMap<String, String>,
}

impl ActiveDialogue {
    /// 創建新的進行中對話
    pub fn new(dialogue_id: DialogueId, start_node: u32) -> Self {
        Self {
            dialogue_id,
            current_node: start_node,
            typing_progress: 0.0,
            typing_complete: false,
            can_skip: true,
            selected_choice: None,
            start_time: 0.0,
            participants: HashMap::new(),
        }
    }

    /// 添加參與者（用於變數替換）
    pub fn with_participant(mut self, key: impl Into<String>, name: impl Into<String>) -> Self {
        self.participants.insert(key.into(), name.into());
        self
    }
}

/// 對話歷史記錄條目
#[derive(Debug, Clone)]
pub struct DialogueHistoryEntry {
    /// 說話者名稱
    pub speaker_name: String,
    /// 對話文字
    pub text: String,
    /// 時間戳
    pub timestamp: f32,
}

/// 對話事件
#[derive(Message, Debug, Clone)]
pub enum DialogueEvent {
    /// 開始對話
    Start {
        dialogue_id: DialogueId,
        participants: HashMap<String, String>,
    },
    /// 跳轉到節點
    GoToNode(u32),
    /// 選擇選項
    SelectChoice(usize),
    /// 跳過打字效果
    SkipTyping,
    /// 結束對話
    End,
    /// 對話完成（正常結束）
    Completed(DialogueId),
}

/// NPC 資料（用於對話系統）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDialogueData {
    /// NPC ID
    pub id: NpcId,
    /// 顯示名稱
    pub name: String,
    /// 預設頭像路徑
    pub portrait: String,
    /// 預設語音風格
    pub voice_style: Option<String>,
}

/// 對話資料庫資源（儲存所有對話樹）
#[derive(Resource, Default)]
pub struct DialogueDatabase {
    /// 所有對話樹
    pub dialogues: HashMap<DialogueId, DialogueTree>,
    /// NPC 資料
    pub npcs: HashMap<NpcId, NpcDialogueData>,
}

impl DialogueDatabase {
    /// 註冊對話樹
    pub fn register_dialogue(&mut self, dialogue: DialogueTree) {
        self.dialogues.insert(dialogue.id, dialogue);
    }

    /// 取得對話樹
    pub fn get_dialogue(&self, id: DialogueId) -> Option<&DialogueTree> {
        self.dialogues.get(&id)
    }

    /// 註冊 NPC 資料
    pub fn register_npc(&mut self, npc: NpcDialogueData) {
        self.npcs.insert(npc.id, npc);
    }

    /// 取得 NPC 資料
    pub fn get_npc(&self, id: NpcId) -> Option<&NpcDialogueData> {
        self.npcs.get(&id)
    }

    /// 取得對話數量
    pub fn dialogue_count(&self) -> usize {
        self.dialogues.len()
    }
}

/// 對話系統設定
#[derive(Resource)]
pub struct DialogueSettings {
    /// 預設打字速度（每秒字數）
    pub default_typing_speed: f32,
    /// 是否啟用語音
    pub voice_enabled: bool,
    /// 字幕字體大小
    pub font_size: f32,
    /// 對話框背景透明度
    pub box_opacity: f32,
    /// 是否顯示說話者名稱
    pub show_speaker_name: bool,
    /// 是否顯示頭像
    pub show_portrait: bool,
    /// 自動前進模式
    pub auto_advance: bool,
    /// 自動前進等待時間
    pub auto_advance_delay: f32,
}

impl Default for DialogueSettings {
    fn default() -> Self {
        Self {
            default_typing_speed: 30.0,
            voice_enabled: true,
            font_size: 24.0,
            box_opacity: 0.85,
            show_speaker_name: true,
            show_portrait: true,
            auto_advance: false,
            auto_advance_delay: 2.0,
        }
    }
}

// ============================================================================
// 範例對話樹建構
// ============================================================================

/// 創建範例對話樹（用於測試）
pub fn create_sample_dialogue() -> DialogueTree {
    let mut tree = DialogueTree::new(1, "第一章：初遇");

    // 節點 0：旁白開場
    tree.add_node(
        DialogueNode::new(0, DialogueSpeaker::Narrator, "你在酒吧裡注意到一個神秘的男人正看著你...")
            .with_auto_advance(2.0)
            .then(1),
    );

    // 節點 1：NPC 開口
    tree.add_node(
        DialogueNode::new(1, DialogueSpeaker::Npc(100), "嘿，你看起來像是在找工作的人。")
            .with_emotion(SpeakerEmotion::Smirk)
            .then(2),
    );

    // 節點 2：玩家選擇
    tree.add_node(
        DialogueNode::new(2, DialogueSpeaker::Npc(100), "我這裡有個...機會。你有興趣嗎？")
            .with_emotion(SpeakerEmotion::Serious)
            .with_choice(DialogueChoice::simple("告訴我更多", 3))
            .with_choice(DialogueChoice::simple("我不感興趣", 4))
            .with_choice(
                DialogueChoice::simple("你是誰？", 5)
                    .with_condition(DialogueCondition::NotHasFlag("know_mysterious_man".to_string())),
            ),
    );

    // 節點 3：接受任務
    tree.add_node(
        DialogueNode::new(3, DialogueSpeaker::Npc(100), "很好。有個人欠了我錢，我需要有人去...提醒他一下。")
            .with_emotion(SpeakerEmotion::Serious)
            .with_choice(
                DialogueChoice::simple("我接了", 6)
                    .with_consequence(DialogueConsequence::UnlockMission(1)),
            )
            .with_choice(DialogueChoice::simple("讓我考慮一下", 7)),
    );

    // 節點 4：拒絕
    tree.add_node(
        DialogueNode::new(4, DialogueSpeaker::Npc(100), "可惜了。如果你改變主意，你知道在哪找我。")
            .with_emotion(SpeakerEmotion::Neutral)
            .with_choice(DialogueChoice::end("離開")),
    );

    // 節點 5：詢問身份
    tree.add_node(
        DialogueNode::new(5, DialogueSpeaker::Npc(100), "叫我老王就行。這座島上，有些事情需要...特殊人才來處理。")
            .with_emotion(SpeakerEmotion::Smirk)
            .with_action(DialogueAction::PlaySound("reveal.ogg".to_string()))
            .with_choice(
                DialogueChoice::simple("繼續", 2)
                    .with_consequence(DialogueConsequence::SetStoryFlag {
                        flag: "know_mysterious_man".to_string(),
                        value: true,
                    }),
            ),
    );

    // 節點 6：任務接受
    tree.add_node(
        DialogueNode::new(6, DialogueSpeaker::Npc(100), "很好。地址我傳到你手機了。記住，要活的。")
            .with_emotion(SpeakerEmotion::Serious)
            .with_choice(DialogueChoice::end("離開")),
    );

    // 節點 7：考慮
    tree.add_node(
        DialogueNode::new(7, DialogueSpeaker::Npc(100), "別太久。機會不等人。")
            .with_emotion(SpeakerEmotion::Neutral)
            .with_choice(DialogueChoice::end("離開")),
    );

    tree.with_start_node(0)
}
