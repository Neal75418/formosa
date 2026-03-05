//! 基本類型和狀態

use serde::{Deserialize, Serialize};

/// 劇情任務 ID
pub type StoryMissionId = u32;

/// 對話 ID
pub type DialogueId = u32;

/// 過場動畫 ID
pub type CutsceneId = u32;

/// NPC ID
pub type NpcId = u32;

/// 區域 ID
pub type AreaId = u32;

/// 劇情任務狀態
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum StoryMissionStatus {
    /// 尚未解鎖
    #[default]
    Locked,
    /// 可接取
    Available,
    /// 進行中
    InProgress,
    /// 已完成
    Completed,
    /// 失敗（可重試）
    Failed,
}

/// 劇情任務類型
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum StoryMissionType {
    // === 戰鬥類 ===
    /// 刺殺：消滅特定目標
    Assassination,
    /// 清除：消滅所有敵人
    Elimination,
    /// 防禦：保護位置/人物
    Defend,

    // === 追逐類 ===
    /// 追車：追上並攔截目標
    Chase,
    /// 逃脫：甩開追兵
    Escape,

    // === 護送類 ===
    /// 護送：保護 NPC 到達目的地
    Escort,
    /// 車隊護送：保護車輛
    Convoy,

    // === 潛入類 ===
    /// 潛入：不被發現完成任務
    Stealth,
    /// 滲透：進入敵方據點
    Infiltrate,

    // === 收集/互動類 ===
    /// 取回：取得特定物品
    Retrieve,
    /// 破壞：破壞特定目標
    Sabotage,

    // === 劇情類 ===
    /// 純過場
    Cutscene,
    /// 對話任務
    #[default]
    Dialogue,
}
