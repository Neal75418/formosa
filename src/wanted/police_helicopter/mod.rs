//! 警用直升機系統
//!
//! 5 星通緝時出動，追蹤玩家並使用機槍射擊。

mod components;
mod spawning;
mod ai;
mod combat;

pub use spawning::*;
pub use ai::*;
pub use combat::*;
