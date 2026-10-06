//! 地圖資料：讀真實資料檔、檢查規則

use bevy::prelude::*;

use super::*;
use crate::world::{MapBounds, WorldPlugin};

/// 真實資料檔解析成 `MapFile`，給「改一個欄位看會不會報錯」的測試用
pub(super) fn real_file() -> MapFile {
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

pub(super) fn assert_error(file: &MapFile, needle: &str) {
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
fn pedestrian_area_is_map_bounds() {
    let area = ximending_layout().pedestrian_area();
    assert_eq!(
        (area.min, area.max),
        (Vec2::new(-119.0, -94.0), Vec2::new(109.0, 64.0))
    );
}

#[test]
fn flee_area_is_5m_inside_map_bounds() {
    // 越界線（地圖邊界 X −119〜109、Z −94〜64）再往內 5 m
    let area = ximending_layout().flee_area();
    assert_eq!(
        (area.min, area.max),
        (Vec2::new(-114.0, -89.0), Vec2::new(104.0, 59.0))
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

#[test]
fn minimap_roads_from_file() {
    let roads = ximending_layout().minimap_roads;
    assert_eq!(roads.len(), 9);
    assert_eq!(
        roads[3],
        MinimapRoadSpec {
            street: "漢中街".to_string(),
            center: 0.0,
            length: 100.0
        }
    );
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

#[test]
fn rejects_unknown_field_in_minimap_road() {
    let text = super::XIMENDING_RON.replacen(
        "(street: \"中華路\", center:",
        "(street: \"中華路\", lenght: 1.0, center:",
        1,
    );
    let errors = load_map(&text).unwrap_err();
    assert!(errors[0].0.contains("lenght"), "{errors:?}");
}

#[test]
fn crosswalks_resolve_to_junctions() {
    let layout = ximending_layout();
    assert_eq!(layout.crosswalks.len(), 6);
    assert_eq!(
        layout.crosswalks[2],
        Junction {
            center: Vec3::new(0.0, 0.0, 50.0),
            ns_width: 15.0,
            ew_width: 16.0,
            // 漢中街到成都路就停了，南邊沒有路
            arms: [true, false, true, true],
        }
    );
}

#[test]
fn zebra_crossings_sit_2_5m_outside_the_junction() {
    // 西寧南路×成都路：X、Z 都不是 0，寬度也不同，中心與兩條路寬各自對到正確的軸
    let j = Junction {
        center: Vec3::new(-55.0, 0.0, 50.0),
        ns_width: 12.0,
        ew_width: 16.0,
        arms: [true; 4],
    };
    assert_eq!(
        zebra_crossings(&j, 0.06),
        vec![
            (Vec3::new(-55.0, 0.06, 39.5), 12.0, true),
            (Vec3::new(-55.0, 0.06, 60.5), 12.0, true),
            (Vec3::new(-63.5, 0.06, 50.0), 16.0, false),
            (Vec3::new(-46.5, 0.06, 50.0), 16.0, false),
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
    assert_error(
        &file,
        "斑馬線 #1（漢中街×武昌街）：「漢中街」和「武昌街」沒有交會",
    );
}

#[test]
fn rejects_crosswalk_where_east_west_street_stops_short() {
    // 峨嵋街西段從 X −49 起，到不了康定路（X −100，半寬 8）
    let mut file = real_file();
    file.crosswalks
        .push(("康定路".to_string(), "峨嵋街".to_string()));
    assert_error(
        &file,
        "斑馬線 #6（康定路×峨嵋街）：「康定路」和「峨嵋街」沒有交會",
    );
}

#[test]
fn rejects_crosswalk_with_parallel_streets() {
    let mut file = real_file();
    file.crosswalks
        .push(("漢中街".to_string(), "西寧南路".to_string()));
    assert_error(
        &file,
        "斑馬線 #6（漢中街×西寧南路）：「漢中街」和「西寧南路」不是一南北、一東西",
    );
}

#[test]
fn rejects_crosswalk_with_unknown_street() {
    let mut file = real_file();
    file.crosswalks
        .push(("漢中街".to_string(), "峨眉街".to_string()));
    assert_error(&file, "斑馬線 #6（漢中街×峨眉街）：沒有「峨眉街」這條路");
}

#[test]
fn crosswalk_junction_has_both_streets_on_their_own_axes() {
    let expected = Junction {
        center: Vec3::new(-55.0, 0.0, 50.0),
        ns_width: 12.0,
        ew_width: 16.0,
        arms: [true; 4],
    };
    let layout = ximending_layout();
    assert_eq!(layout.crosswalks[5], expected);
    // 順序不拘：東西向寫在前也是同一個路口
    assert_eq!(layout.junction("西寧南路", "成都路"), Ok(expected));
    assert_eq!(layout.junction("成都路", "西寧南路"), Ok(expected));
}

#[test]
fn junction_slack_is_the_other_streets_half_width() {
    // 昆明街寬 8、停在 X ±7.5：等於漢中街的半寬（通過），但大於昆明街自己的半寬 4
    assert!(ximending_layout().junction("漢中街", "昆明街").is_ok());
}

#[test]
fn segment_reaches_tolerates_float_error_at_the_curb() {
    // OSM 那種小數：實數上剛好碰到路緣，f32 下差 1 ulp，要靠 0.01 m 的容差
    assert!(segment_reaches(0.15, 30.0, -8.0, 16.3 / 2.0));
    assert!(segment_reaches(-100.0, -64.05, -60.0, 8.1 / 2.0));
}

#[test]
fn rejects_duplicate_crosswalk_junction() {
    // 順序對調也是同一個路口；號誌共用這段解析，重複的話會多蓋一組燈
    let mut file = real_file();
    file.crosswalks
        .push(("峨嵋街".to_string(), "漢中街".to_string()));
    assert_error(&file, "斑馬線 #6（峨嵋街×漢中街）：和 #0 是同一個路口");
}

#[test]
fn signals_resolve_to_junctions() {
    let layout = ximending_layout();
    assert_eq!(layout.signals.len(), 4);
    assert_eq!(
        layout.signals[3],
        Junction {
            center: Vec3::new(80.0, 0.0, -80.0),
            ns_width: 40.0,
            ew_width: 12.0,
            // 漢口街東端 X 90 沒超過中華路東側路緣 X 100
            arms: [true, true, true, false],
        }
    );
}

#[test]
fn rejects_signal_with_unknown_street() {
    let mut file = real_file();
    file.signals
        .push(("中華路".to_string(), "峨眉街".to_string()));
    assert_error(&file, "號誌 #4（中華路×峨眉街）：沒有「峨眉街」這條路");
}

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
    assert_eq!(
        layout.route("成都路西段").points[1],
        Vec3::new(-55.0, 0.0, 48.0)
    );
}

#[test]
fn rejects_route_corner_that_does_not_meet() {
    let mut file = real_file();
    file.npc_routes[0].corners[0].ns = "康定路".to_string();
    file.npc_routes[0].corners[0].ew = "峨嵋街".to_string();
    assert_error(
        &file,
        "NPC 路線 #0「外圈」轉角 #0（康定路×峨嵋街）：「康定路」和「峨嵋街」沒有交會",
    );
}

#[test]
fn rejects_route_with_single_corner() {
    let mut file = real_file();
    file.npc_routes[1].corners.truncate(1);
    assert_error(&file, "NPC 路線 #1「內圈」：至少要兩個轉角");
}

#[test]
fn rejects_route_corner_with_streets_swapped() {
    // 車道係數依欄位套到 ns／ew：路名寫反時要報錯，不能默默套到另一條路
    let mut file = real_file();
    let corner = &mut file.npc_routes[0].corners[0];
    std::mem::swap(&mut corner.ns, &mut corner.ew);
    assert_error(
        &file,
        "NPC 路線 #0「外圈」轉角 #0（成都路×西寧南路）：「成都路」的方向不對",
    );
}

#[test]
fn rejects_route_corner_with_north_south_street_as_ew() {
    let mut file = real_file();
    file.npc_routes[0].corners[0].ew = "中華路".to_string();
    assert_error(
        &file,
        "NPC 路線 #0「外圈」轉角 #0（西寧南路×中華路）：「中華路」的方向不對",
    );
}

#[test]
#[should_panic(expected = "地圖沒有「峨眉街」這條 NPC 路線")]
fn route_lookup_panics_on_unknown_name() {
    ximending_layout().route("峨眉街");
}

#[test]
fn rejects_duplicate_route_name_pointing_at_the_first() {
    // route() 只會回傳第一條，後面同名的路線永遠拿不到；每條都指向最早那條
    let mut file = real_file();
    file.npc_routes[1].name = "外圈".to_string();
    file.npc_routes[2].name = "外圈".to_string();
    assert_error(&file, "NPC 路線 #1「外圈」：和 #0 同名");
    assert_error(&file, "NPC 路線 #2「外圈」：和 #0 同名");
}

#[test]
fn duplicate_route_still_checks_its_corners() {
    // 同名不中斷檢查：同一輪就把那條路線自己的錯誤一起報出來
    let mut file = real_file();
    file.npc_routes[1].name = "外圈".to_string();
    file.npc_routes[1].corners[0].ns = "無名街".to_string();
    assert_error(&file, "NPC 路線 #1「外圈」：和 #0 同名");
    assert_error(
        &file,
        "NPC 路線 #1「外圈」轉角 #0（無名街×成都路）：沒有「無名街」這條路",
    );
}

#[test]
fn corner_building_hugs_both_curbs() {
    // 萬年大樓：西寧南路西側、峨嵋街北側，各留 1.5 m
    let b = &ximending_layout().buildings[0];
    assert_eq!(
        (b.name.as_str(), b.pos, b.size),
        (
            "萬年大樓",
            Vec3::new(-72.5, 14.0, -16.5),
            Vec3::new(20.0, 28.0, 15.0)
        )
    );
}

#[test]
fn along_building_keeps_current_axis_mixup() {
    // 已知問題（尚未修）：沿東西向成都路的阿宗麵線被放到 (36.5, −27.5)，成都路的位置被當成 X
    let layout = ximending_layout();
    let b = layout
        .buildings
        .iter()
        .find(|b| b.name == "阿宗麵線")
        .unwrap();
    assert_eq!(
        (b.pos, b.size),
        (Vec3::new(36.5, 10.0, -27.5), Vec3::new(8.0, 20.0, 6.0))
    );
}

#[test]
fn buildings_keep_generation_order() {
    let names: Vec<String> = ximending_layout()
        .buildings
        .into_iter()
        .map(|b| b.name)
        .collect();
    assert_eq!(names.len(), 38);
    assert_eq!(
        [
            names[0].as_str(),
            names[21].as_str(),
            names[22].as_str(),
            names[25].as_str(),
            names[37].as_str()
        ],
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
fn rejects_corner_building_whose_ew_street_runs_north_south() {
    // 只換 ew：ns 檢查會過，要靠 ew 自己的方向檢查擋下（否則漢中街的 X 會被當成 Z）
    let mut file = real_file();
    if let BuildingEntry::Corner { ew, .. } = &mut file.buildings[0] {
        *ew = "漢中街".to_string();
    }
    assert_error(&file, "建築 #0（萬年大樓）：「漢中街」的方向不對");
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

#[test]
fn rejects_unknown_field_in_building() {
    let text = super::XIMENDING_RON.replacen(
        "Corner(name: \"萬年大樓\", ns:",
        "Corner(name: \"萬年大樓\", hight: 1.0, ns:",
        1,
    );
    let errors = load_map(&text).unwrap_err();
    assert!(errors[0].0.contains("hight"), "{errors:?}");
}

#[test]
fn edge_is_curb_on_given_side() {
    let layout = ximending_layout();
    assert_eq!(layout.edge("康定路", -1.0), -108.0);
    assert_eq!(layout.edge("成都路", 1.0), 58.0);
    // 寬度要取自那條路本身（中華路 40 m），兩側都要對
    assert_eq!(layout.edge("中華路", -1.0), 60.0);
    assert_eq!(layout.edge("中華路", 1.0), 100.0);
}

#[test]
fn junction_arms_follow_the_road_network() {
    let layout = ximending_layout();
    let arms = |a: &str, b: &str| layout.junction(a, b).unwrap().arms;
    // 北、南、西、東：那一側有沒有路伸出交會路的路緣
    assert_eq!(arms("漢中街", "峨嵋街"), [true, true, true, true]);
    assert_eq!(arms("漢中街", "武昌街"), [false, true, true, true]);
    assert_eq!(arms("漢中街", "成都路"), [true, false, true, true]);
    assert_eq!(arms("西寧南路", "峨嵋街"), [true, true, false, true]);
    assert_eq!(arms("西寧南路", "成都路"), [true, true, true, true]);
}

#[test]
fn zebra_crossings_skip_sides_without_a_road() {
    // T 字路口：漢中街×武昌街北邊沒有漢中街，只畫南、西、東
    let j = ximending_layout().junction("漢中街", "武昌街").unwrap();
    let crossings = zebra_crossings(&j, 0.06);
    assert_eq!(crossings.len(), 3);
    assert!(
        crossings.iter().all(|(c, _, _)| c.z > -60.0),
        "{crossings:?}"
    );
}

#[test]
fn junction_arm_ignores_float_overshoot_at_the_far_curb() {
    // 路段剛好停在交會路的遠側路緣：多出 0.005 m 的浮點誤差不算那一側有路（容差 0.01 m），四個方向各一例
    let cases: [(usize, f32, bool, (&str, &str), [bool; 4]); 4] = [
        // 漢中街北端伸到武昌街北側路緣 −57.5 再多一點
        (
            3,
            -57.505,
            true,
            ("漢中街", "武昌街"),
            [false, true, true, true],
        ),
        // 漢中街南端停在成都路南側路緣 58 再多一點
        (
            3,
            58.005,
            false,
            ("漢中街", "成都路"),
            [true, false, true, true],
        ),
        // 武昌街西段西端停在西寧南路西側路緣 −61 再多一點
        (
            6,
            -61.005,
            true,
            ("西寧南路", "武昌街"),
            [true, true, false, true],
        ),
        // 漢口街東端停在中華路東側路緣 100 再多一點
        (
            4,
            100.005,
            false,
            ("中華路", "漢口街"),
            [true, true, true, false],
        ),
    ];
    for (road, value, is_from, (a, b), expected) in cases {
        let mut file = real_file();
        if is_from {
            file.roads[road].from = value;
        } else {
            file.roads[road].to = value;
        }
        let layout = MapLayout::from_file(&file).unwrap();
        assert_eq!(layout.junction(a, b).unwrap().arms, expected, "{a}×{b}");
    }
}

#[test]
fn junction_arm_needs_a_segment_that_reaches_the_junction() {
    // 峨嵋街東段改從 X 30 起：漢中街東側路緣（X 7.5）到 30 之間沒有路，遠處那一段不算東邊有路
    let mut file = real_file();
    file.roads[11].from = 30.0;
    let layout = MapLayout::from_file(&file).unwrap();
    let j = layout.junction("漢中街", "峨嵋街").unwrap();
    assert_eq!(j.arms, [true, true, true, false]);
    assert_eq!(zebra_crossings(&j, 0.06).len(), 3);
}
