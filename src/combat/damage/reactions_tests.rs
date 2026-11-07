//! 擊退：玩家、警察走角色控制器，不能被同一幀的移動系統蓋掉；敵人、行人只走直接推 Transform 那條

use std::time::Duration;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy_rapier3d::prelude::*;

use super::reactions::{
    character_knockback_plugin, enemy_hit_reaction_knockback_system, hit_reaction_knockback_system,
    hit_reaction_update_system, pedestrian_hit_reaction_knockback_system,
};
use crate::combat::{CombatState, Enemy, HitReaction, PlayerCoverState, RespawnState};
use crate::core::{AppState, CameraSettings, GameState};
use crate::pedestrian::Pedestrian;
use crate::player::{
    player_jump, player_movement, CharacterSwitchAnimation, ClimbState, DodgeState, Player,
    PlayerConfig, PlayerSprintState,
};

/// 挨了 40 的傷害、往 +X 擊退中的受傷反應
fn knocked_back() -> HitReaction {
    let mut reaction = HitReaction::default();
    reaction.trigger(40.0, Vec3::X, false);
    reaction
}

/// 真的 Rapier 角色控制器：玩家站在地上沒按鍵（時間每幀走 0.05 秒），回傳 (App, 玩家)。
/// 擊退照遊戲用 character_knockback_plugin 註冊；玩家移動、跳躍刻意排在擊退系統之後
/// （沒按鍵時，玩家移動每幀把角色控制器的位移覆寫成零）。擊退在 PostUpdate 時這條排序不起作用；
/// 擊退被改回 Update 時，它保證移動排在後面、蓋掉擊退
fn standing_player_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        TransformPlugin,
        StatesPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        character_knockback_plugin,
    ))
    .init_asset::<Mesh>()
    .insert_state(AppState::InGame)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )))
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .init_resource::<GameState>()
    .init_resource::<RespawnState>()
    .init_resource::<CombatState>()
    .init_resource::<CameraSettings>()
    .init_resource::<CharacterSwitchAnimation>()
    .init_resource::<PlayerConfig>()
    .add_systems(
        Update,
        (
            hit_reaction_update_system,
            (player_movement, player_jump)
                .chain()
                .after(hit_reaction_knockback_system),
        ),
    );
    app.world_mut().spawn((
        Transform::default(),
        RigidBody::Fixed,
        Collider::cuboid(100.0, 0.1, 100.0),
    ));
    let player = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0.0, 0.85, 0.0),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.45, 0.25),
            KinematicCharacterController {
                slide: true,
                ..default()
            },
            Player::default(),
            HitReaction::default(),
            PlayerCoverState::default(),
            DodgeState::default(),
            ClimbState::default(),
            PlayerSprintState::default(),
        ))
        .id();
    for _ in 0..20 {
        app.update();
    }
    (app, player)
}

fn position(app: &App, entity: Entity) -> Vec3 {
    app.world()
        .get::<Transform>(entity)
        .expect("實體還在")
        .translation
}

/// 站著不動的玩家挨一下擊退要真的被推開：玩家移動每幀覆寫角色控制器的位移，
/// 擊退排在它前面就會被蓋掉、一步都推不動
#[test]
fn a_standing_player_is_knocked_back() {
    let (mut app, player) = standing_player_app();
    let start = position(&app, player);
    *app.world_mut()
        .get_mut::<HitReaction>(player)
        .expect("有受傷反應") = knocked_back();
    for _ in 0..20 {
        app.update();
    }
    let pushed = position(&app, player).x - start.x;
    assert!(pushed > 1.0, "只被推開 {pushed} m");
}

/// 擊退要加在角色控制器的位移上、不能覆寫：玩家在空中挨打，擊退期間仍會往下掉
/// （擊退排在最後，覆寫的話跳躍、重力寫的位移都會被抹掉）
#[test]
fn a_player_knocked_back_in_the_air_keeps_falling() {
    let (mut app, player) = standing_player_app();
    app.world_mut()
        .get_mut::<Transform>(player)
        .expect("有玩家")
        .translation
        .y = 3.0;
    // 傳送後第一幀 Rapier 回報的著地還是傳送前位置的結果（跳躍系統因此把垂直速度歸零）；
    // 多跑兩幀，擊退開始時已經離地、重力在作用
    app.update();
    app.update();
    let start = position(&app, player);
    *app.world_mut()
        .get_mut::<HitReaction>(player)
        .expect("有受傷反應") = knocked_back();
    // 擊退持續 0.5 秒，這 4 幀（0.2 秒）都在擊退中
    for _ in 0..4 {
        app.update();
    }
    let fallen = start.y - position(&app, player).y;
    assert!(fallen > 0.3, "擊退中只往下掉 {fallen} m");
}

/// AI 替身每幀把敵人、行人角色控制器的位移寫成只有重力，擊退照遊戲註冊（時間每幀走 0.05 秒）
fn npc_app() -> App {
    fn ai_writes_gravity_only(
        time: Res<Time>,
        mut query: Query<&mut KinematicCharacterController, Or<(With<Enemy>, With<Pedestrian>)>>,
    ) {
        for mut controller in &mut query {
            controller.translation = Some(Vec3::new(0.0, -9.81 * time.delta_secs(), 0.0));
        }
    }
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        TransformPlugin,
        StatesPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        character_knockback_plugin,
    ))
    .init_asset::<Mesh>()
    .insert_state(AppState::InGame)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )))
    .add_systems(
        Update,
        (
            hit_reaction_update_system,
            ai_writes_gravity_only,
            enemy_hit_reaction_knockback_system,
            pedestrian_hit_reaction_knockback_system,
        ),
    );
    app.world_mut().spawn((
        Transform::default(),
        RigidBody::Fixed,
        Collider::cuboid(100.0, 0.1, 100.0),
    ));
    app
}

/// 敵人、行人站在地上挨一下擊退：要被推開，但只推一次（推 Transform 那條；
/// 角色控制器那條也推的話，距離會變兩倍）
#[test]
fn enemies_and_pedestrians_are_knocked_back_once() {
    let mut app = npc_app();
    let npc = |app: &mut App, z: f32, enemy: bool| {
        let mut entity = app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.85, z),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.45, 0.25),
            KinematicCharacterController::default(),
            HitReaction::default(),
        ));
        if enemy {
            entity.insert(Enemy);
        } else {
            entity.insert(Pedestrian);
        }
        entity.id()
    };
    let enemy = npc(&mut app, 0.0, true);
    let pedestrian = npc(&mut app, 10.0, false);
    for _ in 0..20 {
        app.update();
    }
    let starts = [position(&app, enemy), position(&app, pedestrian)];
    for entity in [enemy, pedestrian] {
        *app.world_mut()
            .get_mut::<HitReaction>(entity)
            .expect("有受傷反應") = knocked_back();
    }
    for _ in 0..20 {
        app.update();
    }
    for (name, entity, start) in [("敵人", enemy, starts[0]), ("行人", pedestrian, starts[1])] {
        let pushed = position(&app, entity).x - start.x;
        assert!(
            (1.0..4.0).contains(&pushed),
            "{name}被推開 {pushed} m（推一次約 2.8 m）"
        );
    }
}

/// 敵人、行人由敵人、行人擊退系統直接推 Transform；角色控制器這條也推的話，會被推兩倍遠
#[test]
fn character_knockback_leaves_enemies_and_pedestrians_to_their_own_systems() {
    let mut world = World::new();
    let mut time = Time::<()>::default();
    time.advance_by(Duration::from_millis(50));
    world.insert_resource(time);
    let player = world
        .spawn((knocked_back(), KinematicCharacterController::default()))
        .id();
    let enemy = world
        .spawn((
            Enemy,
            knocked_back(),
            KinematicCharacterController::default(),
        ))
        .id();
    let pedestrian = world
        .spawn((
            Pedestrian,
            knocked_back(),
            KinematicCharacterController::default(),
        ))
        .id();
    world
        .run_system_once(hit_reaction_knockback_system)
        .expect("跑得起來");
    let translation = |entity: Entity| {
        world
            .get::<KinematicCharacterController>(entity)
            .expect("有角色控制器")
            .translation
    };
    assert!(
        translation(player).is_some_and(|step| step.x > 0.0),
        "玩家要被推：{:?}",
        translation(player)
    );
    assert_eq!(translation(enemy), None, "敵人");
    assert_eq!(translation(pedestrian), None, "行人");
}

/// label 這個排程裡有沒有名為 system 的系統
fn registered_in(app: &App, label: impl ScheduleLabel, system: &str) -> bool {
    let suffix = format!("::{system}");
    app.get_schedule(label).is_some_and(|schedule| {
        schedule
            .graph()
            .systems
            .iter()
            .any(|(_, registered, _)| registered.name().to_string().ends_with(&suffix))
    })
}

/// 遊戲的戰鬥 plugin：角色控制器的擊退排在 PostUpdate（排在 Update 會被移動系統蓋掉）；
/// 敵人、行人推 Transform 的擊退在 Update（他們唯一的擊退）
#[test]
fn the_combat_plugin_registers_every_knockback_system() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .init_state::<AppState>()
        .add_plugins(crate::combat::CombatPlugin);
    let character = "hit_reaction_knockback_system";
    assert!(
        registered_in(&app, PostUpdate, character),
        "PostUpdate 沒有角色控制器的擊退"
    );
    assert!(
        !registered_in(&app, Update, character),
        "Update 也有角色控制器的擊退"
    );
    for system in [
        "enemy_hit_reaction_knockback_system",
        "pedestrian_hit_reaction_knockback_system",
    ] {
        assert!(registered_in(&app, Update, system), "Update 沒有 {system}");
    }
}

/// 擊退要排在 Rapier 套用角色控制器位移之前：排在後面的話要等下一幀才套用，
/// 下一幀的移動系統又會先把它蓋掉
#[test]
fn knockback_runs_before_rapier_moves_the_character() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        TransformPlugin,
        StatesPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        character_knockback_plugin,
    ))
    .init_asset::<Mesh>()
    .insert_state(AppState::InGame);
    let mut schedule = app
        .world_mut()
        .resource_mut::<Schedules>()
        .remove(PostUpdate)
        .expect("有 PostUpdate");
    schedule.initialize(app.world_mut()).expect("排得出來");
    let order: Vec<(bevy::ecs::schedule::SystemKey, String)> = schedule
        .systems()
        .expect("已經初始化")
        .map(|(key, system)| (key, system.name().to_string()))
        .collect();
    let find = |suffix: &str| {
        order
            .iter()
            .position(|(_, name)| name.ends_with(suffix))
            .unwrap_or_else(|| panic!("排程裡沒有 {suffix}"))
    };
    let knockback = find("::hit_reaction_knockback_system");
    let rapier = find("::update_character_controls");
    let unordered = schedule
        .graph()
        .conflicting_systems()
        .iter()
        .any(|(a, b, _)| {
            let pair = [*a, *b];
            pair.contains(&order[knockback].0) && pair.contains(&order[rapier].0)
        });
    assert!(!unordered, "擊退和 Rapier 套用位移之間沒有排序");
    assert!(knockback < rapier, "擊退排在 Rapier 套用位移之後");
}
