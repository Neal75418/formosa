//! 玩家組件

#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

use bevy::prelude::*;

/// 玩家標記與移動參數
/// 注意：生命值使用獨立的 Health 組件，不在此結構中
#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub rotation_speed: f32,
    pub sprint_speed: f32,
    pub is_sprinting: bool,
    pub jump_force: f32,
    pub vertical_velocity: f32,
    pub is_grounded: bool,
    pub money: u32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            speed: 10.0,           // 提高基礎速度（原 8.0）
            rotation_speed: 3.0,
            sprint_speed: 18.0,    // 提高衝刺速度（原 15.0）
            is_sprinting: false,
            jump_force: 12.0,
            vertical_velocity: 0.0,
            is_grounded: true,
            money: 5000,
        }
    }
}

/// 閃避狀態
#[derive(Component, Default)]
pub struct DodgeState {
    /// 是否正在閃避中
    pub is_dodging: bool,
    /// 閃避方向（世界座標）
    pub dodge_direction: Vec3,
    /// 閃避剩餘時間
    pub dodge_timer: f32,
    /// 閃避冷卻時間
    pub cooldown: f32,
}

impl DodgeState {
    /// 閃避持續時間（秒）
    pub const DODGE_DURATION: f32 = 0.25;
    /// 閃避距離（公尺）
    pub const DODGE_DISTANCE: f32 = 5.0;
    /// 閃避冷卻時間（秒）
    pub const DODGE_COOLDOWN: f32 = 0.5;
    /// 閃避期間無敵
    pub const DODGE_INVINCIBLE: bool = true;

    /// 開始閃避
    pub fn start_dodge(&mut self, direction: Vec3) {
        if self.cooldown <= 0.0 && !self.is_dodging {
            self.is_dodging = true;
            self.dodge_direction = direction.normalize();
            self.dodge_timer = Self::DODGE_DURATION;
            self.cooldown = Self::DODGE_COOLDOWN;
        }
    }

    /// 更新閃避狀態
    pub fn update(&mut self, dt: f32) {
        if self.cooldown > 0.0 {
            self.cooldown -= dt;
        }
        if self.is_dodging {
            self.dodge_timer -= dt;
            if self.dodge_timer <= 0.0 {
                self.is_dodging = false;
            }
        }
    }

    /// 取得閃避移動速度
    pub fn get_dodge_velocity(&self) -> Vec3 {
        if self.is_dodging {
            self.dodge_direction * (Self::DODGE_DISTANCE / Self::DODGE_DURATION)
        } else {
            Vec3::ZERO
        }
    }
}

/// 雙擊偵測器（追蹤每個方向鍵的按壓時間）
#[derive(Resource, Default)]
pub struct DoubleTapTracker {
    /// 上次按下 W 的時間
    pub last_w: f32,
    /// 上次按下 S 的時間
    pub last_s: f32,
    /// 上次按下 A 的時間
    pub last_a: f32,
    /// 上次按下 D 的時間
    pub last_d: f32,
    /// 累計遊戲時間
    pub time: f32,
}

impl DoubleTapTracker {
    /// 雙擊判定時間窗口（秒）
    pub const DOUBLE_TAP_WINDOW: f32 = 0.25;

    /// 檢查是否雙擊（返回 true 表示觸發閃避）
    pub fn check_double_tap(&mut self, key: KeyCode, current_time: f32) -> bool {
        let last_time = match key {
            KeyCode::KeyW => &mut self.last_w,
            KeyCode::KeyS => &mut self.last_s,
            KeyCode::KeyA => &mut self.last_a,
            KeyCode::KeyD => &mut self.last_d,
            _ => return false,
        };

        let is_double_tap = current_time - *last_time < Self::DOUBLE_TAP_WINDOW;
        *last_time = current_time;
        is_double_tap
    }
}

// ============================================================================
// 車輛進出動畫系統 (GTA 5 風格)
// ============================================================================

/// 車輛進出動畫階段
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum VehicleTransitionPhase {
    #[default]
    None,
    /// 走向車輛（上車）
    WalkingToVehicle,
    /// 開門
    OpeningDoor,
    /// 進入車輛
    EnteringVehicle,
    /// 關門
    ClosingDoor,
    /// 開門（下車）
    OpeningDoorExit,
    /// 離開車輛
    ExitingVehicle,
    /// 關門（下車）
    ClosingDoorExit,
    /// 走離車輛
    WalkingAway,
}

/// 車輛進出動畫狀態
#[derive(Resource, Default)]
pub struct VehicleTransitionState {
    /// 當前動畫階段
    pub phase: VehicleTransitionPhase,
    /// 階段進度 (0.0 ~ 1.0)
    pub progress: f32,
    /// 目標車輛
    pub target_vehicle: Option<Entity>,
    /// 動畫起始位置
    pub start_position: Vec3,
    /// 動畫目標位置（門旁/座位/下車點）
    pub target_position: Vec3,
    /// 門的位置（相對於車輛）
    pub door_offset: Vec3,
    /// 是否從右側上車
    pub from_right_side: bool,
    /// 當前門開角度 (0.0 ~ 1.0)
    pub door_angle: f32,
}

impl VehicleTransitionState {
    // === 動畫時間常數 ===
    /// 走向車輛的時間（秒）
    pub const WALK_TO_VEHICLE_DURATION: f32 = 0.4;
    /// 開門時間（秒）
    pub const OPEN_DOOR_DURATION: f32 = 0.25;
    /// 進入車輛時間（秒）
    pub const ENTER_DURATION: f32 = 0.35;
    /// 關門時間（秒）
    pub const CLOSE_DOOR_DURATION: f32 = 0.2;
    /// 離開車輛時間（秒）
    pub const EXIT_DURATION: f32 = 0.3;
    /// 走離車輛時間（秒）
    pub const WALK_AWAY_DURATION: f32 = 0.3;

    /// 是否正在進行動畫
    pub fn is_animating(&self) -> bool {
        self.phase != VehicleTransitionPhase::None
    }

    /// 開始上車動畫
    pub fn start_enter(&mut self, player_pos: Vec3, vehicle: Entity, door_pos: Vec3, from_right: bool) {
        self.phase = VehicleTransitionPhase::WalkingToVehicle;
        self.progress = 0.0;
        self.target_vehicle = Some(vehicle);
        self.start_position = player_pos;
        self.target_position = door_pos;
        self.from_right_side = from_right;
        self.door_angle = 0.0;
    }

    /// 開始下車動畫
    pub fn start_exit(&mut self, seat_pos: Vec3, vehicle: Entity, exit_pos: Vec3, from_right: bool) {
        self.phase = VehicleTransitionPhase::OpeningDoorExit;
        self.progress = 0.0;
        self.target_vehicle = Some(vehicle);
        self.start_position = seat_pos;
        self.target_position = exit_pos;
        self.from_right_side = from_right;
        self.door_angle = 0.0;
    }

    /// 取得當前階段的持續時間
    pub fn current_phase_duration(&self) -> f32 {
        match self.phase {
            VehicleTransitionPhase::None => 0.0,
            VehicleTransitionPhase::WalkingToVehicle => Self::WALK_TO_VEHICLE_DURATION,
            VehicleTransitionPhase::OpeningDoor => Self::OPEN_DOOR_DURATION,
            VehicleTransitionPhase::EnteringVehicle => Self::ENTER_DURATION,
            VehicleTransitionPhase::ClosingDoor => Self::CLOSE_DOOR_DURATION,
            VehicleTransitionPhase::OpeningDoorExit => Self::OPEN_DOOR_DURATION,
            VehicleTransitionPhase::ExitingVehicle => Self::EXIT_DURATION,
            VehicleTransitionPhase::ClosingDoorExit => Self::CLOSE_DOOR_DURATION,
            VehicleTransitionPhase::WalkingAway => Self::WALK_AWAY_DURATION,
        }
    }

    /// 更新動畫進度，返回是否需要切換到下一階段
    pub fn update(&mut self, dt: f32) -> bool {
        if self.phase == VehicleTransitionPhase::None {
            return false;
        }

        let duration = self.current_phase_duration();
        if duration > 0.0 {
            self.progress += dt / duration;
        }

        // 更新門角度
        match self.phase {
            VehicleTransitionPhase::OpeningDoor | VehicleTransitionPhase::OpeningDoorExit => {
                self.door_angle = ease_out_cubic(self.progress.min(1.0));
            }
            VehicleTransitionPhase::ClosingDoor | VehicleTransitionPhase::ClosingDoorExit => {
                self.door_angle = 1.0 - ease_in_cubic(self.progress.min(1.0));
            }
            _ => {}
        }

        self.progress >= 1.0
    }

    /// 切換到下一階段
    pub fn advance_phase(&mut self) {
        self.progress = 0.0;
        self.phase = match self.phase {
            VehicleTransitionPhase::WalkingToVehicle => VehicleTransitionPhase::OpeningDoor,
            VehicleTransitionPhase::OpeningDoor => VehicleTransitionPhase::EnteringVehicle,
            VehicleTransitionPhase::EnteringVehicle => VehicleTransitionPhase::ClosingDoor,
            VehicleTransitionPhase::ClosingDoor => VehicleTransitionPhase::None,
            VehicleTransitionPhase::OpeningDoorExit => VehicleTransitionPhase::ExitingVehicle,
            VehicleTransitionPhase::ExitingVehicle => VehicleTransitionPhase::ClosingDoorExit,
            VehicleTransitionPhase::ClosingDoorExit => VehicleTransitionPhase::WalkingAway,
            VehicleTransitionPhase::WalkingAway => VehicleTransitionPhase::None,
            VehicleTransitionPhase::None => VehicleTransitionPhase::None,
        };
    }

    /// 重置狀態
    pub fn reset(&mut self) {
        self.phase = VehicleTransitionPhase::None;
        self.progress = 0.0;
        self.target_vehicle = None;
        self.door_angle = 0.0;
    }
}

/// 緩出曲線（用於門打開）
fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// 緩入曲線（用於門關閉）
fn ease_in_cubic(t: f32) -> f32 {
    t.powi(3)
}

/// 車門組件標記
#[derive(Component)]
pub struct VehicleDoor {
    /// 所屬車輛
    pub vehicle_entity: Entity,
    /// 是否為右側門
    pub is_right_side: bool,
    /// 門軸心點（相對於車輛）
    pub hinge_offset: Vec3,
    /// 最大開門角度（弧度）
    pub max_angle: f32,
}
