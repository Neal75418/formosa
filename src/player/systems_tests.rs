use bevy::ecs::system::RunSystemOnce;

use super::*;

#[test]
fn facing_is_the_forward_walk_direction() {
    // 往前走時角色轉向移動方向；小地圖箭頭、GPS 讀的 Player::facing 要和這裡一致
    for direction in [Vec3::NEG_Z, Vec3::X, Vec3::new(-0.6, 0.0, 0.8)] {
        let mut transform = Transform::default();
        update_character_rotation(&mut transform, direction, 0.0, false, true, 1.0, 100.0);
        let facing = Player::facing(&transform);
        assert!(
            facing.distance(direction) < 1e-4,
            "direction={direction} facing={facing}"
        );
    }
}

/// 真的跑 Rapier：玩家膠囊（和遊戲裡同尺寸）和汽車放在給的位置，可在 X 1.2 m 加一道牆；
/// 按下 F 跑一次上下車系統，回傳鎖定的車（上不了是 None）和要走去的車門位置
fn press_f(player: Transform, vehicle: Transform, wall: bool) -> (Option<Entity>, Vec3) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        TransformPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
    ))
    .init_asset::<Mesh>();
    app.world_mut().spawn((
        Player::default(),
        player,
        RigidBody::KinematicPositionBased,
        Collider::capsule_y(0.45, 0.25),
    ));
    let vehicle = app
        .world_mut()
        .spawn((
            Vehicle::default(),
            vehicle,
            RigidBody::KinematicPositionBased,
            Collider::cuboid(1.0, 0.75, 2.0),
        ))
        .id();
    if wall {
        app.world_mut().spawn((
            Transform::from_xyz(1.2, 1.0, 0.0),
            RigidBody::Fixed,
            Collider::cuboid(0.1, 2.0, 3.0),
        ));
    }
    for _ in 0..3 {
        app.update();
    }
    app.insert_resource(PlayerConfig::default())
        .insert_resource(GameState::default())
        .insert_resource(VehicleTransitionState::default())
        .insert_resource(InteractionState {
            pressed: true,
            consumed: false,
        });
    app.world_mut()
        .run_system_once(enter_exit_vehicle)
        .expect("跑得起來");
    let transition = app.world().resource::<VehicleTransitionState>();
    let target = transition.target_vehicle;
    assert!(target.is_none() || target == Some(vehicle), "{target:?}");
    (target, transition.target_position)
}

/// 玩家站在原點，汽車在 X 3 m
fn boarding_target(wall: bool) -> Option<Entity> {
    press_f(
        Transform::from_xyz(0.0, 0.7, 0.0),
        Transform::from_xyz(3.0, 0.5, 0.0),
        wall,
    )
    .0
}

#[test]
fn boarding_ignores_the_player_itself() {
    // 上車前的路徑檢查從玩家膠囊裡面射出：不能先打到自己就判定被擋
    assert!(boarding_target(false).is_some(), "沒有牆要上得了車");
    // 中間有牆時仍然要擋
    assert_eq!(boarding_target(true), None);
}

#[test]
fn boarding_walks_to_the_door_on_the_players_side() {
    // 門在車子的左右兩側，玩家在哪一側就走那一側，不能穿過車身走到另一邊
    let player = Transform::from_xyz(0.0, 0.7, 0.0);
    for vehicle in [
        Transform::from_xyz(3.0, 0.5, 0.0),
        Transform::from_xyz(-3.0, 0.5, 0.0),
        // 車子轉 ±90°：車的左右變成世界的南北
        Transform::from_xyz(0.0, 0.5, -3.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        Transform::from_xyz(0.0, 0.5, -3.0)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        // 玩家在車子左前方，不在車的左右軸上
        Transform::from_xyz(2.0, 0.5, 2.5),
    ] {
        let (target, door) = press_f(player, vehicle, false);
        assert!(target.is_some(), "{vehicle:?}");
        let door_side = door - vehicle.translation;
        let player_side = player.translation - vehicle.translation;
        assert!(
            door_side.cross(vehicle.right().as_vec3()).length() < 1e-4,
            "車在 {} 門不在左右軸上 {door}",
            vehicle.translation
        );
        assert!(
            door_side.dot(player_side) > 0.0,
            "車在 {} 門在 {door}",
            vehicle.translation
        );
    }
}

#[test]
fn boarding_at_the_vehicle_center_uses_the_side_the_player_came_from() {
    // 和車子中心重疊、看不出在哪一側：當作是面向車子走過來的，走背後那一側的門
    let vehicle = Transform::from_xyz(3.0, 0.5, 0.0);
    for facing in [Vec3::X, Vec3::NEG_X] {
        let player = vehicle.with_rotation(Player::rotation_facing(facing));
        let (target, door) = press_f(player, vehicle, false);
        assert!(target.is_some(), "facing={facing}");
        let door_side = door - vehicle.translation;
        assert!(door_side.dot(facing) < 0.0, "facing={facing} 門在 {door}");
    }
}
