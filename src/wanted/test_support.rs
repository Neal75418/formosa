//! 通緝系統的真 Rapier 測試場景：地面、走路或坐在車上的玩家、警察、擋牆、停著的車

use std::time::Duration;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_rapier3d::prelude::*;

use super::config::{
    OFFICER_CAPSULE_HALF_HEIGHT, OFFICER_CAPSULE_RADIUS, OFFICER_CONTROLLER_OFFSET,
    OFFICER_RUN_SPEED, OFFICER_SPAWN_HEIGHT, OFFICER_WALK_SPEED,
};
use super::*;
use crate::ai::AiMovement;
use crate::combat::DamageEvent;
use crate::core::{GameState, PLAYER_COLLISION_GROUPS, VEHICLE_COLLISION_GROUPS};
use crate::player::Player;

/// 跑得動 Rapier 射線的最小 App（時間每幀走 0.05 秒），地面頂在 0.1（同遊戲）
pub fn rapier_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        TransformPlugin,
        RapierPhysicsPlugin::<NoUserData>::default(),
    ))
    .init_asset::<Mesh>()
    .init_resource::<GameState>()
    .init_resource::<crate::player::VehicleTransitionState>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )));
    app.world_mut().spawn((
        Transform::default(),
        RigidBody::Fixed,
        Collider::cuboid(100.0, 0.1, 100.0),
    ));
    app
}

/// 有通緝（stars 星）的場景，跑得動警察的視線、AI、開槍、逮捕系統
pub fn police_app(stars: u8) -> App {
    let mut app = rapier_app();
    app.init_resource::<PoliceConfig>()
        .init_resource::<crate::combat::RespawnState>()
        .insert_resource(crate::core::PoliceSpatialHash::new())
        .insert_resource(WantedLevel { stars, ..default() })
        .add_message::<WantedLevelChanged>()
        .add_message::<DamageEvent>()
        .add_message::<ArrestEvent>();
    app
}

/// 玩家站在地上的 position（xz），碰撞體和碰撞群組同遊戲
pub fn spawn_player_on_foot(app: &mut App, position: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Player::default(),
            Transform::from_translation(position.with_y(0.8)),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.45, 0.25),
            PLAYER_COLLISION_GROUPS,
        ))
        .id()
}

/// 測試用的車種：碰撞體尺寸和中心高度同遊戲（汽車、機車都是一整塊方盒）
#[derive(Clone, Copy)]
pub enum TestVehicle {
    Car,
    Scooter,
}

impl TestVehicle {
    fn half_extents(self) -> Vec3 {
        match self {
            Self::Car => Vec3::new(1.0, 0.75, 2.0),
            Self::Scooter => Vec3::new(0.25, 0.5, 0.8),
        }
    }

    fn center_height(self) -> f32 {
        match self {
            Self::Car => 0.5,
            Self::Scooter => 0.4,
        }
    }
}

/// 停在 position（xz）的車，車身沿 Z 軸
pub fn spawn_parked_vehicle(app: &mut App, position: Vec3, kind: TestVehicle) -> Entity {
    let half = kind.half_extents();
    app.world_mut()
        .spawn((
            Transform::from_translation(position.with_y(kind.center_height())),
            RigidBody::KinematicPositionBased,
            Collider::cuboid(half.x, half.y, half.z),
            VEHICLE_COLLISION_GROUPS,
        ))
        .id()
}

/// 玩家坐在停在 position（xz）的車上，回傳 (玩家, 車)：同遊戲的跟車，玩家在車中心、碰撞體關掉，
/// GameState 記著這台車
pub fn spawn_player_in_vehicle(
    app: &mut App,
    position: Vec3,
    kind: TestVehicle,
) -> (Entity, Entity) {
    let vehicle = spawn_parked_vehicle(app, position, kind);
    let player = app
        .world_mut()
        .spawn((
            Player::default(),
            Transform::from_translation(position.with_y(kind.center_height())),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.45, 0.25),
            PLAYER_COLLISION_GROUPS,
            ColliderDisabled,
        ))
        .id();
    let mut game_state = app.world_mut().resource_mut::<GameState>();
    game_state.player_in_vehicle = true;
    game_state.current_vehicle = Some(vehicle);
    (player, vehicle)
}

/// 跑通緝消退系統（裡面判斷警察看不看得到玩家），回傳有沒有被看到
pub fn police_see_player(app: &mut App) -> bool {
    settle(app);
    app.world_mut()
        .run_system_once(update_police_spatial_hash_system)
        .expect("跑得起來");
    app.world_mut()
        .run_system_once(wanted_cooldown_system)
        .expect("跑得起來");
    app.world().resource::<WantedLevel>().player_visible
}

/// 警察站在 at（xz）、面向玩家，生成高度和元件比照遊戲生成的步警
pub fn spawn_officer(app: &mut App, at: Vec3, state: PoliceState) -> Entity {
    let player_pos = app
        .world_mut()
        .query_filtered::<&Transform, With<Player>>()
        .single(app.world())
        .expect("有玩家")
        .translation;
    let position = at.with_y(OFFICER_SPAWN_HEIGHT);
    app.world_mut()
        .spawn((
            Transform::from_translation(position)
                .looking_at(player_pos.with_y(position.y), Vec3::Y),
            PoliceOfficer { state, ..default() },
            AiMovement {
                walk_speed: OFFICER_WALK_SPEED,
                run_speed: OFFICER_RUN_SPEED,
                ..default()
            },
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(OFFICER_CAPSULE_HALF_HEIGHT, OFFICER_CAPSULE_RADIUS),
            KinematicCharacterController {
                offset: CharacterLength::Absolute(OFFICER_CONTROLLER_OFFSET),
                ..default()
            },
        ))
        .id()
}

/// 玩家在原地舉手投降
pub fn surrender(app: &mut App, player: Entity) {
    let position = app
        .world()
        .get::<Transform>(player)
        .expect("玩家有 Transform")
        .translation;
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerSurrenderState {
            has_surrendered: true,
            surrender_position: position,
            ..default()
        });
}

/// 固定不動的方塊（牆、圍籬、頂棚）
pub fn spawn_block(app: &mut App, center: Vec3, half_extents: Vec3) {
    app.world_mut().spawn((
        Transform::from_translation(center),
        RigidBody::Fixed,
        Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
    ));
}

/// 跑警察開槍系統，回傳警察有沒有開槍（開槍才會重設攻擊冷卻；有沒有命中是隨機的）
pub fn officer_fires(app: &mut App, officer: Entity) -> bool {
    settle(app);
    app.world_mut()
        .run_system_once(police_combat_system)
        .expect("跑得起來");
    app.world()
        .get::<PoliceOfficer>(officer)
        .expect("警察還在")
        .attack_cooldown
        > 0.0
}

/// 讓 Rapier 把新生的碰撞體放進射線查詢
pub fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}
