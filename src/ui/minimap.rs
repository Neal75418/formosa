//! 小地圖與大地圖系統
//!
//! 包含：小地圖更新、大地圖切換、縮放控制、世界名稱標籤

use bevy::prelude::*;

use super::components::{
    FullMapContainer, FullMapPlayerMarker, MinimapContainer, MinimapPlayerMarker, UiState,
};
use super::constants::ESLITE_GREEN;
use crate::camera::GameCamera;
use crate::mission::MissionMarker;
use crate::player::Player;
use crate::vehicle::{Vehicle, VehicleType};
use crate::world::Building;

// ============================================================================
// 世界名稱標籤組件
// ============================================================================
/// 3D 世界名稱標籤組件
#[derive(Component)]
pub struct WorldNameTag {
    pub target_entity: Entity,
    pub offset: Vec3,
}

// ============================================================================
// 地圖切換與更新系統
// ============================================================================
/// 切換大地圖顯示 (M 鍵)
pub fn toggle_map(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut full_map_query: Query<&mut Visibility, With<FullMapContainer>>,
) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        ui_state.show_full_map = !ui_state.show_full_map;

        if let Ok(mut visibility) = full_map_query.single_mut() {
            *visibility = if ui_state.show_full_map {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// 更新小地圖（同步玩家真實位置和方向）
pub fn update_minimap(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<MinimapPlayerMarker>)>,
    mut player_marker_query: Query<
        (&mut Node, &mut Transform),
        (With<MinimapPlayerMarker>, Without<Player>),
    >,
) {
    // 獲取玩家在 3D 世界的位置和方向
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let pos = player_transform.translation;
    let forward = player_transform.forward();

    // 將 3D 世界座標轉換為小地圖 UI 座標
    // 小地圖尺寸: 300x300
    let map_scale = 0.9;
    let offset_x = 150.0;
    let offset_y = 150.0;

    let minimap_x = (pos.x * map_scale + offset_x).clamp(10.0, 290.0);
    // Z 軸翻轉：讓北方（正 Z）在上方
    let minimap_y = (-pos.z * map_scale + offset_y).clamp(10.0, 290.0);

    // 計算旋轉角度（基於玩家面向方向）
    // ▲ 預設朝上（北），需要根據玩家朝向旋轉
    // forward.x = 東西方向, forward.z = 南北方向
    // 地圖上北方在上，所以 forward.z > 0 時箭頭朝上
    let rotation_angle = forward.x.atan2(forward.z);
    let target_rotation = Quat::from_rotation_z(-rotation_angle);

    // 更新玩家標記位置和旋轉
    // 容器: 20x34, 圓心在 (10, 24)（從容器左上角算）
    if let Ok((mut node, mut transform)) = player_marker_query.single_mut() {
        node.left = Val::Px(minimap_x - 10.0); // 置中調整 (20/2)
        node.top = Val::Px(minimap_y - 24.0); // 圓心偏移 (19 + 10/2)
                                              // 平滑旋轉插值（每秒旋轉速度約 10 倍，讓旋轉看起來平滑）
        let rotation_speed = 10.0;
        let t = (rotation_speed * time.delta_secs()).min(1.0);
        transform.rotation = transform.rotation.slerp(target_rotation, t);
    }
}

/// 更新大地圖玩家標記位置和方向
pub fn update_fullmap(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<FullMapPlayerMarker>)>,
    mut fullmap_marker_query: Query<
        (&mut Node, &mut Transform),
        (With<FullMapPlayerMarker>, Without<Player>),
    >,
) {
    // 獲取玩家在 3D 世界的位置和方向
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let pos = player_transform.translation;
    let forward = player_transform.forward();

    // 將 3D 世界座標轉換為大地圖 UI 座標
    // Full Map: 1200x800
    let fm_scale = 2.0;
    let fm_off_x = 600.0;
    let fm_off_y = 400.0;

    // Z 軸翻轉：讓北方在上方
    let map_x = (pos.x * fm_scale + fm_off_x).clamp(20.0, 1180.0);
    let map_y = (-pos.z * fm_scale + fm_off_y).clamp(20.0, 780.0);

    // 計算旋轉角度
    let rotation_angle = forward.x.atan2(forward.z);
    let target_rotation = Quat::from_rotation_z(-rotation_angle);

    // 更新玩家標記位置和旋轉
    // 容器: 30x52, 圓心在 (15, 37)（從容器左上角算）
    if let Ok((mut node, mut transform)) = fullmap_marker_query.single_mut() {
        node.left = Val::Px(map_x - 15.0); // 置中調整 (30/2)
        node.top = Val::Px(map_y - 37.0); // 圓心偏移 (29 + 16/2)
                                          // 平滑旋轉插值
        let rotation_speed = 10.0;
        let t = (rotation_speed * time.delta_secs()).min(1.0);
        transform.rotation = transform.rotation.slerp(target_rotation, t);
    }
}

/// 小地圖縮放控制（+/- 鍵）
pub fn minimap_zoom_control(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut minimap_query: Query<&mut Node, With<MinimapContainer>>,
) {
    let mut changed = false;

    // + 鍵放大
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        ui_state.minimap_zoom = (ui_state.minimap_zoom + 0.25).min(2.0);
        changed = true;
    }
    // - 鍵縮小
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        ui_state.minimap_zoom = (ui_state.minimap_zoom - 0.25).max(0.5);
        changed = true;
    }

    // 更新小地圖大小 (基準為 setup_ui 中設定的 300x300)
    if changed {
        if let Ok(mut node) = minimap_query.single_mut() {
            let base_size = 300.0;
            let new_size = base_size * ui_state.minimap_zoom;
            node.width = Val::Px(new_size);
            node.height = Val::Px(new_size);
        }
    }
}

// ============================================================================
// 世界名稱標籤系統
// ============================================================================
/// 為所有有名字的建築生成世界標籤 UI
pub fn setup_world_name_tags(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    buildings: Query<(Entity, &GlobalTransform, &Building), Added<Building>>,
    vehicles: Query<(Entity, &GlobalTransform, &Vehicle), Added<Vehicle>>,
    missions: Query<(Entity, &GlobalTransform, &MissionMarker), Added<MissionMarker>>,
) {
    let font = asset_server.load("fonts/STHeiti.ttc");

    // 建築物標籤 (白色)
    for (entity, _transform, building) in &buildings {
        if building.name.is_empty() {
            continue;
        }

        commands.spawn((
            Text::new(&building.name),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            TextLayout::default(),
            WorldNameTag {
                target_entity: entity,
                offset: Vec3::new(0.0, 10.0, 0.0),
            },
        ));
    }

    // 載具標籤 (黃色)
    for (entity, _transform, vehicle) in &vehicles {
        let name = match vehicle.vehicle_type {
            VehicleType::Scooter => "[機車]",
            VehicleType::Car => "[汽車]",
            VehicleType::Taxi => "[計程車]",
            VehicleType::Bus => "[公車]",
        };

        commands.spawn((
            Text::new(name),
            TextFont {
                font: font.clone(),
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.3)), // 黃色
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            TextLayout::default(),
            WorldNameTag {
                target_entity: entity,
                offset: Vec3::new(0.0, 3.0, 0.0), // 載具較矮，偏移量小
            },
        ));
    }

    // 任務標記標籤 (綠色)
    for (entity, _transform, _marker) in &missions {
        commands.spawn((
            Text::new("[!] 任務"),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.3, 1.0, 0.4)), // 綠色
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            TextLayout::default(),
            WorldNameTag {
                target_entity: entity,
                offset: Vec3::new(0.0, 4.0, 0.0),
            },
        ));
    }
}

/// 更新世界標籤位置 (World to Screen)
pub fn update_world_name_tags(
    camera_query: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    mut tag_query: Query<(&mut Node, &mut Visibility, &WorldNameTag)>,
    target_query: Query<&GlobalTransform>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    for (mut node, mut visibility, tag) in &mut tag_query {
        if let Ok(target_transform) = target_query.get(tag.target_entity) {
            let world_position = target_transform.translation() + tag.offset;

            // World to Screen position
            if let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_position) {
                // 檢查是否在相機前方
                let forward = camera_transform.forward();
                let direction = (world_position - camera_transform.translation()).normalize();

                if forward.dot(direction) > 0.0 {
                    *visibility = Visibility::Visible;
                    node.left = Val::Px(screen_pos.x);
                    node.top = Val::Px(screen_pos.y);
                } else {
                    *visibility = Visibility::Hidden;
                }
            } else {
                *visibility = Visibility::Hidden;
            }
        } else {
            // 目標實體不存在，隱藏標籤（稍後由清理系統移除）
            *visibility = Visibility::Hidden;
        }
    }
}

/// 清理孤立的世界標籤（目標實體已被銷毀）
pub fn cleanup_orphaned_world_tags(
    mut commands: Commands,
    tag_query: Query<(Entity, &WorldNameTag)>,
    target_query: Query<Entity>,
) {
    for (tag_entity, tag) in &tag_query {
        // 如果目標實體不存在，清理標籤
        if target_query.get(tag.target_entity).is_err() {
            commands.entity(tag_entity).despawn();
        }
    }
}

// ============================================================================
// 地圖生成通用邏輯
// ============================================================================
/// 地標數據結構
struct MapLandmark {
    name: &'static str,
    world_x: f32, // World X center
    world_z: f32, // World Z center
    w: f32,       // World Width
    d: f32,       // World Depth
    color: Color,
}

/// 地圖繪製上下文：整合縮放、偏移、字型
struct MapDrawCtx {
    scale: f32,
    off_x: f32,
    off_y: f32,
    font: Handle<Font>,
}

/// 統一生成地圖內容（道路 + 地標）
#[allow(clippy::too_many_lines)]
pub fn spawn_map_layer(
    parent: &mut ChildSpawnerCommands,
    scale: f32,
    off_x: f32,
    off_y: f32,
    road_width_factor: f32, // 道路寬度縮放係數
    is_fullmap: bool,       // true: 大地圖(顯示路名、完整方塊), false: 小地圖(簡化)
    font: Handle<Font>,
) {
    // 引用世界常數 (更新為新的道路佈局)
    use crate::world::{
        W_ALLEY, W_MAIN, W_PEDESTRIAN, W_SECONDARY, W_ZHONGHUA, X_HAN, X_KANGDING, X_XINING,
        X_ZHONGHUA, Z_CHENGDU, Z_EMEI, Z_HANKOU, Z_KUNMING, Z_WUCHANG,
    };

    let ctx = MapDrawCtx {
        scale,
        off_x,
        off_y,
        font,
    };

    // 1. 繪製道路 (Roads) - 完整西門町道路網格
    let v_len_main = 180.0;
    let h_center_x = -10.0; // 水平道路中心點

    // 南北向道路 (Vertical)
    draw_road_rect(
        parent,
        X_ZHONGHUA,
        -15.0,
        W_ZHONGHUA * road_width_factor,
        v_len_main,
        &ctx,
        if is_fullmap { "中華路" } else { "" },
    );
    draw_road_rect(
        parent,
        X_XINING,
        -15.0,
        W_SECONDARY * road_width_factor,
        v_len_main,
        &ctx,
        if is_fullmap { "西寧南路" } else { "" },
    );
    draw_road_rect(
        parent,
        X_KANGDING,
        -15.0,
        W_MAIN * road_width_factor,
        v_len_main,
        &ctx,
        if is_fullmap { "康定路" } else { "" },
    );
    draw_road_rect(
        parent,
        X_HAN,
        0.0,
        W_PEDESTRIAN * road_width_factor,
        100.0,
        &ctx,
        if is_fullmap { "漢中街" } else { "" },
    );

    // 東西向道路 (Horizontal)
    let h_len = 200.0;
    draw_road_rect(
        parent,
        h_center_x,
        Z_HANKOU,
        h_len,
        W_SECONDARY * road_width_factor,
        &ctx,
        if is_fullmap { "漢口街" } else { "" },
    );
    draw_road_rect(
        parent,
        h_center_x,
        Z_WUCHANG,
        h_len,
        W_PEDESTRIAN * road_width_factor,
        &ctx,
        if is_fullmap { "武昌街" } else { "" },
    );
    draw_road_rect(
        parent,
        h_center_x,
        Z_KUNMING,
        h_len,
        W_ALLEY * road_width_factor,
        &ctx,
        if is_fullmap { "昆明街" } else { "" },
    );
    draw_road_rect(
        parent,
        h_center_x,
        Z_EMEI,
        h_len,
        W_PEDESTRIAN * road_width_factor,
        &ctx,
        if is_fullmap { "峨嵋街" } else { "" },
    );
    draw_road_rect(
        parent,
        h_center_x,
        Z_CHENGDU,
        h_len,
        W_MAIN * road_width_factor,
        &ctx,
        if is_fullmap { "成都路" } else { "" },
    );

    // 2. 繪製地標 (Landmarks) - 根據新的建築位置更新
    let landmarks = [
        // 西寧南路沿線
        MapLandmark {
            name: "萬年",
            world_x: X_XINING - 16.0,
            world_z: Z_EMEI - 17.5,
            w: 20.0,
            d: 15.0,
            color: Color::srgb(0.5, 0.5, 0.7),
        },
        MapLandmark {
            name: "獅子林",
            world_x: X_XINING - 17.0,
            world_z: Z_WUCHANG - 18.5,
            w: 22.0,
            d: 22.0,
            color: Color::srgb(0.5, 0.4, 0.3),
        },
        MapLandmark {
            name: "Donki",
            world_x: X_XINING + 20.0,
            world_z: Z_WUCHANG + 18.5,
            w: 28.0,
            d: 22.0,
            color: Color::srgb(1.0, 0.85, 0.0),
        },
        MapLandmark {
            name: "電影公園",
            world_x: X_XINING - 18.5,
            world_z: Z_KUNMING - 14.0,
            w: 25.0,
            d: 20.0,
            color: Color::srgb(0.25, 0.4, 0.25),
        },
        // 漢中街沿線
        MapLandmark {
            name: "誠品西門",
            world_x: X_HAN - 16.5,
            world_z: Z_EMEI - 15.5,
            w: 18.0,
            d: 16.0,
            color: ESLITE_GREEN,
        },
        MapLandmark {
            name: "誠品武昌",
            world_x: X_HAN - 14.5,
            world_z: Z_WUCHANG + 14.5,
            w: 14.0,
            d: 14.0,
            color: ESLITE_GREEN,
        },
        MapLandmark {
            name: "UQ",
            world_x: X_HAN + 13.5,
            world_z: Z_EMEI - 15.0,
            w: 12.0,
            d: 12.0,
            color: Color::srgb(0.85, 0.15, 0.15),
        },
        MapLandmark {
            name: "H&M",
            world_x: X_HAN + 14.5,
            world_z: Z_CHENGDU - 15.0,
            w: 14.0,
            d: 14.0,
            color: Color::srgb(0.85, 0.85, 0.85),
        },
        // 中華路沿線
        MapLandmark {
            name: "捷運6號",
            world_x: X_ZHONGHUA - 26.0,
            world_z: Z_CHENGDU - 14.0,
            w: 12.0,
            d: 12.0,
            color: Color::srgb(0.2, 0.35, 0.65),
        },
        MapLandmark {
            name: "紅樓",
            world_x: X_ZHONGHUA - 31.0,
            world_z: Z_CHENGDU + 19.0,
            w: 22.0,
            d: 22.0,
            color: Color::srgb(0.7, 0.22, 0.18),
        },
        MapLandmark {
            name: "錢櫃",
            world_x: X_ZHONGHUA + 28.0,
            world_z: Z_CHENGDU - 16.0,
            w: 16.0,
            d: 16.0,
            color: Color::srgb(0.75, 0.45, 0.55),
        },
        MapLandmark {
            name: "鴨肉扁",
            world_x: X_ZHONGHUA - 25.0,
            world_z: Z_WUCHANG + 12.5,
            w: 10.0,
            d: 10.0,
            color: Color::srgb(0.85, 0.65, 0.35),
        },
        // 康定路沿線
        MapLandmark {
            name: "西門國小",
            world_x: X_KANGDING + 23.0,
            world_z: Z_WUCHANG - 20.0,
            w: 30.0,
            d: 25.0,
            color: Color::srgb(0.7, 0.65, 0.55),
        },
    ];

    for lm in &landmarks {
        if is_fullmap {
            // 大地圖：顯示完整矩形
            // Size mapping based on scale 1.2 adjusted:
            // 為了保持與之前手動調整的一致性 (UI Size ~= World Size * 1.2)
            // 這裡我們直接使用 (World Size * Scale)
            draw_building_rect(
                parent, lm.world_x, lm.world_z, lm.w, lm.d, &ctx, lm.color, lm.name,
            );
        } else {
            // 小地圖：顯示簡化點
            draw_minimap_point(parent, lm.world_x, lm.world_z, &ctx, lm.color, lm.name);
        }
    }
}

// --- 底層繪圖 Helpers ---

fn draw_road_rect(
    parent: &mut ChildSpawnerCommands,
    x: f32,
    z: f32,
    width: f32,
    length: f32, // w=thickness, l=length
    ctx: &MapDrawCtx,
    label: &str,
) {
    // width = 世界 X 軸尺寸，length = 世界 Z 軸尺寸
    // 直接乘以縮放比例轉換為 UI 座標
    let ui_w = width * ctx.scale;
    let ui_h = length * ctx.scale;
    let ui_x = x * ctx.scale + ctx.off_x;
    let ui_y = -z * ctx.scale + ctx.off_y; // Z 軸翻轉

    spawn_centered_rect(
        parent,
        ui_x,
        ui_y,
        ui_w,
        ui_h,
        Color::srgba(0.5, 0.5, 0.55, 0.6),
        label,
        14.0,
        ctx.font.clone(),
    );
}

fn draw_building_rect(
    parent: &mut ChildSpawnerCommands,
    x: f32,
    z: f32,
    w: f32,
    d: f32, // World dims
    ctx: &MapDrawCtx,
    color: Color,
    name: &str,
) {
    let ui_w = w * ctx.scale;
    let ui_h = d * ctx.scale;
    let ui_x = x * ctx.scale + ctx.off_x;
    let ui_y = -z * ctx.scale + ctx.off_y; // Z 軸翻轉

    spawn_centered_rect(
        parent,
        ui_x,
        ui_y,
        ui_w,
        ui_h,
        color,
        name,
        10.0,
        ctx.font.clone(),
    );
}

/// 生成置中矩形 (共用 helper)
#[allow(clippy::too_many_arguments)]
fn spawn_centered_rect(
    parent: &mut ChildSpawnerCommands,
    ui_x: f32,
    ui_y: f32,
    ui_w: f32,
    ui_h: f32,
    color: Color,
    name: &str,
    font_size: f32,
    font: Handle<Font>,
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(ui_x - ui_w / 2.0),
                top: Val::Px(ui_y - ui_h / 2.0),
                width: Val::Px(ui_w),
                height: Val::Px(ui_h),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(color),
        ))
        .with_children(|bg| {
            if !name.is_empty() {
                bg.spawn((
                    Text::new(name),
                    TextFont {
                        font_size,
                        font,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            }
        });
}

fn draw_minimap_point(
    parent: &mut ChildSpawnerCommands,
    x: f32,
    z: f32,
    ctx: &MapDrawCtx,
    color: Color,
    name: &str,
) {
    let ui_x = x * ctx.scale + ctx.off_x;
    let ui_y = -z * ctx.scale + ctx.off_y; // Z 軸翻轉
    let size = 10.0; // Fixed point size for minimap

    // Point
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(ui_x - size / 2.0),
            top: Val::Px(ui_y - size / 2.0),
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        },
        BackgroundColor(color),
    ));

    // Label (Offset)
    parent.spawn((
        Text::new(name),
        TextFont {
            font_size: 8.0,
            font: ctx.font.clone(),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(ui_x + 6.0),
            top: Val::Px(ui_y - 4.0),
            ..default()
        },
    ));
}

pub(super) struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                toggle_map,
                update_minimap,
                minimap_zoom_control,
                update_fullmap,
                setup_world_name_tags,
                update_world_name_tags,
                cleanup_orphaned_world_tags,
            )
                .in_set(super::UiActive),
        );
    }
}
