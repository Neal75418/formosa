//! GPS 導航系統 (GTA5 風格)
//!
//! 提供導航方向指示和距離顯示

use bevy::prelude::*;
use bevy::ui::UiTransform;

use super::components::{
    ChineseFont, GpsDirectionArrow, GpsDistanceDisplay, GpsNavigationState, GpsTurnDirection,
    GpsTurnIndicator, MinimapContainer, MinimapGpsMarker,
};
use super::map_projection::{heading, MINIMAP};
use crate::mission::{MissionManager, MissionType};
use crate::player::Player;

// ============================================================================
// GPS 顏色常數
// ============================================================================
const GPS_MARKER_COLOR: Color = Color::srgba(1.0, 0.85, 0.0, 0.9); // 黃色目標點

/// 設置 GPS UI 元素
pub fn setup_gps_ui(mut commands: Commands, font: Option<Res<ChineseFont>>) {
    let Some(font) = font else { return };

    // 屏幕頂部的方向指示箭頭（在小地圖上方）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(330.0),   // 小地圖下方
                right: Val::Px(145.0), // 居中於小地圖
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            BorderRadius::all(Val::Px(20.0)),
            Visibility::Hidden,
            GpsDirectionArrow,
            Name::new("GPS_DirectionArrow"),
        ))
        .with_children(|parent| {
            // 箭頭符號 ▲
            parent.spawn((
                Text::new("▲"),
                TextFont {
                    font: font.font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(GPS_MARKER_COLOR),
            ));
        });

    // 轉彎提示（在方向箭頭旁邊）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(330.0),
                right: Val::Px(85.0), // 方向箭頭左側
                width: Val::Px(50.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            BorderRadius::all(Val::Px(6.0)),
            Visibility::Hidden,
            GpsTurnIndicator,
            Name::new("GPS_TurnIndicator"),
        ))
        .with_children(|parent| {
            // 轉彎方向符號
            parent.spawn((
                Text::new("↑"),
                TextFont {
                    font: font.font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            // 距離文字
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.8)),
            ));
        });

    // 距離顯示（在方向箭頭下方）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(375.0),
                right: Val::Px(120.0),
                width: Val::Px(90.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            BorderRadius::all(Val::Px(4.0)),
            Visibility::Hidden,
            GpsDistanceDisplay,
            Name::new("GPS_DistanceDisplay"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("0 m"),
                TextFont {
                    font: font.font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// 目的地在玩家面向的哪一側：順時針角度，範圍 (−π, π]（正的在右、負的在左）
fn calculate_gps_direction_angle(facing: Vec3, to_dest: Vec3) -> f32 {
    (heading(to_dest) * heading(facing).inverse()).as_radians()
}

/// 格式化 GPS 距離顯示
pub fn format_gps_distance(distance_xz: f32) -> String {
    if distance_xz >= 1000.0 {
        let km = distance_xz / 1000.0;
        format!("{km:.1} km")
    } else {
        format!("{distance_xz:.0} m")
    }
}

/// 更新 GPS 導航狀態
pub fn update_gps_navigation(
    time: Res<Time>,
    mut gps: ResMut<GpsNavigationState>,
    player_query: Query<&Transform, With<Player>>,
    mut arrow_query: Query<
        (&mut Visibility, &mut UiTransform),
        (With<GpsDirectionArrow>, Without<Player>),
    >,
    mut distance_query: Query<
        (&mut Visibility, &Children),
        (
            With<GpsDistanceDisplay>,
            Without<GpsDirectionArrow>,
            Without<Player>,
        ),
    >,
    mut text_query: Query<&mut Text>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let facing = Player::facing(player_transform);

    // 更新冷卻計時器
    if gps.route_recalc_cooldown > 0.0 {
        gps.route_recalc_cooldown -= time.delta_secs();
    }

    // 如果導航未啟用，隱藏 UI
    let should_hide = gps.destination.is_none() || !gps.active;
    if should_hide {
        for (mut vis, _) in &mut arrow_query {
            *vis = Visibility::Hidden;
        }
        for (mut vis, _) in &mut distance_query {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let Some(destination) = gps.destination else {
        return;
    };

    // 計算距離和方向
    let to_dest = destination - player_pos;
    let distance_xz = (to_dest.x.powi(2) + to_dest.z.powi(2)).sqrt();
    gps.distance_to_target = distance_xz;

    // 檢查是否到達目標
    if gps.is_at_destination(player_pos, 5.0) {
        gps.clear();
        return;
    }

    // 計算方向角度並更新箭頭
    let angle = calculate_gps_direction_angle(facing, to_dest);
    for (mut vis, mut ui_transform) in &mut arrow_query {
        *vis = Visibility::Visible;
        ui_transform.rotation = Rot2::radians(angle);
    }

    // 計算轉彎方向
    let turn_dir = GpsTurnDirection::from_angle(angle);
    gps.next_turn_direction = turn_dir;
    gps.distance_to_next_turn = distance_xz.min(50.0); // 簡化：以直線距離作為轉彎距離

    // 到達臨近時標記為 Arrived
    if distance_xz < 10.0 {
        gps.next_turn_direction = GpsTurnDirection::Arrived;
    }

    // 更新距離顯示
    let distance_str = format_gps_distance(distance_xz);
    for (mut vis, children) in &mut distance_query {
        *vis = Visibility::Visible;
        for child in children {
            let Ok(mut text) = text_query.get_mut(*child) else {
                continue;
            };
            (**text).clone_from(&distance_str);
        }
    }
}

/// 更新小地圖上的 GPS 目標標記
/// 優化：只在目標變化時重建標記，避免每幀 despawn/spawn 造成抖動
pub fn update_minimap_gps_marker(
    mut commands: Commands,
    gps: Res<GpsNavigationState>,
    minimap_query: Query<Entity, With<MinimapContainer>>,
    mut marker_query: Query<(Entity, &mut Node, &mut Visibility), With<MinimapGpsMarker>>,
) {
    // 如果 GPS 未啟用或無目標，隱藏現有標記
    if !gps.active || gps.destination.is_none() {
        for (_, _, mut vis) in &mut marker_query {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let Some(destination) = gps.destination else {
        return;
    };

    // 將世界座標轉換為小地圖座標
    let p = MINIMAP.project(destination.x, destination.z);
    let minimap_x = p.x.clamp(5.0, 295.0);
    let minimap_y = p.y.clamp(5.0, 295.0);

    // 收集現有標記
    let markers: Vec<_> = marker_query.iter_mut().collect();

    // 如果標記已存在，只更新位置和可見性（避免每幀重建）
    if markers.len() >= 2 {
        let mut iter = markers.into_iter();
        // 外圈脈衝（第一個標記）
        if let Some((_, mut node, mut vis)) = iter.next() {
            node.left = Val::Px(minimap_x - 8.0);
            node.top = Val::Px(minimap_y - 8.0);
            *vis = Visibility::Visible;
        }
        // 核心點（第二個標記）
        if let Some((_, mut node, mut vis)) = iter.next() {
            node.left = Val::Px(minimap_x - 4.0);
            node.top = Val::Px(minimap_y - 4.0);
            *vis = Visibility::Visible;
        }
        return;
    }

    // 標記不存在，需要創建（只在首次或標記被清理後）
    // 先清理可能的殘留
    for (entity, _, _) in marker_query.iter() {
        commands.entity(entity).despawn();
    }

    // 找到小地圖容器
    let Ok(minimap_entity) = minimap_query.single() else {
        return;
    };

    // 在小地圖上生成目標標記（黃色圓點 + 脈衝效果）
    commands.entity(minimap_entity).with_children(|parent| {
        // 外圈脈衝
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(minimap_x - 8.0),
                top: Val::Px(minimap_y - 8.0),
                width: Val::Px(16.0),
                height: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 0.85, 0.0, 0.3)),
            BorderRadius::all(Val::Px(8.0)),
            Visibility::Visible,
            MinimapGpsMarker,
        ));

        // 核心點
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(minimap_x - 4.0),
                top: Val::Px(minimap_y - 4.0),
                width: Val::Px(8.0),
                height: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(GPS_MARKER_COLOR),
            BorderRadius::all(Val::Px(4.0)),
            Visibility::Visible,
            MinimapGpsMarker,
        ));
    });
}

/// 根據任務類型設置 GPS 目標
fn set_gps_for_mission(gps: &mut GpsNavigationState, mission: &crate::mission::ActiveMission) {
    let data = &mission.data;

    match data.mission_type {
        MissionType::Delivery => {
            if mission.picked_up {
                gps.set_destination(data.end_pos, "送貨目的地");
            } else {
                gps.set_destination(data.start_pos, "取餐點");
            }
        }
        MissionType::Taxi => {
            if let Some(taxi_data) = &data.taxi_data {
                let (pos, name) = if taxi_data.passenger_picked_up {
                    (data.end_pos, taxi_data.destination_name.as_str())
                } else {
                    (data.start_pos, "接乘客")
                };
                gps.set_destination(pos, name);
            }
        }
        MissionType::Race => {
            if let Some(race_data) = &data.race_data {
                if let Some(cp) = race_data.current_checkpoint_pos() {
                    gps.set_destination(
                        cp,
                        &format!("檢查點 {}", race_data.current_checkpoint + 1),
                    );
                }
            }
        }
        MissionType::Explore
        | MissionType::Assassination
        | MissionType::Escort
        | MissionType::ChaseDown
        | MissionType::Photography => {
            gps.set_destination(data.end_pos, "目標位置");
        }
    }
}

/// 檢查是否應該清除任務導航
fn should_clear_mission_gps(destination_name: &str) -> bool {
    destination_name.contains("目的地")
        || destination_name.contains("檢查點")
        || destination_name.contains("乘客")
}

/// 處理任務開始時自動設置 GPS 目標
pub fn gps_mission_integration(
    mut gps: ResMut<GpsNavigationState>,
    mission_manager: Res<MissionManager>,
) {
    if let Some(mission) = &mission_manager.active_mission {
        if !gps.active {
            set_gps_for_mission(&mut gps, mission);
        }
    } else if gps.active && should_clear_mission_gps(&gps.destination_name) {
        gps.clear();
    }
}

/// 更新 GPS 轉彎提示 UI
pub fn update_gps_turn_indicator(
    gps: Res<GpsNavigationState>,
    mut turn_query: Query<(&mut Visibility, &Children), With<GpsTurnIndicator>>,
    mut text_query: Query<&mut Text>,
) {
    for (mut vis, children) in &mut turn_query {
        if !gps.active || gps.destination.is_none() {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Visible;
        let mut child_iter = children.iter();

        // 更新方向符號
        if let Some(child) = child_iter.next() {
            if let Ok(mut text) = text_query.get_mut(child) {
                **text = gps.next_turn_direction.symbol().to_string();
            }
        }

        // 更新距離文字
        if let Some(child) = child_iter.next() {
            if let Ok(mut text) = text_query.get_mut(child) {
                **text = gps.next_turn_direction.label().to_string();
            }
        }
    }
}

pub(super) struct GpsNavigationPlugin;

impl Plugin for GpsNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_gps_ui.in_set(super::UiSetup))
            .add_systems(
                Update,
                (
                    update_gps_navigation,
                    update_minimap_gps_marker,
                    gps_mission_integration,
                    update_gps_turn_indicator.after(update_gps_navigation),
                )
                    .in_set(super::UiActive),
            );
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use bevy::ui::UiTransform;

    use super::*;

    /// 玩家站在原點、旋轉 rotation，目的地在 destination；跑一次 update，
    /// 回傳 (箭頭顯示與否, 箭頭的順時針角度, 轉彎提示)
    fn gps_after_update(rotation: Quat, destination: Vec3) -> (Visibility, f32, GpsTurnDirection) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(GpsNavigationState {
                active: true,
                destination: Some(destination),
                ..default()
            })
            .add_systems(Update, update_gps_navigation);
        app.world_mut()
            .spawn((Player::default(), Transform::from_rotation(rotation)));
        let arrow = app
            .world_mut()
            .spawn((Node::default(), Visibility::Hidden, GpsDirectionArrow))
            .id();
        app.update();
        let world = app.world();
        (
            *world.get::<Visibility>(arrow).unwrap(),
            world
                .get::<UiTransform>(arrow)
                .unwrap()
                .rotation
                .as_radians(),
            world.resource::<GpsNavigationState>().next_turn_direction,
        )
    }

    /// 和正北（−Z）夾 offset 弧度的水平方向：offset 正的往東（順時針）
    fn from_north(offset: f32) -> Vec3 {
        Vec3::new(offset.sin(), 0.0, -offset.cos())
    }

    #[test]
    fn direction_angle_takes_the_near_side() {
        let west = Vec3::new(-100.0, 0.0, 0.0);
        let east = Vec3::new(100.0, 0.0, 0.0);
        // (面向, 目的地方向, 期望角度, 期望提示)；角度都要落在 (−π, π]，不能因為跨過某個方向就變成「迴轉」
        let cases = [
            // 面向北偏東 0.1、目的地在正西 → 左轉
            (
                from_north(0.1),
                west,
                -FRAC_PI_2 - 0.1,
                GpsTurnDirection::Left,
            ),
            // 面向南偏西 0.1（跨過 ±π 那一側）、目的地在正東 → 面向南時東在左邊
            (
                from_north(PI + 0.1),
                east,
                -FRAC_PI_2 - 0.1,
                GpsTurnDirection::Left,
            ),
            // 面向南偏東 0.1、目的地在正西 → 面向南時西在右邊
            (
                from_north(PI - 0.1),
                west,
                FRAC_PI_2 + 0.1,
                GpsTurnDirection::Right,
            ),
        ];
        for (facing, to_dest, expected, turn) in cases {
            let angle = calculate_gps_direction_angle(facing, to_dest);
            assert!(
                (angle - expected).abs() < 1e-4,
                "facing={facing} to_dest={to_dest} angle={angle}"
            );
            assert_eq!(GpsTurnDirection::from_angle(angle), turn);
        }
    }

    #[test]
    fn gps_arrow_and_turn_follow_player_facing() {
        // 往北走時玩家的旋轉是 yaw 180°（角色模型正面是本地 +Z，見 update_character_rotation）
        let facing_north = Quat::from_rotation_y(PI);
        let (visibility, angle, turn) = gps_after_update(facing_north, Vec3::new(0.0, 0.0, -100.0));
        assert_eq!(
            (visibility, turn),
            (Visibility::Visible, GpsTurnDirection::Straight)
        );
        assert!(angle.abs() < 1e-3, "目的地在正前方 angle={angle}");
        let (_, angle, turn) = gps_after_update(facing_north, Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(turn, GpsTurnDirection::Right);
        assert!(
            (angle - FRAC_PI_2).abs() < 1e-3,
            "目的地在右邊 angle={angle}"
        );
    }
}
