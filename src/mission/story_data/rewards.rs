//! 任務獎勵

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use crate::combat::WeaponType;
use crate::vehicle::VehicleType;
use super::{AreaId, StoryMissionId};

/// 難度等級
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Difficulty {
    /// 簡單
    Easy,
    /// 普通
    #[default]
    Normal,
    /// 困難
    Hard,
    /// 極難
    Extreme,
}

/// 任務獎勵
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MissionRewards {
    /// 金錢獎勵
    pub money: u32,
    /// 聲望點數
    #[serde(default)]
    pub respect: u32,
    /// 解鎖武器
    #[serde(default)]
    pub unlock_weapons: Vec<WeaponType>,
    /// 解鎖車輛
    #[serde(default)]
    pub unlock_vehicles: Vec<VehicleType>,
    /// 解鎖區域
    #[serde(default)]
    pub unlock_areas: Vec<AreaId>,
    /// 解鎖任務
    #[serde(default)]
    pub unlock_missions: Vec<StoryMissionId>,
    /// 設置劇情標記
    #[serde(default)]
    pub set_flags: Vec<String>,
}

impl MissionRewards {
    /// 創建金錢獎勵
    pub fn money(amount: u32) -> Self {
        Self {
            money: amount,
            ..Default::default()
        }
    }

    /// 添加聲望
    pub fn with_respect(mut self, respect: u32) -> Self {
        self.respect = respect;
        self
    }

    /// 解鎖下一任務
    pub fn unlock_mission(mut self, mission_id: StoryMissionId) -> Self {
        self.unlock_missions.push(mission_id);
        self
    }

    /// 解鎖武器
    pub fn unlock_weapon(mut self, weapon: WeaponType) -> Self {
        self.unlock_weapons.push(weapon);
        self
    }

    /// 解鎖車輛
    pub fn unlock_vehicle(mut self, vehicle: VehicleType) -> Self {
        self.unlock_vehicles.push(vehicle);
        self
    }

    /// 設置劇情旗標
    pub fn set_flag(mut self, flag: String) -> Self {
        self.set_flags.push(flag);
        self
    }
}
