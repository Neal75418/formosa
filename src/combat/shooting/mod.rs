//! 射擊系統
//!
//! 處理射擊輸入、武器發射、彈道計算、視覺特效。
//!
//! 子模組：
//! - `auto_aim` — GTA 5 風格自動瞄準/鎖定系統
//! - `input` — 輸入收集、武器切換、換彈
//! - `firing` — 武器發射、瞄準計算、近戰攻擊
//! - `effects` — 彈道拖尾、槍口閃光、揮拳動畫、武器模型

mod auto_aim;
mod effects;
mod firing;
mod input;

// Re-exports — 保持對外 API 不變
pub use auto_aim::auto_aim_system;
pub use effects::{
    bleed_damage_system, bullet_tracer_system, holding_pose_system, impact_effect_system,
    muzzle_flash_system, punch_animation_trigger_system, punch_animation_update_system,
    spawn_bullet_tracer, spawn_muzzle_flash, spawn_player_weapons, weapon_visibility_init_system,
    weapon_visibility_system,
};
pub use firing::fire_weapon_system;
pub use input::{
    melee_combo_timeout_system, reload_system, shooting_input_system, weapon_cooldown_system,
};
