//! 劇情任務資料結構
//!
//! 定義主線劇情任務的所有資料類型，支援多階段任務、解鎖條件和獎勵系統。
//!
//! 包含：
//! - `types`: 基本類型和狀態
//! - `objectives`: 任務目標
//! - `conditions`: 失敗和解鎖條件
//! - `phases`: 任務階段
//! - `rewards`: 任務獎勵
//! - `mission`: 任務定義
//! - `performance`: 評分系統

// 完整任務系統定義，部分組件預留供未來關卡使用。

mod conditions;
mod mission;
mod objectives;
mod performance;
mod phases;
mod rewards;
mod types;

// 重新導出所有公開項目
pub use conditions::{FailCondition, UnlockCondition};
pub use mission::{ActiveStoryMission, StoryMission};
pub use objectives::{MissionObjective, ObjectiveType};
pub use performance::{MissionCompletionResult, MissionPerformance, StoryMissionRating};
pub use phases::MissionPhase;
pub use rewards::{Difficulty, MissionRewards};
pub use types::{
    AreaId, CutsceneId, DialogueId, NpcId, StoryMissionId, StoryMissionStatus, StoryMissionType,
};
