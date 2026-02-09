//! 劇情任務資料結構
//!
//! 定義主線劇情任務的所有資料類型，支援多階段任務、解鎖條件和獎勵系統。
#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::combat::{EnemyType, WeaponType};
use crate::vehicle::VehicleType;

/// 劇情任務 ID
pub type StoryMissionId = u32;

/// 對話 ID
pub type DialogueId = u32;

/// 過場動畫 ID
pub type CutsceneId = u32;

/// NPC ID
pub type NpcId = u32;

/// 區域 ID
pub type AreaId = u32;

// ============================================================================
// 任務狀態
// ============================================================================

/// 劇情任務狀態
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum StoryMissionStatus {
    /// 尚未解鎖
    #[default]
    Locked,
    /// 可接取
    Available,
    /// 進行中
    InProgress,
    /// 已完成
    Completed,
    /// 失敗（可重試）
    Failed,
}

// ============================================================================
// 任務類型
// ============================================================================

/// 劇情任務類型
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[derive(Default)]
pub enum StoryMissionType {
    // === 戰鬥類 ===
    /// 刺殺：消滅特定目標
    Assassination,
    /// 清除：消滅所有敵人
    Elimination,
    /// 防禦：保護位置/人物
    Defend,

    // === 追逐類 ===
    /// 追車：追上並攔截目標
    Chase,
    /// 逃脫：甩開追兵
    Escape,

    // === 護送類 ===
    /// 護送：保護 NPC 到達目的地
    Escort,
    /// 車隊護送：保護車輛
    Convoy,

    // === 潛入類 ===
    /// 潛入：不被發現完成任務
    Stealth,
    /// 滲透：進入敵方據點
    Infiltrate,

    // === 收集/互動類 ===
    /// 取回：取得特定物品
    Retrieve,
    /// 破壞：破壞特定目標
    Sabotage,

    // === 劇情類 ===
    /// 純過場
    Cutscene,
    /// 對話任務
    #[default]
    Dialogue,
}


// ============================================================================
// 任務目標
// ============================================================================

/// 目標類型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ObjectiveType {
    /// 消滅特定目標（目標標記 ID）
    KillTarget(String),
    /// 消滅指定數量敵人
    KillCount(u32),
    /// 到達位置（座標, 半徑）
    ReachLocation(Vec3, f32),
    /// 收集物品（物品 ID）
    CollectItem(String),
    /// 護送 NPC（NPC 標記 ID）
    EscortNpc(String),
    /// 追蹤目標（目標 ID, 最大距離）
    FollowTarget(String, f32),
    /// 生存指定時間（秒）
    SurviveTime(f32),
    /// 破壞物件（物件標記 ID）
    DestroyObject(String),
    /// 保持潛行（未被發現）
    StayUndetected,
    /// 與 NPC 對話
    TalkToNpc(String),
    /// 進入車輛
    EnterVehicle(String),
    /// 自定義目標（由程式碼處理）
    Custom(String),
}

/// 任務目標
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionObjective {
    /// 目標 ID
    pub id: u32,
    /// 目標類型
    pub objective_type: ObjectiveType,
    /// 描述文字
    pub description: String,
    /// 目標數量
    pub target_count: u32,
    /// 當前完成數量
    #[serde(default)]
    pub current_count: u32,
    /// 是否為可選目標
    #[serde(default)]
    pub is_optional: bool,
    /// 是否已完成
    #[serde(default)]
    pub is_completed: bool,
}

impl MissionObjective {
    /// 創建新目標
    pub fn new(id: u32, objective_type: ObjectiveType, description: impl Into<String>) -> Self {
        Self {
            id,
            objective_type,
            description: description.into(),
            target_count: 1,
            current_count: 0,
            is_optional: false,
            is_completed: false,
        }
    }

    /// 設置目標數量
    pub fn with_count(mut self, count: u32) -> Self {
        self.target_count = count;
        self
    }

    /// 設為可選目標
    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    /// 增加完成計數
    pub fn increment(&mut self) {
        self.current_count = (self.current_count + 1).min(self.target_count);
        if self.current_count >= self.target_count {
            self.is_completed = true;
        }
    }

    /// 檢查是否完成
    pub fn check_completion(&self) -> bool {
        self.is_completed || self.current_count >= self.target_count
    }

    /// 取得進度百分比
    pub fn progress(&self) -> f32 {
        if self.target_count == 0 {
            return 1.0;
        }
        self.current_count as f32 / self.target_count as f32
    }
}

// ============================================================================
// 失敗條件
// ============================================================================

/// 失敗條件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FailCondition {
    /// 玩家死亡
    PlayerDeath,
    /// 特定 NPC 死亡
    NpcDeath(String),
    /// 時間耗盡
    TimeExpired,
    /// 車輛被摧毀
    VehicleDestroyed(String),
    /// 被發現（潛入任務）
    Detected,
    /// 目標逃跑
    TargetEscaped,
    /// 離開指定區域（中心點, 半徑）
    ZoneExit(Vec3, f32),
    /// 護送目標血量過低
    EscortHealthLow(f32),
}

// ============================================================================
// 任務階段
// ============================================================================

/// 敵人生成資料
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemySpawnData {
    /// 生成位置
    pub position: Vec3,
    /// 敵人類型
    pub enemy_type: EnemyType,
    /// 巡邏路徑
    pub patrol_path: Option<Vec<Vec3>>,
    /// 武器覆蓋
    pub weapon_override: Option<WeaponType>,
    /// 是否為 Boss
    #[serde(default)]
    pub is_boss: bool,
    /// 延遲生成時間（秒）
    #[serde(default)]
    pub spawn_delay: f32,
    /// 任務標記 ID（用於目標追蹤）
    pub marker_id: Option<String>,
}

/// NPC 生成資料
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NpcSpawnData {
    /// NPC ID
    pub npc_id: NpcId,
    /// 生成位置
    pub position: Vec3,
    /// 模型名稱
    pub model: String,
    /// 顯示名稱
    pub name: String,
    /// 是否無敵
    #[serde(default)]
    pub is_invulnerable: bool,
    /// 是否跟隨玩家
    #[serde(default)]
    pub follow_player: bool,
    /// 對話樹 ID
    pub dialogue_tree: Option<DialogueId>,
}

/// 任務階段
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionPhase {
    /// 階段 ID
    pub id: u32,
    /// 階段類型
    pub phase_type: StoryMissionType,
    /// 階段描述
    pub description: String,
    /// 目標列表
    pub objectives: Vec<MissionObjective>,
    /// 開始時播放的對話
    pub start_dialogue: Option<DialogueId>,
    /// 結束時播放的對話
    pub end_dialogue: Option<DialogueId>,
    /// 開始時播放的過場
    pub cutscene: Option<CutsceneId>,
    /// 敵人生成列表
    #[serde(default)]
    pub spawn_enemies: Vec<EnemySpawnData>,
    /// NPC 生成列表
    #[serde(default)]
    pub spawn_npcs: Vec<NpcSpawnData>,
    /// 路徑點
    #[serde(default)]
    pub waypoints: Vec<Vec3>,
    /// 時間限制（秒）
    pub time_limit: Option<f32>,
    /// 失敗條件
    #[serde(default)]
    pub fail_conditions: Vec<FailCondition>,
}

impl MissionPhase {
    /// 創建新階段
    pub fn new(id: u32, phase_type: StoryMissionType, description: impl Into<String>) -> Self {
        Self {
            id,
            phase_type,
            description: description.into(),
            objectives: Vec::new(),
            start_dialogue: None,
            end_dialogue: None,
            cutscene: None,
            spawn_enemies: Vec::new(),
            spawn_npcs: Vec::new(),
            waypoints: Vec::new(),
            time_limit: None,
            fail_conditions: Vec::new(),
        }
    }

    /// 添加目標
    pub fn with_objective(mut self, objective: MissionObjective) -> Self {
        self.objectives.push(objective);
        self
    }

    /// 設置時間限制
    pub fn with_time_limit(mut self, seconds: f32) -> Self {
        self.time_limit = Some(seconds);
        self.fail_conditions.push(FailCondition::TimeExpired);
        self
    }

    /// 設置開始對話
    pub fn with_start_dialogue(mut self, dialogue_id: DialogueId) -> Self {
        self.start_dialogue = Some(dialogue_id);
        self
    }

    /// 設置過場動畫
    pub fn with_cutscene(mut self, cutscene_id: CutsceneId) -> Self {
        self.cutscene = Some(cutscene_id);
        self
    }

    /// 添加失敗條件
    pub fn with_fail_condition(mut self, condition: FailCondition) -> Self {
        self.fail_conditions.push(condition);
        self
    }

    /// 設置結束對話
    pub fn with_end_dialogue(mut self, dialogue_id: DialogueId) -> Self {
        self.end_dialogue = Some(dialogue_id);
        self
    }

    /// 檢查所有必要目標是否完成
    pub fn is_complete(&self) -> bool {
        self.objectives
            .iter()
            .filter(|obj| !obj.is_optional)
            .all(|obj| obj.check_completion())
    }
}

// ============================================================================
// 解鎖條件
// ============================================================================

/// 解鎖條件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnlockCondition {
    /// 完成指定任務
    CompleteMission(StoryMissionId),
    /// 達到指定章節
    ChapterReached(u32),
    /// 金錢門檻
    MoneyAmount(u32),
    /// 時間範圍（開始小時, 結束小時）
    TimeOfDay(f32, f32),
    /// 劇情標記
    HasFlag(String),
    /// 無條件（總是可接）
    None,
}

// ============================================================================
// 任務獎勵
// ============================================================================

/// 難度等級
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Difficulty {
    /// 簡單
    Easy,
    /// 普通
    #[default]
    Normal,
    /// 困難
    Hard,
    /// 極難
    Extreme,
}

/// 任務獎勵
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MissionRewards {
    /// 金錢獎勵
    pub money: u32,
    /// 聲望點數
    #[serde(default)]
    pub respect: u32,
    /// 解鎖武器
    #[serde(default)]
    pub unlock_weapons: Vec<WeaponType>,
    /// 解鎖車輛
    #[serde(default)]
    pub unlock_vehicles: Vec<VehicleType>,
    /// 解鎖區域
    #[serde(default)]
    pub unlock_areas: Vec<AreaId>,
    /// 解鎖任務
    #[serde(default)]
    pub unlock_missions: Vec<StoryMissionId>,
    /// 設置劇情標記
    #[serde(default)]
    pub set_flags: Vec<String>,
}

impl MissionRewards {
    /// 創建金錢獎勵
    pub fn money(amount: u32) -> Self {
        Self {
            money: amount,
            ..default()
        }
    }

    /// 添加聲望
    pub fn with_respect(mut self, respect: u32) -> Self {
        self.respect = respect;
        self
    }

    /// 解鎖下一任務
    pub fn unlock_mission(mut self, mission_id: StoryMissionId) -> Self {
        self.unlock_missions.push(mission_id);
        self
    }

    /// 解鎖武器
    pub fn unlock_weapon(mut self, weapon: WeaponType) -> Self {
        self.unlock_weapons.push(weapon);
        self
    }

    /// 解鎖車輛
    pub fn unlock_vehicle(mut self, vehicle: VehicleType) -> Self {
        self.unlock_vehicles.push(vehicle);
        self
    }

    /// 設置劇情旗標
    pub fn set_flag(mut self, flag: String) -> Self {
        self.set_flags.push(flag);
        self
    }
}

// ============================================================================
// 劇情任務定義
// ============================================================================

/// 劇情任務定義（完整任務）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoryMission {
    /// 任務 ID
    pub id: StoryMissionId,
    /// 任務標題
    pub title: String,
    /// 任務描述
    pub description: String,
    /// 章節編號
    pub chapter: u32,
    /// 任務給予者 NPC
    pub quest_giver: Option<NpcId>,
    /// 觸發位置
    pub trigger_location: Option<Vec3>,
    /// 觸發半徑
    #[serde(default = "default_trigger_radius")]
    pub trigger_radius: f32,
    /// 解鎖條件
    #[serde(default)]
    pub unlock_conditions: Vec<UnlockCondition>,
    /// 任務階段
    pub phases: Vec<MissionPhase>,
    /// 任務獎勵
    #[serde(default)]
    pub rewards: MissionRewards,
    /// 難度
    #[serde(default)]
    pub difficulty: Difficulty,
    /// 預估時間（分鐘）
    #[serde(default = "default_estimated_time")]
    pub estimated_time: f32,
    /// 開場過場動畫
    pub intro_cutscene: Option<CutsceneId>,
    /// 結束過場動畫
    pub outro_cutscene: Option<CutsceneId>,
}

fn default_trigger_radius() -> f32 {
    5.0
}

fn default_estimated_time() -> f32 {
    10.0
}

impl StoryMission {
    /// 創建新任務
    pub fn new(id: StoryMissionId, title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            description: description.into(),
            chapter: 1,
            quest_giver: None,
            trigger_location: None,
            trigger_radius: 5.0,
            unlock_conditions: Vec::new(),
            phases: Vec::new(),
            rewards: MissionRewards::default(),
            difficulty: Difficulty::Normal,
            estimated_time: 10.0,
            intro_cutscene: None,
            outro_cutscene: None,
        }
    }

    /// 設置章節
    pub fn chapter(mut self, chapter: u32) -> Self {
        self.chapter = chapter;
        self
    }

    /// 設置觸發位置
    pub fn at_location(mut self, position: Vec3) -> Self {
        self.trigger_location = Some(position);
        self
    }

    /// 設置任務給予者
    pub fn with_quest_giver(mut self, npc_id: NpcId) -> Self {
        self.quest_giver = Some(npc_id);
        self
    }

    /// 添加階段
    pub fn with_phase(mut self, phase: MissionPhase) -> Self {
        self.phases.push(phase);
        self
    }

    /// 設置獎勵
    pub fn with_rewards(mut self, rewards: MissionRewards) -> Self {
        self.rewards = rewards;
        self
    }

    /// 需要先完成其他任務
    pub fn requires_mission(mut self, mission_id: StoryMissionId) -> Self {
        self.unlock_conditions.push(UnlockCondition::CompleteMission(mission_id));
        self
    }

    /// 需要劇情旗標
    pub fn requires_flag(mut self, flag: impl Into<String>) -> Self {
        self.unlock_conditions.push(UnlockCondition::HasFlag(flag.into()));
        self
    }

    /// 設置難度
    pub fn difficulty(mut self, difficulty: Difficulty) -> Self {
        self.difficulty = difficulty;
        self
    }

    /// 取得總階段數
    pub fn phase_count(&self) -> usize {
        self.phases.len()
    }

    /// 取得指定階段
    pub fn get_phase(&self, index: usize) -> Option<&MissionPhase> {
        self.phases.get(index)
    }
}

// ============================================================================
// 進行中任務狀態
// ============================================================================

/// 進行中的劇情任務
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveStoryMission {
    /// 任務 ID
    pub mission_id: StoryMissionId,
    /// 當前階段索引
    pub current_phase: usize,
    /// 階段計時器
    pub phase_timer: f32,
    /// 目標狀態（複製自當前階段）
    pub objectives: Vec<MissionObjective>,
    /// 檢查點位置
    pub checkpoint_position: Option<Vec3>,
    /// 檢查點階段
    pub checkpoint_phase: usize,
    /// 已生成的實體 ID（用於清理）
    #[serde(skip)]
    pub spawned_entities: Vec<Entity>,
}

impl ActiveStoryMission {
    /// 創建新的進行中任務
    pub fn new(mission_id: StoryMissionId, first_phase: &MissionPhase) -> Self {
        Self {
            mission_id,
            current_phase: 0,
            phase_timer: 0.0,
            objectives: first_phase.objectives.clone(),
            checkpoint_position: None,
            checkpoint_phase: 0,
            spawned_entities: Vec::new(),
        }
    }

    /// 更新計時器
    pub fn tick(&mut self, delta: f32) {
        self.phase_timer += delta;
    }

    /// 設置檢查點
    pub fn set_checkpoint(&mut self, position: Vec3) {
        self.checkpoint_position = Some(position);
        self.checkpoint_phase = self.current_phase;
    }

    /// 前進到下一階段
    pub fn advance_phase(&mut self, next_phase: &MissionPhase) {
        self.current_phase += 1;
        self.phase_timer = 0.0;
        self.objectives = next_phase.objectives.clone();
    }

    /// 檢查當前階段是否完成
    pub fn is_phase_complete(&self) -> bool {
        self.objectives
            .iter()
            .filter(|obj| !obj.is_optional)
            .all(|obj| obj.check_completion())
    }

    /// 更新目標進度
    pub fn update_objective(&mut self, objective_id: u32, count: u32) {
        if let Some(obj) = self.objectives.iter_mut().find(|o| o.id == objective_id) {
            obj.current_count = count;
            if obj.current_count >= obj.target_count {
                obj.is_completed = true;
            }
        }
    }

    /// 標記目標完成
    pub fn complete_objective(&mut self, objective_id: u32) {
        if let Some(obj) = self.objectives.iter_mut().find(|o| o.id == objective_id) {
            obj.is_completed = true;
            obj.current_count = obj.target_count;
        }
    }
}

// ============================================================================
// 任務評分系統
// ============================================================================

/// 劇情任務評分 (1-5 星)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryMissionRating {
    #[default]
    None,
    OneStar,    // 勉強完成
    TwoStars,   // 普通
    ThreeStars, // 良好
    FourStars,  // 優秀
    FiveStars,  // 完美
}

impl StoryMissionRating {
    /// 從星數建立評分（用於 v1 存檔 fallback 轉換）
    pub fn from_stars(stars: u8) -> Self {
        match stars {
            0 => Self::None,
            1 => Self::OneStar,
            2 => Self::TwoStars,
            3 => Self::ThreeStars,
            4 => Self::FourStars,
            5 => Self::FiveStars,
            _ => {
                warn!("無效的任務評分星數: {}，預設為 None", stars);
                Self::None
            }
        }
    }

    /// 取得星星數
    pub fn stars(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::OneStar => 1,
            Self::TwoStars => 2,
            Self::ThreeStars => 3,
            Self::FourStars => 4,
            Self::FiveStars => 5,
        }
    }

    /// 取得表情符號
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::OneStar => "⭐",
            Self::TwoStars => "⭐⭐",
            Self::ThreeStars => "⭐⭐⭐",
            Self::FourStars => "⭐⭐⭐⭐",
            Self::FiveStars => "⭐⭐⭐⭐⭐",
        }
    }

    /// 取得獎金倍率
    pub fn bonus_multiplier(&self) -> f32 {
        match self {
            Self::None => 0.5,
            Self::OneStar => 0.8,
            Self::TwoStars => 1.0,
            Self::ThreeStars => 1.25,
            Self::FourStars => 1.5,
            Self::FiveStars => 2.0,
        }
    }

    /// 取得評價文字
    pub fn description(&self) -> &'static str {
        match self {
            Self::None => "任務失敗",
            Self::OneStar => "勉強過關",
            Self::TwoStars => "普通表現",
            Self::ThreeStars => "良好表現",
            Self::FourStars => "優秀表現",
            Self::FiveStars => "完美達成！",
        }
    }
}

/// 任務表現追蹤
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MissionPerformance {
    /// 任務開始時間
    pub start_time: f32,
    /// 任務完成時間
    pub completion_time: f32,
    /// 玩家死亡次數
    pub player_deaths: u32,
    /// 檢查點重試次數
    pub checkpoint_retries: u32,
    /// 命中數
    pub shots_hit: u32,
    /// 射擊數
    pub shots_fired: u32,
    /// 擊殺數
    pub enemies_killed: u32,
    /// 爆頭數
    pub headshots: u32,
    /// 傷害承受量
    pub damage_taken: f32,
    /// 隱匿任務：被發現次數
    pub times_detected: u32,
    /// 護送任務：護送目標血量百分比
    pub escort_health_percent: f32,
    /// 駕駛任務：車輛損壞百分比
    pub vehicle_damage_percent: f32,
    /// 收集物找到數
    pub collectibles_found: u32,
    /// 收集物總數
    pub collectibles_total: u32,
    /// 可選目標完成數
    pub optional_objectives_completed: u32,
    /// 可選目標總數
    pub optional_objectives_total: u32,
}

impl MissionPerformance {
    /// 計算射擊精準度 (0.0 - 1.0)
    pub fn accuracy(&self) -> f32 {
        if self.shots_fired == 0 {
            1.0 // 沒射擊視為滿分
        } else {
            (self.shots_hit as f32 / self.shots_fired as f32).min(1.0)
        }
    }

    /// 計算爆頭率 (0.0 - 1.0)
    pub fn headshot_ratio(&self) -> f32 {
        if self.enemies_killed == 0 {
            0.0
        } else {
            (self.headshots as f32 / self.enemies_killed as f32).min(1.0)
        }
    }

    /// 計算最終評分
    pub fn calculate_rating(&self, target_time: f32) -> StoryMissionRating {
        let mut score: f32 = 100.0;

        // 時間懲罰 (超時每 30 秒 -10 分)
        let overtime = (self.completion_time - target_time).max(0.0);
        score -= (overtime / 30.0) * 10.0;

        // 死亡懲罰 (每次 -15 分)
        score -= self.player_deaths as f32 * 15.0;

        // 重試懲罰 (每次 -10 分)
        score -= self.checkpoint_retries as f32 * 10.0;

        // 精準度加分 (最多 +10 分)
        score += self.accuracy() * 10.0;

        // 爆頭加分 (最多 +10 分)
        score += self.headshot_ratio() * 10.0;

        // 可選目標加分 (每完成一個 +5 分)
        score += self.optional_objectives_completed as f32 * 5.0;

        // 收集物加分
        if self.collectibles_total > 0 {
            score += (self.collectibles_found as f32 / self.collectibles_total as f32) * 10.0;
        }

        // 隱匿任務懲罰 (每次被發現 -10 分)
        score -= self.times_detected as f32 * 10.0;

        // 轉換為評分
        if score >= 95.0 {
            StoryMissionRating::FiveStars
        } else if score >= 80.0 {
            StoryMissionRating::FourStars
        } else if score >= 65.0 {
            StoryMissionRating::ThreeStars
        } else if score >= 50.0 {
            StoryMissionRating::TwoStars
        } else if score >= 30.0 {
            StoryMissionRating::OneStar
        } else {
            StoryMissionRating::None
        }
    }

    /// 記錄玩家死亡
    pub fn record_death(&mut self) {
        self.player_deaths += 1;
    }

    /// 記錄檢查點重試
    pub fn record_retry(&mut self) {
        self.checkpoint_retries += 1;
    }

    /// 記錄射擊
    pub fn record_shot(&mut self, hit: bool, headshot: bool) {
        self.shots_fired += 1;
        if hit {
            self.shots_hit += 1;
        }
        if headshot {
            self.headshots += 1;
        }
    }

    /// 記錄擊殺
    pub fn record_kill(&mut self) {
        self.enemies_killed += 1;
    }

    /// 記錄被發現
    pub fn record_detection(&mut self) {
        self.times_detected += 1;
    }
}

/// 任務完成結果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionCompletionResult {
    /// 任務 ID
    pub mission_id: StoryMissionId,
    /// 任務名稱
    pub mission_name: String,
    /// 評分
    pub rating: StoryMissionRating,
    /// 表現數據
    pub performance: MissionPerformance,
    /// 基礎獎金
    pub base_reward: i32,
    /// 最終獎金（含加成）
    pub final_reward: i32,
    /// 解鎖的物品
    pub unlocked_items: Vec<String>,
    /// 解鎖的任務
    pub unlocked_missions: Vec<StoryMissionId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- MissionObjective ---

    #[test]
    fn objective_increment_and_completion() {
        let mut obj =
            MissionObjective::new(1, ObjectiveType::KillCount(3), "Kill 3 enemies").with_count(3);
        assert!(!obj.check_completion());
        assert!(obj.progress().abs() < f32::EPSILON);

        obj.increment();
        assert_eq!(obj.current_count, 1);
        assert!(!obj.check_completion());

        obj.increment();
        obj.increment();
        assert!(obj.check_completion());
        assert!(obj.is_completed);
        assert!((obj.progress() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn objective_increment_capped_at_target() {
        let mut obj =
            MissionObjective::new(1, ObjectiveType::KillCount(2), "Kill 2").with_count(2);
        obj.increment();
        obj.increment();
        obj.increment(); // extra
        assert_eq!(obj.current_count, 2);
    }

    #[test]
    fn objective_progress_zero_target() {
        let obj = MissionObjective {
            id: 1,
            objective_type: ObjectiveType::Custom("test".into()),
            description: "test".into(),
            target_count: 0,
            current_count: 0,
            is_optional: false,
            is_completed: false,
        };
        assert!((obj.progress() - 1.0).abs() < f32::EPSILON);
    }

    // --- MissionPhase ---

    #[test]
    fn phase_complete_ignores_optional() {
        let mut required =
            MissionObjective::new(1, ObjectiveType::KillCount(1), "required").with_count(1);
        required.increment();

        let optional =
            MissionObjective::new(2, ObjectiveType::CollectItem("bonus".into()), "optional")
                .optional();

        let phase = MissionPhase::new(1, StoryMissionType::Elimination, "test phase")
            .with_objective(required)
            .with_objective(optional);

        assert!(phase.is_complete());
    }

    // --- MissionPerformance ---

    #[test]
    fn performance_accuracy() {
        let mut perf = MissionPerformance::default();
        assert!((perf.accuracy() - 1.0).abs() < f32::EPSILON);

        perf.shots_fired = 10;
        perf.shots_hit = 7;
        assert!((perf.accuracy() - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn performance_headshot_ratio() {
        let perf = MissionPerformance {
            enemies_killed: 10,
            headshots: 4,
            ..default()
        };
        assert!((perf.headshot_ratio() - 0.4).abs() < f32::EPSILON);

        let no_kills = MissionPerformance::default();
        assert!(no_kills.headshot_ratio().abs() < f32::EPSILON);
    }

    #[test]
    fn performance_rating_perfect_run() {
        let perf = MissionPerformance {
            completion_time: 50.0,
            shots_fired: 10,
            shots_hit: 10,
            enemies_killed: 5,
            headshots: 5,
            ..default()
        };
        let rating = perf.calculate_rating(60.0);
        assert_eq!(rating, StoryMissionRating::FiveStars);
    }

    #[test]
    fn performance_rating_poor_run() {
        let perf = MissionPerformance {
            completion_time: 180.0,
            player_deaths: 3,
            checkpoint_retries: 2,
            ..default()
        };
        // score = 100 - 40(overtime) - 45(deaths) - 20(retries) + 10(accuracy) = 5
        let rating = perf.calculate_rating(60.0);
        assert_eq!(rating, StoryMissionRating::None);
    }

    // --- StoryMissionRating ---

    #[test]
    fn story_rating_stars_and_bonus() {
        assert_eq!(StoryMissionRating::None.stars(), 0);
        assert_eq!(StoryMissionRating::FiveStars.stars(), 5);
        assert!((StoryMissionRating::FiveStars.bonus_multiplier() - 2.0).abs() < f32::EPSILON);
        assert!((StoryMissionRating::None.bonus_multiplier() - 0.5).abs() < f32::EPSILON);
    }
}
