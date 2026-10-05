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
    /// 路網：每一段路（同一條路可以分好幾段），順序就是生成順序
    pub roads: Vec<RoadSegmentSpec>,
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

/// 道路方向
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadAxis {
    /// 南北向：位置是 X，起訖是 Z
    NorthSouth,
    /// 東西向：位置是 Z，起訖是 X
    EastWest,
}

/// 路面種類
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadKind {
    /// 柏油車行道（兩側有人行道、中央雙黃線）
    Asphalt,
    /// 徒步區鋪面
    Pedestrian,
}

/// 一段路：`at` 是中線位置，`from`／`to` 是沿路方向的起訖
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoadSegmentSpec {
    pub street: String,
    pub axis: RoadAxis,
    pub at: f32,
    pub width: f32,
    pub from: f32,
    pub to: f32,
    pub kind: RoadKind,
}
