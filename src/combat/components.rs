//! 戰鬥系統組件
//!
//! 定義武器、子彈、傷害等戰鬥相關的組件和事件。

#![allow(dead_code)] // Phase 2+ 功能預留
#![allow(clippy::upper_case_acronyms)] // SMG 等縮寫保持大寫
#![allow(clippy::trivially_copy_pass_by_ref)] // enum 方法使用 &self 更一致
#![allow(clippy::match_same_arms)] // 故意為相似武器使用相同圖標
#![allow(clippy::doc_markdown)] // 中文文檔混合類型名稱
#![allow(clippy::struct_excessive_bools)] // 輸入狀態結構需要多個布爾值

use bevy::prelude::*;

// ============================================================================
// 武器類型與數據
// ============================================================================

/// 武器類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum WeaponType {
    #[default]
    Fist,       // 拳頭（近戰）
    Staff,      // 棍棒（近戰）
    Knife,      // 刀（近戰）
    Pistol,     // 手槍
    SMG,        // 衝鋒槍
    Shotgun,    // 霰彈槍
    Rifle,      // 步槍
}

impl WeaponType {
    /// 取得武器名稱
    pub fn name(&self) -> &'static str {
        match self {
            WeaponType::Fist => "拳頭",
            WeaponType::Staff => "棍棒",
            WeaponType::Knife => "刀",
            WeaponType::Pistol => "手槍",
            WeaponType::SMG => "衝鋒槍",
            WeaponType::Shotgun => "霰彈槍",
            WeaponType::Rifle => "步槍",
        }
    }

    /// 取得武器圖示
    pub fn icon(&self) -> &'static str {
        match self {
            WeaponType::Fist => "👊",
            WeaponType::Staff => "🏏",
            WeaponType::Knife => "🔪",
            WeaponType::Pistol => "🔫",
            WeaponType::SMG => "🔫",
            WeaponType::Shotgun => "🎯",
            WeaponType::Rifle => "🎯",
        }
    }

    /// 取得彈道風格索引（對應 CombatVisuals 的材質陣列）
    pub fn tracer_style(&self) -> TracerStyle {
        match self {
            WeaponType::Fist => TracerStyle::None,
            WeaponType::Staff => TracerStyle::None,
            WeaponType::Knife => TracerStyle::None,
            WeaponType::Pistol => TracerStyle::Pistol,
            WeaponType::SMG => TracerStyle::SMG,
            WeaponType::Shotgun => TracerStyle::Shotgun,
            WeaponType::Rifle => TracerStyle::Rifle,
        }
    }

    /// 是否為近戰武器
    pub fn is_melee(&self) -> bool {
        matches!(self, WeaponType::Fist | WeaponType::Staff | WeaponType::Knife)
    }
}

/// 彈道視覺風格
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TracerStyle {
    None,     // 無彈道（近戰）
    Pistol,   // 手槍：淡黃短軌跡
    SMG,      // 衝鋒槍：橙色細軌跡
    Shotgun,  // 霰彈槍：白色散射彈丸
    Rifle,    // 步槍：紅色長曳光彈
}

/// 武器數據（定義武器屬性）
#[derive(Clone, Debug)]
pub struct WeaponStats {
    pub weapon_type: WeaponType,
    pub damage: f32,           // 單發傷害
    pub fire_rate: f32,        // 射擊間隔（秒）
    pub magazine_size: u32,    // 彈匣容量
    pub max_ammo: u32,         // 最大後備彈藥
    pub range: f32,            // 有效射程（公尺）
    pub reload_time: f32,      // 換彈時間（秒）
    pub spread: f32,           // 散射角度（度）
    pub pellet_count: u32,     // 彈丸數量（霰彈槍用）
    pub bullet_speed: f32,     // 子彈速度
    pub is_automatic: bool,    // 是否全自動
    // === 距離傷害衰減 ===
    pub falloff_start: f32,    // 開始衰減距離（公尺）
    pub falloff_end: f32,      // 最低傷害距離（公尺），此距離後傷害為 25%
    // === 後座力系統 ===
    pub recoil_vertical: f32,    // 垂直後座力（向上偏移）
    pub recoil_horizontal: f32,  // 水平後座力（隨機左右偏移）
    pub recoil_recovery: f32,    // 後座力恢復速度（每秒恢復量）
}

impl WeaponStats {
    /// 手槍預設數據
    pub fn pistol() -> Self {
        Self {
            weapon_type: WeaponType::Pistol,
            damage: 25.0,
            fire_rate: 0.3,
            magazine_size: 12,
            max_ammo: 120,
            range: 50.0,
            reload_time: 1.5,
            spread: 2.0,
            pellet_count: 1,
            bullet_speed: 200.0,
            is_automatic: false,
            // 手槍：中距離衰減
            falloff_start: 30.0,
            falloff_end: 50.0,
            // 手槍：輕微後座力
            recoil_vertical: 0.02,
            recoil_horizontal: 0.01,
            recoil_recovery: 8.0,
        }
    }

    /// 衝鋒槍預設數據
    pub fn smg() -> Self {
        Self {
            weapon_type: WeaponType::SMG,
            damage: 15.0,
            fire_rate: 0.08,
            magazine_size: 30,
            max_ammo: 300,
            range: 40.0,
            reload_time: 2.0,
            spread: 5.0,
            pellet_count: 1,
            bullet_speed: 250.0,
            is_automatic: true,
            // 衝鋒槍：近距離衰減
            falloff_start: 20.0,
            falloff_end: 40.0,
            // 衝鋒槍：中等後座力，水平偏移較大
            recoil_vertical: 0.015,
            recoil_horizontal: 0.02,
            recoil_recovery: 10.0,
        }
    }

    /// 霰彈槍預設數據
    pub fn shotgun() -> Self {
        Self {
            weapon_type: WeaponType::Shotgun,
            damage: 15.0,  // 每顆彈丸 15，8 顆共 120
            fire_rate: 0.8,
            magazine_size: 8,
            max_ammo: 64,
            range: 15.0,
            reload_time: 3.0,
            spread: 15.0,
            pellet_count: 8,
            bullet_speed: 150.0,
            is_automatic: false,
            // 霰彈槍：極近距離衰減
            falloff_start: 8.0,
            falloff_end: 15.0,
            // 霰彈槍：強烈後座力
            recoil_vertical: 0.05,
            recoil_horizontal: 0.03,
            recoil_recovery: 6.0,
        }
    }

    /// 步槍預設數據
    pub fn rifle() -> Self {
        Self {
            weapon_type: WeaponType::Rifle,
            damage: 35.0,
            fire_rate: 0.15,
            magazine_size: 30,
            max_ammo: 180,
            range: 100.0,
            reload_time: 2.5,
            spread: 1.0,
            pellet_count: 1,
            bullet_speed: 400.0,
            is_automatic: true,
            // 步槍：遠距離衰減
            falloff_start: 50.0,
            falloff_end: 100.0,
            // 步槍：中等後座力，垂直為主
            recoil_vertical: 0.025,
            recoil_horizontal: 0.005,
            recoil_recovery: 7.0,
        }
    }

    /// 拳頭（近戰）
    pub fn fist() -> Self {
        Self {
            weapon_type: WeaponType::Fist,
            damage: 20.0,    // 提高傷害：3 拳殺小混混(50HP)
            fire_rate: 0.35, // 稍微加快出拳速度
            magazine_size: 0,  // 無限
            max_ammo: 0,
            range: 2.5,      // 稍微增加攻擊距離
            reload_time: 0.0,
            spread: 0.0,
            pellet_count: 1,
            bullet_speed: 0.0,
            is_automatic: false,
            // 拳頭：無距離衰減
            falloff_start: 0.0,
            falloff_end: 0.0,
            // 拳頭：無後座力
            recoil_vertical: 0.0,
            recoil_horizontal: 0.0,
            recoil_recovery: 0.0,
        }
    }

    /// 棍棒（近戰）- 弧形掃擊，可命中多目標
    pub fn staff() -> Self {
        Self {
            weapon_type: WeaponType::Staff,
            damage: 35.0,     // 高傷害
            fire_rate: 0.55,  // 較慢的揮動
            magazine_size: 0, // 無限
            max_ammo: 0,
            range: 3.2,       // 較長射程
            reload_time: 0.0,
            spread: 60.0,     // 掃擊角度（度）
            pellet_count: 1,  // 掃擊邏輯在系統中處理
            bullet_speed: 0.0,
            is_automatic: false,
            // 近戰：無距離衰減
            falloff_start: 0.0,
            falloff_end: 0.0,
            // 近戰：無後座力
            recoil_vertical: 0.0,
            recoil_horizontal: 0.0,
            recoil_recovery: 0.0,
        }
    }

    /// 刀（近戰）- 快速攻擊，有流血效果
    pub fn knife() -> Self {
        Self {
            weapon_type: WeaponType::Knife,
            damage: 28.0,     // 中等傷害
            fire_rate: 0.25,  // 快速揮刀
            magazine_size: 0, // 無限
            max_ammo: 0,
            range: 2.0,       // 短射程
            reload_time: 0.0,
            spread: 0.0,      // 單目標
            pellet_count: 1,
            bullet_speed: 0.0,
            is_automatic: false,
            // 近戰：無距離衰減
            falloff_start: 0.0,
            falloff_end: 0.0,
            // 近戰：無後座力
            recoil_vertical: 0.0,
            recoil_horizontal: 0.0,
            recoil_recovery: 0.0,
        }
    }

    /// 計算距離傷害衰減
    /// 返回實際傷害倍率 (0.25 - 1.0)
    pub fn calculate_damage_falloff(&self, distance: f32) -> f32 {
        // 近戰武器不衰減
        if self.falloff_end <= 0.0 {
            return 1.0;
        }

        if distance <= self.falloff_start {
            // 在有效射程內，滿傷害
            1.0
        } else if distance >= self.falloff_end {
            // 超過最遠距離，最低傷害
            0.25
        } else {
            // 線性衰減
            let t = (distance - self.falloff_start) / (self.falloff_end - self.falloff_start);
            1.0 - 0.75 * t
        }
    }
}

// ============================================================================
// 武器組件
// ============================================================================

/// 武器組件 - 附加到武器實體上
#[derive(Component, Clone, Debug)]
pub struct Weapon {
    pub stats: WeaponStats,
    pub current_ammo: u32,      // 當前彈匣內子彈
    pub reserve_ammo: u32,      // 後備彈藥
    pub fire_cooldown: f32,     // 射擊冷卻計時器
    pub is_reloading: bool,     // 是否正在換彈
    pub reload_timer: f32,      // 換彈計時器
}

impl Weapon {
    pub fn new(stats: WeaponStats) -> Self {
        let current_ammo = stats.magazine_size;
        let reserve_ammo = stats.max_ammo;
        Self {
            stats,
            current_ammo,
            reserve_ammo,
            fire_cooldown: 0.0,
            is_reloading: false,
            reload_timer: 0.0,
        }
    }

    /// 檢查是否可以射擊
    pub fn can_fire(&self) -> bool {
        !self.is_reloading
            && self.fire_cooldown <= 0.0
            && (self.current_ammo > 0 || self.stats.magazine_size == 0)
    }

    /// 消耗一發子彈
    pub fn consume_ammo(&mut self) {
        if self.stats.magazine_size > 0 && self.current_ammo > 0 {
            self.current_ammo -= 1;
        }
    }

    /// 開始換彈
    pub fn start_reload(&mut self) -> bool {
        if self.is_reloading || self.reserve_ammo == 0 || self.current_ammo == self.stats.magazine_size {
            return false;
        }
        self.is_reloading = true;
        self.reload_timer = self.stats.reload_time;
        true
    }

    /// 完成換彈
    pub fn finish_reload(&mut self) {
        let needed = self.stats.magazine_size - self.current_ammo;
        let to_reload = needed.min(self.reserve_ammo);
        self.current_ammo += to_reload;
        self.reserve_ammo -= to_reload;
        self.is_reloading = false;
        self.reload_timer = 0.0;
    }

    /// 是否需要換彈
    pub fn needs_reload(&self) -> bool {
        self.current_ammo == 0 && self.reserve_ammo > 0 && self.stats.magazine_size > 0
    }

    /// 取消換彈
    pub fn cancel_reload(&mut self) {
        self.is_reloading = false;
        self.reload_timer = 0.0;
    }
}

/// 玩家武器庫存
#[derive(Component, Debug)]
pub struct WeaponInventory {
    pub weapons: Vec<Weapon>,
    pub current_index: usize,
    pub max_weapons: usize,
}

impl Default for WeaponInventory {
    fn default() -> Self {
        // 預設只有拳頭
        Self {
            weapons: vec![Weapon::new(WeaponStats::fist())],
            current_index: 0,
            max_weapons: 6, // 拳頭 + 5 種武器
        }
    }
}

impl WeaponInventory {
    /// 取得當前武器
    pub fn current_weapon(&self) -> Option<&Weapon> {
        self.weapons.get(self.current_index)
    }

    /// 取得當前武器（可變）
    pub fn current_weapon_mut(&mut self) -> Option<&mut Weapon> {
        self.weapons.get_mut(self.current_index)
    }

    /// 切換到下一把武器
    pub fn next_weapon(&mut self) {
        if !self.weapons.is_empty() {
            self.current_index = (self.current_index + 1) % self.weapons.len();
        }
    }

    /// 切換到上一把武器
    pub fn prev_weapon(&mut self) {
        if !self.weapons.is_empty() {
            self.current_index = if self.current_index == 0 {
                self.weapons.len() - 1
            } else {
                self.current_index - 1
            };
        }
    }

    /// 選擇指定武器（1-based index）
    pub fn select_weapon(&mut self, slot: usize) {
        if slot > 0 && slot <= self.weapons.len() {
            self.current_index = slot - 1;
        }
    }

    /// 添加武器
    pub fn add_weapon(&mut self, weapon: Weapon) -> bool {
        // 檢查是否已有此類型武器
        for w in &mut self.weapons {
            if w.stats.weapon_type == weapon.stats.weapon_type {
                // 補充彈藥
                w.reserve_ammo = (w.reserve_ammo + weapon.reserve_ammo).min(w.stats.max_ammo);
                return true;
            }
        }

        // 添加新武器
        if self.weapons.len() < self.max_weapons {
            self.weapons.push(weapon);
            return true;
        }

        false
    }

    /// 檢查是否有指定類型武器
    pub fn has_weapon(&self, weapon_type: WeaponType) -> bool {
        self.weapons.iter().any(|w| w.stats.weapon_type == weapon_type)
    }
}

// ============================================================================
// 射擊視覺效果
// ============================================================================

/// 子彈拖尾效果標記
#[derive(Component)]
pub struct BulletTracer {
    pub start_pos: Vec3,
    pub end_pos: Vec3,
    pub lifetime: f32,
}

/// 槍口閃光標記
#[derive(Component)]
pub struct MuzzleFlash {
    pub lifetime: f32,
}

/// 擊中特效標記
#[derive(Component)]
pub struct ImpactEffect {
    pub lifetime: f32,
    pub max_lifetime: f32,
}

// ============================================================================
// 傷害系統
// ============================================================================

/// 傷害來源
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageSource {
    Bullet,           // 子彈
    Explosion,        // 爆炸
    Melee,            // 近戰
    Vehicle,          // 車輛撞擊
    Fall,             // 墜落
    Fire,             // 火焰
    Environment,      // 環境傷害
}

/// 傷害事件
#[derive(Message)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
    pub source: DamageSource,
    pub attacker: Option<Entity>,
    pub hit_position: Option<Vec3>,
    pub is_headshot: bool,
}

impl DamageEvent {
    pub fn new(target: Entity, amount: f32, source: DamageSource) -> Self {
        Self {
            target,
            amount,
            source,
            attacker: None,
            hit_position: None,
            is_headshot: false,
        }
    }

    pub fn with_attacker(mut self, attacker: Entity) -> Self {
        self.attacker = Some(attacker);
        self
    }

    pub fn with_position(mut self, position: Vec3) -> Self {
        self.hit_position = Some(position);
        self
    }

    pub fn with_headshot(mut self, is_headshot: bool) -> Self {
        self.is_headshot = is_headshot;
        self
    }
}

/// 爆頭傷害倍率
pub const HEADSHOT_MULTIPLIER: f32 = 2.5;

/// 檢查是否為爆頭（根據擊中位置和目標位置判斷）
/// 假設角色高度約 2m，頭部在 1.5m - 2.0m 之間
pub fn check_headshot(hit_position: Vec3, target_base_y: f32) -> bool {
    let head_min = target_base_y + 1.5;
    let head_max = target_base_y + 2.0;
    hit_position.y >= head_min && hit_position.y <= head_max
}

/// 死亡事件
#[derive(Message)]
pub struct DeathEvent {
    pub entity: Entity,
    pub killer: Option<Entity>,
    pub cause: DamageSource,
    pub hit_position: Option<Vec3>,   // 擊中位置（用於計算布娃娃方向）
    pub hit_direction: Option<Vec3>,  // 擊中方向（用於施加衝擊力）
}

// ============================================================================
// 流血效果（刀傷）
// ============================================================================

/// 流血效果常數
pub const BLEED_DAMAGE_PER_SECOND: f32 = 5.0;
pub const BLEED_DURATION: f32 = 4.0;
pub const BLEED_CHANCE: f32 = 0.35; // 35% 機率觸發流血

/// 流血效果組件
/// 由刀攻擊觸發，持續造成傷害
#[derive(Component)]
pub struct BleedEffect {
    /// 每秒傷害
    pub damage_per_second: f32,
    /// 剩餘時間
    pub remaining_time: f32,
    /// 攻擊者（用於歸屬擊殺）
    pub source: Option<Entity>,
    /// 下次傷害計時器
    pub tick_timer: f32,
}

impl Default for BleedEffect {
    fn default() -> Self {
        Self {
            damage_per_second: BLEED_DAMAGE_PER_SECOND,
            remaining_time: BLEED_DURATION,
            source: None,
            tick_timer: 0.0,
        }
    }
}

impl BleedEffect {
    pub fn new(source: Entity) -> Self {
        Self {
            source: Some(source),
            ..Default::default()
        }
    }

    /// 檢查流血是否結束
    pub fn is_finished(&self) -> bool {
        self.remaining_time <= 0.0
    }
}

// ============================================================================
// 近戰動畫類型
// ============================================================================

/// 近戰動畫類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MeleeAnimationType {
    #[default]
    Punch,  // 拳頭
    Swing,  // 棍棒揮擊
    Slash,  // 刀砍
    Stab,   // 刀刺
}

impl MeleeAnimationType {
    /// 從武器類型推斷動畫類型
    pub fn from_weapon(weapon_type: WeaponType) -> Self {
        match weapon_type {
            WeaponType::Fist => MeleeAnimationType::Punch,
            WeaponType::Staff => MeleeAnimationType::Swing,
            WeaponType::Knife => MeleeAnimationType::Slash,
            _ => MeleeAnimationType::Punch,
        }
    }
}

// ============================================================================
// 布娃娃系統
// ============================================================================

/// 布娃娃狀態組件
/// 當敵人死亡時，添加此組件來啟用物理布娃娃效果
#[derive(Component)]
pub struct Ragdoll {
    /// 布娃娃持續時間計時器
    pub lifetime: f32,
    /// 最大持續時間（秒）
    pub max_lifetime: f32,
    /// 是否已完成物理轉換
    pub physics_applied: bool,
    /// 衝擊力方向
    pub impulse_direction: Vec3,
    /// 衝擊力大小
    pub impulse_strength: f32,
}

impl Default for Ragdoll {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 5.0,  // 5 秒後消失
            physics_applied: false,
            impulse_direction: Vec3::NEG_Z,
            impulse_strength: 300.0,
        }
    }
}

impl Ragdoll {
    /// 創建帶方向的布娃娃
    pub fn with_impulse(direction: Vec3, strength: f32) -> Self {
        Self {
            impulse_direction: direction.normalize_or_zero(),
            impulse_strength: strength,
            ..Default::default()
        }
    }
}

// ============================================================================
// 血液粒子系統
// ============================================================================

/// 血液粒子組件
#[derive(Component)]
pub struct BloodParticle {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl BloodParticle {
    pub fn new(velocity: Vec3, max_lifetime: f32) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime,
        }
    }
}

/// 血液視覺效果資源（預生成的 mesh 和 material）
#[derive(Resource)]
pub struct BloodVisuals {
    /// 血液粒子 mesh
    pub particle_mesh: Handle<Mesh>,
    /// 血液粒子材質
    pub particle_material: Handle<StandardMaterial>,
}

impl BloodVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            particle_mesh: meshes.add(Sphere::new(0.04)),
            particle_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.0, 0.0),  // 深紅色
                emissive: LinearRgba::new(0.3, 0.0, 0.0, 1.0),
                perceptual_roughness: 0.8,
                metallic: 0.0,
                ..default()
            }),
        }
    }
}

/// 布娃娃追蹤器（限制屍體數量）
#[derive(Resource)]
pub struct RagdollTracker {
    /// 追蹤的布娃娃實體和生成時間
    pub ragdolls: Vec<(Entity, f32)>,
    /// 最大屍體數量
    pub max_count: usize,
}

impl Default for RagdollTracker {
    fn default() -> Self {
        Self {
            ragdolls: Vec::new(),
            max_count: 10,
        }
    }
}

// ============================================================================
// 生命值與護甲
// ============================================================================

/// 生命值組件（通用，可附加到任何可受傷實體）
#[derive(Component, Debug, Clone)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    pub regeneration: f32,     // 每秒回復量
    pub regen_delay: f32,      // 受傷後多久開始回復
    pub last_damage_time: f32, // 上次受傷時間
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
            regeneration: 0.0,
            regen_delay: 5.0,
            last_damage_time: 0.0,
        }
    }
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
            ..default()
        }
    }

    pub fn with_regen(mut self, regen_per_sec: f32, delay: f32) -> Self {
        self.regeneration = regen_per_sec;
        self.regen_delay = delay;
        self
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    pub fn percentage(&self) -> f32 {
        (self.current / self.max).clamp(0.0, 1.0)
    }

    pub fn take_damage(&mut self, amount: f32, time: f32) -> f32 {
        let actual = amount.min(self.current);
        self.current -= actual;
        self.last_damage_time = time;
        actual
    }

    pub fn heal(&mut self, amount: f32) -> f32 {
        let space = self.max - self.current;
        let actual = amount.min(space);
        self.current += actual;
        actual
    }
}

/// 護甲組件
#[derive(Component, Debug, Clone)]
pub struct Armor {
    pub current: f32,
    pub max: f32,
    pub damage_reduction: f32, // 傷害減免比例 (0.0 - 1.0)
}

impl Default for Armor {
    fn default() -> Self {
        Self {
            current: 0.0,
            max: 100.0,
            damage_reduction: 0.5, // 護甲吸收 50% 傷害
        }
    }
}

impl Armor {
    pub fn new(amount: f32) -> Self {
        Self {
            current: amount,
            max: 100.0,
            damage_reduction: 0.5,
        }
    }

    pub fn percentage(&self) -> f32 {
        (self.current / self.max).clamp(0.0, 1.0)
    }

    /// 處理傷害，回傳實際傳遞給生命值的傷害量
    /// 護甲優先吸收傷害，剩餘傷害再應用減免比例
    pub fn absorb_damage(&mut self, damage: f32) -> f32 {
        if self.current <= 0.0 {
            return damage;
        }

        // 護甲優先吸收傷害（1:1 吸收）
        let armor_absorption = damage.min(self.current);
        self.current -= armor_absorption;

        // 剩餘傷害應用減免比例
        let remaining_damage = damage - armor_absorption;
        remaining_damage * (1.0 - self.damage_reduction)
    }

    /// 檢查護甲是否剛剛被擊破
    pub fn is_broken(&self) -> bool {
        self.current <= 0.0
    }

    /// 檢查護甲是否受到顯著傷害（用於觸發火花特效）
    pub fn took_significant_hit(&self, damage: f32) -> bool {
        damage >= 15.0 && self.current > 0.0
    }
}

// ============================================================================
// 護甲破碎特效
// ============================================================================

/// 護甲破碎事件
#[derive(Message, Clone, Debug)]
pub struct ArmorBreakEvent {
    /// 被破甲的實體
    pub entity: Entity,
    /// 破碎位置
    pub position: Vec3,
    /// 是否完全破碎（護甲歸零）
    pub is_full_break: bool,
}

/// 護甲碎片粒子組件
#[derive(Component)]
pub struct ArmorShardParticle {
    /// 速度
    pub velocity: Vec3,
    /// 角速度
    pub angular_velocity: Vec3,
    /// 生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl ArmorShardParticle {
    pub fn new(velocity: Vec3, angular_velocity: Vec3, max_lifetime: f32) -> Self {
        Self {
            velocity,
            angular_velocity,
            lifetime: 0.0,
            max_lifetime,
        }
    }
}

/// 護甲火花粒子組件（受擊時的火花）
#[derive(Component)]
pub struct ArmorSparkParticle {
    /// 速度
    pub velocity: Vec3,
    /// 生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl ArmorSparkParticle {
    pub fn new(velocity: Vec3, max_lifetime: f32) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime,
        }
    }
}

/// 護甲特效視覺資源
#[derive(Resource)]
pub struct ArmorEffectVisuals {
    /// 碎片 Mesh
    pub shard_mesh: Handle<Mesh>,
    /// 碎片材質（金屬質感）
    pub shard_material: Handle<StandardMaterial>,
    /// 火花 Mesh
    pub spark_mesh: Handle<Mesh>,
    /// 火花材質（發光）
    pub spark_material: Handle<StandardMaterial>,
}

impl ArmorEffectVisuals {
    pub fn new(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        // 碎片 Mesh（小三角形）
        let shard_mesh = meshes.add(Cuboid::new(0.03, 0.015, 0.02));

        // 碎片材質（藍灰色金屬）
        let shard_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.5, 0.6),
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        });

        // 火花 Mesh（小球）
        let spark_mesh = meshes.add(Sphere::new(0.015));

        // 火花材質（明亮的黃/橙色發光）
        let spark_material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.2),
            emissive: LinearRgba::new(10.0, 6.0, 1.0, 1.0),
            ..default()
        });

        Self {
            shard_mesh,
            shard_material,
            spark_mesh,
            spark_material,
        }
    }
}

// ============================================================================
// 戰鬥狀態資源
// ============================================================================

/// 戰鬥狀態（全域資源）
#[derive(Resource, Default)]
pub struct CombatState {
    pub is_aiming: bool,           // 是否正在瞄準
    pub crosshair_bloom: f32,      // 準星擴散程度
    pub last_shot_time: f32,       // 上次射擊時間
    pub hit_marker_timer: f32,     // 命中標記顯示計時器
    pub hit_marker_headshot: bool, // 是否為爆頭（影響顏色）
    // === 車上射擊相關 ===
    pub can_fire_in_vehicle: bool,   // 是否可在車上射擊
    pub vehicle_aim_valid: bool,     // 車上瞄準角度是否有效
    pub last_hit_time: Option<f32>,  // 上次命中時間
}

/// 單一彈道風格配置
#[derive(Clone)]
pub struct TracerConfig {
    pub material: Handle<StandardMaterial>,
    pub mesh: Handle<Mesh>,
    pub lifetime: f32,    // 拖尾存活時間
    pub thickness: f32,   // 用於 scale 調整
}

/// 戰鬥視覺效果共用資源（避免每次射擊創建新 Mesh/Material）
#[derive(Resource)]
pub struct CombatVisuals {
    /// 各武器類型的彈道配置
    pub tracers: std::collections::HashMap<TracerStyle, TracerConfig>,
    /// 槍口閃光
    pub muzzle_material: Handle<StandardMaterial>,
    pub muzzle_mesh: Handle<Mesh>,
    /// 擊中特效（火花/塵土）
    pub impact_material: Handle<StandardMaterial>,
    pub impact_mesh: Handle<Mesh>,
}

impl CombatVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        use std::collections::HashMap;

        let mut tracers = HashMap::new();

        // 手槍：淡黃色短軌跡，較淡
        tracers.insert(TracerStyle::Pistol, TracerConfig {
            material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.9, 0.6, 0.5), // 淡黃，半透明
                emissive: LinearRgba::new(4.0, 3.5, 1.5, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            mesh: meshes.add(Capsule3d::new(0.015, 0.5)), // 很細
            lifetime: 0.08,
            thickness: 1.0,
        });

        // 衝鋒槍：橙黃色細軌跡
        tracers.insert(TracerStyle::SMG, TracerConfig {
            material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.7, 0.3, 0.7),
                emissive: LinearRgba::new(8.0, 5.0, 1.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            mesh: meshes.add(Capsule3d::new(0.02, 0.5)),
            lifetime: 0.1,
            thickness: 1.0,
        });

        // 霰彈槍：白色/灰色彈丸軌跡
        tracers.insert(TracerStyle::Shotgun, TracerConfig {
            material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.9, 0.9, 0.95, 0.6),
                emissive: LinearRgba::new(3.0, 3.0, 3.5, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            mesh: meshes.add(Capsule3d::new(0.012, 0.3)), // 更短更細（彈丸）
            lifetime: 0.06,
            thickness: 1.0,
        });

        // 步槍：紅/橙色長曳光彈（軍用曳光彈風格）
        tracers.insert(TracerStyle::Rifle, TracerConfig {
            material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.4, 0.2, 0.9),
                emissive: LinearRgba::new(15.0, 6.0, 2.0, 1.0), // 明亮的紅橙色
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            mesh: meshes.add(Capsule3d::new(0.025, 0.5)),
            lifetime: 0.18,
            thickness: 1.2,
        });

        Self {
            tracers,
            // 槍口閃光：明亮的橙黃色火光
            muzzle_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.7, 0.3, 0.9),
                emissive: LinearRgba::new(20.0, 12.0, 3.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            muzzle_mesh: meshes.add(Sphere::new(0.15)),
            // 擊中特效：橙黃色火花（比槍口閃光小）
            impact_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.8, 0.4, 0.9),
                emissive: LinearRgba::new(12.0, 8.0, 2.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            impact_mesh: meshes.add(Sphere::new(0.08)),
        }
    }

    /// 取得指定風格的彈道配置
    pub fn get_tracer(&self, style: TracerStyle) -> Option<&TracerConfig> {
        self.tracers.get(&style)
    }
}

/// 射擊輸入緩衝
#[derive(Resource, Default)]
pub struct ShootingInput {
    pub fire_pressed: bool,     // 射擊鍵按下
    pub fire_held: bool,        // 射擊鍵持續按住
    pub aim_pressed: bool,      // 瞄準鍵按住
    pub reload_pressed: bool,   // 換彈鍵按下
    pub weapon_switch: Option<usize>, // 切換武器 (1-4)
    pub mouse_wheel: f32,       // 滑鼠滾輪
}

// ============================================================================
// 可受傷標記
// ============================================================================

/// 可受傷實體標記
#[derive(Component)]
pub struct Damageable;

/// 敵人標記
#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
}

/// 敵人類型
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnemyType {
    Gangster,   // 小混混
    Thug,       // 打手
    Boss,       // 老大
}

impl EnemyType {
    pub fn health(&self) -> f32 {
        match self {
            EnemyType::Gangster => 50.0,
            EnemyType::Thug => 80.0,
            EnemyType::Boss => 200.0,
        }
    }

    pub fn weapon(&self) -> WeaponStats {
        match self {
            EnemyType::Gangster => WeaponStats::pistol(),
            EnemyType::Thug => WeaponStats::smg(),
            EnemyType::Boss => WeaponStats::shotgun(),
        }
    }
}

// ============================================================================
// 揮拳動畫
// ============================================================================

/// 玩家手臂標記（用於揮拳動畫）
#[derive(Component, Debug)]
pub struct PlayerArm {
    pub is_right: bool,           // 是否為右手臂
    pub rest_position: Vec3,      // 靜止位置
    pub rest_rotation: Quat,      // 靜止旋轉
}

impl PlayerArm {
    pub fn left(position: Vec3, rotation: Quat) -> Self {
        Self {
            is_right: false,
            rest_position: position,
            rest_rotation: rotation,
        }
    }

    pub fn right(position: Vec3, rotation: Quat) -> Self {
        Self {
            is_right: true,
            rest_position: position,
            rest_rotation: rotation,
        }
    }
}

/// 玩家手部標記（附加在手臂的子實體上）
#[derive(Component, Debug)]
pub struct PlayerHand {
    pub is_right: bool,
}

/// 武器模型標記（附加在武器視覺實體上）
#[derive(Component, Debug)]
pub struct WeaponModel {
    pub weapon_type: WeaponType,
}

/// 武器模型視覺資源（預生成的 mesh 和 material）
#[derive(Resource)]
pub struct WeaponVisuals {
    pub staff: WeaponModelData,
    pub knife: WeaponModelData,
    pub pistol: WeaponModelData,
    pub smg: WeaponModelData,
    pub shotgun: WeaponModelData,
    pub rifle: WeaponModelData,
}

/// 單一武器模型數據（用於多部件組合）
#[derive(Clone)]
pub struct WeaponModelData {
    /// 武器各部件（mesh, material, local_transform）
    pub parts: Vec<WeaponPart>,
    /// 槍口相對於武器根的偏移（本地座標）
    pub muzzle_offset: Vec3,
    /// 武器根相對於手的偏移和旋轉
    pub hand_offset: Vec3,
    pub hand_rotation: Quat,
}

/// 武器部件
#[derive(Clone)]
pub struct WeaponPart {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub transform: Transform,
}

impl WeaponVisuals {
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        // === 材質定義 ===
        // 金屬槍身（深灰/黑色）
        let gun_metal = materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.15, 0.18),
            metallic: 0.9,
            perceptual_roughness: 0.3,
            ..default()
        });
        // 槍管（更深的黑色）
        let barrel_metal = materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.08, 0.10),
            metallic: 0.95,
            perceptual_roughness: 0.2,
            ..default()
        });
        // 握把（黑色塑膠/橡膠）
        let grip_plastic = materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 0.1),
            metallic: 0.0,
            perceptual_roughness: 0.8,
            ..default()
        });
        // 木質槍托
        let wood = materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.2, 0.1),
            metallic: 0.0,
            perceptual_roughness: 0.6,
            ..default()
        });
        // 彈匣（深色金屬）
        let mag_metal = materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.12, 0.14),
            metallic: 0.7,
            perceptual_roughness: 0.5,
            ..default()
        });

        Self {
            // ========================================
            // 棍棒（木製棍棒）
            // ========================================
            staff: WeaponModelData {
                parts: vec![
                    // 棍身（主體）
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.025, 0.9)),
                        material: wood.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.0)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                    // 握把纏繞
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.028, 0.15)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.35)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.0, 0.45),
                hand_offset: Vec3::new(0.0, 0.0, -0.35),
                hand_rotation: Quat::from_rotation_x(-0.3),
            },

            // ========================================
            // 刀（戰術刀）
            // ========================================
            knife: WeaponModelData {
                parts: vec![
                    // 刀刃
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.005, 0.025, 0.18)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.12),
                    },
                    // 刀背（較厚）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.008, 0.015, 0.16)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.012, 0.11),
                    },
                    // 護手
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.04, 0.012, 0.015)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.02),
                    },
                    // 握把
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.022, 0.028, 0.10)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.04),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.0, 0.22),
                hand_offset: Vec3::new(0.0, 0.02, -0.04),
                hand_rotation: Quat::from_rotation_x(-0.1),
            },

            // ========================================
            // 手槍（Glock 風格）
            // ========================================
            pistol: WeaponModelData {
                parts: vec![
                    // 滑套（上部）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.028, 0.032, 0.16)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.016, 0.02),
                    },
                    // 槍身/握把框架
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.026, 0.08, 0.10)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.04, -0.01),
                    },
                    // 扳機護弓
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.022, 0.015, 0.04)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.008, 0.03),
                    },
                    // 槍口
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.006, 0.02)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.016, 0.09)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.016, 0.10),
                hand_offset: Vec3::new(0.0, 0.04, 0.0),
                hand_rotation: Quat::from_rotation_x(-0.2),
            },

            // ========================================
            // 衝鋒槍（UZI/MP5 風格）
            // ========================================
            smg: WeaponModelData {
                parts: vec![
                    // 機匣（主體）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.045, 0.06, 0.22)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.05),
                    },
                    // 槍管
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.012, 0.15)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.01, 0.20)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                    // 握把
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.032, 0.09, 0.04)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.06, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.2)),
                    },
                    // 彈匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.025, 0.12, 0.03)),
                        material: mag_metal.clone(),
                        transform: Transform::from_xyz(0.0, -0.08, 0.06),
                    },
                    // 摺疊槍托（簡化）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.02, 0.04, 0.12)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.10),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.01, 0.28),
                hand_offset: Vec3::new(0.0, 0.06, -0.02),
                hand_rotation: Quat::from_rotation_x(-0.15),
            },

            // ========================================
            // 霰彈槍（Remington 870 風格）
            // ========================================
            shotgun: WeaponModelData {
                parts: vec![
                    // 機匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.045, 0.055, 0.18)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    },
                    // 槍管（粗）
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.018, 0.45)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.01, 0.28)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                    // 泵動護木
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.038, 0.045, 0.12)),
                        material: wood.clone(),
                        transform: Transform::from_xyz(0.0, -0.015, 0.18),
                    },
                    // 握把
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.035, 0.08, 0.045)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.055, -0.02)
                            .with_rotation(Quat::from_rotation_x(0.25)),
                    },
                    // 槍托
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.04, 0.06, 0.22)),
                        material: wood.clone(),
                        transform: Transform::from_xyz(0.0, -0.01, -0.18),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.01, 0.52),
                hand_offset: Vec3::new(0.0, 0.055, -0.05),
                hand_rotation: Quat::from_rotation_x(-0.1),
            },

            // ========================================
            // 步槍（M4/AR-15 風格）
            // ========================================
            rifle: WeaponModelData {
                parts: vec![
                    // 上機匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.04, 0.05, 0.25)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.01, 0.05),
                    },
                    // 下機匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.038, 0.045, 0.15)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, -0.02, 0.0),
                    },
                    // 槍管
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.01, 0.35)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.015, 0.32)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                    // 護木
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.035, 0.04, 0.18)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.22),
                    },
                    // 握把
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.03, 0.08, 0.04)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.06, -0.02)
                            .with_rotation(Quat::from_rotation_x(0.3)),
                    },
                    // 彈匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.022, 0.10, 0.035)),
                        material: mag_metal.clone(),
                        transform: Transform::from_xyz(0.0, -0.07, 0.04),
                    },
                    // 槍托（伸縮）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.035, 0.05, 0.16)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.15),
                    },
                    // 提把/瞄準鏡座
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.025, 0.025, 0.08)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.045, 0.08),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.015, 0.50),
                hand_offset: Vec3::new(0.0, 0.06, -0.08),
                hand_rotation: Quat::from_rotation_x(-0.08),
            },
        }
    }

    /// 根據武器類型取得模型數據
    pub fn get(&self, weapon_type: WeaponType) -> Option<&WeaponModelData> {
        match weapon_type {
            WeaponType::Fist => None,  // 拳頭無模型
            WeaponType::Staff => Some(&self.staff),
            WeaponType::Knife => Some(&self.knife),
            WeaponType::Pistol => Some(&self.pistol),
            WeaponType::SMG => Some(&self.smg),
            WeaponType::Shotgun => Some(&self.shotgun),
            WeaponType::Rifle => Some(&self.rifle),
        }
    }
}

/// 揮拳動畫階段
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum PunchPhase {
    #[default]
    WindUp,    // 蓄力（向後拉）
    Strike,    // 出拳（向前伸）
    Return,    // 收回
}

/// 揮拳動畫組件
#[derive(Component, Debug)]
pub struct PunchAnimation {
    pub timer: f32,           // 動畫計時器
    pub duration: f32,        // 總時長
    pub phase: PunchPhase,    // 當前階段
}

impl Default for PunchAnimation {
    fn default() -> Self {
        Self {
            timer: 0.0,
            duration: 0.3,        // 0.3 秒完成
            phase: PunchPhase::WindUp,
        }
    }
}

impl PunchAnimation {
    /// 取得各階段時間佔比
    pub fn phase_times(&self) -> (f32, f32, f32) {
        // WindUp: 0-33%, Strike: 33-66%, Return: 66-100%
        let wind_up_end = self.duration * 0.33;
        let strike_end = self.duration * 0.66;
        (wind_up_end, strike_end, self.duration)
    }

    /// 取得當前進度 (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        (self.timer / self.duration).clamp(0.0, 1.0)
    }

    /// 檢查動畫是否完成
    pub fn is_finished(&self) -> bool {
        self.timer >= self.duration
    }

    /// 根據計時器更新動畫階段
    pub fn update_phase(&mut self) {
        let (wind_up_end, strike_end, duration) = self.phase_times();
        let t = self.timer;
        if t < wind_up_end {
            self.phase = PunchPhase::WindUp;
        } else if t < strike_end {
            self.phase = PunchPhase::Strike;
        } else if t < duration {
            self.phase = PunchPhase::Return;
        }
    }
}

// ============================================================================
// 敵人揮拳動畫
// ============================================================================

/// 敵人手臂標記（用於揮拳動畫）
#[derive(Component, Debug)]
pub struct EnemyArm {
    pub is_right: bool,           // 是否為右手臂
    pub rest_position: Vec3,      // 靜止位置
    pub rest_rotation: Quat,      // 靜止旋轉
}

impl EnemyArm {
    pub fn left(position: Vec3, rotation: Quat) -> Self {
        Self {
            is_right: false,
            rest_position: position,
            rest_rotation: rotation,
        }
    }

    pub fn right(position: Vec3, rotation: Quat) -> Self {
        Self {
            is_right: true,
            rest_position: position,
            rest_rotation: rotation,
        }
    }
}

/// 敵人揮拳動畫組件
#[derive(Component, Debug)]
pub struct EnemyPunchAnimation {
    pub timer: f32,           // 動畫計時器
    pub duration: f32,        // 總時長
    pub phase: PunchPhase,    // 當前階段
    pub target: Option<Entity>,    // 攻擊目標
    pub attacker: Option<Entity>,  // 攻擊者
    pub damage_dealt: bool,        // 是否已造成傷害
}

impl Default for EnemyPunchAnimation {
    fn default() -> Self {
        Self {
            timer: 0.0,
            duration: 0.35,       // 敵人出拳稍慢一點
            phase: PunchPhase::WindUp,
            target: None,
            attacker: None,
            damage_dealt: false,
        }
    }
}

impl EnemyPunchAnimation {
    /// 創建帶攻擊目標的揮拳動畫
    pub fn with_target(target: Entity, attacker: Entity) -> Self {
        Self {
            target: Some(target),
            attacker: Some(attacker),
            ..Default::default()
        }
    }
}

impl EnemyPunchAnimation {
    /// 取得各階段時間佔比
    pub fn phase_times(&self) -> (f32, f32, f32) {
        let wind_up_end = self.duration * 0.33;
        let strike_end = self.duration * 0.66;
        (wind_up_end, strike_end, self.duration)
    }

    /// 檢查動畫是否完成
    pub fn is_finished(&self) -> bool {
        self.timer >= self.duration
    }

    /// 根據計時器更新動畫階段
    pub fn update_phase(&mut self) {
        let (wind_up_end, strike_end, duration) = self.phase_times();
        let t = self.timer;
        if t < wind_up_end {
            self.phase = PunchPhase::WindUp;
        } else if t < strike_end {
            self.phase = PunchPhase::Strike;
        } else if t < duration {
            self.phase = PunchPhase::Return;
        }
    }
}

// ============================================================================
// 受傷反應系統 (GTA 5 風格)
// ============================================================================

/// 受傷反應階段
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum HitReactionPhase {
    #[default]
    None,       // 無反應
    Flinch,     // 畏縮（輕傷）
    Stagger,    // 踉蹌（中傷）
    Knockback,  // 擊退（重傷）
    Recovery,   // 恢復中
}

/// 受傷反應組件
/// 當實體受到傷害時，根據傷害量觸發不同的反應動畫
#[derive(Component, Debug)]
pub struct HitReaction {
    /// 當前反應階段
    pub phase: HitReactionPhase,
    /// 反應計時器
    pub timer: f32,
    /// 反應持續時間
    pub duration: f32,
    /// 擊退方向（標準化）
    pub knockback_direction: Vec3,
    /// 擊退速度
    pub knockback_velocity: Vec3,
    /// 視覺旋轉偏移（身體後仰）
    pub visual_rotation: Quat,
    /// 是否免疫連續擊退（硬直保護）
    pub is_immune: bool,
    /// 免疫計時器
    pub immunity_timer: f32,
}

impl Default for HitReaction {
    fn default() -> Self {
        Self {
            phase: HitReactionPhase::None,
            timer: 0.0,
            duration: 0.0,
            knockback_direction: Vec3::ZERO,
            knockback_velocity: Vec3::ZERO,
            visual_rotation: Quat::IDENTITY,
            is_immune: false,
            immunity_timer: 0.0,
        }
    }
}

impl HitReaction {
    /// 傷害門檻常數
    pub const FLINCH_THRESHOLD: f32 = 10.0;    // 10+ 傷害觸發畏縮
    pub const STAGGER_THRESHOLD: f32 = 25.0;   // 25+ 傷害觸發踉蹌
    pub const KNOCKBACK_THRESHOLD: f32 = 40.0; // 40+ 傷害觸發擊退

    /// 反應持續時間常數
    pub const FLINCH_DURATION: f32 = 0.15;
    pub const STAGGER_DURATION: f32 = 0.3;
    pub const KNOCKBACK_DURATION: f32 = 0.5;
    pub const RECOVERY_DURATION: f32 = 0.2;

    /// 免疫時間（防止連續擊退）
    pub const IMMUNITY_DURATION: f32 = 0.5;

    /// 根據傷害量和方向觸發受傷反應
    pub fn trigger(&mut self, damage: f32, hit_direction: Vec3, is_headshot: bool) {
        // 如果正在免疫，不觸發新的反應
        if self.is_immune {
            return;
        }

        // 根據傷害量決定反應類型
        let (phase, duration, knockback_speed) = if is_headshot || damage >= Self::KNOCKBACK_THRESHOLD {
            (HitReactionPhase::Knockback, Self::KNOCKBACK_DURATION, 8.0)
        } else if damage >= Self::STAGGER_THRESHOLD {
            (HitReactionPhase::Stagger, Self::STAGGER_DURATION, 4.0)
        } else if damage >= Self::FLINCH_THRESHOLD {
            (HitReactionPhase::Flinch, Self::FLINCH_DURATION, 1.5)
        } else {
            return;
        };

        self.phase = phase;
        self.duration = duration;
        self.timer = 0.0;

        // 計算擊退方向和速度
        let direction = if hit_direction.length_squared() > 0.001 {
            Vec3::new(hit_direction.x, 0.0, hit_direction.z).normalize_or_zero()
        } else {
            Vec3::NEG_Z
        };

        self.knockback_direction = direction;
        self.knockback_velocity = direction * knockback_speed;

        // 啟動免疫
        self.is_immune = true;
        self.immunity_timer = Self::IMMUNITY_DURATION;
    }

    /// 更新反應狀態，返回是否仍在反應中
    pub fn update(&mut self, delta: f32) -> bool {
        // 更新免疫計時器
        if self.is_immune {
            self.immunity_timer -= delta;
            if self.immunity_timer <= 0.0 {
                self.is_immune = false;
            }
        }

        if self.phase == HitReactionPhase::None {
            return false;
        }

        self.timer += delta;
        let progress = (self.timer / self.duration).clamp(0.0, 1.0);

        // 根據階段更新視覺效果
        match self.phase {
            HitReactionPhase::Flinch => {
                let intensity = 1.0 - (1.0 - progress).powi(2);
                let back_lean = (1.0 - intensity) * 0.15;
                self.visual_rotation = Quat::from_rotation_x(back_lean);
            }
            HitReactionPhase::Stagger => {
                let intensity = 1.0 - (1.0 - progress).powi(3);
                let back_lean = (1.0 - intensity) * 0.3;
                self.visual_rotation = Quat::from_rotation_x(back_lean);
            }
            HitReactionPhase::Knockback => {
                let intensity = 1.0 - (1.0 - progress).powi(3);
                let back_lean = (1.0 - intensity) * 0.5;
                self.visual_rotation = Quat::from_rotation_x(back_lean);
                self.knockback_velocity *= 0.92;
            }
            HitReactionPhase::Recovery => {
                let t = progress;
                let intensity = if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 };
                self.visual_rotation = Quat::slerp(self.visual_rotation, Quat::IDENTITY, intensity);
            }
            HitReactionPhase::None => {}
        }

        // 檢查是否完成當前階段
        if self.timer >= self.duration {
            match self.phase {
                HitReactionPhase::Flinch | HitReactionPhase::Stagger | HitReactionPhase::Knockback => {
                    self.phase = HitReactionPhase::Recovery;
                    self.duration = Self::RECOVERY_DURATION;
                    self.timer = 0.0;
                }
                HitReactionPhase::Recovery => {
                    self.phase = HitReactionPhase::None;
                    self.visual_rotation = Quat::IDENTITY;
                    self.knockback_velocity = Vec3::ZERO;
                    return false;
                }
                HitReactionPhase::None => {}
            }
        }

        true
    }

    /// 是否正在受傷反應中
    pub fn is_reacting(&self) -> bool {
        self.phase != HitReactionPhase::None
    }

    /// 取得當前擊退速度（用於移動系統）
    pub fn get_knockback_velocity(&self) -> Vec3 {
        if matches!(self.phase, HitReactionPhase::Knockback | HitReactionPhase::Stagger) {
            self.knockback_velocity
        } else {
            Vec3::ZERO
        }
    }
}
