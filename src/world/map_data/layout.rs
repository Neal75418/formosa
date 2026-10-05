//! 檢查並解析地圖資料檔，產生系統直接取用的 `MapLayout`

use bevy::prelude::*;

use super::file::{MapFile, RoadAxis, RoadKind, RoadSegmentSpec};
use super::geometry::SIDEWALK_WIDTH;
use crate::world::MapBounds;

/// 資料檔的一筆錯誤（訊息指出是哪一筆）
#[derive(Debug, Clone, PartialEq)]
pub struct MapError(pub String);

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 一面邊界牆的碰撞盒
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallBox {
    pub center: Vec3,
    pub half_extents: Vec3,
}

/// 一條路（同名路段合併）：方向、中線位置、寬度
#[derive(Debug, Clone, PartialEq)]
pub struct Street {
    pub name: String,
    pub axis: RoadAxis,
    pub at: f32,
    pub width: f32,
}

/// 解析後的地圖：路名都已換成座標，系統只負責生成
#[derive(Resource, Debug, Clone)]
pub struct MapLayout {
    pub bounds: MapBounds,
    /// 玩家出生點 (x, z)
    pub spawn: Vec2,
    pub ground_center: Vec3,
    pub ground_collider_half_extents: Vec3,
    /// 東、西、南、北
    pub walls: [WallBox; 4],
    /// 路網的每一段，順序同資料檔
    pub segments: Vec<RoadSegmentSpec>,
    /// 同名路段合併成的路，順序依第一次出現
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "A* 網格與小地圖接上路網後才會用到")
    )]
    streets: Vec<Street>,
}

impl MapLayout {
    /// 檢查並解析；有錯時回傳每一筆錯誤
    pub fn from_file(file: &MapFile) -> Result<Self, Vec<MapError>> {
        let mut errors = Vec::new();
        let bounds_ok = check_bounds_and_spawn(file, &mut errors);
        check_segments(file, bounds_ok, &mut errors);
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Self::base(file))
    }

    /// 不需要查路名的部分
    fn base(file: &MapFile) -> Self {
        let b = file.bounds;
        let (ground_x, ground_z) = file.ground.center;
        let (hx, hy, hz) = file.ground.collider_half_extents;
        let w = file.walls;
        let along_z = Vec3::new(w.half_thickness, w.half_height, w.along_z_half_length);
        let along_x = Vec3::new(w.along_x_half_length, w.half_height, w.half_thickness);
        Self {
            bounds: MapBounds {
                min_x: b.min_x,
                max_x: b.max_x,
                min_z: b.min_z,
                max_z: b.max_z,
            },
            spawn: Vec2::new(file.spawn.0, file.spawn.1),
            ground_center: Vec3::new(ground_x, 0.0, ground_z),
            ground_collider_half_extents: Vec3::new(hx, hy, hz),
            walls: [
                WallBox {
                    center: Vec3::new(b.max_x + w.offset, w.center_y, ground_z),
                    half_extents: along_z,
                },
                WallBox {
                    center: Vec3::new(b.min_x - w.offset, w.center_y, ground_z),
                    half_extents: along_z,
                },
                WallBox {
                    center: Vec3::new(ground_x, w.center_y, b.max_z + w.offset),
                    half_extents: along_x,
                },
                WallBox {
                    center: Vec3::new(ground_x, w.center_y, b.min_z - w.offset),
                    half_extents: along_x,
                },
            ],
            segments: file.roads.clone(),
            streets: build_streets(&file.roads),
        }
    }

    /// 所有的路
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "A* 網格與小地圖接上路網後才會用到")
    )]
    pub fn streets(&self) -> &[Street] {
        &self.streets
    }

    /// 依路名取路。路名寫在程式裡（例如「漢中街 + 8」），打錯字時快照測試會在這裡 panic
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "A* 網格與小地圖接上路網後才會用到")
    )]
    pub fn street(&self, name: &str) -> &Street {
        self.find_street(name)
            .unwrap_or_else(|| panic!("地圖沒有「{name}」這條路"))
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "A* 網格與小地圖接上路網後才會用到")
    )]
    fn find_street(&self, name: &str) -> Option<&Street> {
        self.streets.iter().find(|s| s.name == name)
    }
}

/// 邊界要是正的矩形、出生點要在邊界內；回傳邊界本身是否有效
fn check_bounds_and_spawn(file: &MapFile, errors: &mut Vec<MapError>) -> bool {
    let b = file.bounds;
    if !(b.min_x < b.max_x && b.min_z < b.max_z) {
        errors.push(MapError(format!("邊界：min 必須小於 max（{b:?}）")));
        return false;
    }
    let (x, z) = file.spawn;
    if !(b.min_x..=b.max_x).contains(&x) || !(b.min_z..=b.max_z).contains(&z) {
        errors.push(MapError(format!("出生點 ({x}, {z}) 在邊界外")));
    }
    true
}

/// 每段路的數值、同名路段的一致性；邊界無效時不拿它檢查位置（避免連帶誤報）
fn check_segments(file: &MapFile, bounds_ok: bool, errors: &mut Vec<MapError>) {
    let b = file.bounds;
    for (i, seg) in file.roads.iter().enumerate() {
        let tag = format!("路段 #{i}（{}）", seg.street);
        if seg.from >= seg.to {
            errors.push(MapError(format!(
                "{tag}：起點 {} 必須小於終點 {}",
                seg.from, seg.to
            )));
        }
        if seg.width <= 0.0 {
            errors.push(MapError(format!("{tag}：寬度必須大於 0")));
        }
        if seg.kind == RoadKind::Asphalt && seg.width <= SIDEWALK_WIDTH * 2.0 {
            errors.push(MapError(format!(
                "{tag}：柏油路寬 {} 要大於兩側人行道 {}",
                seg.width,
                SIDEWALK_WIDTH * 2.0
            )));
        }
        let (low, high) = match seg.axis {
            RoadAxis::NorthSouth => (b.min_x, b.max_x),
            RoadAxis::EastWest => (b.min_z, b.max_z),
        };
        if bounds_ok && !(low..=high).contains(&seg.at) {
            errors.push(MapError(format!("{tag}：位置 {} 在邊界外", seg.at)));
        }
        // 跟第一段比：build_streets 採用的就是第一段
        let first = file.roads[..i].iter().find(|s| s.street == seg.street);
        if first.is_some_and(|f| (f.axis, f.at, f.width) != (seg.axis, seg.at, seg.width)) {
            errors.push(MapError(format!(
                "{tag}：和同名路段的方向、位置或寬度不一致"
            )));
        }
    }
}

/// 同名路段合併成路，順序依第一次出現
fn build_streets(segments: &[RoadSegmentSpec]) -> Vec<Street> {
    let mut streets: Vec<Street> = Vec::new();
    for seg in segments {
        if !streets.iter().any(|s| s.name == seg.street) {
            streets.push(Street {
                name: seg.street.clone(),
                axis: seg.axis,
                at: seg.at,
                width: seg.width,
            });
        }
    }
    streets
}
