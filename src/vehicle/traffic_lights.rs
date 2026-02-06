//! 紅綠燈交通系統

use super::*;
use bevy::prelude::*;

// ============================================================================
// 紅綠燈交通系統
// ============================================================================

/// 初始化紅綠燈視覺效果資源
pub fn setup_traffic_lights(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TrafficLightVisuals::new(&mut meshes, &mut materials));
    info!("🚦 紅綠燈系統已初始化");
}

/// 紅綠燈狀態更新系統
/// 處理紅綠燈的循環切換
pub fn traffic_light_cycle_system(
    time: Res<Time>,
    mut traffic_lights: Query<(Entity, &mut TrafficLight, &Children)>,
    mut bulb_query: Query<(&TrafficLightBulb, &mut MeshMaterial3d<StandardMaterial>)>,
    visuals: Option<Res<TrafficLightVisuals>>,
) {
    let Some(visuals) = visuals else { return };

    for (_entity, mut light, children) in traffic_lights.iter_mut() {
        // 更新計時器
        light.timer.tick(time.delta());

        // 計時器結束時切換狀態
        if light.timer.just_finished() {
            light.advance();

            // 更新燈泡材質
            for child in children.iter() {
                if let Ok((bulb, mut material)) = bulb_query.get_mut(child) {
                    *material =
                        MeshMaterial3d(visuals.get_bulb_material(bulb.light_type, light.state));
                }
            }
        }
    }
}

/// 生成紅綠燈實體
pub fn spawn_traffic_light(
    commands: &mut Commands,
    visuals: &TrafficLightVisuals,
    position: Vec3,
    direction: Vec3,
    is_primary: bool,
) {
    let rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
    let initial_state = if is_primary {
        TrafficLightState::Green
    } else {
        TrafficLightState::Red
    };

    commands
        .spawn((
            // 空間組件
            Transform {
                translation: position,
                rotation,
                ..default()
            },
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // 紅綠燈組件
            TrafficLight::new(direction, is_primary),
            Name::new("TrafficLight"),
        ))
        .with_children(|parent| {
            // 燈柱
            parent.spawn((
                Mesh3d(visuals.pole_mesh.clone()),
                MeshMaterial3d(visuals.pole_material.clone()),
                Transform::from_xyz(0.0, 2.0, 0.0),
                GlobalTransform::default(),
            ));

            // 燈箱
            parent.spawn((
                Mesh3d(visuals.box_mesh.clone()),
                MeshMaterial3d(visuals.box_material.clone()),
                Transform::from_xyz(0.0, 4.5, 0.0),
                GlobalTransform::default(),
            ));

            // 紅燈（頂部）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Red, initial_state)),
                Transform::from_xyz(0.0, 4.9, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Red,
                },
            ));

            // 黃燈（中間）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Yellow, initial_state)),
                Transform::from_xyz(0.0, 4.5, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Yellow,
                },
            ));

            // 綠燈（底部）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Green, initial_state)),
                Transform::from_xyz(0.0, 4.1, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Green,
                },
            ));
        });
}

/// 生成交叉路口的紅綠燈組（4個方向）
/// ns_road_width: 南北向道路寬度（X方向）
/// ew_road_width: 東西向道路寬度（Z方向）
pub fn spawn_intersection_lights(
    commands: &mut Commands,
    visuals: &TrafficLightVisuals,
    center: Vec3,
    ns_road_width: f32,
    ew_road_width: f32,
) {
    // 紅綠燈放在道路邊緣外側 1 公尺
    let offset_x = ns_road_width / 2.0 + 1.0; // X 方向偏移（南北向道路寬度）
    let offset_z = ew_road_width / 2.0 + 1.0; // Z 方向偏移（東西向道路寬度）

    // 北向（控制南北向車流）- 主燈
    // 放在交叉口西北角（西側人行道，面向南來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(-offset_x, 0.0, -offset_z),
        Vec3::NEG_Z,
        true,
    );

    // 南向（控制南北向車流）- 主燈
    // 放在交叉口東南角（東側人行道，面向北來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(offset_x, 0.0, offset_z),
        Vec3::Z,
        true,
    );

    // 東向（控制東西向車流）- 副燈
    // 放在交叉口東北角（北側人行道，面向西來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(offset_x, 0.0, -offset_z),
        Vec3::X,
        false,
    );

    // 西向（控制東西向車流）- 副燈
    // 放在交叉口西南角（南側人行道，面向東來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(-offset_x, 0.0, offset_z),
        Vec3::NEG_X,
        false,
    );
}

/// 在世界中生成紅綠燈（西門町主要路口）
/// 此系統需要在 setup_traffic_lights 之後執行
pub fn spawn_world_traffic_lights(
    mut commands: Commands,
    visuals: Option<Res<TrafficLightVisuals>>,
) {
    let Some(visuals) = visuals else {
        warn!("TrafficLightVisuals 資源不存在，無法生成紅綠燈");
        return;
    };

    info!("🚦 正在生成交通燈...");

    // 道路常數（與 setup.rs 一致）
    // 南北向道路 X 位置
    const X_ZHONGHUA: f32 = 80.0; // 中華路
    const X_XINING: f32 = -55.0; // 西寧南路
                                 // 東西向道路 Z 位置
    const Z_HANKOU: f32 = -80.0; // 漢口街
    const Z_CHENGDU: f32 = 50.0; // 成都路
                                 // 道路寬度
    const W_ZHONGHUA: f32 = 40.0; // 中華路寬度
    const W_MAIN: f32 = 16.0; // 成都路寬度
    const W_SECONDARY: f32 = 12.0; // 西寧路、漢口街寬度

    // 主要路口：(位置, 南北道路寬度, 東西道路寬度)
    let intersections: [(Vec3, f32, f32); 4] = [
        // 西寧路/成都路交叉口
        (Vec3::new(X_XINING, 0.0, Z_CHENGDU), W_SECONDARY, W_MAIN),
        // 中華路/成都路交叉口
        (Vec3::new(X_ZHONGHUA, 0.0, Z_CHENGDU), W_ZHONGHUA, W_MAIN),
        // 西寧路/漢口街交叉口
        (Vec3::new(X_XINING, 0.0, Z_HANKOU), W_SECONDARY, W_SECONDARY),
        // 中華路/漢口街交叉口
        (
            Vec3::new(X_ZHONGHUA, 0.0, Z_HANKOU),
            W_ZHONGHUA,
            W_SECONDARY,
        ),
    ];

    for (center, ns_width, ew_width) in intersections.iter() {
        spawn_intersection_lights(&mut commands, &visuals, *center, *ns_width, *ew_width);
    }

    info!(
        "✅ 已生成 {} 組交通燈（共 {} 個）",
        intersections.len(),
        intersections.len() * 4
    );
}
