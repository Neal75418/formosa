//! 行人組件模組
//!
//! 定義行人 NPC 的組件、狀態和資源。
//!
//! 包含：
//! - `core`: 基本組件和狀態
//! - `witness`: 報警系統
//! - `appearance`: 外觀配置
//! - `resources`: 資源（配置、視覺、路徑）
//! - `animation`: 動畫組件
//! - `collision`: 碰撞組件

// 完整行人系統組件定義，部分組件預留供未來關卡使用。

mod core;
mod witness;
mod appearance;
mod resources;
mod animation;
mod collision;

// 重新導出所有公開項目
pub use self::core::{Pedestrian, PedestrianType, PedestrianState, PedState};
pub use witness::{WitnessState, WitnessedCrime, BRIBE_COST, BRIBE_DISTANCE, BRIBE_HEAT_REDUCTION};
pub use resources::{
    PedestrianConfig,
    PedestrianPaths,
    SidewalkPath,
    GunshotTracker,
    PedestrianVisuals,
};
pub use animation::{PedestrianLeg, PedestrianArm, WalkingAnimation};
pub use collision::HitByVehicle;
