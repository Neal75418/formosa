//! 地圖快照：照遊戲順序跑真的啟動系統，把生成的世界記成排序後的文字，
//! 當地圖資料驅動重構的標準答案（docs/superpowers/specs/2026-10-05-map-data-driven-design.md 第二節）
//!
//! 更新金檔：`UPDATE_SNAPSHOTS=1 cargo test map_snapshot`（寫完檔後測試會刻意失敗，檢查 git diff 後再跑一次）。
//! 看被容差吸收的行數：`cargo test map_snapshot_matches_golden -- --nocapture`（通過的測試預設不顯示輸出）。
//! 升 rand 版本可能改變唐吉訶德招牌的排列（`StdRng` 不保證跨版本相同），那幾行變了要重產金檔

use std::collections::BTreeMap;

use bevy::camera::primitives::MeshAabb;
use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;

use super::components::{FullMapContainer, MinimapContainer};
use crate::ai::CoverPoint;
use crate::pedestrian::{PathfindingGrid, PointsOfInterest};
use crate::vehicle::{NpcVehicle, TrafficLight};
use crate::world::{Building, MapBounds, StreetLight};

const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/world/snapshots/ximending_world.txt"
);

/// 訊息裡顯示的快照檔路徑（相對 repo）
const GOLDEN_DISPLAY: &str = "src/world/snapshots/ximending_world.txt";

/// 數字差在這個範圍內視為相同（四捨五入邊界、跨平台 sin／cos 的 1 ulp 差異）
const TOLERANCE: f64 = 0.002;

/// UI 用的一次性系統：直接呼叫小地圖、大地圖的建立函式（字型給預設 handle 就好）
fn spawn_map_huds(mut commands: Commands, layout: Res<crate::world::MapLayout>) {
    let font = Handle::<Font>::default();
    super::setup_map::setup_minimap_hud(&mut commands, &font, &layout);
    super::setup_map::setup_full_map(&mut commands, &font, &layout);
}

/// 不開視窗的 App：照遊戲的順序跑地圖相關的啟動系統一次
fn snapshot_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
    crate::world::install_map(&mut app);
    app.init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Image>()
        .add_systems(
            Startup,
            (
                crate::world::setup_world,
                crate::vehicle::setup_traffic_lights,
                crate::vehicle::spawn_world_traffic_lights
                    .after(crate::vehicle::setup_traffic_lights),
                crate::vehicle::spawn_initial_traffic.after(crate::world::setup_world),
                crate::pedestrian::setup_pathfinding_grid,
                spawn_map_huds,
            ),
        );
    app.update();
    app
}

/// 四捨五入到 0.001，−0 記成 0
fn num(v: f32) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    let r = if r == 0.0 { 0.0 } else { r };
    format!("{r:.3}")
}

fn vec3(v: Vec3) -> String {
    format!("({},{},{})", num(v.x), num(v.y), num(v.z))
}

fn val(v: Val) -> String {
    match v {
        Val::Px(p) => format!("{}px", num(p)),
        Val::Percent(p) => format!("{}%", num(p)),
        Val::Auto => "auto".to_string(),
        other => format!("{other:?}"),
    }
}

/// 名稱：自己的 Name；沒有的話沿 ChildOf 往上找最近一個 Building（含自己）；都沒有就空字串
fn entity_label(world: &World, entity: Entity) -> String {
    if let Some(name) = world.get::<Name>(entity) {
        return name.as_str().to_string();
    }
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Some(building) = world.get::<Building>(e) {
            return building.name.clone();
        }
        current = world.get::<ChildOf>(e).map(ChildOf::parent);
    }
    String::new()
}

fn collider_shape(c: &Collider) -> String {
    if let Some(v) = c.as_cuboid() {
        return format!("cuboid{}", vec3(v.half_extents()));
    }
    if let Some(v) = c.as_cylinder() {
        return format!("cylinder({},{})", num(v.half_height()), num(v.radius()));
    }
    if let Some(v) = c.as_capsule() {
        return format!("capsule({},{})", num(v.half_height()), num(v.radius()));
    }
    if let Some(v) = c.as_ball() {
        return format!("ball({})", num(v.radius()));
    }
    format!("{:?}", c.raw.shape_type())
}

/// 3D 實體一行：world 座標、旋轉後的 X／Z 軸（不記四元數：q 與 −q 是同一個旋轉）、縮放、mesh 外框、碰撞體、地圖相關元件
fn line_3d(world: &World, entity: Entity, global: &GlobalTransform) -> String {
    let (scale, rotation, translation) = global.to_scale_rotation_translation();
    let mut parts = vec![format!(
        "3d|{}|pos{}|x{}|z{}|scale{}",
        entity_label(world, entity),
        vec3(translation),
        vec3(rotation * Vec3::X),
        vec3(rotation * Vec3::Z),
        vec3(scale)
    )];
    // 父實體的位置與祖先層數：子實體換到不同位置、或不同層的父實體時看得出來
    // （換到同位置、同層的另一個父實體仍看不出來）
    let parent = world
        .get::<ChildOf>(entity)
        .and_then(|c| world.get::<GlobalTransform>(c.parent()));
    if let Some(parent) = parent {
        let depth = std::iter::successors(world.get::<ChildOf>(entity), |c| {
            world.get::<ChildOf>(c.parent())
        })
        .count();
        parts.push(format!(
            "|parent{}|depth{depth}",
            vec3(parent.translation())
        ));
    }
    if let Some(mesh) = world.get::<Mesh3d>(entity) {
        let meshes = world.resource::<Assets<Mesh>>();
        parts.push(match meshes.get(&mesh.0).and_then(MeshAabb::compute_aabb) {
            Some(aabb) => format!(
                "|mesh{}{}",
                vec3(aabb.center.into()),
                vec3(aabb.half_extents.into())
            ),
            None => "|mesh?".to_string(),
        });
    }
    if let Some(collider) = world.get::<Collider>(entity) {
        parts.push(format!("|col:{}", collider_shape(collider)));
    }
    if let Some(building) = world.get::<Building>(entity) {
        parts.push(format!(
            "|bld:{}:{:?}",
            building.name, building.building_type
        ));
    }
    if let Some(light) = world.get::<TrafficLight>(entity) {
        parts.push(format!(
            "|light{}:{}",
            vec3(light.control_direction),
            light.is_primary
        ));
    }
    if let Some(npc) = world.get::<NpcVehicle>(entity) {
        let points: Vec<String> = npc.waypoints.iter().map(|p| vec3(*p)).collect();
        parts.push(format!(
            "|route[{}]@{}",
            points.join(","),
            npc.current_wp_index
        ));
    }
    if let Some(cover) = world.get::<CoverPoint>(entity) {
        parts.push(format!(
            "|cover{}:{}:{}",
            vec3(cover.cover_direction),
            num(cover.height),
            num(cover.damage_reduction)
        ));
    }
    if let Some(light) = world.get::<StreetLight>(entity) {
        parts.push(format!("|streetlight:{}", light.is_on));
    }
    parts.concat()
}

/// UI 節點：深度優先，路徑帶兄弟索引（bevy_ui 依兄弟順序疊放，排序不能抹掉它）
fn walk_ui(world: &World, entity: Entity, path: &str, out: &mut Vec<String>) {
    if let Some(node) = world.get::<Node>(entity) {
        let text = world
            .get::<Text>(entity)
            .map_or_else(String::new, |t| format!("|text:{}", t.0));
        out.push(format!(
            "ui|{path}|l{}|t{}|w{}|h{}{text}",
            val(node.left),
            val(node.top),
            val(node.width),
            val(node.height)
        ));
    }
    if let Some(children) = world.get::<Children>(entity) {
        for (i, child) in children.iter().enumerate() {
            walk_ui(world, child, &format!("{path}/{i}"), out);
        }
    }
}

fn subtree_has<T: Component>(world: &World, entity: Entity) -> bool {
    world.get::<T>(entity).is_some()
        || world
            .get::<Children>(entity)
            .is_some_and(|children| children.iter().any(|c| subtree_has::<T>(world, c)))
}

/// 一列 A* 網格壓縮成「W12 .4 W90」（W 可走、. 不可走，後面是連續格數）
fn walkable_runs(grid: &PathfindingGrid, gz: usize) -> String {
    let mut runs: Vec<(bool, usize)> = Vec::new();
    for gx in 0..grid.width {
        let walkable = grid.is_walkable(gx, gz);
        match runs.last_mut() {
            Some((value, count)) if *value == walkable => *count += 1,
            _ => runs.push((walkable, 1)),
        }
    }
    runs.iter()
        .map(|(value, count)| format!("{}{count}", if *value { 'W' } else { '.' }))
        .collect::<Vec<_>>()
        .join(" ")
}

fn resource_lines(world: &World, out: &mut Vec<String>) {
    if let Some(b) = world.get_resource::<MapBounds>() {
        out.push(format!(
            "res|MapBounds|{},{},{},{}",
            num(b.min_x),
            num(b.max_x),
            num(b.min_z),
            num(b.max_z)
        ));
    }
    if let Some(grid) = world.get_resource::<PathfindingGrid>() {
        out.push(format!(
            "res|grid|origin{}|{}x{}|cell{}",
            vec3(grid.origin),
            grid.width,
            grid.height,
            num(grid.cell_size)
        ));
        for gz in 0..grid.height {
            out.push(format!("res|grid_row|{gz:03}|{}", walkable_runs(grid, gz)));
        }
    }
    if let Some(poi) = world.get_resource::<PointsOfInterest>() {
        for p in &poi.shop_windows {
            out.push(format!("res|poi|shop{}", vec3(*p)));
        }
        for p in &poi.benches {
            out.push(format!("res|poi|bench{}", vec3(*p)));
        }
        for p in &poi.photo_spots {
            out.push(format!("res|poi|photo{}", vec3(*p)));
        }
        for s in &poi.shelters {
            out.push(format!(
                "res|poi|shelter{}|{:?}|{}",
                vec3(s.position),
                s.shelter_type,
                s.capacity
            ));
        }
    }
}

/// 整個世界記成排序後的文字行
fn snapshot_lines(app: &mut App) -> Vec<String> {
    let world = app.world_mut();
    let solids: Vec<(Entity, GlobalTransform)> = world
        .query_filtered::<(Entity, &GlobalTransform), Without<Node>>()
        .iter(world)
        .map(|(e, g)| (e, *g))
        .collect();
    let nodes: Vec<Entity> = world
        .query_filtered::<Entity, With<Node>>()
        .iter(world)
        .collect();
    // 既沒有 GlobalTransform 也不是 UI 節點的實體記不到內容，至少記下數量
    // （三個查詢都套 Bevy 預設過濾：被 Disabled 的實體不在其中）
    let untracked = world
        .query_filtered::<Entity, (Without<GlobalTransform>, Without<Node>)>()
        .iter(world)
        .count();
    let world = app.world();
    let mut lines: Vec<String> = solids.iter().map(|(e, g)| line_3d(world, *e, g)).collect();
    let roots = nodes.iter().copied().filter(|e| {
        world
            .get::<ChildOf>(*e)
            .is_none_or(|parent| world.get::<Node>(parent.parent()).is_none())
    });
    for root in roots {
        let label = if world.get::<FullMapContainer>(root).is_some() {
            "fullmap"
        } else if subtree_has::<MinimapContainer>(world, root) {
            "minimap"
        } else {
            "ui"
        };
        walk_ui(world, root, label, &mut lines);
    }
    resource_lines(world, &mut lines);
    lines.push(format!("res|untracked_entities|{untracked}"));
    lines.sort();
    lines
}

/// 把一行拆成「非數字骨架」與數字清單
fn split_numbers(line: &str) -> (String, Vec<f64>) {
    let chars: Vec<char> = line.chars().collect();
    let mut skeleton = String::new();
    let mut numbers = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let starts_number = chars[i].is_ascii_digit()
            || (chars[i] == '-' && chars.get(i + 1).is_some_and(char::is_ascii_digit));
        if starts_number {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            numbers.push(text.parse().unwrap_or(f64::NAN));
            skeleton.push('#');
        } else {
            skeleton.push(chars[i]);
            i += 1;
        }
    }
    (skeleton, numbers)
}

fn close_enough(a: &str, b: &str) -> bool {
    let (sa, na) = split_numbers(a);
    let (sb, nb) = split_numbers(b);
    sa == sb
        && na.len() == nb.len()
        && na
            .iter()
            .zip(&nb)
            .all(|(x, y)| (x - y).abs() <= TOLERANCE + 1e-9)
}

/// 先以 multiset 逐行比對；剩下對不上的行，結構相同且每個數字差 ≤ TOLERANCE 的視為相同。
/// 通過時回傳被容差吸收的行數。配對採先到先配：只可能誤報（明明有完美配對卻報錯），
/// 不會把超過容差的差異當成相同
fn compare(expected: &[String], actual: &[String]) -> Result<usize, String> {
    let mut counts: BTreeMap<&str, i64> = BTreeMap::new();
    for line in expected {
        *counts.entry(line.as_str()).or_default() += 1;
    }
    for line in actual {
        *counts.entry(line.as_str()).or_default() -= 1;
    }
    let mut missing: Vec<&str> = Vec::new();
    let mut extra: Vec<&str> = Vec::new();
    for (line, n) in counts {
        for _ in 0..n.max(0) {
            missing.push(line);
        }
        for _ in 0..(-n).max(0) {
            extra.push(line);
        }
    }
    let mut absorbed = 0;
    missing.retain(|m| match extra.iter().position(|x| close_enough(m, x)) {
        Some(i) => {
            extra.swap_remove(i);
            absorbed += 1;
            false
        }
        None => true,
    });
    if missing.is_empty() && extra.is_empty() {
        return Ok(absorbed);
    }
    let mut report = vec![format!(
        "少了 {} 行、多了 {} 行",
        missing.len(),
        extra.len()
    )];
    report.extend(missing.iter().take(40).map(|line| format!("- {line}")));
    report.extend(extra.iter().take(40).map(|line| format!("+ {line}")));
    Err(report.join("\n"))
}

enum Verdict {
    Matches { absorbed: usize },
    Rewrite,
    Mismatch(String),
}

/// 更新模式一律重寫（並讓測試失敗）；否則和金檔比對
fn verdict(golden: Option<&str>, actual: &[String], update: bool) -> Verdict {
    if update {
        return Verdict::Rewrite;
    }
    let Some(golden) = golden else {
        return Verdict::Mismatch(format!(
            "找不到快照檔 {GOLDEN_DISPLAY}：先用 UPDATE_SNAPSHOTS=1 產生"
        ));
    };
    let expected: Vec<String> = golden.lines().map(str::to_string).collect();
    match compare(&expected, actual) {
        Ok(absorbed) => Verdict::Matches { absorbed },
        Err(report) => Verdict::Mismatch(report),
    }
}

/// 依判定收尾：相符時回傳被容差吸收的行數；更新模式先寫檔、再一律回報失敗；不符時回報差異
fn conclude(verdict: Verdict, line_count: usize, write: impl FnOnce()) -> Result<usize, String> {
    match verdict {
        Verdict::Matches { absorbed } => Ok(absorbed),
        Verdict::Rewrite => {
            write();
            Err(format!(
                "已重寫 {GOLDEN_DISPLAY}（{line_count} 行）：用 git diff 檢查後，不帶 UPDATE_SNAPSHOTS 再跑一次"
            ))
        }
        Verdict::Mismatch(report) => Err(format!("快照不符：\n{report}")),
    }
}

/// 讀金檔：不存在時回傳 None；其他錯誤（權限、編碼）直接報錯——
/// 避免誤報成「找不到」，引導人跑 UPDATE_SNAPSHOTS=1 把讀不到的檔覆寫掉
fn read_golden_at(path: &str, display: &str) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => panic!("讀不到快照檔 {display}：{e}"),
    }
}

#[test]
fn map_snapshot_matches_golden() {
    let actual = snapshot_lines(&mut snapshot_app());
    let update = std::env::var("UPDATE_SNAPSHOTS").is_ok_and(|v| v == "1");
    let golden = if update {
        None
    } else {
        read_golden_at(GOLDEN, GOLDEN_DISPLAY)
    };
    let result = conclude(
        verdict(golden.as_deref(), &actual, update),
        actual.len(),
        || {
            if let Some(dir) = std::path::Path::new(GOLDEN).parent() {
                std::fs::create_dir_all(dir).expect("建立快照目錄");
            }
            std::fs::write(GOLDEN, actual.join("\n") + "\n").expect("寫入快照檔");
        },
    );
    match result {
        Ok(absorbed) => eprintln!("快照相符；被容差吸收 {absorbed} 行"),
        Err(message) => panic!("{message}"),
    }
}

#[test]
fn map_snapshot_is_deterministic() {
    assert_eq!(
        snapshot_lines(&mut snapshot_app()),
        snapshot_lines(&mut snapshot_app())
    );
}

/// 地圖問題診斷：被略過的建築、彼此重疊的建築、壓到路面的建築、中心不在牆面上的招牌
/// （對照 spec 第四節「不在第二段修」的清單用；只印結果、不斷言）
#[test]
#[ignore = "診斷報告，手動執行：cargo test map_diagnostics_report -- --ignored --nocapture"]
fn map_diagnostics_report() {
    let mut app = snapshot_app();
    let world = app.world_mut();
    let layout = world.resource::<crate::world::MapLayout>().clone();
    // 蓋出來的建築與它們的 XZ 外框（資料檔裡沒有的，例如停車場，用碰撞體推）
    let spawned: Vec<(String, Rect)> = world
        .query::<(&Building, &GlobalTransform, Option<&Collider>)>()
        .iter(world)
        .map(|(b, g, collider)| {
            let half = layout
                .buildings
                .iter()
                .find(|p| p.name == b.name)
                .map(|p| Vec2::new(p.size.x, p.size.z) / 2.0)
                .or_else(|| {
                    collider
                        .and_then(Collider::as_cuboid)
                        .map(|c| Vec2::new(c.half_extents().x, c.half_extents().z))
                })
                .unwrap_or(Vec2::ZERO);
            let c = g.translation();
            (
                b.name.clone(),
                Rect::from_center_half_size(Vec2::new(c.x, c.z), half),
            )
        })
        .collect();
    let overlaps = |a: Rect, b: Rect| {
        a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
    };

    println!("== 因重疊被略過的建築 ==");
    for p in &layout.buildings {
        if !spawned.iter().any(|(n, _)| *n == p.name) {
            println!("{}", p.name);
        }
    }
    println!("== 彼此重疊的建築實體 ==");
    for (i, (a, ra)) in spawned.iter().enumerate() {
        for (b, rb) in &spawned[i + 1..] {
            if overlaps(*ra, *rb) {
                println!("{a} × {b}");
            }
        }
    }
    println!("== 壓到路面的建築 ==");
    for (name, rect) in &spawned {
        for seg in &layout.segments {
            let half_w = seg.width / 2.0;
            let road = match seg.axis {
                crate::world::RoadAxis::NorthSouth => {
                    Rect::new(seg.at - half_w, seg.from, seg.at + half_w, seg.to)
                }
                crate::world::RoadAxis::EastWest => {
                    Rect::new(seg.from, seg.at - half_w, seg.to, seg.at + half_w)
                }
            };
            if overlaps(*rect, road) {
                println!("{name} 壓到 {}", seg.street);
            }
        }
    }
    print_misplaced_signs(world, &spawned);
}

/// 印出中心離最近牆面超過 0.3 m 的招牌（在牆外，或埋在建築裡）
fn print_misplaced_signs(world: &mut World, spawned: &[(String, Rect)]) {
    println!("== 中心離最近牆面超過 0.3 m 的招牌（牆外或牆內）==");
    let signs: Vec<(String, Vec3)> = world
        .query::<(&Name, &GlobalTransform)>()
        .iter(world)
        .filter(|(n, _)| n.as_str().starts_with("NeonSign_"))
        .map(|(n, g)| (n.as_str().to_string(), g.translation()))
        .collect();
    for (name, pos) in signs {
        let Some((building, place, d)) = nearest_wall(spawned, Vec2::new(pos.x, pos.z)) else {
            println!("{name}：沒有任何建築");
            continue;
        };
        if d.abs() <= 0.3 {
            continue;
        }
        let side = if d > 0.0 { "外" } else { "內" };
        // 招牌沒有旋轉，板面朝 ±Z：和南北兩面牆平行、和東西兩面牆垂直；在角落外時不分
        let facing = match place.as_str() {
            "北牆" | "南牆" => "，板面與牆平行",
            "東牆" | "西牆" => "，板面與牆垂直",
            _ => "",
        };
        println!(
            "{name} @ ({:.1}, {:.1})：{building} {place}{side} {:.1} m{facing}",
            pos.x,
            pos.z,
            d.abs()
        );
    }
}

/// 點最靠近哪棟建築的哪裡：點在某棟裡面時就算那棟（取有號距離最小的）
///
/// 回傳 (建築, 位置, 有號距離)；位置是最近的那面牆（「西牆」），同時在兩面牆外時是角落（「西南角」）
fn nearest_wall(spawned: &[(String, Rect)], p: Vec2) -> Option<(&str, String, f32)> {
    let (name, rect, d) = spawned
        .iter()
        .map(|(name, r)| (name.as_str(), *r, signed_distance(*r, p)))
        .min_by(|a, b| a.2.total_cmp(&b.2))?;
    let gap = (rect.min - p).max(p - rect.max);
    let west_east = if p.x < rect.center().x { "西" } else { "東" };
    let north_south = if p.y < rect.center().y { "北" } else { "南" };
    let place = if gap.x > 0.0 && gap.y > 0.0 {
        format!("{west_east}{north_south}角")
    } else if gap.x > gap.y {
        // 牆外看超出的那一軸，牆內看離牆最近的那一軸
        format!("{west_east}牆")
    } else {
        format!("{north_south}牆")
    };
    Some((name, place, d))
}

/// 點到外框的有號距離：在外面是正的（到外框的距離），在裡面是負的（到最近一面牆的距離）
fn signed_distance(r: Rect, p: Vec2) -> f32 {
    let gap = (r.min - p).max(p - r.max);
    gap.max(Vec2::ZERO).length() + gap.x.max(gap.y).min(0.0)
}

#[test]
fn nearest_wall_prefers_the_building_the_point_is_in() {
    // A、B 重疊：點在 A 裡、離 A 的東牆 4 m，離 B 的西牆外只有 2 m，仍要算 A
    let spawned = vec![
        ("B".to_string(), Rect::new(8.0, -20.0, 20.0, 30.0)),
        ("A".to_string(), Rect::new(0.0, 0.0, 10.0, 10.0)),
    ];
    let (name, place, d) = nearest_wall(&spawned, Vec2::new(6.0, 5.0)).unwrap();
    assert_eq!((name, place.as_str(), d), ("A", "東牆", -4.0));
}

#[test]
fn nearest_wall_names_the_corner_when_outside_two_walls() {
    let spawned = vec![("A".to_string(), Rect::new(0.0, 0.0, 10.0, 4.0))];
    // 東南角外：到角的直線距離 5（3-4-5），不是某一面牆
    let (_, place, d) = nearest_wall(&spawned, Vec2::new(13.0, 8.0)).unwrap();
    assert_eq!((place.as_str(), d), ("東南角", 5.0));
    // 只在一面牆外：北牆（−Z）
    let (_, place, d) = nearest_wall(&spawned, Vec2::new(5.0, -2.0)).unwrap();
    assert_eq!((place.as_str(), d), ("北牆", 2.0));
}

#[test]
fn signed_distance_is_negative_inside_and_euclidean_outside() {
    let r = Rect::new(0.0, 0.0, 10.0, 4.0);
    // 裡面：到最近一面牆（北牆 y=0 離 1、南牆離 3、東西牆離 4 以上）
    assert_eq!(signed_distance(r, Vec2::new(5.0, 1.0)), -1.0);
    // 外面：正對一面牆是垂直距離、斜角外是到角的直線距離（3-4-5）
    assert_eq!(signed_distance(r, Vec2::new(12.0, 2.0)), 2.0);
    assert_eq!(signed_distance(r, Vec2::new(13.0, 8.0)), 5.0);
    // 剛好在牆上
    assert_eq!(signed_distance(r, Vec2::new(0.0, 2.0)), 0.0);
}

mod compare_tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn compare_reports_missing_and_extra_lines() {
        let err = compare(&lines("a|1.000\nb|2.000"), &lines("a|1.000\nc|3.000")).unwrap_err();
        assert!(
            err.contains("- b|2.000") && err.contains("+ c|3.000"),
            "{err}"
        );
    }

    #[test]
    fn compare_counts_duplicate_lines() {
        // 同一行少了一份也要看得出來（multiset，不是 set）
        assert!(compare(&lines("a|1.000\na|1.000"), &lines("a|1.000")).is_err());
    }

    #[test]
    fn compare_tolerates_crlf_and_last_digit() {
        // 金檔若被轉成 CRLF、或數字差在 0.002 以內（四捨五入邊界、跨平台 1 ulp），視為相同；走正式的 verdict 判定
        let golden = "pos(1.000,2.000)\r\nx(0.000,0.000,1.000)\r\n";
        let actual = lines("pos(1.001,2.000)\nx(0.000,0.000,1.000)");
        assert!(matches!(
            verdict(Some(golden), &actual, false),
            Verdict::Matches { absorbed: 1 }
        ));
    }

    #[test]
    fn compare_tolerates_sign_flip_near_zero() {
        // −0.0005 附近差 1 ulp：四捨五入後一邊是 −0.001、另一邊是 0.000（數字的負號要算進數字，不算進骨架）
        assert_eq!(compare(&lines("x(-0.001)"), &lines("x(0.000)")), Ok(1));
    }

    #[test]
    fn compare_tolerates_exactly_the_tolerance() {
        assert_eq!(compare(&lines("x(1.000)"), &lines("x(1.002)")), Ok(1));
    }

    #[test]
    fn compare_rejects_differences_beyond_tolerance() {
        assert!(compare(&lines("pos(1.000)"), &lines("pos(1.003)")).is_err());
    }

    #[test]
    fn compare_rejects_same_numbers_with_different_text() {
        assert!(compare(
            &lines("3d|中華路|pos(1.000)"),
            &lines("3d|西寧南路|pos(1.000)")
        )
        .is_err());
    }

    #[test]
    fn num_rounds_and_normalizes_negative_zero() {
        assert_eq!(num(-0.0004), "0.000");
        assert_eq!(num(1.23456), "1.235");
    }

    #[test]
    fn update_mode_always_fails_after_rewrite() {
        // 更新模式一定先寫檔、再回報失敗，避免環境變數一直開著時快照永遠綠
        let mut wrote = false;
        let result = conclude(verdict(Some("a"), &lines("a"), true), 1, || wrote = true);
        assert!(result.is_err() && wrote, "{result:?}");
        let matched = conclude(verdict(Some("a"), &lines("a"), false), 1, || {
            panic!("相符時不該寫檔");
        });
        assert_eq!(matched, Ok(0));
        // 不符、或找不到金檔時也不能寫檔，否則第二次跑就綠了
        let mismatch = conclude(verdict(Some("a"), &lines("b"), false), 1, || {
            panic!("不符時不該寫檔");
        });
        assert!(mismatch.is_err());
        let missing = conclude(verdict(None, &lines("a"), false), 1, || {
            panic!("找不到金檔時不該寫檔");
        });
        assert!(missing.is_err());
    }

    #[test]
    fn missing_golden_reads_as_none() {
        assert_eq!(
            read_golden_at("/nonexistent/map_snapshot.txt", "測試"),
            None
        );
    }

    #[test]
    #[should_panic(expected = "讀不到快照檔 測試")]
    fn unreadable_golden_is_an_error_not_missing() {
        // 讀到的是目錄：不是「不存在」，要直接報錯
        read_golden_at(env!("CARGO_MANIFEST_DIR"), "測試");
    }
}
