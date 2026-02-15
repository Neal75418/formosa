//! 車輛進出動畫系統 (GTA 5 風格)
//!
//! 處理上下車動畫的位置插值和狀態轉換

use super::PlayerConfig;
use super::{Player, VehicleTransitionPhase, VehicleTransitionState};
use crate::core::{ease_in_out_cubic, GameState};
use crate::pedestrian::Pedestrian;
use crate::vehicle::{apply_vehicle_physics_mode, NpcVehicle, Vehicle, VehiclePhysicsMode};
use crate::wanted::CrimeEvent;
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
        transition.reset();
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
                let to_vehicle = to_vehicle_delta.normalize();
                let target_rotation = Quat::from_rotation_y((-to_vehicle.x).atan2(-to_vehicle.z));
                player_transform.rotation = player_transform.rotation.slerp(target_rotation, ROTATION_SMOOTHNESS);
            }
        }
        VehicleTransitionPhase::OpeningDoor => {
            // 玩家停在門旁，門正在打開（視覺效果在其他系統處理）
        }
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
        VehicleTransitionPhase::None => {}
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
    pedestrian_query
        .iter()
        .any(|ped_transform| ped_transform.translation.distance_squared(vehicle_pos) < witness_range_sq)
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
