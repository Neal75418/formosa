//! 警車追逐系統
//!
//! GTA 風格的警車追逐機制，包含：
//! - 警車生成與管理
//! - 警車 AI 駕駛（追逐、攔截、PIT 機動）
//! - 警車損壞與爆炸

#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

mod spawn;
mod ai;

pub use spawn::*;
pub use ai::*;

use bevy::prelude::*;

// ============================================================================
// 常數
// ============================================================================

/// 警車生成距離（最小）
pub const POLICE_CAR_SPAWN_DISTANCE_MIN: f32 = 50.0;
/// 警車生成距離（最大）
pub const POLICE_CAR_SPAWN_DISTANCE_MAX: f32 = 80.0;
/// 警車消失距離
pub const POLICE_CAR_DESPAWN_DISTANCE: f32 = 120.0;
/// 預計算距離平方（避免 sqrt）
pub const POLICE_CAR_DESPAWN_DISTANCE_SQ: f32 = POLICE_CAR_DESPAWN_DISTANCE * POLICE_CAR_DESPAWN_DISTANCE;
pub const POLICE_CAR_DESPAWN_FAR_DISTANCE_SQ: f32 = (POLICE_CAR_DESPAWN_DISTANCE * 1.5) * (POLICE_CAR_DESPAWN_DISTANCE * 1.5);
/// 警車最大數量（依通緝等級）
pub const MAX_POLICE_CARS_PER_STAR: u32 = 1;
/// 警車生成間隔（秒）
pub const POLICE_CAR_SPAWN_INTERVAL: f32 = 8.0;
/// PIT 機動距離（側面撞擊範圍）
pub const PIT_MANEUVER_DISTANCE: f32 = 3.0;
pub const PIT_MANEUVER_DISTANCE_SQ: f32 = PIT_MANEUVER_DISTANCE * PIT_MANEUVER_DISTANCE;
/// 追逐切換距離
pub const CHASE_SWITCH_DISTANCE: f32 = 30.0;
pub const CHASE_SWITCH_DISTANCE_SQ: f32 = CHASE_SWITCH_DISTANCE * CHASE_SWITCH_DISTANCE;
/// PIT 機動角度（與目標車輛的夾角，弧度）
pub const PIT_MANEUVER_ANGLE: f32 = 0.5; // 約 30 度
/// 攔截距離（前方阻擋）
pub const INTERCEPT_DISTANCE: f32 = 20.0;
pub const INTERCEPT_DISTANCE_SQ: f32 = INTERCEPT_DISTANCE * INTERCEPT_DISTANCE;
/// 追逐速度倍率
pub const CHASE_SPEED_MULTIPLIER: f32 = 1.2;
/// 警車碰撞傷害
pub const POLICE_CAR_COLLISION_DAMAGE: f32 = 15.0;
/// PIT 放棄距離（15 公尺）
pub const PIT_ABANDON_DISTANCE: f32 = 15.0;
pub const PIT_ABANDON_DISTANCE_SQ: f32 = PIT_ABANDON_DISTANCE * PIT_ABANDON_DISTANCE;
/// 攔截放棄距離（50 公尺）
pub const INTERCEPT_ABANDON_DISTANCE: f32 = 50.0;
pub const INTERCEPT_ABANDON_DISTANCE_SQ: f32 = INTERCEPT_ABANDON_DISTANCE * INTERCEPT_ABANDON_DISTANCE;
/// 前方檢測點積閾值（cos(45°) ≈ 0.7）
pub const FRONT_DOT_THRESHOLD: f32 = 0.7;

// ============================================================================
// 組件
// ============================================================================

/// 警車標記組件
#[derive(Component)]
pub struct PoliceCar {
    /// 警車 AI 狀態
    pub state: PoliceCarState,
    /// 駕駛警察實體（可選）
    pub driver: Option<Entity>,
    /// 目標玩家實體
    pub target: Option<Entity>,
    /// 追逐計時器
    pub chase_timer: f32,
    /// PIT 機動冷卻
    pub pit_cooldown: f32,
    /// 警笛是否啟動
    pub siren_active: bool,
    /// 最後碰撞時間
    pub last_collision_time: f32,
}

impl Default for PoliceCar {
    fn default() -> Self {
        Self {
            state: PoliceCarState::Responding,
            driver: None,
            target: None,
            chase_timer: 0.0,
            pit_cooldown: 0.0,
            siren_active: true,
            last_collision_time: 0.0,
        }
    }
}

/// 警車 AI 狀態
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PoliceCarState {
    /// 回應中（前往玩家位置）
    #[default]
    Responding,
    /// 追逐中（跟隨玩家車輛）
    Chasing,
    /// 執行 PIT 機動
    PitManeuver,
    /// 攔截（嘗試擋在前方）
    Intercepting,
    /// 撞毀（無法移動）
    Disabled,
}

/// 警車配置資源
#[derive(Resource)]
pub struct PoliceCarConfig {
    /// 生成間隔（秒）
    pub spawn_interval: f32,
    /// 上次生成時間
    pub last_spawn_time: f32,
    /// 追逐速度
    pub chase_speed: f32,
    /// PIT 機動速度
    pub pit_speed: f32,
    /// 攔截速度
    pub intercept_speed: f32,
}

impl Default for PoliceCarConfig {
    fn default() -> Self {
        Self {
            spawn_interval: POLICE_CAR_SPAWN_INTERVAL,
            last_spawn_time: 0.0,
            chase_speed: 40.0,
            pit_speed: 35.0,
            intercept_speed: 45.0,
        }
    }
}

/// 警車視覺資源
#[derive(Resource)]
pub struct PoliceCarVisuals {
    /// 車身 mesh
    pub body_mesh: Handle<Mesh>,
    /// 警車材質（白色配藍色）
    pub body_material: Handle<StandardMaterial>,
    /// 警笛燈 mesh
    pub siren_mesh: Handle<Mesh>,
    /// 警笛燈材質（紅）
    pub siren_red_material: Handle<StandardMaterial>,
    /// 警笛燈材質（藍）
    pub siren_blue_material: Handle<StandardMaterial>,
    /// 輪胎 mesh
    pub wheel_mesh: Handle<Mesh>,
    /// 輪胎材質
    pub wheel_material: Handle<StandardMaterial>,
}

impl PoliceCarVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            body_mesh: meshes.add(Cuboid::new(2.0, 0.8, 4.5)),
            body_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.9, 0.9, 0.95),
                metallic: 0.6,
                perceptual_roughness: 0.4,
                ..default()
            }),
            siren_mesh: meshes.add(Cuboid::new(0.8, 0.15, 0.4)),
            siren_red_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: LinearRgba::new(20.0, 0.0, 0.0, 1.0),
                ..default()
            }),
            siren_blue_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.0, 1.0),
                emissive: LinearRgba::new(0.0, 0.0, 20.0, 1.0),
                ..default()
            }),
            wheel_mesh: meshes.add(Cylinder::new(0.35, 0.2)),
            wheel_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.1),
                perceptual_roughness: 0.9,
                ..default()
            }),
        }
    }
}

/// 警笛燈組件（閃爍效果）
#[derive(Component)]
pub struct SirenLight {
    /// 是否為紅燈（否則藍燈）
    pub is_red: bool,
    /// 閃爍計時器
    pub flash_timer: f32,
    /// 是否亮起
    pub is_on: bool,
}
