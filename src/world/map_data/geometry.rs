//! 從路網推算位置的規則與常數（純函式，換地圖時直接沿用）

use bevy::prelude::*;

use super::layout::{Junction, Street};

/// 車行道兩側人行道的寬度（全路網共用一個值）
pub const SIDEWALK_WIDTH: f32 = 4.0;

/// 行人逃跑目標離越界線（地圖邊界）的距離
pub const FLEE_INSET: f32 = 5.0;

/// 斑馬線中心離交會路路緣的距離
pub const ZEBRA_CROSSING_OFFSET: f32 = 2.5;

/// 判斷交會時的浮點容差（公尺）
pub const JUNCTION_EPSILON: f32 = 0.01;

/// 建築與路緣之間的緩衝距離
pub const BUILDING_ROAD_BUFFER: f32 = 1.5;

/// 沿街建築的固定高度
pub const ALONG_BUILDING_HEIGHT: f32 = 20.0;

/// 路段 [from, to] 是否涵蓋 t；兩端各容許 slack。呼叫端傳另一條路的半寬：
/// 等同兩條路的路面矩形相接（含剛好碰到路緣）
pub fn segment_reaches(from: f32, to: f32, t: f32, slack: f32) -> bool {
    t >= from - slack - JUNCTION_EPSILON && t <= to + slack + JUNCTION_EPSILON
}

/// 一個路口四邊的斑馬線：(中心, 長度, 是否東西向)，順序北、南、西、東
pub fn zebra_crossings(junction: &Junction, y: f32) -> [(Vec3, f32, bool); 4] {
    let c = junction.center;
    let (ns, ew) = (junction.ns_width, junction.ew_width);
    [
        (
            Vec3::new(c.x, y, c.z - ew / 2.0 - ZEBRA_CROSSING_OFFSET),
            ns,
            true,
        ),
        (
            Vec3::new(c.x, y, c.z + ew / 2.0 + ZEBRA_CROSSING_OFFSET),
            ns,
            true,
        ),
        (
            Vec3::new(c.x - ns / 2.0 - ZEBRA_CROSSING_OFFSET, y, c.z),
            ew,
            false,
        ),
        (
            Vec3::new(c.x + ns / 2.0 + ZEBRA_CROSSING_OFFSET, y, c.z),
            ew,
            false,
        ),
    ]
}

/// 雙向車道的中心離道路中線的距離：扣掉兩側人行道後車行道寬的 1/4
pub fn lane_offset(total_width: f32) -> f32 {
    let drive_width = (total_width - SIDEWALK_WIDTH * 2.0).max(0.0);
    drive_width * 0.25
}

/// 路口建築：貼著兩條路的路緣（加緩衝）放在指定的角；size 是 (寬, 高, 深)
pub fn corner_building_pos(
    ns: &Street,
    ns_side: f32,
    ew: &Street,
    ew_side: f32,
    size: Vec3,
) -> Vec3 {
    let x = ns.at + ns_side * (ns.width / 2.0 + size.x / 2.0 + BUILDING_ROAD_BUFFER);
    let z = ew.at + ew_side * (ew.width / 2.0 + size.z / 2.0 + BUILDING_ROAD_BUFFER);
    Vec3::new(x, size.y / 2.0, z)
}

/// 沿街建築：沿著 street 放在兩條橫路中間
///
/// 已知問題：不分道路方向，一律把 street 的位置當 X、兩條橫路當 Z 範圍，沿東西向道路的建築
/// 因此放錯軸；改用建築錨點後整條規則刪除
pub fn along_building_pos(
    street: &Street,
    side: f32,
    from_at: f32,
    to_at: f32,
    width: f32,
) -> Vec3 {
    let x = street.at + side * (street.width / 2.0 + width / 2.0 + BUILDING_ROAD_BUFFER);
    let z = f32::midpoint(from_at, to_at);
    Vec3::new(x, ALONG_BUILDING_HEIGHT / 2.0, z)
}
