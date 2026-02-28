//! 車輛改裝系統
//!
//! 允許玩家購買和安裝車輛改裝，提升性能

// 部分功能（視覺改裝）為將來擴展預留
#![allow(dead_code)]

mod performance;
mod visuals;
mod systems;

#[cfg(test)]
mod tests;

pub use performance::*;
pub use visuals::*;
pub use systems::*;
