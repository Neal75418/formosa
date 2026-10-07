//! 車輛進出動畫系統 (GTA 5 風格)
//!
//! 處理上下車動畫的位置插值和狀態轉換

use super::PlayerConfig;
use super::{Player, VehicleTransitionPhase, VehicleTransitionState};
use crate::combat::RespawnState;
use crate::core::{ease_in_out_cubic, GameState};
use crate::pedestrian::Pedestrian;
use crate::vehicle::{apply_vehicle_physics_mode, NpcVehicle, Vehicle, VehiclePhysicsMode};
use crate::wanted::CrimeEvent;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

// ============================================================================
// 常數
// ============================================================================

/// 方向向量最小長度平方（低於此值不做旋轉）
const MIN_DIRECTION_SQ: f32 = 0.01;
/// 角色朝向插值係數（越小轉向越慢）
const ROTATION_SMOOTHNESS: f32 = 0.2;
/// 上車動畫進度超過此值後隱藏玩家
const ENTER_VISIBILITY_POINT: f32 = 0.5;
/// 下車動畫進度超過此值後顯示玩家
const EXIT_VISIBILITY_START: f32 = 0.3;
/// 下車後走離車輛的距離
const WALK_AWAY_DISTANCE: f32 = 0.5;

// ============================================================================
// 車輛進出動畫系統
// ============================================================================

/// 車輛進出動畫更新系統
/// 處理上下車動畫的位置插值和狀態轉換
#[allow(clippy::too_many_lines)]
pub fn vehicle_transition_animation_system(
    time: Res<Time>,
    mut commands: Commands,
    mut transition: ResMut<VehicleTransitionState>,
    mut game_state: ResMut<GameState>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<Vehicle>)>,
    mut vehicle_query: Query<(&Transform, &mut Vehicle), Without<Player>>,
    velocity_query: Query<&Velocity>,
    mut visibility_query: Query<&mut Visibility, With<Player>>,
    mut crime_events: MessageWriter<CrimeEvent>,
    pedestrian_query: Query<&Transform, (With<Pedestrian>, Without<Player>, Without<Vehicle>)>,
    config: Res<PlayerConfig>,
) {
    if !transition.is_animating() {
        return;
    }

    let dt = time.delta_secs();
    let should_advance = transition.update(dt);

    let Ok(mut player_transform) = player_query.single_mut() else {
        return;
    };
    let Some(vehicle_entity) = transition.target_vehicle else {
        transition.reset();
        return;
    };

    // 取得車輛資訊
    let vehicle_info = vehicle_query
        .get(vehicle_entity)
        .ok()
        .map(|(t, _)| t.translation);
    let Some(vehicle_pos) = vehicle_info else {
        // 車不見了：動畫中止，上車途中已隱藏的玩家要重新出現、回到地面高度
        transition.reset();
        set_player_visibility(&mut visibility_query, true);
        player_transform.translation.y = config.interaction.exit_ground_offset;
        return;
    };

    // 根據當前階段處理動畫
    let progress = ease_in_out_cubic(transition.progress.clamp(0.0, 1.0));
    let ground_y = config.interaction.exit_ground_offset;

    match transition.phase {
        VehicleTransitionPhase::WalkingToVehicle => {
            // 玩家走向車門
            let new_pos = transition
                .start_position
                .lerp(transition.target_position, progress);
            player_transform.translation = new_pos;
            player_transform.translation.y = ground_y;

            // 面向車輛
            let to_vehicle_delta = vehicle_pos - player_transform.translation;
            if to_vehicle_delta.length_squared() > MIN_DIRECTION_SQ {
                let target_rotation = Player::rotation_facing(to_vehicle_delta);
                player_transform.rotation = player_transform
                    .rotation
                    .slerp(target_rotation, ROTATION_SMOOTHNESS);
            }
        }
        // 玩家停在門旁，門正在打開（視覺效果在其他系統處理）
        VehicleTransitionPhase::OpeningDoor | VehicleTransitionPhase::None => {}
        VehicleTransitionPhase::EnteringVehicle => {
            // 玩家從門旁移動到座位
            let new_pos = transition.target_position.lerp(vehicle_pos, progress);
            player_transform.translation = new_pos;
            // 逐漸隱藏玩家
            if progress > ENTER_VISIBILITY_POINT {
                set_player_visibility(&mut visibility_query, false);
            }
        }
        VehicleTransitionPhase::ClosingDoor => {
            // 門正在關閉，玩家已經在車內
            set_player_visibility(&mut visibility_query, false);
        }
        VehicleTransitionPhase::OpeningDoorExit => {
            // 下車：門正在打開，玩家即將可見
            if progress > EXIT_VISIBILITY_START {
                set_player_visibility(&mut visibility_query, true);
                player_transform.translation = vehicle_pos;
                player_transform.translation.y = ground_y;
            }
        }
        VehicleTransitionPhase::ExitingVehicle => {
            // 玩家從座位移動到門外
            let new_pos = transition
                .start_position
                .lerp(transition.target_position, progress);
            player_transform.translation = new_pos;
            player_transform.translation.y = ground_y;
            set_player_visibility(&mut visibility_query, true);
        }
        VehicleTransitionPhase::ClosingDoorExit => {
            // 門正在關閉
            player_transform.translation = transition.target_position;
            player_transform.translation.y = ground_y;
        }
        VehicleTransitionPhase::WalkingAway => {
            // 玩家走離車輛（小距離移動）
            let away_delta = transition.target_position - vehicle_pos;
            let away_dir = if away_delta.length_squared() > 1e-6 {
                away_delta.normalize()
            } else {
                Vec3::Z // 預設朝前走
            };
            let final_pos = transition.target_position + away_dir * WALK_AWAY_DISTANCE;
            let new_pos = transition.target_position.lerp(final_pos, progress);
            player_transform.translation = new_pos;
            player_transform.translation.y = ground_y;
        }
    }

    // 切換到下一階段
    if !should_advance {
        return;
    }

    let current_phase = transition.phase;
    transition.advance_phase();

    // 處理狀態變更
    match current_phase {
        VehicleTransitionPhase::ClosingDoor => {
            handle_enter_vehicle_complete(
                vehicle_entity,
                &mut commands,
                &velocity_query,
                &mut game_state,
                &mut vehicle_query,
                &pedestrian_query,
                &mut crime_events,
                &config,
            );
        }
        VehicleTransitionPhase::WalkingAway => {
            handle_exit_vehicle_complete(
                vehicle_entity,
                &mut commands,
                &mut game_state,
                &mut vehicle_query,
            );
        }
        _ => {}
    }
}

/// 設定玩家可見性
fn set_player_visibility(
    visibility_query: &mut Query<&mut Visibility, With<Player>>,
    visible: bool,
) {
    let Ok(mut vis) = visibility_query.single_mut() else {
        return;
    };
    *vis = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

/// 檢查是否有目擊者（GTA 風格搶車犯罪判定）
fn has_witness_nearby(
    vehicle_pos: Vec3,
    pedestrian_query: &Query<&Transform, (With<Pedestrian>, Without<Player>, Without<Vehicle>)>,
    config: &PlayerConfig,
) -> bool {
    let witness_range_sq = config.interaction.witness_range * config.interaction.witness_range;
    pedestrian_query.iter().any(|ped_transform| {
        ped_transform.translation.distance_squared(vehicle_pos) < witness_range_sq
    })
}

/// 處理上車動畫完成
fn handle_enter_vehicle_complete(
    vehicle_entity: Entity,
    commands: &mut Commands,
    velocity_query: &Query<&Velocity>,
    game_state: &mut GameState,
    vehicle_query: &mut Query<(&Transform, &mut Vehicle), Without<Player>>,
    pedestrian_query: &Query<&Transform, (With<Pedestrian>, Without<Player>, Without<Vehicle>)>,
    crime_events: &mut MessageWriter<CrimeEvent>,
    config: &PlayerConfig,
) {
    if let Ok((vehicle_transform, mut vehicle)) = vehicle_query.get_mut(vehicle_entity) {
        let vehicle_pos = vehicle_transform.translation;
        if has_witness_nearby(vehicle_pos, pedestrian_query, config) {
            crime_events.write(CrimeEvent::VehicleTheft {
                position: vehicle_pos,
            });
        }
        vehicle.is_occupied = true;

        let existing_velocity = velocity_query.get(vehicle_entity).ok();
        apply_vehicle_physics_mode(
            commands,
            vehicle_entity,
            VehiclePhysicsMode::Dynamic,
            vehicle_transform,
            &vehicle,
            existing_velocity,
        );
        commands.entity(vehicle_entity).remove::<NpcVehicle>();
    }
    game_state.player_in_vehicle = true;
    game_state.current_vehicle = Some(vehicle_entity);
}

/// 處理下車動畫完成
fn handle_exit_vehicle_complete(
    vehicle_entity: Entity,
    commands: &mut Commands,
    game_state: &mut GameState,
    vehicle_query: &mut Query<(&Transform, &mut Vehicle), Without<Player>>,
) {
    if let Ok((vehicle_transform, mut vehicle)) = vehicle_query.get_mut(vehicle_entity) {
        vehicle.is_occupied = false;
        vehicle.current_speed = 0.0;
        apply_vehicle_physics_mode(
            commands,
            vehicle_entity,
            VehiclePhysicsMode::Kinematic,
            vehicle_transform,
            &vehicle,
            None,
        );
    }
    game_state.player_in_vehicle = false;
    game_state.current_vehicle = None;
}

// ============================================================================
// 開車時玩家跟著車
// ============================================================================

/// 開車時玩家跟著車、面向車頭：小地圖、GPS、警察、任務都讀玩家的位置。
/// 在車上的整段期間（含下車動畫）關掉玩家的碰撞體，不然留在車中心的膠囊會把車擠開
pub fn player_follow_vehicle_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    transition: Res<VehicleTransitionState>,
    mut player_query: Query<
        (Entity, &mut Transform, Has<ColliderDisabled>),
        (With<Player>, Without<Vehicle>),
    >,
    vehicle_query: Query<&Transform, (With<Vehicle>, Without<Player>)>,
) {
    let Ok((player, mut player_transform, collider_disabled)) = player_query.single_mut() else {
        return;
    };
    let in_vehicle = game_state.player_in_vehicle;
    if in_vehicle && !collider_disabled {
        commands.entity(player).insert(ColliderDisabled);
    } else if !in_vehicle && collider_disabled {
        commands.entity(player).remove::<ColliderDisabled>();
    }

    // 上下車動畫期間位置由動畫決定
    if !in_vehicle || transition.is_animating() {
        return;
    }
    let Some(vehicle_transform) = game_state
        .current_vehicle
        .and_then(|vehicle| vehicle_query.get(vehicle).ok())
    else {
        return;
    };
    player_transform.translation = vehicle_transform.translation;
    player_transform.rotation = Player::rotation_facing(vehicle_transform.forward().as_vec3());
}

/// 在車上但人死了、或車不見了（例如爆炸）：比照下車完成清掉在車上的狀態，玩家重新出現；
/// 上車途中被打死則中止上車。不清的話，重生後會被跟車拉回車上；車不見了則一直隱形、操作全被當成在開車
pub fn leave_vehicle_when_stranded_system(
    mut commands: Commands,
    respawn_state: Res<RespawnState>,
    mut game_state: ResMut<GameState>,
    mut transition: ResMut<VehicleTransitionState>,
    mut vehicle_query: Query<(&Transform, &mut Vehicle), Without<Player>>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<Vehicle>)>,
    mut visibility_query: Query<&mut Visibility, With<Player>>,
    config: Res<PlayerConfig>,
) {
    let ground_y = config.interaction.exit_ground_offset;
    if !game_state.player_in_vehicle {
        // 上車途中被打死：中止上車，不然重生後動畫跑完又把玩家拉進車裡
        if respawn_state.is_dead && transition.is_animating() {
            transition.reset();
            release_player(&mut player_query, &mut visibility_query, ground_y);
        }
        return;
    }
    let vehicle_missing = game_state
        .current_vehicle
        .is_none_or(|vehicle| !vehicle_query.contains(vehicle));
    if !respawn_state.is_dead && !vehicle_missing {
        return;
    }
    match game_state.current_vehicle {
        Some(vehicle) => {
            handle_exit_vehicle_complete(
                vehicle,
                &mut commands,
                &mut game_state,
                &mut vehicle_query,
            );
        }
        None => game_state.player_in_vehicle = false,
    }
    transition.reset();
    release_player(&mut player_query, &mut visibility_query, ground_y);
}

/// 比照下車完成讓玩家出現、回到地面高度：進座位和坐在車上時玩家高度等於車，
/// 停著的機車車心只有 0.4，留在那裡會被游泳偵測當成入水
fn release_player(
    player_query: &mut Query<&mut Transform, (With<Player>, Without<Vehicle>)>,
    visibility_query: &mut Query<&mut Visibility, With<Player>>,
    ground_y: f32,
) {
    if let Ok(mut transform) = player_query.single_mut() {
        transform.translation.y = ground_y;
    }
    set_player_visibility(visibility_query, true);
}

/// 上下車系統依序執行：先處理死亡或車不見，再跑動畫，最後跟車。
/// 跟車排在動畫之後：上車完成那一幀動畫把車切成物理模式，跟車要在同一幀關掉玩家碰撞體，不然車會被擠開
pub(super) fn vehicle_transition_systems() -> ScheduleConfigs<ScheduleSystem> {
    (
        leave_vehicle_when_stranded_system,
        vehicle_transition_animation_system,
        player_follow_vehicle_system,
    )
        .chain()
        .into_configs()
}

#[cfg(test)]
#[path = "vehicle_transition_tests.rs"]
mod tests;
