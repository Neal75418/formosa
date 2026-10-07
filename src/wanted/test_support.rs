//! 通緝系統的真 Rapier 測試場景：地面、走路的玩家、警察、擋牆

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

/// 玩家站在地上的 position（xz），碰撞體開著
pub fn spawn_player_on_foot(app: &mut App, position: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Player::default(),
            Transform::from_translation(position.with_y(0.8)),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.45, 0.25),
        ))
        .id()
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
