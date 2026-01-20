//! 任務觸發點組件
//!
//! 定義任務觸發區域、NPC 互動點等組件

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::story_data::{DialogueId, NpcId, StoryMissionId};

/// 觸發器類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerType {
    /// 進入區域自動觸發
    OnEnter,
    /// 需要按鍵互動
    OnInteract,
    /// 進入區域後延遲觸發
    OnEnterDelayed { delay: u32 }, // 毫秒
    /// 離開區域時觸發
    OnExit,
    /// 停留一段時間後觸發
    OnStay { duration: u32 }, // 毫秒
}

impl Default for TriggerType {
    fn default() -> Self {
        Self::OnInteract
    }
}

/// 觸發器形狀
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TriggerShape {
    /// 圓形（半徑）
    Circle(f32),
    /// 矩形（寬、高）
    Rectangle { width: f32, height: f32 },
    /// 膠囊形（半徑、高度）
    Capsule { radius: f32, height: f32 },
}

impl Default for TriggerShape {
    fn default() -> Self {
        Self::Circle(3.0)
    }
}

impl TriggerShape {
    /// 檢查點是否在形狀內（2D 平面檢查）
    pub fn contains(&self, center: Vec3, point: Vec3) -> bool {
        let dx = point.x - center.x;
        let dz = point.z - center.z;

        match self {
            Self::Circle(radius) => dx * dx + dz * dz <= radius * radius,
            Self::Rectangle { width, height } => {
                dx.abs() <= width / 2.0 && dz.abs() <= height / 2.0
            }
            Self::Capsule { radius, height: _ } => {
                // 簡化為圓柱體檢查
                dx * dx + dz * dz <= radius * radius
            }
        }
    }
}

/// 任務觸發點組件
#[derive(Component, Debug, Clone)]
pub struct MissionTrigger {
    /// 關聯的任務 ID
    pub mission_id: StoryMissionId,
    /// 觸發器類型
    pub trigger_type: TriggerType,
    /// 觸發器形狀
    pub shape: TriggerShape,
    /// 是否只觸發一次
    pub one_shot: bool,
    /// 是否已觸發
    pub triggered: bool,
    /// 是否啟用
    pub enabled: bool,
    /// 顯示的提示文字
    pub prompt_text: Option<String>,
    /// 顯示的任務名稱
    pub mission_name: Option<String>,
    /// 需要的劇情旗標
    pub required_flag: Option<String>,
}

impl Default for MissionTrigger {
    fn default() -> Self {
        Self {
            mission_id: 0,
            trigger_type: TriggerType::OnInteract,
            shape: TriggerShape::Circle(3.0),
            one_shot: false,
            triggered: false,
            enabled: true,
            prompt_text: Some("按 F 開始任務".to_string()),
            mission_name: None,
            required_flag: None,
        }
    }
}

impl MissionTrigger {
    /// 創建新的任務觸發點
    pub fn new(mission_id: StoryMissionId) -> Self {
        Self {
            mission_id,
            ..Default::default()
        }
    }

    /// 設置觸發類型
    pub fn with_trigger_type(mut self, trigger_type: TriggerType) -> Self {
        self.trigger_type = trigger_type;
        self
    }

    /// 設置形狀
    pub fn with_shape(mut self, shape: TriggerShape) -> Self {
        self.shape = shape;
        self
    }

    /// 設置為一次性觸發
    pub fn one_shot(mut self) -> Self {
        self.one_shot = true;
        self
    }

    /// 設置提示文字
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt_text = Some(prompt.into());
        self
    }

    /// 設置任務名稱
    pub fn with_mission_name(mut self, name: impl Into<String>) -> Self {
        self.mission_name = Some(name.into());
        self
    }

    /// 設置需要的旗標
    pub fn requires_flag(mut self, flag: impl Into<String>) -> Self {
        self.required_flag = Some(flag.into());
        self
    }
}

/// 對話觸發點組件
#[derive(Component, Debug, Clone)]
pub struct DialogueTrigger {
    /// 對話樹 ID
    pub dialogue_id: DialogueId,
    /// 觸發器類型
    pub trigger_type: TriggerType,
    /// 觸發器形狀
    pub shape: TriggerShape,
    /// 是否只觸發一次
    pub one_shot: bool,
    /// 是否已觸發
    pub triggered: bool,
    /// 是否啟用
    pub enabled: bool,
    /// 顯示的提示文字
    pub prompt_text: Option<String>,
}

impl Default for DialogueTrigger {
    fn default() -> Self {
        Self {
            dialogue_id: 0,
            trigger_type: TriggerType::OnInteract,
            shape: TriggerShape::Circle(2.0),
            one_shot: false,
            triggered: false,
            enabled: true,
            prompt_text: Some("按 F 交談".to_string()),
        }
    }
}

impl DialogueTrigger {
    /// 創建新的對話觸發點
    pub fn new(dialogue_id: DialogueId) -> Self {
        Self {
            dialogue_id,
            ..Default::default()
        }
    }

    /// 設置為一次性觸發
    pub fn one_shot(mut self) -> Self {
        self.one_shot = true;
        self
    }

    /// 設置提示文字
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt_text = Some(prompt.into());
        self
    }
}

/// 任務 NPC 組件（可與之互動開始任務的 NPC）
#[derive(Component, Debug, Clone)]
pub struct MissionNpc {
    /// NPC ID
    pub npc_id: NpcId,
    /// 顯示名稱
    pub name: String,
    /// 提供的任務 ID（若有）
    pub offers_mission: Option<StoryMissionId>,
    /// 對話樹 ID（非任務對話）
    pub idle_dialogue: Option<DialogueId>,
    /// 互動半徑
    pub interaction_radius: f32,
    /// 是否顯示頭頂標記
    pub show_marker: bool,
    /// 標記類型
    pub marker_type: NpcMarkerType,
    /// 是否可互動
    pub can_interact: bool,
}

impl Default for MissionNpc {
    fn default() -> Self {
        Self {
            npc_id: 0,
            name: "NPC".to_string(),
            offers_mission: None,
            idle_dialogue: None,
            interaction_radius: 2.5,
            show_marker: true,
            marker_type: NpcMarkerType::None,
            can_interact: true,
        }
    }
}

impl MissionNpc {
    /// 創建新的任務 NPC
    pub fn new(npc_id: NpcId, name: impl Into<String>) -> Self {
        Self {
            npc_id,
            name: name.into(),
            ..Default::default()
        }
    }

    /// 設置提供的任務
    pub fn offers_mission(mut self, mission_id: StoryMissionId) -> Self {
        self.offers_mission = Some(mission_id);
        self.marker_type = NpcMarkerType::Mission;
        self
    }

    /// 設置閒聊對話
    pub fn with_idle_dialogue(mut self, dialogue_id: DialogueId) -> Self {
        self.idle_dialogue = Some(dialogue_id);
        self
    }

    /// 設置標記類型
    pub fn with_marker(mut self, marker_type: NpcMarkerType) -> Self {
        self.marker_type = marker_type;
        self
    }
}

/// NPC 頭頂標記類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NpcMarkerType {
    #[default]
    None,
    /// 黃色驚嘆號（有新任務）
    Mission,
    /// 黃色問號（任務進行中）
    MissionInProgress,
    /// 灰色問號（任務完成回報）
    MissionComplete,
    /// 藍色氣泡（有對話）
    Dialogue,
    /// 綠色美元符號（商店）
    Shop,
    /// 紅色拳頭（敵對）
    Hostile,
}

impl NpcMarkerType {
    /// 取得標記顏色
    pub fn color(&self) -> Color {
        match self {
            Self::None => Color::NONE,
            Self::Mission => Color::srgb(1.0, 0.9, 0.0),        // 黃色
            Self::MissionInProgress => Color::srgb(1.0, 0.8, 0.0),
            Self::MissionComplete => Color::srgb(0.6, 0.6, 0.6), // 灰色
            Self::Dialogue => Color::srgb(0.3, 0.7, 1.0),       // 藍色
            Self::Shop => Color::srgb(0.2, 0.8, 0.2),           // 綠色
            Self::Hostile => Color::srgb(1.0, 0.2, 0.2),        // 紅色
        }
    }

    /// 取得標記圖示
    pub fn icon(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Mission => "!",
            Self::MissionInProgress => "?",
            Self::MissionComplete => "?",
            Self::Dialogue => "...",
            Self::Shop => "$",
            Self::Hostile => "!",
        }
    }
}

/// 區域觸發點組件（用於目標區域）
#[derive(Component, Debug, Clone)]
pub struct AreaTrigger {
    /// 區域名稱
    pub name: String,
    /// 觸發器形狀
    pub shape: TriggerShape,
    /// 是否是任務目標區域
    pub is_objective: bool,
    /// 關聯的任務 ID
    pub mission_id: Option<StoryMissionId>,
    /// 進入時顯示的文字
    pub enter_text: Option<String>,
    /// 離開時顯示的文字
    pub exit_text: Option<String>,
    /// 玩家是否在區域內
    pub player_inside: bool,
    /// 停留時間（秒）
    pub stay_duration: f32,
}

impl Default for AreaTrigger {
    fn default() -> Self {
        Self {
            name: "區域".to_string(),
            shape: TriggerShape::Circle(10.0),
            is_objective: false,
            mission_id: None,
            enter_text: None,
            exit_text: None,
            player_inside: false,
            stay_duration: 0.0,
        }
    }
}

impl AreaTrigger {
    /// 創建新的區域觸發點
    pub fn new(name: impl Into<String>, radius: f32) -> Self {
        Self {
            name: name.into(),
            shape: TriggerShape::Circle(radius),
            ..Default::default()
        }
    }

    /// 設置為任務目標區域
    pub fn with_objective(mut self, mission_id: StoryMissionId) -> Self {
        self.is_objective = true;
        self.mission_id = Some(mission_id);
        self
    }

    /// 設置進入提示
    pub fn with_enter_text(mut self, text: impl Into<String>) -> Self {
        self.enter_text = Some(text.into());
        self
    }
}

/// 任務目標標記組件
#[derive(Component, Debug, Clone)]
pub struct ObjectiveMarker {
    /// 標記類型
    pub marker_type: ObjectiveMarkerType,
    /// 是否顯示在小地圖
    pub show_on_minimap: bool,
    /// 是否顯示距離
    pub show_distance: bool,
    /// 脈衝動畫相位
    pub pulse_phase: f32,
    /// 高度偏移（顯示在目標上方）
    pub height_offset: f32,
}

impl Default for ObjectiveMarker {
    fn default() -> Self {
        Self {
            marker_type: ObjectiveMarkerType::Location,
            show_on_minimap: true,
            show_distance: true,
            pulse_phase: 0.0,
            height_offset: 2.0,
        }
    }
}

/// 目標標記類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ObjectiveMarkerType {
    #[default]
    Location,
    /// 目標 NPC
    TargetNpc,
    /// 敵人目標
    TargetEnemy,
    /// 物品
    Item,
    /// 載具
    Vehicle,
    /// 護送目標
    Escort,
    /// 區域邊界
    AreaBoundary,
}

impl ObjectiveMarkerType {
    /// 取得標記顏色
    pub fn color(&self) -> Color {
        match self {
            Self::Location => Color::srgb(1.0, 0.9, 0.0),    // 黃色
            Self::TargetNpc => Color::srgb(0.3, 0.7, 1.0),   // 藍色
            Self::TargetEnemy => Color::srgb(1.0, 0.2, 0.2), // 紅色
            Self::Item => Color::srgb(0.2, 1.0, 0.5),        // 青綠色
            Self::Vehicle => Color::srgb(0.8, 0.5, 1.0),     // 紫色
            Self::Escort => Color::srgb(0.2, 0.8, 0.2),      // 綠色
            Self::AreaBoundary => Color::srgba(1.0, 1.0, 0.0, 0.5), // 半透明黃
        }
    }
}

/// 觸發器事件
#[derive(Message, Debug, Clone)]
pub enum TriggerEvent {
    /// 玩家進入觸發區域
    PlayerEntered {
        entity: Entity,
        trigger_type: TriggerEventType,
    },
    /// 玩家離開觸發區域
    PlayerExited {
        entity: Entity,
        trigger_type: TriggerEventType,
    },
    /// 玩家與觸發器互動
    PlayerInteracted {
        entity: Entity,
        trigger_type: TriggerEventType,
    },
    /// 玩家在觸發區域停留足夠時間
    PlayerStayed {
        entity: Entity,
        trigger_type: TriggerEventType,
        duration: f32,
    },
}

/// 觸發事件類型
#[derive(Debug, Clone, Copy)]
pub enum TriggerEventType {
    Mission(StoryMissionId),
    Dialogue(DialogueId),
    Area,
}

/// 觸發器視覺效果組件（用於渲染觸發區域指示）
#[derive(Component)]
pub struct TriggerVisual {
    /// 基礎顏色
    pub color: Color,
    /// 是否顯示
    pub visible: bool,
    /// 動畫相位
    pub animation_phase: f32,
    /// 旋轉速度
    pub rotation_speed: f32,
}

impl Default for TriggerVisual {
    fn default() -> Self {
        Self {
            color: Color::srgba(1.0, 0.9, 0.0, 0.3),
            visible: true,
            animation_phase: 0.0,
            rotation_speed: 0.5,
        }
    }
}

/// 互動提示 UI 組件
#[derive(Component)]
pub struct InteractionPrompt {
    /// 提示文字
    pub text: String,
    /// 按鍵提示
    pub key_hint: String,
    /// 顯示進度（0.0-1.0，用於淡入淡出）
    pub visibility: f32,
    /// 目標可見度
    pub target_visibility: f32,
}

impl Default for InteractionPrompt {
    fn default() -> Self {
        Self {
            text: "互動".to_string(),
            key_hint: "F".to_string(),
            visibility: 0.0,
            target_visibility: 0.0,
        }
    }
}

impl InteractionPrompt {
    /// 創建新的互動提示
    pub fn new(text: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            key_hint: key.into(),
            ..Default::default()
        }
    }

    /// 顯示提示
    pub fn show(&mut self) {
        self.target_visibility = 1.0;
    }

    /// 隱藏提示
    pub fn hide(&mut self) {
        self.target_visibility = 0.0;
    }

    /// 更新可見度（用於淡入淡出）
    pub fn update(&mut self, delta: f32) {
        let speed = 5.0;
        if self.visibility < self.target_visibility {
            self.visibility = (self.visibility + speed * delta).min(self.target_visibility);
        } else if self.visibility > self.target_visibility {
            self.visibility = (self.visibility - speed * delta).max(self.target_visibility);
        }
    }
}

/// 玩家在觸發區域內的追蹤組件
#[derive(Component, Default)]
pub struct PlayerInTrigger {
    /// 進入時間
    pub enter_time: f32,
    /// 停留時間
    pub stay_duration: f32,
}

// ============================================================================
// 任務目標實體標記
// ============================================================================

/// 任務目標實體標記
///
/// 用於標記需要追蹤的實體（如追蹤任務中的車輛、護送任務中的 NPC）
#[derive(Component, Debug, Clone)]
pub struct MissionTargetEntity {
    /// 目標 ID（與 ObjectiveType 中的 ID 對應）
    pub target_id: String,
    /// 關聯的任務 ID
    pub mission_id: StoryMissionId,
    /// 目標類型
    pub target_type: MissionTargetType,
    /// 是否已被追蹤（玩家已經開始追蹤）
    pub is_tracked: bool,
    /// 最後與玩家的距離
    pub last_distance_to_player: f32,
    /// 目標路徑（追蹤任務用）
    pub waypoints: Vec<Vec3>,
    /// 當前路徑索引
    pub current_waypoint: usize,
    /// 移動速度
    pub speed: f32,
}

impl Default for MissionTargetEntity {
    fn default() -> Self {
        Self {
            target_id: String::new(),
            mission_id: 0,
            target_type: MissionTargetType::Follow,
            is_tracked: false,
            last_distance_to_player: 0.0,
            waypoints: Vec::new(),
            current_waypoint: 0,
            speed: 15.0,
        }
    }
}

impl MissionTargetEntity {
    /// 創建追蹤目標
    pub fn follow_target(target_id: impl Into<String>, mission_id: StoryMissionId) -> Self {
        Self {
            target_id: target_id.into(),
            mission_id,
            target_type: MissionTargetType::Follow,
            ..Default::default()
        }
    }

    /// 創建護送目標
    pub fn escort_target(target_id: impl Into<String>, mission_id: StoryMissionId) -> Self {
        Self {
            target_id: target_id.into(),
            mission_id,
            target_type: MissionTargetType::Escort,
            ..Default::default()
        }
    }

    /// 設置路徑
    pub fn with_waypoints(mut self, waypoints: Vec<Vec3>) -> Self {
        self.waypoints = waypoints;
        self
    }

    /// 設置移動速度
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}

/// 任務目標類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissionTargetType {
    /// 追蹤（保持距離）
    #[default]
    Follow,
    /// 護送（保護）
    Escort,
    /// 刺殺（消滅）
    Kill,
    /// 收集（拾取）
    Collect,
}
