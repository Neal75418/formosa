//! NPC 車輛相關組件

use bevy::prelude::*;
use std::sync::Arc;

/// NPC 車輛行為狀態
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NpcState {
    Cruising,       // 巡航（直走）
    Braking,        // 煞車（前方有障礙）
    Stopped,        // 停止
    Reversing,      // 倒車（太近或卡住）
    WaitingAtLight, // 等紅燈
}

/// NPC 車輛標記組件
#[derive(Component)]
pub struct NpcVehicle {
    pub state: NpcState,
    pub check_timer: Timer,
    pub waypoints: Arc<Vec<Vec3>>, // 預定行駛路線（Arc 共享，避免每次 spawn 複製）
    pub current_wp_index: usize,   // 當前目標點索引
    pub stuck_timer: f32,          // 卡住計時器
}

impl Default for NpcVehicle {
    fn default() -> Self {
        Self {
            state: NpcState::Cruising,
            check_timer: Timer::from_seconds(0.2, TimerMode::Repeating),
            waypoints: Arc::new(vec![]),
            current_wp_index: 0,
            stuck_timer: 0.0,
        }
    }
}
