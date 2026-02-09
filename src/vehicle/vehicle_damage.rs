//! 載具損壞系統（類型定義、碰撞傷害、火災、爆炸）
#![allow(dead_code)]

use super::*;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;
use crate::core::lifetime_linear_alpha;
use crate::combat::{DamageEvent, DamageSource, Enemy};

// ============================================================================
// 車輛損壞類型定義
// ============================================================================

/// 車輛損壞狀態
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VehicleDamageState {
    /// 完好無損
    #[default]
    Pristine,
    /// 輕微損壞（<25% 傷害）：有刮痕
    Light,
    /// 中度損壞（25-50%）：凹陷、冒煙
    Moderate,
    /// 嚴重損壞（50-75%）：大量冒煙、引擎聲異常
    Heavy,
    /// 瀕臨爆炸（75-99%）：著火、閃爍
    Critical,
    /// 已爆炸
    Destroyed,
}

impl VehicleDamageState {
    /// 根據血量百分比計算損壞狀態
    pub fn from_health_percent(percent: f32) -> Self {
        if percent >= 1.0 {
            VehicleDamageState::Pristine
        } else if percent >= 0.75 {
            VehicleDamageState::Light
        } else if percent >= 0.50 {
            VehicleDamageState::Moderate
        } else if percent >= 0.25 {
            VehicleDamageState::Heavy
        } else if percent > 0.0 {
            VehicleDamageState::Critical
        } else {
            VehicleDamageState::Destroyed
        }
    }
}

/// 車輛血量組件
#[derive(Component, Debug)]
pub struct VehicleHealth {
    /// 當前血量
    pub current: f32,
    /// 最大血量
    pub max: f32,
    /// 損壞狀態
    pub damage_state: VehicleDamageState,
    /// 是否無敵（暫時）
    pub is_invulnerable: bool,
    /// 最後受傷時間
    pub last_damage_time: f32,
    /// 著火狀態
    pub is_on_fire: bool,
    /// 著火計時器（到 0 時爆炸）
    pub fire_timer: f32,
    /// 爆炸冷卻（防止連續爆炸）
    pub explosion_cooldown: f32,
}

impl Default for VehicleHealth {
    fn default() -> Self {
        Self {
            current: 1000.0,
            max: 1000.0,
            damage_state: VehicleDamageState::Pristine,
            is_invulnerable: false,
            last_damage_time: 0.0,
            is_on_fire: false,
            fire_timer: 5.0,
            explosion_cooldown: 0.0,
        }
    }
}

impl VehicleHealth {
    /// 創建指定血量的車輛
    pub fn new(max_health: f32) -> Self {
        Self {
            current: max_health,
            max: max_health,
            ..default()
        }
    }

    /// 根據車輛類型創建
    pub fn for_vehicle_type(vehicle_type: VehicleType) -> Self {
        let max_health = match vehicle_type {
            VehicleType::Scooter => 500.0,   // 機車較脆弱
            VehicleType::Car => 1000.0,      // 汽車標準
            VehicleType::Taxi => 1200.0,     // 計程車較耐打
            VehicleType::Bus => 2000.0,      // 公車最耐打
        };
        Self::new(max_health)
    }

    /// 受傷
    pub fn take_damage(&mut self, amount: f32, time: f32) -> f32 {
        if self.is_invulnerable || self.damage_state == VehicleDamageState::Destroyed {
            return 0.0;
        }

        let actual_damage = amount.min(self.current);
        self.current = (self.current - amount).max(0.0);
        self.last_damage_time = time;

        // 更新損壞狀態
        self.damage_state = VehicleDamageState::from_health_percent(self.percentage());

        // 瀕臨爆炸時開始著火
        if self.damage_state == VehicleDamageState::Critical && !self.is_on_fire {
            self.is_on_fire = true;
            self.fire_timer = 5.0;  // 5 秒後爆炸
        }

        actual_damage
    }

    /// 修復
    pub fn repair(&mut self, amount: f32) {
        if self.damage_state == VehicleDamageState::Destroyed {
            return;
        }

        self.current = (self.current + amount).min(self.max);
        self.damage_state = VehicleDamageState::from_health_percent(self.percentage());

        // 修復到一定程度時滅火
        if self.percentage() > 0.3 {
            self.is_on_fire = false;
        }
    }

    /// 套用裝甲改裝（增加最大血量）
    /// 只應在改裝時呼叫一次，會增加最大血量並按比例恢復當前血量
    pub fn apply_armor_upgrade(&mut self, multiplier: f32) {
        let old_max = self.max;
        let new_max = old_max * multiplier;
        let health_ratio = self.current / old_max;

        self.max = new_max;
        self.current = new_max * health_ratio;  // 保持相同的血量比例
        self.damage_state = VehicleDamageState::from_health_percent(self.percentage());
    }

    /// 完全修復
    pub fn full_repair(&mut self) {
        self.current = self.max;
        self.damage_state = VehicleDamageState::Pristine;
        self.is_on_fire = false;
        self.fire_timer = 5.0;
    }

    /// 血量百分比
    pub fn percentage(&self) -> f32 {
        self.current / self.max
    }

    /// 是否已爆炸
    pub fn is_destroyed(&self) -> bool {
        self.damage_state == VehicleDamageState::Destroyed
    }

    /// 更新著火計時器
    ///
    /// # 著火機制說明
    /// 車輛著火時有兩種傷害來源：
    /// 1. **計時器爆炸**：`fire_timer` 倒數到 0 時觸發爆炸（預設 5 秒）
    /// 2. **持續燒傷**：每秒扣除 `FIRE_DAMAGE_PER_SECOND` 血量（20 HP/s）
    ///
    /// 這意味著即使車輛血量充足，著火 5 秒後仍會爆炸。
    /// 玩家應該在車輛著火後盡快離開。
    ///
    /// # 回傳值
    /// - `true`：車輛爆炸（計時器歸零或血量歸零）
    /// - `false`：車輛仍在燃燒
    pub fn tick_fire(&mut self, dt: f32) -> bool {
        /// 著火時每秒造成的傷害
        const FIRE_DAMAGE_PER_SECOND: f32 = 20.0;

        if self.is_on_fire {
            // 計時器倒數（到 0 時強制爆炸）
            self.fire_timer -= dt;

            // 持續燒傷：著火時每秒扣血
            self.current = (self.current - dt * FIRE_DAMAGE_PER_SECOND).max(0.0);
            self.damage_state = VehicleDamageState::from_health_percent(self.percentage());

            // 爆炸條件：計時器歸零 或 血量歸零
            if self.fire_timer <= 0.0 || self.current <= 0.0 {
                self.damage_state = VehicleDamageState::Destroyed;
                return true; // 爆炸！
            }
        }
        false
    }
}

/// 輪胎損壞組件
#[derive(Component, Debug)]
pub struct TireDamage {
    /// 四個輪胎的狀態（true = 爆胎）
    /// 順序：左前、右前、左後、右後
    pub flat_tires: [bool; 4],
    /// 爆胎後的操控懲罰（0.0-1.0）
    pub handling_penalty: f32,
    /// 爆胎後的最大速度懲罰（0.0-1.0）
    pub speed_penalty: f32,
}

impl Default for TireDamage {
    fn default() -> Self {
        Self {
            flat_tires: [false; 4],
            handling_penalty: 0.0,
            speed_penalty: 0.0,
        }
    }
}

impl TireDamage {
    /// 爆破指定輪胎
    pub fn pop_tire(&mut self, index: usize) {
        if index < 4 {
            self.flat_tires[index] = true;
            self.update_penalties();
        }
    }

    /// 修復指定輪胎
    pub fn repair_tire(&mut self, index: usize) {
        if index < 4 {
            self.flat_tires[index] = false;
            self.update_penalties();
        }
    }

    /// 修復所有輪胎
    pub fn repair_all(&mut self) {
        self.flat_tires = [false; 4];
        self.handling_penalty = 0.0;
        self.speed_penalty = 0.0;
    }

    /// 爆胎數量
    pub fn flat_count(&self) -> usize {
        self.flat_tires.iter().filter(|&&f| f).count()
    }

    /// 更新懲罰值
    fn update_penalties(&mut self) {
        let flat_count = self.flat_count();
        // flat_count 只可能是 0-4（4 個輪胎）
        debug_assert!(flat_count <= 4, "flat_count 超出範圍: {}", flat_count);

        self.handling_penalty = match flat_count {
            0 => 0.0,
            1 => 0.15,  // 一個爆胎：輕微影響
            2 => 0.30,  // 兩個爆胎：明顯影響
            3 => 0.45,  // 三個爆胎：嚴重影響
            _ => 0.60,  // 全部爆胎：困難但可操控
        };
        self.speed_penalty = match flat_count {
            0 => 0.0,
            1 => 0.10,  // 速度降低 10%
            2 => 0.20,  // 速度降低 20%
            3 => 0.35,  // 速度降低 35%
            _ => 0.50,  // 速度降低 50%
        };
    }

    /// 檢查前輪是否有爆胎（影響轉向）
    pub fn has_front_flat(&self) -> bool {
        self.flat_tires[0] || self.flat_tires[1]
    }

    /// 檢查後輪是否有爆胎（影響穩定性）
    pub fn has_rear_flat(&self) -> bool {
        self.flat_tires[2] || self.flat_tires[3]
    }
}

/// 車輛損壞視覺效果資源
#[derive(Resource)]
pub struct VehicleDamageVisuals {
    /// 冒煙粒子 mesh
    pub smoke_mesh: Handle<Mesh>,
    /// 輕微冒煙材質（白煙）
    pub light_smoke_material: Handle<StandardMaterial>,
    /// 嚴重冒煙材質（黑煙）
    pub heavy_smoke_material: Handle<StandardMaterial>,
    /// 火焰粒子 mesh
    pub fire_mesh: Handle<Mesh>,
    /// 火焰材質
    pub fire_material: Handle<StandardMaterial>,
    /// 爆炸粒子 mesh
    pub explosion_mesh: Handle<Mesh>,
    /// 爆炸材質
    pub explosion_material: Handle<StandardMaterial>,
}

impl VehicleDamageVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            smoke_mesh: meshes.add(Sphere::new(0.3)),
            light_smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.8, 0.8, 0.4),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            heavy_smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.2, 0.2, 0.6),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            fire_mesh: meshes.add(Sphere::new(0.2)),
            fire_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0),
                emissive: LinearRgba::new(15.0, 8.0, 0.0, 1.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            explosion_mesh: meshes.add(Sphere::new(2.0)),
            explosion_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.2),
                emissive: LinearRgba::new(50.0, 30.0, 5.0, 1.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        }
    }
}

/// 車輛損壞煙霧粒子
#[derive(Component)]
pub struct VehicleDamageSmoke {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 是否為黑煙（嚴重損壞）
    pub is_heavy: bool,
}

impl VehicleDamageSmoke {
    /// 建立新實例
    pub fn new(velocity: Vec3, is_heavy: bool) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime: if is_heavy { 2.0 } else { 1.5 },
            is_heavy,
        }
    }

    /// 計算透明度
    pub fn alpha(&self) -> f32 {
        lifetime_linear_alpha(self.lifetime, self.max_lifetime)
    }
}

/// 車輛火焰粒子
#[derive(Component)]
pub struct VehicleFire {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl VehicleFire {
    /// 建立新實例
    pub fn new(velocity: Vec3) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime: 0.5,
        }
    }

    /// 計算縮放
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        (1.0 - progress * 0.5).max(0.3)
    }
}

/// 車輛爆炸效果
#[derive(Component)]
pub struct VehicleExplosion {
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 爆炸中心
    pub center: Vec3,
    /// 爆炸範圍
    pub radius: f32,
    /// 傷害
    pub damage: f32,
    /// 是否已造成傷害
    pub has_damage_dealt: bool,
}

impl VehicleExplosion {
    /// 建立新實例
    pub fn new(center: Vec3, radius: f32, damage: f32) -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 1.0,
            center,
            radius,
            damage,
            has_damage_dealt: false,
        }
    }

    /// 計算當前縮放（先擴大後縮小）
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if progress < 0.3 {
            // 快速擴大
            progress / 0.3 * 1.5
        } else {
            // 緩慢縮小
            1.5 - (progress - 0.3) / 0.7 * 1.5
        }
    }

    /// 計算透明度
    pub fn alpha(&self) -> f32 {
        lifetime_linear_alpha(self.lifetime, self.max_lifetime)
    }
}

// ============================================================================
// 車輛損壞系統（GTA 5 風格）
// ============================================================================

use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::wanted::PoliceOfficer;

/// 初始化車輛損壞視覺效果資源
pub fn setup_vehicle_damage_effects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(VehicleDamageVisuals::new(&mut meshes, &mut materials));
    info!("💥 車輛損壞系統已初始化");
}

// ============================================================================
// 車輛碰撞傷害常數
// ============================================================================
/// 碰撞傷害冷卻時間（秒）- 防止持續接觸時每幀扣血
const COLLISION_DAMAGE_COOLDOWN: f32 = 0.5;
/// 造成傷害的最低速度門檻（m/s）
const COLLISION_DAMAGE_SPEED_THRESHOLD: f32 = 10.0;
/// 傷害倍率：每超過門檻 1 m/s 造成此數值傷害
const COLLISION_DAMAGE_MULTIPLIER: f32 = 5.0;

/// 車輛碰撞傷害系統
/// 根據碰撞速度計算車輛傷害
pub fn vehicle_collision_damage_system(
    time: Res<Time>,
    rapier_context: ReadRapierContext,
    mut vehicle_query: Query<(Entity, &Transform, &Vehicle, &mut VehicleHealth)>,
) {
    let current_time = time.elapsed_secs();

    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    for (entity, _transform, vehicle, mut health) in vehicle_query.iter_mut() {
        // 已爆炸的車輛不處理
        if health.is_destroyed() {
            continue;
        }

        // 碰撞傷害冷卻：防止持續接觸時每幀扣血
        if current_time - health.last_damage_time < COLLISION_DAMAGE_COOLDOWN {
            continue;
        }

        // 檢查碰撞事件
        for contact_pair in rapier.contact_pairs_with(entity) {
            // 只處理有接觸的碰撞
            if !contact_pair.has_any_active_contact() {
                continue;
            }

            // 根據速度計算傷害
            // 速度越快，傷害越高
            let speed = vehicle.current_speed.abs();
            if speed < COLLISION_DAMAGE_SPEED_THRESHOLD {
                continue; // 低速碰撞不造成傷害
            }

            // 傷害公式：(速度 - 門檻) * 倍率
            // 例如：30 m/s = (30-10) * 5 = 100 傷害
            let damage = (speed - COLLISION_DAMAGE_SPEED_THRESHOLD) * COLLISION_DAMAGE_MULTIPLIER;
            health.take_damage(damage, current_time);
            break; // 一次碰撞只計算一次傷害
        }
    }
}

/// 取得車輛類型對應的爆炸半徑
fn get_explosion_radius(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 5.0,
        VehicleType::Car | VehicleType::Taxi => 8.0,
        VehicleType::Bus => 12.0,
    }
}

/// 取得車輛類型對應的爆炸傷害
fn get_explosion_damage(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 100.0,
        VehicleType::Car | VehicleType::Taxi => 200.0,
        VehicleType::Bus => 300.0,
    }
}

/// 車輛火焰系統
/// 處理著火狀態和爆炸倒計時
pub fn vehicle_fire_system(
    mut commands: Commands,
    time: Res<Time>,
    mut vehicle_query: Query<(Entity, &Transform, &Vehicle, &mut VehicleHealth)>,
    damage_visuals: Option<Res<VehicleDamageVisuals>>,
) {
    let dt = time.delta_secs();

    for (entity, transform, vehicle, mut health) in vehicle_query.iter_mut() {
        if health.is_destroyed() {
            continue;
        }

        if !health.tick_fire(dt) {
            continue;
        }

        // 爆炸！
        let explosion_pos = transform.translation + Vec3::Y * 0.5;
        let explosion_radius = get_explosion_radius(vehicle.vehicle_type);
        let explosion_damage = get_explosion_damage(vehicle.vehicle_type);

        if let Some(ref visuals) = damage_visuals {
            commands.spawn((
                Mesh3d(visuals.explosion_mesh.clone()),
                MeshMaterial3d(visuals.explosion_material.clone()),
                Transform::from_translation(explosion_pos),
                VehicleExplosion::new(explosion_pos, explosion_radius, explosion_damage),
            ));
        }

        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
        }
    }
}

// ============================================================================
// 車輛損壞視覺效果常數
// ============================================================================
/// 中度損壞煙霧生成率（每秒）
const MODERATE_SMOKE_RATE: f32 = 1.2;
/// 嚴重損壞煙霧生成率（每秒）
const HEAVY_SMOKE_RATE: f32 = 6.0;
/// 瀕臨爆炸煙霧生成率（每秒）
const CRITICAL_SMOKE_RATE: f32 = 9.0;
/// 瀕臨爆炸火焰生成率（每秒）
const CRITICAL_FIRE_RATE: f32 = 6.0;

/// 車輛損壞視覺效果系統
/// 根據損壞狀態生成煙霧和火焰粒子
/// 使用時間基準的生成率，確保效果與幀率無關
pub fn vehicle_damage_effect_system(
    mut commands: Commands,
    time: Res<Time>,
    damage_visuals: Option<Res<VehicleDamageVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle, &VehicleHealth)>,
) {
    let Some(visuals) = damage_visuals else {
        return;
    };
    let dt = time.delta_secs();
    let mut rng = rand::rng();

    for (transform, vehicle, health) in vehicle_query.iter() {
        // 計算引擎蓋位置（車頭）
        let hood_offset = match vehicle.vehicle_type {
            VehicleType::Scooter => Vec3::new(0.0, 0.3, -0.6),
            VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.5, -1.5),
            VehicleType::Bus => Vec3::new(0.0, 1.0, -4.0),
        };
        let hood_pos = transform.translation + transform.rotation * hood_offset;

        // 根據損壞狀態生成效果
        // 使用時間基準的機率：rate * dt 使生成與幀率無關
        match health.damage_state {
            VehicleDamageState::Moderate => {
                // 中度損壞：偶爾冒白煙
                if rng.random::<f32>() < MODERATE_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, false, &mut rng);
                }
            }
            VehicleDamageState::Heavy => {
                // 嚴重損壞：持續冒黑煙
                if rng.random::<f32>() < HEAVY_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, true, &mut rng);
                }
            }
            VehicleDamageState::Critical => {
                // 瀕臨爆炸：冒黑煙 + 火焰
                if rng.random::<f32>() < CRITICAL_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, true, &mut rng);
                }
                if rng.random::<f32>() < CRITICAL_FIRE_RATE * dt {
                    spawn_vehicle_fire(&mut commands, &visuals, hood_pos, &mut rng);
                }
            }
            _ => {}
        }
    }
}

/// 生成損壞煙霧粒子
fn spawn_damage_smoke(
    commands: &mut Commands,
    visuals: &VehicleDamageVisuals,
    position: Vec3,
    is_heavy: bool,
    rng: &mut rand::prelude::ThreadRng,
) {
    let spread = Vec3::new(
        rng.random_range(-0.3..0.3),
        rng.random_range(0.5..1.5),
        rng.random_range(-0.3..0.3),
    );

    let material = if is_heavy {
        visuals.heavy_smoke_material.clone()
    } else {
        visuals.light_smoke_material.clone()
    };

    commands.spawn((
        Mesh3d(visuals.smoke_mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(position).with_scale(Vec3::splat(0.2)),
        VehicleDamageSmoke::new(spread, is_heavy),
    ));
}

/// 生成車輛火焰粒子
fn spawn_vehicle_fire(
    commands: &mut Commands,
    visuals: &VehicleDamageVisuals,
    position: Vec3,
    rng: &mut rand::prelude::ThreadRng,
) {
    let spread = Vec3::new(
        rng.random_range(-0.2..0.2),
        rng.random_range(0.8..1.5),
        rng.random_range(-0.2..0.2),
    );

    commands.spawn((
        Mesh3d(visuals.fire_mesh.clone()),
        MeshMaterial3d(visuals.fire_material.clone()),
        Transform::from_translation(position + Vec3::Y * 0.1)
            .with_scale(Vec3::splat(rng.random_range(0.3..0.6))),
        VehicleFire::new(spread),
    ));
}

/// 對範圍內的目標造成爆炸傷害
fn apply_explosion_damage_to_targets<'a>(
    targets: impl Iterator<Item = (Entity, &'a Transform)>,
    explosion_center: Vec3,
    explosion_radius: f32,
    explosion_damage: f32,
    damage_events: &mut MessageWriter<DamageEvent>,
    exclude_entity: Option<Entity>,
) {
    for (target_entity, target_transform) in targets {
        if Some(target_entity) == exclude_entity {
            continue;
        }
        let distance = explosion_center.distance(target_transform.translation);
        if distance < explosion_radius {
            let damage_factor = 1.0 - (distance / explosion_radius);
            damage_events.write(
                DamageEvent::new(
                    target_entity,
                    explosion_damage * damage_factor,
                    DamageSource::Explosion,
                )
                .with_position(explosion_center),
            );
        }
    }
}

/// 觸發爆炸攝影機震動效果
fn trigger_explosion_camera_shake(
    explosion_center: Vec3,
    explosion_radius: f32,
    player_pos: Vec3,
    camera_shake: &mut crate::core::CameraShake,
) {
    let distance_to_player = explosion_center.distance(player_pos);
    let max_shake_distance = explosion_radius * 3.0;

    if distance_to_player < max_shake_distance {
        let falloff = 1.0 - distance_to_player / max_shake_distance;
        camera_shake.trigger(0.5 * falloff, 0.4 + 0.3 * falloff);
    }
}

/// 車輛爆炸系統
/// 處理爆炸效果和範圍傷害
/// 對範圍內的所有可傷害實體（玩家、敵人、行人、警察、其他車輛）造成傷害
#[allow(clippy::type_complexity)]
pub fn vehicle_explosion_system(
    mut commands: Commands,
    time: Res<Time>,
    mut camera_shake: ResMut<crate::core::CameraShake>,
    mut explosion_query: Query<(Entity, &mut VehicleExplosion, &mut Transform)>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<VehicleExplosion>)>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<VehicleExplosion>)>,
    pedestrian_query: Query<
        (Entity, &Transform),
        (
            With<Pedestrian>,
            Without<Player>,
            Without<Enemy>,
            Without<VehicleExplosion>,
        ),
    >,
    police_query: Query<
        (Entity, &Transform),
        (
            With<PoliceOfficer>,
            Without<Player>,
            Without<Enemy>,
            Without<VehicleExplosion>,
        ),
    >,
    vehicle_query: Query<(Entity, &Transform), (With<VehicleHealth>, Without<VehicleExplosion>)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let dt = time.delta_secs();

    for (entity, mut explosion, mut transform) in explosion_query.iter_mut() {
        explosion.lifetime += dt;
        transform.scale = Vec3::splat(explosion.scale());

        if !explosion.has_damage_dealt {
            explosion.has_damage_dealt = true;

            if let Ok((_, player_transform)) = player_query.single() {
                trigger_explosion_camera_shake(
                    explosion.center,
                    explosion.radius,
                    player_transform.translation,
                    &mut camera_shake,
                );
            }

            apply_explosion_damage_to_targets(
                player_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                enemy_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                pedestrian_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                police_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                vehicle_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                Some(entity),
            );
        }

        if explosion.lifetime >= explosion.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
        }
    }
}

/// 車輛損壞粒子更新系統
/// 處理煙霧和火焰粒子的移動和刪除
pub fn vehicle_damage_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut smoke_query: Query<(Entity, &mut VehicleDamageSmoke, &mut Transform)>,
    mut fire_query: Query<(Entity, &mut VehicleFire, &mut Transform), Without<VehicleDamageSmoke>>,
) {
    let dt = time.delta_secs();

    // 更新煙霧粒子
    for (entity, mut smoke, mut transform) in smoke_query.iter_mut() {
        smoke.lifetime += dt;

        if smoke.lifetime >= smoke.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 煙霧上飄並減速
        smoke.velocity *= 1.0 - dt * 1.5;
        transform.translation += smoke.velocity * dt;

        // 擴散變大
        let progress = smoke.lifetime / smoke.max_lifetime;
        let scale = 0.2 + progress * 0.6;
        transform.scale = Vec3::splat(scale);
    }

    // 更新火焰粒子
    for (entity, mut fire, mut transform) in fire_query.iter_mut() {
        fire.lifetime += dt;

        if fire.lifetime >= fire.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 火焰快速上飄
        transform.translation += fire.velocity * dt;

        // 閃爍效果
        let flicker = (fire.lifetime * 20.0).sin() * 0.1 + 1.0;
        transform.scale = Vec3::splat(fire.scale() * flicker);
    }
}

/// 車輛傷害事件處理系統
/// 監聽 DamageEvent 並對車輛 VehicleHealth 造成傷害
/// 這使車輛可以被子彈、爆炸等傷害
pub fn vehicle_damage_event_system(
    time: Res<Time>,
    mut damage_events: MessageReader<DamageEvent>,
    mut vehicle_query: Query<&mut VehicleHealth>,
) {
    let current_time = time.elapsed_secs();

    for event in damage_events.read() {
        // 檢查目標是否是有 VehicleHealth 的車輛
        if let Ok(mut health) = vehicle_query.get_mut(event.target) {
            // 已爆炸的車輛不處理
            if health.damage_state == VehicleDamageState::Destroyed {
                continue;
            }

            // 對車輛造成傷害
            let damage_dealt = health.take_damage(event.amount, current_time);

            if damage_dealt > 0.0 {
                // 可以在這裡添加車輛受傷的視覺/音效回饋
                // 例如金屬撞擊音效
            }
        }
    }
}

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- VehicleDamageState ---

    #[test]
    fn damage_state_from_health_percent_boundaries() {
        assert_eq!(VehicleDamageState::from_health_percent(1.0), VehicleDamageState::Pristine);
        assert_eq!(VehicleDamageState::from_health_percent(1.5), VehicleDamageState::Pristine);
        assert_eq!(VehicleDamageState::from_health_percent(0.75), VehicleDamageState::Light);
        assert_eq!(VehicleDamageState::from_health_percent(0.50), VehicleDamageState::Moderate);
        assert_eq!(VehicleDamageState::from_health_percent(0.25), VehicleDamageState::Heavy);
        assert_eq!(VehicleDamageState::from_health_percent(0.10), VehicleDamageState::Critical);
        assert_eq!(VehicleDamageState::from_health_percent(0.0), VehicleDamageState::Destroyed);
    }

    // --- VehicleHealth ---

    #[test]
    fn health_new_sets_max_and_current() {
        let h = VehicleHealth::new(500.0);
        assert_eq!(h.current, 500.0);
        assert_eq!(h.max, 500.0);
        assert_eq!(h.percentage(), 1.0);
        assert!(!h.is_destroyed());
    }

    #[test]
    fn health_for_vehicle_type_correct_values() {
        assert_eq!(VehicleHealth::for_vehicle_type(VehicleType::Scooter).max, 500.0);
        assert_eq!(VehicleHealth::for_vehicle_type(VehicleType::Car).max, 1000.0);
        assert_eq!(VehicleHealth::for_vehicle_type(VehicleType::Taxi).max, 1200.0);
        assert_eq!(VehicleHealth::for_vehicle_type(VehicleType::Bus).max, 2000.0);
    }

    #[test]
    fn health_take_damage_reduces_hp() {
        let mut h = VehicleHealth::new(100.0);
        let actual = h.take_damage(30.0, 1.0);
        assert!((actual - 30.0).abs() < f32::EPSILON);
        assert!((h.current - 70.0).abs() < f32::EPSILON);
        assert!((h.percentage() - 0.7).abs() < 0.01);
    }

    #[test]
    fn health_take_damage_clamps_to_zero() {
        let mut h = VehicleHealth::new(50.0);
        let actual = h.take_damage(200.0, 1.0);
        assert!((actual - 50.0).abs() < f32::EPSILON);
        assert_eq!(h.current, 0.0);
        assert_eq!(h.damage_state, VehicleDamageState::Destroyed);
    }

    #[test]
    fn health_take_damage_invulnerable_returns_zero() {
        let mut h = VehicleHealth::new(100.0);
        h.is_invulnerable = true;
        let actual = h.take_damage(50.0, 1.0);
        assert_eq!(actual, 0.0);
        assert_eq!(h.current, 100.0);
    }

    #[test]
    fn health_take_damage_critical_triggers_fire() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(80.0, 1.0);
        assert!(h.is_on_fire);
        assert_eq!(h.damage_state, VehicleDamageState::Critical);
    }

    #[test]
    fn health_repair_increases_hp() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(60.0, 1.0);
        h.repair(30.0);
        assert!((h.current - 70.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_repair_clamps_to_max() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(20.0, 1.0);
        h.repair(500.0);
        assert_eq!(h.current, 100.0);
    }

    #[test]
    fn health_repair_extinguishes_fire_above_threshold() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(85.0, 1.0);
        assert!(h.is_on_fire);
        h.repair(60.0);
        assert!(!h.is_on_fire);
        assert!(h.percentage() > 0.3);
    }

    #[test]
    fn health_repair_does_nothing_when_destroyed() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(100.0, 1.0);
        assert!(h.is_destroyed());
        h.repair(50.0);
        assert_eq!(h.current, 0.0);
    }

    #[test]
    fn health_full_repair_restores_everything() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(80.0, 1.0);
        h.full_repair();
        assert_eq!(h.current, 100.0);
        assert_eq!(h.damage_state, VehicleDamageState::Pristine);
        assert!(!h.is_on_fire);
    }

    #[test]
    fn health_apply_armor_upgrade_preserves_ratio() {
        let mut h = VehicleHealth::new(100.0);
        h.take_damage(50.0, 1.0);
        assert!((h.percentage() - 0.5).abs() < 0.01);
        h.apply_armor_upgrade(1.5);
        assert!((h.max - 150.0).abs() < f32::EPSILON);
        assert!((h.percentage() - 0.5).abs() < 0.01);
    }

    #[test]
    fn health_tick_fire_countdown_explodes() {
        let mut h = VehicleHealth::new(1000.0);
        h.is_on_fire = true;
        h.fire_timer = 1.0;
        assert!(h.tick_fire(1.5));
        assert_eq!(h.damage_state, VehicleDamageState::Destroyed);
    }

    #[test]
    fn health_tick_fire_burn_damage() {
        let mut h = VehicleHealth::new(100.0);
        h.is_on_fire = true;
        h.fire_timer = 10.0;
        h.tick_fire(1.0);
        assert!((h.current - 80.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_tick_fire_not_on_fire_returns_false() {
        let mut h = VehicleHealth::new(100.0);
        assert!(!h.tick_fire(1.0));
        assert_eq!(h.current, 100.0);
    }

    // --- TireDamage ---

    #[test]
    fn tire_pop_and_count() {
        let mut td = TireDamage::default();
        assert_eq!(td.flat_count(), 0);
        td.pop_tire(0);
        assert_eq!(td.flat_count(), 1);
        assert!(td.has_front_flat());
        assert!(!td.has_rear_flat());
    }

    #[test]
    fn tire_pop_rear_detected() {
        let mut td = TireDamage::default();
        td.pop_tire(2);
        assert!(!td.has_front_flat());
        assert!(td.has_rear_flat());
    }

    #[test]
    fn tire_repair_single() {
        let mut td = TireDamage::default();
        td.pop_tire(1);
        td.pop_tire(3);
        assert_eq!(td.flat_count(), 2);
        td.repair_tire(1);
        assert_eq!(td.flat_count(), 1);
    }

    #[test]
    fn tire_repair_all_resets() {
        let mut td = TireDamage::default();
        td.pop_tire(0);
        td.pop_tire(1);
        td.pop_tire(2);
        td.repair_all();
        assert_eq!(td.flat_count(), 0);
        assert_eq!(td.handling_penalty, 0.0);
        assert_eq!(td.speed_penalty, 0.0);
    }

    #[test]
    fn tire_penalties_scale_with_count() {
        let mut td = TireDamage::default();
        td.pop_tire(0);
        assert!((td.handling_penalty - 0.15).abs() < f32::EPSILON);
        assert!((td.speed_penalty - 0.10).abs() < f32::EPSILON);
        td.pop_tire(1);
        assert!((td.handling_penalty - 0.30).abs() < f32::EPSILON);
        assert!((td.speed_penalty - 0.20).abs() < f32::EPSILON);
    }

    #[test]
    fn tire_out_of_bounds_ignored() {
        let mut td = TireDamage::default();
        td.pop_tire(99);
        assert_eq!(td.flat_count(), 0);
    }

    // --- VehicleExplosion ---

    #[test]
    fn explosion_scale_expand_then_shrink() {
        let mut e = VehicleExplosion::new(Vec3::ZERO, 5.0, 100.0);
        assert!((e.scale() - 0.0).abs() < 0.01);
        e.lifetime = 0.3;
        assert!((e.scale() - 1.5).abs() < 0.01);
        e.lifetime = 1.0;
        assert!((e.scale() - 0.0).abs() < 0.01);
    }

    #[test]
    fn explosion_alpha_fades() {
        let mut e = VehicleExplosion::new(Vec3::ZERO, 5.0, 100.0);
        assert!((e.alpha() - 1.0).abs() < f32::EPSILON);
        e.lifetime = 0.5;
        assert!((e.alpha() - 0.5).abs() < f32::EPSILON);
        e.lifetime = 1.0;
        assert!((e.alpha() - 0.0).abs() < f32::EPSILON);
    }
}
