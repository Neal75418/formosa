//! 任務定義

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use super::{
    StoryMissionId, CutsceneId, NpcId,
    UnlockCondition, MissionPhase, MissionRewards, Difficulty, MissionObjective
};

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
