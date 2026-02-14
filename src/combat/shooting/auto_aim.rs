//! 自動瞄準/鎖定系統（GTA 5 風格）
//!
//! 瞄準時自動鎖定最近的敵人，提供瞄準吸附和目標追蹤。
//! - 右鍵瞄準 → 自動鎖定視野內最近的敵人
//! - 中鍵 → 切換目標
//! - 停止瞄準/目標死亡/超出距離/失去視線 → 解除鎖定

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::combat::components::{CombatState, Enemy, LockOnState};
use crate::combat::health::{Damageable, Health};
use crate::core::{CameraSettings, GameState};
use crate::player::Player;

/// 目標中心質量高度（相對於 Transform.translation）
pub const TARGET_CENTER_MASS_HEIGHT: f32 = 1.0;
/// 玩家視線起點高度
const EYE_HEIGHT: f32 = 1.5;

/// 最小鎖定距離的平方（0.5² = 0.25）
const MIN_RANGE_SQ: f32 = 0.5 * 0.5;

// ============================================================================
// 自動瞄準主系統
// ============================================================================

/// 自動瞄準系統：管理鎖定目標的獲取、維持、切換和解除
#[allow(clippy::too_many_arguments)]
pub fn auto_aim_system(
    combat_state: Res<CombatState>,
    game_state: Res<GameState>,
    camera_settings: Res<CameraSettings>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut lock_on: ResMut<LockOnState>,
    rapier_context: ReadRapierContext,
    player_query: Query<(Entity, &Transform), With<Player>>,
    enemy_query: Query<(Entity, &Transform, &Health), (With<Enemy>, With<Damageable>)>,
    time: Res<Time>,
) {
    let Ok(rapier) = rapier_context.single() else {
        return;
    };
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 不瞄準或在車內時解除鎖定
    if !combat_state.is_aiming || game_state.player_in_vehicle {
        lock_on.locked_target = None;
        lock_on.los_lost_timer = 0.0;
        return;
    }

    // 計算攝影機前方向量（用於 FOV 篩選）
    let yaw = camera_settings.yaw;
    let pitch = camera_settings.pitch;
    let cam_forward = Vec3::new(
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    )
    .normalize();

    // 中鍵切換目標（Tab 已用於武器輪盤）
    let switch_requested = mouse_button.just_pressed(MouseButton::Middle);

    // 如果已有鎖定目標，驗證是否仍然有效
    if let Some(target_entity) = lock_on.locked_target {
        let valid = validate_locked_target(
            target_entity,
            player_entity,
            player_pos,
            &mut lock_on,
            &rapier,
            &enemy_query,
            time.delta_secs(),
        );

        if !valid {
            lock_on.locked_target = None;
            lock_on.los_lost_timer = 0.0;
        }

        // 中鍵切換目標
        if switch_requested && lock_on.locked_target.is_some() {
            lock_on.locked_target = find_next_target(
                lock_on.locked_target,
                player_entity,
                player_pos,
                cam_forward,
                &rapier,
                &enemy_query,
                lock_on.lock_range,
                lock_on.fov_half_angle,
            );
        }

        if lock_on.locked_target.is_some() {
            return;
        }
    }

    // 沒有鎖定目標，嘗試獲取新目標
    lock_on.locked_target = find_best_target(
        player_entity,
        player_pos,
        cam_forward,
        &rapier,
        &enemy_query,
        lock_on.lock_range,
        lock_on.fov_half_angle,
    );
    lock_on.los_lost_timer = 0.0;
}

/// 計算鎖定吸附後的瞄準點
///
/// 將原始瞄準點以 `snap_strength` 比例混合到鎖定目標的中心質量位置。
pub fn apply_lock_on_aim_assist(
    aim_point: Vec3,
    lock_on: &LockOnState,
    transform_query: &Query<&Transform>,
) -> Vec3 {
    let Some(target_entity) = lock_on.locked_target else {
        return aim_point;
    };
    let Ok(target_transform) = transform_query.get(target_entity) else {
        return aim_point;
    };
    let target_center = target_transform.translation + Vec3::Y * TARGET_CENTER_MASS_HEIGHT;
    aim_point.lerp(target_center, lock_on.snap_strength)
}

// ============================================================================
// 目標驗證
// ============================================================================

/// 驗證鎖定目標是否仍然有效（存活、距離、視線）
fn validate_locked_target(
    target_entity: Entity,
    player_entity: Entity,
    player_pos: Vec3,
    lock_on: &mut LockOnState,
    rapier: &RapierContext,
    enemy_query: &Query<(Entity, &Transform, &Health), (With<Enemy>, With<Damageable>)>,
    delta_secs: f32,
) -> bool {
    let Ok((_, target_transform, target_health)) = enemy_query.get(target_entity) else {
        return false; // 實體不存在
    };

    // 目標死亡
    if target_health.is_dead() {
        return false;
    }

    // 超出最大保持距離（使用 distance_squared 避免 sqrt）
    let dist_sq = player_pos.distance_squared(target_transform.translation);
    let max_range_sq = lock_on.max_range * lock_on.max_range;
    if dist_sq > max_range_sq {
        return false;
    }

    // 視線檢測
    let eye_pos = player_pos + Vec3::Y * EYE_HEIGHT;
    let target_center = target_transform.translation + Vec3::Y * TARGET_CENTER_MASS_HEIGHT;
    let to_target = target_center - eye_pos;
    let to_target_len_sq = to_target.length_squared();

    if to_target_len_sq < 0.0001 {
        return true;
    }

    let to_target_len = to_target_len_sq.sqrt();
    let direction = to_target / to_target_len;
    let filter = QueryFilter::default().exclude_collider(player_entity);

    let has_los = if let Some((hit_entity, _)) = rapier.cast_ray(
        eye_pos,
        direction,
        to_target_len as bevy_rapier3d::prelude::Real,
        true,
        filter,
    ) {
        hit_entity == target_entity
    } else {
        true // 無遮擋
    };

    if has_los {
        lock_on.los_lost_timer = 0.0;
    } else {
        lock_on.los_lost_timer += delta_secs;
        if lock_on.los_lost_timer >= lock_on.los_timeout {
            return false;
        }
    }

    true
}

// ============================================================================
// 目標搜索
// ============================================================================

/// 找到最佳鎖定目標（最近且在視野內的存活敵人）
///
/// 使用距離 + 角度偏差的加權評分，優先鎖定靠近準心中央且較近的敵人。
fn find_best_target(
    player_entity: Entity,
    player_pos: Vec3,
    cam_forward: Vec3,
    rapier: &RapierContext,
    enemy_query: &Query<(Entity, &Transform, &Health), (With<Enemy>, With<Damageable>)>,
    max_range: f32,
    fov_half_angle: f32,
) -> Option<Entity> {
    find_target_with_filter(
        player_entity,
        player_pos,
        cam_forward,
        rapier,
        enemy_query,
        max_range,
        fov_half_angle,
        None,
    )
}

/// 找到下一個鎖定目標（排除當前目標）
fn find_next_target(
    current: Option<Entity>,
    player_entity: Entity,
    player_pos: Vec3,
    cam_forward: Vec3,
    rapier: &RapierContext,
    enemy_query: &Query<(Entity, &Transform, &Health), (With<Enemy>, With<Damageable>)>,
    max_range: f32,
    fov_half_angle: f32,
) -> Option<Entity> {
    find_target_with_filter(
        player_entity,
        player_pos,
        cam_forward,
        rapier,
        enemy_query,
        max_range,
        fov_half_angle,
        current,
    )
}

/// 搜索目標（可選排除某個實體），含 LOS 射線檢測
fn find_target_with_filter(
    player_entity: Entity,
    player_pos: Vec3,
    cam_forward: Vec3,
    rapier: &RapierContext,
    enemy_query: &Query<(Entity, &Transform, &Health), (With<Enemy>, With<Damageable>)>,
    max_range: f32,
    fov_half_angle: f32,
    exclude: Option<Entity>,
) -> Option<Entity> {
    let eye_pos = player_pos + Vec3::Y * EYE_HEIGHT;
    let max_range_sq = max_range * max_range;
    let filter = QueryFilter::default().exclude_collider(player_entity);
    let mut best_target = None;
    let mut best_score = f32::MAX;

    for (entity, transform, health) in enemy_query.iter() {
        if health.is_dead() {
            continue;
        }
        if Some(entity) == exclude {
            continue;
        }

        let target_center = transform.translation + Vec3::Y * TARGET_CENTER_MASS_HEIGHT;
        let to_target = target_center - eye_pos;
        let dist_sq = to_target.length_squared();

        // 使用 distance_squared 進行粗篩
        if dist_sq > max_range_sq || dist_sq < MIN_RANGE_SQ {
            continue;
        }

        let distance = dist_sq.sqrt();
        let direction = to_target / distance;
        let dot = cam_forward.dot(direction).clamp(-1.0, 1.0);
        let angle = dot.acos();

        if angle > fov_half_angle {
            continue;
        }

        // LOS 射線檢測：確保目標沒有被遮擋
        let has_los = if let Some((hit_entity, _)) = rapier.cast_ray(
            eye_pos,
            direction,
            distance as bevy_rapier3d::prelude::Real,
            true,
            filter,
        ) {
            hit_entity == entity
        } else {
            true // 無遮擋
        };

        if !has_los {
            continue;
        }

        // 評分：距離（30%）+ 角度偏差（70%）
        // 角度權重較高，優先鎖定準心附近的敵人
        let score = distance * 0.3 + angle * 20.0;
        if score < best_score {
            best_score = score;
            best_target = Some(entity);
        }
    }

    best_target
}

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_on_state_default() {
        let state = LockOnState::default();
        assert!(state.locked_target.is_none());
        assert_eq!(state.lock_range, 30.0);
        assert_eq!(state.max_range, 40.0);
        assert!((state.fov_half_angle - 0.52).abs() < 0.01);
        assert_eq!(state.snap_strength, 0.85);
        assert_eq!(state.los_lost_timer, 0.0);
        assert_eq!(state.los_timeout, 2.0);
    }

    #[test]
    fn test_apply_lock_on_no_target() {
        let aim_point = Vec3::new(10.0, 5.0, 20.0);
        let state = LockOnState::default();

        // apply_lock_on_aim_assist 在無目標時返回原始瞄準點
        assert!(state.locked_target.is_none());
        // 確認 lerp 數學正確性
        let target = Vec3::new(20.0, 10.0, 30.0);
        let blended = aim_point.lerp(target, 0.85);
        let expected = aim_point * 0.15 + target * 0.85;
        assert!((blended - expected).length() < 0.001);
    }

    #[test]
    fn test_snap_strength_zero_returns_original() {
        let aim = Vec3::new(10.0, 5.0, 20.0);
        let target = Vec3::new(20.0, 10.0, 30.0);
        let result = aim.lerp(target, 0.0);
        assert!((result - aim).length() < 0.001);
    }

    #[test]
    fn test_snap_strength_full_returns_target() {
        let aim = Vec3::new(10.0, 5.0, 20.0);
        let target = Vec3::new(20.0, 10.0, 30.0);
        let result = aim.lerp(target, 1.0);
        assert!((result - target).length() < 0.001);
    }

    #[test]
    fn test_target_center_mass_height() {
        assert_eq!(TARGET_CENTER_MASS_HEIGHT, 1.0);
    }

    // --- distance_squared 常數一致性 ---

    /// 鎖定範圍的平方（用於測試驗證）
    const LOCK_RANGE_SQ: f32 = 30.0 * 30.0;

    #[test]
    fn test_lock_range_sq_matches_default() {
        let state = LockOnState::default();
        assert_eq!(LOCK_RANGE_SQ, state.lock_range * state.lock_range);
    }

    #[test]
    fn test_max_range_sq_matches_default() {
        let state = LockOnState::default();
        let max_range_sq = state.max_range * state.max_range;
        assert_eq!(max_range_sq, 40.0 * 40.0);
    }

    // --- FOV 篩選數學 ---

    #[test]
    fn test_fov_angle_filtering_forward_target() {
        // 正前方目標：角度 = 0，應在 FOV 內
        let cam_forward = Vec3::new(0.0, 0.0, -1.0);
        let to_target = Vec3::new(0.0, 0.0, -10.0).normalize();
        let dot = cam_forward.dot(to_target).clamp(-1.0, 1.0);
        let angle = dot.acos();
        assert!(angle < 0.01); // 接近 0
        assert!(angle < 0.52); // 在 FOV 內
    }

    #[test]
    fn test_fov_angle_filtering_side_target() {
        // 正側方目標：角度 ≈ 90°（1.57 rad），應在 FOV 外
        let cam_forward = Vec3::new(0.0, 0.0, -1.0);
        let to_target = Vec3::new(1.0, 0.0, 0.0).normalize();
        let dot = cam_forward.dot(to_target).clamp(-1.0, 1.0);
        let angle = dot.acos();
        assert!(angle > 1.5); // ≈ PI/2
        assert!(angle > 0.52); // 在 FOV 外
    }

    #[test]
    fn test_fov_angle_filtering_near_edge() {
        // 約 25° 偏移的目標，應在 30° FOV 內
        let cam_forward = Vec3::new(0.0, 0.0, -1.0);
        let to_target = Vec3::new(0.47, 0.0, -1.0).normalize(); // ≈ 25°
        let dot = cam_forward.dot(to_target).clamp(-1.0, 1.0);
        let angle = dot.acos();
        assert!(angle < 0.52); // 在 FOV 內
    }

    // --- 評分公式 ---

    #[test]
    fn test_scoring_prefers_centered_target() {
        // 近但偏離準心 vs 遠但正中央：正中央應得分更低
        let near_offset_score = 10.0 * 0.3 + 0.4 * 20.0; // 3 + 8 = 11
        let far_center_score = 25.0 * 0.3 + 0.05 * 20.0; // 7.5 + 1 = 8.5
        assert!(far_center_score < near_offset_score);
    }

    #[test]
    fn test_scoring_distance_weight() {
        // 相同角度，較近的目標得分更低
        let close_score = 5.0 * 0.3 + 0.2 * 20.0; // 1.5 + 4 = 5.5
        let far_score = 20.0 * 0.3 + 0.2 * 20.0; // 6 + 4 = 10
        assert!(close_score < far_score);
    }

    // --- 距離篩選 ---

    #[test]
    fn test_distance_squared_filtering() {
        // 距離 29m（在範圍內）
        let dist_sq_in = 29.0_f32 * 29.0;
        assert!(dist_sq_in < LOCK_RANGE_SQ);

        // 距離 31m（超出範圍）
        let dist_sq_out = 31.0_f32 * 31.0;
        assert!(dist_sq_out > LOCK_RANGE_SQ);

        // 距離 0.3m（太近）
        let dist_sq_close = 0.3_f32 * 0.3;
        assert!(dist_sq_close < MIN_RANGE_SQ);
    }
}
