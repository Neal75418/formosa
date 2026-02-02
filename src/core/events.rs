//! 遊戲事件

#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

use bevy::prelude::*;

/// 遊戲事件（預留多人連線）
#[derive(Message, Clone, Debug)]
pub enum GameEvent {
    PlayerMove { direction: Vec3 },
    PlayerSprint { active: bool },
    VehicleAccelerate { vehicle: Entity, throttle: f32 },
    VehicleTurn { vehicle: Entity, direction: f32 },
    VehicleBrake { vehicle: Entity },
    EnterVehicle { player: Entity, vehicle: Entity },
    ExitVehicle { player: Entity },
}

/// 事件處理（預留）
pub fn handle_game_events(_events: MessageReader<GameEvent>) {
    // 未來：處理從伺服器來的事件
}
