//! 爆炸物系統
//!
//! 手榴彈、燃燒瓶、黏性炸彈等投擲武器

mod effects;
mod explosion;
mod systems;

use bevy::prelude::*;

pub use effects::{
    explosion_effect_update_system, fire_particle_update_system, shockwave_effect_update_system,
    smoke_emitter_update_system, smoke_particle_update_system, throw_preview_render_system,
};
pub use explosion::{fire_zone_update_system, handle_explosion_event_system};
pub use systems::{
    detonate_sticky_bomb_system, explosive_input_system, explosive_update_system,
    handle_throw_event_system,
};

// ============================================================================
// 常數
// ============================================================================

/// 手榴彈投擲力道
pub(crate) const GRENADE_THROW_FORCE: f32 = 15.0;
/// 手榴彈引爆時間（秒）
pub(crate) const GRENADE_FUSE_TIME: f32 = 3.0;
/// 手榴彈爆炸半徑
pub(crate) const GRENADE_EXPLOSION_RADIUS: f32 = 8.0;
/// 手榴彈爆炸傷害
pub(crate) const GRENADE_DAMAGE: f32 = 150.0;

/// 燃燒瓶投擲力道
pub(crate) const MOLOTOV_THROW_FORCE: f32 = 12.0;
/// 燃燒瓶火焰半徑
pub(crate) const MOLOTOV_FIRE_RADIUS: f32 = 4.0;
/// 燃燒瓶火焰持續時間
pub(crate) const MOLOTOV_FIRE_DURATION: f32 = 8.0;
/// 燃燒瓶每秒傷害
pub(crate) const MOLOTOV_DPS: f32 = 15.0;

/// 黏性炸彈投擲力道
pub(crate) const STICKY_THROW_FORCE: f32 = 10.0;
/// 黏性炸彈爆炸半徑
pub(crate) const STICKY_EXPLOSION_RADIUS: f32 = 6.0;
/// 黏性炸彈爆炸傷害
pub(crate) const STICKY_DAMAGE: f32 = 200.0;

/// 投擲預覽線段數
pub(crate) const TRAJECTORY_SEGMENTS: usize = 30;
/// 投擲預覽時間步長
pub(crate) const TRAJECTORY_TIME_STEP: f32 = 0.05;
/// 投擲冷卻時間（秒）
pub(crate) const THROW_COOLDOWN: f32 = 0.5;
/// 衝擊波最大存活時間（秒）
pub(crate) const SHOCKWAVE_MAX_LIFETIME: f32 = 0.4;
/// 衝擊波初始透明度
pub(crate) const SHOCKWAVE_INITIAL_ALPHA: f32 = 0.8;

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
            max_lifetime: SHOCKWAVE_MAX_LIFETIME,
            initial_alpha: SHOCKWAVE_INITIAL_ALPHA,
        }
    }
}

/// 軌跡預覽點標記
#[derive(Component)]
pub struct TrajectoryPreviewPoint;

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
// 設置系統
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
