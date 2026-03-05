//! 車輛健康和損壞狀態組件

use super::super::VehicleType;
use bevy::prelude::*;

// ============================================================================
// 車輛損壞類型定義
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
            VehicleType::Scooter => 500.0, // 機車較脆弱
            VehicleType::Car => 1000.0,    // 汽車標準
            VehicleType::Taxi => 1200.0,   // 計程車較耐打
            VehicleType::Bus => 2000.0,    // 公車最耐打
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
            self.fire_timer = 5.0; // 5 秒後爆炸
        }

        actual_damage
    }

    /// 修復
    #[allow(dead_code)] // 修車廠系統預留
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

    /// 套用裝甲改裝（增加最大血量）
    /// 只應在改裝時呼叫一次，會增加最大血量並按比例恢復當前血量
    pub fn apply_armor_upgrade(&mut self, multiplier: f32) {
        let old_max = self.max;
        let new_max = old_max * multiplier;
        let health_ratio = self.current / old_max;

        self.max = new_max;
        self.current = new_max * health_ratio; // 保持相同的血量比例
        self.damage_state = VehicleDamageState::from_health_percent(self.percentage());
    }

    /// 完全修復
    #[allow(dead_code)] // 修車廠系統預留
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
    #[allow(dead_code)] // 修車廠系統預留
    pub fn repair_tire(&mut self, index: usize) {
        if index < 4 {
            self.flat_tires[index] = false;
            self.update_penalties();
        }
    }

    /// 修復所有輪胎
    #[allow(dead_code)] // 修車廠系統預留
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
        debug_assert!(flat_count <= 4, "flat_count 超出範圍: {flat_count}");

        self.handling_penalty = match flat_count {
            0 => 0.0,
            1 => 0.15, // 一個爆胎：輕微影響
            2 => 0.30, // 兩個爆胎：明顯影響
            3 => 0.45, // 三個爆胎：嚴重影響
            _ => 0.60, // 全部爆胎：困難但可操控
        };
        self.speed_penalty = match flat_count {
            0 => 0.0,
            1 => 0.10, // 速度降低 10%
            2 => 0.20, // 速度降低 20%
            3 => 0.35, // 速度降低 35%
            _ => 0.50, // 速度降低 50%
        };
    }

    /// 檢查前輪是否有爆胎（影響轉向）
    #[allow(dead_code)] // 進階操控差異化預留
    pub fn has_front_flat(&self) -> bool {
        self.flat_tires[0] || self.flat_tires[1]
    }

    /// 檢查後輪是否有爆胎（影響穩定性）
    #[allow(dead_code)] // 進階操控差異化預留
    pub fn has_rear_flat(&self) -> bool {
        self.flat_tires[2] || self.flat_tires[3]
    }
}

// ============================================================================
// 車門/車窗狀態
// ============================================================================

/// 車門位置索引
/// 順序：左前（駕駛座）、右前（副駕駛）、左後、右後
#[allow(dead_code)]
pub const DOOR_FRONT_LEFT: usize = 0;
#[allow(dead_code)]
pub const DOOR_FRONT_RIGHT: usize = 1;
#[allow(dead_code)]
pub const DOOR_BACK_LEFT: usize = 2;
#[allow(dead_code)]
pub const DOOR_BACK_RIGHT: usize = 3;

/// 車門開關動畫時長（秒）
const DOOR_ANIMATION_DURATION: f32 = 0.5;
/// 車門最大開啟角度（弧度）
#[allow(dead_code)]
pub const DOOR_MAX_ANGLE: f32 = std::f32::consts::FRAC_PI_3; // 60°
/// 高速開門風阻閾值（m/s）
const HIGH_SPEED_DOOR_THRESHOLD: f32 = 20.0;

/// 單一車門狀態
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DoorState {
    /// 關閉
    #[default]
    Closed,
    /// 正在開啟（0.0-1.0 進度）
    Opening(f32),
    /// 完全開啟
    Open,
    /// 正在關閉（1.0-0.0 進度）
    Closing(f32),
    /// 被撕裂/脫落（碰撞或高速開門）
    Broken,
}

impl DoorState {
    /// 取得當前開啟角度（0.0 = 關閉，`DOOR_MAX_ANGLE` = 全開）
    #[allow(dead_code)] // 車門視覺動畫預留
    pub fn angle(&self) -> f32 {
        match self {
            DoorState::Closed | DoorState::Broken => 0.0,
            DoorState::Opening(p) | DoorState::Closing(p) => *p * DOOR_MAX_ANGLE,
            DoorState::Open => DOOR_MAX_ANGLE,
        }
    }

    /// 是否可互動（開啟或關閉）
    #[allow(dead_code)] // UI 互動提示預留
    pub fn can_toggle(&self) -> bool {
        matches!(self, DoorState::Closed | DoorState::Open)
    }

    /// 是否為開啟或正在開啟狀態
    pub fn is_open_or_opening(&self) -> bool {
        matches!(self, DoorState::Open | DoorState::Opening(_))
    }
}

/// 單一車窗狀態
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WindowState {
    /// 完好
    #[default]
    Intact,
    /// 裂痕（中等損壞後觸發）
    Cracked,
    /// 破碎
    Broken,
}

/// 車門與車窗狀態組件
///
/// 追蹤 4 門 4 窗的獨立狀態，類似 `TireDamage` 的陣列模式。
/// 機車無此組件（只有 Car/Taxi/Bus 會附加）。
#[derive(Component, Debug)]
pub struct DoorWindowState {
    /// 四扇車門狀態：[左前, 右前, 左後, 右後]
    pub doors: [DoorState; 4],
    /// 四扇車窗狀態：[左前, 右前, 左後, 右後]
    pub windows: [WindowState; 4],
    /// 開門造成的風阻懲罰（0.0-1.0），影響最大速度
    pub drag_penalty: f32,
}

impl Default for DoorWindowState {
    fn default() -> Self {
        Self {
            doors: [DoorState::Closed; 4],
            windows: [WindowState::Intact; 4],
            drag_penalty: 0.0,
        }
    }
}

impl DoorWindowState {
    /// 切換指定車門（開→關 / 關→開）
    pub fn toggle_door(&mut self, index: usize) {
        if index >= 4 {
            return;
        }
        match self.doors[index] {
            DoorState::Closed => self.doors[index] = DoorState::Opening(0.0),
            DoorState::Open => self.doors[index] = DoorState::Closing(1.0),
            _ => {}
        }
    }

    /// 更新車門動畫進度，回傳是否有任何門正在移動
    pub fn tick_doors(&mut self, dt: f32) -> bool {
        let mut animating = false;
        let speed = dt / DOOR_ANIMATION_DURATION;

        for door in &mut self.doors {
            match door {
                DoorState::Opening(progress) => {
                    *progress = (*progress + speed).min(1.0);
                    if *progress >= 1.0 {
                        *door = DoorState::Open;
                    }
                    animating = true;
                }
                DoorState::Closing(progress) => {
                    *progress = (*progress - speed).max(0.0);
                    if *progress <= 0.0 {
                        *door = DoorState::Closed;
                    }
                    animating = true;
                }
                _ => {}
            }
        }

        self.update_drag_penalty();
        animating
    }

    /// 破壞指定車窗
    pub fn break_window(&mut self, index: usize) {
        if index < 4 {
            self.windows[index] = WindowState::Broken;
        }
    }

    /// 裂痕指定車窗（碰撞造成的輕微損壞）
    pub fn crack_window(&mut self, index: usize) {
        if index < 4 && self.windows[index] == WindowState::Intact {
            self.windows[index] = WindowState::Cracked;
        }
    }

    /// 破壞指定車門（高速碰撞或極端情況）
    #[allow(dead_code)] // 碰撞系統擴展預留
    pub fn break_door(&mut self, index: usize) {
        if index < 4 {
            self.doors[index] = DoorState::Broken;
            self.update_drag_penalty();
        }
    }

    /// 高速開門判定：速度超過閾值時門會被風吹斷
    pub fn check_high_speed_door_break(&mut self, speed: f32) {
        if speed < HIGH_SPEED_DOOR_THRESHOLD {
            return;
        }
        for i in 0..4 {
            if self.doors[i].is_open_or_opening() {
                self.doors[i] = DoorState::Broken;
            }
        }
        self.update_drag_penalty();
    }

    /// 計算開門風阻
    #[allow(clippy::cast_precision_loss)]
    fn update_drag_penalty(&mut self) {
        let open_count = self
            .doors
            .iter()
            .filter(|d| matches!(d, DoorState::Open | DoorState::Opening(_)))
            .count();
        // 每扇開門增加 5% 風阻（最多 20%）
        self.drag_penalty = open_count as f32 * 0.05;
    }

    /// 完好車窗數量
    #[allow(dead_code)] // UI 顯示預留
    pub fn intact_window_count(&self) -> usize {
        self.windows
            .iter()
            .filter(|w| **w == WindowState::Intact)
            .count()
    }

    /// 破碎車窗數量
    #[allow(dead_code)] // UI 顯示預留
    pub fn broken_window_count(&self) -> usize {
        self.windows
            .iter()
            .filter(|w| **w == WindowState::Broken)
            .count()
    }

    /// 脫落車門數量
    #[allow(dead_code)] // UI 顯示預留
    pub fn broken_door_count(&self) -> usize {
        self.doors
            .iter()
            .filter(|d| matches!(d, DoorState::Broken))
            .count()
    }

    /// 修復所有車門車窗
    #[allow(dead_code)] // 修車廠系統預留
    pub fn repair_all(&mut self) {
        self.doors = [DoorState::Closed; 4];
        self.windows = [WindowState::Intact; 4];
        self.drag_penalty = 0.0;
    }
}

// ============================================================================
// 車體部位損壞
// ============================================================================

/// 車體部位索引
pub const BODY_HOOD: usize = 0; // 引擎蓋
pub const BODY_FRONT_BUMPER: usize = 1; // 前保險桿
pub const BODY_REAR_BUMPER: usize = 2; // 後保險桿
pub const BODY_LEFT_PANEL: usize = 3; // 左側板
pub const BODY_RIGHT_PANEL: usize = 4; // 右側板
pub const BODY_ROOF: usize = 5; // 車頂

/// 車體部位數量
pub const BODY_PART_COUNT: usize = 6;

/// 單一部位損壞等級
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BodyPartState {
    /// 完好
    #[default]
    Intact,
    /// 刮痕（輕微損壞）
    Scratched,
    /// 凹陷（中度損壞）
    Dented,
    /// 嚴重變形（重度損壞）
    Crushed,
}

impl BodyPartState {
    /// 損壞等級數值（0-3，用於材質計算）
    pub fn severity(self) -> u8 {
        match self {
            BodyPartState::Intact => 0,
            BodyPartState::Scratched => 1,
            BodyPartState::Dented => 2,
            BodyPartState::Crushed => 3,
        }
    }

    /// 從傷害值推算部位狀態
    pub fn from_damage(damage: f32) -> Self {
        if damage < 50.0 {
            BodyPartState::Intact
        } else if damage < 150.0 {
            BodyPartState::Scratched
        } else if damage < 300.0 {
            BodyPartState::Dented
        } else {
            BodyPartState::Crushed
        }
    }

    /// 材質顏色偏移因子（0.0 = 原色，1.0 = 全黑）
    pub fn color_darken_factor(self) -> f32 {
        match self {
            BodyPartState::Intact => 0.0,
            BodyPartState::Scratched => 0.1,
            BodyPartState::Dented => 0.3,
            BodyPartState::Crushed => 0.55,
        }
    }
}

/// 車體部位損壞組件
///
/// 追蹤 6 個車體部位的獨立損壞值和狀態。
/// 碰撞時根據撞擊方向分配傷害到對應部位。
#[derive(Component, Debug)]
pub struct BodyPartDamage {
    /// 各部位累積傷害值
    pub damage: [f32; BODY_PART_COUNT],
    /// 各部位狀態（從 damage 推算）
    pub states: [BodyPartState; BODY_PART_COUNT],
}

impl Default for BodyPartDamage {
    fn default() -> Self {
        Self {
            damage: [0.0; BODY_PART_COUNT],
            states: [BodyPartState::Intact; BODY_PART_COUNT],
        }
    }
}

impl BodyPartDamage {
    /// 對指定部位施加傷害
    pub fn apply_damage(&mut self, part: usize, amount: f32) {
        if part >= BODY_PART_COUNT {
            return;
        }
        self.damage[part] += amount;
        self.states[part] = BodyPartState::from_damage(self.damage[part]);
    }

    /// 根據碰撞方向分配傷害
    ///
    /// `local_dir` 是碰撞方向在車輛座標系中的方向向量：
    /// - +Z = 車頭方向 → 前保險桿 + 引擎蓋
    /// - -Z = 車尾方向 → 後保險桿
    /// - +X = 左側 → 左側板
    /// - -X = 右側 → 右側板
    /// - +Y = 上方 → 車頂
    pub fn apply_directional_damage(&mut self, local_dir: Vec3, amount: f32) {
        // 主要受擊部位得到 60% 傷害，次要部位 40%
        let primary_ratio = 0.6;
        let secondary_ratio = 0.4;

        if local_dir.z.abs() > local_dir.x.abs() && local_dir.z.abs() > local_dir.y.abs() {
            // 前後碰撞
            if local_dir.z > 0.0 {
                // 車頭碰撞
                self.apply_damage(BODY_FRONT_BUMPER, amount * primary_ratio);
                self.apply_damage(BODY_HOOD, amount * secondary_ratio);
            } else {
                // 車尾碰撞
                self.apply_damage(BODY_REAR_BUMPER, amount);
            }
        } else if local_dir.x.abs() > local_dir.y.abs() {
            // 側面碰撞
            if local_dir.x > 0.0 {
                self.apply_damage(BODY_LEFT_PANEL, amount * primary_ratio);
                self.apply_damage(BODY_FRONT_BUMPER, amount * secondary_ratio);
            } else {
                self.apply_damage(BODY_RIGHT_PANEL, amount * primary_ratio);
                self.apply_damage(BODY_REAR_BUMPER, amount * secondary_ratio);
            }
        } else {
            // 上方碰撞（翻車）
            self.apply_damage(BODY_ROOF, amount);
        }
    }

    /// 取得最嚴重的部位損壞等級
    #[allow(dead_code)] // UI 顯示預留
    pub fn worst_state(&self) -> BodyPartState {
        self.states
            .iter()
            .max_by_key(|s| s.severity())
            .copied()
            .unwrap_or(BodyPartState::Intact)
    }

    /// 所有部位的平均損壞因子（0.0-1.0，用於整體材質偏移）
    #[allow(clippy::cast_precision_loss)]
    pub fn average_darken_factor(&self) -> f32 {
        let sum: f32 = self.states.iter().map(|s| s.color_darken_factor()).sum();
        sum / BODY_PART_COUNT as f32
    }

    /// 修復所有部位
    #[allow(dead_code)] // 修車廠系統預留
    pub fn repair_all(&mut self) {
        self.damage = [0.0; BODY_PART_COUNT];
        self.states = [BodyPartState::Intact; BODY_PART_COUNT];
    }

    /// 損壞部位數量
    #[allow(dead_code)] // UI 顯示預留
    pub fn damaged_count(&self) -> usize {
        self.states
            .iter()
            .filter(|s| **s != BodyPartState::Intact)
            .count()
    }
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;
