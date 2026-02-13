//! 行人基本組件和狀態

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

/// 行人標記組件
#[derive(Component, Debug)]
pub struct Pedestrian {
    /// 行人類型
    pub ped_type: PedestrianType,
}

impl Default for Pedestrian {
    fn default() -> Self {
        Self {
            ped_type: PedestrianType::Casual,
        }
    }
}

/// 行人類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PedestrianType {
    #[default]
    Casual,     // 一般路人
    Business,   // 上班族
    Student,    // 學生
    Tourist,    // 觀光客
}

/// 行人狀態組件
#[derive(Component, Debug)]
pub struct PedestrianState {
    /// 當前狀態
    pub state: PedState,
    /// 恐懼程度 (0.0-1.0)
    pub fear_level: f32,
    /// 逃跑持續時間
    pub flee_timer: f32,
    /// 最後威脅位置
    pub last_threat_pos: Option<Vec3>,
    /// 卡住計時器（用於檢測行人是否卡在障礙物）
    pub stuck_timer: f32,
    /// 上一次記錄的位置（用於卡住檢測）
    pub last_recorded_pos: Vec3,
}

impl Default for PedestrianState {
    fn default() -> Self {
        Self {
            state: PedState::Walking,
            fear_level: 0.0,
            flee_timer: 0.0,
            last_threat_pos: None,
            stuck_timer: 0.0,
            last_recorded_pos: Vec3::ZERO,
        }
    }
}

/// 行人行為狀態
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PedState {
    Idle,           // 站著（等紅燈、看手機）
    #[default]
    Walking,        // 正常行走
    Fleeing,        // 逃跑中
    CallingPolice,  // 報警中（掏出手機打電話）
}
