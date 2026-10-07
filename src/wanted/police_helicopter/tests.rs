//! 直升機對玩家投降、通緝清掉的反應：停火

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

/// 攻擊中的直升機停在玩家（原點）斜上方，機頭朝向玩家
fn spawn_attacking_helicopter(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Transform::from_xyz(0.0, HELICOPTER_HOVER_ALTITUDE, -20.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            PoliceHelicopter {
                state: HelicopterState::Attacking,
                ..default()
            },
        ))
        .id()
}

/// 跑直升機射擊，回傳有沒有開火（開火才會重設射擊冷卻；有沒有打中看射線）
fn helicopter_fires(app: &mut App, helicopter: Entity) -> bool {
    settle(app);
    app.world_mut()
        .run_system_once(helicopter_combat_system)
        .expect("跑得起來");
    app.world()
        .get::<PoliceHelicopter>(helicopter)
        .expect("直升機還在")
        .fire_cooldown
        > 0.0
}

#[test]
fn helicopter_fires_at_a_wanted_player() {
    // 對照：5 星、沒投降會開火
    let mut app = helicopter_app(5);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    let helicopter = spawn_attacking_helicopter(&mut app);
    assert!(helicopter_fires(&mut app, helicopter));
}

#[test]
fn helicopter_holds_fire_at_a_surrendered_player() {
    let mut app = helicopter_app(5);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    surrender(&mut app, player);
    let helicopter = spawn_attacking_helicopter(&mut app);
    assert!(!helicopter_fires(&mut app, helicopter));
}

#[test]
fn helicopter_holds_fire_once_the_wanted_level_clears() {
    let mut app = helicopter_app(0);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    let helicopter = spawn_attacking_helicopter(&mut app);
    assert!(!helicopter_fires(&mut app, helicopter));
}
