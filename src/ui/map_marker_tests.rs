//! 小地圖、大地圖、GPS 標記的投影（期望值一律寫字面數字）

use std::f32::consts::{FRAC_PI_2, PI};
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy::ui::UiTransform;

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
    // (20, −7) 投影到 (168, 143.7)，容器左上角再減 (10, 24)；X、Z 不對稱，軸對調會被抓到
    let (left, top) = player_marker_at(false, Vec3::new(20.0, 0.0, -7.0));
    assert!(approx(left, 158.0) && approx(top, 119.7), "({left}, {top})");
    // 超出範圍時夾在 10〜290：兩個對角各一點，四個邊都碰到
    let (left, top) = player_marker_at(false, Vec3::new(500.0, 0.0, -500.0));
    assert!(approx(left, 280.0) && approx(top, -14.0), "({left}, {top})");
    let (left, top) = player_marker_at(false, Vec3::new(-500.0, 0.0, 500.0));
    assert!(approx(left, 0.0) && approx(top, 266.0), "({left}, {top})");
}

#[test]
fn fullmap_marker_projection() {
    // (20, −7) 投影到 (640, 386)，容器左上角再減 (15, 37)
    let (left, top) = player_marker_at(true, Vec3::new(20.0, 0.0, -7.0));
    assert!(approx(left, 625.0) && approx(top, 349.0), "({left}, {top})");
    // 超出範圍時 X 夾在 20〜1180、Y 夾在 20〜780：兩個對角各一點，四個邊都碰到（兩軸範圍不同，對調會紅）
    let (left, top) = player_marker_at(true, Vec3::new(1000.0, 0.0, 1000.0));
    assert!(
        approx(left, 1165.0) && approx(top, 743.0),
        "({left}, {top})"
    );
    let (left, top) = player_marker_at(true, Vec3::new(-1000.0, 0.0, -1000.0));
    assert!(approx(left, 5.0) && approx(top, -17.0), "({left}, {top})");
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
    // (20, −7) 投影到 (168, 143.7)；外圈脈衝左上角減 8、核心點減 4
    let m = gps_markers(Vec3::new(20.0, 0.0, -7.0));
    assert!(
        approx(m[0].0, 160.0)
            && approx(m[0].1, 135.7)
            && approx(m[1].0, 164.0)
            && approx(m[1].1, 139.7),
        "{m:?}"
    );
    // 超出範圍時夾在 5〜295：兩個對角各一點，四個邊都碰到
    let m = gps_markers(Vec3::new(300.0, 0.0, 300.0));
    assert!(
        approx(m[0].0, 287.0)
            && approx(m[0].1, 287.0)
            && approx(m[1].0, 291.0)
            && approx(m[1].1, 291.0),
        "{m:?}"
    );
    let m = gps_markers(Vec3::new(-300.0, 0.0, -300.0));
    assert!(
        approx(m[0].0, -3.0) && approx(m[0].1, -3.0) && approx(m[1].0, 1.0) && approx(m[1].1, 1.0),
        "{m:?}"
    );
}

/// 玩家旋轉 rotation，跑兩次 update（第一次 dt 是 0；第二次 0.1 s，插值係數 min(10·dt, 1) = 1），回傳箭頭的順時針角度
fn arrow_heading(fullmap: bool, rotation: Quat) -> f32 {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
    app.world_mut()
        .spawn((Player::default(), Transform::from_rotation(rotation)));
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
    app.world()
        .get::<UiTransform>(marker)
        .unwrap()
        .rotation
        .as_radians()
}

#[test]
fn player_arrow_turns_with_player() {
    // 角色模型正面是本地 +Z（見 update_character_rotation）：往北走時玩家的旋轉是 yaw 180°，箭頭朝上；
    // yaw 90° 面向東，箭頭朝右（順時針 90°）
    for fullmap in [false, true] {
        let north = arrow_heading(fullmap, Quat::from_rotation_y(PI));
        assert!(north.abs() < 1e-3, "fullmap={fullmap} north={north}");
        let east = arrow_heading(fullmap, Quat::from_rotation_y(FRAC_PI_2));
        assert!(
            (east - FRAC_PI_2).abs() < 1e-3,
            "fullmap={fullmap} east={east}"
        );
    }
}
