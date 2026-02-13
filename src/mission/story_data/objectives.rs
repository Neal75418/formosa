//! 任務目標

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// 目標類型
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ObjectiveType {
    /// 消滅特定目標（目標標記 ID）
    KillTarget(String),
    /// 消滅指定數量敵人
    KillCount(u32),
    /// 到達位置（座標, 半徑）
    ReachLocation(Vec3, f32),
    /// 收集物品（物品 ID）
    CollectItem(String),
    /// 護送 NPC（NPC 標記 ID）
    EscortNpc(String),
    /// 追蹤目標（目標 ID, 最大距離）
    FollowTarget(String, f32),
    /// 生存指定時間（秒）
    SurviveTime(f32),
    /// 破壞物件（物件標記 ID）
    DestroyObject(String),
    /// 保持潛行（未被發現）
    StayUndetected,
    /// 與 NPC 對話
    TalkToNpc(String),
    /// 進入車輛
    EnterVehicle(String),
    /// 自定義目標（由程式碼處理）
    Custom(String),
}

/// 任務目標
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionObjective {
    /// 目標 ID
    pub id: u32,
    /// 目標類型
    pub objective_type: ObjectiveType,
    /// 描述文字
    pub description: String,
    /// 目標數量
    pub target_count: u32,
    /// 當前完成數量
    #[serde(default)]
    pub current_count: u32,
    /// 是否為可選目標
    #[serde(default)]
    pub is_optional: bool,
    /// 是否已完成
    #[serde(default)]
    pub is_completed: bool,
}

impl MissionObjective {
    /// 創建新目標
    pub fn new(id: u32, objective_type: ObjectiveType, description: impl Into<String>) -> Self {
        Self {
            id,
            objective_type,
            description: description.into(),
            target_count: 1,
            current_count: 0,
            is_optional: false,
            is_completed: false,
        }
    }

    /// 設置目標數量
    pub fn with_count(mut self, count: u32) -> Self {
        self.target_count = count;
        self
    }

    /// 設為可選目標
    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    /// 增加完成計數
    pub fn increment(&mut self) {
        self.current_count = (self.current_count + 1).min(self.target_count);
        if self.current_count >= self.target_count {
            self.is_completed = true;
        }
    }

    /// 檢查是否完成
    pub fn check_completion(&self) -> bool {
        self.is_completed || self.current_count >= self.target_count
    }

    /// 取得進度百分比
    pub fn progress(&self) -> f32 {
        if self.target_count == 0 {
            return 1.0;
        }
        self.current_count as f32 / self.target_count as f32
    }
}
