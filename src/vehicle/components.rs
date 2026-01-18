//! 載具組件

use bevy::prelude::*;
use bevy::pbr::StandardMaterial;

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
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum VehicleType {
    Scooter,    // 機車
    Car,        // 汽車
    Taxi,       // 計程車
    Bus,        // 公車
}

/// 載具組件（GTA 風格街機物理）
#[derive(Component)]
pub struct Vehicle {
    // === 基本屬性 ===
    pub vehicle_type: VehicleType,
    pub max_speed: f32,
    pub acceleration: f32,
    pub turn_speed: f32,
    pub current_speed: f32,
    pub is_occupied: bool,

    // === 機車傾斜系統 ===
    pub lean_angle: f32,        // 當前傾斜角度 (弧度)
    pub max_lean_angle: f32,    // 最大傾斜角度 (弧度)

    // === 加速系統（非線性扭力曲線）===
    pub power_band_low: f32,    // 低速扭力倍率 (0~30% 速度)
    pub power_band_peak: f32,   // 峰值扭力倍率 (30~70% 速度)
    pub top_end_falloff: f32,   // 高速衰減倍率 (70~100% 速度)

    // === 煞車系統 ===
    pub braking_power: f32,     // 煞車力道基礎值
    pub brake_force: f32,       // 一般煞車力道
    pub handbrake_force: f32,   // 手煞車力道（漂移用）

    // === 轉向/操控 ===
    pub handling: f32,                  // 操控靈敏度
    pub high_speed_turn_factor: f32,    // 高速轉向衰減 (0.0~1.0)
    pub steering_response: f32,         // 轉向響應速度
    pub counter_steer_assist: f32,      // 反打救車輔助

    // === 漂移系統 ===
    pub drift_threshold: f32,   // 漂移觸發角度
    pub drift_grip: f32,        // 漂移中的抓地力
    pub is_drifting: bool,      // 漂移狀態
    pub drift_angle: f32,       // 當前漂移角度
    pub is_handbraking: bool,   // 手煞車狀態

    // === 車身動態（汽車/公車用）===
    pub body_roll_factor: f32,  // 車身側傾係數
    pub body_pitch_factor: f32, // 車身前後傾係數
    pub body_roll: f32,         // 當前側傾角
    pub body_pitch: f32,        // 當前前後傾角
    pub suspension_stiffness: f32, // 懸吊硬度（影響傾斜恢復速度）

    // === 輸入狀態 ===
    pub throttle_input: f32,    // 油門輸入 (0.0~1.0)
    pub brake_input: f32,       // 煞車輸入 (0.0~1.0)
    pub steer_input: f32,       // 轉向輸入 (-1.0~1.0)
    pub wheel_spin: f32,        // 輪胎打滑程度 (0.0~1.0)
}

impl Vehicle {
    /// 機車 - 台灣街頭最常見的交通工具
    /// 特色：靈活、加速快、可傾斜過彎、容易漂移
    pub fn scooter() -> Self {
        Self {
            vehicle_type: VehicleType::Scooter,
            max_speed: 22.0,           // 約 80 km/h
            acceleration: 18.0,        // 機車加速快
            turn_speed: 4.0,           // 轉向靈活
            current_speed: 0.0,
            is_occupied: false,

            // 機車傾斜
            lean_angle: 0.0,
            max_lean_angle: 0.5,       // 約 28 度傾斜

            // 加速系統（機車低速扭力強）
            power_band_low: 1.3,
            power_band_peak: 1.0,
            top_end_falloff: 0.7,      // 高速衰減明顯

            // 煞車
            braking_power: 0.85,
            brake_force: 25.0,
            handbrake_force: 35.0,     // 機車拉手煞容易甩尾

            // 轉向
            handling: 1.5,
            high_speed_turn_factor: 0.5, // 高速仍可轉向
            steering_response: 8.0,      // 快速響應
            counter_steer_assist: 0.3,

            // 漂移
            drift_threshold: 0.3,
            drift_grip: 0.6,
            is_drifting: false,
            drift_angle: 0.0,
            is_handbraking: false,

            // 車身動態（機車用 lean 不用 roll）
            body_roll_factor: 0.0,
            body_pitch_factor: 0.15,   // 輕微前後傾
            body_roll: 0.0,
            body_pitch: 0.0,
            suspension_stiffness: 5.0,

            // 輸入狀態
            throttle_input: 0.0,
            brake_input: 0.0,
            steer_input: 0.0,
            wheel_spin: 0.0,
        }
    }

    /// 汽車 - 平衡型，GTA 風格漂移
    pub fn car() -> Self {
        Self {
            vehicle_type: VehicleType::Car,
            max_speed: 35.0,
            acceleration: 12.0,
            turn_speed: 2.0,
            current_speed: 0.0,
            is_occupied: false,

            lean_angle: 0.0,
            max_lean_angle: 0.0,       // 汽車不傾斜

            // 加速系統（中速區最強）
            power_band_low: 1.0,
            power_band_peak: 1.2,
            top_end_falloff: 0.5,

            // 煞車
            braking_power: 0.7,
            brake_force: 20.0,
            handbrake_force: 40.0,     // 手煞車漂移關鍵

            // 轉向
            handling: 1.0,
            high_speed_turn_factor: 0.3, // 高速難轉
            steering_response: 5.0,
            counter_steer_assist: 0.5,   // GTA 風格救車

            // 漂移
            drift_threshold: 0.4,
            drift_grip: 0.5,
            is_drifting: false,
            drift_angle: 0.0,
            is_handbraking: false,

            // 車身動態（明顯側傾）
            body_roll_factor: 0.08,
            body_pitch_factor: 0.06,   // 加速後仰/煞車前傾
            body_roll: 0.0,
            body_pitch: 0.0,
            suspension_stiffness: 4.0,

            // 輸入狀態
            throttle_input: 0.0,
            brake_input: 0.0,
            steer_input: 0.0,
            wheel_spin: 0.0,
        }
    }

    /// 計程車 - 平衡型，略高操控性
    pub fn taxi() -> Self {
        Self {
            vehicle_type: VehicleType::Taxi,
            max_speed: 30.0,
            acceleration: 11.0,
            turn_speed: 2.2,
            current_speed: 0.0,
            is_occupied: false,

            lean_angle: 0.0,
            max_lean_angle: 0.0,

            power_band_low: 1.1,
            power_band_peak: 1.1,
            top_end_falloff: 0.6,

            braking_power: 0.75,
            brake_force: 22.0,
            handbrake_force: 38.0,

            handling: 1.1,
            high_speed_turn_factor: 0.35,
            steering_response: 5.5,
            counter_steer_assist: 0.45,

            drift_threshold: 0.4,
            drift_grip: 0.5,
            is_drifting: false,
            drift_angle: 0.0,
            is_handbraking: false,

            body_roll_factor: 0.07,
            body_pitch_factor: 0.05,
            body_roll: 0.0,
            body_pitch: 0.0,
            suspension_stiffness: 4.5,

            throttle_input: 0.0,
            brake_input: 0.0,
            steer_input: 0.0,
            wheel_spin: 0.0,
        }
    }

    /// 公車 - 笨重、難漂移、誇張傾斜（有趣）
    pub fn bus() -> Self {
        Self {
            vehicle_type: VehicleType::Bus,
            max_speed: 15.0,
            acceleration: 8.0,
            turn_speed: 1.5,
            current_speed: 0.0,
            is_occupied: false,

            lean_angle: 0.0,
            max_lean_angle: 0.0,

            // 柴油引擎低速扭力大
            power_band_low: 1.5,
            power_band_peak: 0.8,
            top_end_falloff: 0.3,      // 高速無力

            braking_power: 0.6,
            brake_force: 15.0,         // 煞車較弱
            handbrake_force: 25.0,

            handling: 0.7,
            high_speed_turn_factor: 0.15, // 高速幾乎轉不動
            steering_response: 2.0,       // 慢響應
            counter_steer_assist: 0.2,

            drift_threshold: 0.6,      // 難進入漂移
            drift_grip: 0.3,
            is_drifting: false,
            drift_angle: 0.0,
            is_handbraking: false,

            // 誇張側傾（有趣的視覺效果）
            body_roll_factor: 0.15,
            body_pitch_factor: 0.10,   // 明顯點頭
            body_roll: 0.0,
            body_pitch: 0.0,
            suspension_stiffness: 2.0, // 軟懸吊

            throttle_input: 0.0,
            brake_input: 0.0,
            steer_input: 0.0,
            wheel_spin: 0.0,
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
    pub waypoints: Vec<Vec3>,   // 預定行駛路線
    pub current_wp_index: usize, // 當前目標點索引
    pub stuck_timer: f32, // 卡住計時器
}

impl Default for NpcVehicle {
    fn default() -> Self {
        Self {
            state: NpcState::Cruising,
            check_timer: Timer::from_seconds(0.2, TimerMode::Repeating),
            waypoints: vec![],
            current_wp_index: 0,
            stuck_timer: 0.0,
        }
    }
}

// ============================================================================
// 車輛視覺效果組件（GTA 5 風格）
// ============================================================================

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
    pub fn new(points: Vec<(Vec3, f32)>) -> Self {
        Self {
            points,
            ..Default::default()
        }
    }

    /// 計算當前透明度
    pub fn alpha(&self) -> f32 {
        // 前 70% 時間完全不透明，之後淡出
        let fade_start = 0.7;
        let progress = self.lifetime / self.max_lifetime;
        if progress < fade_start {
            1.0
        } else {
            1.0 - (progress - fade_start) / (1.0 - fade_start)
        }
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
        let progress = self.lifetime / self.max_lifetime;
        (1.0 - progress).max(0.0)
    }

    /// 計算當前縮放（煙霧會擴散變大）
    pub fn scale(&self) -> f32 {
        let progress = self.lifetime / self.max_lifetime;
        self.initial_scale * (1.0 + progress * 2.0)  // 最終是初始的 3 倍大
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
}

impl VehicleEffectVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            smoke_mesh: meshes.add(Sphere::new(0.5)),
            smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.8, 0.8, 0.5),  // 灰白色半透明
                alpha_mode: bevy::prelude::AlphaMode::Blend,
                unlit: true,  // 不受光照影響
                ..default()
            }),
            tire_track_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.1, 0.1, 0.1, 0.8),  // 深色輪胎痕
                alpha_mode: bevy::prelude::AlphaMode::Blend,
                unlit: true,
                double_sided: true,  // 雙面可見
                ..default()
            }),
            tire_track_mesh: meshes.add(Cuboid::new(0.3, 0.01, 0.5)),  // 薄平面
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
// 車輛損壞系統（GTA 5 風格）
// ============================================================================

/// 車輛損壞狀態
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VehicleDamageState {
    /// 完好無損
    #[default]
    Pristine,
    /// 輕微損壞（<25% 傷害）：有刮痕
    Light,
    /// 中度損壞（25-50%）：凹陷、冒煙
    Moderate,
    /// 嚴重損壞（50-75%）：大量冒煙、引擎聲異常
    Heavy,
    /// 瀕臨爆炸（75-99%）：著火、閃爍
    Critical,
    /// 已爆炸
    Destroyed,
}

impl VehicleDamageState {
    /// 根據血量百分比計算損壞狀態
    pub fn from_health_percent(percent: f32) -> Self {
        if percent >= 1.0 {
            VehicleDamageState::Pristine
        } else if percent >= 0.75 {
            VehicleDamageState::Light
        } else if percent >= 0.50 {
            VehicleDamageState::Moderate
        } else if percent >= 0.25 {
            VehicleDamageState::Heavy
        } else if percent > 0.0 {
            VehicleDamageState::Critical
        } else {
            VehicleDamageState::Destroyed
        }
    }
}

/// 車輛血量組件
#[derive(Component, Debug)]
pub struct VehicleHealth {
    /// 當前血量
    pub current: f32,
    /// 最大血量
    pub max: f32,
    /// 損壞狀態
    pub damage_state: VehicleDamageState,
    /// 是否無敵（暫時）
    pub is_invulnerable: bool,
    /// 最後受傷時間
    pub last_damage_time: f32,
    /// 著火狀態
    pub is_on_fire: bool,
    /// 著火計時器（到 0 時爆炸）
    pub fire_timer: f32,
    /// 爆炸冷卻（防止連續爆炸）
    pub explosion_cooldown: f32,
}

impl Default for VehicleHealth {
    fn default() -> Self {
        Self {
            current: 1000.0,
            max: 1000.0,
            damage_state: VehicleDamageState::Pristine,
            is_invulnerable: false,
            last_damage_time: 0.0,
            is_on_fire: false,
            fire_timer: 5.0,
            explosion_cooldown: 0.0,
        }
    }
}

impl VehicleHealth {
    /// 創建指定血量的車輛
    pub fn new(max_health: f32) -> Self {
        Self {
            current: max_health,
            max: max_health,
            ..default()
        }
    }

    /// 根據車輛類型創建
    pub fn for_vehicle_type(vehicle_type: VehicleType) -> Self {
        let max_health = match vehicle_type {
            VehicleType::Scooter => 500.0,   // 機車較脆弱
            VehicleType::Car => 1000.0,      // 汽車標準
            VehicleType::Taxi => 1200.0,     // 計程車較耐打
            VehicleType::Bus => 2000.0,      // 公車最耐打
        };
        Self::new(max_health)
    }

    /// 受傷
    pub fn take_damage(&mut self, amount: f32, time: f32) -> f32 {
        if self.is_invulnerable || self.damage_state == VehicleDamageState::Destroyed {
            return 0.0;
        }

        let actual_damage = amount.min(self.current);
        self.current = (self.current - amount).max(0.0);
        self.last_damage_time = time;

        // 更新損壞狀態
        self.damage_state = VehicleDamageState::from_health_percent(self.percentage());

        // 瀕臨爆炸時開始著火
        if self.damage_state == VehicleDamageState::Critical && !self.is_on_fire {
            self.is_on_fire = true;
            self.fire_timer = 5.0;  // 5 秒後爆炸
        }

        actual_damage
    }

    /// 修復
    pub fn repair(&mut self, amount: f32) {
        if self.damage_state == VehicleDamageState::Destroyed {
            return;
        }

        self.current = (self.current + amount).min(self.max);
        self.damage_state = VehicleDamageState::from_health_percent(self.percentage());

        // 修復到一定程度時滅火
        if self.percentage() > 0.3 {
            self.is_on_fire = false;
        }
    }

    /// 完全修復
    pub fn full_repair(&mut self) {
        self.current = self.max;
        self.damage_state = VehicleDamageState::Pristine;
        self.is_on_fire = false;
        self.fire_timer = 5.0;
    }

    /// 血量百分比
    pub fn percentage(&self) -> f32 {
        self.current / self.max
    }

    /// 是否已爆炸
    pub fn is_destroyed(&self) -> bool {
        self.damage_state == VehicleDamageState::Destroyed
    }

    /// 更新著火計時器
    ///
    /// # 著火機制說明
    /// 車輛著火時有兩種傷害來源：
    /// 1. **計時器爆炸**：`fire_timer` 倒數到 0 時觸發爆炸（預設 5 秒）
    /// 2. **持續燒傷**：每秒扣除 `FIRE_DAMAGE_PER_SECOND` 血量（20 HP/s）
    ///
    /// 這意味著即使車輛血量充足，著火 5 秒後仍會爆炸。
    /// 玩家應該在車輛著火後盡快離開。
    ///
    /// # 回傳值
    /// - `true`：車輛爆炸（計時器歸零或血量歸零）
    /// - `false`：車輛仍在燃燒
    pub fn tick_fire(&mut self, dt: f32) -> bool {
        /// 著火時每秒造成的傷害
        const FIRE_DAMAGE_PER_SECOND: f32 = 20.0;

        if self.is_on_fire {
            // 計時器倒數（到 0 時強制爆炸）
            self.fire_timer -= dt;

            // 持續燒傷：著火時每秒扣血
            self.current = (self.current - dt * FIRE_DAMAGE_PER_SECOND).max(0.0);
            self.damage_state = VehicleDamageState::from_health_percent(self.percentage());

            // 爆炸條件：計時器歸零 或 血量歸零
            if self.fire_timer <= 0.0 || self.current <= 0.0 {
                self.damage_state = VehicleDamageState::Destroyed;
                return true; // 爆炸！
            }
        }
        false
    }
}

/// 輪胎損壞組件
#[derive(Component, Debug)]
pub struct TireDamage {
    /// 四個輪胎的狀態（true = 爆胎）
    /// 順序：左前、右前、左後、右後
    pub flat_tires: [bool; 4],
    /// 爆胎後的操控懲罰（0.0-1.0）
    pub handling_penalty: f32,
    /// 爆胎後的最大速度懲罰（0.0-1.0）
    pub speed_penalty: f32,
}

impl Default for TireDamage {
    fn default() -> Self {
        Self {
            flat_tires: [false; 4],
            handling_penalty: 0.0,
            speed_penalty: 0.0,
        }
    }
}

impl TireDamage {
    /// 爆破指定輪胎
    pub fn pop_tire(&mut self, index: usize) {
        if index < 4 {
            self.flat_tires[index] = true;
            self.update_penalties();
        }
    }

    /// 修復指定輪胎
    pub fn repair_tire(&mut self, index: usize) {
        if index < 4 {
            self.flat_tires[index] = false;
            self.update_penalties();
        }
    }

    /// 修復所有輪胎
    pub fn repair_all(&mut self) {
        self.flat_tires = [false; 4];
        self.handling_penalty = 0.0;
        self.speed_penalty = 0.0;
    }

    /// 爆胎數量
    pub fn flat_count(&self) -> usize {
        self.flat_tires.iter().filter(|&&f| f).count()
    }

    /// 更新懲罰值
    fn update_penalties(&mut self) {
        let flat_count = self.flat_count();
        // flat_count 只可能是 0-4（4 個輪胎）
        debug_assert!(flat_count <= 4, "flat_count 超出範圍: {}", flat_count);

        self.handling_penalty = match flat_count {
            0 => 0.0,
            1 => 0.2,   // 一個爆胎：輕微影響
            2 => 0.4,   // 兩個爆胎：明顯影響
            3 => 0.6,   // 三個爆胎：嚴重影響
            4 | _ => 0.8,   // 全部爆胎：幾乎無法操控（_ 用於消除警告但不應觸發）
        };
        self.speed_penalty = match flat_count {
            0 => 0.0,
            1 => 0.1,   // 速度降低 10%
            2 => 0.25,  // 速度降低 25%
            3 => 0.4,   // 速度降低 40%
            4 | _ => 0.6,   // 速度降低 60%（_ 用於消除警告但不應觸發）
        };
    }

    /// 檢查前輪是否有爆胎（影響轉向）
    pub fn has_front_flat(&self) -> bool {
        self.flat_tires[0] || self.flat_tires[1]
    }

    /// 檢查後輪是否有爆胎（影響穩定性）
    pub fn has_rear_flat(&self) -> bool {
        self.flat_tires[2] || self.flat_tires[3]
    }
}

/// 車輛損壞視覺效果資源
#[derive(Resource)]
pub struct VehicleDamageVisuals {
    /// 冒煙粒子 mesh
    pub smoke_mesh: Handle<Mesh>,
    /// 輕微冒煙材質（白煙）
    pub light_smoke_material: Handle<StandardMaterial>,
    /// 嚴重冒煙材質（黑煙）
    pub heavy_smoke_material: Handle<StandardMaterial>,
    /// 火焰粒子 mesh
    pub fire_mesh: Handle<Mesh>,
    /// 火焰材質
    pub fire_material: Handle<StandardMaterial>,
    /// 爆炸粒子 mesh
    pub explosion_mesh: Handle<Mesh>,
    /// 爆炸材質
    pub explosion_material: Handle<StandardMaterial>,
}

impl VehicleDamageVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            smoke_mesh: meshes.add(Sphere::new(0.3)),
            light_smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.8, 0.8, 0.4),
                alpha_mode: bevy::prelude::AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            heavy_smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.2, 0.2, 0.6),
                alpha_mode: bevy::prelude::AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            fire_mesh: meshes.add(Sphere::new(0.2)),
            fire_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0),
                emissive: LinearRgba::new(15.0, 8.0, 0.0, 1.0),
                alpha_mode: bevy::prelude::AlphaMode::Blend,
                ..default()
            }),
            explosion_mesh: meshes.add(Sphere::new(2.0)),
            explosion_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.2),
                emissive: LinearRgba::new(50.0, 30.0, 5.0, 1.0),
                alpha_mode: bevy::prelude::AlphaMode::Blend,
                ..default()
            }),
        }
    }
}

/// 車輛損壞煙霧粒子
#[derive(Component)]
pub struct VehicleDamageSmoke {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 是否為黑煙（嚴重損壞）
    pub is_heavy: bool,
}

impl VehicleDamageSmoke {
    pub fn new(velocity: Vec3, is_heavy: bool) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime: if is_heavy { 2.0 } else { 1.5 },
            is_heavy,
        }
    }

    pub fn alpha(&self) -> f32 {
        (1.0 - self.lifetime / self.max_lifetime).max(0.0)
    }
}

/// 車輛火焰粒子
#[derive(Component)]
pub struct VehicleFire {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl VehicleFire {
    pub fn new(velocity: Vec3) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime: 0.5,
        }
    }

    pub fn scale(&self) -> f32 {
        let progress = self.lifetime / self.max_lifetime;
        (1.0 - progress * 0.5).max(0.3)
    }
}

// ============================================================================
// 紅綠燈交通系統（GTA 5 風格）
// ============================================================================

/// 紅綠燈狀態
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TrafficLightState {
    /// 紅燈 - 停止
    Red,
    /// 黃燈 - 準備停止
    Yellow,
    /// 綠燈 - 通行
    #[default]
    Green,
}

impl TrafficLightState {
    /// 取得下一個狀態
    pub fn next(&self) -> Self {
        match self {
            TrafficLightState::Green => TrafficLightState::Yellow,
            TrafficLightState::Yellow => TrafficLightState::Red,
            TrafficLightState::Red => TrafficLightState::Green,
        }
    }

    /// 取得狀態持續時間（秒）
    pub fn duration(&self) -> f32 {
        match self {
            TrafficLightState::Green => 8.0,   // 綠燈 8 秒
            TrafficLightState::Yellow => 2.0,  // 黃燈 2 秒
            TrafficLightState::Red => 10.0,    // 紅燈 10 秒
        }
    }

    /// 取得燈光顏色
    pub fn color(&self) -> Color {
        match self {
            TrafficLightState::Green => Color::srgb(0.0, 1.0, 0.0),
            TrafficLightState::Yellow => Color::srgb(1.0, 0.9, 0.0),
            TrafficLightState::Red => Color::srgb(1.0, 0.0, 0.0),
        }
    }

    /// 取得發光顏色（用於燈泡）
    pub fn emissive(&self) -> LinearRgba {
        match self {
            TrafficLightState::Green => LinearRgba::new(0.0, 15.0, 0.0, 1.0),
            TrafficLightState::Yellow => LinearRgba::new(15.0, 13.0, 0.0, 1.0),
            TrafficLightState::Red => LinearRgba::new(15.0, 0.0, 0.0, 1.0),
        }
    }
}

/// 紅綠燈組件
#[derive(Component)]
pub struct TrafficLight {
    /// 當前狀態
    pub state: TrafficLightState,
    /// 狀態計時器
    pub timer: Timer,
    /// 控制方向（車輛前進方向需與此方向一致才受此燈控制）
    /// 通常是燈面向的方向
    pub control_direction: Vec3,
    /// 偵測範圍（NPC 車輛在此範圍內會看到紅燈）
    pub detection_range: f32,
    /// 是否為主燈（主燈和副燈狀態相反）
    pub is_primary: bool,
}

impl Default for TrafficLight {
    fn default() -> Self {
        Self {
            state: TrafficLightState::Green,
            timer: Timer::from_seconds(TrafficLightState::Green.duration(), TimerMode::Once),
            control_direction: Vec3::NEG_Z,  // 默認面向 -Z
            detection_range: 15.0,
            is_primary: true,
        }
    }
}

impl TrafficLight {
    /// 創建指定方向的紅綠燈
    pub fn new(direction: Vec3, is_primary: bool) -> Self {
        let initial_state = if is_primary {
            TrafficLightState::Green
        } else {
            TrafficLightState::Red  // 副燈初始為紅燈
        };
        Self {
            state: initial_state,
            timer: Timer::from_seconds(initial_state.duration(), TimerMode::Once),
            control_direction: direction.normalize_or_zero(),
            detection_range: 15.0,
            is_primary,
        }
    }

    /// 切換到下一個狀態
    pub fn advance(&mut self) {
        self.state = self.state.next();
        self.timer = Timer::from_seconds(self.state.duration(), TimerMode::Once);
    }

    /// 檢查車輛是否應該停止
    /// - 車輛位置在偵測範圍內
    /// - 車輛行駛方向與控制方向大致相同
    pub fn should_vehicle_stop(&self, vehicle_pos: Vec3, vehicle_forward: Vec3, light_pos: Vec3) -> bool {
        // 只有紅燈需要停止
        if self.state != TrafficLightState::Red {
            return false;
        }

        // 檢查距離
        let to_light = light_pos - vehicle_pos;
        let distance = to_light.length();
        if distance > self.detection_range || distance < 2.0 {
            return false;  // 太遠或已經過燈
        }

        // 檢查車輛是否面向燈（車輛往燈的方向行駛）
        let to_light_flat = Vec3::new(to_light.x, 0.0, to_light.z).normalize_or_zero();
        let vehicle_forward_flat = Vec3::new(vehicle_forward.x, 0.0, vehicle_forward.z).normalize_or_zero();

        // 車輛需要朝向燈的方向（點積 > 0.5，約 60 度內）
        let dot_to_light = vehicle_forward_flat.dot(to_light_flat);
        if dot_to_light < 0.5 {
            return false;
        }

        // 檢查車輛行駛方向是否受此燈控制
        // 車輛前進方向需要與燈的控制方向相反（車輛朝向燈）
        let dot_control = vehicle_forward_flat.dot(-self.control_direction);
        dot_control > 0.5
    }
}

/// 紅綠燈燈泡標記（用於更新發光顏色）
#[derive(Component)]
pub struct TrafficLightBulb {
    /// 對應的燈光狀態（紅/黃/綠）
    pub light_type: TrafficLightState,
}

/// 紅綠燈視覺效果資源
#[derive(Resource)]
pub struct TrafficLightVisuals {
    /// 燈柱 mesh
    pub pole_mesh: Handle<Mesh>,
    /// 燈柱材質
    pub pole_material: Handle<StandardMaterial>,
    /// 燈箱 mesh
    pub box_mesh: Handle<Mesh>,
    /// 燈箱材質
    pub box_material: Handle<StandardMaterial>,
    /// 燈泡 mesh
    pub bulb_mesh: Handle<Mesh>,
    /// 紅燈材質（亮）
    pub red_on_material: Handle<StandardMaterial>,
    /// 紅燈材質（暗）
    pub red_off_material: Handle<StandardMaterial>,
    /// 黃燈材質（亮）
    pub yellow_on_material: Handle<StandardMaterial>,
    /// 黃燈材質（暗）
    pub yellow_off_material: Handle<StandardMaterial>,
    /// 綠燈材質（亮）
    pub green_on_material: Handle<StandardMaterial>,
    /// 綠燈材質（暗）
    pub green_off_material: Handle<StandardMaterial>,
}

impl TrafficLightVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            pole_mesh: meshes.add(Cylinder::new(0.1, 4.0)),
            pole_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.3, 0.3),
                metallic: 0.8,
                perceptual_roughness: 0.6,
                ..default()
            }),
            box_mesh: meshes.add(Cuboid::new(0.5, 1.2, 0.3)),
            box_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.15, 0.15),
                metallic: 0.5,
                perceptual_roughness: 0.8,
                ..default()
            }),
            bulb_mesh: meshes.add(Sphere::new(0.12)),
            red_on_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: LinearRgba::new(15.0, 0.0, 0.0, 1.0),
                ..default()
            }),
            red_off_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.1, 0.1),
                ..default()
            }),
            yellow_on_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.9, 0.0),
                emissive: LinearRgba::new(15.0, 13.0, 0.0, 1.0),
                ..default()
            }),
            yellow_off_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.27, 0.1),
                ..default()
            }),
            green_on_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                emissive: LinearRgba::new(0.0, 15.0, 0.0, 1.0),
                ..default()
            }),
            green_off_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.3, 0.1),
                ..default()
            }),
        }
    }

    /// 根據當前狀態取得燈泡材質
    pub fn get_bulb_material(&self, bulb_type: TrafficLightState, current_state: TrafficLightState) -> Handle<StandardMaterial> {
        let is_on = bulb_type == current_state;
        match bulb_type {
            TrafficLightState::Red => if is_on { self.red_on_material.clone() } else { self.red_off_material.clone() },
            TrafficLightState::Yellow => if is_on { self.yellow_on_material.clone() } else { self.yellow_off_material.clone() },
            TrafficLightState::Green => if is_on { self.green_on_material.clone() } else { self.green_off_material.clone() },
        }
    }
}

/// 車輛爆炸效果
#[derive(Component)]
pub struct VehicleExplosion {
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 爆炸中心
    pub center: Vec3,
    /// 爆炸範圍
    pub radius: f32,
    /// 傷害
    pub damage: f32,
    /// 是否已造成傷害
    pub damage_dealt: bool,
}

impl VehicleExplosion {
    pub fn new(center: Vec3, radius: f32, damage: f32) -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 1.0,
            center,
            radius,
            damage,
            damage_dealt: false,
        }
    }

    /// 計算當前縮放（先擴大後縮小）
    pub fn scale(&self) -> f32 {
        let progress = self.lifetime / self.max_lifetime;
        if progress < 0.3 {
            // 快速擴大
            progress / 0.3 * 1.5
        } else {
            // 緩慢縮小
            1.5 - (progress - 0.3) / 0.7 * 1.5
        }
    }

    pub fn alpha(&self) -> f32 {
        let progress = self.lifetime / self.max_lifetime;
        (1.0 - progress).max(0.0)
    }
}
