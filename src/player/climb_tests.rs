use super::*;

#[test]
fn test_climb_type_from_height() {
    assert_eq!(ClimbType::from_height(0.2), ClimbType::None);
    assert_eq!(ClimbType::from_height(0.5), ClimbType::Vault);
    assert_eq!(ClimbType::from_height(1.2), ClimbType::Climb);
    assert_eq!(ClimbType::from_height(2.0), ClimbType::HighClimb);
    assert_eq!(ClimbType::from_height(3.0), ClimbType::None);
}

#[test]
fn test_climb_state_phases_vault() {
    let mut state = ClimbState::default();
    state.start(
        ClimbType::Vault,
        Vec3::ZERO,
        Vec3::Y,
        Vec3::new(0.0, 0.0, 1.0),
        0.8,
        Vec3::Z,
    );

    assert_eq!(state.phase, ClimbPhase::Approaching);
    state.advance_phase();
    assert_eq!(state.phase, ClimbPhase::Ascending); // Vault 跳過 GrabbingEdge
    state.advance_phase();
    assert_eq!(state.phase, ClimbPhase::Landing);
    state.advance_phase();
    assert_eq!(state.phase, ClimbPhase::None);
}

#[test]
fn test_climb_state_phases_climb() {
    let mut state = ClimbState::default();
    state.start(
        ClimbType::Climb,
        Vec3::ZERO,
        Vec3::Y * 1.5,
        Vec3::new(0.0, 1.5, 1.0),
        1.5,
        Vec3::Z,
    );

    assert_eq!(state.phase, ClimbPhase::Approaching);
    state.advance_phase();
    assert_eq!(state.phase, ClimbPhase::GrabbingEdge); // Climb 需要抓邊緣
    state.advance_phase();
    assert_eq!(state.phase, ClimbPhase::Ascending);
    state.advance_phase();
    assert_eq!(state.phase, ClimbPhase::Landing);
    state.advance_phase();
    assert_eq!(state.phase, ClimbPhase::None);
}

#[test]
fn test_total_duration() {
    let mut state = ClimbState {
        climb_type: ClimbType::Vault,
        ..ClimbState::default()
    };
    let vault_duration = state.total_duration();
    assert!(vault_duration < 1.0);
    assert!(vault_duration > 0.0);

    state.climb_type = ClimbType::HighClimb;
    let high_climb_duration = state.total_duration();
    assert!(high_climb_duration > vault_duration); // HighClimb 應比 Vault 更長
}

#[test]
fn test_easing_functions() {
    assert!((ease_out_cubic(0.0) - 0.0).abs() < 0.001);
    assert!((ease_out_cubic(1.0) - 1.0).abs() < 0.001);

    assert!((ease_in_out_quad(0.0) - 0.0).abs() < 0.001);
    assert!((ease_in_out_quad(0.5) - 0.5).abs() < 0.001);
    assert!((ease_in_out_quad(1.0) - 1.0).abs() < 0.001);
}

/// 放在 obstacle_dir 方向 1 m 處的東西
#[derive(Clone, Copy)]
enum Obstacle {
    /// 0.8 寬、1.8 高、指定厚度的方塊（0.6 厚像自動販賣機）
    Block { depth: f32 },
    /// 行人大小的角色膠囊（掛角色碰撞群組，頂端約 2.0）
    Person,
    /// 同樣大小、剛倒下還直立的敵人屍體（死亡後換成屍體碰撞群組）
    Corpse,
}

/// 怎麼觸發攀爬
#[derive(Clone, Copy)]
enum Trigger {
    /// 按一下 Space
    Space,
    /// 按住 W 用走路速度（10）前進
    Walk,
    /// 按住 W 衝刺
    Sprint,
    /// 按住 Shift 橫移（衝刺速度 18）、沒按 W
    SprintSideways,
}

struct ClimbSetup {
    facing: Vec3,
    obstacle_dir: Vec3,
    obstacle: Obstacle,
    trigger: Trigger,
    in_vehicle: bool,
    dead: bool,
    switching_character: bool,
}

impl ClimbSetup {
    fn new(facing: Vec3, obstacle_dir: Vec3, obstacle: Obstacle) -> Self {
        Self {
            facing,
            obstacle_dir,
            obstacle,
            trigger: Trigger::Space,
            in_vehicle: false,
            dead: false,
            switching_character: false,
        }
    }
}

/// 在 dir 方向 1 m 處放障礙物
fn spawn_obstacle(app: &mut App, dir: Vec3, obstacle: Obstacle) {
    use bevy_rapier3d::prelude::*;

    match obstacle {
        Obstacle::Block { depth } => {
            app.world_mut().spawn((
                Transform::from_translation(dir * (1.0 + depth / 2.0) + Vec3::Y * 0.9)
                    .with_rotation(super::super::Player::rotation_facing(dir)),
                RigidBody::Fixed,
                Collider::cuboid(0.4, 0.9, depth / 2.0),
            ));
        }
        // 尺寸同行人（pedestrian/systems/lifecycle.rs），碰撞群組用同一個常數
        Obstacle::Person => {
            app.world_mut().spawn((
                Transform::from_translation(dir * 1.25 + Vec3::Y * 1.1),
                RigidBody::KinematicPositionBased,
                Collider::capsule_y(0.65, 0.25),
                crate::core::PEDESTRIAN_COLLISION_GROUPS,
            ));
        }
        // 碰撞群組和死亡的敵人用同一個常數（combat/damage/death.rs）
        Obstacle::Corpse => {
            app.world_mut().spawn((
                Transform::from_translation(dir * 1.25 + Vec3::Y * 1.1),
                RigidBody::Dynamic,
                Collider::capsule_y(0.65, 0.25),
                crate::core::ENEMY_CORPSE_COLLISION_GROUPS,
            ));
        }
    }
}

/// 走公開 API 讓角色切換動畫開始：解鎖小美、按 6 跑輸入系統
fn start_character_switch(app: &mut App) {
    use bevy::ecs::system::RunSystemOnce;

    let mut manager = super::super::CharacterManager::default();
    manager.unlock(super::super::CharacterId::XiaoMei);
    app.insert_resource(manager)
        .init_resource::<crate::core::CameraSettings>()
        .init_resource::<crate::ui::ScreenEffectState>();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Digit6);
    app.world_mut()
        .run_system_once(super::super::character_switch_animation::character_switch_input_system)
        .expect("跑得起來");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    assert!(
        app.world()
            .resource::<super::super::CharacterSwitchAnimation>()
            .is_active(),
        "角色切換動畫沒有開始"
    );
}

/// 真的跑 Rapier 和攀爬偵測、動畫（時間每幀走 0.05 秒）：地面頂在 0.1（同遊戲），
/// 玩家站在原點、面向 facing，障礙物在 obstacle_dir 方向 1 m 處。
/// 觸發一次後跑 3 秒，回傳 (玩家起點, 玩家最後的 Transform, 有沒有開始攀爬)
fn climb_run(setup: ClimbSetup) -> (Vec3, Transform, bool) {
    use bevy::ecs::system::RunSystemOnce;
    use bevy_rapier3d::prelude::*;

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        TransformPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
    ))
    .init_asset::<Mesh>()
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ))
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<super::super::VehicleTransitionState>()
    .init_resource::<super::super::PlayerSkills>()
    .init_resource::<super::super::CharacterSwitchAnimation>()
    .insert_resource(crate::core::GameState {
        player_in_vehicle: setup.in_vehicle,
        current_vehicle: None,
    })
    .insert_resource(crate::combat::RespawnState {
        is_dead: setup.dead,
        respawn_timer: 3.0,
        death_position: Vec3::ZERO,
    });
    app.world_mut().spawn((
        Transform::default(),
        RigidBody::Fixed,
        Collider::cuboid(50.0, 0.1, 50.0),
    ));
    spawn_obstacle(&mut app, setup.obstacle_dir, setup.obstacle);
    let start = Vec3::new(0.0, 0.8, 0.0);
    let player = app
        .world_mut()
        .spawn((
            super::super::Player {
                is_sprinting: matches!(setup.trigger, Trigger::Sprint | Trigger::SprintSideways),
                current_speed: match setup.trigger {
                    Trigger::Space => 0.0,
                    Trigger::Walk => 10.0,
                    Trigger::Sprint | Trigger::SprintSideways => 18.0,
                },
                ..default()
            },
            Transform::from_translation(start)
                .with_rotation(super::super::Player::rotation_facing(setup.facing)),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.45, 0.25),
            ClimbState::default(),
            super::super::DodgeState::default(),
            crate::combat::PlayerCoverState::default(),
        ))
        .id();
    for _ in 0..3 {
        app.update();
    }
    if setup.switching_character {
        start_character_switch(&mut app);
    }

    let key = match setup.trigger {
        Trigger::Space => KeyCode::Space,
        Trigger::Walk | Trigger::Sprint => KeyCode::KeyW,
        Trigger::SprintSideways => KeyCode::KeyA,
    };
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.world_mut()
        .run_system_once(climb_detection_system)
        .expect("跑得起來");
    let started = app.world().get::<ClimbState>(player).unwrap().is_climbing();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.add_systems(Update, climb_animation_system);
    for _ in 0..60 {
        app.update();
    }
    (
        start,
        *app.world().get::<Transform>(player).unwrap(),
        started,
    )
}

fn thin_wall() -> Obstacle {
    Obstacle::Block { depth: 0.6 }
}

#[test]
fn climbs_over_a_thin_wall_in_front() {
    // 面對一面薄牆（像自動販賣機）按 Space：往東、西、北、南都要翻過去，結束時面向攀爬方向
    for dir in [Vec3::X, Vec3::NEG_X, Vec3::NEG_Z, Vec3::Z] {
        let (start, end, started) = climb_run(ClimbSetup::new(dir, dir, thin_wall()));
        assert!(started, "{dir} 沒開始攀爬");
        let travelled = (end.translation - start).dot(dir);
        assert!(travelled > 1.6, "{dir} 只往前 {travelled} m，沒翻過牆");
        let facing = super::super::Player::facing(&end);
        assert!(facing.dot(dir) > 0.99, "{dir} 結束時面向 {facing}");
    }
}

#[test]
fn climbs_onto_a_thick_block_and_stands_on_top() {
    // 爬上一塊 3 m 厚、頂在 1.8 的平台：落點在平台上，身體中心是頂面加站立高度
    let setup = ClimbSetup::new(Vec3::X, Vec3::X, Obstacle::Block { depth: 3.0 });
    let (_, end, started) = climb_run(setup);
    assert!(started);
    assert!(
        (end.translation.y - (1.8 + STANDING_OFFSET)).abs() < 0.05,
        "落點高度 {}",
        end.translation.y
    );
}

#[test]
fn a_wall_behind_does_not_trigger_climbing() {
    // 牆在背後按 Space：不能攀爬
    let (start, end, started) = climb_run(ClimbSetup::new(Vec3::NEG_X, Vec3::X, thin_wall()));
    assert!(!started, "牆在背後也開始攀爬");
    assert_eq!(end.translation, start);
}

#[test]
fn walking_into_a_wall_does_not_climb_but_sprinting_does() {
    // 自動攀爬要衝刺加前進：一般走路（速度 10）撞到牆不爬，衝刺撞上去要爬，
    // 沿著牆前按住 Shift 橫移（速度和衝刺一樣快）、沒按 W 也不爬
    let walk = ClimbSetup {
        trigger: Trigger::Walk,
        ..ClimbSetup::new(Vec3::X, Vec3::X, thin_wall())
    };
    assert!(!climb_run(walk).2, "走路撞到牆就自動攀爬");
    let sprint = ClimbSetup {
        trigger: Trigger::Sprint,
        ..ClimbSetup::new(Vec3::X, Vec3::X, thin_wall())
    };
    assert!(climb_run(sprint).2, "衝刺撞上去沒有自動攀爬");
    let shift_only = ClimbSetup {
        trigger: Trigger::SprintSideways,
        ..ClimbSetup::new(Vec3::X, Vec3::X, thin_wall())
    };
    assert!(!climb_run(shift_only).2, "橫移衝刺沒按 W 就自動攀爬");
}

#[test]
fn does_not_climb_over_people() {
    // 面前是行人按 Space：不能把人當障礙物爬過去
    let (start, end, started) = climb_run(ClimbSetup::new(Vec3::X, Vec3::X, Obstacle::Person));
    assert!(!started, "把行人當成障礙物爬");
    assert_eq!(end.translation, start);
}

#[test]
fn does_not_climb_while_in_a_vehicle() {
    // 開車時按 Space 是手煞車，不偵測攀爬（騎機車時車頭前的障礙物在偵測距離內）
    let setup = ClimbSetup {
        in_vehicle: true,
        ..ClimbSetup::new(Vec3::X, Vec3::X, thin_wall())
    };
    let (start, end, started) = climb_run(setup);
    assert!(!started, "在車上開始攀爬");
    assert_eq!(end.translation, start);
}

#[test]
fn does_not_climb_while_dead() {
    // 死掉時按 Space 不能攀爬
    let setup = ClimbSetup {
        dead: true,
        ..ClimbSetup::new(Vec3::X, Vec3::X, thin_wall())
    };
    let (start, end, started) = climb_run(setup);
    assert!(!started, "死掉了還開始攀爬");
    assert_eq!(end.translation, start);
}

#[test]
fn does_not_climb_over_corpses() {
    // 剛倒下還直立的敵人屍體不是障礙物
    let (start, end, started) = climb_run(ClimbSetup::new(Vec3::X, Vec3::X, Obstacle::Corpse));
    assert!(!started, "把屍體當成障礙物爬");
    assert_eq!(end.translation, start);
}

#[test]
fn does_not_climb_while_switching_characters() {
    // 角色切換動畫期間按 Space 不能攀爬（切換會把玩家傳送走）
    let setup = ClimbSetup {
        switching_character: true,
        ..ClimbSetup::new(Vec3::X, Vec3::X, thin_wall())
    };
    let (start, end, started) = climb_run(setup);
    assert!(!started, "角色切換中開始攀爬");
    assert_eq!(end.translation, start);
}
