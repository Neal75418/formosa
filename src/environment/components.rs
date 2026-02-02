//! 環境互動組件
#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

use bevy::prelude::*;
use crate::core::{EntityPool, calculate_fade_alpha};

// ============================================================================
// 可破壞物件
// ============================================================================

/// 可破壞物件材質類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DestructibleMaterial {
    /// 玻璃（脆弱、碎片透明）
    #[default]
    Glass,
    /// 木材（中等耐久、碎片較大）
    Wood,
    /// 金屬（較耐久、火花效果）
    Metal,
    /// 塑膠（輕微傷害即破）
    Plastic,
}

impl DestructibleMaterial {
    /// 取得材質的基礎血量
    pub fn base_health(&self) -> f32 {
        match self {
            DestructibleMaterial::Glass => 10.0,
            DestructibleMaterial::Wood => 50.0,
            DestructibleMaterial::Metal => 100.0,
            DestructibleMaterial::Plastic => 20.0,
        }
    }

    /// 取得碎片數量
    pub fn debris_count(&self) -> usize {
        match self {
            DestructibleMaterial::Glass => 8,
            DestructibleMaterial::Wood => 5,
            DestructibleMaterial::Metal => 3,
            DestructibleMaterial::Plastic => 4,
        }
    }

    /// 取得碎片縮放範圍
    pub fn debris_scale_range(&self) -> (f32, f32) {
        match self {
            DestructibleMaterial::Glass => (0.05, 0.15),
            DestructibleMaterial::Wood => (0.1, 0.3),
            DestructibleMaterial::Metal => (0.08, 0.2),
            DestructibleMaterial::Plastic => (0.06, 0.18),
        }
    }

    /// 取得碎片生命時間（秒）
    pub fn debris_lifetime(&self) -> f32 {
        match self {
            DestructibleMaterial::Glass => 2.0,
            DestructibleMaterial::Wood => 3.0,
            DestructibleMaterial::Metal => 4.0,
            DestructibleMaterial::Plastic => 2.5,
        }
    }
}

/// 可破壞物件組件
#[derive(Component, Debug)]
pub struct Destructible {
    /// 材質類型
    pub material: DestructibleMaterial,
    /// 當前血量
    pub health: f32,
    /// 最大血量
    pub max_health: f32,
    /// 是否已破壞
    pub is_destroyed: bool,
    /// 原始尺寸（用於碎片生成）
    pub original_size: Vec3,
    /// 最後受傷時間
    pub last_damage_time: f32,
}

impl Default for Destructible {
    fn default() -> Self {
        Self {
            material: DestructibleMaterial::Glass,
            health: 10.0,
            max_health: 10.0,
            is_destroyed: false,
            original_size: Vec3::ONE,
            last_damage_time: 0.0,
        }
    }
}

impl Destructible {
    /// 創建指定材質的可破壞物件
    pub fn new(material: DestructibleMaterial, size: Vec3) -> Self {
        let base_health = material.base_health();
        Self {
            material,
            health: base_health,
            max_health: base_health,
            original_size: size,
            ..default()
        }
    }

    /// 玻璃窗
    pub fn glass_window(width: f32, height: f32) -> Self {
        Self::new(DestructibleMaterial::Glass, Vec3::new(width, height, 0.05))
    }

    /// 木板
    pub fn wooden_plank(width: f32, height: f32, depth: f32) -> Self {
        Self::new(DestructibleMaterial::Wood, Vec3::new(width, height, depth))
    }

    /// 金屬板
    pub fn metal_plate(width: f32, height: f32) -> Self {
        Self::new(DestructibleMaterial::Metal, Vec3::new(width, height, 0.02))
    }

    /// 受傷
    pub fn take_damage(&mut self, amount: f32, time: f32) -> bool {
        if self.is_destroyed {
            return false;
        }

        self.health = (self.health - amount).max(0.0);
        self.last_damage_time = time;

        if self.health <= 0.0 {
            self.is_destroyed = true;
            return true; // 已破壞
        }

        false
    }
}

// ============================================================================
// 碎片系統
// ============================================================================

/// 碎片粒子組件
#[derive(Component, Debug)]
pub struct Debris {
    /// 材質類型
    pub material: DestructibleMaterial,
    /// 速度
    pub velocity: Vec3,
    /// 角速度
    pub angular_velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 是否受重力影響
    pub has_gravity: bool,
    /// 彈跳計數（避免無限彈跳）
    pub bounce_count: u8,
}

impl Debris {
    pub fn new(material: DestructibleMaterial, velocity: Vec3) -> Self {
        Self {
            material,
            velocity,
            angular_velocity: Vec3::new(
                rand::random::<f32>() * 10.0 - 5.0,
                rand::random::<f32>() * 10.0 - 5.0,
                rand::random::<f32>() * 10.0 - 5.0,
            ),
            lifetime: 0.0,
            max_lifetime: material.debris_lifetime(),
            has_gravity: true,
            bounce_count: 0,
        }
    }

    /// 計算當前透明度
    pub fn alpha(&self) -> f32 {
        let progress = self.lifetime / self.max_lifetime;
        calculate_fade_alpha(progress, 0.7)
    }
}

// ============================================================================
// 破壞事件
// ============================================================================

/// 破壞事件
#[derive(Clone, Debug, bevy::prelude::Message)]
pub struct DestructionEvent {
    /// 被破壞的實體
    pub entity: Entity,
    /// 破壞位置
    pub position: Vec3,
    /// 材質類型
    pub material: DestructibleMaterial,
    /// 原始尺寸
    pub size: Vec3,
    /// 衝擊方向（用於碎片飛濺方向）
    pub impact_direction: Option<Vec3>,
}

impl DestructionEvent {
    pub fn new(entity: Entity, position: Vec3, material: DestructibleMaterial, size: Vec3) -> Self {
        Self {
            entity,
            position,
            material,
            size,
            impact_direction: None,
        }
    }

    pub fn with_impact_direction(mut self, direction: Vec3) -> Self {
        self.impact_direction = Some(direction);
        self
    }
}

// ============================================================================
// 碎片物件池（效能優化）
// ============================================================================

/// 碎片物件池
///
/// 避免頻繁的 spawn/despawn 造成記憶體分配開銷。
/// 碎片結束生命週期時歸還池中重用，而非銷毀。
///
/// 使用通用 `EntityPool` 實現，避免重複程式碼。
#[derive(Resource, Default)]
pub struct DebrisPool {
    /// 內部實體池
    pool: EntityPool,
}

impl DebrisPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: EntityPool::new(max_size),
        }
    }

    /// 從池中取得一個碎片實體（僅標記為候選，需呼叫 confirm_acquire 確認）
    #[inline]
    pub fn acquire(&mut self) -> Option<Entity> {
        self.pool.acquire()
    }

    /// 確認取得實體（將實體加入使用中列表）
    #[inline]
    pub fn confirm_acquire(&mut self, entity: Entity) {
        self.pool.confirm_acquire(entity);
    }

    /// 取消取得（將實體退回可用列表）
    #[inline]
    pub fn cancel_acquire(&mut self, entity: Entity) {
        self.pool.cancel_acquire(entity);
    }

    /// 歸還碎片實體到池中
    #[inline]
    pub fn release(&mut self, entity: Entity) {
        self.pool.release(entity);
    }

    /// 清理無效實體（當外部系統刪除了池中的實體時使用）
    #[inline]
    pub fn cleanup_invalid(&mut self, is_valid: impl Fn(Entity) -> bool) {
        self.pool.cleanup_invalid(is_valid);
    }

    /// 取得目前使用中的碎片數量
    #[inline]
    pub fn active_count(&self) -> usize {
        self.pool.active_count()
    }

    /// 取得池中可用的碎片數量
    #[inline]
    pub fn available_count(&self) -> usize {
        self.pool.available_count()
    }

    /// 檢查是否可以生成更多碎片
    #[inline]
    pub fn can_spawn(&self) -> bool {
        self.pool.can_spawn()
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

// ============================================================================
// 視覺效果資源
// ============================================================================

/// 可破壞物件視覺效果資源
#[derive(Resource)]
pub struct DestructibleVisuals {
    /// 玻璃碎片 mesh
    pub glass_shard_mesh: Handle<Mesh>,
    /// 玻璃碎片材質
    pub glass_shard_material: Handle<StandardMaterial>,
    /// 木材碎片 mesh
    pub wood_chunk_mesh: Handle<Mesh>,
    /// 木材碎片材質
    pub wood_chunk_material: Handle<StandardMaterial>,
    /// 金屬碎片 mesh
    pub metal_shard_mesh: Handle<Mesh>,
    /// 金屬碎片材質
    pub metal_shard_material: Handle<StandardMaterial>,
    /// 塑膠碎片 mesh
    pub plastic_chunk_mesh: Handle<Mesh>,
    /// 塑膠碎片材質
    pub plastic_chunk_material: Handle<StandardMaterial>,
}

impl DestructibleVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            // 玻璃碎片（三角形薄片）
            glass_shard_mesh: meshes.add(Cuboid::new(0.1, 0.15, 0.01)),
            glass_shard_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.9, 1.0, 0.6),
                alpha_mode: AlphaMode::Blend,
                metallic: 0.1,
                perceptual_roughness: 0.1,
                ..default()
            }),
            // 木材碎片（小方塊）
            wood_chunk_mesh: meshes.add(Cuboid::new(0.15, 0.08, 0.1)),
            wood_chunk_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.4, 0.2),
                perceptual_roughness: 0.8,
                ..default()
            }),
            // 金屬碎片（薄片）
            metal_shard_mesh: meshes.add(Cuboid::new(0.08, 0.12, 0.02)),
            metal_shard_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.55),
                metallic: 0.9,
                perceptual_roughness: 0.3,
                ..default()
            }),
            // 塑膠碎片
            plastic_chunk_mesh: meshes.add(Cuboid::new(0.1, 0.1, 0.05)),
            plastic_chunk_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.9, 0.9, 0.85),
                perceptual_roughness: 0.6,
                ..default()
            }),
        }
    }

    /// 取得對應材質的 mesh 和 material
    pub fn get_debris_visuals(&self, material: DestructibleMaterial) -> (Handle<Mesh>, Handle<StandardMaterial>) {
        match material {
            DestructibleMaterial::Glass => (self.glass_shard_mesh.clone(), self.glass_shard_material.clone()),
            DestructibleMaterial::Wood => (self.wood_chunk_mesh.clone(), self.wood_chunk_material.clone()),
            DestructibleMaterial::Metal => (self.metal_shard_mesh.clone(), self.metal_shard_material.clone()),
            DestructibleMaterial::Plastic => (self.plastic_chunk_mesh.clone(), self.plastic_chunk_material.clone()),
        }
    }
}
