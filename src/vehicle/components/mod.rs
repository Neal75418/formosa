//! 載具組件模組
//!
//! 包含：
//! - `core`: 基本類型、ID和材質
//! - `physics`: 物理相關組件（傾斜、煞車、轉向等）
//! - `presets`: 車輛預設配置
//! - `npc`: NPC 車輛相關
//! - `visuals`: 視覺效果組件

// 完整車輛系統組件定義，部分組件預留供未來關卡使用。

mod core;
mod physics;
mod presets;
mod npc;
mod visuals;

// 重新導出所有公開項目
pub use self::core::{VehicleId, VehicleMaterials, VehicleType, VehiclePhysicsMode, Vehicle};
pub use physics::{
    VehicleLean,
    VehiclePowerBand,
    VehicleBraking,
    VehicleSteering,
    VehicleDrift,
    VehicleBodyDynamics,
    VehicleInput,
};
pub use presets::VehiclePreset;
pub use npc::{NpcState, NpcVehicle};
pub use visuals::{
    VehicleVisualRoot,
    VehicleChassisMesh,
    VehicleCabinMesh,
    VehicleOriginalColor,
    TireTrack,
    DriftSmoke,
    NitroFlame,
    VehicleEffectVisuals,
    VehicleEffectTracker,
};
