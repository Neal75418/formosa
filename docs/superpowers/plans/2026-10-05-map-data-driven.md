# 西門町第二段：地圖改資料驅動 — 實作計畫

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把散在各系統的地圖數字收進 `assets/levels/ximending.ron`，各系統改從解析後的 `MapLayout` 取座標；重構期間生成的世界零變化，之後修小地圖的兩個 bug 與行人越界線。

**Architecture:** 新模組 `src/world/map_data/`：`file.rs`（RON 格式，只存「決定」）、`layout.rs`（檢查＋一次解析成 `MapLayout` resource，路名都換成座標）、`geometry.rs`（從路網推算位置的純函式）。`install_map` 在 `WorldPlugin::build` 插入 `MapLayout` 與 `MapBounds`，所有 Startup／Update 系統讀 resource。重構前先用快照測試（不開視窗跑真的啟動系統，記錄世界座標）與執行期行為測試記下標準答案，每個重構 task 都要和它相符。

**Tech Stack:** Rust 2021、Bevy 0.17.3、bevy_rapier3d 0.32、serde（RON 用 `bevy::asset::ron` re-export，ron 0.10.1）、rand 0.9.2

**Spec:** `docs/superpowers/specs/2026-10-05-map-data-driven-design.md`

## Global Constraints

- 不新增依賴：RON 用 `bevy::asset::ron`（`bevy_asset` 的 re-export），不改 `Cargo.toml`
- 資料檔 `assets/levels/ximending.ron`，以 `include_str!` 編進執行檔
- 資料只存「決定」，能推算的交給程式；現在找不到規則的數字照原樣存（A* 網格範圍、牆的長度厚度高度、地面中心與碰撞體、小地圖道路方塊與地標偏移）
- 單檔不超過 800 行；註解與文件用繁體中文、技術詞保留英文；註解不寫開發過程
- 一個 commit 只做一件事：純重構（快照不變），或修一個 bug（快照刻意改變）
- 執行期行為測試的期望值一律寫字面數字
- 每個 task 完成的條件：先跑 `cargo fmt`（計畫裡的程式碼不保證已是 rustfmt 排版），再 `cargo test`、`cargo clippy --all-targets --all-features -- -D warnings`（CI 同款；`Cargo.toml` 開了 pedantic，警告即錯誤）、`cargo fmt --check` 全過；重構 task 的快照比對通過，並回報「被容差吸收」的行數（應為 0，非 0 要查原因）
- 每個 task 送 code-reviewer 審過才提交；commit 由 user 說「提交」才做，訊息 Conventional Commits、繁體中文、不加任何署名行；`git commit -F <訊息檔> -- <明確路徑>`；不 push
- 改數字做突變驗證時，用編輯還原，不用 `git checkout`

## Review Focus

1. **遊戲本體沒裝地圖**：`WorldPlugin::build` 沒呼叫 `install_map`（或順序錯），單元測試全走 `install_map` 看不到，遊戲一啟動就因缺 `MapLayout` panic → Task 4 的 `world_plugin_installs_map_layout`
2. **交會容差剛好落在半寬**：現有資料有多處交點離段端「剛好等於」對方半寬（6、7.5、8 m），浮點下可能被誤判成不交會 → Task 8 的 `junction_accepts_exact_half_width_and_rejects_beyond`
3. **金檔在 Ubuntu CI 上紅**：CRLF 換行或末位數字的 1 ulp 差異 → Task 2 的 `compare_tolerates_crlf_and_last_digit`
4. **`UPDATE_SNAPSHOTS` 一直開著，快照永遠綠** → Task 2 的 `update_mode_always_fails_after_rewrite`（更新模式寫完檔一定讓測試失敗）
5. **行人越界線放寬到地圖邊界後，行人走到 A* 網格外**（網格 X 從 −110 起，邊界 −119）→ Task 16 的 `pedestrian_outside_grid_falls_back`

---

## 檔案地圖

| 檔案 | 動作 | 職責 |
|---|---|---|
| `assets/levels/ximending.ron` | 新增（Task 4 起逐步加段落） | 地圖的「決定」 |
| `src/world/map_data/mod.rs` | 新增 | 載入、`install_map`、re-export |
| `src/world/map_data/file.rs` | 新增 | RON 格式（serde） |
| `src/world/map_data/layout.rs` | 新增 | `MapLayout`、檢查、解析、查詢 |
| `src/world/map_data/geometry.rs` | 新增（Task 5） | 推算規則與常數（人行道寬、車道偏移、建築位置、斑馬線） |
| `src/world/map_data/tests.rs` | 新增 | 資料檔與檢查規則的測試 |
| `src/ui/map_snapshot.rs` | 新增（Task 2） | 快照測試、比對、診斷報告（`#[cfg(test)]`） |
| `src/world/snapshots/ximending_world.txt` | 新增（Task 2） | 快照金檔 |
| `src/ui/map_marker_tests.rs` | 新增（Task 3） | 小地圖／大地圖／GPS 標記的行為測試（`minimap.rs` 已 732 行，測試另放） |
| `src/ui/map_projection.rs` | 新增（Task 7） | 世界 → 小地圖／大地圖投影（唯一一份） |
| `src/world/constants.rs` | 修改（Task 4、8、11、13） | 刪除道路、出生點常數與 `impl Default for MapBounds` |
| 各系統檔 | 修改 | 改讀 `MapLayout`（見各 task） |

測試命名：`cargo test map_snapshot` 跑快照；`cargo test map_data` 跑資料檔測試。

---

### Task 1：前置——唐吉訶德招牌與夾娃娃機燈球改用固定種子

**Files:**
- Modify: `src/world/buildings/entertainment.rs:116-120`（`spawn_donki` 的 `rand::rng()`）、`:270-282`（`spawn_claw_machine` 的 `rand::rng()`）
- Test: `src/world/buildings/entertainment.rs`（檔尾新增 `#[cfg(test)] mod tests`）

**Interfaces:**
- Consumes: `super::name_hash(name: &str) -> u32`（`facade.rs`，經 `pub use facade::*`）
- Produces: 無新介面；之後的快照測試靠它穩定

- [ ] **Step 1：寫失敗的測試**（檔尾加入）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// 生成一棟樓，回傳所有子實體的 Transform（排序後的文字）
    fn child_transforms(
        spawn: impl Fn(&mut Commands, &mut ResMut<Assets<Mesh>>, &mut ResMut<Assets<StandardMaterial>>)
            + Send
            + Sync
            + 'static,
    ) -> Vec<String> {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<Image>()
            .add_systems(
                Startup,
                move |mut commands: Commands,
                      mut meshes: ResMut<Assets<Mesh>>,
                      mut materials: ResMut<Assets<StandardMaterial>>| {
                    spawn(&mut commands, &mut meshes, &mut materials);
                },
            );
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<&Transform, With<ChildOf>>();
        let mut out: Vec<String> = query
            .iter(world)
            .map(|t| format!("{:?}", (t.translation, t.rotation, t.scale)))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn donki_signs_same_every_launch() {
        let donki = || {
            child_transforms(|c, m, s| {
                spawn_donki(c, m, s, Vec3::ZERO, 28.0, 35.0, 22.0, "Don Don Donki");
            })
        };
        assert_eq!(donki(), donki());
    }

    #[test]
    fn claw_machine_lights_same_every_launch() {
        let claw = || {
            child_transforms(|c, m, s| {
                spawn_claw_machine(c, m, s, Vec3::ZERO, 8.0, 10.0, 8.0, "夾娃娃機");
            })
        };
        assert_eq!(claw(), claw());
    }
}
```

- [ ] **Step 2：跑測試確認失敗**

Run: `cargo test entertainment::tests`
Expected: 兩條都 FAIL（`assertion left == right failed`，兩次的招牌位置不同）

- [ ] **Step 3：改用固定種子**

`spawn_donki` 的閉包開頭 `use rand::Rng;` 改成 `use rand::{Rng, SeedableRng};`，`let mut rng = rand::rng();` 改成：

```rust
        // 以店名雜湊當種子：招牌排列每次啟動都一樣
        let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(super::name_hash(name)));
```

`spawn_claw_machine` 同樣兩處照改（註解改成「燈球排列每次啟動都一樣」）。

- [ ] **Step 4：跑測試確認通過**

Run: `cargo test entertainment::tests`
Expected: 2 passed

- [ ] **Step 5：全套驗證**

Run: `cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過

- [ ] **Step 6：送 code-reviewer 審 `git diff`，修完後等 user 說「提交」**

訊息檔內容：

```
refactor(world): 唐吉訶德招牌與夾娃娃機燈球改用店名雜湊當亂數種子

- 每次啟動排列固定，地圖快照才穩定（第二段重構的前置）
```

```bash
git commit -F <訊息檔> -- src/world/buildings/entertainment.rs
```

---

### Task 2：第 0 步之一——地圖快照測試與金檔

**Files:**
- Create: `src/ui/map_snapshot.rs`
- Create: `src/world/snapshots/ximending_world.txt`（由測試產生）
- Modify: `src/ui/mod.rs`（在 `#[cfg(test)] mod tests;` 旁加 `#[cfg(test)] mod map_snapshot;`）

**Interfaces:**
- Consumes: `crate::world::setup_world`、`crate::vehicle::{setup_traffic_lights, spawn_world_traffic_lights, spawn_initial_traffic, NpcVehicle, TrafficLight}`、`crate::pedestrian::{setup_pathfinding_grid, PathfindingGrid, PointsOfInterest}`、`super::setup_map::{setup_minimap_hud, setup_full_map}`（`pub(super)`，所以測試放在 `ui` 底下）
- Produces（後續 task 會改到）：`fn snapshot_app() -> App`、`fn snapshot_lines(app: &mut App) -> Vec<String>`、`fn compare(expected: &[String], actual: &[String]) -> Result<usize, String>`

- [ ] **Step 1：先寫比對邏輯的測試**（`src/ui/map_snapshot.rs` 檔尾；此時檔案只有這段，`compare`、`num`、`verdict` 還不存在，編譯失敗就是紅）

```rust
#[cfg(test)]
mod compare_tests {
    use super::*;

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_string).collect()
    }

    #[test]
    fn compare_reports_missing_and_extra_lines() {
        let err = compare(&lines("a|1.000\nb|2.000"), &lines("a|1.000\nc|3.000")).unwrap_err();
        assert!(err.contains("- b|2.000") && err.contains("+ c|3.000"), "{err}");
    }

    #[test]
    fn compare_counts_duplicate_lines() {
        // 同一行少了一份也要看得出來（multiset，不是 set）
        assert!(compare(&lines("a|1.000\na|1.000"), &lines("a|1.000")).is_err());
    }

    #[test]
    fn compare_tolerates_crlf_and_last_digit() {
        // 金檔若被轉成 CRLF、或數字差在 0.002 以內（四捨五入邊界、跨平台 1 ulp），視為相同
        let golden = "pos(1.000,2.000)\r\nx(0.000,0.000,1.000)\r\n";
        let actual = lines("pos(1.001,2.000)\nx(0.000,0.000,1.000)");
        assert_eq!(compare(&lines(golden), &actual), Ok(1));
    }

    #[test]
    fn compare_rejects_differences_beyond_tolerance() {
        assert!(compare(&lines("pos(1.000)"), &lines("pos(1.003)")).is_err());
    }

    #[test]
    fn compare_rejects_same_numbers_with_different_text() {
        assert!(compare(&lines("3d|中華路|pos(1.000)"), &lines("3d|西寧南路|pos(1.000)")).is_err());
    }

    #[test]
    fn num_rounds_and_normalizes_negative_zero() {
        assert_eq!(num(-0.0004), "0.000");
        assert_eq!(num(1.23456), "1.235");
    }

    #[test]
    fn update_mode_always_fails_after_rewrite() {
        // 更新模式寫完檔一定失敗，避免環境變數一直開著時快照永遠綠
        assert!(matches!(verdict(Some("a"), &lines("a"), true), Verdict::Rewrite));
        assert!(matches!(verdict(Some("a"), &lines("a"), false), Verdict::Matches { absorbed: 0 }));
    }
}
```

- [ ] **Step 2：建立 `src/ui/map_snapshot.rs` 本體**

```rust
//! 地圖快照：照遊戲順序跑真的啟動系統，把生成的世界記成排序後的文字，
//! 當地圖資料驅動重構的標準答案（docs/superpowers/specs/2026-10-05-map-data-driven-design.md 第二節）
//!
//! 更新金檔：`UPDATE_SNAPSHOTS=1 cargo test map_snapshot`（寫完檔後測試會刻意失敗，檢查 git diff 後再跑一次）

use std::collections::BTreeMap;

use bevy::camera::primitives::MeshAabb;
use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;

use super::components::{FullMapContainer, MinimapContainer};
use crate::pedestrian::{PathfindingGrid, PointsOfInterest};
use crate::vehicle::{NpcVehicle, TrafficLight};
use crate::world::{Building, MapBounds};

const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/world/snapshots/ximending_world.txt"
);

/// 數字差在這個範圍內視為相同（四捨五入邊界、跨平台 sin／cos 的 1 ulp 差異）
const TOLERANCE: f64 = 0.002;

/// UI 用的一次性系統：直接呼叫小地圖、大地圖的建立函式（字型給預設 handle 就好）
fn spawn_map_huds(mut commands: Commands) {
    let font = Handle::<Font>::default();
    super::setup_map::setup_minimap_hud(&mut commands, &font);
    super::setup_map::setup_full_map(&mut commands, &font);
}

/// 不開視窗的 App：照遊戲的順序跑地圖相關的啟動系統一次
fn snapshot_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Image>()
        .init_resource::<MapBounds>()
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
    let mut line = format!(
        "3d|{}|pos{}|x{}|z{}|scale{}",
        entity_label(world, entity),
        vec3(translation),
        vec3(rotation * Vec3::X),
        vec3(rotation * Vec3::Z),
        vec3(scale)
    );
    if let Some(mesh) = world.get::<Mesh3d>(entity) {
        let meshes = world.resource::<Assets<Mesh>>();
        match meshes.get(&mesh.0).and_then(MeshAabb::compute_aabb) {
            Some(aabb) => {
                line += &format!(
                    "|mesh{}{}",
                    vec3(aabb.center.into()),
                    vec3(aabb.half_extents.into())
                );
            }
            None => line += "|mesh?",
        }
    }
    if let Some(collider) = world.get::<Collider>(entity) {
        line += &format!("|col:{}", collider_shape(collider));
    }
    if let Some(building) = world.get::<Building>(entity) {
        line += &format!("|bld:{}:{:?}", building.name, building.building_type);
    }
    if let Some(light) = world.get::<TrafficLight>(entity) {
        line += &format!("|light{}:{}", vec3(light.control_direction), light.is_primary);
    }
    if let Some(npc) = world.get::<NpcVehicle>(entity) {
        let points: Vec<String> = npc.waypoints.iter().map(|p| vec3(*p)).collect();
        line += &format!("|route[{}]@{}", points.join(","), npc.current_wp_index);
    }
    line
}

/// UI 節點：深度優先，路徑帶兄弟索引（bevy_ui 依兄弟順序疊放，排序不能抹掉它）
fn walk_ui(world: &World, entity: Entity, path: &str, out: &mut Vec<String>) {
    if let Some(node) = world.get::<Node>(entity) {
        let mut line = format!(
            "ui|{path}|l{}|t{}|w{}|h{}",
            val(node.left),
            val(node.top),
            val(node.width),
            val(node.height)
        );
        if let Some(text) = world.get::<Text>(entity) {
            line += &format!("|text:{}", text.0);
        }
        out.push(line);
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
    let world = app.world();
    let mut lines: Vec<String> = solids
        .iter()
        .map(|(e, g)| line_3d(world, *e, g))
        .collect();
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
        && na.iter().zip(&nb).all(|(x, y)| (x - y).abs() <= TOLERANCE + 1e-9)
}

/// 先以 multiset 逐行比對；剩下對不上的行，結構相同且每個數字差 ≤ TOLERANCE 的視為相同。
/// 通過時回傳被容差吸收的行數
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
    let mut report = format!("少了 {} 行、多了 {} 行\n", missing.len(), extra.len());
    for line in missing.iter().take(40) {
        report += &format!("- {line}\n");
    }
    for line in extra.iter().take(40) {
        report += &format!("+ {line}\n");
    }
    Err(report)
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
        return Verdict::Mismatch("找不到快照檔：先用 UPDATE_SNAPSHOTS=1 產生".to_string());
    };
    let expected: Vec<String> = golden.lines().map(str::to_string).collect();
    match compare(&expected, actual) {
        Ok(absorbed) => Verdict::Matches { absorbed },
        Err(report) => Verdict::Mismatch(report),
    }
}

#[test]
fn map_snapshot_matches_golden() {
    let actual = snapshot_lines(&mut snapshot_app());
    let golden = std::fs::read_to_string(GOLDEN).ok();
    let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    match verdict(golden.as_deref(), &actual, update) {
        Verdict::Matches { absorbed } => eprintln!("快照相符；被容差吸收 {absorbed} 行"),
        Verdict::Rewrite => {
            if let Some(dir) = std::path::Path::new(GOLDEN).parent() {
                std::fs::create_dir_all(dir).expect("建立快照目錄");
            }
            std::fs::write(GOLDEN, actual.join("\n") + "\n").expect("寫入快照檔");
            panic!(
                "已重寫 {GOLDEN}（{} 行）：用 git diff 檢查後，不帶 UPDATE_SNAPSHOTS 再跑一次",
                actual.len()
            );
        }
        Verdict::Mismatch(report) => panic!("快照不符：\n{report}"),
    }
}

#[test]
fn map_snapshot_is_deterministic() {
    assert_eq!(
        snapshot_lines(&mut snapshot_app()),
        snapshot_lines(&mut snapshot_app())
    );
}
```

`src/ui/mod.rs` 在 `#[cfg(test)] mod tests;` 下一行加：

```rust
#[cfg(test)]
mod map_snapshot;
```

- [ ] **Step 3：跑比對邏輯的測試**

Run: `cargo test map_snapshot::compare_tests`
Expected: 7 passed

- [ ] **Step 4：產生金檔**

Run: `UPDATE_SNAPSHOTS=1 cargo test map_snapshot_matches_golden`
Expected: FAIL，訊息「已重寫 …ximending_world.txt（約 1,300 行）」

- [ ] **Step 5：抽查金檔記到的是對的東西**

Run: `grep -c '' src/world/snapshots/ximending_world.txt; grep -n 'pos(0.000,0.200,0.000)' src/world/snapshots/ximending_world.txt | head -3; grep -n 'pos(80.000,0.050,-15.000)\|pos(98.000,0.300,-15.000)\|pos(62.000,0.300,-15.000)' src/world/snapshots/ximending_world.txt`
Expected:
- 漢中街徒步區：`pos(0.000,0.200,0.000)` 的 mesh 半長 `(7.500,0.000,42.500)`（長 85）
- 中華路：`pos(80.000,0.050,-15.000)` 車道 mesh 半長 `(16.000,0.000,90.000)`（寬 32）；人行道 `pos(98.000,0.300,-15.000)`、`pos(62.000,0.300,-15.000)` 半長 `(2.000,0.000,90.000)`
- `res|MapBounds|-119.000,109.000,-94.000,64.000`
- 蓋出來的建築 24 棟：`grep -c '|bld:' src/world/snapshots/ximending_world.txt` 應為 25（24 棟加峨嵋停車場）

數字不符就停下來查，不要往下做。

- [ ] **Step 6：跑快照測試確認通過、而且穩定**

Run: `cargo test map_snapshot`
Expected: 9 passed（含 `map_snapshot_matches_golden` 輸出「被容差吸收 0 行」、`map_snapshot_is_deterministic`）

- [ ] **Step 7：突變驗證——每一類各改一個數字，確認快照會紅、紅的是預期的行，改完用編輯還原**

| 改哪裡 | 改成 | 預期失敗的行 |
|---|---|---|
| `world/setup/roads_layout.rs` 的 `hanzhong_len` | 後面加 `+ 1.0` | 漢中街的 mesh 半長 |
| `world/setup/buildings_layout.rs` 萬年大樓的 `width: 20.0` | `21.0` | 萬年大樓本體與子實體 |
| `world/constants.rs` 的 `ZEBRA_CROSSING_OFFSET` | `2.6` | 斑馬線條紋 |
| `vehicle/traffic_lights.rs` 本地 `X_XINING` | `-56.0` | 西寧南路兩個路口的號誌 |
| `vehicle/spawning.rs` 的 `lane_offset` 係數 `0.25` | `0.3` | NPC 車的位置與 `route[...]` |
| `pedestrian/systems/pathfinding_grid.rs` 的 origin `-110.0` | `-112.0` | `res|grid` 與各列 |
| `ui/minimap.rs` 的 `h_len` | `201.0` | 東西向道路方塊 |
| `world/constants.rs` `MapBounds::default` 的 `max_x` | `110.0` | `res|MapBounds` |
| `world/setup/mod.rs` 東牆的 `110.0` | `111.0` | 東牆碰撞體 |

每次：改 → `cargo test map_snapshot_matches_golden` → 確認失敗清單只有預期的行 → 改回 → 再跑一次確認綠。

- [ ] **Step 8：全套驗證**

Run: `cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`

- [ ] **Step 9：送 code-reviewer 審，修完後等 user 說「提交」**

```
test(map): 加上地圖快照測試當資料驅動重構的標準答案

- 不開視窗跑真的啟動系統（世界、號誌、NPC 車、行人網格、小地圖與大地圖），記錄世界座標、旋轉軸、mesh 外框、碰撞體、建築、號誌、路線、UI 節點與地圖 resource
- 排序後以 multiset 比對，數字差 0.002 以內視為相同；UPDATE_SNAPSHOTS=1 重寫金檔後測試刻意失敗
```

```bash
git commit -F <訊息檔> -- src/ui/map_snapshot.rs src/ui/mod.rs src/world/snapshots/ximending_world.txt
```

提交後請 user push，確認 CI（Ubuntu、nextest）也綠：`gh run list --limit 1`。

---

### Task 3：第 0 步之二——執行期行為測試、盤點、基準截圖

**Files:**
- Test: `src/pedestrian/systems/lifecycle.rs`（檔尾新增 `mod tests`）
- Test: `src/pedestrian/systems/pathfinding_grid.rs`（檔尾新增 `mod tests`）
- Test: `src/vehicle/npc_ai.rs`（檔尾新增 `mod tests`）
- Test: `src/combat/damage/death.rs`（檔尾新增 `mod tests`）
- Create: `src/ui/map_marker_tests.rs`；Modify: `src/ui/mod.rs`（加 `#[cfg(test)] mod map_marker_tests;`）

**Interfaces:**
- Consumes: 現有系統 `pedestrian_despawn_system`、`get_movement_target`（私有，測試在同檔）、`astar_movement_system`、`npc_vehicle_motion_system`、`update_minimap`、`update_fullmap`、`update_minimap_gps_marker`、`player_respawn_system`
- Produces: 8 個行為測試；Task 4、6、14、16 會改它們的 App 建法或期望值

這些測試在現在的程式上就會通過（它們記錄現況），所以每條都要做一次突變：把被測的數字改掉一次，看它紅。

- [ ] **Step 1：盤點執行期才用到的地圖數字**

Run:

```bash
grep -rnE '\b(X|Z|W)_[A-Z]+\b|PLAYER_SPAWN|MapBounds|clamp\(-?[0-9]+\.0, -?[0-9]+\.0\)|0\.9;|2\.0;|150\.0|600\.0|400\.0' src --include='*.rs' | grep -v '^src/world/constants.rs' | grep -v 'world/setup/' | grep -v '#\[cfg(test)\]'
```

逐筆判斷是不是「每幀執行的系統裡、和地圖位置有關」。spec 第二節的表列了 8 列（越界移除、兩份逃跑範圍、NPC 夾邊界、小地圖／大地圖／GPS 標記、重生位置）。找到表外的，補一個同樣寫法的行為測試，並把那一列加進 spec 第二節的表（同一個 commit）；屬於散落字面座標的（第三段第 4 批）只記進 spec 該段的清單，不寫測試。

- [ ] **Step 2：行人越界移除與逃跑目標（`lifecycle.rs` 檔尾）**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// 只跑行人消失系統：玩家站在 player，回傳每個行人跑完一次後還在不在
    fn despawn_survivors(player: Vec3, peds: &[Vec3]) -> Vec<bool> {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PedestrianConfig>()
            .add_systems(Update, pedestrian_despawn_system);
        app.world_mut()
            .spawn((Player::default(), Transform::from_translation(player)));
        let ids: Vec<Entity> = peds
            .iter()
            .map(|p| {
                app.world_mut()
                    .spawn((
                        Pedestrian,
                        PedestrianState::default(),
                        Transform::from_translation(*p),
                    ))
                    .id()
            })
            .collect();
        app.update();
        ids.iter()
            .map(|e| app.world().get_entity(*e).is_ok())
            .collect()
    }

    #[test]
    fn pedestrians_beyond_outer_road_centerlines_are_removed() {
        // 界線內 0.5 m 留下、界線外 0.5 m 移除；玩家站在界線上，兩個行人都在 60 m 的消失半徑內
        let cases = [
            (Vec3::new(-100.0, 0.0, 0.0), Vec3::new(-99.5, 0.0, 0.0), Vec3::new(-100.5, 0.0, 0.0)),
            (Vec3::new(80.0, 0.0, 0.0), Vec3::new(79.5, 0.0, 0.0), Vec3::new(80.5, 0.0, 0.0)),
            (Vec3::new(0.0, 0.0, -80.0), Vec3::new(0.0, 0.0, -79.5), Vec3::new(0.0, 0.0, -80.5)),
            (Vec3::new(0.0, 0.0, 50.0), Vec3::new(0.0, 0.0, 49.5), Vec3::new(0.0, 0.0, 50.5)),
        ];
        for (player, inside, outside) in cases {
            assert_eq!(despawn_survivors(player, &[inside, outside]), [true, false], "{player}");
        }
    }

    fn fleeing_from(threat: Vec3) -> PedestrianState {
        PedestrianState {
            state: PedState::Fleeing,
            last_threat_pos: Some(threat),
            ..default()
        }
    }

    #[test]
    fn flee_target_stays_5m_inside_outer_roads() {
        // 背對威脅逃 20 m 的目標，超出範圍時被夾回；往內的不受影響
        let patrol = PatrolPath::new(Vec::new());
        let target = |pos: Vec3, threat: Vec3| {
            get_movement_target(&fleeing_from(threat), pos, &patrol).expect("逃跑一定有目標")
        };
        assert_eq!(target(Vec3::new(74.0, 0.0, 0.0), Vec3::new(64.0, 0.0, 0.0)).x, 75.0);
        assert_eq!(target(Vec3::new(-90.0, 0.0, 0.0), Vec3::new(-80.0, 0.0, 0.0)).x, -95.0);
        assert_eq!(target(Vec3::new(0.0, 0.0, 40.0), Vec3::new(0.0, 0.0, 30.0)).z, 45.0);
        assert_eq!(target(Vec3::new(0.0, 0.0, -70.0), Vec3::new(0.0, 0.0, -60.0)).z, -75.0);
        assert_eq!(target(Vec3::new(50.0, 0.0, 0.0), Vec3::new(40.0, 0.0, 0.0)).x, 70.0);
    }
}
```

- [ ] **Step 3：A* 行人逃跑方向（`pathfinding_grid.rs` 檔尾）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    /// 逃跑中的行人（位置, 威脅位置）跑兩次 update（第一次的 dt 是 0），回傳控制器這一幀的水平位移
    fn flee_steps(peds: &[(Vec3, Vec3)]) -> Vec<Vec3> {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                1.0 / 60.0,
            )))
            .init_resource::<PedestrianConfig>()
            .add_systems(Update, astar_movement_system);
        let ids: Vec<Entity> = peds
            .iter()
            .map(|&(pos, threat)| {
                app.world_mut()
                    .spawn((
                        Pedestrian,
                        PedestrianState {
                            state: PedState::Fleeing,
                            last_threat_pos: Some(threat),
                            ..default()
                        },
                        DailyBehavior::default(),
                        Transform::from_translation(pos),
                        AStarPath::new(Vec3::ZERO),
                        KinematicCharacterController::default(),
                    ))
                    .id()
            })
            .collect();
        app.update();
        app.update();
        ids.iter()
            .map(|e| {
                let t = app
                    .world()
                    .get::<KinematicCharacterController>(*e)
                    .and_then(|c| c.translation)
                    .unwrap_or(Vec3::ZERO);
                Vec3::new(t.x, 0.0, t.z)
            })
            .collect()
    }

    #[test]
    fn flee_direction_clamped_5m_inside_outer_roads() {
        // 每邊兩個往外逃的行人：界外 0.5 m 的目標被夾回、方向反轉；界內 0.5 m 的照樣往外
        let s = flee_steps(&[
            (Vec3::new(75.5, 0.0, 0.0), Vec3::new(65.5, 0.0, 0.0)),
            (Vec3::new(74.5, 0.0, 0.0), Vec3::new(64.5, 0.0, 0.0)),
            (Vec3::new(-95.5, 0.0, 0.0), Vec3::new(-85.5, 0.0, 0.0)),
            (Vec3::new(-94.5, 0.0, 0.0), Vec3::new(-84.5, 0.0, 0.0)),
            (Vec3::new(0.0, 0.0, 45.5), Vec3::new(0.0, 0.0, 35.5)),
            (Vec3::new(0.0, 0.0, 44.5), Vec3::new(0.0, 0.0, 34.5)),
            (Vec3::new(0.0, 0.0, -75.5), Vec3::new(0.0, 0.0, -65.5)),
            (Vec3::new(0.0, 0.0, -74.5), Vec3::new(0.0, 0.0, -64.5)),
        ]);
        assert!(s[0].x < 0.0 && s[1].x > 0.0, "東：{s:?}");
        assert!(s[2].x > 0.0 && s[3].x < 0.0, "西：{s:?}");
        assert!(s[4].z < 0.0 && s[5].z > 0.0, "南：{s:?}");
        assert!(s[6].z > 0.0 && s[7].z < 0.0, "北：{s:?}");
    }
}
```

- [ ] **Step 4：NPC 車夾在地圖邊界內（`npc_ai.rs` 檔尾）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle::{NpcVehicle, VehiclePreset};

    #[test]
    fn npc_vehicles_clamped_to_map_bounds() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<MapBounds>()
            .init_resource::<WeatherState>()
            .init_resource::<VehicleConfig>()
            .add_systems(Update, npc_vehicle_motion_system);
        let starts = [
            Vec3::new(109.5, 0.5, 0.0),
            Vec3::new(-119.5, 0.5, 0.0),
            Vec3::new(0.0, 0.5, 64.5),
            Vec3::new(0.0, 0.5, -94.5),
        ];
        let ids: Vec<Entity> = starts
            .iter()
            .map(|p| {
                app.world_mut()
                    .spawn((
                        Transform::from_translation(*p),
                        VehiclePreset::car().into_components(),
                        NpcVehicle::default(),
                    ))
                    .id()
            })
            .collect();
        app.update();
        let pos = |e: Entity| app.world().get::<Transform>(e).unwrap().translation;
        assert_eq!(pos(ids[0]).x, 109.0);
        assert_eq!(pos(ids[1]).x, -119.0);
        assert_eq!(pos(ids[2]).z, 64.0);
        assert_eq!(pos(ids[3]).z, -94.0);
    }
}
```

- [ ] **Step 5：重生位置（`death.rs` 檔尾）**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respawn_puts_player_at_spawn_point() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(RespawnState {
                is_dead: true,
                respawn_timer: 0.0,
                death_position: Vec3::ZERO,
            })
            .init_resource::<crate::ui::ScreenEffectState>()
            .init_resource::<crate::ui::NotificationQueue>()
            .add_systems(Update, player_respawn_system);
        let player = app
            .world_mut()
            .spawn((
                Player::default(),
                Transform::from_xyz(50.0, 3.0, 50.0),
                Health::new(100.0),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(player).unwrap().translation,
            Vec3::new(5.0, 0.7, -5.0)
        );
    }
}
```

- [ ] **Step 6：小地圖、大地圖、GPS 標記（新檔 `src/ui/map_marker_tests.rs`）**

```rust
//! 小地圖、大地圖、GPS 標記的投影（期望值一律寫字面數字）

use bevy::prelude::*;

use super::components::{
    FullMapPlayerMarker, GpsNavigationState, MinimapContainer, MinimapGpsMarker,
    MinimapPlayerMarker,
};
use super::gps_navigation::update_minimap_gps_marker;
use super::minimap::{update_fullmap, update_minimap};
use crate::player::Player;

fn px(v: Val) -> f32 {
    match v {
        Val::Px(p) => p,
        other => panic!("預期 px，實際 {other:?}"),
    }
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// 玩家站在 player_pos，跑一次 update，回傳標記容器的 (left, top)
fn player_marker_at(fullmap: bool, player_pos: Vec3) -> (f32, f32) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.world_mut()
        .spawn((Player::default(), Transform::from_translation(player_pos)));
    let marker = if fullmap {
        app.add_systems(Update, update_fullmap);
        app.world_mut()
            .spawn((Node::default(), Transform::default(), FullMapPlayerMarker))
            .id()
    } else {
        app.add_systems(Update, update_minimap);
        app.world_mut()
            .spawn((Node::default(), Transform::default(), MinimapPlayerMarker))
            .id()
    };
    app.update();
    let node = app.world().get::<Node>(marker).unwrap();
    (px(node.left), px(node.top))
}

#[test]
fn minimap_marker_projection() {
    // (20, −7) 投影到 (168, 156.3)，容器左上角再減 (10, 24)；X、Z 不對稱，軸對調會被抓到
    let (left, top) = player_marker_at(false, Vec3::new(20.0, 0.0, -7.0));
    assert!(approx(left, 158.0) && approx(top, 132.3), "({left}, {top})");
    // 超出範圍時夾在 10〜290
    let (left, top) = player_marker_at(false, Vec3::new(500.0, 0.0, -500.0));
    assert!(approx(left, 280.0) && approx(top, 266.0), "({left}, {top})");
}

#[test]
fn fullmap_marker_projection() {
    // (20, −7) 投影到 (640, 414)，容器左上角再減 (15, 37)
    let (left, top) = player_marker_at(true, Vec3::new(20.0, 0.0, -7.0));
    assert!(approx(left, 625.0) && approx(top, 377.0), "({left}, {top})");
    // 超出範圍時夾在 20〜1180／20〜780
    let (left, top) = player_marker_at(true, Vec3::new(1000.0, 0.0, 1000.0));
    assert!(approx(left, 1165.0) && approx(top, -17.0), "({left}, {top})");
}

/// GPS 目的地在 destination，回傳兩個標記的 (left, top)，依 left 排序
fn gps_markers(destination: Vec3) -> Vec<(f32, f32)> {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(GpsNavigationState {
            active: true,
            destination: Some(destination),
            ..default()
        })
        .add_systems(Update, update_minimap_gps_marker);
    app.world_mut().spawn((Node::default(), MinimapContainer));
    let markers: Vec<Entity> = (0..2)
        .map(|_| {
            app.world_mut()
                .spawn((Node::default(), Visibility::Hidden, MinimapGpsMarker))
                .id()
        })
        .collect();
    app.update();
    let mut out: Vec<(f32, f32)> = markers
        .iter()
        .map(|e| {
            let node = app.world().get::<Node>(*e).unwrap();
            (px(node.left), px(node.top))
        })
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

#[test]
fn gps_marker_projection() {
    // (20, −7) 投影到 (168, 156.3)；外圈脈衝左上角減 8、核心點減 4
    let m = gps_markers(Vec3::new(20.0, 0.0, -7.0));
    assert!(
        approx(m[0].0, 160.0) && approx(m[0].1, 148.3) && approx(m[1].0, 164.0) && approx(m[1].1, 152.3),
        "{m:?}"
    );
    // X 420 夾回 295
    let m = gps_markers(Vec3::new(300.0, 0.0, 0.0));
    assert!(approx(m[0].0, 287.0) && approx(m[1].0, 291.0), "{m:?}");
}
```

`src/ui/mod.rs` 加：

```rust
#[cfg(test)]
mod map_marker_tests;
```

- [ ] **Step 7：跑測試確認都通過**

Run: `cargo test lifecycle::tests pathfinding_grid::tests npc_ai::tests death::tests map_marker_tests`
（cargo test 一次只吃一個過濾字串時，分開跑五次）
Expected: 全部 PASS

- [ ] **Step 8：每條測試做一次突變，確認會紅，改完用編輯還原**

| 改哪裡 | 改成 | 應該紅的測試 |
|---|---|---|
| `lifecycle.rs` 的 `MAP_MIN_X` | `-101.0` | `pedestrians_beyond_outer_road_centerlines_are_removed` |
| `lifecycle.rs` `get_movement_target` 的 `clamp(-95.0, 75.0)` | `clamp(-95.0, 76.0)` | `flee_target_stays_5m_inside_outer_roads` |
| `pathfinding_grid.rs` `astar_movement_system` 的 `clamp(-75.0, 45.0)` | `clamp(-75.0, 44.0)` | `flee_direction_clamped_5m_inside_outer_roads` |
| `constants.rs` `MapBounds::default` 的 `min_z` | `-95.0` | `npc_vehicles_clamped_to_map_bounds` |
| `minimap.rs` `update_minimap` 的 `offset_y` | `151.0` | `minimap_marker_projection` |
| `minimap.rs` `update_fullmap` 的 `fm_off_x` | `601.0` | `fullmap_marker_projection` |
| `gps_navigation.rs` 的 `clamp(5.0, 295.0)`（X） | `clamp(5.0, 294.0)` | `gps_marker_projection` |
| `constants.rs` 的 `PLAYER_SPAWN_Z` | `-6.0` | `respawn_puts_player_at_spawn_point`（同時快照也會紅） |

- [ ] **Step 9：拍重構前的基準截圖**

照 `.claude/rules/dev-tools.md` 的 BRP 流程。先記下 user 目前的前景 app，測完把焦點還回去；請 user 測試期間不要點遊戲視窗。

```bash
FRONT=$(osascript -e 'tell application "System Events" to get bundle identifier of first application process whose frontmost is true')
SHOTS=<scratchpad>/map-baseline; mkdir -p "$SHOTS"
cargo brp > "$SHOTS/brp.log" 2>&1 &
# 等 log 出現「📦 載入完成，轉場至 InGame」後：
open -b "$FRONT"
rpc() { curl -s -X POST http://127.0.0.1:15702 -H 'Content-Type: application/json' -d "$1"; }
rpc '{"jsonrpc":"2.0","id":1,"method":"brp_extras/screenshot","params":{"path":"'"$SHOTS"'/0800-default.png"}}'
rpc '{"jsonrpc":"2.0","id":2,"method":"brp_extras/send_keys","params":{"keys":["KeyM"],"duration_ms":100}}'
rpc '{"jsonrpc":"2.0","id":3,"method":"brp_extras/screenshot","params":{"path":"'"$SHOTS"'/0800-fullmap.png"}}'
rpc '{"jsonrpc":"2.0","id":4,"method":"brp_extras/send_keys","params":{"keys":["KeyM"],"duration_ms":100}}'
rpc '{"jsonrpc":"2.0","id":5,"method":"brp_extras/send_keys","params":{"keys":["KeyW","KeyD"],"duration_ms":2000}}'
rpc '{"jsonrpc":"2.0","id":6,"method":"brp_extras/screenshot","params":{"path":"'"$SHOTS"'/0800-turned.png"}}'
rpc '{"jsonrpc":"2.0","id":7,"method":"brp_extras/shutdown"}'
```

每張截圖回傳後要等檔案出現再送下一個指令。21:00 那組：暫改 `src/core/resources.rs` `WorldTime::default()` 的 `hour: 8.0` 為 `21.0`，重跑同一段（檔名改 `2100-*`），截完改回 `8.0`。

看 `0800-turned.png`：按住 W+D 讓玩家轉向之後（只按 D 是平移、角色不會轉向），小地圖箭頭有沒有轉（spec 判斷它不會轉，Task 15 修）。觀察結果記進 spec 第四節 Bug 2 的第一點。

- [ ] **Step 10：全套驗證、送審、等 user 說「提交」**

Run: `cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`

```
test(map): 記下執行期才用到的地圖數字（行人越界、逃跑範圍、NPC 邊界、地圖標記、重生點）

- 期望值寫字面數字，重構時改錯會紅；每條都做過突變確認
```

```bash
git commit -F <訊息檔> -- src/pedestrian/systems/lifecycle.rs src/pedestrian/systems/pathfinding_grid.rs src/vehicle/npc_ai.rs src/combat/damage/death.rs src/ui/map_marker_tests.rs src/ui/mod.rs
```

（Step 1 有補進 spec 的話，把 spec 一起列進路徑。）

---

### Task 4：第 1 步——骨架：邊界、出生點、地面、牆

**Files:**
- Create: `assets/levels/ximending.ron`、`src/world/map_data/{mod.rs, file.rs, layout.rs, tests.rs}`
- Modify: `src/world/mod.rs`（`mod map_data;`、`pub use map_data::*;`、`.init_resource::<MapBounds>()` 換成 `install_map`）
- Modify: `src/world/constants.rs`（刪 `impl Default for MapBounds`、`PLAYER_SPAWN_X`、`PLAYER_SPAWN_Z`）
- Modify: `src/world/setup/mod.rs`（`setup_world` 加 `layout: Res<MapLayout>`；`setup_ground` 讀地面與牆；刪 `GROUND_CENTER`；改測試）
- Modify: `src/world/setup/vehicles_spawn.rs`（出生點）
- Modify: `src/combat/damage/mod.rs:126`（`RESPAWN_POSITION` 改成函式）、`src/combat/damage/death.rs:435-460`
- Modify: `src/ui/map_snapshot.rs`、`src/vehicle/npc_ai.rs`、`src/combat/damage/death.rs` 的測試（改用 `install_map`）

**Interfaces:**
- Produces:
  - `pub fn install_map(app: &mut App)`、`pub fn ximending_layout() -> MapLayout`、`pub fn load_map(text: &str) -> Result<MapLayout, Vec<MapError>>`
  - `pub struct MapLayout { pub bounds: MapBounds, pub spawn: Vec2, pub ground_center: Vec3, pub ground_collider_half_extents: Vec3, pub walls: [WallBox; 4] }`（`Resource`；之後的 task 加欄位）
  - `pub struct WallBox { pub center: Vec3, pub half_extents: Vec3 }`、`pub struct MapError(pub String)`
  - `pub fn respawn_position(layout: &MapLayout) -> Vec3`（`combat::damage`）

- [ ] **Step 1：建立資料檔 `assets/levels/ximending.ron`**

```ron
// 西門町地圖資料（第二段：現在手做的地圖，不含 OSM 資料）
// 座標：+X 東、+Z 南，單位公尺。只存「決定」，能推算的交給 src/world/map_data
(
    // 可活動範圍（NPC 車夾在這裡面）
    bounds: (min_x: -119.0, max_x: 109.0, min_z: -94.0, max_z: 64.0),
    // 玩家出生點 (x, z)：漢中街與峨嵋街口
    spawn: (5.0, -5.0),
    // 地面平板的中心 (x, z) 與碰撞體半長；視覺尺寸由程式決定
    ground: (center: (-10.0, -15.0), collider_half_extents: (200.0, 0.1, 200.0)),
    // 隱形邊界牆：中心離邊界 offset 公尺；沿牆方向的中心取地面中心
    walls: (
        offset: 1.0,
        half_thickness: 0.5,
        half_height: 20.0,
        center_y: 10.0,
        along_z_half_length: 100.0,
        along_x_half_length: 130.0,
    ),
)
```

- [ ] **Step 2：建立 `file.rs`**

```rust
//! 地圖資料檔（RON）的格式：只存「決定」，能推算的交給 `layout`

use serde::Deserialize;

/// 整份資料檔
#[derive(Deserialize, Debug, Clone)]
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
pub struct BoundsSpec {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

/// 地面平板：中心 (x, z) 與碰撞體半長
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct GroundSpec {
    pub center: (f32, f32),
    pub collider_half_extents: (f32, f32, f32),
}

/// 隱形邊界牆：中心離邊界 `offset` 公尺；沿牆方向的中心取地面中心
#[derive(Deserialize, Debug, Clone, Copy)]
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
```

- [ ] **Step 3：建立 `layout.rs`**

```rust
//! 檢查並解析地圖資料檔，產生系統直接取用的 `MapLayout`

use bevy::prelude::*;

use super::file::MapFile;
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
}

impl MapLayout {
    /// 檢查並解析；有錯時回傳每一筆錯誤
    pub fn from_file(file: &MapFile) -> Result<Self, Vec<MapError>> {
        let mut errors = Vec::new();
        check_bounds_and_spawn(file, &mut errors);
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
        }
    }
}

/// 邊界要是正的矩形、出生點要在邊界內
fn check_bounds_and_spawn(file: &MapFile, errors: &mut Vec<MapError>) {
    let b = file.bounds;
    if !(b.min_x < b.max_x && b.min_z < b.max_z) {
        errors.push(MapError(format!("邊界：min 必須小於 max（{b:?}）")));
        return;
    }
    let (x, z) = file.spawn;
    if !(b.min_x..=b.max_x).contains(&x) || !(b.min_z..=b.max_z).contains(&z) {
        errors.push(MapError(format!("出生點 ({x}, {z}) 在邊界外")));
    }
}
```

- [ ] **Step 4：建立 `mod.rs`**

```rust
//! 地圖資料：讀 `assets/levels/ximending.ron`（只存「決定」），解析成系統直接取用的 `MapLayout`
//!
//! - `file`：資料檔格式
//! - `layout`：檢查、解析、查詢

mod file;
mod layout;
#[cfg(test)]
mod tests;

pub use file::*;
pub use layout::*;

use bevy::prelude::*;

/// 西門町地圖資料檔（編進執行檔）
const XIMENDING_RON: &str = include_str!("../../../assets/levels/ximending.ron");

/// 解析地圖資料；有錯時回傳每一筆錯誤
pub fn load_map(text: &str) -> Result<MapLayout, Vec<MapError>> {
    let file: MapFile = bevy::asset::ron::from_str(text)
        .map_err(|e| vec![MapError(format!("資料檔格式錯誤：{e}"))])?;
    MapLayout::from_file(&file)
}

/// 西門町地圖；資料檔有錯時直接 panic，遊戲一啟動就看得到是哪一筆
pub fn ximending_layout() -> MapLayout {
    load_map(XIMENDING_RON).unwrap_or_else(|errors| {
        let list: Vec<String> = errors.iter().map(ToString::to_string).collect();
        panic!("assets/levels/ximending.ron 有誤：\n{}", list.join("\n"))
    })
}

/// 載入西門町地圖並插入所有地圖 resource：`WorldPlugin` 與需要真實地圖的測試共用
pub fn install_map(app: &mut App) {
    let layout = ximending_layout();
    app.insert_resource(layout.bounds.clone())
        .insert_resource(layout);
}
```

- [ ] **Step 5：寫測試 `tests.rs`**

```rust
//! 地圖資料：讀真實資料檔、檢查規則

use bevy::prelude::*;

use super::*;
use crate::world::{MapBounds, WorldPlugin};

/// 真實資料檔解析成 `MapFile`，給「改一個欄位看會不會報錯」的測試用
fn real_file() -> MapFile {
    bevy::asset::ron::from_str(super::XIMENDING_RON).expect("資料檔要能解析")
}

/// 所有錯誤訊息；解析成功時是空的
fn errors_of(file: &MapFile) -> Vec<String> {
    MapLayout::from_file(file)
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.0)
        .collect()
}

fn assert_error(file: &MapFile, needle: &str) {
    let errors = errors_of(file);
    assert!(
        errors.iter().any(|e| e.contains(needle)),
        "預期有含「{needle}」的錯誤，實際：{errors:?}"
    );
}

#[test]
fn ximending_map_loads() {
    assert_eq!(errors_of(&real_file()), Vec::<String>::new());
}

#[test]
fn bounds_and_spawn_from_file() {
    let layout = ximending_layout();
    let b = &layout.bounds;
    assert_eq!((b.min_x, b.max_x, b.min_z, b.max_z), (-119.0, 109.0, -94.0, 64.0));
    assert_eq!(layout.spawn, Vec2::new(5.0, -5.0));
}

#[test]
fn ground_from_file() {
    let layout = ximending_layout();
    assert_eq!(layout.ground_center, Vec3::new(-10.0, 0.0, -15.0));
    assert_eq!(layout.ground_collider_half_extents, Vec3::new(200.0, 0.1, 200.0));
}

#[test]
fn walls_sit_1m_outside_bounds_centered_on_ground() {
    let along_z = Vec3::new(0.5, 20.0, 100.0);
    let along_x = Vec3::new(130.0, 20.0, 0.5);
    assert_eq!(
        ximending_layout().walls,
        [
            WallBox { center: Vec3::new(110.0, 10.0, -15.0), half_extents: along_z },
            WallBox { center: Vec3::new(-120.0, 10.0, -15.0), half_extents: along_z },
            WallBox { center: Vec3::new(-10.0, 10.0, 65.0), half_extents: along_x },
            WallBox { center: Vec3::new(-10.0, 10.0, -95.0), half_extents: along_x },
        ]
    );
}

#[test]
fn rejects_inverted_bounds() {
    let mut file = real_file();
    file.bounds.min_x = 200.0;
    assert_error(&file, "邊界：min 必須小於 max");
}

#[test]
fn rejects_spawn_outside_bounds() {
    let mut file = real_file();
    file.spawn = (500.0, 0.0);
    assert_error(&file, "出生點 (500, 0) 在邊界外");
}

#[test]
fn reports_ron_syntax_error() {
    let errors = load_map("(").unwrap_err();
    assert!(errors[0].0.contains("資料檔格式錯誤"), "{errors:?}");
}

#[test]
fn world_plugin_installs_map_layout() {
    // 遊戲本體靠 WorldPlugin 插入地圖；所有外掛的 build() 都在第一次 update 前跑完
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .add_plugins(WorldPlugin);
    assert!(app.world().contains_resource::<MapLayout>());
    assert!(app.world().contains_resource::<MapBounds>());
}
```

- [ ] **Step 6：接到 `world/mod.rs`，跑測試確認失敗**

`src/world/mod.rs`：模組清單加 `mod map_data;`（照字母排在 `interior` 後），re-export 加 `pub use map_data::*;`。先不改 `build()`。

Run: `cargo test map_data`
Expected: `world_plugin_installs_map_layout` FAIL（`MapLayout` 不存在）；其餘 PASS

- [ ] **Step 7：`WorldPlugin` 改用 `install_map`**

`build()` 開頭（`app` 鏈之前）加 `map_data::install_map(app);`，並刪掉鏈中的 `.init_resource::<MapBounds>()`。

`src/world/constants.rs`：刪除整段 `impl Default for MapBounds { ... }`，以及 `PLAYER_SPAWN_X`、`PLAYER_SPAWN_Z` 兩個常數（含註解）。`MapBounds` 的 doc 改成「地圖邊界（XZ 平面），由地圖資料檔決定；用於限制 NPC／車輛不駛出地圖」。

- [ ] **Step 8：地面與牆讀 `MapLayout`（`src/world/setup/mod.rs`）**

- `setup_world` 參數加 `layout: Res<MapLayout>`（`use super::MapLayout;` 或 `crate::world::MapLayout`），呼叫改成 `setup_ground(&mut commands, &mut meshes, &mut materials, &layout);`、`vehicles_spawn::setup_player_and_vehicles(&mut commands, &mut meshes, &mut materials, &layout);`
- 刪 `const GROUND_CENTER`；`setup_ground` 參數加 `layout: &MapLayout`：
  - `Transform::from_translation(GROUND_CENTER)` → `Transform::from_translation(layout.ground_center)`
  - `Collider::cuboid(200.0, 0.1, 200.0)` → `Collider::cuboid(layout.ground_collider_half_extents.x, layout.ground_collider_half_extents.y, layout.ground_collider_half_extents.z)`
  - 整段 `let walls: &[(Vec3, Vec3)] = &[...]; for &(pos, half_ext) in walls {` 改成 `for wall in &layout.walls {`，迴圈內 `pos` → `wall.center`、`half_ext` → `wall.half_extents`
  - 註解「地圖範圍：X: -120 ~ +100, Z: -100 ~ +70」刪掉
- 測試 `ground_edge_hidden_by_fog_from_anywhere_on_map`：`let bounds = MapBounds::default();` 改成 `let layout = crate::world::ximending_layout(); let bounds = &layout.bounds;`，`GROUND_CENTER` 改成 `layout.ground_center`；`use crate::world::{MapBounds, CLEAR_FOG_VISIBILITY};` 改成 `use crate::world::CLEAR_FOG_VISIBILITY;`

- [ ] **Step 9：出生點與重生點**

`src/world/setup/vehicles_spawn.rs`：`setup_player_and_vehicles` 參數加 `layout: &MapLayout`；`use` 刪 `PLAYER_SPAWN_X, PLAYER_SPAWN_Z`；`let start_pos = Vec3::new(PLAYER_SPAWN_X, 0.0, PLAYER_SPAWN_Z);` 改成 `let start_pos = Vec3::new(layout.spawn.x, 0.0, layout.spawn.y);`

`src/combat/damage/mod.rs`：刪 `pub const RESPAWN_POSITION`，改成：

```rust
/// 重生位置：地圖資料的出生點
pub fn respawn_position(layout: &MapLayout) -> Vec3 {
    Vec3::new(layout.spawn.x, PLAYER_RESPAWN_Y, layout.spawn.y)
}
```

（`use crate::world::{...}` 刪 `PLAYER_SPAWN_X, PLAYER_SPAWN_Z`、加 `MapLayout`。）

`src/combat/damage/death.rs`：`use super::{...RESPAWN_POSITION}` 改成 `respawn_position`；`player_respawn_system` 參數加 `layout: Res<crate::world::MapLayout>`；`transform.translation = RESPAWN_POSITION;` 改成 `transform.translation = respawn_position(&layout);`

- [ ] **Step 10：測試改用 `install_map`**（期望值都不改）

`src/ui/map_snapshot.rs` 的 `snapshot_app` 開頭改成（`use` 刪 `MapBounds` 以外不動；`MapBounds` 仍給 `resource_lines` 用）：

```rust
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), TransformPlugin));
    crate::world::install_map(&mut app);
    app.init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<Image>()
        .add_systems(
```

（`.add_systems(...)` 以下不變，刪掉原本的 `.init_resource::<MapBounds>()`。）

`src/vehicle/npc_ai.rs` 測試的 App 改成：

```rust
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        crate::world::install_map(&mut app);
        app.init_resource::<WeatherState>()
            .init_resource::<VehicleConfig>()
            .add_systems(Update, npc_vehicle_motion_system);
```

`src/combat/damage/death.rs` 測試的 App 改成：

```rust
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        crate::world::install_map(&mut app);
        app.insert_resource(RespawnState {
            is_dead: true,
            respawn_timer: 0.0,
            death_position: Vec3::ZERO,
        })
        .init_resource::<crate::ui::ScreenEffectState>()
        .init_resource::<crate::ui::NotificationQueue>()
        .add_systems(Update, player_respawn_system);
```

- [ ] **Step 11：跑測試**

Run: `cargo test map_data && cargo test map_snapshot && cargo test npc_ai::tests && cargo test death::tests`
Expected: 全過；快照「被容差吸收 0 行」

- [ ] **Step 12：突變驗證檢查規則**

- `check_bounds_and_spawn` 的邊界條件改成 `if false` → `rejects_inverted_bounds` 紅
- 出生點條件改成 `if false` → `rejects_spawn_outside_bounds` 紅
- `install_map(app);` 從 `WorldPlugin::build` 註解掉 → `world_plugin_installs_map_layout` 紅

每次用編輯還原。

- [ ] **Step 13：開遊戲確認能啟動**

`cargo brp` 背景啟動（同 Task 3 Step 9 的焦點處理），log 出現「✅ 西門町 (重構版) 載入完成！」與「📦 載入完成，轉場至 InGame」後立刻 `brp_extras/shutdown`。

- [ ] **Step 14：全套驗證、送審、等 user 說「提交」**

```
refactor(map): 地圖資料檔骨架：邊界、出生點、地面、牆改讀 ximending.ron

- 新增 src/world/map_data（格式、檢查、解析成 MapLayout）與 assets/levels/ximending.ron；WorldPlugin 以 install_map 插入 MapLayout 與 MapBounds
- 刪除 MapBounds 的 Default 與出生點常數；重生位置改由地圖資料推算
```

```bash
git commit -F <訊息檔> -- assets/levels/ximending.ron src/world/map_data src/world/mod.rs src/world/constants.rs src/world/setup/mod.rs src/world/setup/vehicles_spawn.rs src/combat/damage/mod.rs src/combat/damage/death.rs src/ui/map_snapshot.rs src/vehicle/npc_ai.rs
```

---

### Task 5：第 2 步——路網

**Files:**
- Modify: `assets/levels/ximending.ron`、`src/world/map_data/{mod.rs, file.rs, layout.rs, tests.rs}`
- Create: `src/world/map_data/geometry.rs`
- Modify: `src/world/setup/roads_layout.rs`（整個生成段落）、`src/world/setup/mod.rs`（傳 layout）
- Modify: `src/world/roads.rs:121, 172`、`src/vehicle/spawning.rs:447-454`、`src/pedestrian/behavior.rs:145`（人行道寬統一）

**Interfaces:**
- Consumes: `MapLayout`（Task 4）
- Produces:
  - `pub enum RoadAxis { NorthSouth, EastWest }`、`pub enum RoadKind { Asphalt, Pedestrian }`
  - `pub struct RoadSegmentSpec { pub street: String, pub axis: RoadAxis, pub at: f32, pub width: f32, pub from: f32, pub to: f32, pub kind: RoadKind }`
  - `pub struct Street { pub name: String, pub axis: RoadAxis, pub at: f32, pub width: f32 }`
  - `MapLayout.segments: Vec<RoadSegmentSpec>`、`MapLayout::streets(&self) -> &[Street]`、`MapLayout::street(&self, name: &str) -> &Street`（未知路名 panic）、私有 `find_street(&self, name) -> Option<&Street>`
  - `pub const SIDEWALK_WIDTH: f32 = 4.0`（`geometry.rs`）

- [ ] **Step 1：資料檔加路網**（加在最後的 `)` 之前）

```ron
    // 路網：每一段路。at 是中線位置（南北向是 X、東西向是 Z），from／to 是沿路方向的起訖
    roads: [
        // 南北向車行道
        (street: "中華路", axis: NorthSouth, at: 80.0, width: 40.0, from: -105.0, to: 75.0, kind: Asphalt),
        (street: "西寧南路", axis: NorthSouth, at: -55.0, width: 12.0, from: -105.0, to: 75.0, kind: Asphalt),
        (street: "康定路", axis: NorthSouth, at: -100.0, width: 16.0, from: -105.0, to: 75.0, kind: Asphalt),
        // 漢中街徒步區：北端是武昌街路緣，南端伸進成都路 0.5 m
        (street: "漢中街", axis: NorthSouth, at: 0.0, width: 15.0, from: -42.5, to: 42.5, kind: Pedestrian),
        // 東西向車行道
        (street: "漢口街", axis: EastWest, at: -80.0, width: 12.0, from: -110.0, to: 90.0, kind: Asphalt),
        (street: "成都路", axis: EastWest, at: 50.0, width: 16.0, from: -110.0, to: 90.0, kind: Asphalt),
        // 東西向徒步區：西寧南路東緣到中華路西緣，避開漢中街分兩段
        (street: "武昌街", axis: EastWest, at: -50.0, width: 15.0, from: -49.0, to: -7.5, kind: Pedestrian),
        (street: "武昌街", axis: EastWest, at: -50.0, width: 15.0, from: 7.5, to: 60.0, kind: Pedestrian),
        (street: "昆明街", axis: EastWest, at: -25.0, width: 8.0, from: -49.0, to: -7.5, kind: Pedestrian),
        (street: "昆明街", axis: EastWest, at: -25.0, width: 8.0, from: 7.5, to: 60.0, kind: Pedestrian),
        (street: "峨嵋街", axis: EastWest, at: 0.0, width: 15.0, from: -49.0, to: -7.5, kind: Pedestrian),
        (street: "峨嵋街", axis: EastWest, at: 0.0, width: 15.0, from: 7.5, to: 60.0, kind: Pedestrian),
    ],
```

- [ ] **Step 2：格式（`file.rs`）**

`MapFile` 加欄位：

```rust
    /// 路網：每一段路（同一條路可以分好幾段），順序就是生成順序
    pub roads: Vec<RoadSegmentSpec>,
```

檔尾加：

```rust
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
pub struct RoadSegmentSpec {
    pub street: String,
    pub axis: RoadAxis,
    pub at: f32,
    pub width: f32,
    pub from: f32,
    pub to: f32,
    pub kind: RoadKind,
}
```

- [ ] **Step 3：`geometry.rs`（新檔）與 `mod.rs`**

```rust
//! 從路網推算位置的規則與常數（純函式，第三段的新地圖直接沿用）

/// 車行道兩側人行道的寬度（第二段固定；斷面欄位第三段才加）
pub const SIDEWALK_WIDTH: f32 = 4.0;
```

`mod.rs`：模組文件加一行「- `geometry`：從路網推算位置的規則」，加 `mod geometry;`、`pub use geometry::*;`。

- [ ] **Step 4：寫測試（`tests.rs` 加入）**

```rust
#[test]
fn road_segments_from_file() {
    let layout = ximending_layout();
    assert_eq!(layout.segments.len(), 12);
    let han = &layout.segments[3];
    assert_eq!(
        (han.street.as_str(), han.at, han.width, han.from, han.to),
        ("漢中街", 0.0, 15.0, -42.5, 42.5)
    );
    let emei: Vec<(f32, f32)> = layout
        .segments
        .iter()
        .filter(|s| s.street == "峨嵋街")
        .map(|s| (s.from, s.to))
        .collect();
    assert_eq!(emei, [(-49.0, -7.5), (7.5, 60.0)]);
}

#[test]
fn streets_merge_segments_of_same_name() {
    let layout = ximending_layout();
    assert_eq!(layout.streets().len(), 9);
    assert_eq!(
        *layout.street("峨嵋街"),
        Street { name: "峨嵋街".to_string(), axis: RoadAxis::EastWest, at: 0.0, width: 15.0 }
    );
}

#[test]
#[should_panic(expected = "地圖沒有「峨眉街」這條路")]
fn street_lookup_panics_on_unknown_name() {
    ximending_layout().street("峨眉街");
}

#[test]
fn rejects_segment_whose_start_is_not_before_end() {
    let mut file = real_file();
    file.roads[3].from = 50.0;
    assert_error(&file, "路段 #3（漢中街）：起點 50 必須小於終點 42.5");
}

#[test]
fn rejects_segment_without_width() {
    let mut file = real_file();
    file.roads[0].width = 0.0;
    assert_error(&file, "路段 #0（中華路）：寬度必須大於 0");
}

#[test]
fn rejects_street_outside_bounds() {
    let mut file = real_file();
    file.roads[0].at = 150.0;
    assert_error(&file, "路段 #0（中華路）：位置 150 在邊界外");
}

#[test]
fn rejects_segments_of_same_street_that_disagree() {
    // 第二段的 Street 只有一個寬度，同名路段的方向、位置、寬度都要一致
    let mut file = real_file();
    file.roads[11].at = 1.0;
    assert_error(&file, "路段 #11（峨嵋街）：和同名路段的方向、位置或寬度不一致");
}
```

Run: `cargo test map_data`
Expected: 編譯失敗（`segments`、`Street`、`street()` 不存在）

- [ ] **Step 5：解析與檢查（`layout.rs`）**

`use super::file::MapFile;` 改成 `use super::file::{MapFile, RoadAxis, RoadSegmentSpec};`。加型別：

```rust
/// 一條路（同名路段合併）：方向、中線位置、寬度
#[derive(Debug, Clone, PartialEq)]
pub struct Street {
    pub name: String,
    pub axis: RoadAxis,
    pub at: f32,
    pub width: f32,
}
```

`MapLayout` 加欄位：

```rust
    /// 路網的每一段，順序同資料檔
    pub segments: Vec<RoadSegmentSpec>,
    /// 同名路段合併成的路，順序依第一次出現
    streets: Vec<Street>,
```

`from_file` 在 `check_bounds_and_spawn(file, &mut errors);` 下一行加 `check_segments(file, &mut errors);`。`base()` 的 `Self { ... }` 加 `segments: file.roads.clone(), streets: build_streets(&file.roads),`。

`impl MapLayout` 加方法：

```rust
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
```

檔尾加：

```rust
/// 每段路的數值、同名路段的一致性
fn check_segments(file: &MapFile, errors: &mut Vec<MapError>) {
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
        let (low, high) = match seg.axis {
            RoadAxis::NorthSouth => (b.min_x, b.max_x),
            RoadAxis::EastWest => (b.min_z, b.max_z),
        };
        if !(low..=high).contains(&seg.at) {
            errors.push(MapError(format!("{tag}：位置 {} 在邊界外", seg.at)));
        }
        let disagrees = file.roads[..i].iter().any(|s| {
            s.street == seg.street && (s.axis, s.at, s.width) != (seg.axis, seg.at, seg.width)
        });
        if disagrees {
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
```

Run: `cargo test map_data`
Expected: 全過

- [ ] **Step 6：道路生成改讀路網（`roads_layout.rs`）**

`setup_roads` 參數加 `layout: &MapLayout`。`use` 改成：

```rust
use crate::world::constants::ROAD_Y;
use crate::world::roads::{spawn_road_segment, RoadType};
use crate::world::{MapLayout, RoadAxis, RoadKind};
```

（取代原本的 `use crate::world::constants::{ROAD_Y, W_ALLEY, ...};` 與 `use crate::world::roads::...`；`setup_roads` 上的 `#[allow(clippy::too_many_lines)]` 拿掉。）檔頭常數：

```rust
/// 徒步區鋪面比車行道高的距離
const PAVING_LIFT: f32 = 0.15;
```

從 `// === 2. 生成完整西門町道路網格 ===` 到函式結尾的 12 個 `spawn_road_segment` 呼叫（含 `hanzhong_len`、`west_len` 等計算），整段換成：

```rust
    // === 2. 路網：資料檔的每一段路 ===
    for seg in &layout.segments {
        let (material, y, road_type) = match seg.kind {
            RoadKind::Asphalt => (road_mat.clone(), ROAD_Y, RoadType::Asphalt),
            RoadKind::Pedestrian => (
                pedestrian_mat.clone(),
                ROAD_Y + PAVING_LIFT,
                RoadType::Pedestrian,
            ),
        };
        let center = f32::midpoint(seg.from, seg.to);
        let length = seg.to - seg.from;
        let (pos, width_x, width_z) = match seg.axis {
            RoadAxis::NorthSouth => (Vec3::new(seg.at, y, center), seg.width, length),
            RoadAxis::EastWest => (Vec3::new(center, y, seg.at), length, seg.width),
        };
        spawn_road_segment(
            commands,
            meshes,
            materials,
            material,
            line_mat.clone(),
            pos,
            width_x,
            width_z,
            road_type,
        );
    }
```

`setup/mod.rs`：`roads_layout::setup_roads(&mut commands, &mut meshes, &mut materials, &asset_server, &layout);`

- [ ] **Step 7：人行道寬統一**

- `src/world/roads.rs`：檔頭加 `use super::map_data::SIDEWALK_WIDTH;`；刪 `spawn_sidewalks` 裡的 `const SIDEWALK_WIDTH: f32 = 4.0;`；`spawn_road_segment` 的 `let sidewalk_width = 4.0;` 刪掉，下一行改成 `let drive_width = layout.total_width - SIDEWALK_WIDTH * 2.0;`
- `src/vehicle/spawning.rs`：刪 `const ROAD_SIDEWALK_WIDTH`（含 doc），`lane_offset` 裡改用 `SIDEWALK_WIDTH`，`use crate::world::{...}` 加 `SIDEWALK_WIDTH`
- `src/pedestrian/behavior.rs`：刪 `setup_ximending` 裡的 `const SIDEWALK_WIDTH: f32 = 4.0;`，`use crate::world::{W_MAIN, Z_CHENGDU};` 加 `SIDEWALK_WIDTH`

- [ ] **Step 8：快照與全套**

Run: `cargo test map_snapshot && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過；快照「被容差吸收 0 行」

- [ ] **Step 9：突變驗證**

`check_segments` 的四個 `if` 各改成 `if false` 一次 → 對應的 `rejects_*` 紅；`build_streets` 的 `if !streets...any(...)` 改成 `if true` → `streets_merge_segments_of_same_name` 紅。編輯還原。

- [ ] **Step 10：送審、等 user 說「提交」**

```
refactor(map): 路網改讀地圖資料檔

- 12 段路（起訖、寬度、路面）進 ximending.ron，道路生成照資料逐段產生；同名路段合併成路，可用路名查位置與寬度
- 檢查起訖、寬度、位置在邊界內、同名路段一致；人行道寬 4 m 統一成 map_data 的常數
```

```bash
git commit -F <訊息檔> -- assets/levels/ximending.ron src/world/map_data src/world/setup/roads_layout.rs src/world/setup/mod.rs src/world/roads.rs src/vehicle/spawning.rs src/pedestrian/behavior.rs
```

---

### Task 6：第 3 步——行人（A* 網格、越界線、逃跑範圍、公車站）

**Files:**
- Modify: `assets/levels/ximending.ron`、`src/world/map_data/{file.rs, layout.rs, geometry.rs, tests.rs}`
- Modify: `src/pedestrian/systems/pathfinding_grid.rs`（`setup_pathfinding_grid`、`astar_movement_system`、測試）
- Modify: `src/pedestrian/systems/lifecycle.rs`（`get_movement_target`、`pedestrian_movement_system`、`pedestrian_despawn_system`、測試）
- Modify: `src/pedestrian/behavior.rs`（`setup_ximending` 參數）

**Interfaces:**
- Consumes: `MapLayout::streets()`、`street()`（Task 5）
- Produces:
  - `pub struct GridSpec { pub origin: (f32, f32), pub width: usize, pub height: usize, pub cell_size: f32 }`、`MapLayout.grid: GridSpec`
  - `MapLayout::outer_road_area(&self) -> Rect`、`pedestrian_area(&self) -> Rect`、`flee_area(&self) -> Rect`、`north_sidewalk_z(&self, name: &str) -> f32`（`Rect` 的 x 是世界 X、y 是世界 Z）
  - `pub const FLEE_INSET: f32 = 5.0`
  - `PointsOfInterest::setup_ximending(bus_stop_z: f32) -> Self`
  - `fn get_movement_target(state, current_pos, patrol, flee: Rect) -> Option<Vec3>`

- [ ] **Step 1：資料檔**

```ron
    // 行人 A* 網格：原點 (x, z)、格數、每格大小（找不到能剛好算出它的規則，照原樣存）
    pathfinding_grid: (origin: (-110.0, -90.0), width: 106, height: 75, cell_size: 2.0),
```

- [ ] **Step 2：格式與常數**

`file.rs` 的 `MapFile` 加 `pub pathfinding_grid: GridSpec,`，檔尾加：

```rust
/// 行人 A* 網格：原點 (x, z)、格數、每格大小
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct GridSpec {
    pub origin: (f32, f32),
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
}
```

`geometry.rs` 加：

```rust
/// 行人逃跑目標離最外圍道路中線的距離
pub const FLEE_INSET: f32 = 5.0;
```

- [ ] **Step 3：寫測試（`tests.rs`）**

```rust
#[test]
fn pathfinding_grid_from_file() {
    assert_eq!(
        ximending_layout().grid,
        GridSpec { origin: (-110.0, -90.0), width: 106, height: 75, cell_size: 2.0 }
    );
}

#[test]
fn pedestrian_area_is_outer_road_centerlines() {
    // 康定路到中華路、漢口街到成都路的中線
    let area = ximending_layout().pedestrian_area();
    assert_eq!((area.min, area.max), (Vec2::new(-100.0, -80.0), Vec2::new(80.0, 50.0)));
}

#[test]
fn flee_area_is_5m_inside_outer_roads() {
    let area = ximending_layout().flee_area();
    assert_eq!((area.min, area.max), (Vec2::new(-95.0, -75.0), Vec2::new(75.0, 45.0)));
}

#[test]
fn bus_stop_on_chengdu_north_sidewalk() {
    // 成都路中線 50，往北半寬 8 再退回人行道一半 2
    assert_eq!(ximending_layout().north_sidewalk_z("成都路"), 44.0);
}
```

Run: `cargo test map_data` → Expected: 編譯失敗

- [ ] **Step 4：`layout.rs`**

`use super::file::{...}` 加 `GridSpec`；`use super::geometry::{FLEE_INSET, SIDEWALK_WIDTH};`。`MapLayout` 加欄位 `pub grid: GridSpec,`；`base()` 加 `grid: file.pathfinding_grid,`。方法：

```rust
    /// 最外圍南北向、東西向道路的中線圍出的矩形（x 是世界 X、y 是世界 Z）
    pub fn outer_road_area(&self) -> Rect {
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for s in &self.streets {
            match s.axis {
                RoadAxis::NorthSouth => {
                    min.x = min.x.min(s.at);
                    max.x = max.x.max(s.at);
                }
                RoadAxis::EastWest => {
                    min.y = min.y.min(s.at);
                    max.y = max.y.max(s.at);
                }
            }
        }
        Rect { min, max }
    }

    /// 行人越界即移除的範圍
    pub fn pedestrian_area(&self) -> Rect {
        self.outer_road_area()
    }

    /// 行人逃跑目標的範圍：最外圍道路中線再內縮 FLEE_INSET
    pub fn flee_area(&self) -> Rect {
        self.outer_road_area().inflate(-FLEE_INSET)
    }

    /// 路北側（−Z）人行道的中線 Z
    pub fn north_sidewalk_z(&self, name: &str) -> f32 {
        let s = self.street(name);
        s.at - (s.width / 2.0 - SIDEWALK_WIDTH / 2.0)
    }
```

Run: `cargo test map_data` → Expected: 全過

- [ ] **Step 5：A* 網格與公車站**

`pathfinding_grid.rs`：刪 `use crate::world::{W_ALLEY, ...};`，加 `use crate::world::{MapLayout, RoadAxis};`。`setup_pathfinding_grid` 整個換成：

```rust
/// 初始化 A* 尋路網格：範圍取自地圖資料，每條路只用位置與寬度、延伸滿整個網格
pub fn setup_pathfinding_grid(mut commands: Commands, layout: Res<MapLayout>) {
    let spec = layout.grid;
    let mut grid = PathfindingGrid {
        origin: Vec3::new(spec.origin.0, 0.0, spec.origin.1),
        width: spec.width,
        height: spec.height,
        cell_size: spec.cell_size,
        walkable: vec![false; spec.width * spec.height], // 預設全部不可通行
    };
    for street in layout.streets() {
        match street.axis {
            RoadAxis::NorthSouth => mark_ns_road(&mut grid, street.at, street.width),
            RoadAxis::EastWest => mark_ew_road(&mut grid, street.at, street.width),
        }
    }
    commands.insert_resource(grid);
    commands.insert_resource(PointsOfInterest::setup_ximending(
        layout.north_sidewalk_z("成都路"),
    ));
}
```

`behavior.rs`：`use crate::world::{SIDEWALK_WIDTH, W_MAIN, Z_CHENGDU};` 整行刪掉；`setup_ximending` 改成：

```rust
    /// 設定西門町行人生成區域；`bus_stop_z` 是公車站所在的人行道中線（成都路北側）
    pub fn setup_ximending(bus_stop_z: f32) -> Self {
```

並刪掉原本計算 `bus_stop_z` 的那一行。

- [ ] **Step 6：越界線與兩份逃跑範圍**

`astar_movement_system` 參數加 `layout: Res<MapLayout>`，`let dt = ...;` 下一行加 `let flee = layout.flee_area();`，夾範圍改成：

```rust
                let clamped = Vec3::new(
                    flee_target.x.clamp(flee.min.x, flee.max.x),
                    flee_target.y,
                    flee_target.z.clamp(flee.min.y, flee.max.y),
                );
```

`lifecycle.rs`：`use crate::world::MapLayout;`。`get_movement_target` 加參數 `flee: Rect`，內部改成同樣的 `clamp(flee.min.x, flee.max.x)`／`clamp(flee.min.y, flee.max.y)`，刪掉「地圖邊界：X: -100 ~ 80…」與兩個「留5公尺邊界緩衝」註解，換成一句「// 逃跑目標限制在最外圍道路內縮 5 m 的範圍」。`pedestrian_movement_system` 參數加 `layout: Res<MapLayout>`，`let dt = ...;` 下一行加 `let flee = layout.flee_area();`，呼叫改成 `get_movement_target(state, current_pos, &patrol, flee)`。

`pedestrian_despawn_system` 參數加 `layout: Res<MapLayout>`；刪 `// 地圖邊界常數（與 setup.rs 一致）` 與四個 `MAP_*` 常數，換成 `let area = layout.pedestrian_area();`；條件改成：

```rust
        if current_pos.x < area.min.x
            || current_pos.x > area.max.x
            || current_pos.z < area.min.y
            || current_pos.z > area.max.y
        {
```

- [ ] **Step 7：行為測試改用真實地圖**（期望值不改）

`lifecycle.rs` 的 `despawn_survivors` 開頭改成：

```rust
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        crate::world::install_map(&mut app);
        app.init_resource::<PedestrianConfig>()
            .add_systems(Update, pedestrian_despawn_system);
```

`flee_target_stays_5m_inside_outer_roads`：`let patrol = ...;` 下一行加 `let flee = crate::world::ximending_layout().flee_area();`，`get_movement_target(&fleeing_from(threat), pos, &patrol)` 改成 `get_movement_target(&fleeing_from(threat), pos, &patrol, flee)`。

`pathfinding_grid.rs` 的 `flee_steps` 開頭改成：

```rust
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        crate::world::install_map(&mut app);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            1.0 / 60.0,
        )))
        .init_resource::<PedestrianConfig>()
        .add_systems(Update, astar_movement_system);
```

- [ ] **Step 8：快照與全套**

Run: `cargo test map_snapshot && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過；快照「被容差吸收 0 行」

- [ ] **Step 9：突變**：`FLEE_INSET` 改 `4.0` → `flee_area_is_5m_inside_outer_roads` 與兩個逃跑行為測試紅；`outer_road_area` 的 `min.x.min(s.at)` 改成 `min.x.max(s.at)` → 越界與逃跑測試紅。編輯還原。

- [ ] **Step 10：送審、等 user 說「提交」**

```
refactor(map): 行人 A* 網格、越界線、逃跑範圍與公車站改由地圖資料推算

- A* 網格範圍進 ximending.ron，可走格由路網推算；越界線取最外圍道路中線，兩份逃跑範圍（有／沒有 A* 路徑的行人）再內縮 5 m
- 公車站位置由成都路北側人行道推算
```

```bash
git commit -F <訊息檔> -- assets/levels/ximending.ron src/world/map_data src/pedestrian/systems/pathfinding_grid.rs src/pedestrian/systems/lifecycle.rs src/pedestrian/behavior.rs
```

---

### Task 7：第 4 步——小地圖、大地圖、GPS

**Files:**
- Modify: `assets/levels/ximending.ron`、`src/world/map_data/{file.rs, layout.rs, tests.rs}`
- Create: `src/ui/map_projection.rs`；Modify: `src/ui/mod.rs`（`mod map_projection;`）
- Modify: `src/ui/minimap.rs`（`update_minimap`、`update_fullmap`、`spawn_map_layer`、`MapDrawCtx`、三個繪圖函式）
- Modify: `src/ui/setup_map.rs`（兩個玩家標記的初始位置、`setup_minimap_hud`、`setup_full_map`）
- Modify: `src/ui/systems.rs:214`（`setup_ui` 加 `layout`）、`src/ui/gps_navigation.rs:254-261`
- Modify: `src/ui/map_snapshot.rs`（`spawn_map_huds` 傳 layout）

**Interfaces:**
- Consumes: `MapLayout::street()`（Task 5）
- Produces:
  - `pub struct MinimapRoadSpec { pub street: String, pub center: f32, pub length: f32 }`、`MapLayout.minimap_roads: Vec<MinimapRoadSpec>`
  - `MapLayout::at(&self, name: &str) -> f32`
  - `pub(crate) struct MapProjection { pub scale: f32, pub offset: Vec2 }`、`MINIMAP`、`FULLMAP`、`project(self, x, z) -> Vec2`、`length(self, meters) -> f32`
  - `spawn_map_layer(parent, proj: MapProjection, road_width_factor: f32, is_fullmap: bool, font: Handle<Font>, layout: &MapLayout)`
  - `setup_minimap_hud(commands, font, layout: &MapLayout)`、`setup_full_map(commands, font, layout: &MapLayout)`

- [ ] **Step 1：資料檔**

```ron
    // 小地圖的道路方塊（自成一套，和世界的路段不同；照原樣存）：沿路方向的中心與長度，寬度與位置取自同名的路
    minimap_roads: [
        (street: "中華路", center: -15.0, length: 180.0),
        (street: "西寧南路", center: -15.0, length: 180.0),
        (street: "康定路", center: -15.0, length: 180.0),
        (street: "漢中街", center: 0.0, length: 100.0),
        (street: "漢口街", center: -10.0, length: 200.0),
        (street: "武昌街", center: -10.0, length: 200.0),
        (street: "昆明街", center: -10.0, length: 200.0),
        (street: "峨嵋街", center: -10.0, length: 200.0),
        (street: "成都路", center: -10.0, length: 200.0),
    ],
```

- [ ] **Step 2：格式、解析、測試**

`file.rs`：`MapFile` 加 `pub minimap_roads: Vec<MinimapRoadSpec>,`，檔尾加：

```rust
/// 小地圖上的一條路：沿路方向的中心與長度；寬度、位置取自同名的路
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct MinimapRoadSpec {
    pub street: String,
    pub center: f32,
    pub length: f32,
}
```

`layout.rs`：`MapLayout` 加 `pub minimap_roads: Vec<MinimapRoadSpec>,`，`base()` 加 `minimap_roads: file.minimap_roads.clone(),`。`from_file` 結尾改成：

```rust
        let layout = Self::base(file);
        layout.check_minimap_roads(&mut errors);
        if errors.is_empty() {
            Ok(layout)
        } else {
            Err(errors)
        }
```

方法：

```rust
    /// 路的中線位置（南北向是 X、東西向是 Z）
    pub fn at(&self, name: &str) -> f32 {
        self.street(name).at
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
```

`tests.rs`：

```rust
#[test]
fn minimap_roads_from_file() {
    let roads = ximending_layout().minimap_roads;
    assert_eq!(roads.len(), 9);
    assert_eq!(roads[3], MinimapRoadSpec { street: "漢中街".to_string(), center: 0.0, length: 100.0 });
}

#[test]
fn rejects_minimap_road_on_unknown_street() {
    let mut file = real_file();
    file.minimap_roads[7].street = "峨眉街".to_string();
    assert_error(&file, "小地圖道路 #7：沒有「峨眉街」這條路");
}

#[test]
fn at_is_street_centerline() {
    let layout = ximending_layout();
    assert_eq!((layout.at("西寧南路"), layout.at("峨嵋街")), (-55.0, 0.0));
}
```

Run: `cargo test map_data` → 先紅（未實作時）、實作後全過

- [ ] **Step 3：投影（新檔 `src/ui/map_projection.rs`）**

```rust
//! 世界座標 → 小地圖／大地圖 UI 座標的投影（全遊戲只有這一份）

use bevy::prelude::*;

/// 投影參數：每公尺幾 px、世界原點落在 UI 的哪裡
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MapProjection {
    pub scale: f32,
    pub offset: Vec2,
}

/// 小地圖：300 × 300 px，0.9 px/m，世界原點在中央
pub(crate) const MINIMAP: MapProjection = MapProjection {
    scale: 0.9,
    offset: Vec2::new(150.0, 150.0),
};

/// 大地圖：1200 × 800 px，2.0 px/m，世界原點在中央
pub(crate) const FULLMAP: MapProjection = MapProjection {
    scale: 2.0,
    offset: Vec2::new(600.0, 400.0),
};

impl MapProjection {
    /// 世界 (x, z) → UI 座標（x 往右、y 往下）；Z 取負號
    pub(crate) fn project(self, x: f32, z: f32) -> Vec2 {
        Vec2::new(x * self.scale + self.offset.x, -z * self.scale + self.offset.y)
    }

    /// 世界長度 → UI 長度
    pub(crate) fn length(self, meters: f32) -> f32 {
        meters * self.scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_projects_to_map_center() {
        assert_eq!(MINIMAP.project(0.0, 0.0), Vec2::new(150.0, 150.0));
        assert_eq!(FULLMAP.project(0.0, 0.0), Vec2::new(600.0, 400.0));
    }

    #[test]
    fn lengths_scale_with_projection() {
        assert!((MINIMAP.length(10.0) - 9.0).abs() < 1e-5);
        assert!((FULLMAP.length(10.0) - 20.0).abs() < 1e-5);
    }
}
```

`src/ui/mod.rs` 加 `mod map_projection;`。

- [ ] **Step 4：玩家標記每幀的投影**

`minimap.rs` 加 `use super::map_projection::{FULLMAP, MINIMAP};`。

`update_minimap`：刪 `map_scale`、`offset_x`、`offset_y` 三行與兩行舊算式（含「Z 軸翻轉：讓北方（正 Z）在上方」註解），換成：

```rust
    // 將 3D 世界座標轉換為小地圖 UI 座標（小地圖 300×300）
    let p = MINIMAP.project(pos.x, pos.z);
    let minimap_x = p.x.clamp(10.0, 290.0);
    let minimap_y = p.y.clamp(10.0, 290.0);
```

`update_fullmap` 同樣換成：

```rust
    // 將 3D 世界座標轉換為大地圖 UI 座標（大地圖 1200×800）
    let p = FULLMAP.project(pos.x, pos.z);
    let map_x = p.x.clamp(20.0, 1180.0);
    let map_y = p.y.clamp(20.0, 780.0);
```

`gps_navigation.rs` 的 `update_minimap_gps_marker` 同樣換成（`use super::map_projection::MINIMAP;`）：

```rust
    // 將世界座標轉換為小地圖座標
    let p = MINIMAP.project(destination.x, destination.z);
    let minimap_x = p.x.clamp(5.0, 295.0);
    let minimap_y = p.y.clamp(5.0, 295.0);
```

- [ ] **Step 5：地圖內容層**

`minimap.rs`：

```rust
/// 地圖繪製上下文：投影與字型
struct MapDrawCtx {
    proj: MapProjection,
    font: Handle<Font>,
}
```

（`use super::map_projection::{MapProjection, FULLMAP, MINIMAP};`）

`spawn_map_layer` 參數改成 `(parent, proj: MapProjection, road_width_factor: f32, is_fullmap: bool, font: Handle<Font>, layout: &MapLayout)`（`use crate::world::{MapLayout, RoadAxis};`），刪掉函式內 `use crate::world::{W_ALLEY, ...};`，`ctx` 改成 `MapDrawCtx { proj, font }`。

道路段落（`// 1. 繪製道路` 到 9 個 `draw_road_rect` 結束）整段換成：

```rust
    // 1. 繪製道路：小地圖自己的道路方塊，寬度與位置取自同名的路
    for road in &layout.minimap_roads {
        let street = layout.street(&road.street);
        let label = if is_fullmap { street.name.as_str() } else { "" };
        let width = street.width * road_width_factor;
        match street.axis {
            RoadAxis::NorthSouth => {
                draw_road_rect(parent, street.at, road.center, width, road.length, &ctx, label);
            }
            RoadAxis::EastWest => {
                draw_road_rect(parent, road.center, street.at, road.length, width, &ctx, label);
            }
        }
    }
```

地標陣列之前加 `let at = |name: &str| layout.at(name);`，13 個地標的 `world_x`／`world_z` 照下表把常數換成查路名（偏移數字不動）：

| 常數 | 換成 |
|---|---|
| `X_XINING` | `at("西寧南路")` |
| `X_HAN` | `at("漢中街")` |
| `X_ZHONGHUA` | `at("中華路")` |
| `X_KANGDING` | `at("康定路")` |
| `Z_EMEI` | `at("峨嵋街")` |
| `Z_WUCHANG` | `at("武昌街")` |
| `Z_KUNMING` | `at("昆明街")` |
| `Z_CHENGDU` | `at("成都路")` |

例：`world_x: X_XINING - 16.0, world_z: Z_EMEI - 17.5,` → `world_x: at("西寧南路") - 16.0, world_z: at("峨嵋街") - 17.5,`

三個繪圖函式改用投影：

```rust
    // draw_road_rect
    let ui_w = ctx.proj.length(width);
    let ui_h = ctx.proj.length(length);
    let center = ctx.proj.project(x, z);
    spawn_centered_rect(parent, center.x, center.y, ui_w, ui_h, Color::srgba(0.5, 0.5, 0.55, 0.6), label, 14.0, ctx.font.clone());
```

`draw_building_rect` 同樣（`ui_w = ctx.proj.length(w)`、`ui_h = ctx.proj.length(d)`、`center = ctx.proj.project(x, z)`）；`draw_minimap_point` 的 `ui_x`／`ui_y` 改成 `let center = ctx.proj.project(x, z); let (ui_x, ui_y) = (center.x, center.y);`。三處「Z 軸翻轉」註解刪掉。

- [ ] **Step 6：HUD 建立函式與初始位置（`setup_map.rs`、`systems.rs`）**

`setup_map.rs` 加 `use super::map_projection::{MapProjection, FULLMAP, MINIMAP};`、`use crate::world::MapLayout;`，以及：

```rust
/// 玩家標記的初始位置：投影世界原點，再減標記的半寬、半高（對齊容器中心）
fn marker_origin(proj: MapProjection, size: Vec2) -> Vec2 {
    proj.project(0.0, 0.0) - size / 2.0
}
```

`spawn_minimap_player_marker` 開頭加 `let origin = marker_origin(MINIMAP, Vec2::new(20.0, 34.0));`，`left: Val::Px(140.0), top: Val::Px(133.0),` → `left: Val::Px(origin.x), top: Val::Px(origin.y),`。`spawn_fullmap_player_marker` 同樣，用 `FULLMAP` 與 `Vec2::new(30.0, 52.0)`（取代 585／374）。

`setup_minimap_hud(commands, font, layout: &MapLayout)`：刪 `mm_scale`、`mm_off_x`、`mm_off_y` 三行，呼叫改成 `spawn_map_layer(parent, MINIMAP, mw_fac, false, font.clone(), layout);`。`setup_full_map(commands, font, layout: &MapLayout)`：刪 `fm_scale`、`fm_off_x`、`fm_off_y`，呼叫改成 `spawn_map_layer(map, FULLMAP, fw_fac, true, font.clone(), layout);`。

`systems.rs` 的 `setup_ui`：

```rust
pub fn setup_ui(
    mut commands: Commands,
    chinese_font: Res<ChineseFont>,
    layout: Res<crate::world::MapLayout>,
) {
```

兩個呼叫加 `&layout`。

`map_snapshot.rs` 的 `spawn_map_huds`：

```rust
fn spawn_map_huds(mut commands: Commands, layout: Res<crate::world::MapLayout>) {
    let font = Handle::<Font>::default();
    super::setup_map::setup_minimap_hud(&mut commands, &font, &layout);
    super::setup_map::setup_full_map(&mut commands, &font, &layout);
}
```

- [ ] **Step 7：快照、標記測試與全套**

Run: `cargo test map_snapshot && cargo test map_marker_tests && cargo test map_projection && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過；快照「被容差吸收 0 行」（UI 節點的位置、Text、兄弟順序都不變）

- [ ] **Step 8：突變**：`MINIMAP` 的 offset 改 `151.0` → `minimap_marker_projection`、`gps_marker_projection`、`origin_projects_to_map_center` 與快照都紅；`check_minimap_roads` 的 `if` 改 `if false` → `rejects_minimap_road_on_unknown_street` 紅。編輯還原。

- [ ] **Step 9：送審、等 user 說「提交」**

```
refactor(ui): 小地圖、大地圖、GPS 的投影合成一份，道路方塊改讀地圖資料

- 新增 ui/map_projection.rs（小地圖 0.9 px/m、大地圖 2.0 px/m），取代散在 5 處的比例與偏移；玩家標記初始位置由投影推算
- 小地圖道路方塊照原樣存進 ximending.ron，地標偏移改用路名查
```

```bash
git commit -F <訊息檔> -- assets/levels/ximending.ron src/world/map_data src/ui/map_projection.rs src/ui/mod.rs src/ui/minimap.rs src/ui/setup_map.rs src/ui/systems.rs src/ui/gps_navigation.rs src/ui/map_snapshot.rs
```

---

### Task 8：第 5 步——斑馬線（路口解析與交會規則）

**Files:**
- Modify: `assets/levels/ximending.ron`、`src/world/map_data/{file.rs, layout.rs, geometry.rs, tests.rs}`
- Modify: `src/world/setup/street_elements.rs`（`setup_zebra_crossings`）、`src/world/setup/mod.rs`（傳 layout）、`src/world/constants.rs`（刪 `ZEBRA_CROSSING_OFFSET`）

**Interfaces:**
- Consumes: `segments`、`find_street`（Task 5）
- Produces:
  - `pub struct Junction { pub center: Vec3, pub ns_width: f32, pub ew_width: f32 }`
  - `MapLayout::junction(&self, a: &str, b: &str) -> Result<Junction, String>`（順序不拘）、私有 `segments_of`、`resolve_junctions(&self, kind: &str, pairs: &[(String, String)], errors) -> Vec<Junction>`
  - `MapLayout.crosswalks: Vec<Junction>`
  - `geometry`：`JUNCTION_EPSILON`、`segment_reaches(from, to, t, slack) -> bool`、`ZEBRA_CROSSING_OFFSET`、`zebra_crossings(junction: &Junction, y: f32) -> [(Vec3, f32, bool); 4]`

- [ ] **Step 1：資料檔**

```ron
    // 有斑馬線的路口：兩條路名（一南北、一東西）
    crosswalks: [
        ("漢中街", "峨嵋街"),
        ("漢中街", "武昌街"),
        ("漢中街", "成都路"),
        ("西寧南路", "峨嵋街"),
        ("西寧南路", "武昌街"),
        ("西寧南路", "成都路"),
    ],
```

`file.rs` 的 `MapFile` 加：

```rust
    /// 有斑馬線的路口：兩條路名（一南北、一東西，順序不拘）
    pub crosswalks: Vec<(String, String)>,
```

- [ ] **Step 2：寫測試（`tests.rs`）**

```rust
#[test]
fn crosswalks_resolve_to_junctions() {
    let layout = ximending_layout();
    assert_eq!(layout.crosswalks.len(), 6);
    assert_eq!(
        layout.crosswalks[2],
        Junction { center: Vec3::new(0.0, 0.0, 50.0), ns_width: 15.0, ew_width: 16.0 }
    );
}

#[test]
fn zebra_crossings_sit_2_5m_outside_the_junction() {
    let j = Junction { center: Vec3::new(0.0, 0.0, 50.0), ns_width: 15.0, ew_width: 16.0 };
    assert_eq!(
        zebra_crossings(&j, 0.06),
        [
            (Vec3::new(0.0, 0.06, 39.5), 15.0, true),
            (Vec3::new(0.0, 0.06, 60.5), 15.0, true),
            (Vec3::new(-10.0, 0.06, 50.0), 16.0, false),
            (Vec3::new(10.0, 0.06, 50.0), 16.0, false),
        ]
    );
}

#[test]
fn junction_accepts_exact_half_width_and_rejects_beyond() {
    // 漢中街北端 −42.5 加武昌街半寬 7.5 剛好到武昌街中線 −50：通過
    assert!(ximending_layout().junction("漢中街", "武昌街").is_ok());
    // 北端退到 −42.4，差 0.1 m（超過 0.01 m 的浮點容差）：報錯
    let mut file = real_file();
    file.roads[3].from = -42.4;
    assert_error(&file, "斑馬線 #1（漢中街×武昌街）：「漢中街」和「武昌街」沒有交會");
}

#[test]
fn rejects_crosswalk_where_east_west_street_stops_short() {
    // 峨嵋街西段從 X −49 起，到不了康定路（X −100，半寬 8）
    let mut file = real_file();
    file.crosswalks.push(("康定路".to_string(), "峨嵋街".to_string()));
    assert_error(&file, "斑馬線 #6（康定路×峨嵋街）：「康定路」和「峨嵋街」沒有交會");
}

#[test]
fn rejects_crosswalk_with_parallel_streets() {
    let mut file = real_file();
    file.crosswalks.push(("漢中街".to_string(), "西寧南路".to_string()));
    assert_error(&file, "斑馬線 #6（漢中街×西寧南路）：「漢中街」和「西寧南路」不是一南北、一東西");
}

#[test]
fn rejects_crosswalk_with_unknown_street() {
    let mut file = real_file();
    file.crosswalks.push(("漢中街".to_string(), "峨眉街".to_string()));
    assert_error(&file, "斑馬線 #6（漢中街×峨眉街）：沒有「峨眉街」這條路");
}
```

Run: `cargo test map_data` → Expected: 編譯失敗

- [ ] **Step 3：`geometry.rs`**

```rust
use bevy::prelude::*;

use super::layout::Junction;

/// 斑馬線中心離交會路路緣的距離
pub const ZEBRA_CROSSING_OFFSET: f32 = 2.5;

/// 判斷交會時的浮點容差（公尺）
pub const JUNCTION_EPSILON: f32 = 0.01;

/// 路段 [from, to] 是否涵蓋 t；兩端各容許 slack（另一條路的半寬：現在的路都停在交會路的路緣附近）
pub fn segment_reaches(from: f32, to: f32, t: f32, slack: f32) -> bool {
    t >= from - slack - JUNCTION_EPSILON && t <= to + slack + JUNCTION_EPSILON
}

/// 一個路口四邊的斑馬線：(中心, 長度, 是否東西向)，順序北、南、西、東
pub fn zebra_crossings(junction: &Junction, y: f32) -> [(Vec3, f32, bool); 4] {
    let c = junction.center;
    let (ns, ew) = (junction.ns_width, junction.ew_width);
    [
        (Vec3::new(c.x, y, c.z - ew / 2.0 - ZEBRA_CROSSING_OFFSET), ns, true),
        (Vec3::new(c.x, y, c.z + ew / 2.0 + ZEBRA_CROSSING_OFFSET), ns, true),
        (Vec3::new(c.x - ns / 2.0 - ZEBRA_CROSSING_OFFSET, y, c.z), ew, false),
        (Vec3::new(c.x + ns / 2.0 + ZEBRA_CROSSING_OFFSET, y, c.z), ew, false),
    ]
}
```

`src/world/constants.rs`：刪 `ZEBRA_CROSSING_OFFSET`（含註解）。

- [ ] **Step 4：`layout.rs`**

`use super::geometry::{segment_reaches, FLEE_INSET, SIDEWALK_WIDTH};`。型別：

```rust
/// 路口：中心（南北向路的 X、東西向路的 Z）與兩條路的寬度
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Junction {
    pub center: Vec3,
    pub ns_width: f32,
    pub ew_width: f32,
}
```

`MapLayout` 加 `pub crosswalks: Vec<Junction>,`；`base()` 加 `crosswalks: Vec::new(),`。`from_file` 的 `let layout = Self::base(file);` 改成：

```rust
        let mut layout = Self::base(file);
        layout.check_minimap_roads(&mut errors);
        layout.crosswalks = layout.resolve_junctions("斑馬線", &file.crosswalks, &mut errors);
```

方法：

```rust
    /// 兩條路（一南北、一東西，順序不拘）的路口。交點要落在雙方各自某一段的範圍內，
    /// 或離段端不超過另一條路的半寬
    pub fn junction(&self, a: &str, b: &str) -> Result<Junction, String> {
        let first = self.find_street(a).ok_or_else(|| format!("沒有「{a}」這條路"))?;
        let second = self.find_street(b).ok_or_else(|| format!("沒有「{b}」這條路"))?;
        let (ns, ew) = match (first.axis, second.axis) {
            (RoadAxis::NorthSouth, RoadAxis::EastWest) => (first, second),
            (RoadAxis::EastWest, RoadAxis::NorthSouth) => (second, first),
            _ => return Err(format!("「{a}」和「{b}」不是一南北、一東西")),
        };
        let ns_reaches = self
            .segments_of(&ns.name)
            .any(|s| segment_reaches(s.from, s.to, ew.at, ew.width / 2.0));
        let ew_reaches = self
            .segments_of(&ew.name)
            .any(|s| segment_reaches(s.from, s.to, ns.at, ns.width / 2.0));
        if ns_reaches && ew_reaches {
            Ok(Junction {
                center: Vec3::new(ns.at, 0.0, ew.at),
                ns_width: ns.width,
                ew_width: ew.width,
            })
        } else {
            Err(format!("「{}」和「{}」沒有交會", ns.name, ew.name))
        }
    }

    fn segments_of<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a RoadSegmentSpec> + 'a {
        self.segments.iter().filter(move |s| s.street == name)
    }

    /// 一串「兩條路名」解析成路口；解析不了的記成錯誤（例如「斑馬線 #2（中華路×成都路）：…」）
    fn resolve_junctions(
        &self,
        kind: &str,
        pairs: &[(String, String)],
        errors: &mut Vec<MapError>,
    ) -> Vec<Junction> {
        let mut junctions = Vec::new();
        for (i, (a, b)) in pairs.iter().enumerate() {
            match self.junction(a, b) {
                Ok(j) => junctions.push(j),
                Err(reason) => errors.push(MapError(format!("{kind} #{i}（{a}×{b}）：{reason}"))),
            }
        }
        junctions
    }
```

- [ ] **Step 5：斑馬線生成（`street_elements.rs`）**

`use crate::world::constants::{...}` 刪 `ZEBRA_CROSSING_OFFSET`、`W_MAIN`、`W_PEDESTRIAN`、`W_SECONDARY`（第 9 步才會用到的其他常數先保留）；加 `use crate::world::{zebra_crossings, MapLayout};`。`setup_zebra_crossings` 整個換成：

```rust
/// 斑馬線生成：資料檔指定的每個路口四邊各一條
pub(super) fn setup_zebra_crossings(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    world_mats: &WorldMaterials,
    layout: &MapLayout,
) {
    let zebra_mat = world_mats.zebra_white.clone();
    let y = ROAD_Y + ROAD_MARKING_Y_OFFSET;
    for junction in &layout.crosswalks {
        for (center, length, is_east_west) in zebra_crossings(junction, y) {
            spawn_zebra_crossing(commands, meshes, &zebra_mat, center, length, is_east_west);
        }
    }
    info!(
        "🦓 已生成 {} 條斑馬線於 {} 個交叉口",
        layout.crosswalks.len() * 4,
        layout.crosswalks.len()
    );
}
```

`setup/mod.rs`：`street_elements::setup_zebra_crossings(&mut commands, &mut meshes, &world_mats, &layout);`

- [ ] **Step 6：測試、快照、全套**

Run: `cargo test map_data && cargo test map_snapshot && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過；快照「被容差吸收 0 行」

- [ ] **Step 7：突變**
- 拿掉 `ns_reaches &&` → `junction_accepts_exact_half_width_and_rejects_beyond` 紅
- 拿掉 `&& ew_reaches` → `rejects_crosswalk_where_east_west_street_stops_short` 紅
- `match` 的 `_ => return Err(...)` 改成 `_ => (first, second),` → `rejects_crosswalk_with_parallel_streets` 紅
- `JUNCTION_EPSILON` 改 `0.0`：**不會紅**——現有數字在 f32 都是精確值；保留它是給第三段 OSM 的小數用。把這個結果寫進 code-reviewer 的說明
- 編輯還原

- [ ] **Step 8：送審、等 user 說「提交」**

```
refactor(map): 斑馬線改用路名指定路口，加上交會檢查

- 6 個路口進 ximending.ron，四邊斑馬線位置由路口與路寬推算
- 路口解析：兩條路要一南北、一東西，交點落在雙方路段內或離段端不超過對方半寬（現在的路都停在交會路的路緣附近）
```

```bash
git commit -F <訊息檔> -- assets/levels/ximending.ron src/world/map_data src/world/setup/street_elements.rs src/world/setup/mod.rs src/world/constants.rs
```

---

### Task 9：第 6 步——紅綠燈

**Files:**
- Modify: `assets/levels/ximending.ron`、`src/world/map_data/{file.rs, layout.rs, tests.rs}`
- Modify: `src/vehicle/traffic_lights.rs:451-500`（`spawn_world_traffic_lights`）

**Interfaces:**
- Consumes: `resolve_junctions`（Task 8）
- Produces: `MapLayout.signals: Vec<Junction>`

- [ ] **Step 1：資料檔**

```ron
    // 有號誌的路口（每個路口四支燈）
    signals: [
        ("西寧南路", "成都路"),
        ("中華路", "成都路"),
        ("西寧南路", "漢口街"),
        ("中華路", "漢口街"),
    ],
```

`file.rs` 的 `MapFile` 加：

```rust
    /// 有號誌的路口：兩條路名
    pub signals: Vec<(String, String)>,
```

- [ ] **Step 2：測試（`tests.rs`）**

```rust
#[test]
fn signals_resolve_to_junctions() {
    let layout = ximending_layout();
    assert_eq!(layout.signals.len(), 4);
    assert_eq!(
        layout.signals[3],
        Junction { center: Vec3::new(80.0, 0.0, -80.0), ns_width: 40.0, ew_width: 12.0 }
    );
}

#[test]
fn rejects_signal_with_unknown_street() {
    let mut file = real_file();
    file.signals.push(("中華路".to_string(), "峨眉街".to_string()));
    assert_error(&file, "號誌 #4（中華路×峨眉街）：沒有「峨眉街」這條路");
}
```

- [ ] **Step 3：解析**：`MapLayout` 加 `pub signals: Vec<Junction>,`，`base()` 加 `signals: Vec::new(),`，`from_file` 在斑馬線那行下面加：

```rust
        layout.signals = layout.resolve_junctions("號誌", &file.signals, &mut errors);
```

- [ ] **Step 4：生成（`traffic_lights.rs`）**

加 `use crate::world::MapLayout;`。`spawn_world_traffic_lights` 整個換成：

```rust
/// 在世界中生成紅綠燈：資料檔指定的每個路口四支
/// 此系統需要在 `setup_traffic_lights` 之後執行
pub fn spawn_world_traffic_lights(
    mut commands: Commands,
    visuals: Option<Res<TrafficLightVisuals>>,
    layout: Res<MapLayout>,
) {
    let Some(visuals) = visuals else {
        warn!("TrafficLightVisuals 資源不存在，無法生成紅綠燈");
        return;
    };

    info!("🚦 正在生成交通燈...");
    for junction in &layout.signals {
        spawn_intersection_lights(
            &mut commands,
            &visuals,
            junction.center,
            junction.ns_width,
            junction.ew_width,
        );
    }
    info!(
        "✅ 已生成 {} 組交通燈（共 {} 個）",
        layout.signals.len(),
        layout.signals.len() * 4
    );
}
```

- [ ] **Step 5：測試、快照、全套**

Run: `cargo test map_data && cargo test map_snapshot && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過；快照「被容差吸收 0 行」

- [ ] **Step 6：突變**：資料檔第四個號誌改成 `("中華路", "武昌街")` → `signals_resolve_to_junctions` 與快照紅。編輯還原。

- [ ] **Step 7：送審、等 user 說「提交」**

```
refactor(vehicle): 紅綠燈改用路名指定路口，刪掉 traffic_lights.rs 自抄的道路常數
```

```bash
git commit -F <訊息檔> -- assets/levels/ximending.ron src/world/map_data src/vehicle/traffic_lights.rs
```

---

### Task 10：第 7 步——NPC 車路線

**Files:**
- Modify: `assets/levels/ximending.ron`、`src/world/map_data/{file.rs, layout.rs, geometry.rs, tests.rs}`
- Modify: `src/vehicle/spawning.rs`（刪 `lane_offset` 與道路常數；`spawn_initial_traffic` 讀路線）

**Interfaces:**
- Consumes: `junction`（Task 8）
- Produces:
  - `pub struct RouteSpec { pub name: String, pub corners: Vec<CornerSpec> }`、`pub struct CornerSpec { pub ns: String, pub ns_lane: f32, pub ew: String, pub ew_lane: f32 }`
  - `pub struct NpcRoute { pub name: String, pub points: Vec<Vec3> }`、`MapLayout.routes`、`MapLayout::route(&self, name: &str) -> &NpcRoute`（未知名 panic）
  - `pub fn lane_offset(total_width: f32) -> f32`（`geometry`）

- [ ] **Step 1：資料檔**

```ron
    // NPC 車路線：依序經過的轉角。車道係數乘上車道偏移（扣掉兩側人行道後車行道寬的 1/4）：
    // ±1 外側車道、±0.5 中間車道、0 道路中線；南北向路 −1 西／+1 東，東西向路 −1 北／+1 南
    npc_routes: [
        (name: "外圈", corners: [
            (ns: "西寧南路", ns_lane: -1.0, ew: "成都路", ew_lane: -1.0),
            (ns: "中華路", ns_lane: 1.0, ew: "成都路", ew_lane: -1.0),
            (ns: "中華路", ns_lane: 1.0, ew: "漢口街", ew_lane: 1.0),
            (ns: "西寧南路", ns_lane: -1.0, ew: "漢口街", ew_lane: 1.0),
        ]),
        (name: "內圈", corners: [
            (ns: "中華路", ns_lane: -1.0, ew: "成都路", ew_lane: 1.0),
            (ns: "西寧南路", ns_lane: 1.0, ew: "成都路", ew_lane: 1.0),
            (ns: "西寧南路", ns_lane: 1.0, ew: "漢口街", ew_lane: -1.0),
            (ns: "中華路", ns_lane: -1.0, ew: "漢口街", ew_lane: -1.0),
        ]),
        (name: "中華路", corners: [
            (ns: "中華路", ns_lane: 0.5, ew: "成都路", ew_lane: 1.0),
            (ns: "中華路", ns_lane: 0.5, ew: "漢口街", ew_lane: -1.0),
            (ns: "中華路", ns_lane: -0.5, ew: "漢口街", ew_lane: -1.0),
            (ns: "中華路", ns_lane: -0.5, ew: "成都路", ew_lane: 1.0),
        ]),
        (name: "成都路西段", corners: [
            (ns: "康定路", ns_lane: 0.0, ew: "成都路", ew_lane: -1.0),
            (ns: "西寧南路", ns_lane: 0.0, ew: "成都路", ew_lane: -1.0),
            (ns: "西寧南路", ns_lane: 0.0, ew: "成都路", ew_lane: 1.0),
            (ns: "康定路", ns_lane: 0.0, ew: "成都路", ew_lane: 1.0),
        ]),
        (name: "康定路", corners: [
            (ns: "康定路", ns_lane: 1.0, ew: "成都路", ew_lane: 1.0),
            (ns: "康定路", ns_lane: 1.0, ew: "漢口街", ew_lane: -1.0),
            (ns: "康定路", ns_lane: -1.0, ew: "漢口街", ew_lane: -1.0),
            (ns: "康定路", ns_lane: -1.0, ew: "成都路", ew_lane: 1.0),
        ]),
    ],
```

- [ ] **Step 2：格式（`file.rs`）**

`MapFile` 加 `pub npc_routes: Vec<RouteSpec>,`；檔尾：

```rust
/// 一條 NPC 車路線：依序經過的轉角
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RouteSpec {
    pub name: String,
    pub corners: Vec<CornerSpec>,
}

/// 路線轉角：兩條路名，各帶一個車道係數
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct CornerSpec {
    pub ns: String,
    pub ns_lane: f32,
    pub ew: String,
    pub ew_lane: f32,
}
```

- [ ] **Step 3：測試（`tests.rs`）**

```rust
#[test]
fn lane_offset_is_quarter_of_driving_width() {
    assert_eq!(lane_offset(16.0), 2.0);
    assert_eq!(lane_offset(12.0), 1.0);
    assert_eq!(lane_offset(40.0), 8.0);
    assert_eq!(lane_offset(6.0), 0.0); // 比兩側人行道還窄
}

#[test]
fn routes_resolve_lane_points() {
    let layout = ximending_layout();
    assert_eq!(layout.routes.len(), 5);
    assert_eq!(
        layout.route("外圈").points,
        [
            Vec3::new(-56.0, 0.0, 48.0),
            Vec3::new(88.0, 0.0, 48.0),
            Vec3::new(88.0, 0.0, -79.0),
            Vec3::new(-56.0, 0.0, -79.0),
        ]
    );
    assert_eq!(layout.route("中華路").points[0], Vec3::new(84.0, 0.0, 52.0));
    assert_eq!(layout.route("成都路西段").points[1], Vec3::new(-55.0, 0.0, 48.0));
}

#[test]
fn rejects_route_corner_that_does_not_meet() {
    let mut file = real_file();
    file.npc_routes[0].corners[0].ns = "康定路".to_string();
    file.npc_routes[0].corners[0].ew = "峨嵋街".to_string();
    assert_error(&file, "NPC 路線「外圈」第 0 個轉角（康定路×峨嵋街）：「康定路」和「峨嵋街」沒有交會");
}

#[test]
fn rejects_route_with_single_corner() {
    let mut file = real_file();
    file.npc_routes[1].corners.truncate(1);
    assert_error(&file, "NPC 路線「內圈」：至少要兩個轉角");
}
```

- [ ] **Step 4：推算與解析**

`geometry.rs`：

```rust
/// 雙向車道的中心離道路中線的距離：扣掉兩側人行道後車行道寬的 1/4
pub fn lane_offset(total_width: f32) -> f32 {
    let drive_width = (total_width - SIDEWALK_WIDTH * 2.0).max(0.0);
    drive_width * 0.25
}
```

`layout.rs`：`use super::file::{...}` 加 `RouteSpec`；`use super::geometry::{lane_offset, ...}`。型別：

```rust
/// NPC 車路線：依序經過的點
#[derive(Debug, Clone, PartialEq)]
pub struct NpcRoute {
    pub name: String,
    pub points: Vec<Vec3>,
}
```

`MapLayout` 加 `pub routes: Vec<NpcRoute>,`、`base()` 加 `routes: Vec::new(),`、`from_file` 加 `layout.routes = layout.resolve_routes(&file.npc_routes, &mut errors);`。方法：

```rust
    /// 依名稱取 NPC 路線（名稱寫在程式裡，打錯字時快照測試會在這裡 panic）
    pub fn route(&self, name: &str) -> &NpcRoute {
        self.routes
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("地圖沒有「{name}」這條 NPC 路線"))
    }

    fn resolve_routes(&self, specs: &[RouteSpec], errors: &mut Vec<MapError>) -> Vec<NpcRoute> {
        let mut routes = Vec::new();
        for spec in specs {
            if spec.corners.len() < 2 {
                errors.push(MapError(format!("NPC 路線「{}」：至少要兩個轉角", spec.name)));
                continue;
            }
            let mut points = Vec::new();
            for (k, corner) in spec.corners.iter().enumerate() {
                match self.junction(&corner.ns, &corner.ew) {
                    Ok(j) => points.push(Vec3::new(
                        j.center.x + corner.ns_lane * lane_offset(j.ns_width),
                        0.0,
                        j.center.z + corner.ew_lane * lane_offset(j.ew_width),
                    )),
                    Err(reason) => errors.push(MapError(format!(
                        "NPC 路線「{}」第 {k} 個轉角（{}×{}）：{reason}",
                        spec.name, corner.ns, corner.ew
                    ))),
                }
            }
            routes.push(NpcRoute { name: spec.name.clone(), points });
        }
        routes
    }
```

- [ ] **Step 5：`spawning.rs`**

刪 `use crate::world::{W_MAIN, ...};`（整行）、`const ROAD_SIDEWALK_WIDTH`、`fn lane_offset`（含 doc）。`spawn_initial_traffic` 參數加 `layout: Res<crate::world::MapLayout>`；從 `// === 道路座標參考（與 world/setup.rs 同步）===` 到 `route_kangding` 結尾，整段換成：

```rust
    // 路線由地圖資料推算（路口加減車道偏移）
    let route = |name: &str| Arc::new(layout.route(name).points.clone());
    let route_outer = route("外圈");
    let route_inner = route("內圈");
    let route_zhonghua = route("中華路");
    let route_chengdu = route("成都路西段");
    let route_kangding = route("康定路");
```

函式開頭那段「可用道路：中華路 (X=75…」的註解刪掉，保留「只走柏油路、車道錯開」的原則說明。

- [ ] **Step 6：測試、快照、全套**

Run: `cargo test map_data && cargo test map_snapshot && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過；快照「被容差吸收 0 行」（`route[...]` 每個點都一樣）

- [ ] **Step 7：突變**：`lane_offset` 的 `0.25` 改 `0.3` → `lane_offset_is_quarter_of_driving_width`、`routes_resolve_lane_points` 與快照紅；`if spec.corners.len() < 2` 改 `if false` → `rejects_route_with_single_corner` 紅。編輯還原。

- [ ] **Step 8：送審、等 user 說「提交」**

```
refactor(vehicle): NPC 車路線改由地圖資料推算

- 5 條路線的轉角寫成兩條路名＋車道係數進 ximending.ron，路線點由路口加減車道偏移推算
```

```bash
git commit -F <訊息檔> -- assets/levels/ximending.ron src/world/map_data src/vehicle/spawning.rs
```

---

### Task 11：第 8 步——建築擺放

**Files:**
- Modify: `assets/levels/ximending.ron`、`src/world/map_data/{file.rs, layout.rs, geometry.rs, tests.rs}`
- Modify: `src/world/setup/buildings_layout.rs`（刪 `RoadSide`、`BuildingSpec`、`spawn_building_at_corner`、`spawn_building_at_linear`；`setup_buildings` 讀資料）、`src/world/setup/mod.rs`（傳 layout）
- Modify: `src/world/constants.rs`（刪 `BUILDING_ROAD_BUFFER`）

**Interfaces:**
- Consumes: `street_on`（本 task 新增）、`junction`（Task 8）
- Produces:
  - `pub enum BuildingEntry { Corner {..}, Along {..}, At {..} }` 與 `BuildingEntry::name(&self) -> &str`
  - `pub struct PlacedBuilding { pub name: String, pub pos: Vec3, pub size: Vec3 }`、`MapLayout.buildings: Vec<PlacedBuilding>`
  - `geometry`：`BUILDING_ROAD_BUFFER`、`ALONG_BUILDING_HEIGHT`、`corner_building_pos(ns: &Street, ns_side: f32, ew: &Street, ew_side: f32, size: Vec3) -> Vec3`、`along_building_pos(street: &Street, side: f32, from_at: f32, to_at: f32, width: f32) -> Vec3`

- [ ] **Step 1：資料檔**（下面 38 筆由 `buildings_layout.rs` 機械轉換：`X_XINING`→西寧南路、`X_HAN`→漢中街、`X_ZHONGHUA`→中華路、`X_KANGDING`→康定路、`Z_HANKOU`→漢口街、`Z_WUCHANG`→武昌街、`Z_KUNMING`→昆明街、`Z_EMEI`→峨嵋街、`Z_CHENGDU`→成都路，`align` 原樣、尺寸原樣；路寬全部和所屬道路一致，不必存）

```ron
    // 建築：單一清單，順序＝生成順序（重疊時先蓋先贏，順序一換被略過的就可能換一棟）
    // Corner：貼兩條路的路緣放在指定的角（ns_side −1 西／+1 東，ew_side −1 北／+1 南），size 是 (寬, 高, 深)
    // Along：沿著 street 放在兩條橫路中間，size 是 (寬, 深)，高度固定
    // At：直接給中心座標，size 是 (寬, 高, 深)
    buildings: [
        Corner(name: "萬年大樓", ns: "西寧南路", ns_side: -1.0, ew: "峨嵋街", ew_side: -1.0, size: (20.0, 28.0, 15.0)),
        Corner(name: "獅子林", ns: "西寧南路", ns_side: -1.0, ew: "武昌街", ew_side: -1.0, size: (22.0, 24.0, 22.0)),
        Corner(name: "電影公園", ns: "西寧南路", ns_side: -1.0, ew: "昆明街", ew_side: -1.0, size: (23.0, 4.0, 18.0)),
        Corner(name: "Don Don Donki", ns: "西寧南路", ns_side: 1.0, ew: "武昌街", ew_side: 1.0, size: (28.0, 35.0, 22.0)),
        Corner(name: "誠品西門", ns: "漢中街", ns_side: -1.0, ew: "峨嵋街", ew_side: -1.0, size: (18.0, 20.0, 16.0)),
        Corner(name: "誠品武昌", ns: "漢中街", ns_side: -1.0, ew: "武昌街", ew_side: 1.0, size: (14.0, 18.0, 14.0)),
        Corner(name: "Uniqlo", ns: "漢中街", ns_side: 1.0, ew: "峨嵋街", ew_side: -1.0, size: (12.0, 15.0, 12.0)),
        Corner(name: "H&M", ns: "漢中街", ns_side: 1.0, ew: "成都路", ew_side: -1.0, size: (14.0, 18.0, 14.0)),
        Corner(name: "捷運6號出口", ns: "中華路", ns_side: -1.0, ew: "成都路", ew_side: -1.0, size: (12.0, 8.0, 12.0)),
        Corner(name: "西門紅樓", ns: "中華路", ns_side: -1.0, ew: "成都路", ew_side: 1.0, size: (22.0, 14.0, 22.0)),
        Corner(name: "錢櫃KTV", ns: "中華路", ns_side: 1.0, ew: "成都路", ew_side: -1.0, size: (16.0, 22.0, 16.0)),
        Corner(name: "鴨肉扁", ns: "中華路", ns_side: -1.0, ew: "武昌街", ew_side: 1.0, size: (10.0, 8.0, 10.0)),
        Corner(name: "新光三越", ns: "中華路", ns_side: -1.0, ew: "峨嵋街", ew_side: -1.0, size: (18.0, 28.0, 16.0)),
        Corner(name: "遠東百貨", ns: "中華路", ns_side: 1.0, ew: "漢口街", ew_side: 1.0, size: (20.0, 25.0, 18.0)),
        Corner(name: "商業大樓A", ns: "中華路", ns_side: 1.0, ew: "武昌街", ew_side: -1.0, size: (14.0, 20.0, 12.0)),
        Corner(name: "西門國小", ns: "康定路", ns_side: 1.0, ew: "漢口街", ew_side: 1.0, size: (28.0, 12.0, 23.0)),
        Corner(name: "7-ELEVEN", ns: "康定路", ns_side: 1.0, ew: "峨嵋街", ew_side: -1.0, size: (12.0, 10.0, 12.0)),
        Corner(name: "全家便利", ns: "西寧南路", ns_side: 1.0, ew: "漢口街", ew_side: 1.0, size: (10.0, 8.0, 10.0)),
        Corner(name: "麥當勞", ns: "漢中街", ns_side: -1.0, ew: "漢口街", ew_side: 1.0, size: (14.0, 12.0, 12.0)),
        Corner(name: "摩斯漢堡", ns: "漢中街", ns_side: 1.0, ew: "漢口街", ew_side: 1.0, size: (10.0, 10.0, 10.0)),
        Corner(name: "大創", ns: "康定路", ns_side: 1.0, ew: "成都路", ew_side: 1.0, size: (12.0, 10.0, 12.0)),
        Corner(name: "彈珠台", ns: "康定路", ns_side: 1.0, ew: "成都路", ew_side: -1.0, size: (14.0, 12.0, 14.0)),
        Along(name: "阿宗麵線", street: "成都路", side: -1.0, between: ("西寧南路", "漢中街"), size: (8.0, 6.0)),
        Along(name: "KFC", street: "漢中街", side: -1.0, between: ("峨嵋街", "成都路"), size: (6.0, 6.0)),
        Along(name: "小吃街", street: "峨嵋街", side: 1.0, between: ("西寧南路", "漢中街"), size: (5.0, 5.0)),
        At(name: "統一元氣館", pos: (50.0, 15.0, 25.0), size: (16.0, 30.0, 14.0)),
        At(name: "國賓影城", pos: (41.0, 16.0, -68.0), size: (22.0, 32.0, 18.0)),
        At(name: "樂聲影城", pos: (36.0, 14.0, -34.0), size: (18.0, 28.0, 16.0)),
        At(name: "日新威秀", pos: (59.0, 15.0, -62.0), size: (20.0, 30.0, 20.0)),
        At(name: "湯姆熊", pos: (40.0, 10.0, -64.0), size: (18.0, 20.0, 15.0)),
        At(name: "肯德基", pos: (-20.0, 6.0, 33.0), size: (10.0, 12.0, 10.0)),
        At(name: "50嵐", pos: (14.0, 4.0, 33.0), size: (6.0, 8.0, 6.0)),
        At(name: "夾娃娃機", pos: (26.0, 5.0, 33.0), size: (8.0, 10.0, 8.0)),
        At(name: "潮牌店", pos: (28.0, 7.0, -10.0), size: (10.0, 14.0, 10.0)),
        At(name: "古著店", pos: (40.0, 6.0, -10.0), size: (8.0, 12.0, 8.0)),
        At(name: "球鞋專賣", pos: (52.0, 7.5, 14.0), size: (12.0, 15.0, 12.0)),
        At(name: "刺青店", pos: (20.0, 6.0, -17.0), size: (8.0, 12.0, 8.0)),
        At(name: "潮流刺青", pos: (30.0, 5.0, -17.0), size: (6.0, 10.0, 6.0)),
    ],
```

（統一元氣館原本寫成 `X_ZHONGHUA - W_ZHONGHUA / 2.0 - 10.0`＝50；直接定位的建築一律存字面座標，第三段整批重蓋。）

貼上後用腳本對照原始碼，確認 22 筆路口建築的名稱、`align`、尺寸與 `buildings_layout.rs` 逐筆相同（例如以 `grep -c 'Corner(' assets/levels/ximending.ron` 為 22、`grep -c 'BuildingSpec {' src/world/setup/buildings_layout.rs` 為 22，再逐筆比對名稱順序）。

- [ ] **Step 2：格式（`file.rs`）**

`MapFile` 加 `pub buildings: Vec<BuildingEntry>,`；檔尾：

```rust
/// 一棟建築的擺法
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum BuildingEntry {
    /// 路口建築：貼兩條路的路緣放在指定的角（−1 西／北，+1 東／南）；size 是 (寬, 高, 深)
    Corner {
        name: String,
        ns: String,
        ns_side: f32,
        ew: String,
        ew_side: f32,
        size: (f32, f32, f32),
    },
    /// 沿街建築：沿著 street 放在兩條橫路中間；size 是 (寬, 深)，高度固定
    Along {
        name: String,
        street: String,
        side: f32,
        between: (String, String),
        size: (f32, f32),
    },
    /// 直接給中心座標；size 是 (寬, 高, 深)
    At {
        name: String,
        pos: (f32, f32, f32),
        size: (f32, f32, f32),
    },
}

impl BuildingEntry {
    /// 建築名稱
    pub fn name(&self) -> &str {
        match self {
            Self::Corner { name, .. } | Self::Along { name, .. } | Self::At { name, .. } => name,
        }
    }
}
```

- [ ] **Step 3：測試（`tests.rs`）**

```rust
#[test]
fn corner_building_hugs_both_curbs() {
    // 萬年大樓：西寧南路西側、峨嵋街北側，各留 1.5 m
    let b = &ximending_layout().buildings[0];
    assert_eq!(
        (b.name.as_str(), b.pos, b.size),
        ("萬年大樓", Vec3::new(-72.5, 14.0, -16.5), Vec3::new(20.0, 28.0, 15.0))
    );
}

#[test]
fn along_building_keeps_current_axis_mixup() {
    // 已知問題（交給第三段）：沿東西向成都路的阿宗麵線被放到 (36.5, −27.5)
    let layout = ximending_layout();
    let b = layout.buildings.iter().find(|b| b.name == "阿宗麵線").unwrap();
    assert_eq!((b.pos, b.size), (Vec3::new(36.5, 10.0, -27.5), Vec3::new(8.0, 20.0, 6.0)));
}

#[test]
fn buildings_keep_generation_order() {
    let names: Vec<String> = ximending_layout().buildings.into_iter().map(|b| b.name).collect();
    assert_eq!(names.len(), 38);
    assert_eq!(
        [names[0].as_str(), names[21].as_str(), names[22].as_str(), names[25].as_str(), names[37].as_str()],
        ["萬年大樓", "彈珠台", "阿宗麵線", "統一元氣館", "潮流刺青"]
    );
}

#[test]
fn rejects_corner_building_with_swapped_axes() {
    let mut file = real_file();
    if let BuildingEntry::Corner { ns, ew, .. } = &mut file.buildings[0] {
        std::mem::swap(ns, ew);
    }
    assert_error(&file, "建築 #0（萬年大樓）：「峨嵋街」的方向不對");
}

#[test]
fn rejects_along_building_whose_cross_street_does_not_meet() {
    // 小吃街沿峨嵋街；峨嵋街西段從 X −49 起，到不了康定路
    let mut file = real_file();
    if let BuildingEntry::Along { between, .. } = &mut file.buildings[24] {
        between.0 = "康定路".to_string();
    }
    assert_error(&file, "建築 #24（小吃街）：「康定路」和「峨嵋街」沒有交會");
}
```

- [ ] **Step 4：推算（`geometry.rs`）**

```rust
use super::layout::{Junction, Street};

/// 建築與路緣之間的緩衝距離
pub const BUILDING_ROAD_BUFFER: f32 = 1.5;

/// 沿街建築的固定高度
pub const ALONG_BUILDING_HEIGHT: f32 = 20.0;

/// 路口建築：貼著兩條路的路緣（加緩衝）放在指定的角；size 是 (寬, 高, 深)
pub fn corner_building_pos(ns: &Street, ns_side: f32, ew: &Street, ew_side: f32, size: Vec3) -> Vec3 {
    let x = ns.at + ns_side * (ns.width / 2.0 + size.x / 2.0 + BUILDING_ROAD_BUFFER);
    let z = ew.at + ew_side * (ew.width / 2.0 + size.z / 2.0 + BUILDING_ROAD_BUFFER);
    Vec3::new(x, size.y / 2.0, z)
}

/// 沿街建築：沿著 street 放在兩條橫路中間
///
/// 已知問題：不分道路方向，一律把 street 的位置當 X、兩條橫路當 Z 範圍——沿東西向道路的建築
/// 因此放錯軸（阿宗麵線、小吃街）。第三段改用建築錨點時整條規則刪除
pub fn along_building_pos(street: &Street, side: f32, from_at: f32, to_at: f32, width: f32) -> Vec3 {
    let x = street.at + side * (street.width / 2.0 + width / 2.0 + BUILDING_ROAD_BUFFER);
    let z = f32::midpoint(from_at, to_at);
    Vec3::new(x, ALONG_BUILDING_HEIGHT / 2.0, z)
}
```

`src/world/constants.rs`：刪 `BUILDING_ROAD_BUFFER`（含註解）。

- [ ] **Step 5：解析（`layout.rs`）**

`use super::file::{...}` 加 `BuildingEntry`；`use super::geometry::{along_building_pos, corner_building_pos, ALONG_BUILDING_HEIGHT, ...}`。型別：

```rust
/// 擺好位置的建築：中心與 (寬, 高, 深)
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedBuilding {
    pub name: String,
    pub pos: Vec3,
    pub size: Vec3,
}
```

`MapLayout` 加 `pub buildings: Vec<PlacedBuilding>,`、`base()` 加 `buildings: Vec::new(),`、`from_file` 加 `layout.buildings = layout.resolve_buildings(&file.buildings, &mut errors);`。方法：

```rust
    fn resolve_buildings(&self, entries: &[BuildingEntry], errors: &mut Vec<MapError>) -> Vec<PlacedBuilding> {
        let mut placed = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            match self.place_building(entry) {
                Ok(b) => placed.push(b),
                Err(reason) => errors.push(MapError(format!("建築 #{i}（{}）：{reason}", entry.name()))),
            }
        }
        placed
    }

    fn place_building(&self, entry: &BuildingEntry) -> Result<PlacedBuilding, String> {
        match entry {
            BuildingEntry::Corner { name, ns, ns_side, ew, ew_side, size } => {
                let ns = self.street_on(ns, RoadAxis::NorthSouth)?;
                let ew = self.street_on(ew, RoadAxis::EastWest)?;
                let size = Vec3::new(size.0, size.1, size.2);
                Ok(PlacedBuilding {
                    name: name.clone(),
                    pos: corner_building_pos(ns, *ns_side, ew, *ew_side, size),
                    size,
                })
            }
            BuildingEntry::Along { name, street, side, between, size } => {
                let main = self.find_street(street).ok_or_else(|| format!("沒有「{street}」這條路"))?;
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

    /// 取指定方向的路
    fn street_on(&self, name: &str, axis: RoadAxis) -> Result<&Street, String> {
        let street = self.find_street(name).ok_or_else(|| format!("沒有「{name}」這條路"))?;
        if street.axis == axis {
            Ok(street)
        } else {
            Err(format!("「{name}」的方向不對"))
        }
    }
```

- [ ] **Step 6：生成（`buildings_layout.rs`）**

刪 `RoadSide`、`BuildingSpec`、`spawn_building_at_corner`、`spawn_building_at_linear`；`setup_buildings` 參數加 `layout: &MapLayout`，從 `// === 3. 地標建築` 到結尾的 `info!("🏢 已新增 …")`，整段換成：

```rust
    // === 地標與商店：資料檔的建築清單，順序＝生成順序（重疊時先蓋先贏）===
    for b in &layout.buildings {
        try_spawn_rich_building(
            commands,
            meshes,
            materials,
            building_tracker,
            b.pos,
            b.size.x,
            b.size.y,
            b.size.z,
            &b.name,
        );
    }
    info!("🏢 已新增 {} 棟建築", layout.buildings.len());
```

`#[allow(clippy::too_many_lines)]` 拿掉；`use crate::world::constants::{...}` 只留 `BuildingTracker`，加 `use crate::world::MapLayout;`。`setup/mod.rs`：`buildings_layout::setup_buildings(&mut commands, &mut meshes, &mut materials, &mut building_tracker, &layout);`

- [ ] **Step 7：測試、快照、全套**

Run: `cargo test map_data && cargo test map_snapshot && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過；快照「被容差吸收 0 行」（蓋出來的 24 棟與被略過的 14 棟都不變）

- [ ] **Step 8：突變**
- `BUILDING_ROAD_BUFFER` 改 `2.0` → `corner_building_hugs_both_curbs`、`along_building_keeps_current_axis_mixup` 與快照紅
- `street_on` 的 `if street.axis == axis` 改成 `if true` → `rejects_corner_building_with_swapped_axes` 紅
- 資料檔把 `At(name: "樂聲影城", ...)` 那行移到 `Along(name: "阿宗麵線", ...)` 之前 → 快照紅：樂聲影城先蓋、改成阿宗麵線被略過（證明三類之間的先後也會影響結果，建築必須是單一清單）
- 每次編輯還原

- [ ] **Step 9：送審、等 user 說「提交」**

```
refactor(world): 建築擺放改讀地圖資料檔

- 38 棟建築（路口 22、沿街 3、直接定位 13）以單一清單進 ximending.ron，順序＝生成順序；位置由路名與路緣推算
- 沿街建築保留現在的錯軸算法（第三段改用建築錨點時刪除），快照不變
```

```bash
git commit -F <訊息檔> -- assets/levels/ximending.ron src/world/map_data src/world/setup/buildings_layout.rs src/world/setup/mod.rs src/world/constants.rs
```

---

### Task 12：第 9 步——其他「道路常數＋偏移」的位置

**Files:**
- Modify: `src/world/map_data/{layout.rs, tests.rs}`（`edge`）
- Modify: `src/world/setup/street_elements.rs`（路燈、塗鴉牆、掩體點呼叫）、`src/world/characters.rs`（掩體點）、`src/world/setup/vehicles_spawn.rs`（停車場）、`src/world/setup/mod.rs`（傳 layout）

**Interfaces:**
- Produces: `MapLayout::edge(&self, name: &str, side: f32) -> f32`；`spawn_cover_points(commands: &mut Commands, layout: &MapLayout)`；`setup_street_furniture(..., layout: &MapLayout)`、`setup_special_elements(..., layout: &MapLayout)`

- [ ] **Step 1：測試與 `edge`**

`tests.rs`：

```rust
#[test]
fn edge_is_curb_on_given_side() {
    let layout = ximending_layout();
    assert_eq!(layout.edge("康定路", -1.0), -108.0);
    assert_eq!(layout.edge("成都路", 1.0), 58.0);
}
```

`layout.rs`：

```rust
    /// 路緣：side = −1 是西側／北側，+1 是東側／南側
    pub fn edge(&self, name: &str, side: f32) -> f32 {
        let street = self.street(name);
        street.at + side * street.width / 2.0
    }
```

Run: `cargo test map_data` → 先紅後綠

- [ ] **Step 2：換掉常數**（數字偏移不動，只把常數換成查路名）

在每個函式開頭加 `let at = |name: &str| layout.at(name);`，照下表替換：

| 常數 | 換成 |
|---|---|
| `X_HAN` | `at("漢中街")` |
| `X_XINING` | `at("西寧南路")` |
| `X_ZHONGHUA` | `at("中華路")` |
| `X_KANGDING` | `at("康定路")` |
| `Z_HANKOU` | `at("漢口街")` |
| `Z_WUCHANG` | `at("武昌街")` |
| `Z_EMEI` | `at("峨嵋街")` |
| `Z_CHENGDU` | `at("成都路")` |

- `street_elements.rs`：`setup_street_furniture` 加參數 `layout: &MapLayout`，路燈陣列照表換；`setup_special_elements` 加參數 `layout: &MapLayout`，塗鴉牆改成：

```rust
    // 塗鴉牆：康定路西側路緣再往西 9.5 m
    let graffiti_pos = Vec3::new(
        layout.edge("康定路", -1.0) - 9.5,
        2.5,
        layout.at("峨嵋街") + 18.0,
    );
```

  `spawn_cover_points(commands);` → `spawn_cover_points(commands, layout);`；`use crate::world::constants::{...}` 只留 `ROAD_MARKING_Y_OFFSET, ROAD_Y`
- `characters.rs`：`spawn_cover_points(commands: &mut Commands, layout: &MapLayout)`，陣列照表換；`use` 刪道路常數、加 `MapLayout`
- `vehicles_spawn.rs`：停車場 `Vec3::new(at("康定路") + 25.0, 10.0, at("峨嵋街") + 20.0)`（`layout.at(...)`）；`use` 刪 `X_KANGDING, Z_EMEI`
- `setup/mod.rs`：`setup_street_furniture(..., &layout)`、`setup_special_elements(..., &layout)`

- [ ] **Step 3：確認 Startup 已沒有道路常數的使用者**

Run: `grep -rnE '\b(X_(ZHONGHUA|HAN|XINING|KANGDING)|Z_(HANKOU|WUCHANG|KUNMING|EMEI|CHENGDU)|W_(ZHONGHUA|MAIN|SECONDARY|PEDESTRIAN|ALLEY))\b' src | grep -v '^src/world/constants.rs'`
Expected: 沒有輸出（若有，照同樣方式換掉）

- [ ] **Step 4：快照與全套**

Run: `cargo test map_snapshot && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`
Expected: 全過；快照「被容差吸收 0 行」

- [ ] **Step 5：突變**：`edge` 的 `/ 2.0` 改 `/ 4.0` → `edge_is_curb_on_given_side` 與快照（塗鴉牆）紅。編輯還原。

- [ ] **Step 6：送審、等 user 說「提交」**

```
refactor(world): 路燈、掩體點、塗鴉牆、停車場改用路名查位置
```

```bash
git commit -F <訊息檔> -- src/world/map_data src/world/setup/street_elements.rs src/world/characters.rs src/world/setup/vehicles_spawn.rs src/world/setup/mod.rs
```

---

### Task 13：第 10 步——收尾：刪道路常數、確認沒有副本、診斷報告、基準截圖比對

**Files:**
- Modify: `src/world/constants.rs`
- Modify: `src/ui/map_snapshot.rs`（加診斷報告）
- Modify: `docs/superpowers/specs/2026-10-05-map-data-driven-design.md`（第四節清單依診斷結果確認或補齊）

- [ ] **Step 1：刪道路常數**

`src/world/constants.rs`：刪 `X_ZHONGHUA`、`X_HAN`、`X_XINING`、`X_KANGDING`、`Z_HANKOU`、`Z_WUCHANG`、`Z_KUNMING`、`Z_EMEI`、`Z_CHENGDU`、`W_ZHONGHUA`、`W_MAIN`、`W_SECONDARY`、`W_PEDESTRIAN`、`W_ALLEY` 與它們的分段註解。模組文件改成：

```rust
//! 世界常數：路面高度、重生高度、地圖邊界型別、建築重疊追蹤
//!
//! 道路位置、寬度、邊界與出生點在地圖資料檔 `assets/levels/ximending.ron`
```

Run: `cargo build`
Expected: 編譯通過（有漏改的地方會在這裡失敗）

- [ ] **Step 2：確認沒有手抄副本**

```bash
grep -rnE '\b(X|Z|W)_(ZHONGHUA|HAN|XINING|KANGDING|HANKOU|WUCHANG|KUNMING|EMEI|CHENGDU|MAIN|SECONDARY|PEDESTRIAN|ALLEY)\b|PLAYER_SPAWN_[XZ]|BUILDING_ROAD_BUFFER|ZEBRA_CROSSING_OFFSET|GROUND_CENTER' src
grep -rnE '(-119\.0|109\.0|-94\.0|64\.0)' src --include='*.rs'
grep -rnE 'clamp\(-?(95|75)\.0' src --include='*.rs'
grep -rnE 'SIDEWALK_WIDTH: f32 = 4\.0|sidewalk_width = 4\.0' src --include='*.rs'
grep -rnE 'scale = (0\.9|2\.0);|off(set)?_[xy] = (150|600|400)\.0' src/ui --include='*.rs'
```

Expected：第一行沒有輸出；其餘只出現在測試的字面期望值（`#[cfg(test)]` 裡）、`map_data/geometry.rs` 的定義、或與地圖無關的同值數字（逐筆判讀）。把指令與判讀結果寫進這個 task 的 commit 說明。

- [ ] **Step 3：診斷報告**（`map_snapshot.rs` 加；不是常駐測試）

```rust
/// spec 第四節的三項檢查：結果記成第三段第 1 批的輸入
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
            (b.name.clone(), Rect::from_center_half_size(Vec2::new(c.x, c.z), half))
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
                crate::world::RoadAxis::NorthSouth => Rect::new(seg.at - half_w, seg.from, seg.at + half_w, seg.to),
                crate::world::RoadAxis::EastWest => Rect::new(seg.from, seg.at - half_w, seg.to, seg.at + half_w),
            };
            if overlaps(*rect, road) {
                println!("{name} 壓到 {}", seg.street);
            }
        }
    }
    println!("== 沒有貼在任何建築上的招牌（離建築外框超過 1 m）==");
    let signs: Vec<(String, Vec3)> = world
        .query::<(&Name, &GlobalTransform)>()
        .iter(world)
        .filter(|(n, _)| n.as_str().starts_with("NeonSign_"))
        .map(|(n, g)| (n.as_str().to_string(), g.translation()))
        .collect();
    for (name, pos) in signs {
        let p = Vec2::new(pos.x, pos.z);
        if !spawned.iter().any(|(_, r)| r.inflate(1.0).contains(p)) {
            println!("{name} @ ({:.1}, {:.1})", pos.x, pos.z);
        }
    }
}
```

Run: `cargo test map_diagnostics_report -- --ignored --nocapture`

對照 spec 第四節「不在第二段修」的清單：被略過的應是誠品西門、誠品武昌、西門國小、日新威秀、樂聲影城、7-ELEVEN、統一元氣館、湯姆熊、50嵐、夾娃娃機、古著店、刺青店、潮流刺青、商業大樓A（14 棟）；重疊至少含峨嵋停車場 × 彈珠台；懸空招牌至少含阿宗麵線。清單有出入就改 spec 第四節，與本 task 一起提交。

- [ ] **Step 4：和基準截圖比對**

照 Task 3 Step 9 再拍一組（同檔名、放在 `<scratchpad>/map-after`），逐張和 `map-baseline` 目視比對：道路、建築、招牌、小地圖、大地圖都要一樣（行人與 NPC 車會動，不比）。

- [ ] **Step 5：全套驗證、送審、等 user 說「提交」**

Run: `cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`

```
refactor(world): 刪除道路常數，地圖數字只剩資料檔一個來源

- constants.rs 移除 X_／Z_／W_ 道路常數；搜尋確認程式碼裡沒有道路位置、路寬、邊界、出生點的手抄副本
- 加上手動執行的診斷報告（被略過的建築、重疊、壓到路面、懸空招牌），結果記成第三段第 1 批的輸入

搜尋指令與判讀：<貼上 Step 2 的結果摘要>
```

```bash
git commit -F <訊息檔> -- src/world/constants.rs src/ui/map_snapshot.rs docs/superpowers/specs/2026-10-05-map-data-driven-design.md
```

---

### Task 14：Bug 1——小地圖、大地圖南北顛倒

**Files:**
- Modify: `src/ui/map_projection.rs`（`project` 與測試）
- Modify: `src/ui/map_marker_tests.rs`（期望值改成修正後的值）
- Modify: `src/world/snapshots/ximending_world.txt`（重新產生）
- Modify: `CHANGELOG.md`

- [ ] **Step 1：寫失敗的測試**（`map_projection.rs` 的 `mod tests`）

```rust
    #[test]
    fn north_is_up() {
        // 漢口街（Z −80，北）要畫在成都路（Z 50，南）上方：UI 的 y 越小越上面
        assert!(MINIMAP.project(0.0, -80.0).y < MINIMAP.project(0.0, 50.0).y);
        assert!(FULLMAP.project(0.0, -80.0).y < FULLMAP.project(0.0, 50.0).y);
    }
```

Run: `cargo test map_projection` → Expected: `north_is_up` FAIL

- [ ] **Step 2：修正投影**

```rust
    /// 世界 (x, z) → UI 座標（x 往右、y 往下）：北（−Z）在上、東（+X）在右
    pub(crate) fn project(self, x: f32, z: f32) -> Vec2 {
        Vec2::new(x * self.scale + self.offset.x, z * self.scale + self.offset.y)
    }
```

- [ ] **Step 3：行為測試的字面期望值改成修正後的值**（`map_marker_tests.rs`）

| 測試 | 原期望 | 改成 | 算式 |
|---|---|---|---|
| `minimap_marker_projection` (20, −7) | top 132.3 | 119.7 | −6.3 + 150 − 24 |
| `minimap_marker_projection` (500, −500) | top 266 | −14 | 夾 10，再 −24 |
| `fullmap_marker_projection` (20, −7) | top 377 | 349 | −14 + 400 − 37 |
| `fullmap_marker_projection` (1000, 1000) | top −17 | 743 | 夾 780，再 −37 |
| `gps_marker_projection` (20, −7) | tops 148.3／152.3 | 135.7／139.7 | −6.3 + 150 − 8／−4 |

left 都不變。測試註解裡的投影點同步改（例如「(20, −7) 投影到 (168, 143.7)」）。

- [ ] **Step 4：重新產生快照並逐行檢查**

Run: `UPDATE_SNAPSHOTS=1 cargo test map_snapshot_matches_golden`（刻意失敗）→ `git diff --stat src/world/snapshots/` → `git diff src/world/snapshots/ | grep '^[-+]' | grep -v '^[-+]ui|' | grep -v '^[-+][-+]'`
Expected：最後一個指令沒有輸出（只有 `ui|` 行變）；變的只有道路方塊、地標方塊、地標點與標籤的 `t`（top）。抽查小地圖漢口街方塊：原本中心 y 222、改後 78（高 7.56，top 218.22 → 74.22）。玩家標記的初始位置不變（原點投影不變）。

- [ ] **Step 5：全套**：`cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`

- [ ] **Step 6：開遊戲截圖確認**：照 Task 3 Step 9 拍小地圖與大地圖，漢口街（北）在上、成都路（南）在下，和實際走動方向一致。

- [ ] **Step 7：CHANGELOG**（`## [Unreleased]` 底下，沒有 `### Fixed` 就新增在 `### Added` 之後）

```markdown
### Fixed

- **小地圖、大地圖南北顛倒**：北（漢口街）改在上方；GPS 目的地標記一起修正
```

- [ ] **Step 8：送審、等 user 說「提交」**

```
fix(ui): 小地圖、大地圖南北顛倒

- 投影改成北（−Z）在上；小地圖、大地圖、地標、玩家標記位置、GPS 標記共用同一份投影，一起修正
```

```bash
git commit -F <訊息檔> -- src/ui/map_projection.rs src/ui/map_marker_tests.rs src/world/snapshots/ximending_world.txt CHANGELOG.md
```

---

### Task 15：Bug 2——玩家箭頭不會轉

**Files:**
- Modify: `src/ui/minimap.rs`（`update_minimap`、`update_fullmap`）
- Modify: `src/ui/map_marker_tests.rs`（加測試）
- Modify: `CHANGELOG.md`

- [ ] **Step 1：寫失敗的測試**（`map_marker_tests.rs`）

```rust
use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use bevy::ui::UiTransform;

/// 玩家面向 facing，跑兩次 update（第一次 dt 是 0；第二次 0.1 s，插值係數 min(10·dt, 1) = 1），回傳箭頭的順時針角度
fn arrow_heading(fullmap: bool, facing: Vec3) -> f32 {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(100)));
    app.world_mut()
        .spawn((Player::default(), Transform::default().looking_to(facing, Vec3::Y)));
    let marker = if fullmap {
        app.add_systems(Update, update_fullmap);
        app.world_mut()
            .spawn((Node::default(), Transform::default(), FullMapPlayerMarker))
            .id()
    } else {
        app.add_systems(Update, update_minimap);
        app.world_mut()
            .spawn((Node::default(), Transform::default(), MinimapPlayerMarker))
            .id()
    };
    app.update();
    app.update();
    app.world().get::<UiTransform>(marker).unwrap().rotation.as_radians()
}

#[test]
fn player_arrow_turns_with_player() {
    // 面向東時箭頭朝右（順時針 90°）。修好前箭頭永遠朝上，所以只有這條會紅；面向北朝上只當輔助
    for fullmap in [false, true] {
        let east = arrow_heading(fullmap, Vec3::X);
        assert!((east - std::f32::consts::FRAC_PI_2).abs() < 1e-3, "fullmap={fullmap} east={east}");
        let north = arrow_heading(fullmap, Vec3::NEG_Z);
        assert!(north.abs() < 1e-3, "fullmap={fullmap} north={north}");
    }
}
```

（`use` 併入檔頭。）

Run: `cargo test map_marker_tests` → Expected: `player_arrow_turns_with_player` FAIL（east = 0）

- [ ] **Step 2：改寫旋轉**（`minimap.rs`）

檔頭 `use bevy::ui::UiTransform;`，加：

```rust
/// 箭頭的旋轉插值速度（每秒）
const MARKER_TURN_SPEED: f32 = 10.0;

/// 玩家面向 → 箭頭的順時針旋轉：北（−Z）朝上為 0，東（+X）朝右為 90°
fn marker_heading(forward: Dir3) -> Rot2 {
    Rot2::radians(forward.x.atan2(-forward.z))
}
```

`update_minimap` 與 `update_fullmap`：查詢的 `(&mut Node, &mut Transform)` 改成 `(&mut Node, &mut UiTransform)`；刪掉 `rotation_angle`、`target_rotation` 兩行與「▲ 預設朝上（北）…forward.z > 0 時箭頭朝上」那段註解，換成 `let target_rotation = marker_heading(forward);`；`if let Ok((mut node, mut transform))` 改成 `if let Ok((mut node, mut ui_transform))`，最後三行換成：

```rust
        // UI 只讀 UiTransform（不讀 3D Transform）；平滑插值
        let t = (MARKER_TURN_SPEED * time.delta_secs()).min(1.0);
        ui_transform.rotation = ui_transform.rotation.slerp(target_rotation, t);
```

標記生成時的 `Transform::default()` 不動（不在這個 bug 的範圍）。

- [ ] **Step 3：跑測試**：`cargo test map_marker_tests` → Expected: 全過；`cargo test map_snapshot` → Expected: 快照不變（UI 行不記旋轉）

- [ ] **Step 4：全套**：`cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`

- [ ] **Step 5：開遊戲確認**：照 Task 3 Step 9，按住 `KeyW`＋`KeyD` 2000 ms（只按 D 是平移、角色不會轉向）後截小地圖，箭頭要指向玩家面向（和鏡頭轉過去的方向一致）；開大地圖再截一次。

- [ ] **Step 6：CHANGELOG**（`### Fixed` 底下加一行）

```markdown
- **小地圖、大地圖的玩家箭頭不會轉**：旋轉改寫到 UI 的 `UiTransform`（Bevy 0.17 的 UI 不讀 3D `Transform`），箭頭跟著玩家面向轉
```

- [ ] **Step 7：送審、等 user 說「提交」**

```
fix(ui): 小地圖、大地圖的玩家箭頭不會轉

- 旋轉從 3D Transform 改寫到 UiTransform；角度以北（−Z）朝上為 0、順時針
```

```bash
git commit -F <訊息檔> -- src/ui/minimap.rs src/ui/map_marker_tests.rs CHANGELOG.md
```

---

### Task 16：疑似 bug——行人越界線在最外圍道路的中線

**Files:**
- Modify: `src/world/map_data/{layout.rs, tests.rs}`（`pedestrian_area`）
- Modify: `src/pedestrian/systems/lifecycle.rs`（越界測試期望值）
- Modify: `src/pedestrian/systems/pathfinding_grid.rs`（加網格外的測試）
- Modify: `CHANGELOG.md`

- [ ] **Step 1：確認是不是 bug**

A* 網格的可走格包含康定路、中華路、漢口街、成都路的整條路寬（`mark_ns_road`／`mark_ew_road` 用全寬），所以 X −108〜−100、80〜100、Z −86〜−80、50〜58 都可以走，行人走到這些地方就會被越界線移除。用快照金檔確認：`grep 'res|grid_row|' src/world/snapshots/ximending_world.txt | sed -n 10,12p`。不在東西向道路上的列，開頭應是 `.1 W8`：第 0 格（X −110〜−108）不可走，第 1〜8 格（X −108〜−92，康定路全寬）可走，其中第 1〜4 格在 X −100 以西。

- 有：是 bug，繼續 Step 2
- 沒有：不是 bug，跳到 Step 7，只在 spec 第四節記下判斷依據

- [ ] **Step 2：寫失敗的測試**

`lifecycle.rs` 的越界測試期望值改成地圖邊界（−119／109／−94／64）：

```rust
    #[test]
    fn pedestrians_beyond_map_bounds_are_removed() {
        // 越界線是地圖邊界：界線內 0.5 m 留下、界線外 0.5 m 移除；玩家站在界線上
        let cases = [
            (Vec3::new(-119.0, 0.0, 0.0), Vec3::new(-118.5, 0.0, 0.0), Vec3::new(-119.5, 0.0, 0.0)),
            (Vec3::new(109.0, 0.0, 0.0), Vec3::new(108.5, 0.0, 0.0), Vec3::new(109.5, 0.0, 0.0)),
            (Vec3::new(0.0, 0.0, -94.0), Vec3::new(0.0, 0.0, -93.5), Vec3::new(0.0, 0.0, -94.5)),
            (Vec3::new(0.0, 0.0, 64.0), Vec3::new(0.0, 0.0, 63.5), Vec3::new(0.0, 0.0, 64.5)),
        ];
        for (player, inside, outside) in cases {
            assert_eq!(despawn_survivors(player, &[inside, outside]), [true, false], "{player}");
        }
    }
```

（取代 `pedestrians_beyond_outer_road_centerlines_are_removed`。）`tests.rs` 的 `pedestrian_area_is_outer_road_centerlines` 改成：

```rust
#[test]
fn pedestrian_area_is_map_bounds() {
    let area = ximending_layout().pedestrian_area();
    assert_eq!((area.min, area.max), (Vec2::new(-119.0, -94.0), Vec2::new(109.0, 64.0)));
}
```

`pathfinding_grid.rs` 加 Review Focus 第 5 點的測試：

```rust
    #[test]
    fn pedestrian_outside_grid_falls_back() {
        // 越界線放寬到地圖邊界後，行人可能在 A* 網格（X 從 −110 起）外：找不到路時改走 5 m 的備援點，不卡死
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        crate::world::install_map(&mut app);
        app.add_systems(Startup, setup_pathfinding_grid)
            .add_systems(Update, astar_path_calculation_system);
        let ped = app
            .world_mut()
            .spawn((Pedestrian, Transform::from_xyz(-115.0, 0.0, 0.0), AStarPath::new(Vec3::ZERO)))
            .id();
        app.update();
        let path = app.world().get::<AStarPath>(ped).unwrap();
        assert_eq!(path.waypoints.len(), 1, "{:?}", path.waypoints);
    }
```

Run: `cargo test lifecycle::tests && cargo test map_data` → Expected: 越界的兩條 FAIL；`cargo test pathfinding_grid::tests` → `pedestrian_outside_grid_falls_back` 應已 PASS（記錄它在修法前就成立）

- [ ] **Step 3：修正**（`layout.rs`）

```rust
    /// 行人越界即移除的範圍：地圖邊界（x 是世界 X、y 是世界 Z）
    pub fn pedestrian_area(&self) -> Rect {
        let b = &self.bounds;
        Rect::new(b.min_x, b.min_z, b.max_x, b.max_z)
    }
```

（`flee_area` 仍取 `outer_road_area()` 內縮 5 m，不跟著改。）

- [ ] **Step 4：跑測試**：`cargo test lifecycle::tests && cargo test map_data && cargo test pathfinding_grid::tests && cargo test map_snapshot` → 全過；快照不變（越界線只在執行期用）

- [ ] **Step 5：全套**：`cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --check`

- [ ] **Step 6：CHANGELOG**（`### Fixed`）

```markdown
- **走在外圍道路外側那半邊的行人會突然消失**：行人的越界線從外圍道路中線改成地圖邊界
```

- [ ] **Step 7：送審、等 user 說「提交」**

```
fix(pedestrian): 行人越界線改成地圖邊界

- 原本以康定路、中華路、漢口街、成都路的中線為界，走在這四條路外側那半邊的行人會被立刻移除
```

```bash
git commit -F <訊息檔> -- src/world/map_data src/pedestrian/systems/lifecycle.rs src/pedestrian/systems/pathfinding_grid.rs CHANGELOG.md
```

（Step 1 判定不是 bug 時，改成只提交 spec 的說明：`docs(map): 行人越界線維持外圍道路中線（理由）`。）

---

### Task 17：文件同步

**Files:**
- Modify: `README.md`（測試數）、`.claude/rules/test-coverage.md`（測試數）、`.claude/rules/architecture.md`（world 的檔案數）、`CHANGELOG.md`（`### Changed`）
- Modify: `CLAUDE.md`（關鍵檔案表；用 claude-md-management skill，先給 user 看 diff，核准才改）

- [ ] **Step 1：量測試數與檔案數**

Run: `cargo nextest run 2>&1 | tail -3`（取 Summary 的測試數）；`find src/world -name '*.rs' | wc -l`

- [ ] **Step 2：更新**
- `README.md`、`.claude/rules/test-coverage.md`：照現有寫法更新測試數（test-coverage 的每模組數字照「查法」更新，不寫無法維持的快照數字）
- `.claude/rules/architecture.md`：world 方塊的「25 個檔案」改成量到的數字
- `CHANGELOG.md` `### Changed` 加：

```markdown
- **地圖改由資料檔驅動**：道路、斑馬線、號誌、NPC 車路線、建築、行人網格、小地圖道路與邊界改讀 `assets/levels/ximending.ron`，程式從路網推算位置；讀檔時檢查路名與交會，寫錯時啟動直接報錯並指出是哪一筆
```

- [ ] **Step 3：CLAUDE.md（先問 user）**

用 claude-md-management skill 提出這一列，user 核准才改：

```diff
+| 地圖資料    | `src/world/map_data/`（資料檔 `assets/levels/ximending.ron`；快照 `cargo test map_snapshot`） |
```

- [ ] **Step 4：送審、等 user 說「提交」**

```
docs: 同步地圖資料驅動後的測試數、檔案數與關鍵檔案
```

```bash
git commit -F <訊息檔> -- README.md .claude/rules/test-coverage.md .claude/rules/architecture.md CHANGELOG.md CLAUDE.md
```

---

## 與 spec 的差異（實作計畫的決定）

- **RON 不另寫匯出測試**：spec 第一節說「數量多的先改成型別清單、再由測試輸出成 RON」。計畫改成直接給出由原始碼機械轉換的 RON（路網的起訖由原本的算式算出、建築由腳本逐筆轉換並核對），省掉寫完就刪的中間程式碼；寫錯一樣由快照抓
- **三項檢查改在第 10 步跑**：spec 寫「第 0 步完成後」。那時還沒有解析後的建築清單與路段，改在收尾時用 `MapLayout` 跑，結果更完整；用途（第三段第 1 批的輸入）不變
- **更新模式一定失敗**：`UPDATE_SNAPSHOTS=1` 寫完金檔後測試刻意失敗，要不帶變數再跑一次才綠（Review Focus 第 4 點）
- **同名路段的寬度也要一致**：spec 只寫方向與位置；第二段的 `Street` 只有一個寬度，所以寬度也納入檢查
- **小地圖等標記測試另放 `ui/map_marker_tests.rs`**：`minimap.rs` 已 732 行，加測試會超過 800 行上限
