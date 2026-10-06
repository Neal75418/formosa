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

/// 真的跑 Rapier：玩家膠囊站在原點（和遊戲裡同尺寸），汽車在 X 3 m，可加一道牆；
/// 按下 F 跑一次上下車系統，回傳開始走向車門時鎖定的車（上不了是 None）
fn boarding_target(wall: bool) -> Option<Entity> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        TransformPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
    ))
    .init_asset::<Mesh>();
    let player_pos = Vec3::new(0.0, 0.7, 0.0);
    app.world_mut().spawn((
        Player::default(),
        Transform::from_translation(player_pos),
        RigidBody::KinematicPositionBased,
        Collider::capsule_y(0.45, 0.25),
    ));
    let vehicle_pos = Vec3::new(3.0, 0.5, 0.0);
    let vehicle = app
        .world_mut()
        .spawn((
            Vehicle::default(),
            Transform::from_translation(vehicle_pos),
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
    let target = app
        .world()
        .resource::<VehicleTransitionState>()
        .target_vehicle;
    assert!(target.is_none() || target == Some(vehicle), "{target:?}");
    target
}

#[test]
fn boarding_ignores_the_player_itself() {
    // 上車前的路徑檢查從玩家膠囊裡面射出：不能先打到自己就判定被擋
    assert!(boarding_target(false).is_some(), "沒有牆要上得了車");
    // 中間有牆時仍然要擋
    assert_eq!(boarding_target(true), None);
}
