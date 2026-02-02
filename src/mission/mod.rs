//! 任務系統模組
//!
//! 包含：
//! - 一般任務（Delivery, Taxi, Race, Explore）
//! - 劇情任務（Story Missions）
//! - 對話系統（Dialogue System）
//! - 過場動畫（Cutscene System）
//!
//! 注意：部分任務類型為將來擴展預留

// 一般任務系統
mod data;
mod systems;

// 劇情任務系統
mod cutscene;
mod cutscene_systems;
mod dialogue;
mod dialogue_systems;
mod dialogue_ui;
mod economy;
mod relationship;
mod story_data;
mod story_manager;
mod story_systems;
mod trigger;
mod unlocks;

// 重新導出一般任務
pub use data::*;
pub use systems::*;

// 重新導出劇情任務
pub use cutscene_systems::*;
pub use dialogue_systems::*;
pub use dialogue_ui::*;
pub use economy::*;
pub use relationship::*;
pub use story_manager::*;
pub use story_systems::*;
pub use trigger::*;
pub use unlocks::*;
