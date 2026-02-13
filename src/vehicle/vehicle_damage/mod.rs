//! 載具損壞系統（健康、爆炸、視覺效果）
//!
//! 此模組包含：
//! - `health`: 車輛健康、損壞狀態和輪胎損壞組件
//! - `explosion`: 爆炸效果和系統
//! - `systems`: 碰撞傷害、火焰和事件處理系統
//! - `visuals`: 視覺效果資源和粒子系統

#![allow(dead_code)] // 預留功能模組，等待整合

mod health;
mod explosion;
mod systems;
mod visuals;

// 重新導出公開項目
pub use health::{VehicleDamageState, VehicleHealth, TireDamage};
pub use explosion::{VehicleExplosion, vehicle_explosion_system};
pub use systems::{
    setup_vehicle_damage_effects,
    vehicle_collision_damage_system,
    vehicle_fire_system,
    vehicle_damage_event_system,
};
pub use visuals::{
    VehicleDamageVisuals,
    VehicleDamageSmoke,
    VehicleFire,
    vehicle_damage_effect_system,
    vehicle_damage_particle_update_system,
};
