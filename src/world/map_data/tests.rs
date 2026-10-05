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
    assert_eq!(
        (b.min_x, b.max_x, b.min_z, b.max_z),
        (-119.0, 109.0, -94.0, 64.0)
    );
    assert_eq!(layout.spawn, Vec2::new(5.0, -5.0));
}

#[test]
fn ground_from_file() {
    let layout = ximending_layout();
    assert_eq!(layout.ground_center, Vec3::new(-10.0, 0.0, -15.0));
    assert_eq!(
        layout.ground_collider_half_extents,
        Vec3::new(200.0, 0.1, 200.0)
    );
}

#[test]
fn walls_sit_1m_outside_bounds_centered_on_ground() {
    let along_z = Vec3::new(0.5, 20.0, 100.0);
    let along_x = Vec3::new(130.0, 20.0, 0.5);
    assert_eq!(
        ximending_layout().walls,
        [
            WallBox {
                center: Vec3::new(110.0, 10.0, -15.0),
                half_extents: along_z
            },
            WallBox {
                center: Vec3::new(-120.0, 10.0, -15.0),
                half_extents: along_z
            },
            WallBox {
                center: Vec3::new(-10.0, 10.0, 65.0),
                half_extents: along_x
            },
            WallBox {
                center: Vec3::new(-10.0, 10.0, -95.0),
                half_extents: along_x
            },
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

#[test]
fn rejects_unknown_field() {
    // 多寫或拼錯的欄位要報錯，不能默默忽略
    let text = super::XIMENDING_RON.replacen("spawn:", "spwan: (0.0, 0.0),\n    spawn:", 1);
    let errors = load_map(&text).unwrap_err();
    assert!(errors[0].0.contains("spwan"), "{errors:?}");
}

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
        Street {
            name: "峨嵋街".to_string(),
            axis: RoadAxis::EastWest,
            at: 0.0,
            width: 15.0
        }
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
    // Street 只記一個方向、位置、寬度，所以同名路段三者都要一致
    let mut file = real_file();
    file.roads[11].at = 1.0;
    assert_error(
        &file,
        "路段 #11（峨嵋街）：和同名路段的方向、位置或寬度不一致",
    );
}

#[test]
fn rejects_east_west_street_outside_z_bounds() {
    // 100 在 X 範圍內、Z 範圍外：東西向的路要用 Z 邊界檢查
    let mut file = real_file();
    file.roads[4].at = 100.0;
    assert_error(&file, "路段 #4（漢口街）：位置 100 在邊界外");
}

#[test]
fn street_on_the_boundary_is_allowed() {
    let mut file = real_file();
    file.roads[0].at = 109.0;
    assert_eq!(errors_of(&file), Vec::<String>::new());
}

#[test]
fn rejects_same_street_with_different_width_or_axis() {
    let mut file = real_file();
    file.roads[11].width = 8.0;
    assert_error(&file, "路段 #11（峨嵋街）：和同名路段");
    let mut file = real_file();
    file.roads[11].axis = RoadAxis::NorthSouth;
    assert_error(&file, "路段 #11（峨嵋街）：和同名路段");
}

#[test]
fn same_street_is_compared_with_its_first_segment() {
    // 第三段和第一段一致時不該被報，只報真正不一致的那段
    let mut file = real_file();
    let mut third = file.roads[10].clone();
    third.from = 61.0;
    third.to = 70.0;
    file.roads.push(third);
    file.roads[11].at = 1.0;
    let errors = errors_of(&file);
    assert!(errors.iter().any(|e| e.contains("路段 #11")), "{errors:?}");
    assert!(!errors.iter().any(|e| e.contains("路段 #12")), "{errors:?}");
}

#[test]
fn rejects_asphalt_narrower_than_both_sidewalks() {
    // 柏油路扣掉兩側各 4 m 人行道後車道寬會變負的
    let mut file = real_file();
    file.roads[4].width = 6.0;
    assert_error(&file, "路段 #4（漢口街）：柏油路寬 6 要大於兩側人行道 8");
}

#[test]
fn inverted_bounds_do_not_cascade_into_segment_errors() {
    // 邊界本身錯了就不再拿它檢查路段，免得列出一串其實沒錯的路段
    let mut file = real_file();
    file.bounds.min_x = 200.0;
    assert_eq!(errors_of(&file).len(), 1, "{:?}", errors_of(&file));
}

#[test]
fn rejects_unknown_field_in_segment() {
    let text = super::XIMENDING_RON.replacen(
        "(street: \"中華路\", axis:",
        "(street: \"中華路\", lenght: 1.0, axis:",
        1,
    );
    let errors = load_map(&text).unwrap_err();
    assert!(errors[0].0.contains("lenght"), "{errors:?}");
}

#[test]
fn pathfinding_grid_from_file() {
    assert_eq!(
        ximending_layout().grid,
        GridSpec {
            origin: (-110.0, -90.0),
            width: 106,
            height: 75,
            cell_size: 2.0
        }
    );
}

#[test]
fn pedestrian_area_is_outer_road_centerlines() {
    // 康定路到中華路、漢口街到成都路的中線
    let area = ximending_layout().pedestrian_area();
    assert_eq!(
        (area.min, area.max),
        (Vec2::new(-100.0, -80.0), Vec2::new(80.0, 50.0))
    );
}

#[test]
fn flee_area_is_5m_inside_outer_roads() {
    let area = ximending_layout().flee_area();
    assert_eq!(
        (area.min, area.max),
        (Vec2::new(-95.0, -75.0), Vec2::new(75.0, 45.0))
    );
}

#[test]
fn bus_stop_on_chengdu_north_sidewalk() {
    // 成都路中線 50，往北半寬 8 再退回人行道一半 2
    assert_eq!(ximending_layout().north_sidewalk_z("成都路"), 44.0);
}

#[test]
fn rejects_degenerate_pathfinding_grid() {
    // 格子大小是除數：0、負數、NaN 會讓網格全通或全不通
    for cell_size in [0.0, -2.0, f32::NAN] {
        let mut file = real_file();
        file.pathfinding_grid.cell_size = cell_size;
        assert_error(&file, "A* 網格：格子大小要大於 0");
    }
    let mut file = real_file();
    file.pathfinding_grid.width = 0;
    assert_error(&file, "A* 網格：格數要大於 0");
}
