//! 生命值與護甲系統


use bevy::prelude::*;

// ============================================================================
// 生命值與護甲
// ============================================================================

/// 可受傷實體標記
#[derive(Component)]
pub struct Damageable;

/// 生命值組件（通用，可附加到任何可受傷實體）
#[derive(Component, Debug, Clone)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    #[allow(dead_code)]
    pub regeneration: f32,     // 每秒回復量
    #[allow(dead_code)]
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
    /// 建立新實例
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
            ..default()
        }
    }

    /// 設定自動回復
    #[allow(dead_code)]
    pub fn with_regen(mut self, regen_per_sec: f32, delay: f32) -> Self {
        self.regeneration = regen_per_sec;
        self.regen_delay = delay;
        self
    }

    /// 是否死亡
    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    /// 是否滿血
    #[allow(dead_code)]
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    /// 計算百分比
    pub fn percentage(&self) -> f32 {
        (self.current / self.max).clamp(0.0, 1.0)
    }

    /// 受到傷害
    pub fn take_damage(&mut self, amount: f32, time: f32) -> f32 {
        let actual = amount.min(self.current);
        self.current -= actual;
        self.last_damage_time = time;
        actual
    }

    /// 治療
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
    /// 建立新實例
    #[allow(dead_code)]
    pub fn new(amount: f32) -> Self {
        Self {
            current: amount,
            max: 100.0,
            damage_reduction: 0.5,
        }
    }

    /// 計算百分比
    #[allow(dead_code)]
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

    /// 是否破碎
    #[allow(dead_code)]
    pub fn is_broken(&self) -> bool {
        self.current <= 0.0
    }

    #[allow(dead_code)]
    const SIGNIFICANT_HIT_THRESHOLD: f32 = 15.0;

    /// 是否受到重大打擊
    #[allow(dead_code)]
    pub fn took_significant_hit(&self, damage: f32) -> bool {
        damage >= Self::SIGNIFICANT_HIT_THRESHOLD && self.current > 0.0
    }

    /// 增加護甲值（購買/撿取護甲時使用）
    pub fn add(&mut self, amount: f32) -> f32 {
        let space = self.max - self.current;
        let actual = amount.min(space);
        self.current += actual;
        actual
    }
}

// ============================================================================
// 傷害與死亡事件
// ============================================================================

/// 傷害來源
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageSource {
    Bullet,                  // 子彈
    Explosion,               // 爆炸
    Melee,                   // 近戰
    Vehicle,                 // 車輛撞擊
    #[allow(dead_code)]
    Fall,                    // 墜落
    Fire,                    // 火焰
    Environment,             // 環境傷害
}

/// 傷害事件
#[derive(Message, Debug)]
pub struct DamageEvent {
    pub target: Entity,
    pub amount: f32,
    pub source: DamageSource,
    pub attacker: Option<Entity>,
    pub hit_position: Option<Vec3>,
    pub is_headshot: bool,
    /// 強制擊退（連擊終結技）
    pub force_knockback: bool,
}

impl DamageEvent {
    /// 建立新實例
    pub fn new(target: Entity, amount: f32, source: DamageSource) -> Self {
        Self {
            target,
            amount,
            source,
            attacker: None,
            hit_position: None,
            is_headshot: false,
            force_knockback: false,
        }
    }

    /// 設定攻擊者
    pub fn with_attacker(mut self, attacker: Entity) -> Self {
        self.attacker = Some(attacker);
        self
    }

    /// 設定位置
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.hit_position = Some(position);
        self
    }

    /// 設定爆頭標記
    pub fn with_headshot(mut self, is_headshot: bool) -> Self {
        self.is_headshot = is_headshot;
        self
    }
}

/// 爆頭傷害倍率
pub const HEADSHOT_MULTIPLIER: f32 = 2.5;
/// 頭部判定區域下限（相對於角色腳底，約肩膀高度）
const HEAD_HITBOX_MIN_HEIGHT: f32 = 1.5;
/// 頭部判定區域上限（相對於角色腳底，約頭頂高度）
const HEAD_HITBOX_MAX_HEIGHT: f32 = 2.0;

/// 檢查是否為爆頭（根據擊中位置和目標位置判斷）
pub fn check_headshot(hit_position: Vec3, target_base_y: f32) -> bool {
    let head_min = target_base_y + HEAD_HITBOX_MIN_HEIGHT;
    let head_max = target_base_y + HEAD_HITBOX_MAX_HEIGHT;
    hit_position.y >= head_min && hit_position.y <= head_max
}

/// 死亡事件
#[derive(Message)]
pub struct DeathEvent {
    pub entity: Entity,
    pub killer: Option<Entity>,
    pub cause: DamageSource,
    pub hit_position: Option<Vec3>,  // 擊中位置（用於計算布娃娃方向）
    pub hit_direction: Option<Vec3>, // 擊中方向（用於施加衝擊力）
}

/// 護甲破碎事件
#[derive(Message, Clone, Debug)]
pub struct ArmorBreakEvent {
    #[allow(dead_code)]
    pub entity: Entity,
    /// 破碎位置
    pub position: Vec3,
    /// 是否完全破碎（護甲歸零）
    pub is_full_break: bool,
}

// ============================================================================
// 流血效果
// ============================================================================

/// 流血效果常數
pub const BLEED_DAMAGE_PER_SECOND: f32 = 5.0;
/// 流血效果持續時間（秒）
pub const BLEED_DURATION: f32 = 4.0;
/// 流血觸發機率
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
    /// 建立新實例
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
