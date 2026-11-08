//! 玩家在車上時，步警看得到、會開槍（射線打到玩家坐的車就算打到玩家；真 Rapier 射線）

use bevy::prelude::*;

use super::test_support::*;
use super::*;

/// 玩家在原點，警察站在北邊（−Z）這個距離
const OFFICER_DISTANCE: f32 = 10.0;

#[test]
fn police_see_a_player_driving_a_car() {
    let mut app = police_app(2);
    spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Car);
    spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Pursuing,
    );
    assert!(police_see_player(&mut app));
}

#[test]
fn police_see_a_player_riding_a_scooter() {
    let mut app = police_app(2);
    spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Scooter);
    spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Pursuing,
    );
    assert!(police_see_player(&mut app));
}

#[test]
fn police_shoot_at_a_player_driving_a_car() {
    let mut app = police_app(2);
    spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Car);
    let officer = spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Engaging,
    );
    assert!(officer_fires(&mut app, officer));
}

/// 打到玩家坐的車才算；中間隔著牆一樣看不到、不開槍
#[test]
fn a_wall_hides_a_driving_player_from_police() {
    let mut app = police_app(2);
    spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Car);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 2.0, -5.0),
        Vec3::new(3.0, 2.0, 0.2),
    );
    let officer = spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Engaging,
    );
    assert!(!police_see_player(&mut app), "隔著牆看得到");
    assert!(!officer_fires(&mut app, officer), "隔著牆開槍");
}

/// 只有玩家坐的那台車算；別台停著的車擋在中間，一樣看不到、不開槍
#[test]
fn another_car_hides_a_driving_player_from_police() {
    let mut app = police_app(2);
    spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Car);
    spawn_parked_vehicle(&mut app, Vec3::new(0.0, 0.0, -4.0), TestVehicle::Car);
    let officer = spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Engaging,
    );
    assert!(!police_see_player(&mut app), "隔著別台車看得到");
    assert!(!officer_fires(&mut app, officer), "隔著別台車開槍");
}

/// 下車動畫要整段播完才清掉記著的車；開完門玩家就往車外走：離座、關門、走離那幾段站到車外、
/// 車的另一側（位置照真動畫量到的：2.0、2.5、2.8 m）時車擋在中間，警察看不到、不開槍
#[test]
fn the_car_hides_a_player_who_stepped_out_on_the_far_side() {
    use crate::player::VehicleTransitionPhase::{ClosingDoorExit, ExitingVehicle, WalkingAway};
    for (phase, x) in [
        (ExitingVehicle, 2.0),
        (ClosingDoorExit, 2.5),
        (WalkingAway, 2.8),
    ] {
        let mut app = police_app(2);
        let (player, _) = spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Car);
        app.world_mut()
            .get_mut::<Transform>(player)
            .expect("有玩家")
            .translation = Vec3::new(x, 0.7, 0.0);
        app.world_mut()
            .resource_mut::<crate::player::VehicleTransitionState>()
            .phase = phase;
        let officer = spawn_officer(&mut app, Vec3::new(-10.0, 0.0, 0.0), PoliceState::Engaging);
        assert!(!police_see_player(&mut app), "{phase:?}：隔著車看得到");
        assert!(!officer_fires(&mut app, officer), "{phase:?}：隔著車開槍");
    }
}

/// 下車開門那段玩家還坐在車上（位置在車中心），照樣看得到
#[test]
fn police_see_a_player_opening_the_door_to_get_out() {
    let mut app = police_app(2);
    spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Car);
    app.world_mut()
        .resource_mut::<crate::player::VehicleTransitionState>()
        .phase = crate::player::VehicleTransitionPhase::OpeningDoorExit;
    spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Pursuing,
    );
    assert!(police_see_player(&mut app));
}
