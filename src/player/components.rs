//! 玩家組件

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]


use bevy::prelude::*;
use crate::core::{ease_out_cubic, ease_in_cubic};

/// 玩家標記與移動參數
/// 注意：生命值使用獨立的 Health 組件，不在此結構中
#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub rotation_speed: f32,
    pub sprint_speed: f32,
    pub crouch_speed: f32,
    pub is_sprinting: bool,
    pub is_crouching: bool,
    pub jump_force: f32,
    pub vertical_velocity: f32,
    pub is_grounded: bool,
    // === 加速度系統 ===
    /// 當前實際移動速度（平滑過渡）
    pub current_speed: f32,
    /// 加速時間（從靜止到最大速度）
    pub acceleration_time: f32,
    /// 減速時間（從最大速度到靜止）
    pub deceleration_time: f32,
    /// 上次移動方向（用於慣性滑行）
    pub last_movement_direction: Vec3,
}

impl Default for Player {
    fn default() -> Self {
        let acceleration_time = 0.3;  // 0.3 秒從靜止到全速
        let deceleration_time = 0.4;  // 0.4 秒從全速到靜止
        debug_assert!(acceleration_time > 0.0, "acceleration_time 必須 > 0，否則除零");
        debug_assert!(deceleration_time > 0.0, "deceleration_time 必須 > 0，否則除零");

        Self {
            speed: 10.0,           // 基礎走路速度
            rotation_speed: 3.0,
            sprint_speed: 18.0,    // 衝刺速度
            crouch_speed: 4.0,     // 蹲伏速度
            is_sprinting: false,
            is_crouching: false,
            jump_force: 12.0,
            vertical_velocity: 0.0,
            is_grounded: true,
            // 加速度系統
            current_speed: 0.0,           // 初始靜止
            acceleration_time,
            deceleration_time,
            last_movement_direction: Vec3::Z, // 初始面向 +Z
        }
    }
}

// ============================================================================
// 衝刺狀態機
// ============================================================================

/// 衝刺狀態（用於動畫和效果系統）
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SprintState {
    /// 靜止或慢走
    #[default]
    Idle,
    /// 正在加速到衝刺 (progress: 0.0 → 1.0)
    Accelerating { progress: f32 },
    /// 全速衝刺中
    Sprinting,
    /// 正在從衝刺減速 (progress: 1.0 → 0.0)
    Decelerating { progress: f32 },
}

impl SprintState {
    /// 衝刺門檻（超過此速度比例視為進入衝刺）
    const SPRINT_THRESHOLD: f32 = 0.85;
    /// 走路門檻（低於此速度比例視為走路）
    const WALK_THRESHOLD: f32 = 0.6;

    /// 根據當前速度和目標速度更新狀態
    pub fn update(&mut self, current_speed: f32, walk_speed: f32, sprint_speed: f32, dt: f32) {
        let speed_ratio = current_speed / sprint_speed;

        match *self {
            SprintState::Idle => {
                // 開始加速
                if speed_ratio > Self::WALK_THRESHOLD {
                    *self = SprintState::Accelerating { progress: speed_ratio / Self::SPRINT_THRESHOLD };
                }
            }
            SprintState::Accelerating { progress } => {
                let new_progress = (current_speed - walk_speed) / (sprint_speed * Self::SPRINT_THRESHOLD - walk_speed);
                let clamped = new_progress.clamp(0.0, 1.0);
                if clamped >= 1.0 {
                    *self = SprintState::Sprinting;
                } else if clamped <= 0.0 {
                    *self = SprintState::Idle;
                } else {
                    *self = SprintState::Accelerating { progress: clamped };
                }
                let _ = (progress, dt); // 避免 unused warning
            }
            SprintState::Sprinting => {
                // 開始減速
                if speed_ratio < Self::SPRINT_THRESHOLD {
                    *self = SprintState::Decelerating { progress: speed_ratio / Self::SPRINT_THRESHOLD };
                }
            }
            SprintState::Decelerating { progress } => {
                let new_progress = speed_ratio / Self::SPRINT_THRESHOLD;
                if new_progress >= Self::SPRINT_THRESHOLD {
                    *self = SprintState::Sprinting;
                } else if new_progress <= Self::WALK_THRESHOLD / Self::SPRINT_THRESHOLD {
                    *self = SprintState::Idle;
                } else {
                    *self = SprintState::Decelerating { progress: new_progress };
                }
                let _ = (progress, dt); // 避免 unused warning
            }
        }
    }

    /// 是否處於衝刺相關狀態（用於動畫選擇）
    pub fn is_sprint_related(&self) -> bool {
        !matches!(self, SprintState::Idle)
    }

    /// 獲取動畫混合權重 (0.0 = 走路, 1.0 = 衝刺)
    pub fn animation_blend(&self) -> f32 {
        match *self {
            SprintState::Idle => 0.0,
            SprintState::Accelerating { progress } => progress,
            SprintState::Sprinting => 1.0,
            SprintState::Decelerating { progress } => progress,
        }
    }
}

/// 衝刺狀態組件（附加到玩家實體）
#[derive(Component, Default)]
pub struct PlayerSprintState {
    pub state: SprintState,
}

// ============================================================================
// 體力系統
// ============================================================================

/// 體力組件（控制衝刺能力）
#[derive(Component)]
pub struct Stamina {
    pub current: f32,
    pub max: f32,
    pub drain_rate: f32,   // 衝刺時消耗速度（每秒）
    pub regen_rate: f32,   // 非衝刺時恢復速度（每秒）
    pub exhausted: bool,   // 是否耗盡（需恢復到門檻才能再衝刺）
}

impl Default for Stamina {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
            drain_rate: 15.0,
            regen_rate: 10.0,
            exhausted: false,
        }
    }
}

impl Stamina {
    /// 耗盡後恢復到此比例才能重新衝刺（防止體力閃爍）
    pub const RECOVERY_THRESHOLD: f32 = 0.2;

    /// 體力比例 (0.0 ~ 1.0)
    pub fn ratio(&self) -> f32 {
        self.current / self.max
    }

    /// 消耗體力，返回是否仍有體力
    pub fn drain(&mut self, dt: f32) -> bool {
        self.current = (self.current - self.drain_rate * dt).max(0.0);
        if self.current <= 0.0 {
            self.exhausted = true;
            false
        } else {
            true
        }
    }

    /// 恢復體力
    pub fn regenerate(&mut self, dt: f32) {
        self.current = (self.current + self.regen_rate * dt).min(self.max);
        if self.exhausted && self.ratio() >= Self::RECOVERY_THRESHOLD {
            self.exhausted = false;
        }
    }

    /// 能否衝刺
    pub fn can_sprint(&self) -> bool {
        !self.exhausted && self.current > 0.0
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
            self.dodge_direction = direction.normalize_or_zero();
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

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamina_default() {
        let s = Stamina::default();
        assert_eq!(s.current, 100.0);
        assert_eq!(s.max, 100.0);
        assert!(!s.exhausted);
        assert!(s.can_sprint());
    }

    #[test]
    fn stamina_drain() {
        let mut s = Stamina::default();
        // 1 秒消耗 15
        assert!(s.drain(1.0));
        assert!((s.current - 85.0).abs() < f32::EPSILON);
        assert!(!s.exhausted);
    }

    #[test]
    fn stamina_drain_to_zero() {
        let mut s = Stamina::default();
        // 消耗到 0（100 / 15 ≈ 6.67 秒）
        assert!(!s.drain(7.0));
        assert_eq!(s.current, 0.0);
        assert!(s.exhausted);
        assert!(!s.can_sprint());
    }

    #[test]
    fn stamina_regenerate() {
        let mut s = Stamina { current: 50.0, ..Default::default() };
        // 1 秒恢復 10
        s.regenerate(1.0);
        assert!((s.current - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stamina_regenerate_capped_at_max() {
        let mut s = Stamina { current: 95.0, ..Default::default() };
        s.regenerate(2.0);
        assert_eq!(s.current, 100.0);
    }

    #[test]
    fn stamina_exhausted_recovery_threshold() {
        let mut s = Stamina {
            current: 0.0,
            exhausted: true,
            ..Default::default()
        };
        assert!(!s.can_sprint());

        // 恢復到 15%（低於 20% 門檻）→ 仍然 exhausted
        s.regenerate(1.5); // +15
        assert!(s.exhausted);
        assert!(!s.can_sprint());

        // 恢復到 25%（超過 20% 門檻）→ 解除 exhausted
        s.regenerate(1.0); // +10 = 25
        assert!(!s.exhausted);
        assert!(s.can_sprint());
    }

    #[test]
    fn stamina_ratio() {
        let s = Stamina { current: 75.0, ..Default::default() };
        assert!((s.ratio() - 0.75).abs() < f32::EPSILON);
    }

    // ============================================================================
    // 噪音等級測試
    // ============================================================================

    #[test]
    fn noise_level_values() {
        assert_eq!(NoiseLevel::Silent.value(), 0.0);
        assert!(NoiseLevel::Low.value() > 0.0);
        assert!(NoiseLevel::Loud.value() > NoiseLevel::Low.value());
        assert_eq!(NoiseLevel::Max.value(), 1.0);
    }

    #[test]
    fn noise_level_detection_radius() {
        assert_eq!(NoiseLevel::Silent.detection_radius(), 0.0);
        assert!(NoiseLevel::Low.detection_radius() > 0.0);
        assert!(NoiseLevel::Loud.detection_radius() > NoiseLevel::Low.detection_radius());
        assert!(NoiseLevel::Max.detection_radius() >= 50.0);
    }

    #[test]
    fn stealth_state_default() {
        let state = StealthState::default();
        assert_eq!(state.noise_level, NoiseLevel::Low);
        assert_eq!(state.noise_decay_timer, 0.0);
    }

    #[test]
    fn stealth_kill_multiplier_is_high() {
        assert!(STEALTH_KILL_MULTIPLIER >= 5.0);
    }

    #[test]
    fn player_crouch_defaults() {
        let player = Player::default();
        assert!(!player.is_crouching);
        assert!(player.crouch_speed < player.speed);
        assert!(player.crouch_speed > 0.0);
    }
}

// ============================================================================
// 潛行系統
// ============================================================================

/// 噪音等級
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NoiseLevel {
    /// 蹲伏靜止/緩行：幾乎無聲
    Silent,
    /// 一般步行
    #[default]
    Low,
    /// 衝刺
    Loud,
    /// 射擊/爆炸
    Max,
}

impl NoiseLevel {
    /// 噪音值（用於 AI 聽覺偵測）
    pub fn value(&self) -> f32 {
        match self {
            NoiseLevel::Silent => 0.0,
            NoiseLevel::Low => 0.3,
            NoiseLevel::Loud => 0.7,
            NoiseLevel::Max => 1.0,
        }
    }

    /// 噪音偵測半徑（公尺）
    pub fn detection_radius(&self) -> f32 {
        match self {
            NoiseLevel::Silent => 0.0,
            NoiseLevel::Low => 8.0,
            NoiseLevel::Loud => 25.0,
            NoiseLevel::Max => 50.0,
        }
    }
}

/// 潛行狀態資源
#[derive(Resource)]
pub struct StealthState {
    /// 當前噪音等級
    pub noise_level: NoiseLevel,
    /// 噪音衰減計時器（射擊後慢慢降低）
    pub noise_decay_timer: f32,
}

impl Default for StealthState {
    fn default() -> Self {
        Self {
            noise_level: NoiseLevel::Low,
            noise_decay_timer: 0.0,
        }
    }
}

/// 噪音衰減時間（射擊後幾秒噪音恢復正常）
pub const NOISE_DECAY_TIME: f32 = 2.0;
/// 靜默擊殺傷害倍率
pub const STEALTH_KILL_MULTIPLIER: f32 = 10.0;
