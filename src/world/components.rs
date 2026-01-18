//! 世界組件

use bevy::prelude::*;
use rand::Rng;

/// 共用材質快取（減少重複材質創建）
#[derive(Resource, Clone)]
#[allow(dead_code)]  // 逐步遷移中，部分材質暫未使用
pub struct WorldMaterials {
    // 基礎色
    pub white: Handle<StandardMaterial>,
    pub black: Handle<StandardMaterial>,
    pub dark_gray: Handle<StandardMaterial>,
    pub light_gray: Handle<StandardMaterial>,

    // 道路系統
    pub road_asphalt: Handle<StandardMaterial>,
    pub sidewalk: Handle<StandardMaterial>,
    pub zebra_white: Handle<StandardMaterial>,

    // 玻璃材質
    pub glass_dark: Handle<StandardMaterial>,
    pub glass_tinted: Handle<StandardMaterial>,

    // 金屬材質
    pub metal_dark: Handle<StandardMaterial>,
    pub metal_silver: Handle<StandardMaterial>,
}

impl WorldMaterials {
    /// 初始化共用材質
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            // 基礎色
            white: materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.95, 0.95),
                ..default()
            }),
            black: materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.05, 0.05),
                ..default()
            }),
            dark_gray: materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.15, 0.15),
                ..default()
            }),
            light_gray: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.6, 0.6),
                ..default()
            }),

            // 道路系統
            road_asphalt: materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.15, 0.17),
                perceptual_roughness: 0.95,
                ..default()
            }),
            sidewalk: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.48, 0.45),
                perceptual_roughness: 0.9,
                ..default()
            }),
            zebra_white: materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.95, 0.95),
                perceptual_roughness: 0.9,
                ..default()
            }),

            // 玻璃材質
            glass_dark: materials.add(StandardMaterial {
                base_color: Color::srgba(0.1, 0.1, 0.15, 0.7),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            glass_tinted: materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.3, 0.4, 0.6),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),

            // 金屬材質
            metal_dark: materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.2, 0.22),
                metallic: 0.8,
                perceptual_roughness: 0.4,
                ..default()
            }),
            metal_silver: materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.75),
                metallic: 0.9,
                perceptual_roughness: 0.3,
                ..default()
            }),
        }
    }
}

/// 建築物
#[derive(Component)]
pub struct Building {
    pub name: String,
    #[allow(dead_code)] // 保留給未來系統使用（如任務、NPC 行為）
    pub building_type: BuildingType,
}

/// 建築物類型
#[derive(Clone, Debug)]
#[allow(dead_code)] // 保留給未來擴展
pub enum BuildingType {
    Shop,
    ConvenienceStore,
    Restaurant,
    Cinema,
    Office,
    Residential,
    Temple,
    Other,
}

/// 路燈
#[derive(Component)]
pub struct StreetLight {
    pub is_on: bool,
}

/// 街道家具類型
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum StreetFurnitureType {
    Lamppost,       // 路燈柱
    TrashCan,       // 垃圾桶
    VendingMachine, // 自動販賣機
    Bench,          // 長椅
    PhoneBooth,     // 電話亭
    Billboard,      // 廣告看板
}

/// 街道家具組件
#[derive(Component)]
pub struct StreetFurniture {
    #[allow(dead_code)]  // 保留給未來系統使用（如互動、任務）
    pub furniture_type: StreetFurnitureType,
    #[allow(dead_code)]
    pub can_interact: bool,  // 是否可互動（未來功能）
}

/// 霓虹燈招牌
#[derive(Component)]
pub struct NeonSign {
    pub color: Color,
    pub base_intensity: f32,      // 基礎發光強度
    pub flicker_speed: f32,       // 閃爍速度 (0 = 不閃爍)
    pub flicker_amount: f32,      // 閃爍幅度 (0.0 ~ 1.0)
    pub is_broken: bool,          // 是否故障（隨機閃爍）
    pub phase_offset: f32,        // 相位偏移（讓每個招牌閃爍不同步）
}

impl Default for NeonSign {
    fn default() -> Self {
        Self {
            color: Color::srgb(1.0, 0.2, 0.5),  // 預設粉紅色
            base_intensity: 8.0,
            flicker_speed: 0.0,
            flicker_amount: 0.0,
            is_broken: false,
            phase_offset: 0.0,
        }
    }
}

impl NeonSign {
    /// 穩定發光的招牌
    pub fn steady(color: Color, intensity: f32) -> Self {
        Self {
            color,
            base_intensity: intensity,
            flicker_speed: 0.0,
            flicker_amount: 0.0,
            is_broken: false,
            phase_offset: 0.0,
        }
    }

    /// 輕微閃爍的招牌（正常霓虹燈效果）
    pub fn flickering(color: Color, intensity: f32) -> Self {
        let mut rng = rand::rng();
        Self {
            color,
            base_intensity: intensity,
            flicker_speed: 3.0,
            flicker_amount: 0.15,
            is_broken: false,
            phase_offset: rng.random::<f32>() * std::f32::consts::TAU,
        }
    }

    /// 故障閃爍的招牌（更戲劇化的效果）
    pub fn broken(color: Color, intensity: f32) -> Self {
        let mut rng = rand::rng();
        Self {
            color,
            base_intensity: intensity,
            flicker_speed: 8.0,
            flicker_amount: 0.6,
            is_broken: true,
            phase_offset: rng.random::<f32>() * std::f32::consts::TAU,
        }
    }
}

// ============================================================================
// 室內建築系統
// ============================================================================

/// 室內空間定義
/// 定義一個可進入的室內區域
#[derive(Component)]
pub struct InteriorSpace {
    /// 室內空間名稱
    pub name: String,
    /// 內部邊界（本地座標）
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    /// 入口位置（世界座標）
    pub entrance_position: Vec3,
    /// 出口位置（世界座標）
    pub exit_position: Vec3,
    /// 是否為通緝躲藏點
    pub is_hiding_spot: bool,
    /// 躲藏等級（最高可躲避幾星通緝）
    pub max_hide_stars: u8,
    /// 營業時間（24小時制，None 表示全天開放）
    pub open_hours: Option<(f32, f32)>,  // (開門時間, 關門時間)
}

impl Default for InteriorSpace {
    fn default() -> Self {
        Self {
            name: "未命名".to_string(),
            bounds_min: Vec3::new(-5.0, 0.0, -5.0),
            bounds_max: Vec3::new(5.0, 3.0, 5.0),
            entrance_position: Vec3::ZERO,
            exit_position: Vec3::ZERO,
            is_hiding_spot: true,
            max_hide_stars: 2,
            open_hours: None,
        }
    }
}

impl InteriorSpace {
    /// 便利商店
    pub fn convenience_store(name: &str, entrance: Vec3) -> Self {
        Self {
            name: name.to_string(),
            bounds_min: Vec3::new(-4.0, 0.0, -6.0),
            bounds_max: Vec3::new(4.0, 3.0, 6.0),
            entrance_position: entrance,
            exit_position: entrance + Vec3::new(0.0, 0.0, 1.0),
            is_hiding_spot: true,
            max_hide_stars: 1,  // 只能躲 1 星
            open_hours: None,   // 24 小時營業
        }
    }

    /// 餐廳
    pub fn restaurant(name: &str, entrance: Vec3) -> Self {
        Self {
            name: name.to_string(),
            bounds_min: Vec3::new(-6.0, 0.0, -8.0),
            bounds_max: Vec3::new(6.0, 3.5, 8.0),
            entrance_position: entrance,
            exit_position: entrance + Vec3::new(0.0, 0.0, 1.0),
            is_hiding_spot: true,
            max_hide_stars: 2,
            open_hours: Some((10.0, 22.0)),  // 10:00 - 22:00
        }
    }

    /// 安全屋
    pub fn safe_house(name: &str, entrance: Vec3) -> Self {
        Self {
            name: name.to_string(),
            bounds_min: Vec3::new(-8.0, 0.0, -10.0),
            bounds_max: Vec3::new(8.0, 4.0, 10.0),
            entrance_position: entrance,
            exit_position: entrance + Vec3::new(0.0, 0.0, 1.0),
            is_hiding_spot: true,
            max_hide_stars: 5,  // 可以完全躲避
            open_hours: None,
        }
    }

    /// 檢查位置是否在室內
    pub fn contains(&self, local_pos: Vec3) -> bool {
        local_pos.x >= self.bounds_min.x && local_pos.x <= self.bounds_max.x
            && local_pos.y >= self.bounds_min.y && local_pos.y <= self.bounds_max.y
            && local_pos.z >= self.bounds_min.z && local_pos.z <= self.bounds_max.z
    }

    /// 檢查是否在營業時間內
    pub fn is_open(&self, hour: f32) -> bool {
        match self.open_hours {
            None => true,
            Some((open, close)) => {
                if open < close {
                    hour >= open && hour < close
                } else {
                    // 跨午夜營業
                    hour >= open || hour < close
                }
            }
        }
    }
}

/// 門組件
/// 標記可進入/離開的門
#[derive(Component)]
pub struct Door {
    /// 連接的室內空間實體
    pub interior_entity: Option<Entity>,
    /// 互動半徑
    pub interact_radius: f32,
    /// 是否需要鑰匙
    pub requires_key: bool,
    /// 是否已上鎖
    pub is_locked: bool,
    /// 門當前狀態
    pub state: DoorState,
}

/// 門狀態
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum DoorState {
    #[default]
    Closed,
    Opening,
    Open,
    Closing,
}

impl Default for Door {
    fn default() -> Self {
        Self {
            interior_entity: None,
            interact_radius: 2.0,
            requires_key: false,
            is_locked: false,
            state: DoorState::Closed,
        }
    }
}

/// 玩家室內狀態組件
/// 附加到玩家實體，追蹤玩家是否在室內
#[derive(Component, Default)]
pub struct PlayerInteriorState {
    /// 是否在室內
    pub is_inside: bool,
    /// 當前所在室內空間
    pub current_interior: Option<Entity>,
    /// 進入時間
    pub entered_time: f32,
}

/// 室內提示 UI 標記
#[derive(Component)]
pub struct InteriorPrompt;

/// 建築窗戶（隨日夜變化）
#[derive(Component)]
pub struct BuildingWindow {
    pub is_lit: bool,              // 是否點亮
    pub light_probability: f32,    // 點亮機率 (0.0 ~ 1.0)
    pub base_color: Color,         // 窗戶基礎顏色
    pub lit_emissive: f32,         // 點亮時發光強度
    pub is_shop: bool,             // 是否為商店（深夜會關燈）
}

impl Default for BuildingWindow {
    fn default() -> Self {
        Self {
            is_lit: false,
            light_probability: 0.5,
            base_color: Color::srgb(1.0, 0.95, 0.7),  // 暖黃色
            lit_emissive: 3.0,
            is_shop: false,
        }
    }
}

#[allow(dead_code)]  // 保留給未來不同建築類型使用
impl BuildingWindow {
    /// 住宅窗戶
    pub fn residential() -> Self {
        let mut rng = rand::rng();
        Self {
            is_lit: false,
            light_probability: rng.random_range(0.3..0.6),
            base_color: Color::srgb(1.0, 0.95, 0.7),
            lit_emissive: 2.5,
            is_shop: false,
        }
    }

    /// 商店窗戶（更亮、深夜關閉）
    pub fn shop() -> Self {
        let mut rng = rand::rng();
        Self {
            is_lit: false,
            light_probability: rng.random_range(0.6..0.9),
            base_color: Color::srgb(1.0, 1.0, 0.9),
            lit_emissive: 4.0,
            is_shop: true,
        }
    }

    /// 辦公室窗戶（白色冷光）
    pub fn office() -> Self {
        let mut rng = rand::rng();
        Self {
            is_lit: false,
            light_probability: rng.random_range(0.2..0.5),
            base_color: Color::srgb(0.9, 0.95, 1.0),
            lit_emissive: 3.5,
            is_shop: false,
        }
    }
}
