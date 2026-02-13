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
#![allow(dead_code)]

mod state;
mod perception;
mod movement;
mod combat;
mod resources;

// 重新導出所有公開項目
pub use state::{AiState, AiBehavior};
pub use perception::AiPerception;
pub use movement::{PatrolPath, AiMovement};
pub use combat::{AiCombat, CoverType, CoverPoint, CoverSeeker};
pub use resources::{AiUpdateTimer, EnemySpawnTimer};
