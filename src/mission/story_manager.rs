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
use crate::core::WorldTime;
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
        world_time: &WorldTime,
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
        if !self.check_unlock_conditions(&mission.unlock_conditions, wallet, respect, unlocks, world_time) {
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
        world_time: &WorldTime,
    ) -> bool {
        conditions
            .iter()
            .all(|c| self.check_unlock_condition(c, wallet, respect, unlocks, world_time))
    }

    /// 檢查單個解鎖條件
    pub fn check_unlock_condition(
        &self,
        condition: &UnlockCondition,
        wallet: &PlayerWallet,
        _respect: &RespectManager,
        _unlocks: &UnlockManager,
        world_time: &WorldTime,
    ) -> bool {
        match condition {
            UnlockCondition::CompleteMission(id) => {
                self.get_mission_status(*id) == StoryMissionStatus::Completed
            }
            UnlockCondition::ChapterReached(chapter) => self.current_chapter >= *chapter,
            UnlockCondition::MoneyAmount(min) => wallet.cash >= *min as i32,
            UnlockCondition::TimeOfDay(start, end) => {
                if (*start - *end).abs() < f32::EPSILON {
                    return true; // 相同起止時間 = 全天可用
                }
                let hour = world_time.hour;
                if start < end {
                    hour >= *start && hour < *end
                } else {
                    // 跨午夜（例如 22:00 ~ 06:00）
                    hour >= *start || hour < *end
                }
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
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mission(id: StoryMissionId) -> StoryMission {
        StoryMission::new(id, format!("測試任務{}", id), "測試描述")
            .chapter(1)
            .difficulty(Difficulty::Normal)
            .with_phase(MissionPhase::new(
                1,
                StoryMissionType::Dialogue,
                "階段1",
            ))
    }

    #[test]
    fn test_story_mission_manager_default() {
        let manager = StoryMissionManager::default();

        assert!(manager.mission_states.is_empty());
        assert!(manager.current_mission.is_none());
        assert_eq!(manager.current_chapter, 1);
        assert_eq!(manager.completed_count, 0);
        assert_eq!(manager.total_play_time, 0.0);
    }

    #[test]
    fn test_mission_status_transitions() {
        let mut manager = StoryMissionManager::new();
        let mission_id = 1;

        // 初始狀態應為 Locked
        assert_eq!(
            manager.get_mission_status(mission_id),
            StoryMissionStatus::Locked
        );

        // 解鎖任務
        manager.unlock_mission(mission_id);
        assert_eq!(
            manager.get_mission_status(mission_id),
            StoryMissionStatus::Available
        );

        // 設置為進行中
        manager.set_mission_status(mission_id, StoryMissionStatus::InProgress);
        assert_eq!(
            manager.get_mission_status(mission_id),
            StoryMissionStatus::InProgress
        );

        // 完成任務
        manager.set_mission_status(mission_id, StoryMissionStatus::Completed);
        assert_eq!(
            manager.get_mission_status(mission_id),
            StoryMissionStatus::Completed
        );
    }

    #[test]
    fn test_start_mission_success() {
        let mut manager = StoryMissionManager::new();
        let mission = create_test_mission(1);

        // 先解鎖任務
        manager.unlock_mission(1);

        // 開始任務
        let wallet = PlayerWallet::default();
        let respect = RespectManager::default();
        let unlocks = UnlockManager::default();
        let world_time = WorldTime::default();

        let result = manager.start_mission(&mission, &wallet, &respect, &unlocks, &world_time);
        assert!(result.is_ok());

        // 驗證狀態
        assert!(manager.current_mission.is_some());
        assert_eq!(
            manager.get_mission_status(1),
            StoryMissionStatus::InProgress
        );
        assert!(manager.current_performance.is_some());
    }

    #[test]
    fn test_start_mission_already_in_progress() {
        let mut manager = StoryMissionManager::new();
        let mission1 = create_test_mission(1);
        let mission2 = create_test_mission(2);

        manager.unlock_mission(1);
        manager.unlock_mission(2);

        let wallet = PlayerWallet::default();
        let respect = RespectManager::default();
        let unlocks = UnlockManager::default();
        let world_time = WorldTime::default();

        // 開始第一個任務
        manager
            .start_mission(&mission1, &wallet, &respect, &unlocks, &world_time)
            .unwrap();

        // 嘗試開始第二個任務應該失敗
        let result = manager.start_mission(&mission2, &wallet, &respect, &unlocks, &world_time);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "已有進行中的任務");
    }

    #[test]
    fn test_checkpoint_save_and_restore() {
        let mut manager = StoryMissionManager::new();
        let mission = create_test_mission(1);
        manager.unlock_mission(1);

        let wallet = PlayerWallet::default();
        let respect = RespectManager::default();
        let unlocks = UnlockManager::default();
        let world_time = WorldTime::default();

        manager
            .start_mission(&mission, &wallet, &respect, &unlocks, &world_time)
            .unwrap();

        // 保存 checkpoint
        let checkpoint_pos = Vec3::new(10.0, 0.0, 5.0);
        manager.create_checkpoint(checkpoint_pos, 1);

        assert!(manager.checkpoint.is_some());
        let checkpoint = manager.checkpoint.as_ref().unwrap();
        assert_eq!(checkpoint.mission_id, 1);
        assert_eq!(checkpoint.player_position, checkpoint_pos);
    }

    #[test]
    fn test_story_flag_management() {
        let mut manager = StoryMissionManager::new();

        // 設置旗標
        manager.set_flag("test_flag".to_string(), true);
        assert!(manager.get_flag("test_flag"));

        // 修改旗標
        manager.set_flag("test_flag".to_string(), false);
        assert!(!manager.get_flag("test_flag"));

        // 取得不存在的旗標（返回 false）
        assert!(!manager.get_flag("nonexistent"));
    }

    #[test]
    fn test_database_registration() {
        let mut db = StoryMissionDatabase::default();
        assert_eq!(db.total_count(), 0);

        let mission1 = create_test_mission(1);
        let mission2 = create_test_mission(2);

        db.register(mission1);
        db.register(mission2);

        assert_eq!(db.total_count(), 2);
        assert!(db.get(1).is_some());
        assert!(db.get(2).is_some());
        assert!(db.get(999).is_none());
    }

    #[test]
    fn test_database_chapter_organization() {
        let mut db = StoryMissionDatabase::default();

        let mission1 = StoryMission::new(1, "章節1任務1", "描述").chapter(1);
        let mission2 = StoryMission::new(2, "章節1任務2", "描述").chapter(1);
        let mission3 = StoryMission::new(3, "章節2任務1", "描述").chapter(2);

        db.register(mission1);
        db.register(mission2);
        db.register(mission3);

        let chapter1_missions = db.get_chapter_missions(1);
        assert_eq!(chapter1_missions.len(), 2);

        let chapter2_missions = db.get_chapter_missions(2);
        assert_eq!(chapter2_missions.len(), 1);

        let chapter3_missions = db.get_chapter_missions(3);
        assert_eq!(chapter3_missions.len(), 0);
    }

    #[test]
    fn test_mission_performance_tracking() {
        let performance = MissionPerformance {
            start_time: 0.0,
            completion_time: 120.0, // 2 分鐘
            player_deaths: 1,
            damage_taken: 50.0,
            enemies_killed: 5,
            ..Default::default()
        };

        // 計算評分（目標時間 180 秒 = 3 分鐘）
        let rating = performance.calculate_rating(180.0);

        // 快速完成應有額外獎勵
        assert!(rating.bonus_multiplier() >= 1.0);

        // 但有死亡和損壞應降低評分
        assert!(rating.stars() <= 5);
    }

    #[test]
    fn test_unlock_manager_default() {
        let unlocks = UnlockManager::default();
        // 驗證 UnlockManager 可以正常創建
        assert!(unlocks.unlocked_items.is_empty());
        // unlocked_areas 可能有預設值，所以只驗證欄位存在
    }

    #[test]
    fn test_respect_manager_default() {
        let respect = RespectManager::default();
        assert_eq!(respect.respect, 0);
    }
}
