//! 任務生成器
//!
//! 從 data.rs 拆分，處理任務生成、餐廳/顧客/賽道資料。

#![allow(dead_code)]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use bevy::prelude::*;
use rand::Rng;

use super::{
    CustomerLocation, DeliveryOrder, DeliveryRating, MissionData, MissionManager, MissionType,
    RaceData, Restaurant, TaxiData,
};

// ============================================================================
// 生成器專用常數
// ============================================================================
/// 最小小費比例
const TIP_MIN_RATIO: f32 = 0.1;
/// 最大小費比例
const TIP_MAX_RATIO: f32 = 0.3;
/// 最小時限（秒）
const MIN_TIME_LIMIT: f32 = 30.0;
/// 最大時限（秒）
const MAX_TIME_LIMIT: f32 = 180.0;
/// 每連擊獎勵比例
const STREAK_BONUS_PER_DELIVERY: f32 = 0.05;
/// 最大連擊數
const MAX_STREAK_COUNT: u32 = 10;

// ============================================================================
// MissionManager Default 實作
// ============================================================================

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

// ============================================================================
// MissionManager 生成方法
// ============================================================================

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
            + rating_value)
            / self.total_deliveries as f32;

        // 計算最終報酬（含連擊加成）
        let base_reward = self.active_mission.as_ref().map_or(0, |m| m.data.reward);

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

        let default_pos = Vec3::ZERO;
        let start_pos = checkpoints.first().copied().unwrap_or(default_pos);
        let end_pos = checkpoints.last().copied().unwrap_or(start_pos);

        MissionData {
            id,
            mission_type: MissionType::Race,
            title: format!("🏁 {name}"),
            description: format!("通過所有檢查點！金牌: {gold:.1}秒"),
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
            title: format!("🚕 載客: {passenger_name}"),
            description: format!("將 {passenger_name} 送到 {dest_name}"),
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

// ============================================================================
// 輔助函數
// ============================================================================

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
            food_types: vec![
                "滷鴨舌".to_string(),
                "滷豆干".to_string(),
                "滷雞翅".to_string(),
            ],
        },
        Restaurant {
            name: "成都楊桃冰".to_string(),
            position: Vec3::new(-15.0, 0.5, 30.0),
            food_types: vec!["楊桃冰".to_string(), "酸梅冰".to_string()],
        },
        Restaurant {
            name: "鴨肉扁".to_string(),
            position: Vec3::new(20.0, 0.5, -10.0),
            food_types: vec![
                "鴨肉飯".to_string(),
                "鴨肉切盤".to_string(),
                "米粉湯".to_string(),
            ],
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
            food_types: vec![
                "珍珠奶茶".to_string(),
                "四季春".to_string(),
                "波霸鮮奶".to_string(),
            ],
        },
        Restaurant {
            name: "麥當勞西門店".to_string(),
            position: Vec3::new(45.0, 0.5, 35.0),
            food_types: vec![
                "大麥克".to_string(),
                "麥香雞".to_string(),
                "薯條".to_string(),
            ],
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
            id: 1,
            mission_type: MissionType::Delivery,
            title: "西門町送貨".to_string(),
            description: "將包裹從便利商店送到西門紅樓前".to_string(),
            start_pos: Vec3::new(-20.0, 0.5, 15.0),
            end_pos: Vec3::new(50.0, 0.5, 69.0),
            reward: 500,
            time_limit: Some(60.0),
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        },
        MissionData {
            id: 2,
            mission_type: MissionType::Delivery,
            title: "便利商店補貨".to_string(),
            description: "將貨物從武昌街送到漢中街".to_string(),
            start_pos: Vec3::new(-40.0, 0.5, -80.0),
            end_pos: Vec3::new(-15.0, 0.5, -50.0),
            reward: 600,
            time_limit: Some(90.0),
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        },
        MissionData {
            id: 3,
            mission_type: MissionType::Delivery,
            title: "緊急快遞".to_string(),
            description: "限時送達！從錢櫃送到誠品".to_string(),
            start_pos: Vec3::new(97.5, 0.5, 33.5),
            end_pos: Vec3::new(-13.5, 0.5, -13.5),
            reward: 1000,
            time_limit: Some(45.0),
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
                Vec3::new(0.0, 0.5, 0.0),     // 起點：十字路口
                Vec3::new(30.0, 0.5, 0.0),    // 東行
                Vec3::new(50.0, 0.5, 30.0),   // 東北角
                Vec3::new(30.0, 0.5, 60.0),   // 北行
                Vec3::new(-30.0, 0.5, 60.0),  // 西北角
                Vec3::new(-50.0, 0.5, 30.0),  // 西行
                Vec3::new(-50.0, 0.5, -30.0), // 西南角
                Vec3::new(-20.0, 0.5, -50.0), // 南行
                Vec3::new(20.0, 0.5, -50.0),  // 東南角
                Vec3::new(0.0, 0.5, 0.0),     // 終點
            ],
            45.0, // 金牌
            55.0, // 銀牌
            70.0, // 銅牌
            1500, // 獎勵
        ),
        // 漢中街直線衝刺
        (
            "漢中街衝刺".to_string(),
            vec![
                Vec3::new(0.0, 0.5, -60.0), // 起點
                Vec3::new(0.0, 0.5, -30.0), // 檢查點 1
                Vec3::new(0.0, 0.5, 0.0),   // 檢查點 2
                Vec3::new(0.0, 0.5, 30.0),  // 檢查點 3
                Vec3::new(0.0, 0.5, 60.0),  // 終點
            ],
            20.0, // 金牌
            25.0, // 銀牌
            35.0, // 銅牌
            800,  // 獎勵
        ),
        // 峨嵋街蛇形賽道
        (
            "峨嵋蛇行".to_string(),
            vec![
                Vec3::new(-50.0, 0.5, 0.0),   // 起點
                Vec3::new(-30.0, 0.5, 15.0),  // 左轉
                Vec3::new(-10.0, 0.5, -10.0), // 右轉
                Vec3::new(10.0, 0.5, 15.0),   // 左轉
                Vec3::new(30.0, 0.5, -10.0),  // 右轉
                Vec3::new(50.0, 0.5, 0.0),    // 終點
            ],
            30.0, // 金牌
            40.0, // 銀牌
            50.0, // 銅牌
            1000, // 獎勵
        ),
        // 紅樓繞圈賽
        (
            "紅樓繞圈".to_string(),
            vec![
                Vec3::new(40.0, 0.5, 50.0), // 起點
                Vec3::new(60.0, 0.5, 70.0), // 檢查點 1
                Vec3::new(40.0, 0.5, 90.0), // 檢查點 2
                Vec3::new(20.0, 0.5, 70.0), // 檢查點 3
                Vec3::new(40.0, 0.5, 50.0), // 終點
            ],
            25.0, // 金牌
            32.0, // 銀牌
            40.0, // 銅牌
            900,  // 獎勵
        ),
    ]
}

// ============================================================================
// 計程車任務資料
// ============================================================================

/// 創建乘客列表（位置在人行道上，避開馬路中央）
fn create_taxi_passengers() -> Vec<(String, Vec3)> {
    vec![
        ("陳先生".to_string(), Vec3::new(-20.0, 0.5, 15.0)), // 移離峨嵋街馬路
        ("林小姐".to_string(), Vec3::new(15.0, 0.5, -35.0)), // OK
        ("王太太".to_string(), Vec3::new(-45.0, 0.5, 25.0)), // OK
        ("張同學".to_string(), Vec3::new(30.0, 0.5, 42.0)),  // 稍微調整
        ("李伯伯".to_string(), Vec3::new(-15.0, 0.5, -55.0)), // 移離武昌街馬路
        ("劉小弟".to_string(), Vec3::new(50.0, 0.5, 20.0)),  // OK
        ("黃阿姨".to_string(), Vec3::new(-60.0, 0.5, -20.0)), // OK
        ("趙經理".to_string(), Vec3::new(25.0, 0.5, 45.0)),  // 調整到地圖內
    ]
}

/// 創建目的地列表（位置在地圖範圍內，靠近地標）
fn create_taxi_destinations() -> Vec<(String, Vec3)> {
    vec![
        ("西門紅樓".to_string(), Vec3::new(45.0, 0.5, 45.0)), // 調整到地圖內
        ("捷運西門站".to_string(), Vec3::new(55.0, 0.5, 36.0)), // OK
        ("誠品西門店".to_string(), Vec3::new(-18.0, 0.5, -18.0)), // 移離馬路
        ("萬年大樓".to_string(), Vec3::new(-68.0, 0.5, -15.0)), // 稍微調整
        ("獅子林大樓".to_string(), Vec3::new(-65.0, 0.5, -60.0)), // 稍微調整
        ("錢櫃 KTV".to_string(), Vec3::new(70.0, 0.5, 30.0)), // 調整到地圖內
        ("電影公園".to_string(), Vec3::new(-65.0, 0.5, -40.0)), // 稍微調整
        ("成都路口".to_string(), Vec3::new(10.0, 0.5, 45.0)), // 移離十字路口中央
    ]
}

/// 計算計程車車資
fn calculate_taxi_fare(distance: f32) -> u32 {
    // 起步價 70 + 每 200 米 5 元
    let base = 70;
    let per_unit = (distance / 200.0) as u32 * 5;
    base + per_unit
}
