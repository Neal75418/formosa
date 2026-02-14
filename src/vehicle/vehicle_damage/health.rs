//! 車輛健康和損壞狀態組件


use bevy::prelude::*;
use super::super::VehicleType;

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
        self.current = new_max * health_ratio;  // 保持相同的血量比例
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
        debug_assert!(flat_count <= 4, "flat_count 超出範圍: {}", flat_count);

        self.handling_penalty = match flat_count {
            0 => 0.0,
            1 => 0.15,  // 一個爆胎：輕微影響
            2 => 0.30,  // 兩個爆胎：明顯影響
            3 => 0.45,  // 三個爆胎：嚴重影響
            _ => 0.60,  // 全部爆胎：困難但可操控
        };
        self.speed_penalty = match flat_count {
            0 => 0.0,
            1 => 0.10,  // 速度降低 10%
            2 => 0.20,  // 速度降低 20%
            3 => 0.35,  // 速度降低 35%
            _ => 0.50,  // 速度降低 50%
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
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- VehicleDamageState ---

    #[test]
    fn damage_state_from_health_percent_boundaries() {
        assert_eq!(VehicleDamageState::from_health_percent(1.0), VehicleDamageState::Pristine);
        assert_eq!(VehicleDamageState::from_health_percent(1.5), VehicleDamageState::Pristine);
        assert_eq!(VehicleDamageState::from_health_percent(0.75), VehicleDamageState::Light);
        assert_eq!(VehicleDamageState::from_health_percent(0.50), VehicleDamageState::Moderate);
        assert_eq!(VehicleDamageState::from_health_percent(0.25), VehicleDamageState::Heavy);
        assert_eq!(VehicleDamageState::from_health_percent(0.10), VehicleDamageState::Critical);
        assert_eq!(VehicleDamageState::from_health_percent(0.0), VehicleDamageState::Destroyed);
    }

    // --- VehicleHealth ---

    #[test]
    fn health_new_sets_max_and_current() {
        let h = VehicleHealth::new(500.0);
        assert_eq!(h.current, 500.0);
        assert_eq!(h.max, 500.0);
        assert_eq!(h.percentage(), 1.0);
        assert!(!h.is_destroyed());
    }

    #[test]
    fn health_for_vehicle_type_correct_values() {
        assert_eq!(VehicleHealth::for_vehicle_type(VehicleType::Scooter).max, 500.0);
        assert_eq!(VehicleHealth::for_vehicle_type(VehicleType::Car).max, 1000.0);
        assert_eq!(VehicleHealth::for_vehicle_type(VehicleType::Taxi).max, 1200.0);
        assert_eq!(VehicleHealth::for_vehicle_type(VehicleType::Bus).max, 2000.0);
    }

    #[test]
    fn health_take_damage_reduces_hp() {
        let mut h = VehicleHealth::new(100.0);
        let actual = h.take_damage(30.0, 1.0);
        assert!((actual - 30.0).abs() < f32::EPSILON);
        assert!((h.current - 70.0).abs() < f32::EPSILON);
        assert!((h.percentage() - 0.7).abs() < 0.01);
    }

    #[test]
    fn health_take_damage_clamps_to_zero() {
        let mut h = VehicleHealth::new(50.0);
        let actual = h.take_damage(200.0, 1.0);
        assert!((actual - 50.0).abs() < f32::EPSILON);
        assert_eq!(h.current, 0.0);
        assert_eq!(h.damage_state, VehicleDamageState::Destroyed);
    }

    #[test]
    fn health_take_damage_invulnerable_returns_zero() {
        let mut h = VehicleHealth::new(100.0);
        h.is_invulnerable = true;
        let actual = h.take_damage(50.0, 1.0);
        assert_eq!(actual, 0.0);
        assert_eq!(h.current, 100.0);
    }

    #[test]
    fn health_take_damage_critical_triggers_fire() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(80.0, 1.0);
        assert!(h.is_on_fire);
        assert_eq!(h.damage_state, VehicleDamageState::Critical);
    }

    #[test]
    fn health_repair_increases_hp() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(60.0, 1.0);
        h.repair(30.0);
        assert!((h.current - 70.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_repair_clamps_to_max() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(20.0, 1.0);
        h.repair(500.0);
        assert_eq!(h.current, 100.0);
    }

    #[test]
    fn health_repair_extinguishes_fire_above_threshold() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(85.0, 1.0);
        assert!(h.is_on_fire);
        h.repair(60.0);
        assert!(!h.is_on_fire);
        assert!(h.percentage() > 0.3);
    }

    #[test]
    fn health_repair_does_nothing_when_destroyed() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(100.0, 1.0);
        assert!(h.is_destroyed());
        h.repair(50.0);
        assert_eq!(h.current, 0.0);
    }

    #[test]
    fn health_full_repair_restores_everything() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(80.0, 1.0);
        h.full_repair();
        assert_eq!(h.current, 100.0);
        assert_eq!(h.damage_state, VehicleDamageState::Pristine);
        assert!(!h.is_on_fire);
    }

    #[test]
    fn health_apply_armor_upgrade_preserves_ratio() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(50.0, 1.0);
        assert!((h.percentage() - 0.5).abs() < 0.01);
        h.apply_armor_upgrade(1.5);
        assert!((h.max - 150.0).abs() < f32::EPSILON);
        assert!((h.percentage() - 0.5).abs() < 0.01);
    }

    #[test]
    fn health_tick_fire_countdown_explodes() {
        let mut h = VehicleHealth::new(1000.0);
        h.is_on_fire = true;
        h.fire_timer = 1.0;
        assert!(h.tick_fire(1.5));
        assert_eq!(h.damage_state, VehicleDamageState::Destroyed);
    }

    #[test]
    fn health_tick_fire_burn_damage() {
        let mut h = VehicleHealth::new(100.0);
        h.is_on_fire = true;
        h.fire_timer = 10.0;
        h.tick_fire(1.0);
        assert!((h.current - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_tick_fire_not_on_fire_returns_false() {
        let mut h = VehicleHealth::new(100.0);
        assert!(!h.tick_fire(1.0));
        assert_eq!(h.current, 100.0);
    }

    // --- TireDamage ---

    #[test]
    fn tire_pop_and_count() {
        let mut td = TireDamage::default();
        assert_eq!(td.flat_count(), 0);
        td.pop_tire(0);
        assert_eq!(td.flat_count(), 1);
        assert!(td.has_front_flat());
        assert!(!td.has_rear_flat());
    }

    #[test]
    fn tire_pop_rear_detected() {
        let mut td = TireDamage::default();
        td.pop_tire(2);
        assert!(!td.has_front_flat());
        assert!(td.has_rear_flat());
    }

    #[test]
    fn tire_repair_single() {
        let mut td = TireDamage::default();
        td.pop_tire(1);
        td.pop_tire(3);
        assert_eq!(td.flat_count(), 2);
        td.repair_tire(1);
        assert_eq!(td.flat_count(), 1);
    }

    #[test]
    fn tire_repair_all_resets() {
        let mut td = TireDamage::default();
        td.pop_tire(0);
        td.pop_tire(1);
        td.pop_tire(2);
        td.repair_all();
        assert_eq!(td.flat_count(), 0);
        assert_eq!(td.handling_penalty, 0.0);
        assert_eq!(td.speed_penalty, 0.0);
    }

    #[test]
    fn tire_penalties_scale_with_count() {
        let mut td = TireDamage::default();
        td.pop_tire(0);
        assert!((td.handling_penalty - 0.15).abs() < f32::EPSILON);
        assert!((td.speed_penalty - 0.10).abs() < f32::EPSILON);
        td.pop_tire(1);
        assert!((td.handling_penalty - 0.30).abs() < f32::EPSILON);
        assert!((td.speed_penalty - 0.20).abs() < f32::EPSILON);
    }

    #[test]
    fn tire_out_of_bounds_ignored() {
        let mut td = TireDamage::default();
        td.pop_tire(99);
        assert_eq!(td.flat_count(), 0);
    }
}
