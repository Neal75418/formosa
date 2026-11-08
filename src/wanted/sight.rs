//! 警察、直升機看玩家：瞄準的點、射線打到什麼算看到

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};

use super::config::{PLAYER_NECK_HEIGHT, RAYCAST_HIT_TOLERANCE};
use crate::combat::{PeekState, PlayerCoverState, PlayerCoverType};
use crate::core::{rapier_real_to_f32, GameState};
use crate::player::{Player, VehicleTransitionPhase, VehicleTransitionState};

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

/// 玩家正坐在哪台車上
#[derive(SystemParam)]
pub(crate) struct SeatedVehicle<'w> {
    game_state: Res<'w, GameState>,
    transition: Res<'w, VehicleTransitionState>,
}

impl SeatedVehicle<'_> {
    /// `GameState.current_vehicle` 上車完成才設、下車動畫整段播完才清；下車時開完門玩家就往車外走，
    /// 只有開車中、下車開門那段算坐在車上
    fn vehicle(&self) -> Option<Entity> {
        let seated = matches!(
            self.transition.phase,
            VehicleTransitionPhase::None | VehicleTransitionPhase::OpeningDoorExit
        );
        self.game_state.current_vehicle.filter(|_| seated)
    }
}

/// 射線打到哪些實體算打到玩家：玩家本人，坐在車上時還有那台車
/// （坐在車上時玩家的碰撞體關著、位置在車中心，射線一定先打到車）
#[derive(Clone, Copy)]
pub(crate) struct SightTarget {
    player: Entity,
    vehicle: Option<Entity>,
}

impl SightTarget {
    pub(crate) fn new(player: Entity, seated: &SeatedVehicle) -> Self {
        Self {
            player,
            vehicle: seated.vehicle(),
        }
    }

    pub(crate) fn is_hit_by(self, hit_entity: Entity) -> bool {
        hit_entity == self.player || self.vehicle == Some(hit_entity)
    }
}

/// 從 origin 朝 aim_point 打射線（方向和長度從起點算），看不看得到玩家：
/// 打到玩家（或他坐的車）、打到的東西就在瞄準點附近，都算看到；什麼都沒打到也算
/// （射線只到瞄準點，沒打到就是中間沒有遮擋；膠囊射線偶爾會漏打）
pub(crate) fn can_see_player(
    rapier: &RapierContext,
    origin: Vec3,
    aim_point: Vec3,
    filter: QueryFilter,
    target: SightTarget,
) -> bool {
    let to_target = aim_point - origin;
    let distance = to_target.length();
    if distance <= f32::EPSILON {
        return true;
    }
    rapier
        .cast_ray(
            origin,
            to_target / distance,
            distance as RapierReal,
            true,
            filter,
        )
        .is_none_or(|(hit_entity, toi)| {
            target.is_hit_by(hit_entity)
                || rapier_real_to_f32(toi) >= distance - RAYCAST_HIT_TOLERANCE
        })
}
