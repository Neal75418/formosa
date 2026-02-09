//! 通緝系統除錯可視化

use bevy::prelude::*;
use super::components::*;
use crate::player::Player;

/// 可視化警察視線和 FOV
pub fn debug_police_vision(
    mut gizmos: Gizmos,
    police_query: Query<(&Transform, &PoliceOfficer)>,
    player_query: Query<&Transform, With<Player>>,
    config: Res<PoliceConfig>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (police_transform, officer) in &police_query {
        let police_pos = police_transform.translation;
        let player_pos = player_transform.translation;

        // 1. 畫警察到玩家的視線（紅色/綠色取決於能否看到）
        let color = if officer.can_see_player {
            Color::srgb(1.0, 0.0, 0.0) // 紅色 = 看到玩家
        } else {
            Color::srgb(0.3, 0.3, 0.3) // 灰色 = 看不到
        };
        gizmos.line(police_pos, player_pos, color);

        // 2. 畫 FOV 扇形視野錐
        let forward = police_transform.forward(); // 警察朝向
        let vision_range = config.vision_range;
        let half_fov = config.vision_fov / 2.0; // FOV 一半角度

        // 計算 FOV 扇形的左右邊界向量
        let left_boundary = Quat::from_rotation_y(half_fov) * forward.as_vec3();
        let right_boundary = Quat::from_rotation_y(-half_fov) * forward.as_vec3();

        // 畫扇形邊界線
        gizmos.line(
            police_pos,
            police_pos + left_boundary * vision_range,
            Color::srgba(0.0, 1.0, 0.0, 0.5),
        );
        gizmos.line(
            police_pos,
            police_pos + right_boundary * vision_range,
            Color::srgba(0.0, 1.0, 0.0, 0.5),
        );

        // 畫弧線（用多段線段近似）
        let segments = 16; // 弧線精度
        for i in 0..segments {
            let angle1 = -half_fov + (config.vision_fov / segments as f32) * i as f32;
            let angle2 = -half_fov + (config.vision_fov / segments as f32) * (i + 1) as f32;

            let dir1 = Quat::from_rotation_y(angle1) * forward.as_vec3();
            let dir2 = Quat::from_rotation_y(angle2) * forward.as_vec3();

            gizmos.line(
                police_pos + dir1 * vision_range,
                police_pos + dir2 * vision_range,
                Color::srgba(0.0, 1.0, 0.0, 0.3),
            );
        }
    }
}

/// 可視化 A* 路徑（行人/警察）
pub fn debug_astar_paths(
    mut gizmos: Gizmos,
    path_query: Query<(&Transform, &crate::pedestrian::AStarPath)>,
) {
    for (transform, path) in &path_query {
        if path.waypoints.is_empty() {
            continue;
        }

        let mut prev = transform.translation;
        for waypoint in &path.waypoints {
            // 畫藍色折線連接路徑點
            gizmos.line(prev, *waypoint, Color::srgb(0.0, 0.5, 1.0));
            // 在每個路徑點畫一個小球
            gizmos.sphere(
                Isometry3d::new(*waypoint, Quat::IDENTITY),
                0.2,
                Color::srgb(1.0, 1.0, 0.0),
            );
            prev = *waypoint;
        }
    }
}

/// 可視化恐慌傳播範圍
pub fn debug_panic_propagation(
    mut gizmos: Gizmos,
    panic_query: Query<&Transform, With<crate::pedestrian::PanicState>>,
) {
    for transform in &panic_query {
        // 黃色圓圈 = 恐慌傳播範圍
        gizmos.circle(
            Isometry3d::new(transform.translation, Quat::IDENTITY),
            10.0, // 恐慌傳播半徑
            Color::srgba(1.0, 1.0, 0.0, 0.5),
        );
    }
}

/// F3 切換除錯可視化
pub fn toggle_debug_visualization(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<super::DebugVisualizationState>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        state.enabled = !state.enabled;
        info!(
            "🎨 Debug 可視化: {}",
            if state.enabled { "開啟" } else { "關閉" }
        );
    }
}
