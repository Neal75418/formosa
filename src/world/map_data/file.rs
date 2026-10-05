//! 地圖資料檔（RON）的格式：只存「決定」，能推算的交給 `layout`
//!
//! 每個格式型別都加 `deny_unknown_fields`：多寫或拼錯的欄位要在讀檔時報錯，不能默默忽略

use serde::Deserialize;

/// 整份資料檔
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct MapFile {
    /// 可活動範圍
    pub bounds: BoundsSpec,
    /// 玩家出生點 (x, z)
    pub spawn: (f32, f32),
    pub ground: GroundSpec,
    pub walls: WallSpec,
}

/// 可活動範圍（XZ 平面）
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct BoundsSpec {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

/// 地面平板：中心 (x, z) 與碰撞體半長
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct GroundSpec {
    pub center: (f32, f32),
    pub collider_half_extents: (f32, f32, f32),
}

/// 隱形邊界牆：中心離邊界 `offset` 公尺；沿牆方向的中心取地面中心
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct WallSpec {
    pub offset: f32,
    pub half_thickness: f32,
    pub half_height: f32,
    pub center_y: f32,
    /// 東、西兩面牆（沿 Z 延伸）的半長
    pub along_z_half_length: f32,
    /// 南、北兩面牆（沿 X 延伸）的半長
    pub along_x_half_length: f32,
}
