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
    /// 行人 A* 網格的範圍（找不到規則，照原樣存）
    pub pathfinding_grid: GridSpec,
    /// 小地圖的道路方塊（自成一套，和世界的路段不同；照原樣存）
    pub minimap_roads: Vec<MinimapRoadSpec>,
    /// 有斑馬線的路口：兩條路名（一南北、一東西，順序不拘）
    pub crosswalks: Vec<(String, String)>,
    /// 有號誌的路口：兩條路名（一南北、一東西，順序不拘）
    pub signals: Vec<(String, String)>,
    /// NPC 車路線
    pub npc_routes: Vec<RouteSpec>,
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

/// 行人 A* 網格：原點 (x, z)、格數、每格大小
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GridSpec {
    pub origin: (f32, f32),
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
}

/// 小地圖上的一條路：沿路方向的中心與長度；寬度、位置取自同名的路
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MinimapRoadSpec {
    pub street: String,
    pub center: f32,
    pub length: f32,
}

/// 一條 NPC 車路線：依序經過的轉角
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RouteSpec {
    pub name: String,
    pub corners: Vec<CornerSpec>,
}

/// 路線轉角：兩條路名，各帶一個車道係數
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CornerSpec {
    pub ns: String,
    pub ns_lane: f32,
    pub ew: String,
    pub ew_lane: f32,
}
