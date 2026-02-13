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
#![allow(dead_code)]

mod types;
mod objectives;
mod conditions;
mod phases;
mod rewards;
mod mission;
mod performance;

// 重新導出所有公開項目
pub use types::{
    StoryMissionId, DialogueId, CutsceneId, NpcId, AreaId,
    StoryMissionStatus, StoryMissionType
};
pub use objectives::{ObjectiveType, MissionObjective};
pub use conditions::{FailCondition, UnlockCondition};
pub use phases::{EnemySpawnData, NpcSpawnData, MissionPhase};
pub use rewards::{Difficulty, MissionRewards};
pub use mission::{StoryMission, ActiveStoryMission};
pub use performance::{StoryMissionRating, MissionPerformance, MissionCompletionResult};
