//! 行人報警系統組件

use bevy::prelude::*;

/// 賄賂費用
pub const BRIBE_COST: i32 = 2000;
/// 賄賂成功後減少的熱度（約降 1 星）
pub const BRIBE_HEAT_REDUCTION: f32 = 20.0;
/// 賄賂互動距離
pub const BRIBE_DISTANCE: f32 = 5.0;

/// 行人報警狀態組件
/// 當行人目擊犯罪時，會進入報警狀態
#[derive(Component, Debug)]
pub struct WitnessState {
    /// 是否已目擊犯罪
    pub witnessed_crime: bool,
    /// 目擊的犯罪類型
    pub crime_type: Option<WitnessedCrime>,
    /// 犯罪發生位置
    pub crime_position: Option<Vec3>,
    /// 報警進度（0.0 - 1.0，達到 1.0 時完成報警）
    pub call_progress: f32,
    /// 報警所需時間（秒）
    pub call_duration: f32,
    /// 報警冷卻（避免同一行人重複報警）
    pub report_cooldown: f32,
    /// 是否已完成報警
    pub has_reported: bool,
    /// 是否已被賄賂
    pub bribed: bool,
}

impl Default for WitnessState {
    fn default() -> Self {
        Self {
            witnessed_crime: false,
            crime_type: None,
            crime_position: None,
            call_progress: 0.0,
            call_duration: 3.0, // 預設 3 秒完成報警
            report_cooldown: 0.0,
            has_reported: false,
            bribed: false,
        }
    }
}

impl WitnessState {
    /// 目擊犯罪
    pub fn witness_crime(&mut self, crime: WitnessedCrime, position: Vec3) {
        // 如果冷卻中或已報警，不重複目擊
        if self.report_cooldown > 0.0 || self.has_reported {
            return;
        }
        self.witnessed_crime = true;
        self.crime_type = Some(crime);
        self.crime_position = Some(position);
        self.call_progress = 0.0;
    }

    /// 更新報警進度
    /// 回傳 true 表示報警完成
    pub fn tick(&mut self, dt: f32) -> bool {
        // 更新冷卻
        if self.report_cooldown > 0.0 {
            self.report_cooldown -= dt;
        }

        // 如果正在報警
        if self.witnessed_crime && !self.has_reported {
            self.call_progress += dt / self.call_duration;
            if self.call_progress >= 1.0 {
                self.call_progress = 1.0;
                self.has_reported = true;
                self.report_cooldown = 60.0; // 60 秒內不會再報警
                return true;
            }
        }
        false
    }

    /// 重置狀態（被打斷或逃跑時）
    pub fn reset(&mut self) {
        self.witnessed_crime = false;
        self.crime_type = None;
        self.crime_position = None;
        self.call_progress = 0.0;
    }

    /// 接受賄賂：停止報警並標記為已賄賂
    pub fn bribe(&mut self) {
        self.bribed = true;
        self.reset();
        self.report_cooldown = 120.0; // 賄賂後 2 分鐘內不會再報警
    }

    /// 是否正在報警中（可被賄賂）
    pub fn is_calling(&self) -> bool {
        self.witnessed_crime && !self.has_reported && !self.bribed
    }
}

/// 目擊的犯罪類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessedCrime {
    /// 開槍（聽到槍聲）
    Gunshot,
    /// 攻擊（看到玩家攻擊行人）
    Assault,
    /// 謀殺（看到玩家殺死行人）
    Murder,
    /// 搶車（看到玩家搶車）
    VehicleTheft,
    /// 撞人（看到玩家開車撞人）
    VehicleHit,
}

impl WitnessedCrime {
    /// 獲取犯罪的嚴重程度（影響報警速度）
    pub fn severity(&self) -> f32 {
        match self {
            WitnessedCrime::Gunshot => 0.8,
            WitnessedCrime::Assault => 0.5,
            WitnessedCrime::Murder => 1.0,
            WitnessedCrime::VehicleTheft => 0.6,
            WitnessedCrime::VehicleHit => 0.9,
        }
    }

    /// 獲取目擊範圍（視覺距離）
    pub fn witness_range(&self) -> f32 {
        match self {
            WitnessedCrime::Gunshot => 30.0, // 聽覺範圍較大
            WitnessedCrime::Assault => 15.0, // 視覺範圍
            WitnessedCrime::Murder => 20.0,  // 視覺範圍（較遠也能看到）
            WitnessedCrime::VehicleTheft => 12.0,
            WitnessedCrime::VehicleHit => 25.0, // 撞擊聲音較大
        }
    }
}

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- WitnessState ---

    #[test]
    fn witness_crime_sets_state() {
        let mut ws = WitnessState::default();
        ws.witness_crime(WitnessedCrime::Gunshot, Vec3::new(1.0, 0.0, 2.0));
        assert!(ws.witnessed_crime);
        assert_eq!(ws.crime_type, Some(WitnessedCrime::Gunshot));
        assert_eq!(ws.call_progress, 0.0);
    }

    #[test]
    fn witness_crime_ignored_on_cooldown() {
        let mut ws = WitnessState {
            report_cooldown: 30.0,
            ..WitnessState::default()
        };
        ws.witness_crime(WitnessedCrime::Assault, Vec3::ZERO);
        assert!(!ws.witnessed_crime);
    }

    #[test]
    fn witness_tick_completes_call() {
        let mut ws = WitnessState::default();
        ws.witness_crime(WitnessedCrime::Murder, Vec3::ZERO);
        assert!(!ws.tick(1.0));
        assert!(!ws.tick(1.0));
        assert!(ws.tick(1.0));
        assert!(ws.has_reported);
        assert!((ws.report_cooldown - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn witness_reset_clears_state() {
        let mut ws = WitnessState::default();
        ws.witness_crime(WitnessedCrime::VehicleTheft, Vec3::ZERO);
        ws.tick(1.0);
        ws.reset();
        assert!(!ws.witnessed_crime);
        assert_eq!(ws.crime_type, None);
        assert_eq!(ws.call_progress, 0.0);
    }

    // --- Bribe ---

    #[test]
    fn bribe_stops_call_and_sets_cooldown() {
        let mut ws = WitnessState::default();
        ws.witness_crime(WitnessedCrime::Murder, Vec3::ZERO);
        ws.tick(1.0); // 部分進度
        assert!(ws.is_calling());

        ws.bribe();
        assert!(ws.bribed);
        assert!(!ws.witnessed_crime);
        assert!(!ws.is_calling());
        assert_eq!(ws.report_cooldown, 120.0);
    }

    #[test]
    fn bribed_witness_ignores_new_crimes() {
        let mut ws = WitnessState::default();
        ws.bribe();
        ws.witness_crime(WitnessedCrime::Gunshot, Vec3::ZERO);
        // report_cooldown > 0，不會再報警
        assert!(!ws.witnessed_crime);
    }

    #[test]
    fn is_calling_false_when_not_witnessed() {
        let ws = WitnessState::default();
        assert!(!ws.is_calling());
    }

    #[test]
    fn is_calling_false_after_report() {
        let mut ws = WitnessState::default();
        ws.witness_crime(WitnessedCrime::Assault, Vec3::ZERO);
        ws.tick(3.0); // 完成報警
        assert!(!ws.is_calling());
    }

    #[test]
    fn bribe_constants() {
        assert_eq!(BRIBE_COST, 2000);
        assert_eq!(BRIBE_HEAT_REDUCTION, 20.0);
        assert_eq!(BRIBE_DISTANCE, 5.0);
    }

    // --- WitnessedCrime ---

    #[test]
    fn crime_severity_ordered() {
        assert!(WitnessedCrime::Murder.severity() > WitnessedCrime::Assault.severity());
        assert!(WitnessedCrime::VehicleHit.severity() > WitnessedCrime::Gunshot.severity());
    }
}
