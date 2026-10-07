//! 直升機的射線瞄準玩家、對玩家投降與通緝清掉停火（真 Rapier 射線）

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::components::*;
use super::*;
use crate::combat::{CombatVisuals, DamageEvent};
use crate::wanted::test_support::*;
use crate::wanted::WantedLevel;

fn helicopter_app(stars: u8) -> App {
    let mut app = rapier_app();
    let mut meshes = Assets::<Mesh>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    app.insert_resource(CombatVisuals::new(&mut meshes, &mut materials))
        .insert_resource(WantedLevel { stars, ..default() })
        .add_message::<DamageEvent>();
    app
}

/// 懸停高度、在玩家（原點）北邊 distance 公尺
fn hovering_north(distance: f32) -> Vec3 {
    Vec3::new(0.0, HELICOPTER_HOVER_ALTITUDE, -distance)
}

/// 攻擊中的直升機停在 at，機頭朝 nose_toward（同遊戲只轉水平方向）
fn spawn_attacking_helicopter(app: &mut App, at: Vec3, nose_toward: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Transform::from_translation(at).looking_at(nose_toward.with_y(at.y), Vec3::Y),
            PoliceHelicopter {
                state: HelicopterState::Attacking,
                search_timer: 1.0,
                last_hit_time: f32::NEG_INFINITY,
                ..default()
            },
        ))
        .id()
}

/// 跑直升機 AI，回傳直升機有沒有看到玩家（看到才會把搜索計時歸零）
fn helicopter_sees_player(app: &mut App, helicopter: Entity) -> bool {
    settle(app);
    app.world_mut()
        .run_system_once(helicopter_ai_system)
        .expect("跑得起來");
    app.world()
        .get::<PoliceHelicopter>(helicopter)
        .expect("直升機還在")
        .search_timer
        == 0.0
}

/// 跑直升機射擊，回傳 (有沒有開火, 打到玩家的傷害事件數)；開火才會重設射擊冷卻
fn helicopter_shoots(app: &mut App, helicopter: Entity, player: Entity) -> (bool, usize) {
    settle(app);
    app.world_mut()
        .run_system_once(helicopter_combat_system)
        .expect("跑得起來");
    let fired = app
        .world()
        .get::<PoliceHelicopter>(helicopter)
        .expect("直升機還在")
        .fire_cooldown
        > 0.0;
    let hits = app
        .world_mut()
        .resource_mut::<Messages<DamageEvent>>()
        .drain()
        .filter(|event| event.target == player)
        .count();
    (fired, hits)
}

#[test]
fn helicopter_hits_the_player_in_the_open() {
    for distance in [10.0, 20.0, 30.0] {
        let mut app = helicopter_app(5);
        let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
        let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(distance), Vec3::ZERO);
        assert_eq!(
            helicopter_shoots(&mut app, helicopter, player),
            (true, 1),
            "北邊 {distance} m"
        );
    }
}

#[test]
fn helicopter_hits_the_player_with_its_nose_turned_away() {
    // 機頭背對玩家時槍口比機身中心離玩家更遠：射線長度要從槍口算，從機身中心算會不夠長
    let mut app = helicopter_app(5);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    let at = hovering_north(30.0);
    let helicopter = spawn_attacking_helicopter(&mut app, at, at * 2.0);
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (true, 1));
}

#[test]
fn a_roof_hides_the_player_from_the_helicopter() {
    let mut app = helicopter_app(5);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 6.0, 0.0),
        Vec3::new(30.0, 0.2, 30.0),
    );
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert!(!helicopter_sees_player(&mut app, helicopter));
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (true, 0));
}

#[test]
fn helicopter_holds_fire_at_a_surrendered_player() {
    let mut app = helicopter_app(5);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    surrender(&mut app, player);
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (false, 0));
}

#[test]
fn helicopter_holds_fire_once_the_wanted_level_clears() {
    let mut app = helicopter_app(0);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (false, 0));
}
