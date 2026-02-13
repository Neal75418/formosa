//! AI 計時器資源

use bevy::prelude::*;

/// AI 更新計時器（降低 CPU 負載）
#[derive(Resource)]
pub struct AiUpdateTimer {
    pub perception_timer: Timer,  // 感知更新
    pub decision_timer: Timer,    // 決策更新
}

impl Default for AiUpdateTimer {
    fn default() -> Self {
        Self {
            perception_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            decision_timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }
}

/// 敵人生成計時器
#[derive(Resource)]
pub struct EnemySpawnTimer {
    pub timer: Timer,
    pub max_enemies: usize,
    pub spawn_radius: f32,
}

impl Default for EnemySpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            max_enemies: 10,
            spawn_radius: 70.0,  // 增加到 70m，配合最小生成距離 45m
        }
    }
}
