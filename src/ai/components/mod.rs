//! AI 組件模組
//!
//! 定義 AI 狀態機、感知、巡邏、攻擊、掩體等組件。
//!
//! 包含：
//! - `state`: AI 狀態機
//! - `perception`: 感知系統
//! - `movement`: 巡邏和移動
//! - `combat`: 攻擊和掩體系統
//! - `resources`: 計時器資源

// 完整 AI 系統組件定義，部分組件預留供未來關卡使用。

mod combat;
mod movement;
mod perception;
mod resources;
mod state;

// 重新導出所有公開項目
pub use combat::{AiCombat, CoverPoint, CoverSeeker};
pub use movement::{AiMovement, PatrolPath};
pub use perception::AiPerception;
pub use resources::{EnemySpawnTimer, EnemyTypeAppearance, EnemyVisuals, HairStyle};
#[allow(unused_imports)] // re-export: public API constants for AI awareness thresholds
pub use state::{
    AiBehavior, AiState, AWARENESS_ALERT, AWARENESS_DECAY_RATE, AWARENESS_NOISE_RATE,
    AWARENESS_SUSPICIOUS, AWARENESS_VISUAL_RATE,
};
