//! 車輛改裝系統
//!
//! 允許玩家購買和安裝車輛改裝，提升性能

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

// 車輛改裝商店 UI 尚未實現，等待商店系統整合。
// 改裝邏輯和數據定義已完成，可通過事件觸發。

mod performance;
mod visuals;
mod systems;

#[cfg(test)]
mod tests;

pub use performance::*;
pub use visuals::*;
pub use systems::*;
