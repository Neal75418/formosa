//! 爆炸物系統
//!
//! 手榴彈、燃燒瓶、黏性炸彈等投擲武器


use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::core::CameraSettings;
use crate::player::Player;

use super::health::*;

// ============================================================================
// 常數
// ============================================================================

/// 手榴彈投擲力道
const GRENADE_THROW_FORCE: f32 = 15.0;
/// 手榴彈引爆時間（秒）
const GRENADE_FUSE_TIME: f32 = 3.0;
/// 手榴彈爆炸半徑
const GRENADE_EXPLOSION_RADIUS: f32 = 8.0;
/// 手榴彈爆炸傷害
const GRENADE_DAMAGE: f32 = 150.0;

/// 燃燒瓶投擲力道
const MOLOTOV_THROW_FORCE: f32 = 12.0;
/// 燃燒瓶火焰半徑
const MOLOTOV_FIRE_RADIUS: f32 = 4.0;
/// 燃燒瓶火焰持續時間
const MOLOTOV_FIRE_DURATION: f32 = 8.0;
/// 燃燒瓶每秒傷害
const MOLOTOV_DPS: f32 = 15.0;

/// 黏性炸彈投擲力道
const STICKY_THROW_FORCE: f32 = 10.0;
/// 黏性炸彈爆炸半徑
const STICKY_EXPLOSION_RADIUS: f32 = 6.0;
/// 黏性炸彈爆炸傷害
const STICKY_DAMAGE: f32 = 200.0;

/// 投擲預覽線段數
const TRAJECTORY_SEGMENTS: usize = 30;
/// 投擲預覽時間步長
const TRAJECTORY_TIME_STEP: f32 = 0.05;

// ============================================================================
// 爆炸物類型
// ============================================================================

/// 爆炸物類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ExplosiveType {
    #[default]
    Grenade, // 手榴彈
    Molotov,    // 燃燒瓶
    StickyBomb, // 黏性炸彈
}

struct ExplosiveStats {
    name: &'static str,
    throw_force: f32,
    explosion_radius: f32,
    damage: f32,
}

impl ExplosiveType {
    fn stats(&self) -> ExplosiveStats {
        match self {
            ExplosiveType::Grenade => ExplosiveStats {
                name: "手榴彈",
                throw_force: GRENADE_THROW_FORCE,
                explosion_radius: GRENADE_EXPLOSION_RADIUS,
                damage: GRENADE_DAMAGE,
            },
            ExplosiveType::Molotov => ExplosiveStats {
                name: "燃燒瓶",
                throw_force: MOLOTOV_THROW_FORCE,
                explosion_radius: MOLOTOV_FIRE_RADIUS,
                damage: MOLOTOV_DPS,
            },
            ExplosiveType::StickyBomb => ExplosiveStats {
                name: "黏性炸彈",
                throw_force: STICKY_THROW_FORCE,
                explosion_radius: STICKY_EXPLOSION_RADIUS,
                damage: STICKY_DAMAGE,
            },
        }
    }

    pub fn name(&self) -> &'static str { self.stats().name }
    pub fn throw_force(&self) -> f32 { self.stats().throw_force }
    pub fn explosion_radius(&self) -> f32 { self.stats().explosion_radius }
    pub fn damage(&self) -> f32 { self.stats().damage }
}

// ============================================================================
// 組件
// ============================================================================

/// 爆炸物組件
#[derive(Component)]
pub struct Explosive {
    /// 爆炸物類型
    pub explosive_type: ExplosiveType,
    /// 引爆計時器（手榴彈用）
    pub fuse_timer: f32,
    /// 是否已啟動
    pub armed: bool,
    /// 是否已附著（黏性炸彈用）
    pub attached: bool,
    /// 附著目標實體
    pub attached_to: Option<Entity>,
    /// 投擲者
    pub thrower: Option<Entity>,
}

impl Explosive {
    /// 建立手榴彈
    pub fn grenade(thrower: Entity) -> Self {
        Self {
            explosive_type: ExplosiveType::Grenade,
            fuse_timer: GRENADE_FUSE_TIME,
            armed: true,
            attached: false,
            attached_to: None,
            thrower: Some(thrower),
        }
    }

    /// 建立汽油彈
    pub fn molotov(thrower: Entity) -> Self {
        Self {
            explosive_type: ExplosiveType::Molotov,
            fuse_timer: 0.0, // 燃燒瓶撞擊即爆
            armed: true,
            attached: false,
            attached_to: None,
            thrower: Some(thrower),
        }
    }

    /// 建立黏性炸彈
    pub fn sticky_bomb(thrower: Entity) -> Self {
        Self {
            explosive_type: ExplosiveType::StickyBomb,
            fuse_timer: 0.0, // 黏性炸彈需要手動引爆
            armed: false,
            attached: false,
            attached_to: None,
            thrower: Some(thrower),
        }
    }
}

/// 火焰區域組件（燃燒瓶產生）
#[derive(Component)]
pub struct FireZone {
    /// 半徑
    pub radius: f32,
    /// 每秒傷害
    pub damage_per_second: f32,
    /// 剩餘時間
    pub remaining_time: f32,
    /// 傷害間隔計時器
    pub damage_tick: f32,
}

impl Default for FireZone {
    fn default() -> Self {
        Self {
            radius: MOLOTOV_FIRE_RADIUS,
            damage_per_second: MOLOTOV_DPS,
            remaining_time: MOLOTOV_FIRE_DURATION,
            damage_tick: 0.0,
        }
    }
}

/// 煙霧粒子組件（GTA5 風格上升煙霧）
#[derive(Component)]
pub struct SmokeParticle {
    /// 粒子速度（主要向上）
    pub velocity: Vec3,
    /// 剩餘生命時間
    pub lifetime: f32,
    /// 最大生命時間（用於計算透明度）
    pub max_lifetime: f32,
    /// 初始大小
    pub initial_scale: f32,
    /// 最終大小（會膨脹）
    pub final_scale: f32,
}

impl SmokeParticle {
    /// 建立新實例
    pub fn new(velocity: Vec3, lifetime: f32) -> Self {
        Self {
            velocity,
            lifetime,
            max_lifetime: lifetime,
            initial_scale: 0.5,
            final_scale: 2.0,
        }
    }
}

/// 火焰粒子組件（GTA5 風格閃爍火焰）
#[derive(Component)]
pub struct FireParticle {
    /// 粒子基礎位置偏移（隨機晃動）
    pub base_offset: Vec3,
    /// 剩餘生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 閃爍相位（用於計算亮度波動）
    pub flicker_phase: f32,
    /// 閃爍速度
    pub flicker_speed: f32,
}

impl FireParticle {
    /// 建立新實例
    pub fn new(lifetime: f32) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        Self {
            base_offset: Vec3::new(
                rng.random::<f32>() * 0.5 - 0.25,
                0.0,
                rng.random::<f32>() * 0.5 - 0.25,
            ),
            lifetime,
            max_lifetime: lifetime,
            flicker_phase: rng.random::<f32>() * std::f32::consts::TAU,
            flicker_speed: 8.0 + rng.random::<f32>() * 4.0, // 8-12 Hz 閃爍
        }
    }
}

/// 煙霧發射器（持續產生煙霧粒子）
#[derive(Component)]
pub struct SmokeEmitter {
    /// 每秒發射粒子數量
    pub particles_per_second: f32,
    /// 發射累積計時
    pub spawn_accumulator: f32,
    /// 剩餘時間（0 = 永久）
    pub remaining_time: f32,
    /// 發射半徑
    pub radius: f32,
}

impl Default for SmokeEmitter {
    fn default() -> Self {
        Self {
            particles_per_second: 5.0,
            spawn_accumulator: 0.0,
            remaining_time: 0.0, // 永久
            radius: 1.0,
        }
    }
}

/// 爆炸效果組件
#[derive(Component)]
pub struct ExplosionEffect {
    /// 爆炸半徑
    pub radius: f32,
    /// 最大傷害
    pub max_damage: f32,
    /// 生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl ExplosionEffect {
    /// 建立新實例
    pub fn new(radius: f32, max_damage: f32, max_lifetime: f32) -> Self {
        Self {
            radius,
            max_damage,
            lifetime: 0.0,
            max_lifetime,
        }
    }
}

/// 衝擊波效果組件（GTA5 風格的擴散環）
#[derive(Component)]
pub struct ShockwaveEffect {
    /// 最大半徑
    pub max_radius: f32,
    /// 生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 初始透明度
    pub initial_alpha: f32,
}

impl ShockwaveEffect {
    /// 建立新實例
    pub fn new(max_radius: f32) -> Self {
        Self {
            max_radius,
            lifetime: 0.0,
            max_lifetime: 0.4, // 快速擴散消失
            initial_alpha: 0.8,
        }
    }
}

/// 玩家爆炸物庫存
#[derive(Component, Debug, Default)]
pub struct ExplosiveInventory {
    pub grenades: u32,
    pub molotovs: u32,
    pub sticky_bombs: u32,
    /// 當前選擇的爆炸物類型
    pub selected: Option<ExplosiveType>,
    /// 投擲冷卻
    pub throw_cooldown: f32,
}

impl ExplosiveInventory {
    /// 檢查是否有選定類型的爆炸物
    pub fn has_selected(&self) -> bool {
        match self.selected {
            Some(ExplosiveType::Grenade) => self.grenades > 0,
            Some(ExplosiveType::Molotov) => self.molotovs > 0,
            Some(ExplosiveType::StickyBomb) => self.sticky_bombs > 0,
            None => false,
        }
    }

    /// 消耗一個爆炸物
    pub fn consume_selected(&mut self) -> bool {
        match self.selected {
            Some(ExplosiveType::Grenade) if self.grenades > 0 => {
                self.grenades -= 1;
                true
            }
            Some(ExplosiveType::Molotov) if self.molotovs > 0 => {
                self.molotovs -= 1;
                true
            }
            Some(ExplosiveType::StickyBomb) if self.sticky_bombs > 0 => {
                self.sticky_bombs -= 1;
                true
            }
            _ => false,
        }
    }

    /// 切換到下一個爆炸物類型
    pub fn cycle_next(&mut self) {
        let types = [
            (ExplosiveType::Grenade, self.grenades),
            (ExplosiveType::Molotov, self.molotovs),
            (ExplosiveType::StickyBomb, self.sticky_bombs),
        ];

        let current_idx = self
            .selected
            .map(|s| match s {
                ExplosiveType::Grenade => 0,
                ExplosiveType::Molotov => 1,
                ExplosiveType::StickyBomb => 2,
            })
            .unwrap_or(0);

        // 找下一個有庫存的類型
        for i in 1..=3 {
            let idx = (current_idx + i) % 3;
            if types[idx].1 > 0 {
                self.selected = Some(types[idx].0);
                return;
            }
        }

        self.selected = None;
    }
}

/// 投擲預覽狀態
#[derive(Resource, Default)]
pub struct ThrowPreviewState {
    /// 是否正在預覽
    pub is_previewing: bool,
    /// 預覽軌跡點
    pub trajectory_points: Vec<Vec3>,
    /// 預計落點
    pub predicted_landing: Option<Vec3>,
    /// 投擲方向
    pub throw_direction: Vec3,
    /// 投擲力道（按住時間）
    pub charge_time: f32,
    /// 最大蓄力時間
    pub max_charge_time: f32,
}

impl ThrowPreviewState {
    /// 計算投擲力道倍率（根據蓄力時間）
    pub fn charge_multiplier(&self) -> f32 {
        (self.charge_time / self.max_charge_time).clamp(0.3, 1.0)
    }
}

// ============================================================================
// 事件
// ============================================================================

/// 爆炸事件
#[derive(Message, Clone)]
pub struct ExplosionEvent {
    /// 爆炸位置
    pub position: Vec3,
    /// 爆炸半徑
    pub radius: f32,
    /// 最大傷害
    pub max_damage: f32,
    /// 爆炸物類型
    pub explosive_type: ExplosiveType,
    /// 造成者
    pub source: Option<Entity>,
}

/// 投擲事件
#[derive(Message, Clone)]
pub struct ThrowExplosiveEvent {
    /// 投擲者
    pub thrower: Entity,
    /// 爆炸物類型
    pub explosive_type: ExplosiveType,
    /// 起始位置
    pub origin: Vec3,
    /// 投擲方向
    pub direction: Vec3,
    /// 投擲力道
    pub force: f32,
}

// ============================================================================
// 視覺資源
// ============================================================================

/// 爆炸物視覺資源
#[derive(Resource)]
pub struct ExplosiveVisuals {
    /// 手榴彈 mesh
    pub grenade_mesh: Handle<Mesh>,
    /// 手榴彈材質
    pub grenade_material: Handle<StandardMaterial>,
    /// 燃燒瓶 mesh
    pub molotov_mesh: Handle<Mesh>,
    /// 燃燒瓶材質
    pub molotov_material: Handle<StandardMaterial>,
    /// 黏性炸彈 mesh
    pub sticky_mesh: Handle<Mesh>,
    /// 黏性炸彈材質
    pub sticky_material: Handle<StandardMaterial>,
    /// 爆炸效果 mesh
    pub explosion_mesh: Handle<Mesh>,
    /// 爆炸效果材質
    pub explosion_material: Handle<StandardMaterial>,
    /// 火焰效果 mesh
    pub fire_mesh: Handle<Mesh>,
    /// 火焰效果材質
    pub fire_material: Handle<StandardMaterial>,
    /// 軌跡預覽 mesh
    pub trajectory_mesh: Handle<Mesh>,
    /// 軌跡預覽材質
    pub trajectory_material: Handle<StandardMaterial>,
    /// 衝擊波 mesh（環形）
    pub shockwave_mesh: Handle<Mesh>,
    /// 衝擊波材質
    pub shockwave_material: Handle<StandardMaterial>,
    /// 煙霧粒子 mesh
    pub smoke_mesh: Handle<Mesh>,
    /// 煙霧粒子材質
    pub smoke_material: Handle<StandardMaterial>,
    /// 火焰粒子 mesh
    pub fire_particle_mesh: Handle<Mesh>,
    /// 火焰粒子材質
    pub fire_particle_material: Handle<StandardMaterial>,
}

impl ExplosiveVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            // 手榴彈：深綠色橢圓
            grenade_mesh: meshes.add(Sphere::new(0.08)),
            grenade_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.3, 0.2),
                metallic: 0.4,
                perceptual_roughness: 0.6,
                ..default()
            }),
            // 燃燒瓶：棕色瓶子形狀
            molotov_mesh: meshes.add(Capsule3d::new(0.05, 0.15)),
            molotov_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.4, 0.3, 0.2, 0.8),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            // 黏性炸彈：紅色球體
            sticky_mesh: meshes.add(Sphere::new(0.1)),
            sticky_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.2, 0.2),
                emissive: LinearRgba::new(2.0, 0.2, 0.2, 1.0),
                ..default()
            }),
            // 爆炸效果：橙黃色發光球
            explosion_mesh: meshes.add(Sphere::new(1.0)),
            explosion_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.7, 0.3, 0.9),
                emissive: LinearRgba::new(25.0, 15.0, 5.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            // 火焰效果：紅橙色發光
            fire_mesh: meshes.add(Cylinder::new(1.0, 0.3)),
            fire_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.4, 0.1, 0.7),
                emissive: LinearRgba::new(15.0, 6.0, 1.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            // 軌跡預覽：白色半透明線
            trajectory_mesh: meshes.add(Sphere::new(0.03)),
            trajectory_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            // 衝擊波：白色半透明環形
            shockwave_mesh: meshes.add(Torus::new(0.8, 1.0)), // 內徑 0.8，外徑 1.0
            shockwave_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.95, 0.9, 0.8),
                emissive: LinearRgba::new(8.0, 6.0, 4.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None, // 雙面渲染
                ..default()
            }),
            // 煙霧粒子：深灰色半透明球
            smoke_mesh: meshes.add(Sphere::new(0.5)),
            smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.2, 0.2, 0.6),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            }),
            // 火焰粒子：橙紅色發光不規則形狀
            fire_particle_mesh: meshes.add(Sphere::new(0.3)),
            fire_particle_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.5, 0.1, 0.9),
                emissive: LinearRgba::new(20.0, 8.0, 2.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        }
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 初始化爆炸物視覺資源
pub fn setup_explosive_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ExplosiveVisuals::new(&mut meshes, &mut materials));
    commands.insert_resource(ThrowPreviewState {
        max_charge_time: 2.0,
        ..default()
    });
}

/// 投擲輸入處理系統
pub fn explosive_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    camera_settings: Res<CameraSettings>,
    mut player_query: Query<(Entity, &Transform, &mut ExplosiveInventory), With<Player>>,
    mut throw_state: ResMut<ThrowPreviewState>,
    mut throw_events: MessageWriter<ThrowExplosiveEvent>,
) {
    let Ok((player_entity, player_transform, mut inventory)) = player_query.single_mut() else {
        return;
    };

    // 切換爆炸物類型：Tab 鍵
    if keyboard.just_pressed(KeyCode::Tab) {
        inventory.cycle_next();
        if let Some(selected) = inventory.selected {
            info!("切換到: {}", selected.name());
        }
    }

    // 更新冷卻
    if inventory.throw_cooldown > 0.0 {
        inventory.throw_cooldown -= time.delta_secs();
    }

    // 沒有選擇爆炸物或冷卻中則不處理投擲
    if !inventory.has_selected() || inventory.throw_cooldown > 0.0 {
        throw_state.is_previewing = false;
        return;
    }

    let Some(selected) = inventory.selected else {
        return;
    };

    // G 鍵：投擲
    if keyboard.pressed(KeyCode::KeyG) {
        // 蓄力中
        throw_state.is_previewing = true;
        throw_state.charge_time += time.delta_secs();

        // 計算投擲方向
        let throw_dir = Vec3::new(
            camera_settings.yaw.cos(),
            camera_settings.pitch.sin().max(0.1), // 至少向上一點
            camera_settings.yaw.sin(),
        )
        .normalize();
        throw_state.throw_direction = throw_dir;

        // 計算軌跡預覽
        let force = selected.throw_force() * throw_state.charge_multiplier();
        let origin = player_transform.translation + Vec3::Y * 1.5;
        throw_state.trajectory_points = calculate_trajectory(origin, throw_dir * force);
        throw_state.predicted_landing = throw_state.trajectory_points.last().copied();
    } else if keyboard.just_released(KeyCode::KeyG) && throw_state.is_previewing {
        // 釋放：投擲
        let force = selected.throw_force() * throw_state.charge_multiplier();
        let origin = player_transform.translation + Vec3::Y * 1.5;

        throw_events.write(ThrowExplosiveEvent {
            thrower: player_entity,
            explosive_type: selected,
            origin,
            direction: throw_state.throw_direction,
            force,
        });

        inventory.consume_selected();
        inventory.throw_cooldown = 0.5; // 投擲冷卻

        // 重置預覽狀態
        throw_state.is_previewing = false;
        throw_state.charge_time = 0.0;
        throw_state.trajectory_points.clear();
    }
}

/// 計算投擲軌跡
fn calculate_trajectory(origin: Vec3, initial_velocity: Vec3) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(TRAJECTORY_SEGMENTS);
    let gravity = Vec3::new(0.0, -9.81, 0.0);

    let mut pos = origin;
    let mut vel = initial_velocity;

    for _ in 0..TRAJECTORY_SEGMENTS {
        points.push(pos);
        vel += gravity * TRAJECTORY_TIME_STEP;
        pos += vel * TRAJECTORY_TIME_STEP;

        // 如果碰到地面就停止
        if pos.y < 0.1 {
            pos.y = 0.1;
            points.push(pos);
            break;
        }
    }

    points
}

/// 處理投擲事件
pub fn handle_throw_event_system(
    mut commands: Commands,
    mut throw_events: MessageReader<ThrowExplosiveEvent>,
    visuals: Option<Res<ExplosiveVisuals>>,
) {
    let Some(visuals) = visuals else {
        return;
    };

    for event in throw_events.read() {
        let (mesh, material, explosive) = match event.explosive_type {
            ExplosiveType::Grenade => (
                visuals.grenade_mesh.clone(),
                visuals.grenade_material.clone(),
                Explosive::grenade(event.thrower),
            ),
            ExplosiveType::Molotov => (
                visuals.molotov_mesh.clone(),
                visuals.molotov_material.clone(),
                Explosive::molotov(event.thrower),
            ),
            ExplosiveType::StickyBomb => (
                visuals.sticky_mesh.clone(),
                visuals.sticky_material.clone(),
                Explosive::sticky_bomb(event.thrower),
            ),
        };

        // 生成爆炸物實體
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(event.origin),
            RigidBody::Dynamic,
            Collider::ball(0.08),
            Restitution::coefficient(0.3),
            Friction::coefficient(0.5),
            ExternalImpulse {
                impulse: event.direction * event.force,
                ..default()
            },
            CollisionGroups::new(Group::GROUP_3, Group::ALL),
            explosive,
        ));

        info!("投擲 {}", event.explosive_type.name());
    }
}

// ============================================================================
// 爆炸物更新輔助函數
// ============================================================================
/// 更新手榴彈：倒數計時並引爆
/// 返回 true 表示已引爆需要銷毀實體
#[inline]
fn update_grenade(
    explosive: &mut Explosive,
    delta_secs: f32,
    position: Vec3,
    explosion_events: &mut MessageWriter<ExplosionEvent>,
) -> bool {
    if !explosive.armed {
        return false;
    }

    explosive.fuse_timer -= delta_secs;
    if explosive.fuse_timer > 0.0 {
        return false;
    }

    // 引爆
    explosion_events.write(ExplosionEvent {
        position,
        radius: GRENADE_EXPLOSION_RADIUS,
        max_damage: GRENADE_DAMAGE,
        explosive_type: ExplosiveType::Grenade,
        source: explosive.thrower,
    });
    true
}

/// 更新燃燒瓶：撞擊即爆
/// 返回 true 表示已引爆需要銷毀實體
#[inline]
fn update_molotov(
    explosive: &Explosive,
    colliding: Option<&CollidingEntities>,
    position: Vec3,
    explosion_events: &mut MessageWriter<ExplosionEvent>,
) -> bool {
    if !explosive.armed {
        return false;
    }

    let Some(colliding) = colliding else {
        return false;
    };
    if colliding.is_empty() {
        return false;
    }

    // 撞擊地面或物體
    explosion_events.write(ExplosionEvent {
        position,
        radius: MOLOTOV_FIRE_RADIUS,
        max_damage: MOLOTOV_DPS,
        explosive_type: ExplosiveType::Molotov,
        source: explosive.thrower,
    });
    true
}

/// 更新黏性炸彈：檢查並附著到目標
/// 返回 true 表示已附著，需要移除物理組件
#[inline]
fn update_sticky_bomb(explosive: &mut Explosive, colliding: Option<&CollidingEntities>) -> bool {
    if explosive.attached {
        return false;
    }

    let Some(colliding) = colliding else {
        return false;
    };
    let Some(attached_entity) = colliding.iter().next() else {
        return false;
    };

    explosive.attached = true;
    explosive.attached_to = Some(attached_entity);
    true
}

/// 爆炸物更新系統
pub fn explosive_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut explosive_query: Query<(
        Entity,
        &Transform,
        &mut Explosive,
        Option<&CollidingEntities>,
    )>,
    mut explosion_events: MessageWriter<ExplosionEvent>,
) {
    let delta_secs = time.delta_secs();

    for (entity, transform, mut explosive, colliding) in &mut explosive_query {
        let position = transform.translation;

        match explosive.explosive_type {
            ExplosiveType::Grenade => {
                if update_grenade(&mut explosive, delta_secs, position, &mut explosion_events) {
                    commands.entity(entity).despawn();
                }
            }
            ExplosiveType::Molotov => {
                if update_molotov(&explosive, colliding, position, &mut explosion_events) {
                    commands.entity(entity).despawn();
                }
            }
            ExplosiveType::StickyBomb => {
                if update_sticky_bomb(&mut explosive, colliding) {
                    // 移除物理，附著到目標
                    commands
                        .entity(entity)
                        .remove::<RigidBody>()
                        .remove::<ExternalImpulse>();
                }
            }
        }
    }
}

/// 引爆黏性炸彈系統
pub fn detonate_sticky_bomb_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    sticky_query: Query<(Entity, &Transform, &Explosive)>,
    mut explosion_events: MessageWriter<ExplosionEvent>,
) {
    // H 鍵：引爆所有已附著的黏性炸彈
    if keyboard.just_pressed(KeyCode::KeyH) {
        for (entity, transform, explosive) in &sticky_query {
            if explosive.explosive_type == ExplosiveType::StickyBomb && explosive.attached {
                explosion_events.write(ExplosionEvent {
                    position: transform.translation,
                    radius: STICKY_EXPLOSION_RADIUS,
                    max_damage: STICKY_DAMAGE,
                    explosive_type: ExplosiveType::StickyBomb,
                    source: explosive.thrower,
                });
                commands.entity(entity).despawn();
                info!("引爆黏性炸彈!");
            }
        }
    }
}

/// 處理爆炸事件
pub fn handle_explosion_event_system(
    mut commands: Commands,
    mut explosion_events: MessageReader<ExplosionEvent>,
    visuals: Option<Res<ExplosiveVisuals>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut damage_events: MessageWriter<DamageEvent>,
    damageable_query: Query<(Entity, &Transform), With<Damageable>>,
    rapier_context: ReadRapierContext,
) {
    let Some(visuals) = visuals else {
        return;
    };
    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    for event in explosion_events.read() {
        let position = event.position;

        // 範圍傷害（含牆壁遮擋檢測）
        for (target, target_transform) in &damageable_query {
            let target_pos = target_transform.translation;
            let distance = position.distance(target_pos);

            if distance < event.radius && distance > 0.1 {
                // 檢查是否有牆壁遮擋（raycast）
                // 從稍微上方發射射線（避免地面干擾）
                let ray_origin = position + Vec3::Y * 0.5;
                let ray_target = target_pos + Vec3::Y * 0.5;
                let ray_dir = (ray_target - ray_origin).normalize();
                let max_toi = distance;

                // 使用 solid=true 檢測第一個障礙物
                let filter = QueryFilter::default().exclude_collider(target); // 排除目標本身

                let has_obstacle = rapier
                    .cast_ray(ray_origin, ray_dir, max_toi, true, filter)
                    .is_some();

                // 只有沒有障礙物時才造成傷害
                if !has_obstacle {
                    // 傷害隨距離衰減（平方根曲線，中距離傷害更高）
                    let damage_ratio = (1.0 - (distance / event.radius).sqrt()).max(0.0);
                    let damage = event.max_damage * damage_ratio;

                    damage_events.write(DamageEvent {
                        target,
                        amount: damage,
                        source: DamageSource::Explosion,
                        attacker: event.source,
                        hit_position: Some(target_pos),
                        is_headshot: false,
                    });
                }
            }
        }

        // 生成爆炸視覺效果
        match event.explosive_type {
            ExplosiveType::Molotov => {
                // 燃燒瓶：生成火焰區域 + 煙霧發射器
                commands.spawn((
                    Mesh3d(visuals.fire_mesh.clone()),
                    MeshMaterial3d(visuals.fire_material.clone()),
                    Transform::from_translation(position).with_scale(Vec3::new(
                        event.radius,
                        1.0,
                        event.radius,
                    )),
                    FireZone::default(),
                    SmokeEmitter {
                        particles_per_second: 8.0, // 火焰產生較多煙霧
                        remaining_time: MOLOTOV_FIRE_DURATION,
                        radius: event.radius * 0.8,
                        ..default()
                    },
                ));

                // 生成初始火焰粒子
                spawn_fire_particles(
                    &mut commands,
                    &visuals,
                    &mut materials,
                    position,
                    event.radius,
                    5,
                );
            }
            _ => {
                // 手榴彈/黏性炸彈：生成爆炸效果
                commands.spawn((
                    Mesh3d(visuals.explosion_mesh.clone()),
                    MeshMaterial3d(visuals.explosion_material.clone()),
                    Transform::from_translation(position),
                    ExplosionEffect::new(event.radius, event.max_damage, 0.5),
                ));

                // 生成衝擊波效果（GTA5 風格的擴散環）
                // 每個衝擊波需要獨立的材質實例，避免多個衝擊波共享材質導致視覺錯誤
                let shockwave_material = {
                    let base_mat = materials.get(&visuals.shockwave_material).cloned();
                    base_mat
                        .map(|m| materials.add(m))
                        .unwrap_or_else(|| visuals.shockwave_material.clone())
                };

                commands.spawn((
                    Mesh3d(visuals.shockwave_mesh.clone()),
                    MeshMaterial3d(shockwave_material),
                    Transform::from_translation(position + Vec3::Y * 0.1)  // 稍微抬高避免地面穿透
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),  // 水平放置
                    ShockwaveEffect::new(event.radius * 1.5),  // 衝擊波比爆炸半徑大 50%
                ));

                // 生成爆炸煙霧粒子（GTA5 風格）
                spawn_smoke_particles(
                    &mut commands,
                    &visuals,
                    &mut materials,
                    position,
                    event.radius,
                    8,
                );
            }
        }

        info!("{} 爆炸於 {:?}", event.explosive_type.name(), position);
    }
}

/// 生成煙霧粒子（輔助函數）
/// 每個粒子使用獨立材質避免共享修改問題
fn spawn_smoke_particles(
    commands: &mut Commands,
    visuals: &ExplosiveVisuals,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    radius: f32,
    count: usize,
) {
    use rand::Rng;
    let mut rng = rand::rng();

    // 取得基礎材質用於複製
    let base_material = materials.get(&visuals.smoke_material).cloned();

    for _ in 0..count {
        // 隨機位置（在爆炸半徑內）
        let offset = Vec3::new(
            rng.random::<f32>() * radius - radius * 0.5,
            rng.random::<f32>() * radius * 0.5, // 偏上方
            rng.random::<f32>() * radius - radius * 0.5,
        );

        // 隨機向上速度（帶一點水平擴散）
        let velocity = Vec3::new(
            rng.random::<f32>() * 2.0 - 1.0,
            3.0 + rng.random::<f32>() * 2.0, // 主要向上
            rng.random::<f32>() * 2.0 - 1.0,
        );

        let lifetime = 2.0 + rng.random::<f32>() * 1.5;

        // 每個粒子創建獨立材質
        let particle_material = base_material
            .clone()
            .map(|m| materials.add(m))
            .unwrap_or_else(|| visuals.smoke_material.clone());

        commands.spawn((
            Mesh3d(visuals.smoke_mesh.clone()),
            MeshMaterial3d(particle_material),
            Transform::from_translation(position + offset)
                .with_scale(Vec3::splat(0.5 + rng.random::<f32>() * 0.3)),
            SmokeParticle::new(velocity, lifetime),
        ));
    }
}

/// 生成火焰粒子（輔助函數）
/// 每個粒子使用獨立材質避免共享修改問題
fn spawn_fire_particles(
    commands: &mut Commands,
    visuals: &ExplosiveVisuals,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    radius: f32,
    count: usize,
) {
    use rand::Rng;
    let mut rng = rand::rng();

    // 取得基礎材質用於複製
    let base_material = materials.get(&visuals.fire_particle_material).cloned();

    for _ in 0..count {
        // 隨機位置（在火焰區域內）
        let offset = Vec3::new(
            rng.random::<f32>() * radius - radius * 0.5,
            rng.random::<f32>() * 0.5, // 貼近地面
            rng.random::<f32>() * radius - radius * 0.5,
        );

        let lifetime = 0.5 + rng.random::<f32>() * 0.5; // 火焰粒子短命

        // 每個粒子創建獨立材質
        let particle_material = base_material
            .clone()
            .map(|m| materials.add(m))
            .unwrap_or_else(|| visuals.fire_particle_material.clone());

        commands.spawn((
            Mesh3d(visuals.fire_particle_mesh.clone()),
            MeshMaterial3d(particle_material),
            Transform::from_translation(position + offset)
                .with_scale(Vec3::splat(0.3 + rng.random::<f32>() * 0.2)),
            FireParticle::new(lifetime),
        ));
    }
}

/// 爆炸效果更新系統
pub fn explosion_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_query: Query<(Entity, &mut Transform, &mut ExplosionEffect)>,
) {
    for (entity, mut transform, mut effect) in &mut effect_query {
        effect.lifetime += time.delta_secs();

        // 爆炸擴散然後縮小（防止除零）
        let progress = if effect.max_lifetime > 0.0 {
            (effect.lifetime / effect.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let scale = if progress < 0.3 {
            // 快速擴張
            effect.radius * (progress / 0.3)
        } else {
            // 緩慢消失
            effect.radius * (1.0 - (progress - 0.3) / 0.7)
        };
        transform.scale = Vec3::splat(scale.max(0.01));

        if effect.lifetime >= effect.max_lifetime {
            commands.entity(entity).despawn();
        }
    }
}

/// 衝擊波效果更新系統（GTA5 風格擴散環）
pub fn shockwave_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_query: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
        &mut ShockwaveEffect,
    )>,
) {
    for (entity, mut transform, material_handle, mut effect) in &mut effect_query {
        effect.lifetime += time.delta_secs();

        // 防止除零
        let progress = if effect.max_lifetime > 0.0 {
            (effect.lifetime / effect.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };

        // 線性擴張
        let scale = effect.max_radius * progress;
        // 保持環的厚度不變，只擴大半徑
        transform.scale = Vec3::new(scale.max(0.1), scale.max(0.1), 0.15);

        // 更新透明度（快速淡出）
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let alpha = effect.initial_alpha * (1.0 - progress * progress); // 二次方淡出
            material.base_color = Color::srgba(1.0, 0.95, 0.9, alpha);
            // 減弱發光
            let emissive_strength = 8.0 * (1.0 - progress);
            material.emissive = LinearRgba::new(
                emissive_strength,
                emissive_strength * 0.75,
                emissive_strength * 0.5,
                1.0,
            );
        }

        if effect.lifetime >= effect.max_lifetime {
            commands.entity(entity).despawn();
        }
    }
}

/// 火焰區域更新系統
pub fn fire_zone_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut fire_query: Query<(Entity, &Transform, &mut FireZone)>,
    mut damage_events: MessageWriter<DamageEvent>,
    damageable_query: Query<(Entity, &Transform), With<Damageable>>,
) {
    for (fire_entity, fire_transform, mut fire) in &mut fire_query {
        fire.remaining_time -= time.delta_secs();
        fire.damage_tick -= time.delta_secs();

        if fire.remaining_time <= 0.0 {
            commands.entity(fire_entity).despawn();
            continue;
        }

        // 每 0.5 秒造成一次傷害
        if fire.damage_tick <= 0.0 {
            fire.damage_tick = 0.5;

            let fire_pos = fire_transform.translation;
            let radius_sq = fire.radius * fire.radius;
            let damage = fire.damage_per_second * 0.5; // 半秒傷害（預計算）

            for (target, target_transform) in &damageable_query {
                // 使用距離平方避免 sqrt 計算
                let distance_sq = fire_pos.distance_squared(target_transform.translation);
                if distance_sq < radius_sq {
                    damage_events.write(DamageEvent {
                        target,
                        amount: damage,
                        source: DamageSource::Fire,
                        attacker: None,
                        hit_position: Some(target_transform.translation),
                        is_headshot: false,
                    });
                }
            }
        }
    }
}

/// 煙霧粒子更新系統（GTA5 風格上升漸散煙霧）
pub fn smoke_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut smoke_query: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
        &mut SmokeParticle,
    )>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, material_handle, mut smoke) in &mut smoke_query {
        smoke.lifetime -= dt;

        if smoke.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 更新位置（向上飄動）
        transform.translation += smoke.velocity * dt;

        // 隨時間減慢速度（空氣阻力）
        // 使用 powf 確保幀率無關：0.98^60 ≈ 0.3 每秒
        smoke.velocity *= 0.98_f32.powf(dt * 60.0);

        // 計算進度（0 = 剛生成，1 = 即將消失）
        let progress = 1.0 - smoke.lifetime / smoke.max_lifetime;

        // 膨脹效果：煙霧隨時間變大
        let scale = smoke.initial_scale + (smoke.final_scale - smoke.initial_scale) * progress;
        transform.scale = Vec3::splat(scale);

        // 更新透明度（漸漸消失）
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let alpha = 0.6 * (1.0 - progress * progress); // 二次方淡出
            let gray = 0.2 + 0.1 * progress; // 顏色漸淺
            material.base_color = Color::srgba(gray, gray, gray, alpha);
        }
    }
}

/// 火焰粒子更新系統（GTA5 風格閃爍火焰）
pub fn fire_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut fire_query: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
        &mut FireParticle,
    )>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    for (entity, mut transform, material_handle, mut fire) in &mut fire_query {
        fire.lifetime -= dt;

        if fire.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 計算進度
        let progress = 1.0 - fire.lifetime / fire.max_lifetime;

        // 閃爍效果
        let flicker = (t * fire.flicker_speed + fire.flicker_phase).sin() * 0.5 + 0.5;
        let scale_factor = 0.8 + flicker * 0.4; // 0.8 ~ 1.2

        // 火焰向上飄動並縮小
        transform.translation.y += dt * (1.0 + flicker);
        transform.translation += fire.base_offset * dt * 0.5; // 輕微水平晃動

        let scale = (1.0 - progress * 0.5) * scale_factor;
        transform.scale = Vec3::splat(scale.max(0.1));

        // 更新發光強度（閃爍）
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let intensity = 20.0 * flicker * (1.0 - progress);
            material.emissive = LinearRgba::new(intensity, intensity * 0.4, intensity * 0.1, 1.0);

            // 顏色從橙黃變紅（燃燒後期）
            let r = 1.0;
            let g = 0.5 - progress * 0.3;
            let b = 0.1 - progress * 0.05;
            let alpha = 0.9 * (1.0 - progress);
            material.base_color = Color::srgba(r, g.max(0.1), b.max(0.05), alpha);
        }
    }
}

/// 煙霧發射器更新系統（持續產生煙霧）
pub fn smoke_emitter_update_system(
    mut commands: Commands,
    time: Res<Time>,
    visuals: Option<Res<ExplosiveVisuals>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut emitter_query: Query<(&Transform, &mut SmokeEmitter)>,
) {
    let Some(visuals) = visuals else {
        return;
    };
    let dt = time.delta_secs();

    for (transform, mut emitter) in &mut emitter_query {
        // 更新剩餘時間
        if emitter.remaining_time > 0.0 {
            emitter.remaining_time -= dt;
            if emitter.remaining_time <= 0.0 {
                continue; // 發射器已過期
            }
        }

        // 累積發射計時
        emitter.spawn_accumulator += dt * emitter.particles_per_second;

        // 發射新粒子
        while emitter.spawn_accumulator >= 1.0 {
            emitter.spawn_accumulator -= 1.0;
            spawn_smoke_particles(
                &mut commands,
                &visuals,
                &mut materials,
                transform.translation,
                emitter.radius,
                1,
            );
        }
    }
}

/// 投擲預覽渲染系統
pub fn throw_preview_render_system(
    mut commands: Commands,
    throw_state: Res<ThrowPreviewState>,
    visuals: Option<Res<ExplosiveVisuals>>,
    preview_query: Query<Entity, With<TrajectoryPreviewPoint>>,
) {
    // 清除舊的預覽點
    for entity in &preview_query {
        commands.entity(entity).despawn();
    }

    if !throw_state.is_previewing {
        return;
    }

    let Some(visuals) = visuals else {
        return;
    };

    // 生成新的預覽點
    for (i, &point) in throw_state.trajectory_points.iter().enumerate() {
        // 每隔幾個點顯示一個
        if i % 2 == 0 {
            commands.spawn((
                Mesh3d(visuals.trajectory_mesh.clone()),
                MeshMaterial3d(visuals.trajectory_material.clone()),
                Transform::from_translation(point),
                TrajectoryPreviewPoint,
            ));
        }
    }
}

/// 軌跡預覽點標記
#[derive(Component)]
pub struct TrajectoryPreviewPoint;
