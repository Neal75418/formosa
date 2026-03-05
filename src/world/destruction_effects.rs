//! 破壞特效與碎片生成
//!
//! 碎片物理生成、粒子效果、可破壞物件建構

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

#[allow(clippy::wildcard_imports)]
use super::destructible::*;
use crate::vehicle::random_rotation;

// ============================================================================
// 常數
// ============================================================================

/// 碎片生命週期（秒）
const DEBRIS_LIFETIME: f32 = 5.0;
/// 碎片最小速度
const DEBRIS_MIN_SPEED: f32 = 3.0;
/// 碎片最大速度
const DEBRIS_MAX_SPEED: f32 = 8.0;

// ============================================================================
// 碎片生成輔助
// ============================================================================

/// 碎片隨機參數
struct DebrisParams {
    position: Vec3,
    rotation: Quat,
    velocity: Vec3,
    scale: f32,
    lifetime: f32,
}

/// 根據碎片類型取得對應的 mesh 和 material
fn get_debris_mesh_and_material(
    visuals: &DestructibleVisuals,
    debris_type: DestructibleType,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    match debris_type {
        DestructibleType::Glass => (
            visuals.glass_shard_mesh.clone(),
            visuals.glass_material.clone(),
        ),
        DestructibleType::Wood | DestructibleType::Plastic => (
            visuals.wood_debris_mesh.clone(),
            visuals.wood_material.clone(),
        ),
        DestructibleType::Metal | DestructibleType::Electronic => (
            visuals.metal_debris_mesh.clone(),
            visuals.metal_material.clone(),
        ),
    }
}

/// 計算隨機碎片參數
fn calc_random_debris_params(base_position: Vec3, impact_direction: Vec3) -> DebrisParams {
    let random_offset = Vec3::new(
        (rand::random::<f32>() - 0.5) * 2.0,
        rand::random::<f32>() * 0.5 + 0.3,
        (rand::random::<f32>() - 0.5) * 2.0,
    );

    let velocity = (impact_direction * 0.5 + random_offset).normalize()
        * (DEBRIS_MIN_SPEED + rand::random::<f32>() * (DEBRIS_MAX_SPEED - DEBRIS_MIN_SPEED));

    let position = base_position
        + Vec3::new(
            (rand::random::<f32>() - 0.5) * 0.5,
            rand::random::<f32>() * 0.5,
            (rand::random::<f32>() - 0.5) * 0.5,
        );

    DebrisParams {
        position,
        rotation: random_rotation(),
        velocity,
        scale: 0.5 + rand::random::<f32>() * 0.5,
        lifetime: DEBRIS_LIFETIME + rand::random::<f32>() * 2.0,
    }
}

/// 生成單一粒子（火花或灰塵）
fn spawn_particle(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    is_spark: bool,
) {
    let (velocity, lifetime, scale, y_offset) = if is_spark {
        (
            Vec3::new(
                (rand::random::<f32>() - 0.5) * 6.0,
                rand::random::<f32>() * 4.0 + 2.0,
                (rand::random::<f32>() - 0.5) * 6.0,
            ),
            0.3 + rand::random::<f32>() * 0.3,
            0.05,
            0.3,
        )
    } else {
        (
            Vec3::new(
                (rand::random::<f32>() - 0.5) * 2.0,
                rand::random::<f32>() * 1.5 + 0.5,
                (rand::random::<f32>() - 0.5) * 2.0,
            ),
            1.0 + rand::random::<f32>() * 0.5,
            0.2,
            0.0,
        )
    };

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(position + Vec3::Y * y_offset).with_scale(Vec3::splat(scale)),
        DestructionParticle {
            is_spark,
            lifetime,
            velocity,
        },
    ));
}

/// 生成單一碎片實體
fn spawn_debris_entity(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    params: &DebrisParams,
    debris_type: DestructibleType,
    index: usize,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("Debris_{index}")),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(params.position)
                .with_rotation(params.rotation)
                .with_scale(Vec3::splat(params.scale)),
            RigidBody::Dynamic,
            Collider::cuboid(
                0.05 * params.scale,
                0.05 * params.scale,
                0.05 * params.scale,
            ),
            Velocity::linear(params.velocity),
            GravityScale(1.5),
            Damping {
                linear_damping: 0.5,
                angular_damping: 0.8,
            },
            Debris {
                debris_type,
                lifetime: params.lifetime,
                max_lifetime: DEBRIS_LIFETIME + 2.0,
            },
            Visibility::Visible,
        ))
        .id()
}

// ============================================================================
// 公開生成函數
// ============================================================================

/// 生成碎片（使用物件池優化）
pub fn spawn_debris_pooled<F: bevy::ecs::query::QueryFilter>(
    commands: &mut Commands,
    visuals: &DestructibleVisuals,
    debris_pool: &mut DebrisPool,
    debris_query: &mut Query<(&mut Debris, &mut Transform, &mut Velocity, &mut Visibility), F>,
    position: Vec3,
    debris_type: DestructibleType,
    count: usize,
    impact_direction: Vec3,
) {
    let (mesh, material) = get_debris_mesh_and_material(visuals, debris_type);
    let mut spawned = 0;

    for i in 0..count {
        let params = calc_random_debris_params(position, impact_direction);

        // 嘗試從池中取得實體
        if let Some(pooled_entity) = debris_pool.acquire() {
            // 重用池中的實體
            if let Ok((mut debris, mut transform, mut velocity, mut visibility)) =
                debris_query.get_mut(pooled_entity)
            {
                debris.debris_type = debris_type;
                debris.lifetime = params.lifetime;
                debris.max_lifetime = DEBRIS_LIFETIME + 2.0;

                transform.translation = params.position;
                transform.rotation = params.rotation;
                transform.scale = Vec3::splat(params.scale);

                velocity.linvel = params.velocity;

                *visibility = Visibility::Visible;
                spawned += 1;
                continue;
            }
        }

        // 池中無可用實體，創建新的（但限制總數）
        if debris_pool.can_create_new() {
            let entity = spawn_debris_entity(
                commands,
                mesh.clone(),
                material.clone(),
                &params,
                debris_type,
                i,
            );
            debris_pool.add_new_entity(entity);
            spawned += 1;
        }
    }

    if spawned < count {
        warn!("碎片池已滿，只生成了 {}/{} 個碎片", spawned, count);
    }
}

/// 生成碎片（舊版，不使用物件池 - 用於無池情況）
pub fn spawn_debris(
    commands: &mut Commands,
    visuals: &DestructibleVisuals,
    position: Vec3,
    debris_type: DestructibleType,
    count: usize,
    impact_direction: Vec3,
) {
    // 限制最大生成數量以防止效能問題
    let actual_count = count.min(10);
    let (mesh, material) = get_debris_mesh_and_material(visuals, debris_type);

    for i in 0..actual_count {
        let params = calc_random_debris_params(position, impact_direction);
        spawn_debris_entity(
            commands,
            mesh.clone(),
            material.clone(),
            &params,
            debris_type,
            i,
        );
    }
}

/// 生成破壞粒子效果
pub fn spawn_destruction_particles(
    commands: &mut Commands,
    visuals: &DestructibleVisuals,
    position: Vec3,
    debris_type: DestructibleType,
) {
    // 火花（金屬/電子設備）
    if debris_type.produces_sparks() {
        for _ in 0..10 {
            spawn_particle(
                commands,
                visuals.glass_shard_mesh.clone(),
                visuals.spark_material.clone(),
                position,
                true, // is_spark
            );
        }
    }

    // 灰塵
    for _ in 0..8 {
        spawn_particle(
            commands,
            visuals.glass_shard_mesh.clone(),
            visuals.dust_material.clone(),
            position,
            false, // is_spark
        );
    }
}

/// 在世界中生成可破壞物件（供 setup 使用）
pub fn spawn_destructible_object(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    destructible_type: DestructibleType,
) -> Entity {
    let (mesh, material, size) = match destructible_type {
        DestructibleType::Glass => (
            meshes.add(Mesh::from(Cuboid::new(2.0, 2.5, 0.1))),
            materials.add(StandardMaterial {
                base_color: Color::srgba(0.6, 0.8, 0.9, 0.5),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            Vec3::new(1.0, 1.25, 0.05),
        ),
        DestructibleType::Wood => (
            meshes.add(Mesh::from(Cuboid::new(3.0, 1.0, 0.2))),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.35, 0.2),
                ..default()
            }),
            Vec3::new(1.5, 0.5, 0.1),
        ),
        DestructibleType::Metal => (
            meshes.add(Mesh::from(Cuboid::new(2.0, 0.8, 0.1))),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.55),
                metallic: 0.8,
                ..default()
            }),
            Vec3::new(1.0, 0.4, 0.05),
        ),
        DestructibleType::Plastic => (
            meshes.add(Mesh::from(Cylinder::new(0.3, 0.8))),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.3, 0.2),
                ..default()
            }),
            Vec3::new(0.3, 0.4, 0.3),
        ),
        DestructibleType::Electronic => (
            meshes.add(Mesh::from(Cuboid::new(1.0, 2.0, 0.8))),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.1, 0.1),
                ..default()
            }),
            Vec3::new(0.5, 1.0, 0.4),
        ),
    };

    commands
        .spawn((
            Name::new(format!("Destructible_{destructible_type:?}")),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position),
            Collider::cuboid(size.x, size.y, size.z),
            RigidBody::Fixed,
            Destructible::new(destructible_type),
        ))
        .id()
}
