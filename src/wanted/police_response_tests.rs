//! 警察對玩家投降、通緝清掉的反應：停火、走過去逮捕、收隊

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_rapier3d::prelude::KinematicCharacterController;

use super::config::OFFICER_WALK_SPEED;
use super::test_support::*;
use super::*;

/// 警察看得到玩家時跑一次警察 AI，回傳 (狀態, 這一幀的水平移動)
fn officer_step(app: &mut App, officer: Entity) -> (PoliceState, Vec3) {
    officer_step_seeing(app, officer, true)
}

/// 指定警察看不看得到玩家，跑一次警察 AI，回傳 (狀態, 這一幀的水平移動)
fn officer_step_seeing(app: &mut App, officer: Entity, can_see: bool) -> (PoliceState, Vec3) {
    settle(app);
    app.world_mut()
        .get_mut::<PoliceOfficer>(officer)
        .expect("警察還在")
        .can_see_player = can_see;
    app.world_mut()
        .run_system_once(police_ai_system)
        .expect("跑得起來");
    let state = app
        .world()
        .get::<PoliceOfficer>(officer)
        .expect("警察還在")
        .state;
    let step = app
        .world()
        .get::<KinematicCharacterController>(officer)
        .expect("有角色控制器")
        .translation
        .unwrap_or(Vec3::ZERO)
        .with_y(0.0);
    (state, step)
}

#[test]
fn police_fire_at_a_wanted_player() {
    // 對照：有通緝、沒投降時會開槍（證明下面的「不開槍」不是場景打不出槍）
    let mut app = police_app(2);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -10.0), PoliceState::Engaging);
    assert!(officer_fires(&mut app, officer));
}

#[test]
fn police_hold_fire_at_a_surrendered_player() {
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    surrender(&mut app, player);
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -10.0), PoliceState::Engaging);
    assert!(!officer_fires(&mut app, officer));
}

#[test]
fn police_walk_up_to_a_surrendered_player() {
    // 交戰或追捕中、在開槍距離內：改成朝玩家走過去（走路速度，一幀 0.05 秒），不進入交戰
    for state in [PoliceState::Engaging, PoliceState::Pursuing] {
        let mut app = police_app(2);
        let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
        surrender(&mut app, player);
        let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -10.0), state);
        let (state_after, step) = officer_step(&mut app, officer);
        assert_eq!(state_after, PoliceState::Pursuing, "原本 {state:?}");
        assert!(
            step.normalize_or_zero().dot(Vec3::Z) > 0.99,
            "原本 {state:?}：這一幀移動 {step}"
        );
        assert!(
            (step.length() - OFFICER_WALK_SPEED * 0.05).abs() < 1e-4,
            "原本 {state:?}：這一幀移動 {step}"
        );
    }
}

#[test]
fn police_keep_walking_until_well_inside_the_arrest_distance() {
    // 1.7 m 已在逮捕距離（2 m）內，但還沒到停下的 1.5 m：繼續走近，不停在逮捕距離邊緣
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    surrender(&mut app, player);
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -1.7), PoliceState::Pursuing);
    let (_, step) = officer_step(&mut app, officer);
    assert!(
        step.normalize_or_zero().dot(Vec3::Z) > 0.99,
        "這一幀移動 {step}"
    );
}

#[test]
fn police_stop_next_to_a_surrendered_player() {
    // 走到逮捕距離內就停下，等逮捕系統接手
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    surrender(&mut app, player);
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -1.0), PoliceState::Pursuing);
    let (state_after, step) = officer_step(&mut app, officer);
    assert_eq!(state_after, PoliceState::Pursuing);
    assert_eq!(step, Vec3::ZERO);
}

#[test]
fn police_search_once_they_lose_sight_of_the_player() {
    // 交戰中看不到玩家（例如躲到牆後）：改成搜索，不留在原地
    let mut app = police_app(2);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -10.0), PoliceState::Engaging);
    // 上一次搜索留下的計時要歸零（通緝清掉時，搜索計時到了才收隊）
    app.world_mut()
        .get_mut::<PoliceOfficer>(officer)
        .expect("警察還在")
        .search_timer = 25.0;
    let (state_after, _) = officer_step_seeing(&mut app, officer, false);
    assert_eq!(state_after, PoliceState::Searching);
    assert_eq!(
        app.world()
            .get::<PoliceOfficer>(officer)
            .expect("警察還在")
            .search_timer,
        0.0
    );
}

#[test]
fn police_keep_engaging_while_they_see_the_player() {
    // 對照：看得到就留在交戰
    let mut app = police_app(2);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -10.0), PoliceState::Engaging);
    let (state_after, _) = officer_step(&mut app, officer);
    assert_eq!(state_after, PoliceState::Engaging);
}

#[test]
fn police_hold_fire_once_the_wanted_level_clears() {
    let mut app = police_app(0);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -10.0), PoliceState::Engaging);
    assert!(!officer_fires(&mut app, officer));
}

#[test]
fn police_stand_down_once_the_wanted_level_clears() {
    // 警戒、追捕、交戰中的警察收隊回巡邏
    for state in [
        PoliceState::Alerted,
        PoliceState::Pursuing,
        PoliceState::Engaging,
    ] {
        let mut app = police_app(0);
        spawn_player_on_foot(&mut app, Vec3::ZERO);
        let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -10.0), state);
        let (state_after, _) = officer_step(&mut app, officer);
        assert_eq!(state_after, PoliceState::Patrolling, "原本 {state:?}");
    }
}

#[test]
fn police_walk_up_and_arrest_a_surrendered_player_without_firing() {
    // 交戰中的警察在 6 m 外，玩家投降：一路走過去開始逮捕；逮捕完成後到逮捕被處理之前
    // （BUSTED 畫面播完才處理，這裡不跑處理系統）也維持投降、警察停火。全程不開槍、逮捕事件只送一次
    // （視線由視線系統負責，這裡每幀直接給「看得到」，只測 AI、開槍、逮捕）
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    surrender(&mut app, player);
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -6.0), PoliceState::Engaging);
    settle(&mut app);
    let mut arrest_events = 0;
    let mut fired = false;
    // 10 秒：走過去約 1.5 秒、逮捕 3 秒，之後還有比 BUSTED 畫面（4.5 秒）長的時間
    for _ in 0..200 {
        app.world_mut()
            .get_mut::<PoliceOfficer>(officer)
            .expect("警察還在")
            .can_see_player = true;
        let world = app.world_mut();
        world.run_system_once(police_ai_system).expect("跑得起來");
        world
            .run_system_once(police_combat_system)
            .expect("跑得起來");
        world
            .run_system_once(police_arrest_system)
            .expect("跑得起來");
        fired |= world
            .get::<PoliceOfficer>(officer)
            .expect("警察還在")
            .attack_cooldown
            > 0.0;
        arrest_events += world
            .resource_mut::<Messages<ArrestEvent>>()
            .drain()
            .count();
        app.update();
    }
    assert_eq!(arrest_events, 1, "逮捕事件數");
    assert!(!fired, "警察開過槍");
    assert!(
        app.world()
            .get::<PlayerSurrenderState>(player)
            .expect("玩家還在")
            .has_surrendered,
        "逮捕還沒處理就放下手"
    );
}
