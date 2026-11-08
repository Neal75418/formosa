//! 目擊者打電話報警：玩家死掉時掛斷，重生後才不會遲到報警、把剛清掉的通緝加回來

use std::time::Duration;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::witnesses::witness_phone_call_system;
use crate::combat::RespawnState;
use crate::pedestrian::components::{PedState, PedestrianState, WitnessState, WitnessedCrime};
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::wanted::WitnessReport;

/// 一個在 30 m 外、快打完報警電話（進度 0.9、3 秒打完）的目擊者；每次跑系統走 0.5 秒
fn calling_witness_world(player_dead: bool) -> (World, Entity) {
    let mut world = World::new();
    let mut time = Time::<()>::default();
    time.advance_by(Duration::from_millis(500));
    world.insert_resource(time);
    world.insert_resource(RespawnState {
        is_dead: player_dead,
        ..default()
    });
    world.init_resource::<Messages<WitnessReport>>();
    world.spawn((Player::default(), Transform::default()));
    let witness = world
        .spawn((
            Pedestrian,
            Transform::from_xyz(30.0, 0.0, 0.0),
            PedestrianState {
                state: PedState::CallingPolice,
                ..default()
            },
            WitnessState {
                witnessed_crime: true,
                crime_type: Some(WitnessedCrime::Gunshot),
                crime_position: Some(Vec3::ZERO),
                call_progress: 0.9,
                ..default()
            },
        ))
        .id();
    (world, witness)
}

fn reports(world: &mut World) -> usize {
    world
        .resource_mut::<Messages<WitnessReport>>()
        .drain()
        .count()
}

#[test]
fn a_witness_finishes_the_call_while_the_player_is_alive() {
    let (mut world, _) = calling_witness_world(false);
    world
        .run_system_once(witness_phone_call_system)
        .expect("跑得起來");
    assert_eq!(reports(&mut world), 1);
}

#[test]
fn a_witness_hangs_up_when_the_player_dies() {
    let (mut world, witness) = calling_witness_world(true);
    world
        .run_system_once(witness_phone_call_system)
        .expect("跑得起來");
    assert_eq!(reports(&mut world), 0, "報警");
    let state = world.get::<PedestrianState>(witness).expect("行人還在");
    assert_ne!(state.state, PedState::CallingPolice, "還在打電話");
    let call = world.get::<WitnessState>(witness).expect("行人還在");
    assert!(!call.is_calling(), "報警進度還留著：{call:?}");
    assert!(
        !call.has_reported,
        "被當成報過警，以後再目擊也不會報：{call:?}"
    );
}

/// 只有正在打電話的目擊者掛斷；逃跑中的行人照樣逃
#[test]
fn fleeing_pedestrians_keep_fleeing_when_the_player_dies() {
    let (mut world, witness) = calling_witness_world(true);
    world
        .get_mut::<PedestrianState>(witness)
        .expect("行人還在")
        .state = PedState::Fleeing;
    world
        .run_system_once(witness_phone_call_system)
        .expect("跑得起來");
    let state = world.get::<PedestrianState>(witness).expect("行人還在");
    assert_eq!(state.state, PedState::Fleeing);
}
