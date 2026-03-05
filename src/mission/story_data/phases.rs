//! 任務階段

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use super::{CutsceneId, DialogueId, FailCondition, MissionObjective, NpcId, StoryMissionType};
use crate::combat::{EnemyType, WeaponType};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
            .all(MissionObjective::check_completion)
    }
}
