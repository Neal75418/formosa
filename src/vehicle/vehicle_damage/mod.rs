//! 載具損壞系統（健康、爆炸、視覺效果）
//!
//! 此模組包含：
//! - `health`: 車輛健康、損壞狀態和輪胎損壞組件
//! - `explosion`: 爆炸效果和系統
//! - `systems`: 碰撞傷害、火焰和事件處理系統
//! - `visuals`: 視覺效果資源和粒子系統

mod health;
mod explosion;
mod systems;
mod visuals;

// 重新導出公開項目
#[allow(unused_imports)] // re-export: public API for vehicle damage state, body parts, and door/window types
pub use health::{
    VehicleDamageState, VehicleHealth, TireDamage, DoorWindowState, DoorState, WindowState,
    BodyPartDamage, BodyPartState, BODY_PART_COUNT,
    BODY_HOOD, BODY_FRONT_BUMPER, BODY_REAR_BUMPER, BODY_LEFT_PANEL, BODY_RIGHT_PANEL, BODY_ROOF,
};
pub use explosion::vehicle_explosion_system;
pub use systems::{
    setup_vehicle_damage_effects,
    vehicle_collision_damage_system,
    vehicle_fire_system,
    vehicle_damage_event_system,
    door_animation_system,
    door_input_system,
    collision_window_damage_system,
    bullet_window_damage_system,
};
pub use visuals::{
    vehicle_damage_effect_system,
    vehicle_damage_particle_update_system,
    body_part_visual_damage_system,
    vehicle_deformation_system,
};
