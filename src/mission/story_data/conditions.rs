//! 失敗和解鎖條件

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use super::StoryMissionId;

/// 失敗條件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FailCondition {
    /// 玩家死亡
    PlayerDeath,
    /// 特定 NPC 死亡
    NpcDeath(String),
    /// 時間耗盡
    TimeExpired,
    /// 車輛被摧毀
    VehicleDestroyed(String),
    /// 被發現（潛入任務）
    Detected,
    /// 目標逃跑
    TargetEscaped,
    /// 離開指定區域（中心點, 半徑）
    ZoneExit(Vec3, f32),
    /// 護送目標血量過低
    EscortHealthLow(f32),
}

/// 解鎖條件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnlockCondition {
    /// 完成指定任務
    CompleteMission(StoryMissionId),
    /// 達到指定章節
    ChapterReached(u32),
    /// 金錢門檻
    MoneyAmount(u32),
    /// 時間範圍（開始小時, 結束小時）
    TimeOfDay(f32, f32),
    /// 劇情標記
    HasFlag(String),
    /// 無條件（總是可接）
    None,
}
