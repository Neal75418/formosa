//! 任務資料

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use rand::Rng;

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
/// 最小小費比例
const TIP_MIN_RATIO: f32 = 0.1;
/// 最大小費比例
const TIP_MAX_RATIO: f32 = 0.3;
/// 最小時限（秒）
const MIN_TIME_LIMIT: f32 = 30.0;
/// 最大時限（秒）
const MAX_TIME_LIMIT: f32 = 180.0;

// ============================================================================
// 連擊獎勵
// ============================================================================
/// 每連擊獎勵比例
const STREAK_BONUS_PER_DELIVERY: f32 = 0.05;
/// 最大連擊數
const MAX_STREAK_COUNT: u32 = 10;

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

impl Default for MissionManager {
    fn default() -> Self {
        let restaurants = create_restaurants();
        let customer_locations = create_customer_locations();

        Self {
            available_missions: create_default_missions(),
            active_mission: None,
            completed_count: 0,
            total_earnings: 0,
            completed_missions: Vec::new(),
            delivery_orders: Vec::new(),
            delivery_orders_changed: true, // 初始為 true 以觸發首次渲染
            delivery_streak: 0,
            average_rating: 0.0,
            total_deliveries: 0,
            restaurants,
            customer_locations,
            next_mission_id: 100, // 動態生成的任務從 100 開始
        }
    }
}

impl MissionManager {
    /// 生成新的外送訂單
    pub fn generate_delivery_order(&mut self) -> MissionData {
        let mut rng = rand::rng();

        // 隨機選擇餐廳和顧客
        let restaurant_idx = rng.random_range(0..self.restaurants.len());
        let customer_idx = rng.random_range(0..self.customer_locations.len());

        let restaurant = &self.restaurants[restaurant_idx];
        let customer = &self.customer_locations[customer_idx];

        // 隨機選擇餐點
        let food_idx = rng.random_range(0..restaurant.food_types.len());
        let food_item = restaurant.food_types[food_idx].clone();

        // 計算距離和報酬
        let distance = restaurant.position.distance(customer.position);
        let base_pay = calculate_delivery_pay(distance);
        let tip_min = (base_pay as f32 * TIP_MIN_RATIO) as u32;
        let tip_max = (base_pay as f32 * TIP_MAX_RATIO) as u32;

        // 計算時間限制 (每 10 米給 1 秒)
        let time_limit = (distance / 10.0).clamp(MIN_TIME_LIMIT, MAX_TIME_LIMIT);

        let id = self.next_mission_id;
        self.next_mission_id += 1;

        let delivery_order = DeliveryOrder {
            restaurant_name: restaurant.name.clone(),
            customer_address: customer.address.clone(),
            food_item: food_item.clone(),
            base_pay,
            tip_range: (tip_min, tip_max),
            distance,
        };

        MissionData {
            id,
            mission_type: MissionType::Delivery,
            title: format!("🍜 {} 外送", restaurant.name),
            description: format!("將 {} 送到 {}", food_item, customer.address),
            start_pos: restaurant.position,
            end_pos: customer.position,
            reward: base_pay,
            time_limit: Some(time_limit),
            delivery_order: Some(delivery_order),
            race_data: None,
            taxi_data: None,
        }
    }

    /// 刷新可用的外送訂單列表（生成 3-5 個訂單）
    pub fn refresh_delivery_orders(&mut self) {
        self.delivery_orders.clear();
        let mut rng = rand::rng();
        let order_count = rng.random_range(3..=5);

        for _ in 0..order_count {
            let order = self.generate_delivery_order();
            self.delivery_orders.push(order);
        }

        // 標記訂單已變更，觸發 UI 更新
        self.delivery_orders_changed = true;
    }

    /// 完成外送並計算評價
    pub fn complete_delivery(&mut self, rating: DeliveryRating) -> u32 {
        self.delivery_streak += 1;
        self.total_deliveries += 1;

        // 更新平均評價
        let rating_value = match rating {
            DeliveryRating::None => 0.0,
            DeliveryRating::OneStar => 1.0,
            DeliveryRating::TwoStars => 2.0,
            DeliveryRating::ThreeStars => 3.0,
            DeliveryRating::FourStars => 4.0,
            DeliveryRating::FiveStars => 5.0,
        };
        self.average_rating = (self.average_rating * (self.total_deliveries - 1) as f32
                               + rating_value) / self.total_deliveries as f32;

        // 計算最終報酬（含連擊加成）
        let base_reward = self.active_mission.as_ref()
            .map(|m| m.data.reward)
            .unwrap_or(0);

        let bonus = rating.bonus_multiplier();
        let streak_bonus = 1.0 + (self.delivery_streak.min(10) as f32 * 0.05); // 每連續一單 +5%，最多 +50%

        let final_reward = (base_reward as f32 * bonus * streak_bonus) as u32;
        self.total_earnings += final_reward;
        self.completed_count += 1;

        final_reward
    }

    /// 失敗時重置連擊
    pub fn fail_delivery(&mut self) {
        self.delivery_streak = 0;
    }

    /// 生成競速任務
    pub fn generate_race_mission(&mut self) -> MissionData {
        let mut rng = rand::rng();

        // 選擇預定義的賽道
        let races = create_race_courses();
        let race_idx = rng.random_range(0..races.len());
        let (name, checkpoints, gold, silver, bronze, reward) = races[race_idx].clone();

        let id = self.next_mission_id;
        self.next_mission_id += 1;

        let start_pos = *checkpoints.first().expect("Race must have at least one checkpoint");
        let end_pos = *checkpoints.last().unwrap_or(&start_pos);

        MissionData {
            id,
            mission_type: MissionType::Race,
            title: format!("🏁 {}", name),
            description: format!("通過所有檢查點！金牌: {:.1}秒", gold),
            start_pos,
            end_pos,
            reward,
            time_limit: Some(bronze + 30.0), // 超過銅牌時間 30 秒失敗
            delivery_order: None,
            race_data: Some(RaceData::new(checkpoints, gold, silver, bronze)),
            taxi_data: None,
        }
    }

    /// 生成計程車任務
    pub fn generate_taxi_mission(&mut self) -> MissionData {
        let mut rng = rand::rng();

        // 選擇乘客位置和目的地
        let passengers = create_taxi_passengers();
        let destinations = create_taxi_destinations();

        let passenger_idx = rng.random_range(0..passengers.len());
        let dest_idx = rng.random_range(0..destinations.len());

        let (passenger_name, pickup_pos) = &passengers[passenger_idx];
        let (dest_name, dest_pos) = &destinations[dest_idx];

        // 計算距離和報酬
        let distance = pickup_pos.distance(*dest_pos);
        let base_reward = calculate_taxi_fare(distance);

        // 時間限制：每 10 米 1.5 秒，最少 45 秒
        let time_limit = (distance / 10.0 * 1.5).max(45.0);

        let id = self.next_mission_id;
        self.next_mission_id += 1;

        MissionData {
            id,
            mission_type: MissionType::Taxi,
            title: format!("🚕 載客: {}", passenger_name),
            description: format!("將 {} 送到 {}", passenger_name, dest_name),
            start_pos: *pickup_pos,
            end_pos: *dest_pos,
            reward: base_reward,
            time_limit: Some(time_limit),
            delivery_order: None,
            race_data: None,
            taxi_data: Some(TaxiData::new(passenger_name.clone(), dest_name.clone())),
        }
    }

    /// 刷新競速和計程車任務列表
    pub fn refresh_special_missions(&mut self) {
        // 生成 2 個競速任務
        for _ in 0..2 {
            let race = self.generate_race_mission();
            self.available_missions.push(race);
        }

        // 生成 3 個計程車任務
        for _ in 0..3 {
            let taxi = self.generate_taxi_mission();
            self.available_missions.push(taxi);
        }
    }
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

/// 建立餐廳列表（西門町真實店家）
fn create_restaurants() -> Vec<Restaurant> {
    vec![
        Restaurant {
            name: "阿宗麵線".to_string(),
            position: Vec3::new(-10.0, 0.5, -20.0),
            food_types: vec!["大腸麵線".to_string(), "綜合麵線".to_string()],
        },
        Restaurant {
            name: "老天祿滷味".to_string(),
            position: Vec3::new(5.0, 0.5, -25.0),
            food_types: vec!["滷鴨舌".to_string(), "滷豆干".to_string(), "滷雞翅".to_string()],
        },
        Restaurant {
            name: "成都楊桃冰".to_string(),
            position: Vec3::new(-15.0, 0.5, 30.0),
            food_types: vec!["楊桃冰".to_string(), "酸梅冰".to_string()],
        },
        Restaurant {
            name: "鴨肉扁".to_string(),
            position: Vec3::new(20.0, 0.5, -10.0),
            food_types: vec!["鴨肉飯".to_string(), "鴨肉切盤".to_string(), "米粉湯".to_string()],
        },
        Restaurant {
            name: "天天利美食坊".to_string(),
            position: Vec3::new(-30.0, 0.5, 15.0),
            food_types: vec!["蚵仔煎".to_string(), "肉圓".to_string()],
        },
        Restaurant {
            name: "萬年排骨".to_string(),
            position: Vec3::new(-68.0, 0.5, -12.0),
            food_types: vec!["排骨飯".to_string(), "雞腿飯".to_string()],
        },
        Restaurant {
            name: "50嵐".to_string(),
            position: Vec3::new(10.0, 0.5, 40.0),
            food_types: vec!["珍珠奶茶".to_string(), "四季春".to_string(), "波霸鮮奶".to_string()],
        },
        Restaurant {
            name: "麥當勞西門店".to_string(),
            position: Vec3::new(45.0, 0.5, 35.0),
            food_types: vec!["大麥克".to_string(), "麥香雞".to_string(), "薯條".to_string()],
        },
    ]
}

/// 建立顧客地點列表
fn create_customer_locations() -> Vec<CustomerLocation> {
    vec![
        CustomerLocation {
            address: "西門紅樓附近".to_string(),
            position: Vec3::new(50.0, 0.5, 69.0),
        },
        CustomerLocation {
            address: "捷運西門站出口".to_string(),
            position: Vec3::new(55.0, 0.5, 36.0),
        },
        CustomerLocation {
            address: "誠品西門店".to_string(),
            position: Vec3::new(-13.5, 0.5, -13.5),
        },
        CustomerLocation {
            address: "武昌街商圈".to_string(),
            position: Vec3::new(-12.0, 0.5, -38.0),
        },
        CustomerLocation {
            address: "漢中街夜市".to_string(),
            position: Vec3::new(-45.0, 0.5, 10.0),
        },
        CustomerLocation {
            address: "萬年大樓".to_string(),
            position: Vec3::new(-68.0, 0.5, -12.0),
        },
        CustomerLocation {
            address: "獅子林大樓".to_string(),
            position: Vec3::new(-69.0, 0.5, -66.0),
        },
        CustomerLocation {
            address: "錢櫃 KTV".to_string(),
            position: Vec3::new(97.5, 0.5, 33.5),
        },
        CustomerLocation {
            address: "電影公園".to_string(),
            position: Vec3::new(-69.0, 0.5, -35.0),
        },
        CustomerLocation {
            address: "成都路口".to_string(),
            position: Vec3::new(0.0, 0.5, 50.0),
        },
    ]
}

/// 根據距離計算外送報酬
fn calculate_delivery_pay(distance: f32) -> u32 {
    // 基本費 30 + 每 10 米 5 元
    let base = 30;
    let per_unit = (distance / 10.0) as u32 * 5;
    base + per_unit
}

fn create_default_missions() -> Vec<MissionData> {
    vec![
        MissionData {
            id: 1, mission_type: MissionType::Delivery,
            title: "西門町送貨".to_string(),
            description: "將包裹從便利商店送到西門紅樓前".to_string(),
            start_pos: Vec3::new(-20.0, 0.5, 15.0),
            end_pos: Vec3::new(50.0, 0.5, 69.0),
            reward: 500, time_limit: Some(60.0),
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        },
        MissionData {
            id: 2, mission_type: MissionType::Delivery,
            title: "便利商店補貨".to_string(),
            description: "將貨物從武昌街送到漢中街".to_string(),
            start_pos: Vec3::new(-40.0, 0.5, -80.0),
            end_pos: Vec3::new(-15.0, 0.5, -50.0),
            reward: 600, time_limit: Some(90.0),
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        },
        MissionData {
            id: 3, mission_type: MissionType::Delivery,
            title: "緊急快遞".to_string(),
            description: "限時送達！從錢櫃送到誠品".to_string(),
            start_pos: Vec3::new(97.5, 0.5, 33.5),
            end_pos: Vec3::new(-13.5, 0.5, -13.5),
            reward: 1000, time_limit: Some(45.0),
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        },
    ]
}

// ============================================================================
// 競速任務資料
// ============================================================================

/// 創建預定義賽道
/// 返回: (名稱, 檢查點, 金牌時間, 銀牌時間, 銅牌時間, 獎勵)
fn create_race_courses() -> Vec<(String, Vec<Vec3>, f32, f32, f32, u32)> {
    vec![
        // 西門町環形賽道
        (
            "西門環城賽".to_string(),
            vec![
                Vec3::new(0.0, 0.5, 0.0),       // 起點：十字路口
                Vec3::new(30.0, 0.5, 0.0),      // 東行
                Vec3::new(50.0, 0.5, 30.0),     // 東北角
                Vec3::new(30.0, 0.5, 60.0),     // 北行
                Vec3::new(-30.0, 0.5, 60.0),    // 西北角
                Vec3::new(-50.0, 0.5, 30.0),    // 西行
                Vec3::new(-50.0, 0.5, -30.0),   // 西南角
                Vec3::new(-20.0, 0.5, -50.0),   // 南行
                Vec3::new(20.0, 0.5, -50.0),    // 東南角
                Vec3::new(0.0, 0.5, 0.0),       // 終點
            ],
            45.0,  // 金牌
            55.0,  // 銀牌
            70.0,  // 銅牌
            1500,  // 獎勵
        ),
        // 漢中街直線衝刺
        (
            "漢中街衝刺".to_string(),
            vec![
                Vec3::new(0.0, 0.5, -60.0),     // 起點
                Vec3::new(0.0, 0.5, -30.0),     // 檢查點 1
                Vec3::new(0.0, 0.5, 0.0),       // 檢查點 2
                Vec3::new(0.0, 0.5, 30.0),      // 檢查點 3
                Vec3::new(0.0, 0.5, 60.0),      // 終點
            ],
            20.0,  // 金牌
            25.0,  // 銀牌
            35.0,  // 銅牌
            800,   // 獎勵
        ),
        // 峨嵋街蛇形賽道
        (
            "峨嵋蛇行".to_string(),
            vec![
                Vec3::new(-50.0, 0.5, 0.0),     // 起點
                Vec3::new(-30.0, 0.5, 15.0),    // 左轉
                Vec3::new(-10.0, 0.5, -10.0),   // 右轉
                Vec3::new(10.0, 0.5, 15.0),     // 左轉
                Vec3::new(30.0, 0.5, -10.0),    // 右轉
                Vec3::new(50.0, 0.5, 0.0),      // 終點
            ],
            30.0,  // 金牌
            40.0,  // 銀牌
            50.0,  // 銅牌
            1000,  // 獎勵
        ),
        // 紅樓繞圈賽
        (
            "紅樓繞圈".to_string(),
            vec![
                Vec3::new(40.0, 0.5, 50.0),     // 起點
                Vec3::new(60.0, 0.5, 70.0),     // 檢查點 1
                Vec3::new(40.0, 0.5, 90.0),     // 檢查點 2
                Vec3::new(20.0, 0.5, 70.0),     // 檢查點 3
                Vec3::new(40.0, 0.5, 50.0),     // 終點
            ],
            25.0,  // 金牌
            32.0,  // 銀牌
            40.0,  // 銅牌
            900,   // 獎勵
        ),
    ]
}

// ============================================================================
// 計程車任務資料
// ============================================================================

/// 創建乘客列表（位置在人行道上，避開馬路中央）
fn create_taxi_passengers() -> Vec<(String, Vec3)> {
    vec![
        ("陳先生".to_string(), Vec3::new(-20.0, 0.5, 15.0)),   // 移離峨嵋街馬路
        ("林小姐".to_string(), Vec3::new(15.0, 0.5, -35.0)),   // OK
        ("王太太".to_string(), Vec3::new(-45.0, 0.5, 25.0)),   // OK
        ("張同學".to_string(), Vec3::new(30.0, 0.5, 42.0)),    // 稍微調整
        ("李伯伯".to_string(), Vec3::new(-15.0, 0.5, -55.0)),  // 移離武昌街馬路
        ("劉小弟".to_string(), Vec3::new(50.0, 0.5, 20.0)),    // OK
        ("黃阿姨".to_string(), Vec3::new(-60.0, 0.5, -20.0)),  // OK
        ("趙經理".to_string(), Vec3::new(25.0, 0.5, 45.0)),    // 調整到地圖內
    ]
}

/// 創建目的地列表（位置在地圖範圍內，靠近地標）
fn create_taxi_destinations() -> Vec<(String, Vec3)> {
    vec![
        ("西門紅樓".to_string(), Vec3::new(45.0, 0.5, 45.0)),     // 調整到地圖內
        ("捷運西門站".to_string(), Vec3::new(55.0, 0.5, 36.0)),   // OK
        ("誠品西門店".to_string(), Vec3::new(-18.0, 0.5, -18.0)), // 移離馬路
        ("萬年大樓".to_string(), Vec3::new(-68.0, 0.5, -15.0)),   // 稍微調整
        ("獅子林大樓".to_string(), Vec3::new(-65.0, 0.5, -60.0)), // 稍微調整
        ("錢櫃 KTV".to_string(), Vec3::new(70.0, 0.5, 30.0)),     // 調整到地圖內
        ("電影公園".to_string(), Vec3::new(-65.0, 0.5, -40.0)),   // 稍微調整
        ("成都路口".to_string(), Vec3::new(10.0, 0.5, 45.0)),     // 移離十字路口中央
    ]
}

/// 計算計程車車資
fn calculate_taxi_fare(distance: f32) -> u32 {
    // 起步價 70 + 每 200 米 5 元
    let base = 70;
    let per_unit = (distance / 200.0) as u32 * 5;
    base + per_unit
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
