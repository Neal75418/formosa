//! 劇情任務管理器
//!
//! 管理劇情任務狀態、進度追蹤、存檔讀檔

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::economy::RespectManager;
use super::story_data::*;
use super::unlocks::UnlockManager;
use crate::combat::WeaponType;
use crate::economy::PlayerWallet;

/// 劇情任務管理器資源
#[derive(Resource, Serialize, Deserialize)]
pub struct StoryMissionManager {
    /// 所有任務狀態
    pub mission_states: HashMap<StoryMissionId, StoryMissionStatus>,
    /// 當前進行中的任務
    #[serde(skip)]
    pub current_mission: Option<ActiveStoryMission>,
    /// 當前章節
    pub current_chapter: u32,
    /// 劇情旗標（用於分支判斷）
    pub story_flags: HashMap<String, bool>,
    /// 已完成的任務數量
    pub completed_count: u32,
    /// 總遊戲時間（秒）
    pub total_play_time: f32,
    pub checkpoint: Option<CheckpointData>,
    /// 當前任務表現追蹤
    #[serde(skip)]
    pub current_performance: Option<MissionPerformance>,
    /// 各任務最佳評分
    pub mission_ratings: HashMap<StoryMissionId, StoryMissionRating>,
    /// 最近完成的任務結果（用於顯示 UI）
    #[serde(skip)]
    pub last_completion_result: Option<MissionCompletionResult>,
}

impl Default for StoryMissionManager {
    fn default() -> Self {
        Self {
            mission_states: HashMap::new(),
            current_mission: None,
            current_chapter: 1,
            story_flags: HashMap::new(),
            completed_count: 0,
            total_play_time: 0.0,
            checkpoint: None,
            current_performance: None,
            mission_ratings: HashMap::new(),
            last_completion_result: None,
        }
    }
}

impl StoryMissionManager {
    /// 創建新的管理器
    pub fn new() -> Self {
        Self::default()
    }

    // ========================================================================
    // 任務狀態管理
    // ========================================================================

    /// 取得任務狀態
    pub fn get_mission_status(&self, mission_id: StoryMissionId) -> StoryMissionStatus {
        self.mission_states
            .get(&mission_id)
            .copied()
            .unwrap_or(StoryMissionStatus::Locked)
    }

    /// 設置任務狀態
    pub fn set_mission_status(&mut self, mission_id: StoryMissionId, status: StoryMissionStatus) {
        self.mission_states.insert(mission_id, status);
    }

    /// 解鎖任務
    pub fn unlock_mission(&mut self, mission_id: StoryMissionId) {
        if self.get_mission_status(mission_id) == StoryMissionStatus::Locked {
            self.set_mission_status(mission_id, StoryMissionStatus::Available);
        }
    }

    /// 開始任務
    pub fn start_mission(
        &mut self,
        mission: &StoryMission,
        wallet: &PlayerWallet,
        respect: &RespectManager,
        unlocks: &UnlockManager,
    ) -> Result<(), String> {
        // 檢查是否已有進行中任務
        if self.current_mission.is_some() {
            return Err("已有進行中的任務".to_string());
        }

        // 檢查任務是否可用
        let status = self.get_mission_status(mission.id);
        if status != StoryMissionStatus::Available {
            return Err(format!("任務狀態不可用: {:?}", status));
        }

        // 檢查解鎖條件
        if !self.check_unlock_conditions(&mission.unlock_conditions, wallet, respect, unlocks) {
            return Err("不滿足解鎖條件".to_string());
        }

        // 創建進行中任務
        let first_phase = mission.phases.first().ok_or("任務沒有階段")?;
        let active_mission = ActiveStoryMission::new(mission.id, first_phase);
        self.current_mission = Some(active_mission);

        // 開始追蹤表現
        self.current_performance = Some(MissionPerformance {
            start_time: self.total_play_time,
            ..Default::default()
        });
        self.set_mission_status(mission.id, StoryMissionStatus::InProgress);

        Ok(())
    }

    /// 完成當前任務並返回任務 ID 和需要清理的實體
    pub fn complete_current_mission(
        &mut self,
        database: &StoryMissionDatabase,
    ) -> Option<(StoryMissionId, Vec<Entity>)> {
        let active = self.current_mission.take()?;
        let mission_id = active.mission_id;

        // 從資料庫獲取任務定義
        let mission = database.get(mission_id);

        // 完成表現追蹤並計算評分
        if let Some(mut performance) = self.current_performance.take() {
            performance.completion_time = self.total_play_time - performance.start_time;

            // 從任務定義獲取目標時間（estimated_time 是分鐘，轉換為秒）
            let target_time = mission
                .map(|m| m.estimated_time * 60.0)
                .unwrap_or(300.0);
            let rating = performance.calculate_rating(target_time);

            // 更新最佳評分
            let current_best = self
                .mission_ratings
                .get(&mission_id)
                .copied()
                .unwrap_or_default();
            if rating.stars() > current_best.stars() {
                self.mission_ratings.insert(mission_id, rating);
            }

            // 從任務定義獲取基礎獎勵
            let base_reward = mission
                .map(|m| m.rewards.money as i32)
                .unwrap_or(1000);
            let final_reward = (base_reward as f32 * rating.bonus_multiplier()) as i32;

            // 從任務定義獲取任務名稱
            let mission_name = mission
                .map(|m| m.title.clone())
                .unwrap_or_else(|| format!("任務 {:?}", mission_id));

            // 從任務定義獲取解鎖物品（轉為顯示用字串）
            let unlocked_items: Vec<String> = mission
                .map(|m| {
                    let weapons: Vec<String> = m.rewards.unlock_weapons.iter().map(|w| w.save_key().to_string()).collect();
                    let vehicles: Vec<String> = m.rewards.unlock_vehicles.iter().map(|v| v.save_key().to_string()).collect();
                    weapons.into_iter().chain(vehicles).collect()
                })
                .unwrap_or_default();

            // 從任務定義獲取解鎖任務
            let unlocked_missions = mission
                .map(|m| m.rewards.unlock_missions.clone())
                .unwrap_or_default();

            // 儲存完成結果供 UI 顯示
            self.last_completion_result = Some(MissionCompletionResult {
                mission_id,
                mission_name,
                rating,
                performance,
                base_reward,
                final_reward,
                unlocked_items,
                unlocked_missions,
            });
        }

        self.set_mission_status(mission_id, StoryMissionStatus::Completed);
        self.completed_count += 1;
        Some((mission_id, active.spawned_entities))
    }

    /// 失敗當前任務，返回需要清理的實體和是否可重試
    pub fn fail_current_mission(&mut self, reason: FailCondition) -> Vec<Entity> {
        // 先取出任務，避免借用衝突
        let Some(active) = self.current_mission.take() else {
            info!("任務失敗: {:?}", reason);
            return Vec::new();
        };

        let mission_id = active.mission_id;

        // 記錄死亡（如果是玩家死亡）
        if matches!(reason, FailCondition::PlayerDeath) {
            if let Some(ref mut perf) = self.current_performance {
                perf.record_death();
            }
        }

        // 如果有檢查點，設為可重試狀態；否則設為失敗
        if self.checkpoint.is_some() {
            // 任務保持可用狀態以便重試
            self.set_mission_status(mission_id, StoryMissionStatus::Available);
            info!("任務失敗: {:?}，可從檢查點重試", reason);
        } else {
            self.set_mission_status(mission_id, StoryMissionStatus::Failed);
            self.current_performance = None;
            info!("任務失敗: {:?}", reason);
        }

        // 返回需要清理的實體
        active.spawned_entities
    }

    /// 從檢查點重試任務
    pub fn retry_from_checkpoint(
        &mut self,
        database: &StoryMissionDatabase,
    ) -> Result<Vec3, String> {
        let checkpoint = self.checkpoint.clone().ok_or("沒有可用的檢查點")?;

        let mission = database
            .get(checkpoint.mission_id)
            .ok_or("找不到任務資料")?;

        // 記錄重試次數
        self.record_checkpoint_retry();

        // 重新開始任務到檢查點階段
        if let Some(phase) = mission.get_phase(checkpoint.phase as usize) {
            let mut active_mission = ActiveStoryMission::new(checkpoint.mission_id, phase);
            active_mission.current_phase = checkpoint.phase as usize;
            self.current_mission = Some(active_mission);
            self.set_mission_status(checkpoint.mission_id, StoryMissionStatus::InProgress);

            Ok(checkpoint.player_position)
        } else {
            Err("檢查點階段無效".to_string())
        }
    }

    /// 放棄當前任務，返回需要清理的實體
    pub fn abandon_current_mission(&mut self) -> Vec<Entity> {
        let Some(active) = self.current_mission.take() else {
            return Vec::new();
        };

        // 放棄的任務可以重新嘗試
        self.set_mission_status(active.mission_id, StoryMissionStatus::Available);
        self.current_performance = None;

        // 返回需要清理的實體
        active.spawned_entities
    }

    // ========================================================================
    // 表現追蹤
    // ========================================================================

    /// 記錄玩家死亡
    pub fn record_player_death(&mut self) {
        if let Some(ref mut perf) = self.current_performance {
            perf.record_death();
        }
    }

    /// 記錄檢查點重試
    pub fn record_checkpoint_retry(&mut self) {
        if let Some(ref mut perf) = self.current_performance {
            perf.record_retry();
        }
    }

    /// 記錄射擊
    pub fn record_shot(&mut self, hit: bool, headshot: bool) {
        if let Some(ref mut perf) = self.current_performance {
            perf.record_shot(hit, headshot);
        }
    }

    /// 記錄擊殺
    pub fn record_kill(&mut self) {
        if let Some(ref mut perf) = self.current_performance {
            perf.record_kill();
        }
    }

    /// 記錄被發現（隱匿任務）
    pub fn record_detection(&mut self) {
        if let Some(ref mut perf) = self.current_performance {
            perf.record_detection();
        }
    }

    /// 取得當前任務最佳評分
    pub fn get_best_rating(&self, mission_id: StoryMissionId) -> StoryMissionRating {
        self.mission_ratings
            .get(&mission_id)
            .copied()
            .unwrap_or_default()
    }

    /// 清除最近完成結果（UI 已顯示後）
    pub fn clear_completion_result(&mut self) {
        self.last_completion_result = None;
    }

    // ========================================================================
    // 條件檢查
    // ========================================================================

    /// 檢查多個解鎖條件（全部需滿足）
    pub fn check_unlock_conditions(
        &self,
        conditions: &[UnlockCondition],
        wallet: &PlayerWallet,
        respect: &RespectManager,
        unlocks: &UnlockManager,
    ) -> bool {
        conditions
            .iter()
            .all(|c| self.check_unlock_condition(c, wallet, respect, unlocks))
    }

    /// 檢查單個解鎖條件
    pub fn check_unlock_condition(
        &self,
        condition: &UnlockCondition,
        wallet: &PlayerWallet,
        _respect: &RespectManager,
        _unlocks: &UnlockManager,
    ) -> bool {
        match condition {
            UnlockCondition::CompleteMission(id) => {
                self.get_mission_status(*id) == StoryMissionStatus::Completed
            }
            UnlockCondition::ChapterReached(chapter) => self.current_chapter >= *chapter,
            UnlockCondition::MoneyAmount(min) => wallet.cash >= *min as i32,
            UnlockCondition::TimeOfDay(_start, _end) => {
                // 需要從外部傳入當前時間
                // 這裡暫時返回 true
                true
            }
            UnlockCondition::HasFlag(flag) => self.get_flag(flag),
            UnlockCondition::None => true,
        }
    }

    // ========================================================================
    // 劇情旗標
    // ========================================================================

    /// 取得劇情旗標
    pub fn get_flag(&self, flag: &str) -> bool {
        self.story_flags.get(flag).copied().unwrap_or(false)
    }

    /// 設置劇情旗標
    pub fn set_flag(&mut self, flag: impl Into<String>, value: bool) {
        self.story_flags.insert(flag.into(), value);
    }

    /// 切換劇情旗標
    pub fn toggle_flag(&mut self, flag: &str) {
        let current = self.get_flag(flag);
        self.set_flag(flag.to_string(), !current);
    }

    // ========================================================================
    // 金錢與聲望
    // ========================================================================

    // ========================================================================
    // 獎勵發放
    // ========================================================================

    /// 發放任務獎勵
    pub fn grant_rewards(
        &mut self,
        rewards: &MissionRewards,
        wallet: &mut PlayerWallet,
        respect: &mut RespectManager,
        unlocks: &mut UnlockManager,
    ) {
        wallet.add_cash(rewards.money as i32);
        respect.add_respect(rewards.respect as i32);

        // 解鎖武器
        for weapon in &rewards.unlock_weapons {
            unlocks.unlock_item(weapon.save_key());
        }

        // 解鎖載具
        for vehicle in &rewards.unlock_vehicles {
            unlocks.unlock_item(vehicle.save_key());
        }

        // 解鎖區域
        for &area in &rewards.unlock_areas {
            unlocks.unlock_area(area);
        }

        // 解鎖任務
        for &mission in &rewards.unlock_missions {
            self.unlock_mission(mission);
        }

        // 設置劇情旗標
        for flag in &rewards.set_flags {
            self.set_flag(flag.clone(), true);
        }
    }

    // ========================================================================
    // 檢查點系統
    // ========================================================================

    /// 創建檢查點
    pub fn create_checkpoint(&mut self, position: Vec3, phase: u32) {
        if let Some(active) = &self.current_mission {
            self.checkpoint = Some(CheckpointData {
                mission_id: active.mission_id,
                phase,
                player_position: position,
                timestamp: self.total_play_time,
                objectives_state: active.objectives.clone(),
            });
            info!("檢查點已創建: 任務 {}, 階段 {}", active.mission_id, phase);
        }
    }

    /// 載入檢查點
    pub fn load_checkpoint(&self) -> Option<&CheckpointData> {
        self.checkpoint.as_ref()
    }

    /// 驗證並載入檢查點（需要資料庫來驗證任務是否存在）
    pub fn validate_and_load_checkpoint(
        &self,
        database: &StoryMissionDatabase,
    ) -> Result<CheckpointData, CheckpointError> {
        let checkpoint = self
            .checkpoint
            .as_ref()
            .ok_or(CheckpointError::NoCheckpoint)?;

        // 驗證任務存在
        let mission = database
            .get(checkpoint.mission_id)
            .ok_or(CheckpointError::MissionNotFound(checkpoint.mission_id))?;

        // 驗證階段有效
        if mission.get_phase(checkpoint.phase as usize).is_none() {
            return Err(CheckpointError::InvalidPhase {
                mission_id: checkpoint.mission_id,
                phase: checkpoint.phase,
            });
        }

        // 驗證時間戳合理
        if checkpoint.timestamp > self.total_play_time {
            return Err(CheckpointError::InvalidTimestamp);
        }

        Ok(checkpoint.clone())
    }

    /// 清除檢查點
    pub fn clear_checkpoint(&mut self) {
        self.checkpoint = None;
    }

    // ========================================================================
    // 進度查詢
    // ========================================================================

    /// 取得完成百分比
    pub fn completion_percentage(&self, total_missions: usize) -> f32 {
        if total_missions == 0 {
            return 0.0;
        }
        (self.completed_count as f32 / total_missions as f32) * 100.0
    }

    /// 取得可用任務列表
    pub fn get_available_missions(&self) -> Vec<StoryMissionId> {
        self.mission_states
            .iter()
            .filter(|(_, &status)| status == StoryMissionStatus::Available)
            .map(|(&id, _)| id)
            .collect()
    }

    /// 取得已完成任務列表
    pub fn get_completed_missions(&self) -> Vec<StoryMissionId> {
        self.mission_states
            .iter()
            .filter(|(_, &status)| status == StoryMissionStatus::Completed)
            .map(|(&id, _)| id)
            .collect()
    }
}

/// 檢查點資料
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    /// 任務 ID
    pub mission_id: StoryMissionId,
    /// 階段索引
    pub phase: u32,
    /// 玩家位置
    pub player_position: Vec3,
    /// 時間戳
    pub timestamp: f32,
    /// 目標狀態快照
    pub objectives_state: Vec<MissionObjective>,
}

/// 檢查點錯誤類型
#[derive(Debug, Clone)]
pub enum CheckpointError {
    /// 沒有檢查點
    NoCheckpoint,
    /// 任務不存在
    MissionNotFound(StoryMissionId),
    /// 無效的階段
    InvalidPhase {
        mission_id: StoryMissionId,
        phase: u32,
    },
    /// 無效的時間戳
    InvalidTimestamp,
}

/// 劇情任務事件
#[derive(Message, Debug, Clone)]
pub enum StoryMissionEvent {
    /// 任務開始
    Started(StoryMissionId),
    /// 任務完成
    Completed {
        mission_id: StoryMissionId,
        rewards: MissionRewards,
    },
    /// 任務失敗
    Failed {
        mission_id: StoryMissionId,
        reason: FailCondition,
    },
    /// 任務放棄
    Abandoned(StoryMissionId),
    /// 階段切換
    PhaseChanged {
        mission_id: StoryMissionId,
        new_phase: u32,
    },
    /// 目標完成
    ObjectiveCompleted {
        mission_id: StoryMissionId,
        objective_index: usize,
    },
    /// 目標更新
    ObjectiveUpdated {
        mission_id: StoryMissionId,
        objective_index: usize,
        progress: u32,
        total: u32,
    },
    /// 檢查點到達
    CheckpointReached {
        mission_id: StoryMissionId,
        phase: u32,
    },
    /// 任務解鎖
    MissionUnlocked(StoryMissionId),
    /// 金錢變化
    MoneyChanged { old: i32, new: i32 },
    /// 聲望變化
    RespectChanged { old: i32, new: i32 },
}

/// 劇情任務資料庫資源（儲存所有任務定義）
#[derive(Resource, Default)]
pub struct StoryMissionDatabase {
    /// 所有任務定義
    pub missions: HashMap<StoryMissionId, StoryMission>,
    /// 章節任務映射
    pub chapters: HashMap<u32, Vec<StoryMissionId>>,
}

impl StoryMissionDatabase {
    /// 註冊任務
    pub fn register(&mut self, mission: StoryMission) {
        let id = mission.id;
        let chapter = mission.chapter;
        self.missions.insert(id, mission);

        // 添加到章節映射
        self.chapters.entry(chapter).or_default().push(id);
    }

    /// 取得任務定義
    pub fn get(&self, id: StoryMissionId) -> Option<&StoryMission> {
        self.missions.get(&id)
    }

    /// 取得章節所有任務
    pub fn get_chapter_missions(&self, chapter: u32) -> Vec<&StoryMission> {
        self.chapters
            .get(&chapter)
            .map(|ids| ids.iter().filter_map(|id| self.missions.get(id)).collect())
            .unwrap_or_default()
    }

    /// 取得所有任務數量
    pub fn total_count(&self) -> usize {
        self.missions.len()
    }
}

// ============================================================================
// 範例任務建構
// ============================================================================

/// 創建範例任務（用於測試）
pub fn create_sample_missions(database: &mut StoryMissionDatabase) {
    // 第一章第一個任務：對話任務
    let mission1 = StoryMission::new(1, "初來乍到", "在酒吧與神秘人交談，了解這座島嶼的情況")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(50.0, 0.0, 50.0))
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Dialogue, "找到神秘人")
                .with_objective(MissionObjective::new(
                    1,
                    ObjectiveType::ReachLocation(Vec3::new(55.0, 0.0, 55.0), 3.0),
                    "前往酒吧",
                ))
                .with_start_dialogue(1),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Dialogue, "與老王交談").with_objective(
                MissionObjective::new(
                    2,
                    ObjectiveType::TalkToNpc("mysterious_man".to_string()),
                    "與神秘人交談",
                ),
            ),
        )
        .with_rewards(
            MissionRewards::money(100)
                .with_respect(10)
                .unlock_mission(2),
        );

    // 第一章第二個任務：戰鬥任務
    let mission2 = StoryMission::new(2, "收債", "幫老王去向一個欠錢的人討債")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(100.0, 0.0, 100.0))
        .requires_mission(1) // 需要先完成任務 1
        .difficulty(Difficulty::Normal)
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Dialogue, "前往目標地點")
                .with_objective(MissionObjective::new(
                    1,
                    ObjectiveType::ReachLocation(Vec3::new(150.0, 0.0, 120.0), 5.0),
                    "前往工業區倉庫",
                ))
                .with_start_dialogue(2),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Elimination, "消滅守衛")
                .with_objective(
                    MissionObjective::new(2, ObjectiveType::KillCount(3), "消滅守衛").with_count(3),
                )
                .with_time_limit(180.0),
        )
        .with_phase(
            MissionPhase::new(3, StoryMissionType::Dialogue, "找到目標").with_objective(
                MissionObjective::new(
                    3,
                    ObjectiveType::TalkToNpc("debtor".to_string()),
                    "找到欠債人",
                ),
            ),
        )
        .with_rewards(
            MissionRewards::money(500)
                .with_respect(25)
                .unlock_mission(3),
        );

    // 第一章第三個任務：追車任務
    let mission3 = StoryMission::new(3, "追蹤線索", "追蹤一輛可疑車輛，找出幕後老闆")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(80.0, 0.0, -50.0))
        .requires_mission(2) // 需要先完成任務 2
        .difficulty(Difficulty::Normal)
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Chase, "等待目標出現")
                .with_objective(MissionObjective::new(
                    1,
                    ObjectiveType::ReachLocation(Vec3::new(100.0, 0.0, -80.0), 5.0),
                    "前往監視點",
                ))
                .with_start_dialogue(3),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Chase, "追蹤可疑車輛")
                .with_objective(MissionObjective::new(
                    2,
                    ObjectiveType::FollowTarget("suspect_vehicle".to_string(), 50.0),
                    "追蹤車輛",
                ))
                .with_time_limit(120.0)
                .with_fail_condition(FailCondition::TargetEscaped),
        )
        .with_phase(
            MissionPhase::new(3, StoryMissionType::Dialogue, "記下地點").with_objective(
                MissionObjective::new(
                    3,
                    ObjectiveType::ReachLocation(Vec3::new(200.0, 0.0, -150.0), 5.0),
                    "到達目的地",
                ),
            ),
        )
        .with_rewards(
            MissionRewards::money(300)
                .with_respect(20)
                .unlock_mission(4)
                .set_flag("found_hideout".to_string()),
        );

    // 第一章第四個任務：潛入任務
    let mission4 = StoryMission::new(4, "夜間行動", "潛入老闆的秘密據點，取得證據")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(200.0, 0.0, -150.0))
        .requires_mission(3)
        .requires_flag("found_hideout")
        .difficulty(Difficulty::Hard)
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Stealth, "潛入大樓")
                .with_objective(MissionObjective::new(
                    1,
                    ObjectiveType::ReachLocation(Vec3::new(210.0, 0.0, -160.0), 3.0),
                    "找到側門入口",
                ))
                .with_start_dialogue(4)
                .with_fail_condition(FailCondition::Detected),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Retrieve, "取得證據")
                .with_objective(MissionObjective::new(
                    2,
                    ObjectiveType::CollectItem("evidence_files".to_string()),
                    "找到機密文件",
                ))
                .with_objective(MissionObjective::new(
                    3,
                    ObjectiveType::CollectItem("financial_records".to_string()),
                    "找到財務記錄",
                )),
        )
        .with_phase(
            MissionPhase::new(3, StoryMissionType::Stealth, "離開建築")
                .with_objective(MissionObjective::new(
                    4,
                    ObjectiveType::ReachLocation(Vec3::new(180.0, 0.0, -140.0), 5.0),
                    "安全撤離",
                ))
                .with_fail_condition(FailCondition::Detected),
        )
        .with_rewards(
            MissionRewards::money(800)
                .with_respect(40)
                .unlock_mission(5)
                .set_flag("has_evidence".to_string()),
        );

    // 第一章最終任務：刺殺老闆
    let mission5 = StoryMission::new(5, "清算日", "帶著證據找老闆算帳，結束這一切")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(0.0, 0.0, 200.0))
        .requires_mission(4)
        .requires_flag("has_evidence")
        .difficulty(Difficulty::Hard)
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Elimination, "殺進去")
                .with_objective(
                    MissionObjective::new(1, ObjectiveType::KillCount(5), "消滅門衛").with_count(5),
                )
                .with_start_dialogue(5),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Assassination, "找到老闆")
                .with_objective(MissionObjective::new(
                    2,
                    ObjectiveType::KillTarget("boss".to_string()),
                    "消滅老闆",
                ))
                .with_time_limit(300.0),
        )
        .with_phase(
            MissionPhase::new(3, StoryMissionType::Dialogue, "任務完成")
                .with_objective(MissionObjective::new(
                    3,
                    ObjectiveType::ReachLocation(Vec3::new(50.0, 0.0, 50.0), 5.0),
                    "回去向老王回報",
                ))
                .with_end_dialogue(6),
        )
        .with_rewards(
            MissionRewards::money(2000)
                .with_respect(100)
                .unlock_weapon(WeaponType::Rifle)
                .set_flag("chapter1_complete".to_string()),
        );

    database.register(mission1);
    database.register(mission2);
    database.register(mission3);
    database.register(mission4);
    database.register(mission5);
}
