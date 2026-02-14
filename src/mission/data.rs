//! 任務資料

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

// ============================================================================
// 任務系統常數
// ============================================================================

// ============================================================================
// 外送任務閾值
// ============================================================================
/// 超時閾值（剩餘時間比例）
const OVERTIME_THRESHOLD: f32 = 0.0;
/// 一星評級閾值
const ONE_STAR_TIME_THRESHOLD: f32 = 0.1;
/// 二星評級閾值
const TWO_STAR_TIME_THRESHOLD: f32 = 0.3;
/// 三星評級閾值
const THREE_STAR_TIME_THRESHOLD: f32 = 0.5;

// ============================================================================
// 外送獎勵乘數
// ============================================================================
/// 一星獎勵乘數
const ONE_STAR_REWARD_MULTIPLIER: f32 = 0.5;
/// 二星獎勵乘數
const TWO_STAR_REWARD_MULTIPLIER: f32 = 1.0;
/// 三星獎勵乘數
const THREE_STAR_REWARD_MULTIPLIER: f32 = 1.2;
/// 四星獎勵乘數
const FOUR_STAR_REWARD_MULTIPLIER: f32 = 1.5;
/// 五星獎勵乘數
const FIVE_STAR_REWARD_MULTIPLIER: f32 = 2.0;

// ============================================================================
// 競速獎章乘數
// ============================================================================
/// 金牌獎勵乘數
const GOLD_MEDAL_MULTIPLIER: f32 = 2.0;
/// 銀牌獎勵乘數
const SILVER_MEDAL_MULTIPLIER: f32 = 1.5;
/// 銅牌獎勵乘數
const BRONZE_MEDAL_MULTIPLIER: f32 = 1.2;

// ============================================================================
// 計程車任務
// ============================================================================
/// 最大滿意度
const MAX_SATISFACTION: f32 = 1.5;
/// 滿意度到小費的基礎轉換
const SATISFACTION_TO_TIP_BASE: f32 = 0.5;
/// 優秀評級閾值
const EXCELLENT_RATING_THRESHOLD: f32 = 1.3;
/// 良好評級閾值
const GOOD_RATING_THRESHOLD: f32 = 1.0;
/// 普通評級閾值
const AVERAGE_RATING_THRESHOLD: f32 = 0.7;
/// 差評閾值
const POOR_RATING_THRESHOLD: f32 = 0.4;
/// 優秀評級小費乘數
const EXCELLENT_TIP_MULTIPLIER: f32 = 2.0;
/// 良好評級小費乘數
const GOOD_TIP_MULTIPLIER: f32 = 1.5;
/// 差評小費乘數
const POOR_TIP_MULTIPLIER: f32 = 0.5;
/// 任務狀態
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum MissionStatus {
    #[default]
    Available,
    Active,
    Completed,
    Failed,
}

/// 任務類型
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MissionType {
    Delivery,
    Taxi,
    Race,
    Explore,
    /// 暗殺任務：消滅特定目標
    Assassination,
    /// 護送任務：護送 NPC 到目的地
    Escort,
    /// 飛車追逐：追上並攔截逃跑車輛
    ChaseDown,
    /// 拍照任務：到指定地點拍攝特定場景
    Photography,
}

/// 外送評價星級
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum DeliveryRating {
    #[default]
    None,
    OneStar,    // 超時但完成
    TwoStars,   // 剛好在時限內
    ThreeStars, // 提前完成
    FourStars,  // 大幅提前
    FiveStars,  // 神速配送
}

impl DeliveryRating {
    /// 根據剩餘時間比例計算評價
    pub fn from_time_ratio(remaining_ratio: f32) -> Self {
        if remaining_ratio < OVERTIME_THRESHOLD {
            Self::OneStar      // 超時
        } else if remaining_ratio < ONE_STAR_TIME_THRESHOLD {
            Self::TwoStars     // 剛好
        } else if remaining_ratio < TWO_STAR_TIME_THRESHOLD {
            Self::ThreeStars   // 提前
        } else if remaining_ratio < THREE_STAR_TIME_THRESHOLD {
            Self::FourStars    // 大幅提前
        } else {
            Self::FiveStars    // 神速
        }
    }

    /// 取得獎勵加成倍率
    pub fn bonus_multiplier(&self) -> f32 {
        match self {
            Self::None => TWO_STAR_REWARD_MULTIPLIER,
            Self::OneStar => ONE_STAR_REWARD_MULTIPLIER,
            Self::TwoStars => TWO_STAR_REWARD_MULTIPLIER,
            Self::ThreeStars => THREE_STAR_REWARD_MULTIPLIER,
            Self::FourStars => FOUR_STAR_REWARD_MULTIPLIER,
            Self::FiveStars => FIVE_STAR_REWARD_MULTIPLIER,
        }
    }

    /// 取得星星顯示字串
    pub fn stars(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::OneStar => "⭐",
            Self::TwoStars => "⭐⭐",
            Self::ThreeStars => "⭐⭐⭐",
            Self::FourStars => "⭐⭐⭐⭐",
            Self::FiveStars => "⭐⭐⭐⭐⭐",
        }
    }
}

/// 外送訂單詳情
#[derive(Clone, Debug)]
pub struct DeliveryOrder {
    pub restaurant_name: String,   // 餐廳名稱
    pub customer_address: String,  // 顧客地址描述
    pub food_item: String,         // 餐點名稱
    pub base_pay: u32,             // 基本報酬
    pub tip_range: (u32, u32),     // 小費範圍
    pub distance: f32,             // 預估距離 (米)
}

/// 餐廳資料（外送取餐點）
#[derive(Clone, Debug)]
pub struct Restaurant {
    pub name: String,
    pub position: Vec3,
    pub food_types: Vec<String>,
}

/// 顧客地點（外送目的地）
#[derive(Clone, Debug)]
pub struct CustomerLocation {
    pub address: String,
    pub position: Vec3,
}

/// 任務資料
#[derive(Clone, Debug)]
pub struct MissionData {
    pub id: u32,
    pub mission_type: MissionType,
    pub title: String,
    pub description: String,
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub reward: u32,
    pub time_limit: Option<f32>,
    pub delivery_order: Option<DeliveryOrder>, // 外送訂單詳情
    pub race_data: Option<RaceData>,           // 競速任務資料
    pub taxi_data: Option<TaxiData>,           // 計程車任務資料
}

// ============================================================================
// 競速任務資料
// ============================================================================

/// 競速任務資料
#[derive(Clone, Debug)]
pub struct RaceData {
    /// 檢查點列表
    pub checkpoints: Vec<Vec3>,
    /// 當前檢查點索引
    pub current_checkpoint: usize,
    /// 最佳時間（秒）
    pub best_time: Option<f32>,
    /// 金牌時間（秒）
    pub gold_time: f32,
    /// 銀牌時間（秒）
    pub silver_time: f32,
    /// 銅牌時間（秒）
    pub bronze_time: f32,
}

impl RaceData {
    /// 建立新實例
    pub fn new(checkpoints: Vec<Vec3>, gold: f32, silver: f32, bronze: f32) -> Self {
        Self {
            checkpoints,
            current_checkpoint: 0,
            best_time: None,
            gold_time: gold,
            silver_time: silver,
            bronze_time: bronze,
        }
    }

    /// 取得當前檢查點位置
    pub fn current_checkpoint_pos(&self) -> Option<Vec3> {
        self.checkpoints.get(self.current_checkpoint).copied()
    }

    /// 前進到下一個檢查點
    pub fn advance_checkpoint(&mut self) -> bool {
        if self.current_checkpoint + 1 < self.checkpoints.len() {
            self.current_checkpoint += 1;
            true
        } else {
            false
        }
    }

    /// 是否已通過所有檢查點
    pub fn is_finished(&self) -> bool {
        self.current_checkpoint >= self.checkpoints.len()
    }

    /// 根據完成時間計算獎章
    pub fn medal_for_time(&self, time: f32) -> RaceMedal {
        if time <= self.gold_time {
            RaceMedal::Gold
        } else if time <= self.silver_time {
            RaceMedal::Silver
        } else if time <= self.bronze_time {
            RaceMedal::Bronze
        } else {
            RaceMedal::None
        }
    }
}

/// 競速獎章
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RaceMedal {
    #[default]
    None,
    Bronze,
    Silver,
    Gold,
}

impl RaceMedal {
    /// 取得對應 emoji
    pub fn emoji(&self) -> &'static str {
        match self {
            RaceMedal::Gold => "🥇",
            RaceMedal::Silver => "🥈",
            RaceMedal::Bronze => "🥉",
            RaceMedal::None => "",
        }
    }

    /// 計算獎勵倍率
    pub fn bonus_multiplier(&self) -> f32 {
        match self {
            RaceMedal::Gold => GOLD_MEDAL_MULTIPLIER,
            RaceMedal::Silver => SILVER_MEDAL_MULTIPLIER,
            RaceMedal::Bronze => BRONZE_MEDAL_MULTIPLIER,
            RaceMedal::None => TWO_STAR_REWARD_MULTIPLIER,
        }
    }
}

// ============================================================================
// 計程車任務資料
// ============================================================================

/// 計程車任務資料
#[derive(Clone, Debug)]
pub struct TaxiData {
    /// 乘客名稱
    pub passenger_name: String,
    /// 乘客心情描述
    pub passenger_mood: String,
    /// 目的地名稱
    pub destination_name: String,
    /// 是否已接到乘客
    pub passenger_picked_up: bool,
    /// 乘客耐心值 (0.0 ~ 1.0)
    pub patience: f32,
    /// 乘客滿意度 (根據駕駛行為變化)
    pub satisfaction: f32,
    /// 小費倍率（根據滿意度計算）
    pub tip_multiplier: f32,
}

impl TaxiData {
    /// 建立新實例
    pub fn new(passenger_name: String, destination_name: String) -> Self {
        Self {
            passenger_name,
            passenger_mood: "普通".to_string(),
            destination_name,
            passenger_picked_up: false,
            patience: 1.0,
            satisfaction: 1.0,
            tip_multiplier: 1.0,
        }
    }

    /// 更新乘客滿意度（根據駕駛行為）
    pub fn update_satisfaction(&mut self, delta: f32) {
        self.satisfaction = (self.satisfaction + delta).clamp(0.0, MAX_SATISFACTION);
        // 滿意度影響小費
        self.tip_multiplier = SATISFACTION_TO_TIP_BASE + self.satisfaction;
    }

    /// 根據滿意度取得評價
    pub fn rating(&self) -> TaxiRating {
        if self.satisfaction >= EXCELLENT_RATING_THRESHOLD {
            TaxiRating::Excellent
        } else if self.satisfaction >= GOOD_RATING_THRESHOLD {
            TaxiRating::Good
        } else if self.satisfaction >= AVERAGE_RATING_THRESHOLD {
            TaxiRating::Average
        } else if self.satisfaction >= POOR_RATING_THRESHOLD {
            TaxiRating::Poor
        } else {
            TaxiRating::Terrible
        }
    }
}

/// 計程車評價
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TaxiRating {
    Excellent,  // 非常滿意
    #[default]
    Good,       // 滿意
    Average,    // 普通
    Poor,       // 不滿意
    Terrible,   // 非常不滿意
}

impl TaxiRating {
    /// 取得對應 emoji
    pub fn emoji(&self) -> &'static str {
        match self {
            TaxiRating::Excellent => "😍",
            TaxiRating::Good => "😊",
            TaxiRating::Average => "😐",
            TaxiRating::Poor => "😒",
            TaxiRating::Terrible => "😠",
        }
    }

    /// 計算小費倍率
    pub fn tip_multiplier(&self) -> f32 {
        match self {
            TaxiRating::Excellent => 2.0,
            TaxiRating::Good => 1.5,
            TaxiRating::Average => 1.0,
            TaxiRating::Poor => 0.5,
            TaxiRating::Terrible => 0.0,
        }
    }
}

/// 已完成任務記錄（用於任務日誌）
#[derive(Clone, Debug)]
pub struct CompletedMissionRecord {
    pub title: String,
    pub mission_type: MissionType,
    pub reward: u32,
    pub stars: u8,
    pub rating_label: String,
}

impl CompletedMissionRecord {
    /// 取得星星顯示字串
    pub fn stars_display(&self) -> String {
        "★".repeat(self.stars as usize)
    }

    /// 取得任務類型顯示名稱
    pub fn type_label(&self) -> &'static str {
        match self.mission_type {
            MissionType::Delivery => "外送",
            MissionType::Taxi => "載客",
            MissionType::Race => "競速",
            MissionType::Explore => "探索",
            MissionType::Assassination => "暗殺",
            MissionType::Escort => "護送",
            MissionType::ChaseDown => "飛車追逐",
            MissionType::Photography => "拍照",
        }
    }
}

/// 任務管理器
#[derive(Resource)]
pub struct MissionManager {
    pub available_missions: Vec<MissionData>,
    pub active_mission: Option<ActiveMission>,
    pub completed_count: u32,
    pub total_earnings: u32,
    pub completed_missions: Vec<CompletedMissionRecord>, // 已完成任務歷史
    // 外送系統專用
    pub delivery_orders: Vec<MissionData>,     // 可接的外送訂單
    pub delivery_orders_changed: bool,          // 訂單列表是否變更（用於 UI 優化）
    pub delivery_streak: u32,                   // 連續完成訂單數
    pub average_rating: f32,                    // 平均評價
    pub total_deliveries: u32,                  // 總配送數
    pub restaurants: Vec<Restaurant>,           // 餐廳列表
    pub customer_locations: Vec<CustomerLocation>, // 顧客地點列表
    pub(crate) next_mission_id: u32,              // 下一個任務 ID
}

/// 進行中的任務
#[derive(Clone, Debug)]
pub struct ActiveMission {
    pub data: MissionData,
    pub status: MissionStatus,
    pub time_elapsed: f32,
    pub picked_up: bool,  // 是否已取餐
    pub last_rating: DeliveryRating, // 最後評價
}

/// 任務標記
#[derive(Component)]
pub struct MissionMarker {
    pub mission_id: u32,
    pub is_start: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- DeliveryRating ---

    #[test]
    fn delivery_rating_from_time_ratio() {
        assert_eq!(DeliveryRating::from_time_ratio(-0.1), DeliveryRating::OneStar);
        assert_eq!(DeliveryRating::from_time_ratio(0.05), DeliveryRating::TwoStars);
        assert_eq!(DeliveryRating::from_time_ratio(0.2), DeliveryRating::ThreeStars);
        assert_eq!(DeliveryRating::from_time_ratio(0.4), DeliveryRating::FourStars);
        assert_eq!(DeliveryRating::from_time_ratio(0.6), DeliveryRating::FiveStars);
    }

    #[test]
    fn delivery_rating_bonus_multiplier() {
        assert!((DeliveryRating::OneStar.bonus_multiplier() - 0.5).abs() < f32::EPSILON);
        assert!((DeliveryRating::FiveStars.bonus_multiplier() - 2.0).abs() < f32::EPSILON);
        assert!((DeliveryRating::None.bonus_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    // --- RaceData ---

    #[test]
    fn race_advance_checkpoint_and_finish() {
        let mut race = RaceData::new(
            vec![Vec3::ZERO, Vec3::X, Vec3::new(2.0, 0.0, 0.0)],
            30.0, 40.0, 50.0,
        );
        assert!(!race.is_finished());
        assert_eq!(race.current_checkpoint, 0);

        assert!(race.advance_checkpoint()); // 0 -> 1
        assert!(race.advance_checkpoint()); // 1 -> 2
        assert!(!race.advance_checkpoint()); // at last, can't advance
        assert_eq!(race.current_checkpoint, 2);
    }

    #[test]
    fn race_medal_for_time() {
        let race = RaceData::new(vec![Vec3::ZERO], 30.0, 40.0, 50.0);
        assert_eq!(race.medal_for_time(25.0), RaceMedal::Gold);
        assert_eq!(race.medal_for_time(30.0), RaceMedal::Gold); // exactly gold
        assert_eq!(race.medal_for_time(35.0), RaceMedal::Silver);
        assert_eq!(race.medal_for_time(45.0), RaceMedal::Bronze);
        assert_eq!(race.medal_for_time(60.0), RaceMedal::None);
    }

    // --- TaxiData ---

    #[test]
    fn taxi_update_satisfaction_clamped() {
        let mut taxi = TaxiData::new("Test".into(), "Dest".into());
        assert!((taxi.satisfaction - 1.0).abs() < f32::EPSILON);

        taxi.update_satisfaction(0.3);
        assert!((taxi.satisfaction - 1.3).abs() < f32::EPSILON);

        taxi.update_satisfaction(1.0); // clamp to MAX_SATISFACTION (1.5)
        assert!((taxi.satisfaction - 1.5).abs() < f32::EPSILON);

        taxi.update_satisfaction(-10.0); // clamp to 0.0
        assert!(taxi.satisfaction.abs() < f32::EPSILON);
    }

    #[test]
    fn taxi_rating_thresholds() {
        let mut taxi = TaxiData::new("Test".into(), "Dest".into());

        taxi.satisfaction = 1.4;
        assert_eq!(taxi.rating(), TaxiRating::Excellent);

        taxi.satisfaction = 1.0;
        assert_eq!(taxi.rating(), TaxiRating::Good);

        taxi.satisfaction = 0.7;
        assert_eq!(taxi.rating(), TaxiRating::Average);

        taxi.satisfaction = 0.5;
        assert_eq!(taxi.rating(), TaxiRating::Poor);

        taxi.satisfaction = 0.2;
        assert_eq!(taxi.rating(), TaxiRating::Terrible);
    }
}
