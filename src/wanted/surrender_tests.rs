//! 投降規則：按住 Y 舉手，自己離開原地就算放棄（逮捕途中移動算拒捕；被打退不算），逮捕處理完才放下手

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
        .init_resource::<crate::player::CharacterSwitchAnimation>()
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
            crate::combat::HitReaction::default(),
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

#[test]
fn raising_hands_stands_the_player_up() {
    // 蹲著舉手投降會站起來：警察瞄蹲著的人的身體中心，隔著齊腰欄杆會擋住逮捕
    let (mut app, player) = surrender_app();
    app.world_mut()
        .get_mut::<Player>(player)
        .expect("有玩家")
        .is_crouching = true;
    raise_hands(&mut app, player);
    assert!(
        !app.world()
            .get::<Player>(player)
            .expect("有玩家")
            .is_crouching
    );
}

/// 按一下 Ctrl 跑一次玩家輸入，回傳玩家是不是蹲著
fn press_ctrl(app: &mut App, player: Entity) -> bool {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .run_system_once(crate::player::player_input)
        .expect("跑得起來");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ControlLeft);
    app.world()
        .get::<Player>(player)
        .expect("有玩家")
        .is_crouching
}

#[test]
fn ctrl_crouches_normally() {
    // 對照：沒投降時按 Ctrl 會蹲下
    let (mut app, player) = surrender_app();
    assert!(press_ctrl(&mut app, player));
}

#[test]
fn ctrl_does_not_crouch_while_hands_are_up() {
    // 舉手期間不能蹲下：蹲著時警察瞄身體中心，隔著齊腰欄杆會擋住逮捕
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    assert!(!press_ctrl(&mut app, player));
}

/// 玩家被打中、正在被推開（傷害 50 觸發擊退，往 +X 推）
fn knock_back(app: &mut App, player: Entity) {
    let mut reaction = crate::combat::HitReaction::default();
    reaction.trigger(50.0, Vec3::X, false);
    app.world_mut().entity_mut(player).insert(reaction);
}

#[test]
fn being_knocked_back_keeps_the_hands_up() {
    // 被打退不是自己走開：推開 2 m 還是維持投降
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    knock_back(&mut app, player);
    move_player(&mut app, player, Vec3::new(2.0, 0.0, 0.0));
    surrender_input(&mut app);
    assert!(surrender_state(&app, player).has_surrendered);
}

#[test]
fn the_hands_up_spot_follows_a_knockback() {
    // 被推開後，舉手位置跟到新的地方：推完站穩仍維持投降，從新位置再自己走開 0.6 m 就算放棄
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    knock_back(&mut app, player);
    move_player(&mut app, player, Vec3::new(2.0, 0.0, 0.0));
    surrender_input(&mut app);
    end_knockback(&mut app, player);
    for _ in 0..3 {
        surrender_input(&mut app);
    }
    assert!(surrender_state(&app, player).has_surrendered, "推完站穩");
    move_player(&mut app, player, Vec3::new(0.0, 0.0, 0.6));
    surrender_input(&mut app);
    assert!(!surrender_state(&app, player).has_surrendered, "推完再走開");
}

/// 推開結束（受傷反應回到沒反應）
fn end_knockback(app: &mut App, player: Entity) {
    app.world_mut()
        .entity_mut(player)
        .insert(crate::combat::HitReaction::default());
}

/// 玩家在不在地上（角色控制器的輸出）
fn set_grounded(app: &mut App, player: Entity, grounded: bool) {
    app.world_mut().entity_mut(player).insert(
        bevy_rapier3d::prelude::KinematicCharacterControllerOutput {
            grounded,
            ..default()
        },
    );
}

#[test]
fn the_last_knockback_frame_seen_late_keeps_the_hands_up() {
    // 推開的最後一段位移最晚要隔兩幀才看得到（低幀率時一幀推得遠）：推完第二幀才出現的 0.6 m 也不算自己走開
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    knock_back(&mut app, player);
    move_player(&mut app, player, Vec3::new(2.0, 0.0, 0.0));
    surrender_input(&mut app);
    end_knockback(&mut app, player);
    surrender_input(&mut app);
    move_player(&mut app, player, Vec3::new(0.6, 0.0, 0.0));
    surrender_input(&mut app);
    assert!(surrender_state(&app, player).has_surrendered);
}

#[test]
fn falling_after_a_knockback_keeps_the_hands_up() {
    // 被推下高處：推完還在空中往下掉，落地前的位移都不算；落地站穩後再走開才算放棄
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    knock_back(&mut app, player);
    move_player(&mut app, player, Vec3::new(1.0, 0.0, 0.0));
    surrender_input(&mut app);
    end_knockback(&mut app, player);
    set_grounded(&mut app, player, false);
    for _ in 0..10 {
        move_player(&mut app, player, Vec3::new(0.05, -0.1, 0.0));
        surrender_input(&mut app);
    }
    assert!(surrender_state(&app, player).has_surrendered, "掉落中");
    set_grounded(&mut app, player, true);
    for _ in 0..3 {
        surrender_input(&mut app);
    }
    assert!(surrender_state(&app, player).has_surrendered, "落地站穩");
    move_player(&mut app, player, Vec3::new(0.0, 0.0, 0.6));
    surrender_input(&mut app);
    assert!(!surrender_state(&app, player).has_surrendered, "落地後走開");
}

#[test]
fn a_flinch_does_not_move_the_hands_up_spot() {
    // 輕傷畏縮（10～24 傷害）不會把人推開，也不算被推：舉手後走開 0.6 m 照樣算放棄
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    let mut reaction = crate::combat::HitReaction::default();
    reaction.trigger(15.0, Vec3::X, false);
    app.world_mut().entity_mut(player).insert(reaction);
    move_player(&mut app, player, Vec3::new(0.6, 0.0, 0.0));
    surrender_input(&mut app);
    assert!(!surrender_state(&app, player).has_surrendered);
}

#[test]
fn jumping_during_a_knockback_calls_off_the_surrender() {
    // 挨打後自己跳起來不算被推：跳躍的位移照樣算自己走開（不然離地期間一直跟著移動，能舉著手跳很遠）
    let (mut app, player) = surrender_app();
    raise_hands(&mut app, player);
    knock_back(&mut app, player);
    set_grounded(&mut app, player, false);
    app.world_mut()
        .get_mut::<Player>(player)
        .expect("有玩家")
        .vertical_velocity = 5.0;
    move_player(&mut app, player, Vec3::new(0.0, 0.6, 0.0));
    surrender_input(&mut app);
    assert!(!surrender_state(&app, player).has_surrendered);
}

/// 真的 Rapier 角色控制器：玩家站在地上，系統照「玩家移動 → 受傷反應 → 投降輸入」的順序跑，
/// 擊退照遊戲排在 PostUpdate（時間每幀走 0.05 秒），回傳 (App, 玩家)
fn physics_app() -> (App, Entity) {
    use bevy_rapier3d::prelude::*;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        TransformPlugin,
        bevy::state::app::StatesPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
        crate::combat::character_knockback_plugin,
    ))
    .init_asset::<Mesh>()
    .insert_state(crate::core::AppState::InGame)
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ))
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .init_resource::<GameState>()
    .init_resource::<ArrestConfig>()
    .init_resource::<crate::combat::RespawnState>()
    .init_resource::<crate::combat::CombatState>()
    .init_resource::<crate::core::CameraSettings>()
    .init_resource::<crate::player::CharacterSwitchAnimation>()
    .init_resource::<crate::player::PlayerConfig>()
    .insert_resource(WantedLevel {
        stars: 2,
        ..default()
    })
    .add_systems(
        Update,
        (
            (
                crate::player::player_input,
                crate::player::player_movement,
                crate::player::player_jump,
            )
                .chain(),
            crate::combat::hit_reaction_update_system,
            player_surrender_input_system,
        )
            .chain(),
    );
    app.world_mut().spawn((
        Transform::default(),
        RigidBody::Fixed,
        Collider::cuboid(100.0, 0.1, 100.0),
    ));
    let player = app
        .world_mut()
        .spawn((
            Transform::from_xyz(3.0, 0.85, -4.0),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.45, 0.25),
            KinematicCharacterController {
                slide: true,
                ..default()
            },
            Player::default(),
            crate::combat::HitReaction::default(),
            PlayerSurrenderState::default(),
            crate::combat::PlayerCoverState::default(),
            crate::player::DodgeState::default(),
            crate::player::ClimbState::default(),
            crate::player::PlayerSprintState::default(),
        ))
        .id();
    for _ in 0..20 {
        app.update();
    }
    (app, player)
}

/// 玩家在目前位置舉手、挨一下 damage 的傷害（往 +X 推）
fn surrender_and_get_hit(app: &mut App, player: Entity, damage: f32) {
    let position = app
        .world()
        .get::<Transform>(player)
        .expect("有玩家")
        .translation;
    let mut state = app
        .world_mut()
        .get_mut::<PlayerSurrenderState>(player)
        .expect("有投降狀態");
    state.has_surrendered = true;
    state.surrender_position = position;
    app.world_mut()
        .get_mut::<crate::combat::HitReaction>(player)
        .expect("有受傷反應")
        .trigger(damage, Vec3::X, false);
}

#[test]
fn a_real_knockback_settles_then_walking_away_counts() {
    // 真的角色控制器：挨 25 傷害被推開仍舉著手；挨打一秒後倒數歸零（站著不動時要回報著地），
    // 之後按 W 走開就算放棄
    let (mut app, player) = physics_app();
    surrender_and_get_hit(&mut app, player, 25.0);
    for _ in 0..20 {
        app.update();
    }
    let state = surrender_state(&app, player);
    let pushed = app
        .world()
        .get::<Transform>(player)
        .expect("有玩家")
        .translation
        .x
        - 3.0;
    assert!(pushed > 0.5, "真的被推開（推了 {pushed} m）");
    assert!(state.has_surrendered, "被推開");
    assert_eq!(state.knockback_settle_frames, 0, "站穩後倒數歸零");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    for _ in 0..10 {
        app.update();
    }
    assert!(!surrender_state(&app, player).has_surrendered, "走開");
}

#[test]
fn jumping_during_a_real_knockback_calls_off_the_surrender() {
    // 挨 40 傷害（擊退 0.5 秒）期間自己跳起來：跳躍不算被推，舉著手跳就算放棄
    let (mut app, player) = physics_app();
    surrender_and_get_hit(&mut app, player, 40.0);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Space);
    app.update();
    let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keyboard.release(KeyCode::Space);
    keyboard.clear();
    for _ in 0..20 {
        app.update();
    }
    assert!(!surrender_state(&app, player).has_surrendered);
}
