//! 載具組件
#![allow(dead_code)]

use bevy::prelude::*;
use bevy::pbr::StandardMaterial;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::core::{lifetime_fade_alpha, lifetime_linear_alpha};

// ============================================================================
// 車輛穩定 ID（用於存檔識別）
// ============================================================================

/// 全局車輛 ID 計數器
static VEHICLE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 車輛穩定識別碼組件
///
/// 用於存檔/讀檔時識別特定車輛，不依賴 Query 枚舉順序
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VehicleId(pub u64);

impl VehicleId {
    /// 生成新的唯一 ID
    pub fn new() -> Self {
        Self(VEHICLE_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// 從已知 ID 創建（用於讀檔）
    pub fn from_raw(id: u64) -> Self {
        // 確保計數器超過這個 ID，避免將來衝突
        loop {
            let current = VEHICLE_ID_COUNTER.load(Ordering::SeqCst);
            if current > id {
                break;
            }
            if VEHICLE_ID_COUNTER.compare_exchange(current, id + 1, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                break;
            }
        }
        Self(id)
    }

    /// 取得原始 ID 值
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for VehicleId {
    fn default() -> Self {
        Self::new()
    }
}

/// 共享載具材質（效能優化：避免重複創建相同材質）
/// 每種通用材質只創建一次，所有載具共用
#[derive(Resource, Clone)]
pub struct VehicleMaterials {
    /// 黑色塑膠（座墊、踏板）
    pub black_plastic: Handle<StandardMaterial>,
    /// 輪胎
    pub wheel: Handle<StandardMaterial>,
    /// 頭燈（發光）
    pub headlight: Handle<StandardMaterial>,
    /// 尾燈（紅色發光）
    pub taillight: Handle<StandardMaterial>,
    /// 後照鏡（金屬）
    pub mirror: Handle<StandardMaterial>,
    /// 車窗玻璃（深色）
    pub glass: Handle<StandardMaterial>,
}

impl VehicleMaterials {
    /// 初始化共享材質（在 setup_world 中調用一次）
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            black_plastic: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.1),
                perceptual_roughness: 0.8,
                ..default()
            }),
            wheel: materials.add(StandardMaterial {
                base_color: Color::srgb(0.05, 0.05, 0.05),
                perceptual_roughness: 0.9,
                ..default()
            }),
            headlight: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                emissive: LinearRgba::new(15.0, 14.0, 10.0, 1.0),
                ..default()
            }),
            taillight: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: LinearRgba::new(10.0, 0.0, 0.0, 1.0),
                ..default()
            }),
            mirror: materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.8),
                metallic: 0.9,
                perceptual_roughness: 0.1,
                ..default()
            }),
            glass: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.1),
                perceptual_roughness: 0.1,
                metallic: 0.8,
                ..default()
            }),
        }
    }
}

/// 載具類型
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VehicleType {
    Scooter,    // 機車
    Car,        // 汽車
    Taxi,       // 計程車
    Bus,        // 公車
}

impl VehicleType {
    /// 取得穩定的存檔鍵值（不受 enum 重命名影響）
    pub fn save_key(&self) -> &'static str {
        match self {
            VehicleType::Scooter => "Scooter",
            VehicleType::Car => "Car",
            VehicleType::Taxi => "Taxi",
            VehicleType::Bus => "Bus",
        }
    }
}

/// 載具物理模式
/// - Dynamic：玩家駕駛，使用 Velocity/Impulse/Torque 驅動
/// - Kinematic：NPC/停放車輛，以 Transform 控制
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum VehiclePhysicsMode {
    Dynamic,
    Kinematic,
}

/// 載具核心組件（僅包含基本屬性，子系統拆分為獨立元件）
#[derive(Component)]
pub struct Vehicle {
    pub vehicle_type: VehicleType,
    pub max_speed: f32,
    pub acceleration: f32,
    pub turn_speed: f32,
    pub current_speed: f32,
    pub is_occupied: bool,
}

// ============================================================================
// 車輛子元件（從 Vehicle 上帝元件拆分而來）
// ============================================================================

/// 機車傾斜系統元件
#[derive(Component)]
pub struct VehicleLean {
    /// 當前傾斜角度 (弧度)
    pub lean_angle: f32,
    /// 最大傾斜角度 (弧度)
    pub max_lean_angle: f32,
}

impl Default for VehicleLean {
    fn default() -> Self {
        Self {
            lean_angle: 0.0,
            max_lean_angle: 0.0,
        }
    }
}

/// 加速系統（非線性扭力曲線）元件
#[derive(Component)]
pub struct VehiclePowerBand {
    /// 低速扭力倍率 (0~30% 速度)
    pub power_band_low: f32,
    /// 峰值扭力倍率 (30~70% 速度)
    pub power_band_peak: f32,
    /// 高速衰減倍率 (70~100% 速度)
    pub top_end_falloff: f32,
}

impl Default for VehiclePowerBand {
    fn default() -> Self {
        Self {
            power_band_low: 1.0,
            power_band_peak: 1.0,
            top_end_falloff: 0.5,
        }
    }
}

/// 煞車系統元件
#[derive(Component)]
pub struct VehicleBraking {
    /// 煞車力道基礎值
    pub braking_power: f32,
    /// 一般煞車力道
    pub brake_force: f32,
    /// 手煞車力道（漂移用）
    pub handbrake_force: f32,
}

impl Default for VehicleBraking {
    fn default() -> Self {
        Self {
            braking_power: 0.7,
            brake_force: 20.0,
            handbrake_force: 30.0,
        }
    }
}

/// 轉向/操控系統元件
#[derive(Component)]
pub struct VehicleSteering {
    /// 操控靈敏度
    pub handling: f32,
    /// 高速轉向衰減 (0.0~1.0)
    pub high_speed_turn_factor: f32,
    /// 轉向響應速度
    pub steering_response: f32,
    /// 反打救車輔助
    pub counter_steer_assist: f32,
}

impl Default for VehicleSteering {
    fn default() -> Self {
        Self {
            handling: 1.0,
            high_speed_turn_factor: 0.3,
            steering_response: 5.0,
            counter_steer_assist: 0.4,
        }
    }
}

/// 漂移系統元件
#[derive(Component)]
pub struct VehicleDrift {
    /// 漂移觸發角度
    pub drift_threshold: f32,
    /// 漂移中的抓地力
    pub drift_grip: f32,
    /// 漂移狀態
    pub is_drifting: bool,
    /// 當前漂移角度
    pub drift_angle: f32,
    /// 手煞車狀態
    pub is_handbraking: bool,
}

impl Default for VehicleDrift {
    fn default() -> Self {
        Self {
            drift_threshold: 0.4,
            drift_grip: 0.5,
            is_drifting: false,
            drift_angle: 0.0,
            is_handbraking: false,
        }
    }
}

/// 車身動態（汽車/公車用）元件
#[derive(Component)]
pub struct VehicleBodyDynamics {
    /// 車身側傾係數
    pub body_roll_factor: f32,
    /// 車身前後傾係數
    pub body_pitch_factor: f32,
    /// 當前側傾角
    pub body_roll: f32,
    /// 當前前後傾角
    pub body_pitch: f32,
    /// 懸吊硬度（影響傾斜恢復速度）
    pub suspension_stiffness: f32,
}

impl Default for VehicleBodyDynamics {
    fn default() -> Self {
        Self {
            body_roll_factor: 0.05,
            body_pitch_factor: 0.05,
            body_roll: 0.0,
            body_pitch: 0.0,
            suspension_stiffness: 4.0,
        }
    }
}

/// 車輛輸入狀態元件
#[derive(Component)]
pub struct VehicleInput {
    /// 油門輸入 (0.0~1.0)
    pub throttle_input: f32,
    /// 煞車輸入 (0.0~1.0)
    pub brake_input: f32,
    /// 轉向輸入 (-1.0~1.0)
    pub steer_input: f32,
    /// 輪胎打滑程度 (0.0~1.0)
    pub wheel_spin: f32,
}

impl Default for VehicleInput {
    fn default() -> Self {
        Self {
            throttle_input: 0.0,
            brake_input: 0.0,
            steer_input: 0.0,
            wheel_spin: 0.0,
        }
    }
}

impl Default for Vehicle {
    fn default() -> Self {
        Self {
            vehicle_type: VehicleType::Car,
            max_speed: 30.0,
            acceleration: 10.0,
            turn_speed: 2.0,
            current_speed: 0.0,
            is_occupied: false,
        }
    }
}

// ============================================================================
// 車輛預設配置（工廠方法）
// ============================================================================

/// 車輛預設配置，用於生成車輛時一次設定所有子元件
pub struct VehiclePreset {
    pub vehicle: Vehicle,
    pub lean: VehicleLean,
    pub power_band: VehiclePowerBand,
    pub braking: VehicleBraking,
    pub steering: VehicleSteering,
    pub drift: VehicleDrift,
    pub body_dynamics: VehicleBodyDynamics,
    pub input: VehicleInput,
}

impl VehiclePreset {
    /// 轉換為元件 tuple（可直接用於 Bevy spawn）
    #[allow(clippy::type_complexity)]
    pub fn into_components(
        self,
    ) -> (
        Vehicle,
        VehicleLean,
        VehiclePowerBand,
        VehicleBraking,
        VehicleSteering,
        VehicleDrift,
        VehicleBodyDynamics,
        VehicleInput,
    ) {
        (
            self.vehicle,
            self.lean,
            self.power_band,
            self.braking,
            self.steering,
            self.drift,
            self.body_dynamics,
            self.input,
        )
    }

    /// 機車 - 台灣街頭最常見的交通工具
    /// 特色：靈活、加速快、可傾斜過彎、容易漂移
    pub fn scooter() -> Self {
        Self {
            vehicle: Vehicle {
                vehicle_type: VehicleType::Scooter,
                max_speed: 22.0,
                acceleration: 18.0,
                turn_speed: 4.0,
                ..Default::default()
            },
            lean: VehicleLean {
                max_lean_angle: 0.5,
                ..Default::default()
            },
            power_band: VehiclePowerBand {
                power_band_low: 1.3,
                power_band_peak: 1.0,
                top_end_falloff: 0.7,
            },
            braking: VehicleBraking {
                braking_power: 0.85,
                brake_force: 25.0,
                handbrake_force: 35.0,
            },
            steering: VehicleSteering {
                handling: 1.5,
                high_speed_turn_factor: 0.5,
                steering_response: 8.0,
                counter_steer_assist: 0.3,
            },
            drift: VehicleDrift {
                drift_threshold: 0.3,
                drift_grip: 0.6,
                ..Default::default()
            },
            body_dynamics: VehicleBodyDynamics {
                body_roll_factor: 0.0,
                body_pitch_factor: 0.15,
                suspension_stiffness: 5.0,
                ..Default::default()
            },
            input: VehicleInput::default(),
        }
    }

    /// 汽車 - 平衡型，GTA 風格漂移
    pub fn car() -> Self {
        Self {
            vehicle: Vehicle {
                vehicle_type: VehicleType::Car,
                max_speed: 35.0,
                acceleration: 12.0,
                turn_speed: 2.0,
                ..Default::default()
            },
            lean: VehicleLean::default(),
            power_band: VehiclePowerBand {
                power_band_peak: 1.2,
                ..Default::default()
            },
            braking: VehicleBraking {
                handbrake_force: 40.0,
                ..Default::default()
            },
            steering: VehicleSteering {
                counter_steer_assist: 0.5,
                ..Default::default()
            },
            drift: VehicleDrift::default(),
            body_dynamics: VehicleBodyDynamics {
                body_roll_factor: 0.08,
                body_pitch_factor: 0.06,
                ..Default::default()
            },
            input: VehicleInput::default(),
        }
    }

    /// 計程車 - 平衡型，略高操控性
    pub fn taxi() -> Self {
        Self {
            vehicle: Vehicle {
                vehicle_type: VehicleType::Taxi,
                acceleration: 11.0,
                turn_speed: 2.2,
                ..Default::default()
            },
            lean: VehicleLean::default(),
            power_band: VehiclePowerBand {
                power_band_low: 1.1,
                power_band_peak: 1.1,
                top_end_falloff: 0.6,
            },
            braking: VehicleBraking {
                braking_power: 0.75,
                brake_force: 22.0,
                handbrake_force: 38.0,
            },
            steering: VehicleSteering {
                handling: 1.1,
                high_speed_turn_factor: 0.35,
                steering_response: 5.5,
                counter_steer_assist: 0.45,
            },
            drift: VehicleDrift::default(),
            body_dynamics: VehicleBodyDynamics {
                body_roll_factor: 0.07,
                suspension_stiffness: 4.5,
                ..Default::default()
            },
            input: VehicleInput::default(),
        }
    }

    /// 公車 - 笨重、難漂移、誇張傾斜（有趣）
    pub fn bus() -> Self {
        Self {
            vehicle: Vehicle {
                vehicle_type: VehicleType::Bus,
                max_speed: 15.0,
                acceleration: 8.0,
                turn_speed: 1.8,
                ..Default::default()
            },
            lean: VehicleLean::default(),
            power_band: VehiclePowerBand {
                power_band_low: 1.5,
                power_band_peak: 0.8,
                top_end_falloff: 0.3,
            },
            braking: VehicleBraking {
                braking_power: 0.6,
                brake_force: 15.0,
                handbrake_force: 25.0,
            },
            steering: VehicleSteering {
                handling: 0.7,
                high_speed_turn_factor: 0.15,
                steering_response: 2.0,
                counter_steer_assist: 0.2,
            },
            drift: VehicleDrift {
                drift_threshold: 0.6,
                drift_grip: 0.3,
                ..Default::default()
            },
            body_dynamics: VehicleBodyDynamics {
                body_roll_factor: 0.15,
                body_pitch_factor: 0.10,
                suspension_stiffness: 2.0,
                ..Default::default()
            },
            input: VehicleInput::default(),
        }
    }
}

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

// ============================================================================
// 車輛視覺效果組件（GTA 5 風格）
// ============================================================================

/// Vehicle visual root for applying roll/pitch/lean without affecting physics.
#[derive(Component)]
pub struct VehicleVisualRoot;

/// 輪胎痕跡組件
/// 漂移/急煞時在地面留下的胎痕
#[derive(Component)]
pub struct TireTrack {
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間（痕跡完全消失）
    pub max_lifetime: f32,
    /// 痕跡點列表 (位置, 寬度)
    pub points: Vec<(Vec3, f32)>,
}

impl Default for TireTrack {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 10.0,  // 10 秒後消失
            points: Vec::new(),
        }
    }
}

impl TireTrack {
    /// 建立新實例
    pub fn new(points: Vec<(Vec3, f32)>) -> Self {
        Self {
            points,
            ..Default::default()
        }
    }

    /// 計算當前透明度
    pub fn alpha(&self) -> f32 {
        lifetime_fade_alpha(self.lifetime, self.max_lifetime, 0.7)
    }
}

/// 漂移煙霧粒子組件
#[derive(Component)]
pub struct DriftSmoke {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 初始縮放
    pub initial_scale: f32,
}

impl DriftSmoke {
    /// 建立新實例
    pub fn new(velocity: Vec3, max_lifetime: f32) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime,
            initial_scale: 0.3,
        }
    }

    /// 計算當前透明度（煙霧會擴散變淡）
    pub fn alpha(&self) -> f32 {
        lifetime_linear_alpha(self.lifetime, self.max_lifetime)
    }

    /// 計算當前縮放（煙霧會擴散變大）
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.initial_scale * (1.0 + progress * 2.0)  // 最終是初始的 3 倍大
    }
}

/// 氮氣火焰粒子組件
#[derive(Component)]
pub struct NitroFlame {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 初始縮放
    pub initial_scale: f32,
}

impl NitroFlame {
    /// 建立新實例
    pub fn new(velocity: Vec3) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime: 0.15,  // 火焰粒子生命較短
            initial_scale: 0.2,
        }
    }

    /// 計算當前顏色（從藍白漸變到橙紅）
    pub fn color(&self) -> Color {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if progress < 0.3 {
            // 藍白色（核心高溫）
            Color::srgba(0.8, 0.9, 1.0, 1.0 - progress)
        } else if progress < 0.6 {
            // 黃橙色（中間）
            Color::srgba(1.0, 0.8, 0.3, 1.0 - progress)
        } else {
            // 橙紅色（外焰）
            Color::srgba(1.0, 0.4, 0.1, (1.0 - progress) * 0.5)
        }
    }

    /// 計算當前縮放（火焰會逐漸縮小消散）
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.initial_scale * (1.0 - progress * 0.5)
    }
}

/// 車輛視覺效果資源（預生成的 mesh 和 material）
#[derive(Resource)]
pub struct VehicleEffectVisuals {
    /// 煙霧粒子 mesh (球體)
    pub smoke_mesh: Handle<Mesh>,
    /// 煙霧粒子材質 (半透明灰白色)
    pub smoke_material: Handle<StandardMaterial>,
    /// 輪胎痕跡材質 (深色)
    pub tire_track_material: Handle<StandardMaterial>,
    /// 輪胎痕跡 mesh (薄平面)
    pub tire_track_mesh: Handle<Mesh>,
    /// 氮氣火焰 mesh (拉長的球體模擬火焰)
    pub nitro_flame_mesh: Handle<Mesh>,
    /// 氮氣火焰材質 (發光藍白色)
    pub nitro_flame_material: Handle<StandardMaterial>,
}

impl VehicleEffectVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            smoke_mesh: meshes.add(Sphere::new(0.5)),
            smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.8, 0.8, 0.5),  // 灰白色半透明
                alpha_mode: AlphaMode::Blend,
                unlit: true,  // 不受光照影響
                ..default()
            }),
            tire_track_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.1, 0.1, 0.1, 0.8),  // 深色輪胎痕
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,  // 雙面可見
                ..default()
            }),
            tire_track_mesh: meshes.add(Cuboid::new(0.3, 0.01, 0.5)),  // 薄平面
            // 氮氣火焰：拉長的球體
            nitro_flame_mesh: meshes.add(Sphere::new(0.3)),
            nitro_flame_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.9, 1.0, 0.9),  // 藍白色
                emissive: LinearRgba::rgb(5.0, 6.0, 8.0),  // 強發光
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
        }
    }
}

/// 車輛效果追蹤器
#[derive(Resource, Default)]
pub struct VehicleEffectTracker {
    /// 當前煙霧粒子數量
    pub smoke_count: usize,
    /// 最大煙霧粒子數量
    pub max_smoke_count: usize,
    /// 當前輪胎痕跡數量
    pub track_count: usize,
    /// 最大輪胎痕跡數量
    pub max_track_count: usize,
    /// 上次生成煙霧的時間
    pub last_smoke_spawn: f32,
    /// 煙霧生成間隔（秒）
    pub smoke_spawn_interval: f32,
    /// 上次生成輪胎痕跡的時間
    pub last_track_spawn: f32,
    /// 輪胎痕跡生成間隔（秒）
    pub track_spawn_interval: f32,
}

impl VehicleEffectTracker {
    /// 建立新實例
    pub fn new() -> Self {
        Self {
            smoke_count: 0,
            max_smoke_count: 50,
            track_count: 0,
            max_track_count: 30,
            last_smoke_spawn: 0.0,
            smoke_spawn_interval: 0.05,  // 每 0.05 秒生成一批煙霧
            last_track_spawn: 0.0,
            track_spawn_interval: 0.1,  // 每 0.1 秒生成一段痕跡
        }
    }
}

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- DriftSmoke ---

    #[test]
    fn drift_smoke_alpha_and_scale() {
        let mut s = DriftSmoke::new(Vec3::Y, 2.0);
        assert!((s.alpha() - 1.0).abs() < f32::EPSILON);
        assert!((s.scale() - 0.3).abs() < 0.01);
        s.lifetime = 1.0;
        assert!((s.alpha() - 0.5).abs() < f32::EPSILON);
        assert!((s.scale() - 0.3 * 2.0).abs() < 0.01);
    }

    // --- NitroFlame ---

    #[test]
    fn nitro_flame_scale_shrinks() {
        let mut f = NitroFlame::new(Vec3::Z);
        let s0 = f.scale();
        f.lifetime = 0.15;
        let s1 = f.scale();
        assert!(s0 > s1);
    }
}
