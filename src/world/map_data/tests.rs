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
