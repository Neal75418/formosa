//! 投降規則：按住 Y 舉手，離開原地就算放棄（逮捕途中移動算拒捕），逮捕處理完才放下手

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::*;
use crate::combat::WeaponInventory;
use crate::core::GameState;
use crate::economy::PlayerWallet;
use crate::player::Player;

/// 有 2 星通緝、玩家站在 (3, -4) 的場景（時間每幀走 0.05 秒；不站原點，免得和預設的舉手位置重合）
fn surrender_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(50),
        ))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<GameState>()
        .init_resource::<ArrestConfig>()
        .init_resource::<PlayerWallet>()
        .init_resource::<crate::ui::ScreenEffectState>()
        .init_resource::<crate::combat::RespawnState>()
        .insert_resource(WantedLevel {
            stars: 2,
            ..default()
        })
        .add_message::<ArrestEvent>()
        .add_message::<ArrestComplete>();
    let player = app
        .world_mut()
        .spawn((
            Player::default(),
            Transform::from_xyz(3.0, 0.8, -4.0),
            PlayerSurrenderState::default(),
            WeaponInventory::default(),
        ))
        .id();
    app.update();
    (app, player)
}

fn surrender_state(app: &App, player: Entity) -> &PlayerSurrenderState {
    app.world()
        .get::<PlayerSurrenderState>(player)
        .expect("玩家有投降狀態")
}

/// 跑一幀投降輸入
fn surrender_input(app: &mut App) {
    app.update();
    app.world_mut()
        .run_system_once(player_surrender_input_system)
        .expect("跑得起來");
}

/// 按住 Y 直到舉手
fn raise_hands(app: &mut App, player: Entity) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyY);
    for _ in 0..60 {
        surrender_input(app);
        if surrender_state(app, player).has_surrendered {
            break;
        }
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyY);
    assert!(
        surrender_state(app, player).has_surrendered,
        "按住 Y 沒有舉手"
    );
}

fn move_player(app: &mut App, player: Entity, offset: Vec3) {
    app.world_mut()
        .get_mut::<Transform>(player)
        .expect("玩家有 Transform")
        .translation += offset;
}

/// 警察已經在逮捕，進度到 progress
fn arrest_in_progress(app: &mut App, player: Entity, progress: f32) {
    let officer = app.world_mut().spawn_empty().id();
    let mut state = app
        .world_mut()
        .get_mut::<PlayerSurrenderState>(player)
        .expect("玩家有投降狀態");
    state.being_arrested = true;
    state.arresting_officer = Some(officer);
    state.arrest_progress = progress;
}

#[test]
fn small_shuffles_keep_the_hands_up() {
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    move_player(&mut app, player, Vec3::new(0.3, 0.0, 0.0));
    surrender_input(&mut app);
    assert!(surrender_state(&app, player).has_surrendered);
}

#[test]
fn walking_away_calls_off_the_surrender() {
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    move_player(&mut app, player, Vec3::new(0.0, 0.0, 0.6));
    surrender_input(&mut app);
    assert!(!surrender_state(&app, player).has_surrendered);
}

#[test]
fn moving_during_the_arrest_resists_it() {
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    arrest_in_progress(&mut app, player, 0.5);
    move_player(&mut app, player, Vec3::new(0.6, 0.0, 0.0));
    surrender_input(&mut app);
    let state = surrender_state(&app, player);
    assert!(!state.has_surrendered);
    assert!(!state.being_arrested);
    assert_eq!(state.arrest_progress, 0.0);
}

#[test]
fn moving_after_the_arrest_completes_changes_nothing() {
    // 逮捕已完成、等 BUSTED 播完處理：維持投降，警察繼續停火
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    arrest_in_progress(&mut app, player, 1.0);
    move_player(&mut app, player, Vec3::new(0.6, 0.0, 0.0));
    surrender_input(&mut app);
    let state = surrender_state(&app, player);
    assert!(state.has_surrendered);
    assert!(state.being_arrested);
}

#[test]
fn processing_the_arrest_puts_the_hands_down() {
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    arrest_in_progress(&mut app, player, 1.0);
    let officer = surrender_state(&app, player)
        .arresting_officer
        .expect("有逮捕的警察");
    app.world_mut().write_message(ArrestEvent {
        target: player,
        officer,
        arrest_type: ArrestType::PlayerSurrender,
    });
    app.world_mut()
        .run_system_once(handle_arrest_event_system)
        .expect("跑得起來");
    let state = surrender_state(&app, player);
    assert!(!state.has_surrendered);
    assert!(!state.being_arrested);
    assert_eq!(state.arrest_progress, 0.0);
    assert_eq!(app.world().resource::<WantedLevel>().stars, 0);
}

#[test]
fn an_arrest_without_an_officer_puts_the_hands_down() {
    // 逮捕中卻沒有逮捕的警察（不該發生）：沒有逮捕事件可處理，直接放下手，不會一直舉著
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    arrest_in_progress(&mut app, player, 0.99);
    app.world_mut()
        .get_mut::<PlayerSurrenderState>(player)
        .expect("玩家有投降狀態")
        .arresting_officer = None;
    app.update();
    app.world_mut()
        .run_system_once(police_arrest_system)
        .expect("跑得起來");
    assert!(!surrender_state(&app, player).has_surrendered);
}

#[test]
fn dying_puts_the_hands_down() {
    // 逮捕途中死掉：WASTED 播著時不會觸發 BUSTED，逮捕不會被處理，所以死了就放下手、逮捕作廢
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    arrest_in_progress(&mut app, player, 0.9);
    app.world_mut()
        .resource_mut::<crate::combat::RespawnState>()
        .is_dead = true;
    app.update();
    app.world_mut()
        .run_system_once(police_arrest_system)
        .expect("跑得起來");
    let state = surrender_state(&app, player);
    assert!(!state.has_surrendered);
    assert!(!state.being_arrested);
}

#[test]
fn dying_right_as_the_arrest_completes_puts_the_hands_down() {
    // 逮捕完成那一幀死掉、WASTED 先觸發：逮捕事件被跳過、不會再處理，一樣要放下手
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    arrest_in_progress(&mut app, player, 1.0);
    app.world_mut()
        .resource_mut::<crate::combat::RespawnState>()
        .is_dead = true;
    app.update();
    app.world_mut()
        .run_system_once(police_arrest_system)
        .expect("跑得起來");
    assert!(!surrender_state(&app, player).has_surrendered);
}
