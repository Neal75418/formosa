//! 從路網推算位置的規則與常數（純函式，換地圖時直接沿用）

use bevy::prelude::*;

use super::layout::Junction;

/// 車行道兩側人行道的寬度（全路網共用一個值）
pub const SIDEWALK_WIDTH: f32 = 4.0;

/// 行人逃跑目標離最外圍道路中線的距離
pub const FLEE_INSET: f32 = 5.0;

/// 斑馬線中心離交會路路緣的距離
pub const ZEBRA_CROSSING_OFFSET: f32 = 2.5;

/// 判斷交會時的浮點容差（公尺）
pub const JUNCTION_EPSILON: f32 = 0.01;

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
