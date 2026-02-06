//! 群體恐慌傳播系統（GTA5 風格）
//!
//! 實現恐慌波傳播機制，包括恐慌波管理和行人恐慌狀態。

use bevy::prelude::*;
use super::components::PedState;

// ============================================================================
// 群體恐慌傳播系統（GTA5 風格）
// ============================================================================

/// 恐慌波常數
const PANIC_WAVE_DEFAULT_MAX_RADIUS: f32 = 15.0;      // 預設最大傳播半徑（米）
const PANIC_WAVE_DEFAULT_SPEED: f32 = 8.0;            // 預設傳播速度（米/秒）
const PANIC_WAVE_GUNSHOT_MAX_RADIUS: f32 = 30.0;      // 槍聲恐慌波最大半徑
const PANIC_WAVE_GUNSHOT_SPEED: f32 = 15.0;           // 槍聲恐慌波傳播速度
const PANIC_WAVE_FRONT_WIDTH: f32 = 2.0;              // 恐慌波前緣寬度
const PANIC_SCREAM_COOLDOWN: f32 = 3.0;               // 尖叫冷卻時間（秒）
const PANIC_SPREAD_THRESHOLD: f32 = 0.7;              // 恐慌傳播閾值（panic_level）
const PANIC_IS_PANICKED_THRESHOLD: f32 = 0.3;         // 判斷「正在恐慌」的閾值

/// 恐慌波檢測結果
#[derive(Clone, Debug)]
pub struct PanicWaveHit {
    /// 恐慌強度
    pub intensity: f32,
    /// 恐慌源位置
    pub source: Vec3,
}

/// 恐慌波管理器資源
/// 管理場上所有活躍的恐慌波
#[derive(Resource, Default)]
pub struct PanicWaveManager {
    /// 活躍的恐慌波列表
    pub active_waves: Vec<PanicWave>,
}

/// 同時存在的最大恐慌波數量
const MAX_ACTIVE_WAVES: usize = 32;

impl PanicWaveManager {
    /// 添加新的恐慌波
    pub fn add_wave(&mut self, origin: Vec3, max_radius: f32, speed: f32, intensity: f32, spawn_time: f32) {
        if self.active_waves.len() >= MAX_ACTIVE_WAVES {
            self.active_waves.remove(0);
        }
        self.active_waves.push(PanicWave {
            origin,
            current_radius: 0.0,
            max_radius,
            propagation_speed: speed,
            intensity,
            spawn_time,
        });
    }

    /// 從槍聲位置創建恐慌波
    pub fn create_from_gunshot(&mut self, position: Vec3, spawn_time: f32) {
        self.add_wave(
            position,
            PANIC_WAVE_GUNSHOT_MAX_RADIUS,
            PANIC_WAVE_GUNSHOT_SPEED,
            1.0,  // 槍聲恐慌強度最高
            spawn_time,
        );
    }

    /// 從行人尖叫位置創建恐慌波
    pub fn create_from_scream(&mut self, position: Vec3, intensity: f32, spawn_time: f32) {
        self.add_wave(
            position,
            PANIC_WAVE_DEFAULT_MAX_RADIUS,
            PANIC_WAVE_DEFAULT_SPEED,
            intensity * 0.8,  // 傳播會衰減
            spawn_time,
        );
    }

    /// 更新所有恐慌波（擴展半徑、清理過期）
    pub fn update(&mut self, delta_time: f32) {
        // 更新所有波的半徑
        for wave in &mut self.active_waves {
            wave.current_radius += wave.propagation_speed * delta_time;
        }

        // 清理已達最大半徑的波
        self.active_waves.retain(|w| w.current_radius < w.max_radius);
    }

    /// 檢查位置是否在任何恐慌波的前緣
    /// 返回最強的恐慌波命中資訊（強度 + 源位置）
    pub fn check_panic_at(&self, position: Vec3) -> Option<PanicWaveHit> {
        let mut best_hit: Option<PanicWaveHit> = None;

        for wave in &self.active_waves {
            let dist_sq = position.distance_squared(wave.origin);
            let outer_radius_sq = wave.current_radius * wave.current_radius;
            let inner_radius = (wave.current_radius - PANIC_WAVE_FRONT_WIDTH).max(0.0);
            let inner_radius_sq = inner_radius * inner_radius;
            // 在恐慌波前緣範圍內
            if dist_sq <= outer_radius_sq && dist_sq > inner_radius_sq {
                match &best_hit {
                    None => {
                        best_hit = Some(PanicWaveHit {
                            intensity: wave.intensity,
                            source: wave.origin,
                        });
                    }
                    Some(current) if wave.intensity > current.intensity => {
                        best_hit = Some(PanicWaveHit {
                            intensity: wave.intensity,
                            source: wave.origin,
                        });
                    }
                    _ => {}
                }
            }
        }

        best_hit
    }
}

/// 單個恐慌波
#[derive(Clone, Debug)]
pub struct PanicWave {
    /// 恐慌源位置
    pub origin: Vec3,
    /// 當前傳播半徑（米）
    pub current_radius: f32,
    /// 最大傳播半徑（米）
    pub max_radius: f32,
    /// 傳播速度（米/秒）
    pub propagation_speed: f32,
    /// 恐慌強度（0.0-1.0，影響逃跑速度和傳播）
    pub intensity: f32,
    /// 創建時間（用於調試）
    pub spawn_time: f32,
}

impl PanicWave {
    /// 創建新的恐慌波
    pub fn new(origin: Vec3, max_radius: f32, speed: f32, intensity: f32, spawn_time: f32) -> Self {
        Self {
            origin,
            current_radius: 0.0,
            max_radius,
            propagation_speed: speed,
            intensity: intensity.clamp(0.0, 1.0),
            spawn_time,
        }
    }

    /// 計算逃跑方向（遠離恐慌源）
    pub fn flee_direction(&self, position: Vec3) -> Vec3 {
        (position - self.origin).normalize_or_zero()
    }
}

/// 行人恐慌狀態組件
/// 追蹤個別行人的恐慌程度和傳播能力
#[derive(Component)]
pub struct PanicState {
    /// 恐慌程度（0.0-1.0）
    pub panic_level: f32,
    /// 恐慌來源方向（用於逃跑）
    pub panic_source: Option<Vec3>,
    /// 尖叫冷卻計時器
    pub scream_cooldown: f32,
    /// 是否可以傳播恐慌（尖叫過一次後設為 false）
    pub can_spread_panic: bool,
    /// 恐慌持續時間（累計被恐慌的時間）
    pub panic_duration: f32,
    /// 恐慌前的狀態（用於恢復）
    pub previous_state: Option<PedState>,
}

impl Default for PanicState {
    fn default() -> Self {
        Self {
            panic_level: 0.0,
            panic_source: None,
            scream_cooldown: 0.0,
            can_spread_panic: true,
            panic_duration: 0.0,
            previous_state: None,
        }
    }
}

impl PanicState {
    /// 觸發恐慌
    pub fn trigger_panic(&mut self, intensity: f32, source: Vec3) {
        self.panic_level = (self.panic_level + intensity).min(1.0);
        self.panic_source = Some(source);
    }

    /// 更新冷卻計時器
    pub fn update(&mut self, delta_time: f32) {
        if self.scream_cooldown > 0.0 {
            self.scream_cooldown -= delta_time;
        }

        if self.panic_level > 0.0 {
            self.panic_duration += delta_time;
        }
    }

    /// 檢查是否可以尖叫傳播恐慌
    pub fn can_scream(&self) -> bool {
        self.panic_level >= PANIC_SPREAD_THRESHOLD
            && self.can_spread_panic
            && self.scream_cooldown <= 0.0
    }

    /// 執行尖叫（傳播恐慌後調用）
    pub fn do_scream(&mut self) {
        self.scream_cooldown = PANIC_SCREAM_COOLDOWN;
        self.can_spread_panic = false;
    }

    /// 逐漸平息恐慌
    pub fn calm_down(&mut self, rate: f32, delta_time: f32) {
        if self.panic_level > 0.0 {
            self.panic_level = (self.panic_level - rate * delta_time).max(0.0);
            if self.panic_level == 0.0 {
                self.panic_source = None;
                self.panic_duration = 0.0;
                // 重置傳播能力（下次恐慌時可以再尖叫）
                self.can_spread_panic = true;
            }
        }
    }

    /// 計算逃跑方向
    pub fn flee_direction(&self, current_pos: Vec3) -> Option<Vec3> {
        self.panic_source.map(|source| (current_pos - source).normalize_or_zero())
    }

    /// 是否處於恐慌狀態
    pub fn is_panicked(&self) -> bool {
        self.panic_level > PANIC_IS_PANICKED_THRESHOLD
    }
}

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === PanicWaveManager 測試 ===

    #[test]
    fn test_panic_wave_manager_add_wave() {
        let mut manager = PanicWaveManager::default();
        assert!(manager.active_waves.is_empty());

        manager.add_wave(Vec3::ZERO, 15.0, 8.0, 1.0, 0.0);
        assert_eq!(manager.active_waves.len(), 1);
        assert_eq!(manager.active_waves[0].current_radius, 0.0);
    }

    #[test]
    fn test_panic_wave_manager_update_expands_radius() {
        let mut manager = PanicWaveManager::default();
        manager.add_wave(Vec3::ZERO, 15.0, 8.0, 1.0, 0.0);

        manager.update(1.0);
        assert_eq!(manager.active_waves[0].current_radius, 8.0);

        manager.update(0.5);
        assert_eq!(manager.active_waves[0].current_radius, 12.0);
    }

    #[test]
    fn test_panic_wave_manager_update_removes_expired() {
        let mut manager = PanicWaveManager::default();
        manager.add_wave(Vec3::ZERO, 10.0, 20.0, 1.0, 0.0);

        // 傳播速度 20m/s，0.6s 後半徑 12m > max 10m，應被移除
        manager.update(0.6);
        assert!(manager.active_waves.is_empty());
    }

    #[test]
    fn test_panic_wave_manager_check_panic_at_in_front() {
        let mut manager = PanicWaveManager::default();
        manager.add_wave(Vec3::ZERO, 30.0, 10.0, 1.0, 0.0);

        // 波前還在原點，10m 處的行人不在波前內
        let result = manager.check_panic_at(Vec3::new(10.0, 0.0, 0.0));
        assert!(result.is_none());

        // 更新 1 秒後，波前在 8m-10m（速度 10m/s，波前寬度 2m）
        manager.update(1.0);
        // 9m 處應在波前範圍內
        let result = manager.check_panic_at(Vec3::new(9.0, 0.0, 0.0));
        assert!(result.is_some());
        assert_eq!(result.unwrap().intensity, 1.0);
    }

    #[test]
    fn test_panic_wave_manager_check_panic_at_behind_front() {
        let mut manager = PanicWaveManager::default();
        manager.add_wave(Vec3::ZERO, 30.0, 10.0, 1.0, 0.0);

        // 更新 2 秒後，波前在 18m-20m
        manager.update(2.0);

        // 5m 處已經被波前通過，不再觸發
        let result = manager.check_panic_at(Vec3::new(5.0, 0.0, 0.0));
        assert!(result.is_none());
    }

    #[test]
    fn test_panic_wave_manager_check_panic_at_best_intensity() {
        let mut manager = PanicWaveManager::default();
        // 兩個波源，不同強度
        manager.add_wave(Vec3::ZERO, 30.0, 10.0, 0.5, 0.0);
        manager.add_wave(Vec3::new(20.0, 0.0, 0.0), 30.0, 10.0, 1.0, 0.0);

        // 更新 1 秒
        manager.update(1.0);

        // 波 2 的 9m 處（即 x=11）在波前內
        let result = manager.check_panic_at(Vec3::new(11.0, 0.0, 0.0));
        assert!(result.is_some());
        assert_eq!(result.unwrap().intensity, 1.0);
    }

    #[test]
    fn test_panic_wave_from_gunshot() {
        let mut manager = PanicWaveManager::default();
        manager.create_from_gunshot(Vec3::new(5.0, 0.0, 5.0), 1.0);

        assert_eq!(manager.active_waves.len(), 1);
        let wave = &manager.active_waves[0];
        assert_eq!(wave.max_radius, PANIC_WAVE_GUNSHOT_MAX_RADIUS);
        assert_eq!(wave.propagation_speed, PANIC_WAVE_GUNSHOT_SPEED);
        assert_eq!(wave.intensity, 1.0);
    }

    #[test]
    fn test_panic_wave_from_scream() {
        let mut manager = PanicWaveManager::default();
        manager.create_from_scream(Vec3::ZERO, 0.8, 1.0);

        let wave = &manager.active_waves[0];
        assert_eq!(wave.max_radius, PANIC_WAVE_DEFAULT_MAX_RADIUS);
        assert_eq!(wave.propagation_speed, PANIC_WAVE_DEFAULT_SPEED);
        assert!((wave.intensity - 0.64).abs() < 0.001); // 0.8 * 0.8
    }

    #[test]
    fn test_panic_wave_manager_max_waves() {
        let mut manager = PanicWaveManager::default();
        // 填滿到上限
        for i in 0..MAX_ACTIVE_WAVES {
            manager.add_wave(Vec3::new(i as f32, 0.0, 0.0), 10.0, 5.0, 1.0, 0.0);
        }
        assert_eq!(manager.active_waves.len(), MAX_ACTIVE_WAVES);

        // 再加一個，應該淘汰最舊的
        manager.add_wave(Vec3::new(999.0, 0.0, 0.0), 10.0, 5.0, 1.0, 0.0);
        assert_eq!(manager.active_waves.len(), MAX_ACTIVE_WAVES);
        // 最新的在末尾
        assert_eq!(manager.active_waves.last().unwrap().origin.x, 999.0);
        // 最舊的（index 0）被移除，現在 index 0 是原來的 index 1
        assert_eq!(manager.active_waves[0].origin.x, 1.0);
    }

    // === PanicState 測試 ===

    #[test]
    fn test_panic_state_default() {
        let state = PanicState::default();
        assert_eq!(state.panic_level, 0.0);
        assert!(state.panic_source.is_none());
        assert!(!state.is_panicked());
        assert!(state.can_spread_panic);
    }

    #[test]
    fn test_panic_state_trigger() {
        let mut state = PanicState::default();
        state.trigger_panic(0.5, Vec3::new(10.0, 0.0, 0.0));

        assert_eq!(state.panic_level, 0.5);
        assert_eq!(state.panic_source, Some(Vec3::new(10.0, 0.0, 0.0)));
        assert!(state.is_panicked());
    }

    #[test]
    fn test_panic_state_trigger_cumulative() {
        let mut state = PanicState::default();
        state.trigger_panic(0.3, Vec3::ZERO);
        state.trigger_panic(0.5, Vec3::ZERO);
        assert!((state.panic_level - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_panic_state_trigger_clamped() {
        let mut state = PanicState::default();
        state.trigger_panic(0.8, Vec3::ZERO);
        state.trigger_panic(0.5, Vec3::ZERO);
        assert_eq!(state.panic_level, 1.0); // 不超過 1.0
    }

    #[test]
    fn test_panic_state_can_scream() {
        let mut state = PanicState::default();
        assert!(!state.can_scream()); // 恐慌度不夠

        state.trigger_panic(0.8, Vec3::ZERO);
        assert!(state.can_scream()); // 恐慌度 >= 0.7

        state.do_scream();
        assert!(!state.can_scream()); // 已經尖叫過
    }

    #[test]
    fn test_panic_state_calm_down() {
        let mut state = PanicState::default();
        state.trigger_panic(1.0, Vec3::ZERO);

        state.calm_down(0.5, 1.0); // 每秒降低 0.5
        assert!((state.panic_level - 0.5).abs() < 0.001);

        state.calm_down(0.5, 2.0); // 再降低 1.0
        assert_eq!(state.panic_level, 0.0);
        assert!(state.panic_source.is_none()); // 完全平息後清除來源
        assert!(state.can_spread_panic); // 重置傳播能力
    }

    #[test]
    fn test_panic_state_flee_direction() {
        let mut state = PanicState::default();
        assert!(state.flee_direction(Vec3::ZERO).is_none());

        state.trigger_panic(1.0, Vec3::new(10.0, 0.0, 0.0));
        let dir = state.flee_direction(Vec3::ZERO).unwrap();
        // 應該遠離恐慌源（負 X 方向）
        assert!(dir.x < 0.0);
    }

    // === PanicWave 測試 ===

    #[test]
    fn test_panic_wave_flee_direction() {
        let wave = PanicWave::new(Vec3::ZERO, 30.0, 10.0, 1.0, 0.0);
        let dir = wave.flee_direction(Vec3::new(5.0, 0.0, 0.0));
        assert!((dir.x - 1.0).abs() < 0.001); // 遠離原點
    }

    #[test]
    fn test_panic_wave_intensity_clamped() {
        let wave = PanicWave::new(Vec3::ZERO, 30.0, 10.0, 1.5, 0.0);
        assert_eq!(wave.intensity, 1.0); // 被 clamp 到 1.0
    }
}
