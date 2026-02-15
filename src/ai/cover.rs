//! AI 掩體搜尋與評估

use bevy::prelude::*;

use super::{AiBehavior, AiConfig, AiMovement, AiState, CoverPoint, CoverSeeker};
use crate::combat::{DeathEvent, Enemy, Health, Ragdoll};
use crate::player::Player;

// ============================================================================
// 掩體系統 (GTA 5 風格)
// ============================================================================

// ============================================================================
// 掩體系統輔助函數
// ============================================================================
/// 處理在掩體中的行為（探出射擊）
/// 返回 true 表示正在掩體中，應跳過後續處理
#[inline]
fn handle_in_cover_state(
    seeker: &mut CoverSeeker,
    behavior: &mut AiBehavior,
    current_time: f32,
) -> bool {
    if !seeker.is_in_cover {
        return false;
    }

    // 處理探出射擊
    if seeker.is_peeking {
        // 探出時可以攻擊
        if behavior.state != AiState::Attack {
            behavior.set_state(AiState::Attack, current_time);
        }
        // 探出 0.5 秒後縮回
        if seeker.peek_timer <= seeker.peek_interval - 0.5 {
            seeker.end_peek();
            behavior.set_state(AiState::TakingCover, current_time);
        }
    }
    true
}

/// 尋找最佳掩體
/// 返回 (掩體實體, 掩體位置, 距離平方)
#[inline]
fn find_best_cover(
    my_pos: Vec3,
    player_pos: Vec3,
    max_cover_distance: f32,
    cover_query: &Query<(Entity, &Transform, &mut CoverPoint)>,
) -> Option<(Entity, Vec3, f32)> {
    let max_cover_distance_sq = max_cover_distance * max_cover_distance;
    let mut best_cover: Option<(Entity, Vec3, f32)> = None;

    for (cover_entity, cover_transform, cover) in cover_query.iter() {
        if !cover.is_available() {
            continue;
        }

        let cover_pos = cover_transform.translation;
        let distance_sq = my_pos.distance_squared(cover_pos);

        // 檢查距離是否在範圍內
        if distance_sq > max_cover_distance_sq {
            continue;
        }

        // 檢查掩體是否能遮擋玩家
        if !cover.is_covered_from(
            cover_pos,
            cover_pos - cover.cover_direction * 0.5,
            player_pos,
        ) {
            continue;
        }

        // 選擇最近的掩體
        if best_cover.is_none_or(|(_, _, d)| distance_sq < d) {
            best_cover = Some((cover_entity, cover_pos, distance_sq));
        }
    }

    best_cover
}

/// 移動到掩體並佔用
#[inline]
#[allow(clippy::too_many_arguments)]
fn move_to_cover(
    enemy_entity: Entity,
    cover_entity: Entity,
    cover_pos: Vec3,
    seeker: &mut CoverSeeker,
    behavior: &mut AiBehavior,
    movement: &mut AiMovement,
    cover_query: &mut Query<(Entity, &Transform, &mut CoverPoint)>,
    current_time: f32,
) {
    // 檢查掩體是否有效
    let Ok((_, _, cover)) = cover_query.get(cover_entity) else {
        return;
    };

    seeker.target_cover = Some(cover_entity);
    behavior.set_state(AiState::TakingCover, current_time);
    movement.is_running = true;

    // 移動到掩體後方
    let behind_cover = cover_pos - cover.cover_direction * 0.8;
    movement.move_target = Some(behind_cover);

    // 佔用掩體
    if let Ok((_, _, mut cover_mut)) = cover_query.get_mut(cover_entity) {
        cover_mut.occupy(enemy_entity);
    }
}

/// 檢查是否到達掩體
#[inline]
fn check_cover_arrival(
    my_pos: Vec3,
    seeker: &mut CoverSeeker,
    movement: &mut AiMovement,
    cover_query: &Query<(Entity, &Transform, &mut CoverPoint)>,
    config: &AiConfig,
) {
    let Some(cover_entity) = seeker.target_cover else {
        return;
    };

    if let Ok((_, cover_transform, _)) = cover_query.get(cover_entity) {
        let cover_pos = cover_transform.translation;
        if my_pos.distance_squared(cover_pos) < config.cover_arrival_sq {
            // 到達掩體
            seeker.enter_cover(cover_entity);
            movement.is_running = false;
            movement.move_target = None;
        }
    }
}

/// AI 掩體尋找系統
/// 當 AI 血量低時，尋找附近的掩體並移動過去
pub fn ai_cover_system(
    time: Res<Time>,
    config: Res<AiConfig>,
    mut enemy_query: Query<
        (
            Entity,
            &Transform,
            &Health,
            &mut AiBehavior,
            &mut AiMovement,
            &mut CoverSeeker,
        ),
        (With<Enemy>, Without<Ragdoll>),
    >,
    mut cover_query: Query<(Entity, &Transform, &mut CoverPoint)>,
    player_query: Query<&Transform, With<Player>>,
) {
    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    let player_pos = match player_query.single() {
        Ok(t) => t.translation,
        Err(_) => return,
    };

    for (enemy_entity, transform, health, mut behavior, mut movement, mut seeker) in
        &mut enemy_query
    {
        let my_pos = transform.translation;
        let health_percent = health.percentage();

        // 更新掩體計時器
        seeker.tick(dt);

        // 處理在掩體中的狀態
        if handle_in_cover_state(&mut seeker, &mut behavior, current_time) {
            continue;
        }

        // 檢查是否應該尋找掩體
        if seeker.should_seek_cover(health_percent) && behavior.state != AiState::Flee {
            // 尋找最佳掩體
            if let Some((cover_entity, cover_pos, _)) =
                find_best_cover(my_pos, player_pos, seeker.max_cover_distance, &cover_query)
            {
                // 移動到掩體並佔用
                move_to_cover(
                    enemy_entity,
                    cover_entity,
                    cover_pos,
                    &mut seeker,
                    &mut behavior,
                    &mut movement,
                    &mut cover_query,
                    current_time,
                );
            }
        }

        // 檢查是否到達掩體
        check_cover_arrival(my_pos, &mut seeker, &mut movement, &cover_query, &config);
    }
}

/// 掩體釋放系統
/// 當敵人死亡或變成布娃娃時，釋放其佔用的掩體
/// 也處理掩體佔用者實體不存在的清理
///
///優化版本：只在有死亡事件時執行完整清理
pub fn cover_release_system(
    mut death_events: MessageReader<DeathEvent>,
    enemy_query: Query<&CoverSeeker, With<Enemy>>,
    mut cover_query: Query<&mut CoverPoint>,
) {
    let mut had_deaths = false;

    // 處理死亡事件，釋放死亡敵人佔用的掩體
    for event in death_events.read() {
        had_deaths = true;

        // 使用 let-else 模式減少嵌套
        let Ok(seeker) = enemy_query.get(event.entity) else {
            continue;
        };
        let Some(cover_entity) = seeker.target_cover else {
            continue;
        };
        let Ok(mut cover) = cover_query.get_mut(cover_entity) else {
            continue;
        };
        cover.release();
    }

    // 只在有死亡事件時才執行完整清理
    // 這樣可以避免每幀都遍歷所有掩體
    if !had_deaths {
        return;
    }

    // 清理：釋放佔用者已不存在的掩體
    // 這處理了死亡實體在查詢前已被移除的情況
    for mut cover in cover_query.iter_mut() {
        // 跳過未被佔用的掩體
        if !cover.occupied {
            continue;
        }
        // 檢查佔用者是否仍存在
        let Some(occupant) = cover.occupant else {
            continue;
        };
        if enemy_query.get(occupant).is_err() {
            cover.release();
        }
    }
}
