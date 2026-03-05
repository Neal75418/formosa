//! 通緝系統函數實作
//!
//! 子模組：
//! - `police_ai` - 警察 AI 狀態機
//! - `police_combat` - 警察戰鬥與無線電通訊
//! - `wanted_hud` - 通緝等級 HUD 顯示

mod police_ai;
mod police_combat;
mod wanted_hud;

use crate::core::rapier_real_to_f32;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};

use crate::ai::AiMovement;
use crate::combat::{Health, HitReaction};
use crate::core::PoliceSpatialHash;
use crate::player::Player;

#[allow(clippy::wildcard_imports)]
use super::components::*;
#[allow(clippy::wildcard_imports)]
use super::config::*;
#[allow(clippy::wildcard_imports)]
use super::events::*;

pub use police_ai::police_ai_system;
pub use police_combat::{police_combat_system, police_radio_call_system};
pub use wanted_hud::{update_wanted_hud, wanted_level_change_animation};

/// 設置警察視覺資源
pub fn setup_police_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body_mesh = meshes.add(Cuboid::new(0.4, 0.6, 0.25));
    let head_mesh = meshes.add(Sphere::new(0.15));
    let arm_mesh = meshes.add(Cuboid::new(0.1, 0.4, 0.1));
    let leg_mesh = meshes.add(Cuboid::new(0.12, 0.45, 0.12));

    let uniform_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.15, 0.3),
        ..default()
    });

    let skin_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.87, 0.72, 0.53),
        ..default()
    });

    let badge_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.65, 0.13),
        metallic: 0.9,
        ..default()
    });

    commands.insert_resource(PoliceVisuals {
        body_mesh,
        head_mesh,
        arm_mesh,
        leg_mesh,
        uniform_material,
        skin_material,
        badge_material,
    });
}

// ============================================================================
// 空間哈希系統
// ============================================================================

/// 更新警察空間哈希
pub fn update_police_spatial_hash_system(
    mut police_hash: ResMut<PoliceSpatialHash>,
    police_query: Query<(Entity, &Transform), With<PoliceOfficer>>,
) {
    police_hash.clear();
    police_hash.insert_batch(
        police_query
            .iter()
            .map(|(entity, transform)| (entity, transform.translation)),
    );
}

/// 處理犯罪事件，更新通緝等級
pub fn process_crime_events(
    mut crime_events: MessageReader<CrimeEvent>,
    mut wanted: ResMut<WantedLevel>,
    mut level_changed: MessageWriter<WantedLevelChanged>,
    time: Res<Time>,
) {
    for event in crime_events.read() {
        let old_stars = wanted.stars;
        let heat_increase = event.heat_value();

        wanted.add_heat(heat_increase);
        wanted.last_crime_time = time.elapsed_secs();
        wanted.cooldown_timer = 0.0;

        wanted.search_center = Some(event.position());
        wanted.search_radius = CRIME_SEARCH_RADIUS;
        wanted.search_timer = CRIME_SEARCH_TIMEOUT;

        if wanted.stars != old_stars {
            level_changed.write(WantedLevelChanged::new(old_stars, wanted.stars));
            info!(
                "通緝等級變化: {} -> {} (熱度: {:.1})",
                old_stars, wanted.stars, wanted.heat
            );
        }
    }
}

/// 處理目擊者報警事件
pub fn process_witness_reports(
    mut witness_events: MessageReader<WitnessReport>,
    mut wanted: ResMut<WantedLevel>,
    mut level_changed: MessageWriter<WantedLevelChanged>,
) {
    for report in witness_events.read() {
        let old_stars = wanted.stars;
        wanted.add_heat(WitnessReport::HEAT_VALUE);
        wanted.search_center = Some(report.position);
        wanted.search_timer = 45.0;

        if wanted.stars != old_stars {
            level_changed.write(WantedLevelChanged::new(old_stars, wanted.stars));
            info!(
                "目擊者報警: {} - 通緝等級 {} -> {} (熱度: {:.1})",
                report.crime_description, old_stars, wanted.stars, wanted.heat
            );
        }
    }
}

// ============================================================================
// 通緝消退
// ============================================================================

/// 更新每個警察的視線狀態
fn update_police_vision(
    player_pos: Vec3,
    police_hash: &PoliceSpatialHash,
    police_query: &mut Query<(&Transform, &mut PoliceOfficer)>,
    rapier: &RapierContext,
    config: &PoliceConfig,
) -> bool {
    for (_, mut officer) in police_query.iter_mut() {
        officer.can_see_player = false;
    }

    let mut any_visible = false;
    let half_fov = config.vision_fov / 2.0;

    for (police_entity, police_pos, _) in police_hash.query_radius(player_pos, VISION_RANGE) {
        let Ok((police_transform, mut officer)) = police_query.get_mut(police_entity) else {
            continue;
        };
        if officer.state == PoliceState::Patrolling {
            continue;
        }

        let to_player = player_pos - police_transform.translation;
        let distance = to_player.length();

        let police_forward = police_transform.forward().as_vec3();
        let angle = police_forward
            .dot(to_player.normalize())
            .clamp(-1.0, 1.0)
            .acos();
        if angle > half_fov {
            continue;
        }

        let ray_origin = police_pos + Vec3::Y * RAYCAST_ORIGIN_HEIGHT;
        if let Some((_, toi)) = rapier.cast_ray(
            ray_origin,
            to_player.normalize(),
            distance as RapierReal,
            true,
            QueryFilter::default().exclude_rigid_body(police_entity),
        ) {
            if rapier_real_to_f32(toi) >= distance - RAYCAST_HIT_TOLERANCE {
                officer.can_see_player = true;
                any_visible = true;
            }
        }
    }
    any_visible
}

/// 處理通緝消退邏輯
fn process_cooldown(
    wanted: &mut WantedLevel,
    level_changed: &mut MessageWriter<WantedLevelChanged>,
    dt: f32,
) {
    wanted.cooldown_timer += dt;

    if wanted.cooldown_timer >= wanted.cooldown_duration() {
        let old_stars = wanted.stars;
        let heat_reduction = (wanted.heat * 0.2).max(5.0);
        wanted.reduce_heat(heat_reduction);
        wanted.cooldown_timer = 0.0;

        if wanted.stars != old_stars {
            level_changed.write(WantedLevelChanged::new(old_stars, wanted.stars));
            info!(
                "⭐ 通緝等級消退: {} → {} (熱度: {:.1})",
                old_stars, wanted.stars, wanted.heat
            );
        }

        if wanted.stars == 0 {
            wanted.search_center = None;
            wanted.player_last_seen_pos = None;
        }
    }
}

/// 通緝等級消退系統
pub fn wanted_cooldown_system(
    mut wanted: ResMut<WantedLevel>,
    mut level_changed: MessageWriter<WantedLevelChanged>,
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    police_hash: Res<PoliceSpatialHash>,
    mut police_query: Query<(&Transform, &mut PoliceOfficer)>,
    rapier_context: ReadRapierContext,
    config: Res<PoliceConfig>,
) {
    if wanted.stars == 0 {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    let player_visible = rapier_context
        .single()
        .map(|rapier| {
            update_police_vision(
                player_pos,
                &police_hash,
                &mut police_query,
                &rapier,
                &config,
            )
        })
        .unwrap_or(false);

    wanted.player_visible = player_visible;

    if player_visible {
        wanted.player_last_seen_pos = Some(player_pos);
        wanted.cooldown_timer = 0.0;
        wanted.search_center = Some(player_pos);
        wanted.search_timer = 45.0;
    } else {
        process_cooldown(&mut wanted, &mut level_changed, time.delta_secs());

        if wanted.search_center.is_some() {
            wanted.search_timer -= time.delta_secs();
            if wanted.search_timer <= 0.0 {
                wanted.search_center = None;
                wanted.search_timer = 0.0;
                info!("⭐ 搜索區域超時，警察失去蹤跡");
            }
        }
    }
}

/// 警察生成系統
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn spawn_police_system(
    mut commands: Commands,
    wanted: Res<WantedLevel>,
    mut config: ResMut<PoliceConfig>,
    police_query: Query<Entity, With<PoliceOfficer>>,
    player_query: Query<&Transform, With<Player>>,
    visuals: Option<Res<PoliceVisuals>>,
    time: Res<Time>,
) {
    if wanted.stars == 0 {
        return;
    }
    let Some(visuals) = visuals else {
        return;
    };
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let current_count = police_query.iter().count() as u32;
    let target_count = wanted.target_police_count();
    if current_count >= target_count {
        return;
    }

    let elapsed = time.elapsed_secs();
    if elapsed - config.last_spawn_time < config.spawn_interval {
        return;
    }
    config.last_spawn_time = elapsed;

    let player_pos = player_transform.translation;
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let distance = config.spawn_distance_min
        + rand::random::<f32>() * (config.spawn_distance_max - config.spawn_distance_min);

    let spawn_pos = Vec3::new(
        player_pos.x + angle.cos() * distance,
        0.0,
        player_pos.z + angle.sin() * distance,
    );

    spawn_police_officer(&mut commands, spawn_pos, &visuals, wanted.stars);
    info!(
        "生成警察 at ({:.1}, {:.1}) - 當前: {}/{}",
        spawn_pos.x,
        spawn_pos.z,
        current_count + 1,
        target_count
    );
}

/// 生成單個警察 NPC
#[allow(clippy::too_many_lines)]
fn spawn_police_officer(
    commands: &mut Commands,
    position: Vec3,
    visuals: &PoliceVisuals,
    wanted_stars: u8,
) {
    let officer_type = if wanted_stars >= MILITARY_STAR_THRESHOLD {
        PoliceType::Military
    } else if wanted_stars >= SWAT_STAR_THRESHOLD {
        PoliceType::Swat
    } else {
        PoliceType::Patrol
    };

    let initial_state = if wanted_stars > 0 {
        PoliceState::Alerted
    } else {
        PoliceState::Patrolling
    };

    let patrol_offset = PATROL_OFFSET_RADIUS;
    let y = position.y;
    let patrol_route = vec![
        Vec3::new(position.x + patrol_offset, y, position.z),
        Vec3::new(position.x + patrol_offset, y, position.z + patrol_offset),
        Vec3::new(position.x, y, position.z + patrol_offset),
        Vec3::new(position.x, y, position.z),
    ];

    let police_entity = commands
        .spawn((
            Name::new("PoliceOfficer"),
            Transform::from_translation(position + Vec3::Y * OFFICER_SPAWN_HEIGHT),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            PoliceOfficer {
                state: initial_state,
                officer_type,
                target_player: wanted_stars > 0,
                patrol_route,
                ..default()
            },
            Health {
                current: if officer_type == PoliceType::Military {
                    MILITARY_HEALTH
                } else {
                    POLICE_OFFICER_HEALTH
                },
                max: if officer_type == PoliceType::Military {
                    MILITARY_HEALTH
                } else {
                    POLICE_OFFICER_HEALTH
                },
                ..default()
            },
            HitReaction::default(),
            AiMovement {
                walk_speed: OFFICER_WALK_SPEED,
                run_speed: match officer_type {
                    PoliceType::Military => MILITARY_RUN_SPEED,
                    PoliceType::Swat => SWAT_RUN_SPEED,
                    _ => OFFICER_RUN_SPEED,
                },
                ..default()
            },
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(OFFICER_CAPSULE_HALF_HEIGHT, OFFICER_CAPSULE_RADIUS),
            KinematicCharacterController {
                offset: CharacterLength::Absolute(OFFICER_CONTROLLER_OFFSET),
                ..default()
            },
        ))
        .id();

    let body_parts: &[(&Handle<Mesh>, &Handle<StandardMaterial>, Vec3)] = &[
        (
            &visuals.body_mesh,
            &visuals.uniform_material,
            Vec3::new(0.0, 0.0, 0.0),
        ),
        (
            &visuals.head_mesh,
            &visuals.skin_material,
            Vec3::new(0.0, 0.45, 0.0),
        ),
        (
            &visuals.leg_mesh,
            &visuals.uniform_material,
            Vec3::new(-0.1, -0.5, 0.0),
        ),
        (
            &visuals.leg_mesh,
            &visuals.uniform_material,
            Vec3::new(0.1, -0.5, 0.0),
        ),
        (
            &visuals.arm_mesh,
            &visuals.uniform_material,
            Vec3::new(-0.3, 0.1, 0.0),
        ),
        (
            &visuals.arm_mesh,
            &visuals.uniform_material,
            Vec3::new(0.3, 0.1, 0.0),
        ),
    ];

    commands.entity(police_entity).with_children(|parent| {
        for (mesh, material, offset) in body_parts {
            parent.spawn((
                Mesh3d((*mesh).clone()),
                MeshMaterial3d((*material).clone()),
                Transform::from_translation(*offset),
            ));
        }
    });
}

/// 警察消失系統
pub fn despawn_police_system(
    mut commands: Commands,
    police_query: Query<(Entity, &Transform), With<PoliceOfficer>>,
    player_query: Query<&Transform, (With<Player>, Without<PoliceOfficer>)>,
    wanted: Res<WantedLevel>,
    config: Res<PoliceConfig>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (entity, transform) in &police_query {
        let distance = (transform.translation - player_pos).length();
        let should_despawn = (wanted.stars == 0 && distance > config.despawn_distance)
            || distance > config.despawn_distance * 2.0;

        if should_despawn {
            commands.entity(entity).despawn();
        }
    }
}
