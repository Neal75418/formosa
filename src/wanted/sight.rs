//! 警察看玩家、朝玩家開槍時瞄準的點

use bevy::prelude::*;

use super::config::PLAYER_NECK_HEIGHT;
use crate::combat::{PeekState, PlayerCoverState, PlayerCoverType};
use crate::player::Player;

/// 站著時瞄頸部（頭和脖子露出來就看得到）；蹲下、或躲在矮掩體後沒探出時，瞄身體中心
pub(crate) fn player_sight_point(
    transform: &Transform,
    player: &Player,
    cover: Option<&PlayerCoverState>,
) -> Vec3 {
    let hiding_in_low_cover = cover.is_some_and(|cover| {
        cover.is_in_cover
            && cover.cover_type == PlayerCoverType::Low
            && cover.peek_state == PeekState::Hidden
    });
    if player.is_crouching || hiding_in_low_cover {
        transform.translation
    } else {
        transform.translation + Vec3::Y * PLAYER_NECK_HEIGHT
    }
}
