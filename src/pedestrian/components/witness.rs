//! 行人報警系統組件

use bevy::prelude::*;

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
}

impl Default for WitnessState {
    fn default() -> Self {
        Self {
            witnessed_crime: false,
            crime_type: None,
            crime_position: None,
            call_progress: 0.0,
            call_duration: 3.0,  // 預設 3 秒完成報警
            report_cooldown: 0.0,
            has_reported: false,
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
            WitnessedCrime::Gunshot => 30.0,   // 聽覺範圍較大
            WitnessedCrime::Assault => 15.0,   // 視覺範圍
            WitnessedCrime::Murder => 20.0,    // 視覺範圍（較遠也能看到）
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
        let mut ws = WitnessState { report_cooldown: 30.0, ..WitnessState::default() };
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

    // --- WitnessedCrime ---

    #[test]
    fn crime_severity_ordered() {
        assert!(WitnessedCrime::Murder.severity() > WitnessedCrime::Assault.severity());
        assert!(WitnessedCrime::VehicleHit.severity() > WitnessedCrime::Gunshot.severity());
    }
}
