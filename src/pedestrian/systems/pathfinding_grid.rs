//! A* 尋路網格建構與路徑跟隨

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::pedestrian::behavior::{DailyBehavior, PointsOfInterest};
use crate::pedestrian::components::{PedState, Pedestrian, PedestrianConfig, PedestrianState};
use crate::pedestrian::pathfinding::{AStarPath, PathfindingGrid};

// ============================================================================
// A* 尋路系統
// ============================================================================

// ============================================================================
// 尋路網格輔助函數
// ============================================================================
/// 將世界座標轉換為網格座標
fn world_to_grid_coords(x: i32, z: i32) -> (usize, usize) {
    (((x + 70) / 2) as usize, ((z + 70) / 2) as usize)
}

/// 將矩形區域標記為不可通行
fn mark_area_unwalkable(grid: &mut PathfindingGrid, x1: i32, z1: i32, x2: i32, z2: i32) {
    for x in x1..x2 {
        for z in z1..z2 {
            let (gx, gz) = world_to_grid_coords(x, z);
            if gx < grid.width && gz < grid.height {
                grid.set_walkable(gx, gz, false);
            }
        }
    }
}

/// 將橫向道路標記為可通行
fn mark_horizontal_road(grid: &mut PathfindingGrid, grid_x_start: usize, grid_x_end: usize) {
    for x in grid_x_start..grid_x_end {
        for z in 0..grid.height {
            grid.set_walkable(x, z, true);
        }
    }
}

/// 將縱向道路標記為可通行
fn mark_vertical_road(grid: &mut PathfindingGrid, grid_z_start: usize, grid_z_end: usize) {
    for x in 0..grid.width {
        for z in grid_z_start..grid_z_end {
            grid.set_walkable(x, z, true);
        }
    }
}

/// 初始化 A* 尋路網格
pub fn setup_pathfinding_grid(mut commands: Commands) {
    let mut grid = PathfindingGrid::default();

    // 建築物區域座標 (世界座標)
    let buildings: &[(i32, i32, i32, i32)] = &[
        // 漢中街兩側建築（中央徒步區）
        (-15, -60, 15, 55),
        // 西側建築
        (-70, -70, -20, -50),
        (-70, -45, -20, -25),
        (-70, -20, -20, 0),
        (-70, 5, -20, 25),
        (-70, 30, -20, 60),
        // 東側建築
        (20, -70, 50, -50),
        (20, -45, 50, -25),
        (20, -20, 50, 0),
        (20, 5, 50, 25),
        (20, 30, 50, 60),
    ];

    // 將建築區域設為不可通行
    for &(x1, z1, x2, z2) in buildings {
        mark_area_unwalkable(&mut grid, x1, z1, x2, z2);
    }

    // 漢中街徒步區 (X: -15 ~ 15) → 網格 27..43
    mark_horizontal_road(&mut grid, 27, 43);

    // 峨嵋街 (Z: -15 ~ 15) → 網格 27..43
    mark_vertical_road(&mut grid, 27, 43);

    commands.insert_resource(grid);
    commands.insert_resource(PointsOfInterest::setup_ximending());
}

/// A* 路徑計算系統
pub fn astar_path_calculation_system(
    time: Res<Time>,
    grid: Res<PathfindingGrid>,
    mut ped_query: Query<(&Transform, &mut AStarPath), With<Pedestrian>>,
) {
    let dt = time.delta_secs();

    for (transform, mut path) in ped_query.iter_mut() {
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
) {
    let dt = time.delta_secs();

    for (state, behavior, mut transform, mut path, mut controller) in ped_query.iter_mut() {
        // 逃跑時不使用 A* 路徑
        if state.state == PedState::Fleeing {
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

        if flat_dist < 1.5
            && !path.advance() {
                // 到達終點，標記需要新路徑
                path.needs_recalc = true;
            }
    }
}
