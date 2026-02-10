//! 可破壞環境系統
//!
//! GTA 風格的環境破壞，包含：
//! - 玻璃破碎
//! - 欄杆/護欄摧毀
//! - 街道家具破壞
//! - 碎片物理
//! - 破壞粒子效果

#![allow(dead_code)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::vehicle::{Vehicle, random_rotation};
use crate::combat::{DamageEvent, DamageSource};
use crate::core::EntityPool;

// ============================================================================
// 常數
// ============================================================================

/// 玻璃破碎碎片數量
const GLASS_SHARD_COUNT: usize = 15;
/// 木質碎片數量
const WOOD_DEBRIS_COUNT: usize = 8;
/// 金屬碎片數量
const METAL_DEBRIS_COUNT: usize = 6;
/// 碎片生命週期（秒）
const DEBRIS_LIFETIME: f32 = 5.0;
/// 碎片最小速度
const DEBRIS_MIN_SPEED: f32 = 3.0;
/// 碎片最大速度
const DEBRIS_MAX_SPEED: f32 = 8.0;
/// 車輛撞擊傷害
const VEHICLE_IMPACT_DAMAGE: f32 = 100.0;
/// 子彈傷害
const BULLET_DAMAGE: f32 = 25.0;
/// 爆炸傷害基礎值
const EXPLOSION_DAMAGE_BASE: f32 = 150.0;

// ============================================================================
// 組件與資源
// ============================================================================

/// 碎片物件池（效能優化）
///
/// 使用通用 `EntityPool` 實現，避免重複程式碼。
#[derive(Resource, Default)]
pub struct DebrisPool {
    /// 內部實體池
    pool: EntityPool,
}

impl DebrisPool {
    /// 建立新實例
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: EntityPool::new(max_size),
        }
    }

    /// 從池中取得一個碎片實體
    pub fn acquire(&mut self) -> Option<Entity> {
        let entity = self.pool.acquire();
        if let Some(e) = entity {
            self.pool.confirm_acquire(e);
        }
        entity
    }

    /// 歸還碎片實體到池中
    #[inline]
    pub fn release(&mut self, entity: Entity) {
        self.pool.release(entity);
    }

    /// 檢查池是否還有空間
    #[inline]
    pub fn has_capacity(&self) -> bool {
        self.pool.has_capacity()
    }

    /// 將新建立的實體加入使用中列表
    #[inline]
    pub fn add_new_entity(&mut self, entity: Entity) {
        self.pool.in_use.push(entity);
    }

    /// 檢查是否可以創建新實體
    #[inline]
    pub fn can_create_new(&self) -> bool {
        self.pool.in_use.len() + self.pool.available.len() < self.pool.max_size
    }
}

/// 可破壞環境視覺效果資源
#[derive(Resource)]
pub struct DestructibleVisuals {
    // 碎片網格
    pub glass_shard_mesh: Handle<Mesh>,
    pub wood_debris_mesh: Handle<Mesh>,
    pub metal_debris_mesh: Handle<Mesh>,
    // 碎片材質
    pub glass_material: Handle<StandardMaterial>,
    pub wood_material: Handle<StandardMaterial>,
    pub metal_material: Handle<StandardMaterial>,
    // 破壞效果材質
    pub spark_material: Handle<StandardMaterial>,
    pub dust_material: Handle<StandardMaterial>,
}

impl DestructibleVisuals {
    /// 建立新實例
    pub fn new(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        Self {
            // 玻璃碎片（小三角形）
            glass_shard_mesh: meshes.add(Mesh::from(Cuboid::new(0.1, 0.15, 0.02))),
            // 木質碎片
            wood_debris_mesh: meshes.add(Mesh::from(Cuboid::new(0.15, 0.08, 0.08))),
            // 金屬碎片
            metal_debris_mesh: meshes.add(Mesh::from(Cuboid::new(0.12, 0.04, 0.12))),

            // 玻璃材質（半透明藍綠色）
            glass_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.6, 0.8, 0.9, 0.7),
                alpha_mode: AlphaMode::Blend,
                metallic: 0.1,
                perceptual_roughness: 0.1,
                ..default()
            }),
            // 木質材質
            wood_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.35, 0.2),
                perceptual_roughness: 0.8,
                ..default()
            }),
            // 金屬材質
            metal_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.4, 0.45),
                metallic: 0.8,
                perceptual_roughness: 0.4,
                ..default()
            }),

            // 火花材質
            spark_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.3),
                emissive: LinearRgba::new(5.0, 3.0, 0.5, 1.0),
                unlit: true,
                ..default()
            }),
            // 灰塵材質
            dust_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.6, 0.55, 0.5, 0.6),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
        }
    }
}

/// 可破壞物件類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestructibleType {
    /// 玻璃（窗戶、門）
    Glass,
    /// 木質（欄杆、箱子）
    Wood,
    /// 金屬（護欄、路標）
    Metal,
    /// 塑膠（垃圾桶、路錐）
    Plastic,
    /// 電子設備（自動販賣機、電話亭）
    Electronic,
}

impl DestructibleType {
    /// 獲取最大生命值
    pub fn max_health(&self) -> f32 {
        match self {
            DestructibleType::Glass => 20.0,
            DestructibleType::Wood => 50.0,
            DestructibleType::Metal => 100.0,
            DestructibleType::Plastic => 30.0,
            DestructibleType::Electronic => 80.0,
        }
    }

    /// 獲取碎片數量
    pub fn debris_count(&self) -> usize {
        match self {
            DestructibleType::Glass => GLASS_SHARD_COUNT,
            DestructibleType::Wood => WOOD_DEBRIS_COUNT,
            DestructibleType::Metal => METAL_DEBRIS_COUNT,
            DestructibleType::Plastic => WOOD_DEBRIS_COUNT,
            DestructibleType::Electronic => METAL_DEBRIS_COUNT + 3, // 額外電子零件
        }
    }

    /// 是否會產生火花
    pub fn produces_sparks(&self) -> bool {
        matches!(self, DestructibleType::Metal | DestructibleType::Electronic)
    }

    /// 車輛撞擊是否直接摧毀
    pub fn vehicle_one_hit(&self) -> bool {
        matches!(self, DestructibleType::Glass | DestructibleType::Plastic)
    }
}

/// 可破壞物件狀態
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DestructibleState {
    /// 完整
    #[default]
    Intact,
    /// 損壞（部分破損）
    Damaged,
    /// 摧毀
    Destroyed,
}

/// 可破壞物件組件
#[derive(Component)]
pub struct Destructible {
    /// 物件類型
    pub destructible_type: DestructibleType,
    /// 當前狀態
    pub state: DestructibleState,
    /// 當前生命值
    pub health: f32,
    /// 最大生命值
    pub max_health: f32,
    /// 損壞閾值（百分比）
    pub damage_threshold: f32,
    /// 是否可以被子彈破壞
    pub bullet_vulnerable: bool,
    /// 是否可以被車輛撞毀
    pub vehicle_vulnerable: bool,
    /// 是否可以被爆炸破壞
    pub explosion_vulnerable: bool,
    /// 是否已經生成碎片
    pub debris_spawned: bool,
}

impl Destructible {
    /// 建立新實例
    pub fn new(destructible_type: DestructibleType) -> Self {
        let max_health = destructible_type.max_health();
        Self {
            destructible_type,
            state: DestructibleState::Intact,
            health: max_health,
            max_health,
            damage_threshold: 0.5,
            bullet_vulnerable: true,
            vehicle_vulnerable: true,
            explosion_vulnerable: true,
            debris_spawned: false,
        }
    }

    /// 玻璃窗戶
    pub fn glass_window() -> Self {
        Self::new(DestructibleType::Glass)
    }

    /// 木質欄杆
    pub fn wood_fence() -> Self {
        Self::new(DestructibleType::Wood)
    }

    /// 金屬護欄
    pub fn metal_barrier() -> Self {
        let mut d = Self::new(DestructibleType::Metal);
        d.bullet_vulnerable = false; // 子彈無法破壞金屬護欄
        d
    }

    /// 垃圾桶
    pub fn trash_can() -> Self {
        Self::new(DestructibleType::Plastic)
    }

    /// 自動販賣機
    pub fn vending_machine() -> Self {
        let mut d = Self::new(DestructibleType::Electronic);
        d.bullet_vulnerable = true;
        d.vehicle_vulnerable = true;
        d
    }

    /// 施加傷害
    pub fn apply_damage(&mut self, amount: f32) {
        self.health = (self.health - amount).max(0.0);

        if self.health <= 0.0 {
            self.state = DestructibleState::Destroyed;
        } else if self.health <= self.max_health * self.damage_threshold {
            self.state = DestructibleState::Damaged;
        }
    }

    /// 是否已摧毀
    pub fn is_destroyed(&self) -> bool {
        self.state == DestructibleState::Destroyed
    }
}

/// 碎片組件
#[derive(Component)]
pub struct Debris {
    /// 碎片類型（對應材質）
    pub debris_type: DestructibleType,
    /// 剩餘生命週期
    pub lifetime: f32,
    /// 最大生命週期
    pub max_lifetime: f32,
}

/// 破壞粒子效果
#[derive(Component)]
pub struct DestructionParticle {
    /// 類型（火花/灰塵）
    pub is_spark: bool,
    /// 剩餘生命週期
    pub lifetime: f32,
    /// 速度
    pub velocity: Vec3,
}

/// 環境破壞事件
#[derive(Message, Clone)]
pub struct EnvironmentDamageEvent {
    /// 目標實體
    pub target: Entity,
    /// 傷害量
    pub amount: f32,
    /// 傷害來源
    pub source: DamageSource,
    /// 撞擊位置
    pub impact_position: Option<Vec3>,
    /// 撞擊方向
    pub impact_direction: Option<Vec3>,
}

// ============================================================================
// 系統
// ============================================================================

/// 初始化可破壞環境視覺效果
pub fn setup_destructible_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DestructibleVisuals::new(&mut meshes, &mut materials));
    // 初始化碎片物件池（最多同時存在 100 個碎片）
    commands.insert_resource(DebrisPool::new(100));
}

/// 車輛碰撞破壞系統
pub fn vehicle_destructible_collision_system(
    mut collision_events: MessageReader<CollisionEvent>,
    vehicle_query: Query<(&Transform, &Velocity), With<Vehicle>>,
    destructible_query: Query<(Entity, &Transform, &Destructible)>,
    mut env_damage_events: MessageWriter<EnvironmentDamageEvent>,
) {
    for event in collision_events.read() {
        let CollisionEvent::Started(e1, e2, _) = event else {
            continue;
        };

        let Some((vehicle_entity, destructible_entity)) =
            vehicle_destructible_pair(*e1, *e2, &vehicle_query, &destructible_query)
        else {
            continue;
        };

        let Ok((vehicle_transform, velocity)) = vehicle_query.get(vehicle_entity) else {
            continue;
        };

        let Ok((entity, dest_transform, destructible)) = destructible_query.get(destructible_entity) else {
            continue;
        };

        let Some(damage) = compute_vehicle_impact_damage(destructible, velocity) else {
            continue;
        };
        let impact_direction =
            compute_vehicle_impact_direction(dest_transform, vehicle_transform, velocity);

        env_damage_events.write(EnvironmentDamageEvent {
            target: entity,
            amount: damage,
            source: DamageSource::Vehicle,
            impact_position: Some(dest_transform.translation),
            impact_direction: Some(impact_direction),
        });
    }
}

fn vehicle_destructible_pair(
    e1: Entity,
    e2: Entity,
    vehicle_query: &Query<(&Transform, &Velocity), With<Vehicle>>,
    destructible_query: &Query<(Entity, &Transform, &Destructible)>,
) -> Option<(Entity, Entity)> {
    let e1_is_vehicle = vehicle_query.contains(e1);
    let e2_is_vehicle = vehicle_query.contains(e2);
    let e1_is_destructible = destructible_query.contains(e1);
    let e2_is_destructible = destructible_query.contains(e2);

    if e1_is_vehicle && e2_is_destructible {
        Some((e1, e2))
    } else if e2_is_vehicle && e1_is_destructible {
        Some((e2, e1))
    } else {
        None
    }
}

fn compute_vehicle_impact_damage(destructible: &Destructible, velocity: &Velocity) -> Option<f32> {
    if !destructible.vehicle_vulnerable {
        return None;
    }

    let speed = velocity.linvel.length();
    if speed < 3.0 {
        return None;
    }

    let damage = if destructible.destructible_type.vehicle_one_hit() {
        destructible.max_health * 2.0
    } else {
        VEHICLE_IMPACT_DAMAGE * (speed / 10.0).min(2.0)
    };

    Some(damage)
}

fn compute_vehicle_impact_direction(
    dest_transform: &Transform,
    vehicle_transform: &Transform,
    velocity: &Velocity,
) -> Vec3 {
    let impact_delta = dest_transform.translation - vehicle_transform.translation;
    if impact_delta.length_squared() > 1e-6 {
        impact_delta.normalize()
    } else {
        velocity.linvel.normalize_or_zero()
    }
}

/// 處理環境破壞事件（使用物件池優化）
pub fn handle_environment_damage_system(
    mut commands: Commands,
    mut env_damage_events: MessageReader<EnvironmentDamageEvent>,
    mut destructible_query: Query<(Entity, &Transform, &mut Destructible, Option<&Collider>)>,
    mut debris_query: Query<(&mut Debris, &mut Transform, &mut Velocity, &mut Visibility), Without<Destructible>>,
    visuals: Option<Res<DestructibleVisuals>>,
    mut debris_pool: ResMut<DebrisPool>,
) {
    let Some(visuals) = visuals else { return; };

    for event in env_damage_events.read() {
        let Ok((entity, transform, mut destructible, _collider)) = destructible_query.get_mut(event.target) else {
            continue;
        };

        // 檢查傷害來源是否有效
        let valid_damage = match event.source {
            DamageSource::Bullet => destructible.bullet_vulnerable,
            DamageSource::Explosion => destructible.explosion_vulnerable,
            DamageSource::Vehicle => destructible.vehicle_vulnerable,
            _ => true,
        };

        if !valid_damage {
            continue;
        }

        let previous_state = destructible.state;
        destructible.apply_damage(event.amount);

        // 狀態變化處理
        if destructible.state != previous_state {
            match destructible.state {
                DestructibleState::Damaged => {
                    info!("物件損壞: {:?}", destructible.destructible_type);
                    // 生成少量碎片（使用物件池）
                    spawn_debris_pooled(
                        &mut commands,
                        &visuals,
                        &mut debris_pool,
                        &mut debris_query,
                        transform.translation,
                        destructible.destructible_type,
                        3,
                        event.impact_direction.unwrap_or(Vec3::Y),
                    );
                }
                DestructibleState::Destroyed => {
                    info!("物件摧毀: {:?}", destructible.destructible_type);

                    if !destructible.debris_spawned {
                        destructible.debris_spawned = true;

                        // 生成大量碎片（使用物件池）
                        spawn_debris_pooled(
                            &mut commands,
                            &visuals,
                            &mut debris_pool,
                            &mut debris_query,
                            transform.translation,
                            destructible.destructible_type,
                            destructible.destructible_type.debris_count(),
                            event.impact_direction.unwrap_or(Vec3::Y),
                        );

                        // 生成破壞粒子效果
                        spawn_destruction_particles(
                            &mut commands,
                            &visuals,
                            transform.translation,
                            destructible.destructible_type,
                        );
                    }

                    // 移除碰撞體並隱藏或刪除實體
                    commands.entity(entity).remove::<Collider>();
                    commands.entity(entity).insert(Visibility::Hidden);
                }
                _ => {}
            }
        }
    }
}

/// 整合戰鬥傷害系統（子彈/爆炸對可破壞物件的傷害）
pub fn combat_destructible_damage_system(
    mut damage_events: MessageReader<DamageEvent>,
    destructible_query: Query<Entity, With<Destructible>>,
    mut env_damage_events: MessageWriter<EnvironmentDamageEvent>,
) {
    for event in damage_events.read() {
        // 只處理可破壞物件
        if !destructible_query.contains(event.target) {
            continue;
        }

        let amount = match event.source {
            DamageSource::Bullet => BULLET_DAMAGE,
            DamageSource::Explosion => EXPLOSION_DAMAGE_BASE,
            DamageSource::Melee => BULLET_DAMAGE * 0.5,
            _ => event.amount,
        };

        env_damage_events.write(EnvironmentDamageEvent {
            target: event.target,
            amount,
            source: event.source,
            impact_position: event.hit_position,
            impact_direction: None, // 從攻擊者方向計算（如需要）
        });
    }
}

/// 碎片更新系統
pub fn debris_update_system(
    _commands: Commands,
    mut debris_query: Query<(Entity, &mut Debris, &mut Transform, Option<&mut MeshMaterial3d<StandardMaterial>>, &mut Visibility)>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut debris_pool: ResMut<DebrisPool>,
) {
    let dt = time.delta_secs();

    for (entity, mut debris, mut transform, material_handle, mut visibility) in &mut debris_query {
        // 跳過已隱藏（已歸還池）的碎片
        if *visibility == Visibility::Hidden {
            continue;
        }

        debris.lifetime -= dt;

        if debris.lifetime <= 0.0 {
            // 歸還到物件池而非銷毀
            *visibility = Visibility::Hidden;
            transform.translation = Vec3::new(0.0, -1000.0, 0.0); // 移到地圖外
            debris_pool.release(entity);
            continue;
        }

        // 緩慢縮小
        let scale_factor = (debris.lifetime / debris.max_lifetime).max(0.1);
        transform.scale = Vec3::splat(scale_factor * 0.8 + 0.2);

        // 淡出效果（最後 1 秒）
        if debris.lifetime < 1.0 {
            if let Some(material_handle) = material_handle {
                if let Some(material) = materials.get_mut(&material_handle.0) {
                    let alpha = debris.lifetime;
                    material.base_color = material.base_color.with_alpha(alpha);
                    material.alpha_mode = AlphaMode::Blend;
                }
            }
        }
    }
}

/// 破壞粒子更新系統
pub fn destruction_particle_update_system(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut DestructionParticle, &mut Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut particle, mut transform) in &mut particle_query {
        particle.lifetime -= dt;

        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 移動
        transform.translation += particle.velocity * dt;

        // 重力
        particle.velocity.y -= 9.8 * dt;

        // 火花：快速收縮
        if particle.is_spark {
            let scale = (particle.lifetime * 2.0).min(1.0);
            transform.scale = Vec3::splat(scale * 0.1);
        } else {
            // 灰塵：緩慢擴散和消散
            let scale = 0.2 + (1.0 - particle.lifetime / 1.5) * 0.3;
            transform.scale = Vec3::splat(scale);
        }
    }
}

// ============================================================================
// 輔助函數
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
        DestructibleType::Glass => (visuals.glass_shard_mesh.clone(), visuals.glass_material.clone()),
        DestructibleType::Wood | DestructibleType::Plastic => {
            (visuals.wood_debris_mesh.clone(), visuals.wood_material.clone())
        }
        DestructibleType::Metal | DestructibleType::Electronic => {
            (visuals.metal_debris_mesh.clone(), visuals.metal_material.clone())
        }
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

    let position = base_position + Vec3::new(
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
        Transform::from_translation(position + Vec3::Y * y_offset)
            .with_scale(Vec3::splat(scale)),
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
    commands.spawn((
        Name::new(format!("Debris_{}", index)),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(params.position)
            .with_rotation(params.rotation)
            .with_scale(Vec3::splat(params.scale)),
        RigidBody::Dynamic,
        Collider::cuboid(0.05 * params.scale, 0.05 * params.scale, 0.05 * params.scale),
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
    )).id()
}

/// 生成碎片（使用物件池優化）
fn spawn_debris_pooled<F: bevy::ecs::query::QueryFilter>(
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
            if let Ok((mut debris, mut transform, mut velocity, mut visibility)) = debris_query.get_mut(pooled_entity) {
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
            let entity = spawn_debris_entity(commands, mesh.clone(), material.clone(), &params, debris_type, i);
            debris_pool.add_new_entity(entity);
            spawned += 1;
        }
    }

    if spawned < count {
        warn!("碎片池已滿，只生成了 {}/{} 個碎片", spawned, count);
    }
}

/// 生成碎片（舊版，不使用物件池 - 用於無池情況）
fn spawn_debris(
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
        spawn_debris_entity(commands, mesh.clone(), material.clone(), &params, debris_type, i);
    }
}

/// 生成破壞粒子效果
fn spawn_destruction_particles(
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

    commands.spawn((
        Name::new(format!("Destructible_{:?}", destructible_type)),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        Collider::cuboid(size.x, size.y, size.z),
        RigidBody::Fixed,
        Destructible::new(destructible_type),
    )).id()
}
