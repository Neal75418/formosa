//! NPC 游泳系統
//!
//! 行人落水後進入游泳狀態，體力耗盡後溺水。
//! 行人會嘗試游回最近的岸邊。

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]
// Bevy 系統需要 Res<T> 按值傳遞
#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;

use crate::vehicle::watercraft::WATER_LEVEL;

// ============================================================================
// 游泳組件
// ============================================================================

/// 游泳狀態
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwimState {
    /// 不在水中
    #[default]
    OnLand,
    /// 游泳中
    Swimming,
    /// 溺水中（體力耗盡）
    Drowning,
    /// 已溺斃
    Drowned,
}

/// NPC 游泳組件
#[derive(Component, Clone, Debug)]
pub struct NpcSwimming {
    /// 游泳狀態
    pub state: SwimState,
    /// 游泳體力（0.0-100.0）
    pub stamina: f32,
    /// 最大體力
    pub max_stamina: f32,
    /// 體力消耗速率（/秒）
    pub stamina_drain: f32,
    /// 游泳速度
    pub swim_speed: f32,
    /// 溺水計時器（溺水後幾秒死亡）
    pub drown_timer: f32,
    /// 溺水致死時間
    pub drown_time: f32,
    /// 最近岸邊位置
    pub nearest_shore: Option<Vec3>,
}

impl Default for NpcSwimming {
    fn default() -> Self {
        Self {
            state: SwimState::OnLand,
            stamina: 60.0,
            max_stamina: 60.0,
            stamina_drain: 5.0,   // 每秒消耗 5 體力 → 12 秒耗盡
            swim_speed: 2.0,       // 游泳較慢
            drown_timer: 0.0,
            drown_time: 8.0,       // 溺水後 8 秒死亡
            nearest_shore: None,
        }
    }
}

impl NpcSwimming {
    /// 進入水中
    pub fn enter_water(&mut self) {
        if self.state == SwimState::OnLand {
            self.state = SwimState::Swimming;
        }
    }

    /// 離開水中
    pub fn exit_water(&mut self) {
        self.state = SwimState::OnLand;
        self.drown_timer = 0.0;
        // 上岸後慢慢恢復體力
    }

    /// 更新游泳狀態
    pub fn tick(&mut self, dt: f32) {
        match self.state {
            SwimState::OnLand => {
                // 陸地上恢復體力
                self.stamina = (self.stamina + 10.0 * dt).min(self.max_stamina);
            }
            SwimState::Swimming => {
                // 消耗體力
                self.stamina -= self.stamina_drain * dt;
                if self.stamina <= 0.0 {
                    self.stamina = 0.0;
                    self.state = SwimState::Drowning;
                }
            }
            SwimState::Drowning => {
                // 溺水計時
                self.drown_timer += dt;
                if self.drown_timer >= self.drown_time {
                    self.state = SwimState::Drowned;
                }
            }
            SwimState::Drowned => {
                // 已溺斃，等待 despawn
            }
        }
    }

    /// 體力百分比
    pub fn stamina_ratio(&self) -> f32 {
        self.stamina / self.max_stamina
    }

    /// 是否在水中（游泳或溺水）
    pub fn is_in_water(&self) -> bool {
        matches!(
            self.state,
            SwimState::Swimming | SwimState::Drowning
        )
    }

    /// 是否已死亡
    pub fn is_dead(&self) -> bool {
        self.state == SwimState::Drowned
    }
}

// ============================================================================
// 岸邊定義
// ============================================================================

/// 預定義岸邊位置（用於 NPC 尋路）
pub fn shore_positions() -> Vec<Vec3> {
    vec![
        Vec3::new(-75.0, 1.0, 0.0),
        Vec3::new(85.0, 1.0, -45.0),
        Vec3::new(0.0, 1.0, 85.0),
        Vec3::new(-70.0, 1.0, 30.0),
        Vec3::new(70.0, 1.0, 30.0),
        Vec3::new(-40.0, 1.0, 80.0),
        Vec3::new(40.0, 1.0, 80.0),
    ]
}

/// 找到最近的岸邊
pub fn find_nearest_shore(pos: Vec3) -> Vec3 {
    let shores = shore_positions();
    shores
        .iter()
        .min_by(|a, b| {
            let da = pos.distance_squared(**a);
            let db = pos.distance_squared(**b);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
        .unwrap_or(Vec3::new(0.0, 1.0, 0.0))
}

// ============================================================================
// 系統
// ============================================================================

/// NPC 水中偵測系統
/// 檢查 NPC 是否在水面以下，自動進入游泳狀態
pub fn npc_water_detection_system(
    mut query: Query<(&Transform, &mut NpcSwimming)>,
) {
    for (transform, mut swimming) in &mut query {
        let y = transform.translation.y;

        if y < WATER_LEVEL + 0.5 && !swimming.is_in_water() && !swimming.is_dead() {
            swimming.enter_water();
            swimming.nearest_shore = Some(find_nearest_shore(transform.translation));
        } else if y > WATER_LEVEL + 1.0 && swimming.is_in_water() {
            swimming.exit_water();
        }
    }
}

/// NPC 游泳物理系統
pub fn npc_swim_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut NpcSwimming)>,
) {
    let dt = time.delta_secs();

    for (mut transform, mut swimming) in &mut query {
        swimming.tick(dt);

        match swimming.state {
            SwimState::Swimming => {
                // 保持在水面上
                transform.translation.y = WATER_LEVEL + 0.3;

                // 向最近的岸邊游
                if let Some(shore) = swimming.nearest_shore {
                    let direction = (shore - transform.translation).normalize_or_zero();
                    transform.translation += direction * swimming.swim_speed * dt;

                    // 面朝游泳方向
                    if direction.length_squared() > 0.01 {
                        let target_yaw = direction.x.atan2(direction.z);
                        transform.rotation = Quat::from_rotation_y(target_yaw);
                    }
                }
            }
            SwimState::Drowning => {
                // 溺水時緩慢下沉
                let sink_speed = 0.3;
                transform.translation.y -= sink_speed * dt;

                // 不要沉太深
                transform.translation.y =
                    transform.translation.y.max(WATER_LEVEL - 2.0);
            }
            _ => {}
        }
    }
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swim_state_default() {
        let swim = NpcSwimming::default();
        assert_eq!(swim.state, SwimState::OnLand);
        assert!((swim.stamina - 60.0).abs() < f32::EPSILON);
        assert!(!swim.is_in_water());
    }

    #[test]
    fn enter_and_exit_water() {
        let mut swim = NpcSwimming::default();
        swim.enter_water();
        assert_eq!(swim.state, SwimState::Swimming);
        assert!(swim.is_in_water());

        swim.exit_water();
        assert_eq!(swim.state, SwimState::OnLand);
        assert!(!swim.is_in_water());
    }

    #[test]
    fn swimming_drains_stamina() {
        let mut swim = NpcSwimming::default();
        swim.enter_water();

        swim.tick(1.0); // 1 秒後消耗 5 體力
        assert!((swim.stamina - 55.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stamina_exhaustion_triggers_drowning() {
        let mut swim = NpcSwimming::default();
        swim.enter_water();

        // 游泳 12 秒（60 / 5 = 12），體力耗盡
        swim.tick(12.0);
        assert_eq!(swim.state, SwimState::Drowning);
        assert!(swim.stamina.abs() < f32::EPSILON);
    }

    #[test]
    fn drowning_leads_to_death() {
        let mut swim = NpcSwimming::default();
        swim.enter_water();

        // 耗盡體力
        swim.tick(13.0);
        assert_eq!(swim.state, SwimState::Drowning);

        // 溺水 8 秒後死亡
        swim.tick(8.0);
        assert_eq!(swim.state, SwimState::Drowned);
        assert!(swim.is_dead());
    }

    #[test]
    fn land_recovers_stamina() {
        let mut swim = NpcSwimming { stamina: 30.0, ..Default::default() };

        swim.tick(1.0); // 陸地上恢復 10/秒
        assert!((swim.stamina - 40.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stamina_capped_at_max() {
        let mut swim = NpcSwimming::default();
        swim.tick(100.0); // 大量時間但體力不超過 max
        assert!((swim.stamina - swim.max_stamina).abs() < f32::EPSILON);
    }

    #[test]
    fn find_nearest_shore_works() {
        let shore = find_nearest_shore(Vec3::new(-80.0, 0.0, 0.0));
        // 最近的岸應該是 (-75, 1, 0)
        assert!((shore.x - (-75.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn shore_positions_not_empty() {
        let shores = shore_positions();
        assert!(!shores.is_empty());
        for shore in &shores {
            assert!(shore.y > 0.0, "Shore should be above water");
        }
    }

    #[test]
    fn stamina_ratio() {
        let mut swim = NpcSwimming::default();
        assert!((swim.stamina_ratio() - 1.0).abs() < f32::EPSILON);

        swim.stamina = 30.0;
        assert!((swim.stamina_ratio() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn drowned_cannot_re_enter_water() {
        let mut swim = NpcSwimming { state: SwimState::Drowned, ..Default::default() };

        swim.enter_water(); // 應該無效（only OnLand → Swimming）
        assert_eq!(swim.state, SwimState::Drowned);
    }
}
