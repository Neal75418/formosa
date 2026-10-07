//! 車輛進出動畫系統 (GTA 5 風格)
//!
//! 處理上下車動畫的位置插值和狀態轉換

use super::PlayerConfig;
use super::{Player, VehicleTransitionPhase, VehicleTransitionState};
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

/// 上下車動畫和跟車，跟車排在動畫之後：上車完成那一幀動畫把車切成物理模式，
/// 跟車要在同一幀關掉玩家碰撞體，不然車會被擠開
pub(super) fn vehicle_transition_systems() -> ScheduleConfigs<ScheduleSystem> {
    (
        vehicle_transition_animation_system,
        player_follow_vehicle_system.after(vehicle_transition_animation_system),
    )
        .into_configs()
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// 車停在 (10, 0.5, −4)、車頭轉了一個角度；玩家在原點。
    /// current_vehicle 一律指著這台車，在不在車上只看 player_in_vehicle
    fn run_follow(
        in_vehicle: bool,
        phase: VehicleTransitionPhase,
        collider_disabled: bool,
    ) -> (App, Entity, Transform) {
        let mut app = App::new();
        let vehicle_transform =
            Transform::from_xyz(10.0, 0.5, -4.0).with_rotation(Quat::from_rotation_y(0.5));
        let vehicle = app
            .world_mut()
            .spawn((Vehicle::default(), vehicle_transform))
            .id();
        let player = app
            .world_mut()
            .spawn((Player::default(), Transform::from_xyz(0.0, 0.7, 0.0)))
            .id();
        if collider_disabled {
            app.world_mut().entity_mut(player).insert(ColliderDisabled);
        }
        app.insert_resource(GameState {
            player_in_vehicle: in_vehicle,
            current_vehicle: Some(vehicle),
        });
        app.insert_resource(VehicleTransitionState { phase, ..default() });
        app.world_mut()
            .run_system_once(player_follow_vehicle_system)
            .expect("跑得起來");
        (app, player, vehicle_transform)
    }

    #[test]
    fn driving_moves_the_player_with_the_vehicle() {
        // 開車時玩家跟著車、面向車頭（車頭是 −Z），碰撞體關掉
        let (app, player, vehicle) = run_follow(true, VehicleTransitionPhase::None, false);
        let transform = app.world().get::<Transform>(player).expect("玩家還在");
        assert_eq!(transform.translation, vehicle.translation);
        let ahead = vehicle.forward().as_vec3();
        let ahead = Vec3::new(ahead.x, 0.0, ahead.z).normalize();
        let facing = Player::facing(transform);
        assert!(
            facing.distance(ahead) < 1e-5,
            "facing={facing} ahead={ahead}"
        );
        assert!(app.world().entity(player).contains::<ColliderDisabled>());
    }

    #[test]
    fn exiting_keeps_the_collider_off_and_leaves_the_position_to_the_animation() {
        // 下車動畫的每個階段都還算在車上：位置由動畫決定，碰撞體維持關閉
        for phase in [
            VehicleTransitionPhase::OpeningDoorExit,
            VehicleTransitionPhase::ExitingVehicle,
            VehicleTransitionPhase::ClosingDoorExit,
            VehicleTransitionPhase::WalkingAway,
        ] {
            let (app, player, _) = run_follow(true, phase, true);
            let transform = app.world().get::<Transform>(player).expect("玩家還在");
            assert_eq!(transform.translation, Vec3::new(0.0, 0.7, 0.0), "{phase:?}");
            assert!(
                app.world().entity(player).contains::<ColliderDisabled>(),
                "{phase:?}"
            );
        }
    }

    #[test]
    fn on_foot_the_player_collider_comes_back() {
        // 下車完成、不在車上：碰撞體打開，位置不動
        let (app, player, _) = run_follow(false, VehicleTransitionPhase::None, true);
        let transform = app.world().get::<Transform>(player).expect("玩家還在");
        assert_eq!(transform.translation, Vec3::new(0.0, 0.7, 0.0));
        assert!(!app.world().entity(player).contains::<ColliderDisabled>());
    }

    #[test]
    fn finishing_boarding_does_not_push_the_vehicle() {
        // 真的跑 Rapier 和上下車系統（排序同遊戲）：關門快結束、玩家膠囊在車中心而且碰撞體已建好；
        // 下一幀上車完成、車切成物理模式，碰撞體沒在同一幀關掉的話車會被擠開
        use crate::core::{
            COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE,
        };
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            TransformPlugin,
            RapierPhysicsPlugin::<NoUserData>::default(),
        ))
        .init_asset::<Mesh>()
        .insert_resource(TimestepMode::Fixed {
            dt: 1.0 / 60.0,
            substeps: 1,
        })
        .add_message::<CrimeEvent>()
        .init_resource::<PlayerConfig>()
        .init_resource::<GameState>()
        .init_resource::<VehicleTransitionState>()
        .add_systems(Update, vehicle_transition_systems());
        app.world_mut().spawn((
            Transform::from_xyz(0.0, -0.75, 0.0),
            RigidBody::Fixed,
            Collider::cuboid(50.0, 0.5, 50.0),
            CollisionGroups::new(COLLISION_GROUP_STATIC, Group::ALL),
        ));
        let groups = |own| {
            CollisionGroups::new(
                own,
                COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
            )
        };
        // 汽車和玩家的尺寸、碰撞群組同遊戲（vehicle/spawning.rs、world/characters.rs）
        let start = Vec3::new(0.0, 0.5, 0.0);
        let vehicle = app
            .world_mut()
            .spawn((
                Vehicle::default(),
                Transform::from_translation(start),
                RigidBody::KinematicPositionBased,
                Collider::cuboid(1.0, 0.75, 2.0),
                groups(COLLISION_GROUP_VEHICLE),
            ))
            .id();
        app.world_mut().spawn((
            Player::default(),
            Transform::from_translation(start),
            Visibility::Hidden,
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.45, 0.25),
            KinematicCharacterController::default(),
            groups(COLLISION_GROUP_CHARACTER),
        ));
        for _ in 0..5 {
            app.update();
        }
        app.insert_resource(VehicleTransitionState {
            phase: VehicleTransitionPhase::ClosingDoor,
            progress: 1.0,
            target_vehicle: Some(vehicle),
            ..default()
        });

        for _ in 0..180 {
            app.update();
        }

        assert!(app.world().resource::<GameState>().player_in_vehicle);
        assert_eq!(
            app.world().get::<RigidBody>(vehicle),
            Some(&RigidBody::Dynamic),
            "上車完成後車要是物理模式，才推得動"
        );
        let end = app
            .world()
            .get::<Transform>(vehicle)
            .expect("車還在")
            .translation;
        let drift = Vec3::new(end.x - start.x, 0.0, end.z - start.z).length();
        assert!(drift < 0.01, "車被擠開 {drift} m");
    }

    #[test]
    fn walking_to_the_door_faces_the_vehicle() {
        // 走向車門時角色要轉向車子；停在快走到門邊的地方（時間不前進），一直轉
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<CrimeEvent>()
            .init_resource::<PlayerConfig>()
            .init_resource::<GameState>();
        let player = app
            .world_mut()
            .spawn((Player::default(), Transform::default(), Visibility::Visible))
            .id();
        let vehicle_pos = Vec3::new(3.0, 0.5, 1.0);
        let vehicle = app
            .world_mut()
            .spawn((Vehicle::default(), Transform::from_translation(vehicle_pos)))
            .id();
        let mut transition = VehicleTransitionState::default();
        // 門到車的方向不和世界軸平行，x 或 z 單獨寫反都看得出來
        transition.start_enter(Vec3::ZERO, vehicle, Vec3::new(2.04, 0.0, 0.28), false);
        transition.progress = 0.9;
        app.insert_resource(transition);

        for _ in 0..40 {
            app.world_mut()
                .run_system_once(vehicle_transition_animation_system)
                .expect("跑得起來");
        }

        assert_eq!(
            app.world().resource::<VehicleTransitionState>().phase,
            VehicleTransitionPhase::WalkingToVehicle
        );
        let transform = app.world().get::<Transform>(player).expect("玩家還在");
        let to_vehicle = vehicle_pos - transform.translation;
        let to_vehicle = Vec3::new(to_vehicle.x, 0.0, to_vehicle.z).normalize();
        let facing = Player::facing(transform);
        assert!(
            facing.dot(to_vehicle) > 0.99,
            "facing={facing} to_vehicle={to_vehicle}"
        );
    }
}
