//! 檢查並解析地圖資料檔，產生系統直接取用的 `MapLayout`

use bevy::prelude::*;

use super::file::{
    BuildingEntry, GridSpec, MapFile, MinimapRoadSpec, RoadAxis, RoadKind, RoadSegmentSpec,
    RouteSpec,
};
use super::geometry::{
    along_building_pos, corner_building_pos, lane_offset, segment_reaches, ALONG_BUILDING_HEIGHT,
    FLEE_INSET, JUNCTION_EPSILON, SIDEWALK_WIDTH,
};
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

/// 路口：中心（南北向路的 X、東西向路的 Z）與兩條路的寬度
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Junction {
    pub center: Vec3,
    pub ns_width: f32,
    pub ew_width: f32,
    /// 北、南、西、東：那一側真的有路伸出交會路的路緣（T 字路口有一側是 false）
    pub arms: [bool; 4],
}

/// NPC 車路線：依序經過的點
#[derive(Debug, Clone, PartialEq)]
pub struct NpcRoute {
    pub name: String,
    pub points: Vec<Vec3>,
}

/// 擺好位置的建築：中心與 (寬, 高, 深)
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedBuilding {
    pub name: String,
    pub pos: Vec3,
    pub size: Vec3,
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
    streets: Vec<Street>,
    /// 行人 A* 網格的範圍
    pub grid: GridSpec,
    /// 小地圖的道路方塊
    pub minimap_roads: Vec<MinimapRoadSpec>,
    /// 有斑馬線的路口
    pub crosswalks: Vec<Junction>,
    /// 有號誌的路口
    pub signals: Vec<Junction>,
    /// NPC 車路線
    pub routes: Vec<NpcRoute>,
    pub buildings: Vec<PlacedBuilding>,
}

impl MapLayout {
    /// 檢查並解析，分兩階段：先檢查不需要查路名的部分（邊界、路段、網格），
    /// 都通過才解析路名（小地圖道路、路口……）；每一階段回傳該階段的每一筆錯誤
    pub fn from_file(file: &MapFile) -> Result<Self, Vec<MapError>> {
        let mut errors = Vec::new();
        let bounds_ok = check_bounds_and_spawn(file, &mut errors);
        check_segments(file, bounds_ok, &mut errors);
        check_grid(file, &mut errors);
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut layout = Self::base(file);
        layout.check_minimap_roads(&mut errors);
        layout.crosswalks = layout.resolve_junctions("斑馬線", &file.crosswalks, &mut errors);
        layout.signals = layout.resolve_junctions("號誌", &file.signals, &mut errors);
        layout.routes = layout.resolve_routes(&file.npc_routes, &mut errors);
        layout.buildings = layout.resolve_buildings(&file.buildings, &mut errors);
        if errors.is_empty() {
            Ok(layout)
        } else {
            Err(errors)
        }
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
            grid: file.pathfinding_grid,
            minimap_roads: file.minimap_roads.clone(),
            crosswalks: Vec::new(),
            signals: Vec::new(),
            routes: Vec::new(),
            buildings: Vec::new(),
        }
    }

    /// 所有的路
    pub fn streets(&self) -> &[Street] {
        &self.streets
    }

    /// 依路名取路。路名寫在程式裡（例如「漢中街 + 8」），打錯字時快照測試會在這裡 panic
    pub fn street(&self, name: &str) -> &Street {
        self.find_street(name)
            .unwrap_or_else(|| panic!("地圖沒有「{name}」這條路"))
    }

    fn find_street(&self, name: &str) -> Option<&Street> {
        self.streets.iter().find(|s| s.name == name)
    }

    /// 行人越界即移除的範圍：地圖邊界（x 是世界 X、y 是世界 Z）
    pub fn pedestrian_area(&self) -> Rect {
        let b = &self.bounds;
        Rect::new(b.min_x, b.min_z, b.max_x, b.max_z)
    }

    /// 行人逃跑目標的範圍：越界線再內縮 FLEE_INSET，逃跑目標離越界線留這段緩衝（恐慌逃跑不經過這裡）
    pub fn flee_area(&self) -> Rect {
        self.pedestrian_area().inflate(-FLEE_INSET)
    }

    /// 路北側（−Z）人行道的中線 Z
    pub fn north_sidewalk_z(&self, name: &str) -> f32 {
        let s = self.street(name);
        s.at - (s.width / 2.0 - SIDEWALK_WIDTH / 2.0)
    }

    /// 路的中線位置（南北向是 X、東西向是 Z）
    pub fn at(&self, name: &str) -> f32 {
        self.street(name).at
    }

    /// 路緣：side = −1 是西側／北側，+1 是東側／南側
    pub fn edge(&self, name: &str, side: f32) -> f32 {
        let street = self.street(name);
        street.at + side * street.width / 2.0
    }

    /// 小地圖的路要對得到路網
    fn check_minimap_roads(&self, errors: &mut Vec<MapError>) {
        for (i, road) in self.minimap_roads.iter().enumerate() {
            if self.find_street(&road.street).is_none() {
                errors.push(MapError(format!(
                    "小地圖道路 #{i}：沒有「{}」這條路",
                    road.street
                )));
            }
        }
    }

    /// 兩條路（一南北、一東西，順序不拘）的路口。交點要落在雙方各自某一段的範圍內，
    /// 或離段端不超過另一條路的半寬
    pub fn junction(&self, a: &str, b: &str) -> Result<Junction, String> {
        let first = self
            .find_street(a)
            .ok_or_else(|| format!("沒有「{a}」這條路"))?;
        let second = self
            .find_street(b)
            .ok_or_else(|| format!("沒有「{b}」這條路"))?;
        let (ns, ew) = match (first.axis, second.axis) {
            (RoadAxis::NorthSouth, RoadAxis::EastWest) => (first, second),
            (RoadAxis::EastWest, RoadAxis::NorthSouth) => (second, first),
            _ => return Err(format!("「{a}」和「{b}」不是一南北、一東西")),
        };
        // 只看搆得到這個路口的路段：同名路在別處的另一段不算
        let ns_segments: Vec<&RoadSegmentSpec> = self
            .segments_of(&ns.name)
            .filter(|s| segment_reaches(s.from, s.to, ew.at, ew.width / 2.0))
            .collect();
        let ew_segments: Vec<&RoadSegmentSpec> = self
            .segments_of(&ew.name)
            .filter(|s| segment_reaches(s.from, s.to, ns.at, ns.width / 2.0))
            .collect();
        if !ns_segments.is_empty() && !ew_segments.is_empty() {
            // 那一側有路：某一段伸出交會路的路緣超過浮點容差
            let arms = [
                ns_segments
                    .iter()
                    .any(|s| s.from < ew.at - ew.width / 2.0 - JUNCTION_EPSILON),
                ns_segments
                    .iter()
                    .any(|s| s.to > ew.at + ew.width / 2.0 + JUNCTION_EPSILON),
                ew_segments
                    .iter()
                    .any(|s| s.from < ns.at - ns.width / 2.0 - JUNCTION_EPSILON),
                ew_segments
                    .iter()
                    .any(|s| s.to > ns.at + ns.width / 2.0 + JUNCTION_EPSILON),
            ];
            Ok(Junction {
                center: Vec3::new(ns.at, 0.0, ew.at),
                ns_width: ns.width,
                ew_width: ew.width,
                arms,
            })
        } else {
            Err(format!("「{}」和「{}」沒有交會", ns.name, ew.name))
        }
    }

    fn segments_of<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a RoadSegmentSpec> + 'a {
        self.segments.iter().filter(move |s| s.street == name)
    }

    /// 一串「兩條路名」解析成路口；解析不了或重複的記成錯誤（例如「斑馬線 #2（中華路×成都路）：…」）
    fn resolve_junctions(
        &self,
        kind: &str,
        pairs: &[(String, String)],
        errors: &mut Vec<MapError>,
    ) -> Vec<Junction> {
        let mut junctions: Vec<(usize, Junction)> = Vec::new();
        for (i, (a, b)) in pairs.iter().enumerate() {
            match self.junction(a, b) {
                Ok(j) => match junctions.iter().find(|(_, seen)| seen.center == j.center) {
                    Some((first, _)) => errors.push(MapError(format!(
                        "{kind} #{i}（{a}×{b}）：和 #{first} 是同一個路口"
                    ))),
                    None => junctions.push((i, j)),
                },
                Err(reason) => errors.push(MapError(format!("{kind} #{i}（{a}×{b}）：{reason}"))),
            }
        }
        junctions.into_iter().map(|(_, j)| j).collect()
    }

    /// 依名稱取 NPC 路線（名稱寫在程式裡，打錯字時快照測試會在這裡 panic）
    pub fn route(&self, name: &str) -> &NpcRoute {
        self.routes
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("地圖沒有「{name}」這條 NPC 路線"))
    }

    fn resolve_routes(&self, specs: &[RouteSpec], errors: &mut Vec<MapError>) -> Vec<NpcRoute> {
        let mut routes = Vec::new();
        for (i, spec) in specs.iter().enumerate() {
            // route() 只回傳第一條同名路線，後面同名的永遠拿不到
            if let Some(first) = specs[..i].iter().position(|s| s.name == spec.name) {
                errors.push(MapError(format!(
                    "NPC 路線 #{i}「{}」：和 #{first} 同名",
                    spec.name
                )));
            }
            if spec.corners.len() < 2 {
                errors.push(MapError(format!(
                    "NPC 路線 #{i}「{}」：至少要兩個轉角",
                    spec.name
                )));
                continue;
            }
            let mut points = Vec::new();
            for (k, corner) in spec.corners.iter().enumerate() {
                // 先確認方向：車道係數依欄位套到南北向、東西向的路
                let resolved = self
                    .street_on(&corner.ns, RoadAxis::NorthSouth)
                    .and_then(|_| self.street_on(&corner.ew, RoadAxis::EastWest))
                    .and_then(|_| self.junction(&corner.ns, &corner.ew));
                match resolved {
                    Ok(j) => points.push(Vec3::new(
                        j.center.x + corner.ns_lane * lane_offset(j.ns_width),
                        0.0,
                        j.center.z + corner.ew_lane * lane_offset(j.ew_width),
                    )),
                    Err(reason) => errors.push(MapError(format!(
                        "NPC 路線 #{i}「{}」轉角 #{k}（{}×{}）：{reason}",
                        spec.name, corner.ns, corner.ew
                    ))),
                }
            }
            routes.push(NpcRoute {
                name: spec.name.clone(),
                points,
            });
        }
        routes
    }

    fn resolve_buildings(
        &self,
        entries: &[BuildingEntry],
        errors: &mut Vec<MapError>,
    ) -> Vec<PlacedBuilding> {
        let mut placed = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            match self.place_building(entry) {
                Ok(b) => placed.push(b),
                Err(reason) => {
                    errors.push(MapError(format!("建築 #{i}（{}）：{reason}", entry.name())));
                }
            }
        }
        placed
    }

    fn place_building(&self, entry: &BuildingEntry) -> Result<PlacedBuilding, String> {
        match entry {
            BuildingEntry::Corner {
                name,
                ns,
                ns_side,
                ew,
                ew_side,
                size,
            } => {
                let ns = self.street_on(ns, RoadAxis::NorthSouth)?;
                let ew = self.street_on(ew, RoadAxis::EastWest)?;
                let size = Vec3::new(size.0, size.1, size.2);
                Ok(PlacedBuilding {
                    name: name.clone(),
                    pos: corner_building_pos(ns, *ns_side, ew, *ew_side, size),
                    size,
                })
            }
            BuildingEntry::Along {
                name,
                street,
                side,
                between,
                size,
            } => {
                let main = self
                    .find_street(street)
                    .ok_or_else(|| format!("沒有「{street}」這條路"))?;
                let from = self.junction(street, &between.0)?;
                let to = self.junction(street, &between.1)?;
                let (from_at, to_at) = match main.axis {
                    RoadAxis::NorthSouth => (from.center.z, to.center.z),
                    RoadAxis::EastWest => (from.center.x, to.center.x),
                };
                Ok(PlacedBuilding {
                    name: name.clone(),
                    pos: along_building_pos(main, *side, from_at, to_at, size.0),
                    size: Vec3::new(size.0, ALONG_BUILDING_HEIGHT, size.1),
                })
            }
            BuildingEntry::At { name, pos, size } => Ok(PlacedBuilding {
                name: name.clone(),
                pos: Vec3::new(pos.0, pos.1, pos.2),
                size: Vec3::new(size.0, size.1, size.2),
            }),
        }
    }

    /// 取指定方向的路；名稱不存在或方向不對時回傳原因
    fn street_on(&self, name: &str, axis: RoadAxis) -> Result<&Street, String> {
        let street = self
            .find_street(name)
            .ok_or_else(|| format!("沒有「{name}」這條路"))?;
        if street.axis == axis {
            Ok(street)
        } else {
            Err(format!("「{name}」的方向不對"))
        }
    }
}

/// 邊界要是正的矩形、出生點要在邊界內；回傳邊界本身是否有效
fn check_bounds_and_spawn(file: &MapFile, errors: &mut Vec<MapError>) -> bool {
    let b = file.bounds;
    let finite = [b.min_x, b.max_x, b.min_z, b.max_z]
        .iter()
        .all(|v| v.is_finite());
    if !(finite && b.min_x < b.max_x && b.min_z < b.max_z) {
        errors.push(MapError(format!(
            "邊界：min 必須小於 max，四個值都要是有限數（{b:?}）"
        )));
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
        // 寫成「不是 A」而不是「是 B」：NaN 的比較都是 false，要落在報錯那一邊
        if !(seg.from.is_finite() && seg.to.is_finite() && seg.from < seg.to) {
            errors.push(MapError(format!(
                "{tag}：起點 {} 必須小於終點 {}（都要是有限數）",
                seg.from, seg.to
            )));
        }
        if !(seg.width.is_finite() && seg.width > 0.0) {
            errors.push(MapError(format!(
                "{tag}：寬度必須大於 0 且是有限數（{}）",
                seg.width
            )));
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

/// A* 網格：格子大小是除數，要是有限的正數；格數要大於 0
fn check_grid(file: &MapFile, errors: &mut Vec<MapError>) {
    let g = file.pathfinding_grid;
    if !(g.origin.0.is_finite() && g.origin.1.is_finite()) {
        errors.push(MapError(format!(
            "A* 網格：原點要是有限數（{:?}）",
            g.origin
        )));
    }
    if !g.cell_size.is_finite() || g.cell_size <= 0.0 {
        errors.push(MapError(format!(
            "A* 網格：格子大小要大於 0（{}）",
            g.cell_size
        )));
    }
    if g.width == 0 || g.height == 0 {
        errors.push(MapError(format!(
            "A* 網格：格數要大於 0（{}×{}）",
            g.width, g.height
        )));
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
