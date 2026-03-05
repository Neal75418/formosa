//! AI 行為決策樹（狀態轉換邏輯）

use bevy::prelude::*;

use super::{AiBehavior, AiCombat, AiConfig, AiMovement, AiPerception, AiState, PatrolPath};
use crate::combat::{Enemy, Health};

// ============================================================================
// 決策系統
// ============================================================================

// ============================================================================
// 決策系統輔助函數
// ============================================================================
/// 處理逃跑狀態的開始
/// 返回 true 表示開始逃跑，應跳過後續處理
#[inline]
pub(crate) fn check_start_flee(
    config: &AiConfig,
    health_percent: f32,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    my_pos: Vec3,
    current_time: f32,
) -> bool {
    if health_percent > behavior.flee_threshold || behavior.is_fleeing {
        return false;
    }

    behavior.is_fleeing = true;
    behavior.set_state(AiState::Flee, current_time);
    movement.is_running = true;

    if let Some(target_pos) = behavior.last_known_target_pos {
        let flee_dir = (my_pos - target_pos).normalize_or_zero();
        movement.move_target = Some(my_pos + flee_dir * config.flee_distance);
    }
    true
}

/// 處理逃跑狀態的持續
/// 返回 true 表示仍在逃跑，應跳過狀態機處理
#[inline]
pub(crate) fn handle_fleeing_state(
    config: &AiConfig,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    my_pos: Vec3,
    current_time: f32,
) -> bool {
    if !behavior.is_fleeing {
        return false;
    }

    let Some(target_pos) = behavior.last_known_target_pos else {
        // 沒有目標位置，停止逃跑
        behavior.is_fleeing = false;
        behavior.set_state(AiState::Idle, current_time);
        movement.is_running = false;
        movement.move_target = None;
        return true;
    };

    let distance_from_threat_sq = my_pos.distance_squared(target_pos);

    if distance_from_threat_sq > config.alert_distance_sq() {
        // 逃離超過安全距離
        behavior.is_fleeing = false;
        behavior.set_state(AiState::Alert, current_time);
        movement.is_running = false;
        movement.move_target = None;
    } else {
        // 繼續逃跑
        let flee_dir = (my_pos - target_pos).normalize_or_zero();
        movement.move_target = Some(my_pos + flee_dir * config.flee_distance);
    }
    true
}

/// 設置追逐狀態
#[inline]
pub(crate) fn enter_chase_state(
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    current_time: f32,
) {
    behavior.set_state(AiState::Chase, current_time);
    movement.is_running = true;
    movement.move_target = behavior.last_known_target_pos;
}

/// 處理 Idle 狀態的決策
#[inline]
pub(crate) fn handle_idle_state(
    config: &AiConfig,
    perception: &AiPerception,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    has_patrol: bool,
    current_time: f32,
) {
    if perception.can_see_target {
        enter_chase_state(behavior, movement, current_time);
    } else if perception.has_heard_noise {
        behavior.set_state(AiState::Alert, current_time);
        movement.move_target = perception.noise_position;
    } else if has_patrol && behavior.state_timer > config.patrol_idle_threshold {
        behavior.set_state(AiState::Patrol, current_time);
    }
}

/// 處理 Patrol 狀態的決策
#[inline]
pub(crate) fn handle_patrol_state(
    perception: &AiPerception,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    current_time: f32,
) {
    if perception.can_see_target {
        enter_chase_state(behavior, movement, current_time);
    } else if perception.has_heard_noise {
        behavior.set_state(AiState::Alert, current_time);
        movement.move_target = perception.noise_position;
    }
}

/// 處理 Alert 狀態的決策
#[inline]
pub(crate) fn handle_alert_state(
    config: &AiConfig,
    perception: &AiPerception,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    has_patrol: bool,
    current_time: f32,
) {
    if perception.can_see_target {
        enter_chase_state(behavior, movement, current_time);
    } else if behavior.state_timer > config.alert_timeout {
        if has_patrol {
            behavior.set_state(AiState::Patrol, current_time);
        } else {
            behavior.set_state(AiState::Idle, current_time);
        }
    }
}

/// 處理 Chase 狀態的決策
#[inline]
pub(crate) fn handle_chase_state(
    config: &AiConfig,
    perception: &AiPerception,
    combat: &AiCombat,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    my_pos: Vec3,
    current_time: f32,
) {
    if let Some(target_pos) = behavior.last_known_target_pos {
        movement.move_target = Some(target_pos);

        // 在攻擊範圍內且能看到 → 攻擊
        if combat.is_in_range(my_pos, target_pos) && perception.can_see_target {
            behavior.set_state(AiState::Attack, current_time);
            movement.is_running = false;
            movement.move_target = None;
        }
    }

    // 失去目標超過閾值
    if behavior.lose_target(current_time, config.lose_target_timeout) {
        behavior.set_state(AiState::Alert, current_time);
        movement.is_running = false;
        movement.move_target = behavior.last_known_target_pos;
    }
}

/// 處理 Attack 狀態的決策
#[inline]
pub(crate) fn handle_attack_state(
    perception: &AiPerception,
    combat: &AiCombat,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    my_pos: Vec3,
    current_time: f32,
) {
    let Some(target_pos) = behavior.last_known_target_pos else {
        behavior.set_state(AiState::Alert, current_time);
        return;
    };

    if !combat.is_in_range(my_pos, target_pos) || !perception.can_see_target {
        behavior.set_state(AiState::Chase, current_time);
        movement.is_running = true;
        movement.move_target = Some(target_pos);
    }
}

/// 處理 TakingCover 狀態的決策
#[inline]
pub(crate) fn handle_taking_cover_state(
    config: &AiConfig,
    perception: &AiPerception,
    health_percent: f32,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    current_time: f32,
) {
    let Some(target_pos) = behavior.last_known_target_pos else {
        return;
    };

    if !perception.can_see_target && behavior.state_timer > config.alert_timeout {
        behavior.set_state(AiState::Alert, current_time);
    } else if health_percent > config.low_health_threshold {
        // 血量恢復，重新進攻
        behavior.set_state(AiState::Chase, current_time);
        movement.is_running = true;
        movement.move_target = Some(target_pos);
    }
}

/// AI 決策系統：根據感知更新狀態
/// 每幀執行，確保即時響應
pub fn ai_decision_system(
    time: Res<Time>,
    config: Res<AiConfig>,
    mut enemy_query: Query<
        (
            &Transform,
            &Health,
            &AiPerception,
            &AiCombat,
            &mut AiBehavior,
            &mut AiMovement,
            Option<&PatrolPath>,
        ),
        With<Enemy>,
    >,
    // 用於查詢目標實體的當前位置
    transforms_query: Query<&Transform, Without<Enemy>>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    for (transform, health, perception, combat, mut behavior, mut movement, patrol) in
        &mut enemy_query
    {
        let my_pos = transform.translation;
        let health_percent = health.percentage();
        let has_patrol = patrol.is_some();

        // 更新狀態計時器
        behavior.tick(dt);

        // 更新目標位置（確保逃跑方向基於最新位置）
        if let Some(target_entity) = behavior.target {
            if let Ok(target_transform) = transforms_query.get(target_entity) {
                behavior.last_known_target_pos = Some(target_transform.translation);
            }
        }

        // 檢查是否應該逃跑
        if check_start_flee(
            &config,
            health_percent,
            &mut behavior,
            &mut movement,
            my_pos,
            current_time,
        ) {
            continue;
        }

        // 逃跑狀態持續
        if handle_fleeing_state(&config, &mut behavior, &mut movement, my_pos, current_time) {
            continue;
        }

        // 正常狀態機轉換
        match behavior.state {
            AiState::Idle => {
                handle_idle_state(
                    &config,
                    perception,
                    &mut behavior,
                    &mut movement,
                    has_patrol,
                    current_time,
                );
            }
            AiState::Patrol => {
                handle_patrol_state(perception, &mut behavior, &mut movement, current_time);
            }
            AiState::Alert => {
                handle_alert_state(
                    &config,
                    perception,
                    &mut behavior,
                    &mut movement,
                    has_patrol,
                    current_time,
                );
            }
            AiState::Chase => {
                handle_chase_state(
                    &config,
                    perception,
                    combat,
                    &mut behavior,
                    &mut movement,
                    my_pos,
                    current_time,
                );
            }
            AiState::Attack => {
                handle_attack_state(
                    perception,
                    combat,
                    &mut behavior,
                    &mut movement,
                    my_pos,
                    current_time,
                );
            }
            AiState::Flee => {
                // 逃跑狀態在上面已處理
            }
            AiState::TakingCover => {
                handle_taking_cover_state(
                    &config,
                    perception,
                    health_percent,
                    &mut behavior,
                    &mut movement,
                    current_time,
                );
            }
        }
    }
}
