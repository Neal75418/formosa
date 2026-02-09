//! A* 尋路網格建構與路徑跟隨

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::pedestrian::behavior::{DailyBehavior, PointsOfInterest};
use crate::pedestrian::components::{PedState, Pedestrian, PedestrianConfig, PedestrianState};
use crate::pedestrian::pathfinding::{AStarPath, PathfindingGrid};
use crate::world::{
    X_HAN, X_KANGDING, X_XINING, X_ZHONGHUA,
    Z_CHENGDU, Z_EMEI, Z_HANKOU, Z_KUNMING, Z_WUCHANG,
    W_ALLEY, W_MAIN, W_PEDESTRIAN, W_SECONDARY, W_ZHONGHUA,
};

// ============================================================================
// 尋路網格輔助函數
// ============================================================================

/// 將矩形世界座標區域標記為可通行
fn mark_rect_walkable(grid: &mut PathfindingGrid, x_min: f32, x_max: f32, z_min: f32, z_max: f32) {
    let gx_start = ((x_min - grid.origin.x) / grid.cell_size).floor().max(0.0) as usize;
    let gx_end = ((x_max - grid.origin.x) / grid.cell_size).ceil().min(grid.width as f32) as usize;
    let gz_start = ((z_min - grid.origin.z) / grid.cell_size).floor().max(0.0) as usize;
    let gz_end = ((z_max - grid.origin.z) / grid.cell_size).ceil().min(grid.height as f32) as usize;

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

/// 初始化 A* 尋路網格 — 使用 world::constants 道路常數動態生成
pub fn setup_pathfinding_grid(mut commands: Commands) {
    // 網格覆蓋完整西門町地圖區域：
    // X: -110 to +102 (康定路西側 → 中華路東側)
    // Z:  -90 to +60  (漢口街北側 → 成都路南側)
    let cell_size = 2.0_f32;
    let origin = Vec3::new(-110.0, 0.0, -90.0);
    let width = 106; // 212m / 2m
    let height = 75; // 150m / 2m

    let mut grid = PathfindingGrid {
        origin,
        width,
        height,
        cell_size,
        walkable: vec![false; width * height], // 預設全部不可通行
    };

    // --- 南北向道路（固定 X，沿 Z 軸延伸）---
    mark_ns_road(&mut grid, X_ZHONGHUA, W_ZHONGHUA);  // 中華路
    mark_ns_road(&mut grid, X_HAN, W_PEDESTRIAN);     // 漢中街（徒步區）
    mark_ns_road(&mut grid, X_XINING, W_SECONDARY);   // 西寧南路
    mark_ns_road(&mut grid, X_KANGDING, W_MAIN);      // 康定路

    // --- 東西向道路（固定 Z，沿 X 軸延伸）---
    mark_ew_road(&mut grid, Z_HANKOU, W_SECONDARY);   // 漢口街
    mark_ew_road(&mut grid, Z_WUCHANG, W_PEDESTRIAN); // 武昌街
    mark_ew_road(&mut grid, Z_KUNMING, W_ALLEY);      // 昆明街
    mark_ew_road(&mut grid, Z_EMEI, W_PEDESTRIAN);    // 峨嵋街
    mark_ew_road(&mut grid, Z_CHENGDU, W_MAIN);       // 成都路

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
        // 逃跑時不使用 A* 路徑，改用逃離方向
        // （恐慌逃跑由 panic_flee_direction_system 處理，此處處理非恐慌逃跑如目擊犯罪）
        if state.state == PedState::Fleeing {
            if let Some(threat_pos) = state.last_threat_pos {
                let current_pos = transform.translation;
                let away_dir = (current_pos - threat_pos).normalize_or_zero();
                let flee_target = current_pos + away_dir * 20.0;
                // 將逃跑目標限制在市區範圍內
                let clamped = Vec3::new(
                    flee_target.x.clamp(-95.0, 75.0),
                    flee_target.y,
                    flee_target.z.clamp(-75.0, 45.0),
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

        if flat_dist < 1.5
            && !path.advance() {
                // 到達終點，標記需要新路徑
                path.needs_recalc = true;
            }
    }
}
