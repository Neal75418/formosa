//! 玩家掩體系統
//!
//! 允許玩家進入掩體、探出射擊、掩體間移動


use bevy::prelude::*;

use crate::ai::CoverPoint;
use crate::core::safe_normalize;
use crate::player::Player;

// ============================================================================
// 常數
// ============================================================================

/// 玩家進入掩體的最大距離
const COVER_ENTER_DISTANCE: f32 = 2.5;
/// 玩家進入掩體的距離平方
const COVER_ENTER_DISTANCE_SQ: f32 = COVER_ENTER_DISTANCE * COVER_ENTER_DISTANCE;
/// 掩體間移動的最大距離
const COVER_SWAP_DISTANCE: f32 = 8.0;
/// 掩體間移動距離平方
const COVER_SWAP_DISTANCE_SQ: f32 = COVER_SWAP_DISTANCE * COVER_SWAP_DISTANCE;
/// 探出持續時間（秒）
const PEEK_DURATION: f32 = 0.3;
/// 探出偏移量
const PEEK_OFFSET: f32 = 0.8;

// ============================================================================
// 組件
// ============================================================================

/// 玩家掩體狀態
#[derive(Component, Default)]
pub struct PlayerCoverState {
    /// 是否在掩體中
    pub is_in_cover: bool,
    /// 當前使用的掩體實體
    pub current_cover: Option<Entity>,
    /// 掩體類型
    pub cover_type: PlayerCoverType,
    /// 探出狀態
    pub peek_state: PeekState,
    /// 探出計時器
    pub peek_timer: f32,
    /// 掩體相對位置（沿掩體邊緣的偏移）
    pub cover_offset: f32,
    /// 進入掩體的原始位置（用於平滑過渡）
    pub original_position: Vec3,
    /// 目標掩體位置
    pub target_cover_position: Vec3,
    /// 過渡進度 (0.0 - 1.0)
    pub transition_progress: f32,
    /// 是否正在過渡中
    pub is_transitioning: bool,
}

/// 玩家掩體類型
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerCoverType {
    #[default]
    None,
    /// 低掩體（需蹲下）
    Low,
    /// 高掩體（可站立）
    High,
}

/// 探出狀態
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PeekState {
    #[default]
    Hidden,     // 完全躲藏
    PeekingLeft,  // 向左探出
    PeekingRight, // 向右探出
    PeekingUp,    // 向上探出（低掩體）
}

impl PeekState {
    /// 是否正在探出
    pub fn is_peeking(&self) -> bool {
        !matches!(self, PeekState::Hidden)
    }

    /// 取得探出方向偏移
    pub fn get_offset(&self, cover_direction: Vec3) -> Vec3 {
        match self {
            PeekState::Hidden => Vec3::ZERO,
            PeekState::PeekingLeft => {
                let left = safe_normalize(cover_direction.cross(Vec3::Y));
                left * PEEK_OFFSET
            }
            PeekState::PeekingRight => {
                let right = safe_normalize(Vec3::Y.cross(cover_direction));
                right * PEEK_OFFSET
            }
            PeekState::PeekingUp => Vec3::Y * PEEK_OFFSET,
        }
    }
}

// ============================================================================
// 事件
// ============================================================================

/// 玩家掩體事件
#[derive(Message)]
pub enum PlayerCoverEvent {
    /// 進入掩體
    EnterCover { cover_entity: Entity },
    /// 離開掩體
    ExitCover,
    /// 開始探出
    StartPeek { direction: PeekState },
    /// 結束探出
    EndPeek,
    /// 移動到相鄰掩體
    SwapCover { target_cover: Entity },
}

// ============================================================================
// 系統
// ============================================================================

// ============================================================================
// 輸入系統輔助函數
// ============================================================================
/// 檢查離開掩體輸入
#[inline]
fn check_exit_cover_input(keyboard: &ButtonInput<KeyCode>) -> bool {
    keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::ShiftLeft)
}

/// 檢測探出方向輸入
#[inline]
fn get_peek_direction(keyboard: &ButtonInput<KeyCode>, cover_type: PlayerCoverType) -> Option<PeekState> {
    if keyboard.pressed(KeyCode::KeyA) {
        Some(PeekState::PeekingLeft)
    } else if keyboard.pressed(KeyCode::KeyD) {
        Some(PeekState::PeekingRight)
    } else if keyboard.pressed(KeyCode::KeyW) && cover_type == PlayerCoverType::Low {
        Some(PeekState::PeekingUp)
    } else {
        None
    }
}

/// 處理掩體內探出輸入
#[inline]
fn handle_peek_input(
    keyboard: &ButtonInput<KeyCode>,
    cover_state: &PlayerCoverState,
    cover_events: &mut MessageWriter<PlayerCoverEvent>,
) {
    if let Some(direction) = get_peek_direction(keyboard, cover_state.cover_type) {
        cover_events.write(PlayerCoverEvent::StartPeek { direction });
    } else if cover_state.peek_state.is_peeking() {
        cover_events.write(PlayerCoverEvent::EndPeek);
    }
}

/// 處理掩體間移動輸入
#[inline]
fn handle_cover_swap_input(
    keyboard: &ButtonInput<KeyCode>,
    player_pos: Vec3,
    cover_state: &PlayerCoverState,
    cover_query: &Query<(Entity, &Transform, &CoverPoint)>,
    cover_events: &mut MessageWriter<PlayerCoverEvent>,
) {
    // 左側移動
    let left_pressed = keyboard.just_pressed(KeyCode::KeyQ) || keyboard.just_pressed(KeyCode::ArrowLeft);
    if left_pressed {
        if let Some(target) = find_adjacent_cover(player_pos, cover_state.current_cover, cover_query, true) {
            cover_events.write(PlayerCoverEvent::SwapCover { target_cover: target });
        }
    }

    // 右側移動
    let right_pressed = keyboard.just_pressed(KeyCode::KeyE) || keyboard.just_pressed(KeyCode::ArrowRight);
    if right_pressed {
        if let Some(target) = find_adjacent_cover(player_pos, cover_state.current_cover, cover_query, false) {
            cover_events.write(PlayerCoverEvent::SwapCover { target_cover: target });
        }
    }
}

/// 處理玩家掩體輸入
pub fn player_cover_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    _mouse: Res<ButtonInput<MouseButton>>,
    player_query: Query<(Entity, &Transform, &PlayerCoverState), With<Player>>,
    cover_query: Query<(Entity, &Transform, &CoverPoint)>,
    mut cover_events: MessageWriter<PlayerCoverEvent>,
) {
    let Ok((_player_entity, player_transform, cover_state)) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation;

    if cover_state.is_in_cover {
        // 離開掩體
        if check_exit_cover_input(&keyboard) {
            cover_events.write(PlayerCoverEvent::ExitCover);
            return;
        }

        // 探出射擊
        handle_peek_input(&keyboard, cover_state, &mut cover_events);

        // 掩體間移動
        handle_cover_swap_input(&keyboard, player_pos, cover_state, &cover_query, &mut cover_events);
    } else {
        // 按 C 進入掩體
        if keyboard.just_pressed(KeyCode::KeyC) {
            if let Some(cover_entity) = find_nearest_cover(player_pos, &cover_query) {
                cover_events.write(PlayerCoverEvent::EnterCover { cover_entity });
            }
        }
    }
}

// ============================================================================
// 事件系統輔助函數
// ============================================================================
/// 根據掩體高度決定掩體類型
#[inline]
fn determine_cover_type(height: f32) -> PlayerCoverType {
    if height < 1.2 { PlayerCoverType::Low } else { PlayerCoverType::High }
}

/// 重置掩體狀態為非掩體狀態
#[inline]
fn reset_cover_state(cover_state: &mut PlayerCoverState) {
    cover_state.is_in_cover = false;
    cover_state.current_cover = None;
    cover_state.cover_type = PlayerCoverType::None;
    cover_state.peek_state = PeekState::Hidden;
    cover_state.is_transitioning = false;
}

/// 設置進入掩體的過渡狀態
#[inline]
fn setup_cover_transition(
    cover_state: &mut PlayerCoverState,
    cover_entity: Entity,
    cover_point: &CoverPoint,
    player_pos: Vec3,
    cover_pos: Vec3,
) {
    cover_state.is_in_cover = true;
    cover_state.current_cover = Some(cover_entity);
    cover_state.cover_type = determine_cover_type(cover_point.height);
    cover_state.peek_state = PeekState::Hidden;
    cover_state.original_position = player_pos;
    cover_state.target_cover_position = cover_pos;
    cover_state.transition_progress = 0.0;
    cover_state.is_transitioning = true;
}

/// 處理進入掩體事件
fn handle_enter_cover(
    cover_entity: Entity,
    player_entity: Entity,
    player_pos: Vec3,
    cover_state: &mut PlayerCoverState,
    cover_query: &mut Query<(&Transform, &mut CoverPoint), Without<Player>>,
) {
    let Ok((cover_transform, mut cover_point)) = cover_query.get_mut(cover_entity) else {
        return;
    };

    if cover_point.occupied {
        info!("掩體已被佔用");
        return;
    }

    setup_cover_transition(cover_state, cover_entity, &cover_point, player_pos, cover_transform.translation);
    cover_point.occupy(player_entity);
    info!("進入掩體: {:?}", cover_state.cover_type);
}

/// 處理離開掩體事件
fn handle_exit_cover(
    cover_state: &mut PlayerCoverState,
    cover_query: &mut Query<(&Transform, &mut CoverPoint), Without<Player>>,
) {
    if let Some(cover_entity) = cover_state.current_cover {
        if let Ok((_, mut cover_point)) = cover_query.get_mut(cover_entity) {
            cover_point.release();
        }
    }
    reset_cover_state(cover_state);
    info!("離開掩體");
}

/// 處理掩體間移動事件
fn handle_swap_cover(
    target_cover: Entity,
    player_entity: Entity,
    player_pos: Vec3,
    cover_state: &mut PlayerCoverState,
    cover_query: &mut Query<(&Transform, &mut CoverPoint), Without<Player>>,
) {
    // 釋放當前掩體
    if let Some(old_cover) = cover_state.current_cover {
        if let Ok((_, mut old_point)) = cover_query.get_mut(old_cover) {
            old_point.release();
        }
    }

    // 進入新掩體
    let Ok((new_transform, mut new_point)) = cover_query.get_mut(target_cover) else { return };
    if !new_point.is_available() { return }

    setup_cover_transition(cover_state, target_cover, &new_point, player_pos, new_transform.translation);
    new_point.occupy(player_entity);
    info!("移動到新掩體");
}

/// 處理玩家掩體事件
pub fn player_cover_event_system(
    mut events: MessageReader<PlayerCoverEvent>,
    mut player_query: Query<(Entity, &mut Transform, &mut PlayerCoverState), With<Player>>,
    mut cover_query: Query<(&Transform, &mut CoverPoint), Without<Player>>,
) {
    let Ok((player_entity, player_transform, mut cover_state)) = player_query.single_mut() else {
        return;
    };

    let player_pos = player_transform.translation;

    for event in events.read() {
        match event {
            PlayerCoverEvent::EnterCover { cover_entity } => {
                handle_enter_cover(*cover_entity, player_entity, player_pos, &mut cover_state, &mut cover_query);
            }
            PlayerCoverEvent::ExitCover => {
                handle_exit_cover(&mut cover_state, &mut cover_query);
            }
            PlayerCoverEvent::StartPeek { direction } => {
                if cover_state.is_in_cover && !cover_state.is_transitioning {
                    cover_state.peek_state = *direction;
                    cover_state.peek_timer = PEEK_DURATION;
                }
            }
            PlayerCoverEvent::EndPeek => {
                cover_state.peek_state = PeekState::Hidden;
                cover_state.peek_timer = 0.0;
            }
            PlayerCoverEvent::SwapCover { target_cover } => {
                handle_swap_cover(*target_cover, player_entity, player_pos, &mut cover_state, &mut cover_query);
            }
        }
    }
}

// ============================================================================
// 更新系統輔助函數
// ============================================================================
/// 檢查掩體是否被摧毀，如果是則重置狀態
/// 返回 true 表示掩體被摧毀
#[inline]
fn check_cover_destroyed(
    cover_state: &mut PlayerCoverState,
    cover_query: &Query<(&Transform, &CoverPoint), Without<Player>>,
) -> bool {
    let Some(cover_entity) = cover_state.current_cover else { return false };
    if cover_query.get(cover_entity).is_ok() { return false }

    warn!("掩體被摧毀，強制離開掩體狀態");
    reset_cover_state(cover_state);
    true
}

/// 更新掩體過渡動畫
/// 返回插值後的位置（如果正在過渡中）
#[inline]
fn update_cover_transition(cover_state: &mut PlayerCoverState, delta: f32) -> Option<Vec3> {
    if !cover_state.is_transitioning { return None }

    cover_state.transition_progress += delta * 5.0; // 0.2 秒完成過渡

    if cover_state.transition_progress >= 1.0 {
        cover_state.transition_progress = 1.0;
        cover_state.is_transitioning = false;
    }

    Some(cover_state.original_position.lerp(
        cover_state.target_cover_position,
        cover_state.transition_progress,
    ))
}

/// 應用探出偏移和朝向
#[inline]
fn apply_peek_offset(
    cover_state: &PlayerCoverState,
    cover_transform: &Transform,
    cover_point: &CoverPoint,
    player_transform: &mut Transform,
) {
    let peek_offset = cover_state.peek_state.get_offset(cover_point.cover_direction);
    player_transform.translation = cover_transform.translation + peek_offset;

    // 探出時面向掩體方向
    if cover_state.peek_state.is_peeking() {
        let look_dir = cover_point.cover_direction;
        player_transform.rotation = Quat::from_rotation_y(look_dir.x.atan2(look_dir.z));
    }
}

/// 更新玩家掩體位置和動畫
pub fn player_cover_update_system(
    time: Res<Time>,
    mut player_query: Query<(&mut Transform, &mut PlayerCoverState), With<Player>>,
    cover_query: Query<(&Transform, &CoverPoint), Without<Player>>,
) {
    let Ok((mut player_transform, mut cover_state)) = player_query.single_mut() else {
        return;
    };

    if !cover_state.is_in_cover { return }
    if check_cover_destroyed(&mut cover_state, &cover_query) { return }

    let delta = time.delta_secs();

    // 處理過渡動畫
    if let Some(new_pos) = update_cover_transition(&mut cover_state, delta) {
        player_transform.translation = new_pos;
    }

    // 處理探出偏移（非過渡期間）
    if !cover_state.is_transitioning {
        if let Some(cover_entity) = cover_state.current_cover {
            if let Ok((cover_transform, cover_point)) = cover_query.get(cover_entity) {
                apply_peek_offset(&cover_state, cover_transform, cover_point, &mut player_transform);
            }
        }
    }

    // 更新探出計時器：超時後自動返回隱藏狀態
    if cover_state.peek_timer > 0.0 {
        cover_state.peek_timer -= delta;
        if cover_state.peek_timer <= 0.0 && cover_state.peek_state.is_peeking() {
            cover_state.peek_state = PeekState::Hidden;
            cover_state.peek_timer = 0.0;
        }
    }
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 找到最近的可用掩體
fn find_nearest_cover(
    player_pos: Vec3,
    cover_query: &Query<(Entity, &Transform, &CoverPoint)>,
) -> Option<Entity> {
    let mut nearest: Option<(Entity, f32)> = None;

    for (entity, transform, cover_point) in cover_query.iter() {
        if cover_point.occupied {
            continue;
        }

        let distance_sq = player_pos.distance_squared(transform.translation);
        if distance_sq > COVER_ENTER_DISTANCE_SQ {
            continue;
        }

        match nearest {
            None => nearest = Some((entity, distance_sq)),
            Some((_, d)) if distance_sq < d => nearest = Some((entity, distance_sq)),
            _ => {}
        }
    }

    nearest.map(|(e, _)| e)
}

/// 找到相鄰的掩體（用於掩體間移動）
fn find_adjacent_cover(
    player_pos: Vec3,
    current_cover: Option<Entity>,
    cover_query: &Query<(Entity, &Transform, &CoverPoint)>,
    left_side: bool,
) -> Option<Entity> {
    let current_cover = current_cover?;
    let (_, current_transform, current_point) = cover_query.get(current_cover).ok()?;

    // 計算左/右方向
    let side_dir = if left_side {
        safe_normalize(current_point.cover_direction.cross(Vec3::Y))
    } else {
        safe_normalize(Vec3::Y.cross(current_point.cover_direction))
    };

    let mut best: Option<(Entity, f32)> = None;

    for (entity, transform, cover_point) in cover_query.iter() {
        if entity == current_cover || cover_point.occupied {
            continue;
        }

        let distance_sq = player_pos.distance_squared(transform.translation);
        if distance_sq > COVER_SWAP_DISTANCE_SQ {
            continue;
        }

        // 檢查是否在正確的方向
        let to_cover = safe_normalize(transform.translation - current_transform.translation);
        let dot = side_dir.dot(to_cover);

        if dot > 0.3 {
            // 在指定方向
            match best {
                None => best = Some((entity, distance_sq)),
                Some((_, d)) if distance_sq < d => best = Some((entity, distance_sq)),
                _ => {}
            }
        }
    }

    best.map(|(e, _)| e)
}
