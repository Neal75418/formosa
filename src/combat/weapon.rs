//! 武器系統（類型、屬性、彈藥、冷卻）

#![allow(dead_code)]

/// 彈道視覺風格
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum TracerStyle {
    None,    // 無彈道（近戰）
    Pistol,  // 手槍：淡黃短軌跡
    SMG,     // 衝鋒槍：橙色細軌跡
    Shotgun, // 霰彈槍：白色散射彈丸
    Rifle,   // 步槍：紅色長曳光彈
}

use bevy::prelude::*;

// ============================================================================
// 武器類型與數據
// ============================================================================

/// 武器類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum WeaponType {
    #[default]
    Fist, // 拳頭（近戰）
    Staff,   // 棍棒（近戰）
    Knife,   // 刀（近戰）
    Pistol,  // 手槍
    SMG,     // 衝鋒槍
    Shotgun, // 霰彈槍
    Rifle,   // 步槍
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
        matches!(
            self,
            WeaponType::Fist | WeaponType::Staff | WeaponType::Knife
        )
    }

    /// 取得穩定的存檔鍵值（不受 enum 重命名影響）
    pub fn save_key(&self) -> &'static str {
        match self {
            WeaponType::Fist => "Fist",
            WeaponType::Staff => "Staff",
            WeaponType::Knife => "Knife",
            WeaponType::Pistol => "Pistol",
            WeaponType::SMG => "SMG",
            WeaponType::Shotgun => "Shotgun",
            WeaponType::Rifle => "Rifle",
        }
    }
}

/// 近戰動畫類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MeleeAnimationType {
    #[default]
    Punch, // 拳頭
    Swing, // 棍棒揮擊
    Slash, // 刀砍
    Stab,  // 刀刺
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

/// 武器數據（定義武器屬性）
#[derive(Clone, Debug)]
pub struct WeaponStats {
    pub weapon_type: WeaponType,
    pub damage: f32,        // 單發傷害
    pub fire_rate: f32,     // 射擊間隔（秒）
    pub magazine_size: u32, // 彈匣容量
    pub max_ammo: u32,      // 最大後備彈藥
    pub range: f32,         // 有效射程（公尺）
    pub reload_time: f32,   // 換彈時間（秒）
    pub spread: f32,        // 散射角度（度）
    pub pellet_count: u32,  // 彈丸數量（霰彈槍用）
    pub bullet_speed: f32,  // 子彈速度
    pub is_automatic: bool, // 是否全自動
    // === 距離傷害衰減 ===
    pub falloff_start: f32, // 開始衰減距離（公尺）
    pub falloff_end: f32,   // 最低傷害距離（公尺），此距離後傷害為 25%
    // === 後座力系統 ===
    pub recoil_vertical: f32,   // 垂直後座力（向上偏移）
    pub recoil_horizontal: f32, // 水平後座力（隨機左右偏移）
    pub recoil_recovery: f32,   // 後座力恢復速度（每秒恢復量）
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
            damage: 15.0, // 每顆彈丸 15，8 顆共 120
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
            damage: 20.0,     // 提高傷害：3 拳殺小混混(50HP)
            fire_rate: 0.35,  // 稍微加快出拳速度
            magazine_size: 0, // 無限
            max_ammo: 0,
            range: 2.5, // 稍微增加攻擊距離
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
            range: 3.2, // 較長射程
            reload_time: 0.0,
            spread: 60.0,    // 掃擊角度（度）
            pellet_count: 1, // 掃擊邏輯在系統中處理
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
            range: 2.0, // 短射程
            reload_time: 0.0,
            spread: 0.0, // 單目標
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
    pub current_ammo: u32,  // 當前彈匣內子彈
    pub reserve_ammo: u32,  // 後備彈藥
    pub fire_cooldown: f32, // 射擊冷卻計時器
    pub is_reloading: bool, // 是否正在換彈
    pub reload_timer: f32,  // 換彈計時器
}

impl Weapon {
    /// 建立新實例
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
        if self.is_reloading
            || self.reserve_ammo == 0
            || self.current_ammo == self.stats.magazine_size
        {
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

    /// 推進射擊冷卻計時器
    pub fn tick_cooldown(&mut self, dt: f32) {
        if self.fire_cooldown > 0.0 {
            self.fire_cooldown = (self.fire_cooldown - dt).max(0.0);
        }
    }

    /// 推進換彈計時器，完成時自動換彈
    /// 返回 true 表示本幀正在換彈（應跳過攻擊）
    pub fn tick_reload(&mut self, dt: f32) -> bool {
        if !self.is_reloading {
            return false;
        }
        self.reload_timer = (self.reload_timer - dt).max(0.0);
        if self.reload_timer <= 0.0 {
            self.finish_reload();
        }
        true
    }

    /// 是否正在冷卻中
    pub fn is_cooling_down(&self) -> bool {
        self.fire_cooldown > 0.0
    }

    /// 設定射擊冷卻時間
    pub fn set_fire_cooldown(&mut self, cooldown: f32) {
        self.fire_cooldown = cooldown;
    }

    /// 重置射擊冷卻為武器射速
    pub fn reset_fire_cooldown(&mut self) {
        self.fire_cooldown = self.stats.fire_rate;
    }

    /// 取得武器有效射程
    pub fn effective_range(&self) -> f32 {
        self.stats.range
    }

    /// 取得武器基礎傷害
    pub fn base_damage(&self) -> f32 {
        self.stats.damage
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
        self.weapons
            .iter()
            .any(|w| w.stats.weapon_type == weapon_type)
    }
}
