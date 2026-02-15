//! AI 移動系統（巡邏、追擊、走位）

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use super::{AiBehavior, AiMovement, AiState, PatrolPath};
use crate::combat::Enemy;

// ============================================================================
// 移動系統
// ============================================================================

/// 重力常數
const GRAVITY: f32 = -9.8;

// ============================================================================
// 移動系統輔助函數
// ============================================================================
/// 讓實體面向指定位置（只轉 Y 軸）
#[inline]
fn face_target(transform: &mut Transform, target_pos: Vec3) {
    let direction = (target_pos - transform.translation).normalize_or_zero();
    let flat_direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
    if flat_direction.length_squared() > 0.01 {
        let look_target = transform.translation + flat_direction;
        transform.look_at(look_target, Vec3::Y);
    }
}

/// 處理攻擊狀態的移動（不移動，只面向目標）
/// 返回 true 表示已處理
#[inline]
fn handle_attack_state_movement(
    behavior: &AiBehavior,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    gravity_velocity: f32,
) -> bool {
    if behavior.state != AiState::Attack {
        return false;
    }

    controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
    if let Some(target_pos) = behavior.last_known_target_pos {
        face_target(transform, target_pos);
    }
    true
}

/// 處理巡邏狀態的移動
/// 返回 true 表示需要等待（不執行移動）
#[inline]
fn handle_patrol_movement(
    patrol_path: &mut PatrolPath,
    movement: &mut AiMovement,
    transform_translation: Vec3,
    controller: &mut KinematicCharacterController,
    gravity_velocity: f32,
    dt: f32,
) -> bool {
    // 處理等待
    if patrol_path.wait_timer > 0.0 {
        patrol_path.wait_timer -= dt;
        controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
        return true;
    }

    // 取得當前巡邏點
    if let Some(waypoint) = patrol_path.current_waypoint() {
        movement.move_target = Some(waypoint);
        movement.is_running = false;

        // 檢查是否到達
        if movement.has_arrived(transform_translation) {
            patrol_path.wait_timer = patrol_path.wait_time;
            patrol_path.advance();
        }
    }
    false
}

/// 執行移動到目標位置
#[inline]
fn execute_movement_to_target(
    target: Vec3,
    transform: &mut Transform,
    movement: &AiMovement,
    controller: &mut KinematicCharacterController,
    gravity_velocity: f32,
    dt: f32,
) {
    let my_pos = transform.translation;
    let direction = (target - my_pos).normalize_or_zero();
    let flat_direction = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();

    if flat_direction.length_squared() > 0.01 {
        let speed = movement.current_speed();
        let horizontal = flat_direction * speed * dt;
        controller.translation = Some(Vec3::new(horizontal.x, gravity_velocity, horizontal.z));

        // 面向移動方向
        let look_target = transform.translation + flat_direction;
        transform.look_at(look_target, Vec3::Y);
    } else {
        controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
    }
}

/// 處理閒置狀態的移動（只應用重力）
#[inline]
fn handle_idle_state_movement(
    behavior: &AiBehavior,
    controller: &mut KinematicCharacterController,
    gravity_velocity: f32,
) -> bool {
    if behavior.state != AiState::Idle {
        return false;
    }
    controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
    true
}

/// 處理移動目標或靜止
#[inline]
fn handle_movement_or_idle(
    movement: &AiMovement,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    gravity_velocity: f32,
    dt: f32,
) {
    if let Some(target) = movement.move_target {
        execute_movement_to_target(
            target,
            transform,
            movement,
            controller,
            gravity_velocity,
            dt,
        );
    } else {
        controller.translation = Some(Vec3::new(0.0, gravity_velocity, 0.0));
    }
}

/// 處理巡邏狀態的移動
/// 返回 true 表示已處理（正在巡邏或等待中）
#[inline]
fn handle_patrol_state_movement(
    behavior: &AiBehavior,
    patrol: &mut Option<Mut<PatrolPath>>,
    movement: &mut AiMovement,
    transform_translation: Vec3,
    controller: &mut KinematicCharacterController,
    gravity_velocity: f32,
    dt: f32,
) -> bool {
    if behavior.state != AiState::Patrol {
        return false;
    }
    let Some(ref mut patrol_path) = patrol else {
        return false;
    };
    handle_patrol_movement(
        patrol_path,
        movement,
        transform_translation,
        controller,
        gravity_velocity,
        dt,
    )
}

/// 計算重力速度
#[inline]
fn calculate_gravity_velocity(output: Option<&KinematicCharacterControllerOutput>, dt: f32) -> f32 {
    let is_grounded = output.is_some_and(|o| o.grounded);
    if is_grounded {
        0.0
    } else {
        GRAVITY * dt
    }
}

/// 處理單個敵人的移動邏輯
#[inline]
#[allow(clippy::too_many_arguments)]
fn process_single_enemy_movement(
    transform: &mut Transform,
    behavior: &AiBehavior,
    movement: &mut AiMovement,
    controller: &mut KinematicCharacterController,
    patrol: &mut Option<Mut<PatrolPath>>,
    gravity_velocity: f32,
    dt: f32,
) {
    // 根據狀態處理移動（按優先級順序）
    if handle_attack_state_movement(behavior, transform, controller, gravity_velocity) {
        return;
    }
    if handle_idle_state_movement(behavior, controller, gravity_velocity) {
        return;
    }
    if handle_patrol_state_movement(
        behavior,
        patrol,
        movement,
        transform.translation,
        controller,
        gravity_velocity,
        dt,
    ) {
        return;
    }
    handle_movement_or_idle(movement, transform, controller, gravity_velocity, dt);
}

/// AI 移動系統：移動到目標位置
pub fn ai_movement_system(
    time: Res<Time>,
    mut enemy_query: Query<
        (
            &mut Transform,
            &AiBehavior,
            &mut AiMovement,
            &mut KinematicCharacterController,
            Option<&KinematicCharacterControllerOutput>,
            Option<&mut PatrolPath>,
        ),
        With<Enemy>,
    >,
) {
    let dt = time.delta_secs();

    for (mut transform, behavior, mut movement, mut controller, output, mut patrol) in
        &mut enemy_query
    {
        let gravity_velocity = calculate_gravity_velocity(output, dt);
        process_single_enemy_movement(
            &mut transform,
            behavior,
            &mut movement,
            &mut controller,
            &mut patrol,
            gravity_velocity,
            dt,
        );
    }
}
