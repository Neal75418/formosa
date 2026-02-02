//! 通緝系統事件定義

#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

use bevy::prelude::*;

/// 犯罪事件
#[derive(Message, Clone, Debug)]
pub enum CrimeEvent {
    /// 開槍（位置、目擊者數量）
    Shooting {
        position: Vec3,
        witness_count: u32,
    },
    /// 搶車
    VehicleTheft {
        position: Vec3,
    },
    /// 攻擊行人
    Assault {
        victim: Entity,
        position: Vec3,
    },
    /// 殺死行人
    Murder {
        victim: Entity,
        position: Vec3,
    },
    /// 殺死警察（嚴重犯罪）
    PoliceKilled {
        victim: Entity,
        position: Vec3,
    },
    /// 撞擊行人（用車輛）
    VehicleHit {
        victim: Entity,
        position: Vec3,
        fatal: bool,
    },
}

impl CrimeEvent {
    /// 獲取犯罪的熱度增加量
    pub fn heat_value(&self) -> f32 {
        match self {
            CrimeEvent::Shooting { witness_count, .. } => {
                // 基礎 5 熱度 + 每個目擊者 2 熱度
                5.0 + (*witness_count as f32) * 2.0
            }
            CrimeEvent::VehicleTheft { .. } => 15.0,
            CrimeEvent::Assault { .. } => 10.0,
            CrimeEvent::Murder { .. } => 25.0,
            CrimeEvent::PoliceKilled { .. } => 40.0, // 殺警察是嚴重犯罪
            CrimeEvent::VehicleHit { fatal, .. } => {
                if *fatal { 20.0 } else { 8.0 }
            }
        }
    }

    /// 獲取犯罪位置
    pub fn position(&self) -> Vec3 {
        match self {
            CrimeEvent::Shooting { position, .. } => *position,
            CrimeEvent::VehicleTheft { position } => *position,
            CrimeEvent::Assault { position, .. } => *position,
            CrimeEvent::Murder { position, .. } => *position,
            CrimeEvent::PoliceKilled { position, .. } => *position,
            CrimeEvent::VehicleHit { position, .. } => *position,
        }
    }
}

/// 通緝等級變化事件
#[derive(Message, Clone, Debug)]
pub struct WantedLevelChanged {
    /// 舊等級
    pub old_stars: u8,
    /// 新等級
    pub new_stars: u8,
    /// 是否增加
    pub increased: bool,
}

impl WantedLevelChanged {
    pub fn new(old: u8, new: u8) -> Self {
        Self {
            old_stars: old,
            new_stars: new,
            increased: new > old,
        }
    }
}

/// 目擊者報警完成事件
/// 當行人完成報警電話時發送，用於增加通緝等級
/// 與 CrimeEvent 分開，避免重複計算犯罪
#[derive(Message, Clone, Debug)]
pub struct WitnessReport {
    /// 報警位置
    pub position: Vec3,
    /// 報告的犯罪類型（字串描述）
    pub crime_description: &'static str,
}

impl WitnessReport {
    /// 報警增加的熱度（每次報警增加 5 點）
    pub const HEAT_VALUE: f32 = 5.0;

    pub fn new(position: Vec3, crime_description: &'static str) -> Self {
        Self {
            position,
            crime_description,
        }
    }
}
