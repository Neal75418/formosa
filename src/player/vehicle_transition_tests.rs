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
    use crate::core::{COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE};
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
    .init_resource::<crate::combat::RespawnState>()
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

/// 跑遊戲裡的上下車系統組合加死亡重生：玩家坐在車上（隱藏、碰撞體已關），車在 (20, 0.5, 20)；
/// vehicle_alive 為 false 時車已經不見了（例如爆炸）
fn seated_app(vehicle_alive: bool) -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    crate::world::install_map(&mut app);
    app.add_message::<CrimeEvent>()
        .init_resource::<PlayerConfig>()
        .init_resource::<VehicleTransitionState>()
        .init_resource::<crate::combat::RespawnState>()
        .init_resource::<crate::ui::ScreenEffectState>()
        .init_resource::<crate::ui::NotificationQueue>()
        .add_systems(
            Update,
            (
                vehicle_transition_systems(),
                crate::combat::player_respawn_system,
            ),
        );
    let vehicle = app
        .world_mut()
        .spawn((
            Vehicle {
                is_occupied: true,
                ..default()
            },
            Transform::from_xyz(20.0, 0.5, 20.0),
            RigidBody::Dynamic,
        ))
        .id();
    if !vehicle_alive {
        app.world_mut().entity_mut(vehicle).despawn();
    }
    let player = app
        .world_mut()
        .spawn((
            Player::default(),
            Transform::from_xyz(20.0, 0.5, 20.0),
            Visibility::Hidden,
            ColliderDisabled,
            crate::combat::Health::new(100.0),
        ))
        .id();
    app.insert_resource(GameState {
        player_in_vehicle: true,
        current_vehicle: Some(vehicle),
    });
    (app, player, vehicle)
}

/// 回到走路狀態：不在車上、看得見、碰撞體打開
fn assert_on_foot(app: &App, player: Entity) {
    let game_state = app.world().resource::<GameState>();
    assert!(!game_state.player_in_vehicle);
    assert_eq!(game_state.current_vehicle, None);
    assert_eq!(
        app.world().get::<Visibility>(player),
        Some(&Visibility::Visible)
    );
    assert!(!app.world().entity(player).contains::<ColliderDisabled>());
}

#[test]
fn respawning_after_dying_in_the_vehicle_lands_on_foot() {
    // 在車上（正在下車）被打死：動畫中止；重生後要在出生點走路，不能被跟車拉回車上；
    // 車交還、切回停放模式
    let (mut app, player, vehicle) = seated_app(true);
    let seat = Vec3::new(20.0, 0.5, 20.0);
    app.world_mut()
        .resource_mut::<VehicleTransitionState>()
        .start_exit(seat, vehicle, seat + Vec3::X * 2.5, true);
    let mut respawn = app
        .world_mut()
        .resource_mut::<crate::combat::RespawnState>();
    respawn.is_dead = true;
    respawn.respawn_timer = 5.0;
    app.update();
    assert_eq!(
        app.world().resource::<VehicleTransitionState>().phase,
        VehicleTransitionPhase::None
    );
    app.update();
    app.world_mut()
        .resource_mut::<crate::combat::RespawnState>()
        .respawn_timer = -1.0;
    app.update();
    app.update();

    let spawn = crate::combat::respawn_position(app.world().resource::<crate::world::MapLayout>());
    assert_eq!(
        app.world()
            .get::<Transform>(player)
            .expect("玩家還在")
            .translation,
        spawn
    );
    assert_on_foot(&app, player);
    assert!(
        !app.world()
            .get::<Vehicle>(vehicle)
            .expect("車還在")
            .is_occupied
    );
    assert_eq!(
        app.world().get::<RigidBody>(vehicle),
        Some(&RigidBody::KinematicPositionBased)
    );
}

#[test]
fn losing_the_vehicle_while_inside_leaves_it() {
    // 坐著或正在下車時車不見了（例如爆炸），或根本沒記到是哪台車：清掉在車上的狀態，玩家重新出現
    for (exiting, forget_vehicle) in [(false, false), (true, false), (false, true)] {
        let (mut app, player, vehicle) = seated_app(false);
        if exiting {
            app.world_mut()
                .resource_mut::<VehicleTransitionState>()
                .start_exit(Vec3::ZERO, vehicle, Vec3::X, true);
        }
        if forget_vehicle {
            app.world_mut().resource_mut::<GameState>().current_vehicle = None;
        }
        app.update();
        assert_on_foot(&app, player);
        assert_eq!(
            app.world().resource::<VehicleTransitionState>().phase,
            VehicleTransitionPhase::None,
            "exiting={exiting} forget_vehicle={forget_vehicle}"
        );
    }
}

#[test]
fn dying_while_boarding_does_not_finish_boarding_after_respawn() {
    // 上車途中被打死（還在走向車門，或已在進入座位、玩家已隱藏）：上車要中止、玩家出現，
    // 不然重生後動畫跑完又把玩家拉進車裡（時間每幀走 0.1 秒）
    for seating in [false, true] {
        let (mut app, player, vehicle) = seated_app(true);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
        *app.world_mut().resource_mut::<GameState>() = GameState::default();
        app.world_mut()
            .get_mut::<Vehicle>(vehicle)
            .expect("車還在")
            .is_occupied = false;
        let visibility = if seating {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        app.world_mut()
            .entity_mut(player)
            .insert(visibility)
            .remove::<ColliderDisabled>();
        let door = Vec3::new(18.8, 0.5, 20.0);
        let mut transition = app.world_mut().resource_mut::<VehicleTransitionState>();
        transition.start_enter(door - Vec3::X * 2.0, vehicle, door, false);
        if seating {
            transition.phase = VehicleTransitionPhase::EnteringVehicle;
            transition.progress = 0.6;
        }
        let mut respawn = app
            .world_mut()
            .resource_mut::<crate::combat::RespawnState>();
        respawn.is_dead = true;
        respawn.respawn_timer = 0.5;

        app.update();
        assert_eq!(
            app.world().resource::<VehicleTransitionState>().phase,
            VehicleTransitionPhase::None,
            "seating={seating}"
        );
        for _ in 0..30 {
            app.update();
        }

        let spawn =
            crate::combat::respawn_position(app.world().resource::<crate::world::MapLayout>());
        assert_eq!(
            app.world()
                .get::<Transform>(player)
                .expect("玩家還在")
                .translation,
            spawn,
            "seating={seating}"
        );
        assert_on_foot(&app, player);
        assert!(
            !app.world()
                .get::<Vehicle>(vehicle)
                .expect("車還在")
                .is_occupied,
            "seating={seating}"
        );
    }
}

#[test]
fn vehicle_vanishing_while_boarding_shows_the_player_again() {
    // 上車動畫進行到一半（玩家已隱藏）車不見了：動畫中止，玩家要重新出現
    let (mut app, player, vehicle) = seated_app(false);
    *app.world_mut().resource_mut::<GameState>() = GameState::default();
    let mut transition = VehicleTransitionState::default();
    transition.start_enter(Vec3::ZERO, vehicle, Vec3::ZERO, false);
    transition.phase = VehicleTransitionPhase::EnteringVehicle;
    app.insert_resource(transition);
    app.update();
    assert_on_foot(&app, player);
    assert_eq!(
        app.world().resource::<VehicleTransitionState>().phase,
        VehicleTransitionPhase::None
    );
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
