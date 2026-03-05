//! 任務評分系統

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]
#![allow(clippy::cast_precision_loss, clippy::trivially_copy_pass_by_ref)]

use super::StoryMissionId;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::{MissionObjective, MissionPhase, ObjectiveType, StoryMissionType};
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
        let mut obj = MissionObjective::new(1, ObjectiveType::KillCount(2), "Kill 2").with_count(2);
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
