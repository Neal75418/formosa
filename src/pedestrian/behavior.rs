//! 每日行為系統
//!
//! 定義行人的日常行為類型、興趣點和躲雨行為。

use bevy::prelude::*;
use crate::world::{W_MAIN, Z_CHENGDU};

// ============================================================================
// 日常行為系統
// ============================================================================

/// 行人日常行為組件
#[derive(Component)]
pub struct DailyBehavior {
    /// 當前行為
    pub behavior: BehaviorType,
    /// 行為持續時間
    pub duration: f32,
    /// 行為計時器
    pub timer: f32,
    /// 下一個行為（隨機選擇用）
    pub next_behavior_cooldown: f32,
}

impl Default for DailyBehavior {
    fn default() -> Self {
        Self {
            behavior: BehaviorType::Walking,
            duration: 0.0,
            timer: 0.0,
            next_behavior_cooldown: 5.0,
        }
    }
}

/// 行人行為類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BehaviorType {
    #[default]
    Walking,        // 正常行走
    PhoneWatching,  // 看手機（原地站立，偶爾低頭）
    WindowShopping, // 逛櫥窗（緩慢移動，左右看）
    Chatting,       // 聊天（與另一行人面對面站立）
    Resting,        // 休息（靠牆站或坐在長椅上）
    TakingPhoto,    // 拍照（舉起手機拍照動作）
    SeekingShelter, // 躲雨（快速跑向遮蔽處）
}

impl BehaviorType {
    /// 取得行為的典型持續時間範圍（秒）
    pub fn duration_range(&self) -> (f32, f32) {
        match self {
            BehaviorType::Walking => (10.0, 30.0),
            BehaviorType::PhoneWatching => (5.0, 15.0),
            BehaviorType::WindowShopping => (8.0, 20.0),
            BehaviorType::Chatting => (15.0, 45.0),
            BehaviorType::Resting => (20.0, 60.0),
            BehaviorType::TakingPhoto => (3.0, 8.0),
            BehaviorType::SeekingShelter => (30.0, 120.0), // 躲到雨停為止
        }
    }

    /// 行為的行走速度倍率
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            BehaviorType::Walking => 1.0,
            BehaviorType::PhoneWatching => 0.0,  // 原地不動
            BehaviorType::WindowShopping => 0.3, // 緩慢移動
            BehaviorType::Chatting => 0.0,       // 原地不動
            BehaviorType::Resting => 0.0,        // 原地不動
            BehaviorType::TakingPhoto => 0.0,    // 原地不動
            BehaviorType::SeekingShelter => 2.0, // 快速奔跑
        }
    }
}

/// 興趣點類型（用於日常行為）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointOfInterestType {
    ShopWindow,     // 商店櫥窗
    Bench,          // 長椅
    PhotoSpot,      // 拍照點
    Crosswalk,      // 斑馬線（等紅燈）
    Shelter,        // 遮蔽處（躲雨用）
}

/// 庇護點位置匹配容差
const SHELTER_POSITION_TOLERANCE: f32 = 2.0;
const SHELTER_POSITION_TOLERANCE_SQ: f32 = SHELTER_POSITION_TOLERANCE * SHELTER_POSITION_TOLERANCE;

/// 興趣點資源
#[derive(Resource, Default)]
pub struct PointsOfInterest {
    pub shop_windows: Vec<Vec3>,
    pub benches: Vec<Vec3>,
    pub photo_spots: Vec<Vec3>,
    pub shelters: Vec<ShelterPoint>,
}

/// 庇護點（躲雨用）
#[derive(Clone, Debug)]
pub struct ShelterPoint {
    /// 庇護點位置
    pub position: Vec3,
    /// 庇護點類型
    pub shelter_type: ShelterType,
    /// 容納人數上限
    pub capacity: usize,
    /// 當前佔用人數
    pub current_occupants: usize,
}

impl ShelterPoint {
    /// 建立新實例
    pub fn new(position: Vec3, shelter_type: ShelterType, capacity: usize) -> Self {
        Self {
            position,
            shelter_type,
            capacity,
            current_occupants: 0,
        }
    }

    /// 是否還有空位
    pub fn has_space(&self) -> bool {
        self.current_occupants < self.capacity
    }
}

/// 庇護點類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShelterType {
    Awning,      // 遮雨棚
    BusStop,     // 公車站
    Building,    // 建築物入口
    Overpass,    // 天橋下
}

impl PointsOfInterest {
    /// 設定西門町行人生成區域
    pub fn setup_ximending() -> Self {
        const SIDEWALK_WIDTH: f32 = 4.0;
        let bus_stop_z = Z_CHENGDU - (W_MAIN / 2.0 - SIDEWALK_WIDTH / 2.0);

        Self {
            // 商店櫥窗位置（沿街道兩側）
            shop_windows: vec![
                // 漢中街東側
                Vec3::new(12.0, 0.25, -40.0),
                Vec3::new(12.0, 0.25, -20.0),
                Vec3::new(12.0, 0.25, 0.0),
                Vec3::new(12.0, 0.25, 20.0),
                // 漢中街西側
                Vec3::new(-12.0, 0.25, -30.0),
                Vec3::new(-12.0, 0.25, -10.0),
                Vec3::new(-12.0, 0.25, 10.0),
                // 峨嵋街
                Vec3::new(-20.0, 0.25, 12.0),
                Vec3::new(0.0, 0.25, 12.0),
                Vec3::new(15.0, 0.25, 12.0),
            ],
            // 長椅位置
            benches: vec![
                Vec3::new(0.0, 0.25, -25.0),
                Vec3::new(0.0, 0.25, 15.0),
                Vec3::new(-25.0, 0.25, 0.0),
                Vec3::new(20.0, 0.25, 0.0),
            ],
            // 拍照點（地標附近）
            photo_spots: vec![
                Vec3::new(0.0, 0.25, 0.0),       // 徒步區中心
                Vec3::new(-30.0, 0.25, -30.0),  // 紅樓附近
                Vec3::new(25.0, 0.25, 25.0),    // 廣場
            ],
            // 庇護點（躲雨用）
            shelters: vec![
                // 公車站（有遮雨棚）- 移到人行道上
                ShelterPoint::new(Vec3::new(30.0, 0.25, bus_stop_z), ShelterType::BusStop, 6),
                ShelterPoint::new(Vec3::new(-30.0, 0.25, bus_stop_z), ShelterType::BusStop, 6),
                // 便利商店門口（有雨遮）
                ShelterPoint::new(Vec3::new(-55.0, 0.25, -52.0), ShelterType::Awning, 4),
                ShelterPoint::new(Vec3::new(55.0, 0.25, -52.0), ShelterType::Awning, 4),
                // 建築物入口
                ShelterPoint::new(Vec3::new(-55.0, 0.25, 52.0), ShelterType::Building, 3),
                ShelterPoint::new(Vec3::new(55.0, 0.25, 52.0), ShelterType::Building, 3),
                ShelterPoint::new(Vec3::new(0.0, 0.25, 52.0), ShelterType::Building, 5),
                // 商店騎樓
                ShelterPoint::new(Vec3::new(-25.0, 0.25, -52.0), ShelterType::Awning, 4),
                ShelterPoint::new(Vec3::new(25.0, 0.25, -52.0), ShelterType::Awning, 4),
                ShelterPoint::new(Vec3::new(0.0, 0.25, -52.0), ShelterType::Awning, 4),
                // 停車場入口
                ShelterPoint::new(Vec3::new(-40.0, 0.25, 22.0), ShelterType::Overpass, 8),
            ],
        }
    }

    /// 找到最近的興趣點
    pub fn find_nearest(&self, pos: Vec3, poi_type: PointOfInterestType, max_distance: f32) -> Option<Vec3> {
        match poi_type {
            PointOfInterestType::ShopWindow => self.find_nearest_point(&self.shop_windows, pos, max_distance),
            PointOfInterestType::Bench => self.find_nearest_point(&self.benches, pos, max_distance),
            PointOfInterestType::PhotoSpot => self.find_nearest_point(&self.photo_spots, pos, max_distance),
            PointOfInterestType::Crosswalk => None, // 暫不實作
            PointOfInterestType::Shelter => self.find_nearest_shelter(pos, max_distance),
        }
    }

    /// 找到最近的點位
    fn find_nearest_point(&self, points: &[Vec3], pos: Vec3, max_distance: f32) -> Option<Vec3> {
        let max_distance_sq = max_distance * max_distance;
        points
            .iter()
            .map(|p| (*p, p.distance_squared(pos)))
            .filter(|(_, dist_sq)| *dist_sq < max_distance_sq)
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(p, _)| p)
    }

    /// 找到最近且有空位的庇護點
    /// 優化：先計算距離再排序，避免重複計算
    pub fn find_nearest_shelter(&self, pos: Vec3, max_distance: f32) -> Option<Vec3> {
        self.shelters
            .iter()
            .filter(|s| s.has_space())
            .map(|s| (s, s.position.distance_squared(pos)))
            .filter(|(_, dist_sq)| *dist_sq < max_distance * max_distance)
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(s, _)| s.position)
    }

    /// 佔用庇護點
    pub fn occupy_shelter(&mut self, pos: Vec3) -> bool {
        if let Some(shelter) = self.shelters.iter_mut()
            .find(|s| s.position.distance_squared(pos) < SHELTER_POSITION_TOLERANCE_SQ && s.has_space())
        {
            shelter.current_occupants += 1;
            true
        } else {
            false
        }
    }

    /// 釋放庇護點
    pub fn release_shelter(&mut self, pos: Vec3) {
        if let Some(shelter) = self.shelters.iter_mut()
            .find(|s| s.position.distance_squared(pos) < SHELTER_POSITION_TOLERANCE_SQ)
        {
            shelter.current_occupants = shelter.current_occupants.saturating_sub(1);
        }
    }
}

/// 聊天夥伴標記（用於雙人聊天行為）
#[derive(Component)]
pub struct ChattingPartner {
    pub partner_entity: Entity,
}

// ============================================================================
// 躲雨行為組件
// ============================================================================

/// 躲雨狀態組件
/// 追蹤行人是否正在躲雨以及目標庇護點
#[derive(Component)]
pub struct ShelterSeeker {
    /// 目標庇護點位置
    pub target_shelter: Option<Vec3>,
    /// 是否已到達庇護點
    pub is_sheltered: bool,
    /// 躲雨開始時間（用於計算等待時間）
    pub shelter_start_time: f32,
    /// 之前的行為（雨停後恢復）
    pub previous_behavior: BehaviorType,
}

impl Default for ShelterSeeker {
    fn default() -> Self {
        Self {
            target_shelter: None,
            is_sheltered: false,
            shelter_start_time: 0.0,
            previous_behavior: BehaviorType::Walking,
        }
    }
}

impl ShelterSeeker {
    /// 開始尋找庇護
    pub fn start_seeking(&mut self, shelter_pos: Vec3, current_behavior: BehaviorType) {
        self.target_shelter = Some(shelter_pos);
        self.is_sheltered = false;
        self.previous_behavior = current_behavior;
    }

    /// 到達庇護點
    pub fn arrive_at_shelter(&mut self, current_time: f32) {
        self.is_sheltered = true;
        self.shelter_start_time = current_time;
    }

    /// 停止躲雨（雨停了）
    pub fn stop_sheltering(&mut self) {
        self.target_shelter = None;
        self.is_sheltered = false;
    }

    /// 是否正在尋找庇護（移動中）
    pub fn is_seeking(&self) -> bool {
        self.target_shelter.is_some() && !self.is_sheltered
    }
}
