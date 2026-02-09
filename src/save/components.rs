//! 存檔系統組件
//!
//! 包含存檔資料結構和事件定義
#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::combat::WeaponType;
use crate::core::WeatherType;
use crate::mission::{StoryMissionRating, StoryMissionStatus};

// ============================================================================
// 存檔資料結構
// ============================================================================

/// 完整存檔資料
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveData {
    /// 存檔版本（用於相容性檢查）
    pub version: u32,
    /// 存檔時間戳
    pub timestamp: u64,
    /// 遊戲時間（總遊玩秒數）
    pub play_time_secs: f64,
    /// 玩家資料
    pub player: PlayerSaveData,
    /// 世界資料
    pub world: WorldSaveData,
    /// 任務資料
    pub missions: MissionSaveData,
    /// 統計資料
    pub stats: GameStatistics,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            timestamp: 0,
            play_time_secs: 0.0,
            player: PlayerSaveData::default(),
            world: WorldSaveData::default(),
            missions: MissionSaveData::default(),
            stats: GameStatistics::default(),
        }
    }
}

/// 當前存檔版本
pub const SAVE_VERSION: u32 = 2;

/// 玩家存檔資料
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlayerSaveData {
    /// 位置
    pub position: [f32; 3],
    /// 旋轉（Yaw）
    pub rotation_y: f32,
    /// 生命值
    pub health: f32,
    /// 最大生命值
    pub max_health: f32,
    /// 護甲值
    pub armor: f32,
    /// 現金
    pub cash: i32,
    /// 聲望
    pub respect: i32,
    /// 銀行存款
    pub bank: i32,
    /// 武器庫存
    pub weapons: Vec<WeaponSaveData>,
    /// 當前裝備武器索引
    pub current_weapon_index: usize,
    /// 是否在車內
    pub in_vehicle: bool,
    /// 當前車輛 ID（如果在車內）
    pub current_vehicle_id: Option<u64>,
}

/// 武器存檔資料
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WeaponSaveData {
    /// 武器類型
    pub weapon_type: WeaponType,
    /// 當前彈匣彈藥
    pub current_ammo: u32,
    /// 備用彈藥
    pub reserve_ammo: u32,
}

/// 世界存檔資料
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorldSaveData {
    /// 當前遊戲內時間（小時）
    pub world_hour: f32,
    /// 天氣類型
    pub weather: WeatherType,
    /// 天氣強度
    pub weather_intensity: f32,
    /// 已解鎖的安全屋
    pub unlocked_safehouses: Vec<String>,
    /// 已購買的車輛 ID 列表
    pub owned_vehicles: Vec<u64>,
    /// 車輛改裝資料
    #[serde(default)]
    pub vehicle_modifications: Vec<VehicleModificationSaveData>,
}

impl Default for WorldSaveData {
    fn default() -> Self {
        Self {
            world_hour: 8.0,
            weather: WeatherType::Clear,
            weather_intensity: 1.0,
            unlocked_safehouses: vec!["safehouse_ximending".to_string()],
            owned_vehicles: Vec::new(),
            vehicle_modifications: Vec::new(),
        }
    }
}

/// 車輛改裝存檔資料
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct VehicleModificationSaveData {
    /// 車輛穩定 ID（新版存檔使用）
    #[serde(default)]
    pub vehicle_id: u64,
    /// 車輛實體索引（舊版相容，已棄用）
    #[serde(default)]
    #[deprecated(note = "使用 vehicle_id 代替")]
    pub vehicle_index: u32,
    /// 引擎等級 (0=Stock, 1=Level1, 2=Level2, 3=Level3)
    pub engine_level: u8,
    /// 變速箱等級
    pub transmission_level: u8,
    /// 懸吊等級
    pub suspension_level: u8,
    /// 煞車等級
    pub brakes_level: u8,
    /// 輪胎等級
    pub tires_level: u8,
    /// 裝甲等級
    pub armor_level: u8,
    /// 是否有氮氣
    pub has_nitro: bool,
    /// 氮氣充能量
    pub nitro_charge: f32,
}

/// 任務存檔資料
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MissionSaveData {
    /// 已完成的任務 ID 列表
    pub completed_missions: Vec<String>,
    /// 當前進行中的任務 ID
    pub active_mission: Option<String>,
    /// 任務進度（任務 ID -> 檢查點索引）
    pub mission_progress: Vec<(String, usize)>,
    /// 任務評分（任務 ID -> 星級）— v1 格式，保留向後相容
    pub mission_ratings: Vec<(String, u8)>,
    /// 已解鎖的任務 ID
    pub unlocked_missions: Vec<String>,
    /// 已解鎖的物品 ID
    pub unlocked_items: Vec<String>,
    /// 已解鎖的區域 ID
    pub unlocked_areas: Vec<String>,
    /// NPC 好感度（NPC ID -> 好感度）
    pub npc_relationships: Vec<(String, i32)>,
    /// 劇情旗標（Flag Name -> Value）
    pub flags: Vec<(String, bool)>,
    /// 任務狀態對照表（v2+，完整 round-trip）
    #[serde(default)]
    pub mission_states: HashMap<u32, StoryMissionStatus>,
    /// 當前章節（v2+）
    #[serde(default)]
    pub current_chapter: u32,
    /// 各任務最佳評分（v2+）
    #[serde(default)]
    pub best_ratings: HashMap<u32, StoryMissionRating>,
}

/// 遊戲統計資料
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GameStatistics {
    /// 總殺敵數
    pub enemies_killed: u32,
    /// 爆頭數
    pub headshots: u32,
    /// 總行駛距離（公尺）
    pub distance_driven: f32,
    /// 總步行距離（公尺）
    pub distance_walked: f32,
    /// 完成任務數
    pub missions_completed: u32,
    /// 任務失敗次數
    pub missions_failed: u32,
    /// 累計獲得金錢
    pub total_money_earned: i32,
    /// 累計花費金錢
    pub total_money_spent: i32,
    /// 最高通緝等級達成
    pub max_wanted_level_reached: u8,
    /// 警察擊殺數
    pub police_killed: u32,
    /// 車輛摧毀數
    pub vehicles_destroyed: u32,
}

// ============================================================================
// 存檔管理器
// ============================================================================

/// 存檔管理器資源
#[derive(Resource)]
pub struct SaveManager {
    /// 存檔目錄路徑
    pub save_directory: PathBuf,
    /// 最大存檔槽數
    pub max_slots: usize,
    /// 當前存檔槽索引
    pub current_slot: usize,
    /// 是否啟用自動存檔
    pub auto_save_enabled: bool,
    /// 自動存檔間隔（秒）
    pub auto_save_interval: f32,
    /// 自上次自動存檔的時間
    pub time_since_auto_save: f32,
    /// 存檔槽資訊快取
    pub slot_info: Vec<Option<SaveSlotInfo>>,
    /// 是否正在存檔/讀檔
    pub is_busy: bool,
}

impl Default for SaveManager {
    fn default() -> Self {
        let save_directory = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("IslandRampage")
            .join("saves");

        Self {
            save_directory,
            max_slots: 10,
            current_slot: 0,
            auto_save_enabled: true,
            auto_save_interval: 300.0, // 5 分鐘
            time_since_auto_save: 0.0,
            slot_info: vec![None; 10],
            is_busy: false,
        }
    }
}

impl SaveManager {
    /// 取得存檔檔案路徑
    pub fn get_save_path(&self, slot: usize) -> PathBuf {
        self.save_directory.join(format!("save_{:02}.json", slot))
    }

    /// 取得自動存檔路徑
    pub fn get_auto_save_path(&self) -> PathBuf {
        self.save_directory.join("autosave.json")
    }

    /// 取得快速存檔路徑
    pub fn get_quick_save_path(&self) -> PathBuf {
        self.save_directory.join("quicksave.json")
    }

    /// 確保存檔目錄存在
    pub fn ensure_directory(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.save_directory)
    }
}

/// 存檔槽資訊（用於顯示存檔列表）
#[derive(Clone, Debug)]
pub struct SaveSlotInfo {
    /// 存檔時間戳
    pub timestamp: u64,
    /// 遊玩時間
    pub play_time_secs: f64,
    /// 玩家位置描述
    pub location: String,
    /// 完成度百分比
    pub completion_percent: f32,
    /// 現金
    pub cash: i32,
}

// ============================================================================
// 事件
// ============================================================================

/// 存檔事件
#[derive(Message)]
pub struct SaveGameEvent {
    /// 存檔類型
    pub save_type: SaveType,
    /// 目標存檔槽（僅對 Slot 類型有效）
    pub slot: Option<usize>,
}

/// 讀檔事件
#[derive(Message)]
pub struct LoadGameEvent {
    /// 讀檔類型
    pub load_type: LoadType,
    /// 來源存檔槽（僅對 Slot 類型有效）
    pub slot: Option<usize>,
}

/// 自動存檔觸發事件
#[derive(Message)]
pub struct AutoSaveEvent {
    /// 觸發原因
    pub reason: AutoSaveReason,
}

/// 存檔類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveType {
    /// 存檔到指定槽
    Slot,
    /// 快速存檔
    QuickSave,
    /// 自動存檔
    AutoSave,
}

/// 讀檔類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadType {
    /// 從指定槽讀取
    Slot,
    /// 快速讀檔
    QuickLoad,
    /// 自動存檔讀取
    AutoSave,
}

/// 自動存檔觸發原因
#[derive(Clone, Copy, Debug)]
pub enum AutoSaveReason {
    /// 任務完成
    MissionComplete,
    /// 進入安全屋
    EnteredSafehouse,
    /// 定時自動存檔
    Timer,
    /// 購買重要物品
    ImportantPurchase,
}

// ============================================================================
// 安全屋標記
// ============================================================================

/// 安全屋組件（進入時觸發自動存檔）
#[derive(Component)]
pub struct Safehouse {
    /// 安全屋 ID
    pub id: String,
    /// 名稱
    pub name: String,
    /// 是否已解鎖
    pub is_unlocked: bool,
    /// 存檔點位置
    pub save_point: Vec3,
}
