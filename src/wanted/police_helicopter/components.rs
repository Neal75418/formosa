//! 直升機常數、組件、資源

use bevy::prelude::*;

// ============================================================================
// 常數
// ============================================================================

/// 直升機生成所需通緝等級
pub const HELICOPTER_SPAWN_WANTED_LEVEL: u8 = 5;
/// 最大直升機數量
pub const HELICOPTER_MAX_COUNT: usize = 2;
/// 直升機生成冷卻（秒）
pub const HELICOPTER_SPAWN_COOLDOWN: f32 = 45.0;

/// 直升機懸停高度
pub const HELICOPTER_HOVER_ALTITUDE: f32 = 40.0;
/// 直升機最大高度
pub const HELICOPTER_MAX_ALTITUDE: f32 = 80.0;
/// 直升機最小高度
pub const HELICOPTER_MIN_ALTITUDE: f32 = 25.0;
/// 直升機飛行速度
pub const HELICOPTER_SPEED: f32 = 35.0;
/// 直升機轉向速率
pub const HELICOPTER_TURN_RATE: f32 = 1.2;
/// 直升機垂直移動速度
pub const HELICOPTER_VERTICAL_SPEED: f32 = 10.0;

/// 直升機攻擊範圍
pub const HELICOPTER_ATTACK_RANGE: f32 = 50.0;
/// 直升機攻擊範圍平方
pub const HELICOPTER_ATTACK_RANGE_SQ: f32 = HELICOPTER_ATTACK_RANGE * HELICOPTER_ATTACK_RANGE;
pub(super) const HELICOPTER_ATTACK_RANGE_CLOSE: f32 = HELICOPTER_ATTACK_RANGE * 0.8;
pub(super) const HELICOPTER_ATTACK_RANGE_FAR: f32 = HELICOPTER_ATTACK_RANGE * 1.2;
pub(super) const HELICOPTER_ATTACK_RANGE_CLOSE_SQ: f32 =
    HELICOPTER_ATTACK_RANGE_CLOSE * HELICOPTER_ATTACK_RANGE_CLOSE;
pub(super) const HELICOPTER_ATTACK_RANGE_FAR_SQ: f32 =
    HELICOPTER_ATTACK_RANGE_FAR * HELICOPTER_ATTACK_RANGE_FAR;
pub(super) const HELICOPTER_MOVE_THRESHOLD: f32 = 10.0;
pub(super) const HELICOPTER_MOVE_THRESHOLD_SQ: f32 =
    HELICOPTER_MOVE_THRESHOLD * HELICOPTER_MOVE_THRESHOLD;
/// 直升機射擊頻率（每秒發射數）
pub const HELICOPTER_FIRE_RATE: f32 = 8.0;
/// 直升機子彈傷害
pub const HELICOPTER_BULLET_DAMAGE: f32 = 8.0;
/// 直升機生命值
pub const HELICOPTER_HEALTH: f32 = 500.0;

/// 探照燈範圍
pub const SPOTLIGHT_RANGE: f32 = 60.0;
/// 探照燈錐角（度）
pub const SPOTLIGHT_CONE_ANGLE: f32 = 25.0;

/// 主旋翼旋轉速度（度/秒）
pub const MAIN_ROTOR_SPEED: f32 = 720.0;
/// 尾旋翼旋轉速度（度/秒）
pub const TAIL_ROTOR_SPEED: f32 = 1200.0;

/// 規避時間（秒）
pub const EVADE_DURATION: f32 = 3.0;
/// 墜毀旋轉速度（度/秒）
pub const CRASH_ROTATION_SPEED: f32 = 180.0;

/// 玩家脫逃所需時間（秒）
pub(super) const PLAYER_ESCAPE_TIME: f32 = 15.0;

// ============================================================================
// 組件
// ============================================================================

/// 直升機狀態機
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HelicopterState {
    #[default]
    Approaching, // 飛向玩家
    Hovering,  // 懸停觀察
    Pursuing,  // 追蹤移動中的玩家
    Attacking, // 射擊玩家
    Evading,   // 規避傷害
    Crashing,  // 被擊落墜毀
}

/// 警用直升機組件
#[derive(Component)]
pub struct PoliceHelicopter {
    /// 當前狀態
    pub state: HelicopterState,
    /// 生命值
    pub health: f32,
    /// 目標高度
    pub target_altitude: f32,
    /// 射擊冷卻
    pub fire_cooldown: f32,
    /// 規避計時器
    pub evade_timer: f32,
    /// 目標位置
    pub target_position: Option<Vec3>,
    /// 懸停計時器
    pub hover_timer: f32,
    /// 搜索計時器
    pub search_timer: f32,
    /// 上次受傷時間
    pub last_hit_time: f32,
    /// 墜落速度
    pub crash_velocity: Vec3,
}

impl Default for PoliceHelicopter {
    fn default() -> Self {
        Self {
            state: HelicopterState::Approaching,
            health: HELICOPTER_HEALTH,
            target_altitude: HELICOPTER_HOVER_ALTITUDE,
            fire_cooldown: 0.0,
            evade_timer: 0.0,
            target_position: None,
            hover_timer: 0.0,
            search_timer: 0.0,
            last_hit_time: 0.0,
            crash_velocity: Vec3::ZERO,
        }
    }
}

/// 旋翼組件
#[derive(Component)]
pub struct HelicopterRotor {
    /// 旋轉速度（度/秒）
    pub rotation_speed: f32,
    /// 是否為主旋翼
    pub is_main_rotor: bool,
}

impl HelicopterRotor {
    /// 主旋翼旋轉部件
    pub fn main() -> Self {
        Self {
            rotation_speed: MAIN_ROTOR_SPEED,
            is_main_rotor: true,
        }
    }

    /// 尾旋翼旋轉部件
    pub fn tail() -> Self {
        Self {
            rotation_speed: TAIL_ROTOR_SPEED,
            is_main_rotor: false,
        }
    }
}

/// 探照燈組件
#[derive(Component, Default)]
pub struct HelicopterSpotlight;

/// 直升機父實體標記（用於查找子組件）
#[derive(Component)]
pub struct HelicopterParent(pub Entity);

// ============================================================================
// 資源
// ============================================================================

/// 直升機生成狀態
#[derive(Resource, Default)]
pub struct HelicopterSpawnState {
    /// 當前直升機數量
    pub count: usize,
    /// 生成冷卻計時器
    pub cooldown: f32,
}

/// 直升機視覺資源
#[derive(Resource)]
pub struct HelicopterVisuals {
    /// 機身材質
    pub body_material: Handle<StandardMaterial>,
    /// 旋翼材質
    pub rotor_material: Handle<StandardMaterial>,
    /// 機身 mesh
    pub body_mesh: Handle<Mesh>,
    /// 主旋翼 mesh
    pub main_rotor_mesh: Handle<Mesh>,
    /// 尾旋翼 mesh
    pub tail_rotor_mesh: Handle<Mesh>,
}
