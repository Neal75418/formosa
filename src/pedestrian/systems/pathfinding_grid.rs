//! A* 尋路網格建構與路徑跟隨

#![allow(
    clippy::needless_pass_by_value,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names
)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::pedestrian::behavior::{DailyBehavior, PointsOfInterest};
use crate::pedestrian::components::{PedState, Pedestrian, PedestrianConfig, PedestrianState};
use crate::pedestrian::pathfinding::{AStarPath, PathfindingGrid};
use crate::world::{MapLayout, RoadAxis};

// ============================================================================
// 尋路網格輔助函數
// ============================================================================

/// 將矩形世界座標區域標記為可通行
fn mark_rect_walkable(grid: &mut PathfindingGrid, x_min: f32, x_max: f32, z_min: f32, z_max: f32) {
    let gx_start = ((x_min - grid.origin.x) / grid.cell_size).floor().max(0.0) as usize;
    let gx_end = ((x_max - grid.origin.x) / grid.cell_size)
        .ceil()
        .min(grid.width as f32) as usize;
    let gz_start = ((z_min - grid.origin.z) / grid.cell_size).floor().max(0.0) as usize;
    let gz_end = ((z_max - grid.origin.z) / grid.cell_size)
        .ceil()
        .min(grid.height as f32) as usize;

    for gx in gx_start..gx_end {
        for gz in gz_start..gz_end {
            grid.set_walkable(gx, gz, true);
        }
    }
}

/// 標記南北向道路為可通行（固定 X 中心，沿 Z 軸延伸整個網格）
fn mark_ns_road(grid: &mut PathfindingGrid, center_x: f32, road_width: f32) {
    let x_min = center_x - road_width / 2.0;
    let x_max = center_x + road_width / 2.0;
    let z_min = grid.origin.z;
    let z_max = grid.origin.z + grid.height as f32 * grid.cell_size;
    mark_rect_walkable(grid, x_min, x_max, z_min, z_max);
}

/// 標記東西向道路為可通行（固定 Z 中心，沿 X 軸延伸整個網格）
fn mark_ew_road(grid: &mut PathfindingGrid, center_z: f32, road_width: f32) {
    let z_min = center_z - road_width / 2.0;
    let z_max = center_z + road_width / 2.0;
    let x_min = grid.origin.x;
    let x_max = grid.origin.x + grid.width as f32 * grid.cell_size;
    mark_rect_walkable(grid, x_min, x_max, z_min, z_max);
}

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

/// A* 路徑計算系統
pub fn astar_path_calculation_system(
    time: Res<Time>,
    grid: Res<PathfindingGrid>,
    mut ped_query: Query<(&Transform, &mut AStarPath), With<Pedestrian>>,
) {
    let dt = time.delta_secs();

    for (transform, mut path) in &mut ped_query {
        // 更新冷卻時間
        if path.recalc_cooldown > 0.0 {
            path.recalc_cooldown -= dt;
            continue;
        }

        // 檢查是否需要重新計算路徑
        if path.needs_recalc || path.waypoints.is_empty() {
            let start = transform.translation;
            let goal = path.goal;

            if let Some(new_path) = grid.find_path(start, goal) {
                path.waypoints = new_path;
                path.current_index = 0;
                path.needs_recalc = false;
                path.recalc_cooldown = 2.0; // 2 秒冷卻
            } else {
                // 找不到路徑時，隨機移動而非完全停止
                // 生成隨機方向（水平面上）
                let random_angle = (start.x * 12.9898 + start.z * 78.233).sin() * 43_758.547;
                let angle = random_angle.fract() * std::f32::consts::TAU;
                let random_dir = Vec3::new(angle.cos(), 0.0, angle.sin());
                let fallback_target = start + random_dir * 5.0;

                path.waypoints = vec![fallback_target];
                path.current_index = 0;
                path.needs_recalc = false;
                path.recalc_cooldown = 1.0; // 縮短冷卻時間，更快嘗試重新尋路
            }
        }
    }
}

/// A* 路徑跟隨移動系統
pub fn astar_movement_system(
    time: Res<Time>,
    config: Res<PedestrianConfig>,
    mut ped_query: Query<
        (
            &PedestrianState,
            &DailyBehavior,
            &mut Transform,
            &mut AStarPath,
            &mut KinematicCharacterController,
        ),
        With<Pedestrian>,
    >,
    layout: Res<MapLayout>,
) {
    let dt = time.delta_secs();
    let flee = layout.flee_area();

    for (state, behavior, mut transform, mut path, mut controller) in &mut ped_query {
        // 逃跑時不使用 A* 路徑，改用逃離方向
        // （恐慌逃跑由 panic_flee_direction_system 處理，此處處理非恐慌逃跑如目擊犯罪）
        if state.state == PedState::Fleeing {
            if let Some(threat_pos) = state.last_threat_pos {
                let current_pos = transform.translation;
                let away_dir = (current_pos - threat_pos).normalize_or_zero();
                let flee_target = current_pos + away_dir * 20.0;
                // 逃跑目標限制在越界線內縮 5 m 的範圍
                let clamped = Vec3::new(
                    flee_target.x.clamp(flee.min.x, flee.max.x),
                    flee_target.y,
                    flee_target.z.clamp(flee.min.y, flee.max.y),
                );
                let direction = (clamped - current_pos).normalize_or_zero();
                let flat_dir = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
                if flat_dir.length_squared() > 0.001 {
                    let target_rot = Quat::from_rotation_y((-flat_dir.x).atan2(-flat_dir.z));
                    transform.rotation = transform.rotation.slerp(target_rot, dt * 5.0);
                    let velocity = flat_dir * config.flee_speed;
                    controller.translation = Some(velocity * dt + Vec3::new(0.0, -9.8 * dt, 0.0));
                }
            }
            continue;
        }

        // 取得速度倍率
        let speed_mult = behavior.behavior.speed_multiplier();
        if speed_mult <= 0.0 {
            continue;
        }

        // 取得當前目標點
        let Some(target) = path.current_waypoint() else {
            continue;
        };

        // 計算移動方向
        let current_pos = transform.translation;
        let direction = (target - current_pos).normalize_or_zero();
        let flat_direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();

        if flat_direction.length_squared() < 0.001 {
            continue;
        }

        // 更新朝向
        let target_rotation = Quat::from_rotation_y((-flat_direction.x).atan2(-flat_direction.z));
        transform.rotation = transform.rotation.slerp(target_rotation, dt * 5.0);

        // 移動
        let speed = config.walk_speed * speed_mult;
        let velocity = flat_direction * speed;
        controller.translation = Some(velocity * dt + Vec3::new(0.0, -9.8 * dt, 0.0));

        // 檢查是否到達當前路徑點
        let flat_dist = Vec3::new(target.x - current_pos.x, 0.0, target.z - current_pos.z).length();

        if flat_dist < 1.5 && !path.advance() {
            // 到達終點，標記需要新路徑
            path.needs_recalc = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    #[test]
    fn pedestrian_outside_grid_falls_back() {
        // 越界線是地圖邊界（X 從 −119 起），比 A* 網格（X 從 −110 起）大，行人可能在網格外：找不到路時改走 5 m 外的備援點，不卡死
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        crate::world::install_map(&mut app);
        app.add_systems(Startup, setup_pathfinding_grid)
            .add_systems(Update, astar_path_calculation_system);
        let start = Vec3::new(-115.0, 0.0, 0.0);
        let ped = app
            .world_mut()
            .spawn((
                Pedestrian,
                Transform::from_translation(start),
                AStarPath::new(Vec3::ZERO),
            ))
            .id();
        app.update();
        let path = app.world().get::<AStarPath>(ped).unwrap();
        assert_eq!(path.waypoints.len(), 1, "{:?}", path.waypoints);
        assert!(
            (path.waypoints[0].distance(start) - 5.0).abs() < 1e-3,
            "{:?}",
            path.waypoints
        );
    }

    /// 逃跑中的行人（位置, 威脅位置）跑兩次 update（第一次的 dt 是 0），回傳控制器這一幀的水平位移
    fn flee_steps(peds: &[(Vec3, Vec3)]) -> Vec<Vec3> {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        crate::world::install_map(&mut app);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
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
    fn flee_direction_clamped_5m_inside_map_bounds() {
        // 每邊兩個往外逃的行人：界外 0.5 m 的目標被夾回、方向反轉；界內 0.5 m 的照樣往外
        let s = flee_steps(&[
            (Vec3::new(104.5, 0.0, 0.0), Vec3::new(94.5, 0.0, 0.0)),
            (Vec3::new(103.5, 0.0, 0.0), Vec3::new(93.5, 0.0, 0.0)),
            (Vec3::new(-114.5, 0.0, 0.0), Vec3::new(-104.5, 0.0, 0.0)),
            (Vec3::new(-113.5, 0.0, 0.0), Vec3::new(-103.5, 0.0, 0.0)),
            (Vec3::new(0.0, 0.0, 59.5), Vec3::new(0.0, 0.0, 49.5)),
            (Vec3::new(0.0, 0.0, 58.5), Vec3::new(0.0, 0.0, 48.5)),
            (Vec3::new(0.0, 0.0, -89.5), Vec3::new(0.0, 0.0, -79.5)),
            (Vec3::new(0.0, 0.0, -88.5), Vec3::new(0.0, 0.0, -78.5)),
        ]);
        assert!(s[0].x < 0.0 && s[1].x > 0.0, "東：{s:?}");
        assert!(s[2].x > 0.0 && s[3].x < 0.0, "西：{s:?}");
        assert!(s[4].z < 0.0 && s[5].z > 0.0, "南：{s:?}");
        assert!(s[6].z > 0.0 && s[7].z < 0.0, "北：{s:?}");
    }
}
