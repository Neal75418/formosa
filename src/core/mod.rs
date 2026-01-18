//! 核心系統模組（資源、事件、遊戲狀態）
//!
//! 注意：部分資源欄位為將來擴展預留

#![allow(dead_code)]

mod resources;
mod events;

pub use resources::*;
pub use events::*;
