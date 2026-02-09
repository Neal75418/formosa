//! 環境互動系統
#![allow(dead_code)]


use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;
use std::f32::consts::TAU;

use super::{Destructible, DestructibleMaterial, DestructibleVisuals, Debris, DestructionEvent, DebrisPool};
use crate::combat::{DamageEvent, Damageable};
use crate::core::COLLISION_GROUP_STATIC;

/// 初始化可破壞物件視覺效果資源
pub fn setup_destructible_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DestructibleVisuals::new(&mut meshes, &mut materials));
    // 初始化碎片物件池（最多 150 個碎片同時存在）
    commands.insert_resource(DebrisPool::new(150));
    info!("💥 可破壞環境系統已初始化（含碎片物件池）");
}

// ============================================================================
// 可破壞物件輔助函數
// ============================================================================
/// 處理單個可破壞物件的傷害
fn process_destructible_damage(
    commands: &mut Commands,
    entity: Entity,
    transform: &Transform,
    destructible: &mut Destructible,
    damage_amount: f32,
    hit_position: Option<Vec3>,
    current_time: f32,
    destruction_events: &mut MessageWriter<DestructionEvent>,
) {
    // 計算方向（從攻擊者到目標）
    let impact_direction = hit_position.map(|pos| {
        (transform.translation - pos).normalize_or_zero()
    });

    // 造成傷害
    let destroyed = destructible.take_damage(damage_amount, current_time);

    if !destroyed {
        return;
    }

    // 發送破壞事件
    let mut destruction_event = DestructionEvent::new(
        entity,
        transform.translation,
        destructible.material,
        destructible.original_size,
    );

    if let Some(dir) = impact_direction {
        destruction_event = destruction_event.with_impact_direction(dir);
    }

    destruction_events.write(destruction_event);

    // 移除原始實體
    if let Ok(mut entity_commands) = commands.get_entity(entity) {
        entity_commands.despawn();
    }
}

/// 可破壞物件受傷系統
/// 監聯傷害事件，對可破壞物件造成傷害
pub fn destructible_damage_system(
    mut commands: Commands,
    time: Res<Time>,
    mut damage_events: MessageReader<DamageEvent>,
    mut destruction_events: MessageWriter<DestructionEvent>,
    mut destructible_query: Query<(Entity, &Transform, &mut Destructible)>,
) {
    let current_time = time.elapsed_secs();

    for event in damage_events.read() {
        let Ok((entity, transform, mut destructible)) = destructible_query.get_mut(event.target) else {
            continue;
        };

        if destructible.is_destroyed {
            continue;
        }

        process_destructible_damage(
            &mut commands,
            entity,
            transform,
            &mut destructible,
            event.amount,
            event.hit_position,
            current_time,
            &mut destruction_events,
        );
    }
}

// ============================================================================
// 碎片生成輔助函數
// ============================================================================
/// 碎片生成參數
struct DebrisSpawnParams {
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    scale: f32,
    max_lifetime: f32,
    material: DestructibleMaterial,
}

/// 計算碎片生成參數
fn calculate_debris_params(
    rng: &mut impl Rng,
    event: &DestructionEvent,
    min_scale: f32,
    max_scale: f32,
) -> DebrisSpawnParams {
    let offset = Vec3::new(
        rng.random_range(-event.size.x / 2.0..event.size.x / 2.0),
        rng.random_range(-event.size.y / 2.0..event.size.y / 2.0),
        rng.random_range(-event.size.z / 2.0..event.size.z / 2.0),
    );

    let base_velocity = event.impact_direction
        .map(|dir| dir * rng.random_range(3.0..8.0))
        .unwrap_or(Vec3::ZERO);

    let scatter = Vec3::new(
        rng.random_range(-3.0..3.0),
        rng.random_range(1.0..5.0),
        rng.random_range(-3.0..3.0),
    );

    DebrisSpawnParams {
        position: event.position + offset,
        velocity: base_velocity + scatter,
        rotation: Quat::from_euler(
            EulerRot::XYZ,
            rng.random_range(0.0..TAU),
            rng.random_range(0.0..TAU),
            rng.random_range(0.0..TAU),
        ),
        scale: rng.random_range(min_scale..max_scale),
        max_lifetime: event.material.debris_lifetime(),
        material: event.material,
    }
}

/// 重用池中的碎片實體
fn reuse_pooled_debris(
    rng: &mut impl Rng,
    debris: &mut Debris,
    transform: &mut Transform,
    visibility: &mut Visibility,
    params: &DebrisSpawnParams,
) {
    debris.material = params.material;
    debris.velocity = params.velocity;
    debris.angular_velocity = Vec3::new(
        rng.random_range(-5.0..5.0),
        rng.random_range(-5.0..5.0),
        rng.random_range(-5.0..5.0),
    );
    debris.lifetime = 0.0;
    debris.max_lifetime = params.max_lifetime;
    debris.has_gravity = true;
    debris.bounce_count = 0;

    transform.translation = params.position;
    transform.rotation = params.rotation;
    transform.scale = Vec3::splat(params.scale);

    *visibility = Visibility::Visible;
}

/// 創建新的碎片實體
fn spawn_new_debris(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    params: &DebrisSpawnParams,
) -> Entity {
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(params.position)
            .with_rotation(params.rotation)
            .with_scale(Vec3::splat(params.scale)),
        Debris::new(params.material, params.velocity),
        Visibility::Visible,
    )).id()
}

/// 嘗試從池中取得碎片實體
/// 返回 true 表示成功取得並重用
fn try_reuse_pooled_debris(
    rng: &mut impl Rng,
    debris_pool: &mut DebrisPool,
    debris_query: &mut Query<(&mut Debris, &mut Transform, &mut Visibility), Without<Destructible>>,
    params: &DebrisSpawnParams,
) -> bool {
    let Some(pooled_entity) = debris_pool.acquire() else {
        return false;
    };

    let Ok((mut debris, mut transform, mut visibility)) = debris_query.get_mut(pooled_entity) else {
        warn!("碎片池實體 {:?} 無效，已從池中移除", pooled_entity);
        return false;
    };

    reuse_pooled_debris(rng, &mut debris, &mut transform, &mut visibility, params);
    debris_pool.confirm_acquire(pooled_entity);
    true
}

/// 嘗試創建新碎片實體
/// 返回 true 表示成功創建
fn try_create_new_debris(
    commands: &mut Commands,
    debris_pool: &mut DebrisPool,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    params: &DebrisSpawnParams,
) -> bool {
    if !debris_pool.can_create_new() {
        return false;
    }

    let entity = spawn_new_debris(commands, mesh.clone(), material.clone(), params);
    debris_pool.add_new_entity(entity);
    true
}

/// 破壞效果系統（使用物件池優化）
/// 處理破壞事件，生成碎片
pub fn destruction_effect_system(
    mut commands: Commands,
    mut destruction_events: MessageReader<DestructionEvent>,
    visuals: Option<Res<DestructibleVisuals>>,
    mut debris_pool: ResMut<DebrisPool>,
    mut debris_query: Query<(&mut Debris, &mut Transform, &mut Visibility), Without<Destructible>>,
) {
    let Some(visuals) = visuals else { return };
    let mut rng = rand::rng();

    for event in destruction_events.read() {
        let debris_count = event.material.debris_count();
        let (min_scale, max_scale) = event.material.debris_scale_range();
        let (mesh, material) = visuals.get_debris_visuals(event.material);

        let mut spawned = 0;
        for _ in 0..debris_count {
            let params = calculate_debris_params(&mut rng, event, min_scale, max_scale);

            let success = try_reuse_pooled_debris(&mut rng, &mut debris_pool, &mut debris_query, &params)
                || try_create_new_debris(&mut commands, &mut debris_pool, &mesh, &material, &params);

            if success {
                spawned += 1;
            }
        }

        if spawned < debris_count {
            warn!("碎片池已滿，只生成了 {}/{} 個碎片", spawned, debris_count);
        }
    }
}

// ============================================================================
// 碎片更新輔助函數
// ============================================================================
/// 處理碎片的物理更新
fn update_debris_physics(debris: &mut Debris, transform: &mut Transform, dt: f32) {
    // 重力
    if debris.has_gravity {
        debris.velocity.y -= 9.8 * dt;
    }

    // 空氣阻力
    debris.velocity *= 1.0 - dt * 0.5;

    // 更新位置
    transform.translation += debris.velocity * dt;

    // 更新旋轉
    let rotation_delta = Quat::from_euler(
        EulerRot::XYZ,
        debris.angular_velocity.x * dt,
        debris.angular_velocity.y * dt,
        debris.angular_velocity.z * dt,
    );
    transform.rotation *= rotation_delta;
}

/// 處理碎片地面碰撞
fn handle_debris_ground_collision(debris: &mut Debris, transform: &mut Transform) {
    if transform.translation.y >= 0.05 {
        return;
    }

    transform.translation.y = 0.05;

    // 彈跳
    if debris.bounce_count < 2 && debris.velocity.y.abs() > 1.0 {
        debris.velocity.y = -debris.velocity.y * 0.3;
        debris.velocity.x *= 0.7;
        debris.velocity.z *= 0.7;
        debris.bounce_count += 1;
    } else {
        debris.velocity = Vec3::ZERO;
        debris.angular_velocity *= 0.5;
    }
}

/// 處理碎片淡出效果
fn apply_debris_fade(debris: &Debris, transform: &mut Transform) {
    let progress = debris.lifetime / debris.max_lifetime;
    if progress > 0.7 {
        let fade = 1.0 - (progress - 0.7) / 0.3;
        transform.scale *= Vec3::splat(fade.powf(0.1));
    }
}

/// 碎片更新系統（使用物件池優化）
/// 處理碎片的物理和生命週期
pub fn debris_update_system(
    time: Res<Time>,
    mut debris_pool: ResMut<DebrisPool>,
    mut debris_query: Query<(Entity, &mut Debris, &mut Transform, &mut Visibility)>,
) {
    let dt = time.delta_secs();

    for (entity, mut debris, mut transform, mut visibility) in debris_query.iter_mut() {
        if *visibility == Visibility::Hidden {
            continue;
        }

        debris.lifetime += dt;

        // 檢查是否過期
        if debris.lifetime >= debris.max_lifetime {
            *visibility = Visibility::Hidden;
            transform.translation = Vec3::new(0.0, -1000.0, 0.0);
            debris_pool.release(entity);
            continue;
        }

        update_debris_physics(&mut debris, &mut transform, dt);
        handle_debris_ground_collision(&mut debris, &mut transform);
        apply_debris_fade(&debris, &mut transform);
    }
}

// ============================================================================
// 輔助函數：生成可破壞物件
// ============================================================================

/// 可破壞物件材質快取資源
/// 避免每次生成物件時重複創建材質
#[derive(Resource)]
pub struct DestructibleMaterialCache {
    pub glass: Handle<StandardMaterial>,
    pub wood: Handle<StandardMaterial>,
    pub metal: Handle<StandardMaterial>,
}

impl DestructibleMaterialCache {
    /// 建立新實例
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            glass: materials.add(StandardMaterial {
                base_color: Color::srgba(0.7, 0.85, 1.0, 0.4),
                alpha_mode: AlphaMode::Blend,
                metallic: 0.0,
                perceptual_roughness: 0.1,
                ..default()
            }),
            wood: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.4, 0.25),
                perceptual_roughness: 0.85,
                ..default()
            }),
            metal: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.55),
                metallic: 0.8,
                perceptual_roughness: 0.4,
                ..default()
            }),
        }
    }
}

/// 生成玻璃窗（使用快取材質）
fn spawn_glass_window_cached(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec2,  // (width, height)
    rotation: Quat,
) {
    let thickness = 0.05;

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, thickness))),
        MeshMaterial3d(material),
        Transform::from_translation(position).with_rotation(rotation),
        Collider::cuboid(size.x / 2.0, size.y / 2.0, thickness / 2.0),
        RigidBody::Fixed,
        CollisionGroups::new(
            COLLISION_GROUP_STATIC,
            Group::ALL,
        ),
        Destructible::glass_window(size.x, size.y),
        Damageable,
        Name::new("GlassWindow"),
    ));
}

/// 生成玻璃窗（公開 API，每次創建新材質）
pub fn spawn_glass_window(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    size: Vec2,
    rotation: Quat,
) {
    let glass_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.85, 1.0, 0.4),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.0,
        perceptual_roughness: 0.1,
        ..default()
    });
    spawn_glass_window_cached(commands, meshes, glass_material, position, size, rotation);
}

/// 生成木製障礙物（使用快取材質）
fn spawn_wooden_barrier_cached(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        Collider::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
        RigidBody::Fixed,
        CollisionGroups::new(
            COLLISION_GROUP_STATIC,
            Group::ALL,
        ),
        Destructible::wooden_plank(size.x, size.y, size.z),
        Damageable,
        Name::new("WoodenBarrier"),
    ));
}

/// 生成木製障礙物（公開 API）
pub fn spawn_wooden_barrier(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    size: Vec3,
) {
    let wood_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.4, 0.25),
        perceptual_roughness: 0.85,
        ..default()
    });
    spawn_wooden_barrier_cached(commands, meshes, wood_material, position, size);
}

// ============================================================================
// 世界可破壞物件生成系統
// ============================================================================

/// 在世界中生成可破壞物件（玻璃窗、木製障礙等）
/// 這個系統在 Startup 時執行，為商店和建築添加可破壞的玻璃窗
///
/// 優化：使用材質快取避免重複創建相同材質
pub fn setup_world_destructibles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("🪟 正在生成可破壞環境物件...");

    // 創建材質快取（一次性創建，所有物件共用）
    let cache = DestructibleMaterialCache::new(&mut materials);
    // === 商店玻璃窗位置 ===
    let glass_windows = [
        // 便利商店 - 全家 (pos: -55, 5, -55)
        (Vec3::new(-55.0, 3.0, -50.5), Vec2::new(6.0, 4.0), Quat::IDENTITY),
        // 便利商店 - 7-11 (pos: 55, 5, -55)
        (Vec3::new(55.0, 3.0, -50.5), Vec2::new(6.0, 4.0), Quat::IDENTITY),
        // 刺青店 (pos: -25, 4, -55)
        (Vec3::new(-25.0, 2.5, -50.5), Vec2::new(4.0, 3.5), Quat::IDENTITY),
        // 速食店窗戶 (假設在 pos: 25, 4, -55)
        (Vec3::new(25.0, 2.5, -50.5), Vec2::new(5.0, 3.5), Quat::IDENTITY),
        // 咖啡店（假設在 pos: 0, 4.5, -55）
        (Vec3::new(0.0, 2.8, -50.5), Vec2::new(4.5, 4.0), Quat::IDENTITY),
        // 側面玻璃窗
        (Vec3::new(-50.5, 3.0, -25.0), Vec2::new(5.0, 4.0), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        (Vec3::new(50.5, 3.0, 25.0), Vec2::new(5.0, 4.0), Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
    ];

    let window_count = glass_windows.len();
    for (position, size, rotation) in glass_windows {
        spawn_glass_window_cached(&mut commands, &mut meshes, cache.glass.clone(), position, size, rotation);
    }

    // === 木製障礙物（路障、柵欄等）===
    let wooden_barriers = [
        // 建築工地入口障礙
        (Vec3::new(40.0, 0.75, 40.0), Vec3::new(2.0, 1.5, 0.3)),
        (Vec3::new(42.5, 0.75, 40.0), Vec3::new(2.0, 1.5, 0.3)),
        // 停車場入口障礙
        (Vec3::new(-40.0, 0.6, 20.0), Vec3::new(3.0, 1.2, 0.2)),
        // 巷子裡的木箱
        (Vec3::new(-35.0, 0.5, -35.0), Vec3::new(1.0, 1.0, 1.0)),
        (Vec3::new(-33.5, 0.5, -35.0), Vec3::new(1.0, 1.0, 1.0)),
        (Vec3::new(-34.25, 1.5, -35.0), Vec3::new(1.0, 1.0, 1.0)),
        // 後巷棧板
        (Vec3::new(35.0, 0.15, -40.0), Vec3::new(1.5, 0.3, 1.5)),
        (Vec3::new(35.0, 0.15, -42.0), Vec3::new(1.5, 0.3, 1.5)),
    ];

    let barrier_count = wooden_barriers.len();
    for (position, size) in wooden_barriers {
        spawn_wooden_barrier_cached(&mut commands, &mut meshes, cache.wood.clone(), position, size);
    }

    // === 金屬物件（廢棄車輛零件、垃圾桶等）===
    let metal_objects = [
        // 巷子裡的金屬垃圾桶
        (Vec3::new(-45.0, 0.6, -20.0), Vec3::new(0.8, 1.2, 0.8)),
        (Vec3::new(45.0, 0.6, 20.0), Vec3::new(0.8, 1.2, 0.8)),
        // 建築工地金屬板
        (Vec3::new(38.0, 0.5, 38.0), Vec3::new(2.0, 0.1, 2.0)),
    ];

    let metal_count = metal_objects.len();
    for (position, size) in metal_objects {
        spawn_metal_object_cached(&mut commands, &mut meshes, cache.metal.clone(), position, size);
    }

    // 將快取儲存為資源供後續使用
    commands.insert_resource(cache);

    info!(
        "✅ 可破壞環境物件生成完成: {} 玻璃窗, {} 木製障礙, {} 金屬物件",
        window_count, barrier_count, metal_count
    );
}

/// 生成金屬可破壞物件（使用快取材質）
fn spawn_metal_object_cached(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        Collider::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
        RigidBody::Fixed,
        CollisionGroups::new(
            COLLISION_GROUP_STATIC,
            Group::ALL,
        ),
        Destructible {
            health: 80.0,
            max_health: 80.0,
            material: DestructibleMaterial::Metal,
            original_size: size,
            is_destroyed: false,
            last_damage_time: 0.0,
        },
        Damageable,
        Name::new("MetalObject"),
    ));
}

/// 生成金屬可破壞物件（公開 API）
pub fn spawn_metal_object(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    size: Vec3,
) {
    let metal_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.55),
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });
    spawn_metal_object_cached(commands, meshes, metal_material, position, size);
}
