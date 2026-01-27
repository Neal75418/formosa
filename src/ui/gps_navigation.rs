//! GPS 導航系統 (GTA5 風格)
//!
//! 提供導航方向指示和距離顯示

use bevy::prelude::*;

use super::components::{
    ChineseFont, GpsDirectionArrow, GpsDistanceDisplay, GpsNavigationState, MinimapContainer,
    MinimapGpsMarker,
};
use crate::mission::{MissionManager, MissionType};
use crate::player::Player;

// === GPS 顏色常數 ===
#[allow(dead_code)]
const GPS_ROUTE_COLOR: Color = Color::srgba(0.4, 0.8, 1.0, 0.8); // 淡藍色路線
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

/// 計算玩家面向方向與目標方向的夾角
fn calculate_gps_direction_angle(player_forward: Vec3, to_dest: Vec3) -> f32 {
    let to_dest_normalized = Vec3::new(to_dest.x, 0.0, to_dest.z).normalize_or_zero();
    let player_forward_xz = Vec3::new(player_forward.x, 0.0, player_forward.z).normalize_or_zero();
    player_forward_xz.x.atan2(player_forward_xz.z)
        - to_dest_normalized.x.atan2(to_dest_normalized.z)
}

/// 格式化 GPS 距離顯示
fn format_gps_distance(distance_xz: f32) -> String {
    if distance_xz >= 1000.0 {
        format!("{:.1} km", distance_xz / 1000.0)
    } else {
        format!("{:.0} m", distance_xz)
    }
}

/// 更新 GPS 導航狀態
#[allow(clippy::type_complexity)]
pub fn update_gps_navigation(
    time: Res<Time>,
    mut gps: ResMut<GpsNavigationState>,
    player_query: Query<&Transform, With<Player>>,
    mut arrow_query: Query<
        (&mut Visibility, &mut Transform, &Children),
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
    let player_forward = player_transform.forward().as_vec3();

    // 更新冷卻計時器
    if gps.route_recalc_cooldown > 0.0 {
        gps.route_recalc_cooldown -= time.delta_secs();
    }

    // 如果導航未啟用，隱藏 UI
    let should_hide = gps.destination.is_none() || !gps.active;
    if should_hide {
        for (mut vis, _, _) in arrow_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        for (mut vis, _) in distance_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let destination = gps.destination.unwrap();

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
    let angle = calculate_gps_direction_angle(player_forward, to_dest);
    for (mut vis, mut transform, _children) in arrow_query.iter_mut() {
        *vis = Visibility::Visible;
        transform.rotation = Quat::from_rotation_z(angle);
    }

    // 更新距離顯示
    let distance_str = format_gps_distance(distance_xz);
    for (mut vis, children) in distance_query.iter_mut() {
        *vis = Visibility::Visible;
        for child in children.iter() {
            let Ok(mut text) = text_query.get_mut(child) else {
                continue;
            };
            **text = distance_str.clone();
        }
    }
}

/// 更新小地圖上的 GPS 目標標記
/// 優化：只在目標變化時重建標記，避免每幀 despawn/spawn 造成抖動
#[allow(clippy::type_complexity)]
pub fn update_minimap_gps_marker(
    mut commands: Commands,
    gps: Res<GpsNavigationState>,
    minimap_query: Query<Entity, With<MinimapContainer>>,
    mut marker_query: Query<(Entity, &mut Node, &mut Visibility), With<MinimapGpsMarker>>,
) {
    // 如果 GPS 未啟用或無目標，隱藏現有標記
    if !gps.active || gps.destination.is_none() {
        for (_, _, mut vis) in marker_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let destination = gps.destination.unwrap();

    // 將世界座標轉換為小地圖座標
    let map_scale = 0.9;
    let offset_x = 150.0;
    let offset_y = 150.0;

    let minimap_x = (destination.x * map_scale + offset_x).clamp(5.0, 295.0);
    let minimap_y = (-destination.z * map_scale + offset_y).clamp(5.0, 295.0);

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
            gps.set_destination(data.end_pos, "送貨目的地");
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
        MissionType::Explore => {
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
