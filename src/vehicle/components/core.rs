//! 車輛核心類型、ID 和材質

use bevy::prelude::*;
use bevy::pbr::StandardMaterial;
use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// 車輛穩定 ID（用於存檔識別）
// ============================================================================

/// 全局車輛 ID 計數器
static VEHICLE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 車輛穩定識別碼組件
///
/// 用於存檔/讀檔時識別特定車輛，不依賴 Query 枚舉順序
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VehicleId(pub u64);

impl VehicleId {
    /// 生成新的唯一 ID
    pub fn new() -> Self {
        Self(VEHICLE_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// 從已知 ID 創建（用於讀檔）
    pub fn from_raw(id: u64) -> Self {
        // 確保計數器超過這個 ID，避免將來衝突
        loop {
            let current = VEHICLE_ID_COUNTER.load(Ordering::SeqCst);
            if current > id {
                break;
            }
            if VEHICLE_ID_COUNTER.compare_exchange(current, id + 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                break;
            }
        }
        Self(id)
    }

    /// 取得原始 ID 值
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for VehicleId {
    fn default() -> Self {
        Self::new()
    }
}

/// 共享載具材質（效能優化：避免重複創建相同材質）
/// 每種通用材質只創建一次，所有載具共用
#[derive(Resource, Clone)]
pub struct VehicleMaterials {
    /// 黑色塑膠（座墊、踏板）
    pub black_plastic: Handle<StandardMaterial>,
    /// 輪胎
    pub wheel: Handle<StandardMaterial>,
    /// 頭燈（發光）
    pub headlight: Handle<StandardMaterial>,
    /// 尾燈（紅色發光）
    pub taillight: Handle<StandardMaterial>,
    /// 後照鏡（金屬）
    pub mirror: Handle<StandardMaterial>,
    /// 車窗玻璃（深色）
    pub glass: Handle<StandardMaterial>,
}

impl VehicleMaterials {
    /// 初始化共享材質（在 setup_world 中調用一次）
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            black_plastic: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.1),
                perceptual_roughness: 0.8,
                ..default()
            }),
            wheel: materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.05, 0.05),
                perceptual_roughness: 0.9,
                ..default()
            }),
            headlight: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(15.0, 14.0, 10.0, 1.0),
                ..default()
            }),
            taillight: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: LinearRgba::new(10.0, 0.0, 0.0, 1.0),
                ..default()
            }),
            mirror: materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.8),
                metallic: 0.9,
                perceptual_roughness: 0.1,
                ..default()
            }),
            glass: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.1),
                perceptual_roughness: 0.1,
                metallic: 0.8,
                ..default()
            }),
        }
    }
}

/// 載具類型
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VehicleType {
    Scooter,    // 機車
    Car,        // 汽車
    Taxi,       // 計程車
    Bus,        // 公車
}

impl VehicleType {
    /// 取得穩定的存檔鍵值（不受 enum 重命名影響）
    pub fn save_key(&self) -> &'static str {
        match self {
            VehicleType::Scooter => "Scooter",
            VehicleType::Car => "Car",
            VehicleType::Taxi => "Taxi",
            VehicleType::Bus => "Bus",
        }
    }
}

/// 載具物理模式
/// - Dynamic：玩家駕駛，使用 Velocity/Impulse/Torque 驅動
/// - Kinematic：NPC/停放車輛，以 Transform 控制
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum VehiclePhysicsMode {
    Dynamic,
    Kinematic,
}

/// 載具核心組件（僅包含基本屬性，子系統拆分為獨立元件）
#[derive(Component)]
pub struct Vehicle {
    pub vehicle_type: VehicleType,
    pub max_speed: f32,
    pub acceleration: f32,
    pub turn_speed: f32,
    pub current_speed: f32,
    pub is_occupied: bool,
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            vehicle_type: VehicleType::Car,
            max_speed: 30.0,
            acceleration: 10.0,
            turn_speed: 2.0,
            current_speed: 0.0,
            is_occupied: false,
        }
    }
}
