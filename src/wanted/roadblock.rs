//! 路障系統
//!
//! GTA 風格的警察路障機制，包含：
//! - 路障生成（警車、路障、警察）
//! - 路障破壞物理
//! - 警察站位和射擊

#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::player::Player;
use crate::core::GameState;
use crate::vehicle::Vehicle;
use crate::combat::{DamageEvent, DamageSource, Health, HitReaction};
use crate::ai::AiMovement;

use super::components::*;

// ============================================================================
// 常數
// ============================================================================

/// 路障生成距離（玩家前方）
const ROADBLOCK_SPAWN_DISTANCE: f32 = 80.0;
/// 路障有效距離（玩家靠近後啟動）
const ROADBLOCK_ACTIVE_DISTANCE: f32 = 50.0;
/// 路障消失距離（玩家通過後）
const ROADBLOCK_DESPAWN_DISTANCE: f32 = 100.0;
/// 路障最大數量
const MAX_ROADBLOCKS: usize = 2;
/// 路障生成冷卻（秒）
const ROADBLOCK_SPAWN_COOLDOWN: f32 = 30.0;
/// 路障寬度（公尺）
const ROADBLOCK_WIDTH: f32 = 15.0;
/// 路障警察數量
const ROADBLOCK_POLICE_COUNT: u32 = 4;
/// 路障碰撞傷害
const ROADBLOCK_COLLISION_DAMAGE: f32 = 30.0;
/// 路障血量
const ROADBLOCK_HEALTH: f32 = 500.0;

// ============================================================================
// 組件
// ============================================================================

/// 路障組件
#[derive(Component)]
pub struct Roadblock {
    /// 路障狀態
    pub state: RoadblockState,
    /// 路障中心位置
    pub center: Vec3,
    /// 路障方向（垂直於道路）
    pub direction: Vec3,
    /// 關聯的警察實體
    pub police_officers: Vec<Entity>,
    /// 關聯的路障車輛實體
    pub barrier_vehicles: Vec<Entity>,
    /// 血量（可被撞毀）
    pub health: f32,
    /// 最大血量
    pub max_health: f32,
}

impl Default for Roadblock {
    fn default() -> Self {
        Self {
            state: RoadblockState::Setting,
            center: Vec3::ZERO,
            direction: Vec3::X,
            police_officers: Vec::new(),
            barrier_vehicles: Vec::new(),
            health: ROADBLOCK_HEALTH,
            max_health: ROADBLOCK_HEALTH,
        }
    }
}

/// 路障狀態
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoadblockState {
    /// 設置中
    #[default]
    Setting,
    /// 就位（等待玩家）
    Ready,
    /// 啟動（玩家靠近）
    Active,
    /// 被突破
    Breached,
    /// 撤離
    Retreating,
}

/// 路障護欄組件
#[derive(Component)]
pub struct RoadblockBarrier {
    /// 所屬路障實體
    pub roadblock: Entity,
    /// 血量
    pub health: f32,
}

/// 路障配置資源
#[derive(Resource)]
pub struct RoadblockConfig {
    /// 生成冷卻計時器
    pub spawn_cooldown: f32,
    /// 上次生成時間
    pub last_spawn_time: f32,
    /// 最小通緝等級（3 星以上才有路障）
    pub min_wanted_level: u8,
}

impl Default for RoadblockConfig {
    fn default() -> Self {
        Self {
            spawn_cooldown: ROADBLOCK_SPAWN_COOLDOWN,
            last_spawn_time: -ROADBLOCK_SPAWN_COOLDOWN, // 允許立即生成
            min_wanted_level: 3,
        }
    }
}

/// 路障視覺資源
#[derive(Resource)]
pub struct RoadblockVisuals {
    /// 護欄 mesh
    pub barrier_mesh: Handle<Mesh>,
    /// 護欄材質
    pub barrier_material: Handle<StandardMaterial>,
    /// 警告標誌 mesh
    pub warning_sign_mesh: Handle<Mesh>,
    /// 警告標誌材質
    pub warning_sign_material: Handle<StandardMaterial>,
    /// 路錐 mesh
    pub cone_mesh: Handle<Mesh>,
    /// 路錐材質
    pub cone_material: Handle<StandardMaterial>,
}

impl RoadblockVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            barrier_mesh: meshes.add(Cuboid::new(3.0, 1.0, 0.3)),
            barrier_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0), // 橙色
                ..default()
            }),
            warning_sign_mesh: meshes.add(Cuboid::new(0.8, 0.8, 0.05)),
            warning_sign_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0), // 黃色
                emissive: LinearRgba::new(5.0, 5.0, 0.0, 1.0),
                ..default()
            }),
            cone_mesh: meshes.add(Cone {
                radius: 0.2,
                height: 0.5,
            }),
            cone_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.3, 0.0), // 橙紅色
                ..default()
            }),
        }
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 設置路障視覺資源
pub fn setup_roadblock_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(RoadblockVisuals::new(&mut meshes, &mut materials));
    commands.insert_resource(RoadblockConfig::default());
}

/// 路障生成系統
pub fn spawn_roadblock_system(
    mut commands: Commands,
    wanted: Res<WantedLevel>,
    mut config: ResMut<RoadblockConfig>,
    game_state: Res<GameState>,
    roadblock_query: Query<Entity, With<Roadblock>>,
    player_query: Query<(&Transform, &Velocity), With<Player>>,
    visuals: Option<Res<RoadblockVisuals>>,
    police_visuals: Option<Res<PoliceVisuals>>,
    time: Res<Time>,
) {
    // 3 星以上且玩家在車上才生成路障
    if wanted.stars < config.min_wanted_level || !game_state.player_in_vehicle {
        return;
    }

    let Some(visuals) = visuals else {
        return;
    };

    let Some(police_visuals) = police_visuals else {
        return;
    };

    let Ok((player_transform, player_velocity)) = player_query.single() else {
        return;
    };

    // 檢查是否達到上限
    let current_count = roadblock_query.iter().count();
    if current_count >= MAX_ROADBLOCKS {
        return;
    }

    // 檢查冷卻
    let elapsed = time.elapsed_secs();
    if elapsed - config.last_spawn_time < config.spawn_cooldown {
        return;
    }

    // 玩家需要有一定速度才會觸發路障
    let player_speed = player_velocity.linvel.length();
    if player_speed < 5.0 {
        return;
    }

    config.last_spawn_time = elapsed;

    // 計算路障位置（玩家前方）
    let player_pos = player_transform.translation;
    let player_velocity_vec = player_velocity.linvel;
    let mut spawn_direction = {
        let xz = Vec3::new(player_velocity_vec.x, 0.0, player_velocity_vec.z);
        if xz.length_squared() > 0.01 {
            xz.normalize()
        } else {
            player_transform.forward().as_vec3().normalize_or_zero()
        }
    };
    if spawn_direction.length_squared() < 1e-6 {
        spawn_direction = Vec3::Z;
    }

    let spawn_pos = Vec3::new(
        player_pos.x + spawn_direction.x * ROADBLOCK_SPAWN_DISTANCE,
        0.0,
        player_pos.z + spawn_direction.z * ROADBLOCK_SPAWN_DISTANCE,
    );

    // 路障方向（垂直於玩家行進方向）
    let barrier_direction = Vec3::new(-spawn_direction.z, 0.0, spawn_direction.x);

    // 生成路障
    spawn_roadblock(
        &mut commands,
        spawn_pos,
        barrier_direction,
        &visuals,
        &police_visuals,
        wanted.stars,
    );

    info!(
        "生成路障 at ({:.1}, {:.1}) - 當前: {}/{}",
        spawn_pos.x, spawn_pos.z,
        current_count + 1, MAX_ROADBLOCKS
    );
}

/// 生成單個路障
fn spawn_roadblock(
    commands: &mut Commands,
    position: Vec3,
    direction: Vec3,
    visuals: &RoadblockVisuals,
    police_visuals: &PoliceVisuals,
    wanted_stars: u8,
) {
    let mut police_officers = Vec::new();
    let mut barrier_entities = Vec::new();

    // 創建路障主實體
    let roadblock_entity = commands
        .spawn((
            Name::new("Roadblock"),
            Transform::from_translation(position),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Roadblock {
                center: position,
                direction,
                state: RoadblockState::Setting,
                ..default()
            },
        ))
        .id();

    // 生成護欄（左右各一個）
    for i in [-1.0, 1.0] {
        let barrier_pos = position + direction * (ROADBLOCK_WIDTH * 0.3 * i);

        let barrier = commands
            .spawn((
                Name::new("RoadblockBarrier"),
                Mesh3d(visuals.barrier_mesh.clone()),
                MeshMaterial3d(visuals.barrier_material.clone()),
                Transform::from_translation(barrier_pos + Vec3::Y * 0.5)
                    .with_rotation(Quat::from_rotation_y(direction.x.atan2(direction.z))),
                RoadblockBarrier {
                    roadblock: roadblock_entity,
                    health: 200.0,
                },
                RigidBody::Fixed,
                Collider::cuboid(1.5, 0.5, 0.15),
            ))
            .id();

        barrier_entities.push(barrier);
    }

    // 生成路錐
    for i in -3..=3 {
        let cone_pos = position + direction * (i as f32 * 2.0);
        commands.spawn((
            Mesh3d(visuals.cone_mesh.clone()),
            MeshMaterial3d(visuals.cone_material.clone()),
            Transform::from_translation(cone_pos + Vec3::Y * 0.25),
            RigidBody::Dynamic,
            Collider::cone(0.25, 0.2),
            ColliderMassProperties::Density(50.0),
        ));
    }

    // 生成警察
    let police_count = ROADBLOCK_POLICE_COUNT + (wanted_stars as u32 - 3);
    for i in 0..police_count {
        let offset = direction * ((i as f32 - police_count as f32 / 2.0) * 3.0);
        let police_pos = position - direction.cross(Vec3::Y).normalize() * 5.0 + offset;

        let police = spawn_roadblock_police(commands, police_pos, direction, police_visuals);
        police_officers.push(police);
    }

    // 更新路障實體的關聯
    commands.entity(roadblock_entity).insert(Roadblock {
        center: position,
        direction,
        police_officers,
        barrier_vehicles: barrier_entities,
        state: RoadblockState::Ready,
        health: ROADBLOCK_HEALTH,
        max_health: ROADBLOCK_HEALTH,
    });
}

/// 生成路障警察
fn spawn_roadblock_police(
    commands: &mut Commands,
    position: Vec3,
    facing_direction: Vec3,
    visuals: &PoliceVisuals,
) -> Entity {
    let rotation = Quat::from_rotation_y((-facing_direction.x).atan2(-facing_direction.z));

    let police_entity = commands
        .spawn((
            Name::new("RoadblockPolice"),
            Transform::from_translation(position + Vec3::Y * 0.9).with_rotation(rotation),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .insert((
            PoliceOfficer {
                state: PoliceState::Engaging, // 路障警察直接進入交戰狀態
                officer_type: PoliceType::Patrol,
                target_player: true,
                ..default()
            },
            Health {
                current: 100.0,
                max: 100.0,
                ..default()
            },
            HitReaction::default(),
            AiMovement {
                walk_speed: 3.0,
                run_speed: 5.5,
                ..default()
            },
        ))
        .insert((
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.4, 0.25),
            KinematicCharacterController {
                offset: CharacterLength::Absolute(0.1),
                ..default()
            },
        ))
        .id();

    // 添加視覺模型
    commands.entity(police_entity).with_children(|parent| {
        // 身體
        parent.spawn((
            Mesh3d(visuals.body_mesh.clone()),
            MeshMaterial3d(visuals.uniform_material.clone()),
            Transform::from_translation(Vec3::ZERO),
        ));

        // 頭部
        parent.spawn((
            Mesh3d(visuals.head_mesh.clone()),
            MeshMaterial3d(visuals.skin_material.clone()),
            Transform::from_translation(Vec3::new(0.0, 0.45, 0.0)),
        ));
    });

    police_entity
}

/// 清理路障及其關聯實體
fn cleanup_roadblock(commands: &mut Commands, roadblock: &Roadblock, entity: Entity) {
    for police in &roadblock.police_officers {
        commands.entity(*police).despawn();
    }
    for barrier in &roadblock.barrier_vehicles {
        commands.entity(*barrier).despawn();
    }
    commands.entity(entity).despawn();
}

/// 處理 Active 狀態的路障
fn handle_active_roadblock(roadblock: &mut Roadblock, distance: f32) {
    if roadblock.health <= 0.0 {
        roadblock.state = RoadblockState::Breached;
        info!("路障被突破！");
    } else if distance > ROADBLOCK_DESPAWN_DISTANCE {
        roadblock.state = RoadblockState::Retreating;
    }
}

/// 檢查是否應該切換到撤離狀態
fn should_retreat(distance: f32) -> bool {
    distance > ROADBLOCK_DESPAWN_DISTANCE
}

/// 路障狀態更新系統
pub fn roadblock_update_system(
    mut commands: Commands,
    mut roadblock_query: Query<(Entity, &mut Roadblock, &Transform)>,
    player_query: Query<&Transform, (With<Player>, Without<Roadblock>)>,
    _time: Res<Time>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (entity, mut roadblock, transform) in &mut roadblock_query {
        let distance = (transform.translation - player_pos).length();

        match roadblock.state {
            RoadblockState::Setting => {
                roadblock.state = RoadblockState::Ready;
            }
            RoadblockState::Ready => {
                if distance < ROADBLOCK_ACTIVE_DISTANCE {
                    roadblock.state = RoadblockState::Active;
                    info!("路障啟動！");
                }
            }
            RoadblockState::Active => {
                handle_active_roadblock(&mut roadblock, distance);
            }
            RoadblockState::Breached => {
                if should_retreat(distance) {
                    roadblock.state = RoadblockState::Retreating;
                }
            }
            RoadblockState::Retreating => {
                cleanup_roadblock(&mut commands, &roadblock, entity);
                debug!("路障撤離: {:?}", entity);
            }
        }
    }
}

/// 路障碰撞系統
pub fn roadblock_collision_system(
    mut collision_events: MessageReader<CollisionEvent>,
    mut roadblock_query: Query<&mut Roadblock>,
    barrier_query: Query<&RoadblockBarrier>,
    player_vehicle_query: Query<Entity, (With<Player>, With<Vehicle>)>,
    mut damage_events: MessageWriter<DamageEvent>,
    _time: Res<Time>,
) {
    let Ok(player_vehicle) = player_vehicle_query.single() else {
        return;
    };

    for event in collision_events.read() {
        let CollisionEvent::Started(entity1, entity2, _) = event else {
            continue;
        };

        // 檢查是否是玩家車輛與路障的碰撞
        let (barrier_entity, is_player_collision) = if *entity1 == player_vehicle {
            (*entity2, true)
        } else if *entity2 == player_vehicle {
            (*entity1, true)
        } else {
            continue;
        };

        if !is_player_collision {
            continue;
        }

        // 獲取護欄資料
        let Ok(barrier) = barrier_query.get(barrier_entity) else {
            continue;
        };

        // 獲取路障並扣血
        if let Ok(mut roadblock) = roadblock_query.get_mut(barrier.roadblock) {
            roadblock.health = (roadblock.health - 100.0).max(0.0);

            // 對玩家造成傷害
            damage_events.write(DamageEvent {
                target: player_vehicle,
                amount: ROADBLOCK_COLLISION_DAMAGE,
                source: DamageSource::Explosion,
                attacker: None,
                hit_position: None,
                is_headshot: false,
            });

            info!("撞擊路障！路障血量: {:.0}", roadblock.health);
        }
    }
}

/// 檢查路障是否應該消失
fn should_despawn_roadblock(wanted_stars: u8, distance: f32) -> bool {
    wanted_stars < 3 || distance > ROADBLOCK_DESPAWN_DISTANCE * 1.5
}

/// 路障消失系統
pub fn despawn_roadblock_system(
    mut commands: Commands,
    roadblock_query: Query<(Entity, &Roadblock, &Transform)>,
    player_query: Query<&Transform, (With<Player>, Without<Roadblock>)>,
    wanted: Res<WantedLevel>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (entity, roadblock, transform) in &roadblock_query {
        let distance = (transform.translation - player_pos).length();

        if should_despawn_roadblock(wanted.stars, distance)
            && roadblock.state != RoadblockState::Retreating
        {
            cleanup_roadblock(&mut commands, roadblock, entity);
        }
    }
}
