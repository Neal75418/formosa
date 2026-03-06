//! AI 攻擊和掩體系統（GTA 5 風格）

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

/// AI 攻擊組件
#[derive(Component, Debug)]
pub struct AiCombat {
    /// 攻擊距離
    pub attack_range: f32,
    /// 攻擊冷卻
    pub attack_cooldown: f32,
    /// 當前冷卻計時器
    pub cooldown_timer: f32,
    /// 瞄準精度（0-1，1 = 完美）
    pub accuracy: f32,
    /// 每次射擊彈數
    pub burst_count: u32,
    /// 已射擊彈數
    pub burst_fired: u32,
    /// 連射間隔
    pub burst_interval: f32,
    /// 連射計時器
    pub burst_timer: f32,
}

impl Default for AiCombat {
    fn default() -> Self {
        Self {
            attack_range: 20.0,
            attack_cooldown: 1.5,
            cooldown_timer: 0.0,
            accuracy: 0.6,
            burst_count: 3,
            burst_fired: 0,
            burst_interval: 0.15,
            burst_timer: 0.0,
        }
    }
}

impl AiCombat {
    /// 是否可攻擊
    pub fn can_attack(&self) -> bool {
        self.cooldown_timer <= 0.0 && self.burst_fired == 0
    }

    /// 是否在射程內
    pub fn is_in_range(&self, my_pos: Vec3, target_pos: Vec3) -> bool {
        let attack_range_sq = self.attack_range * self.attack_range;
        my_pos.distance_squared(target_pos) <= attack_range_sq
    }

    /// 開始攻擊
    pub fn start_attack(&mut self) {
        self.burst_fired = 0;
        self.burst_timer = 0.0;
    }

    /// 計時器滴答
    pub fn tick(&mut self, dt: f32) {
        if self.cooldown_timer > 0.0 {
            self.cooldown_timer -= dt;
        }
        if self.burst_fired > 0 && self.burst_fired < self.burst_count {
            self.burst_timer -= dt;
        }
    }

    /// 發射一發，回傳是否完成連射
    pub fn fire_once(&mut self) -> bool {
        self.burst_fired += 1;
        if self.burst_fired >= self.burst_count {
            self.cooldown_timer = self.attack_cooldown;
            self.burst_fired = 0;
            true
        } else {
            self.burst_timer = self.burst_interval;
            false
        }
    }

    /// 是否應連射下一發
    pub fn should_fire_next(&self) -> bool {
        self.burst_fired > 0 && self.burst_fired < self.burst_count && self.burst_timer <= 0.0
    }
}

// ============================================================================
// 掩體系統 (GTA 5 風格)
// ============================================================================

/// 掩體類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverType {
    Low,  // 低掩體（蹲姿射擊）：汽車、矮牆
    High, // 高掩體（站姿射擊）：柱子、牆角
    Full, // 完全掩護（無法射擊）：大型障礙物
}

/// 掩體點組件
/// 標記世界中可以作為掩護的位置
#[derive(Component, Debug)]
pub struct CoverPoint {
    /// 掩護方向（敵人應該從這個方向躲在掩體後面）
    pub cover_direction: Vec3,
    /// 掩體高度
    pub height: f32,
    /// 是否正在被使用
    pub occupied: bool,
    /// 使用中的實體
    pub occupant: Option<Entity>,
    /// 掩體提供的傷害減免 (0.0-1.0)
    pub damage_reduction: f32,
}

impl Default for CoverPoint {
    fn default() -> Self {
        Self {
            cover_direction: Vec3::NEG_Z,
            height: 1.0,
            occupied: false,
            occupant: None,
            damage_reduction: 0.5, // 預設 50% 傷害減免
        }
    }
}

impl CoverPoint {
    /// 創建低掩體
    pub fn low(direction: Vec3) -> Self {
        Self {
            cover_direction: direction.normalize_or_zero(),
            height: 0.8,
            damage_reduction: 0.5,
            ..default()
        }
    }

    /// 創建高掩體
    pub fn high(direction: Vec3) -> Self {
        Self {
            cover_direction: direction.normalize_or_zero(),
            height: 1.8,
            damage_reduction: 0.7,
            ..default()
        }
    }

    /// 創建完全掩體
    pub fn full(direction: Vec3) -> Self {
        Self {
            cover_direction: direction.normalize_or_zero(),
            height: 2.5,
            damage_reduction: 1.0,
            ..default()
        }
    }

    /// 檢查是否可用
    pub fn is_available(&self) -> bool {
        !self.occupied
    }

    /// 佔用此掩體
    pub fn occupy(&mut self, entity: Entity) {
        self.occupied = true;
        self.occupant = Some(entity);
    }

    /// 釋放此掩體
    pub fn release(&mut self) {
        self.occupied = false;
        self.occupant = None;
    }

    /// 檢查某個位置是否在掩體後面
    /// attacker_pos: 攻擊者位置
    /// defender_pos: 防禦者位置
    pub fn is_covered_from(&self, cover_pos: Vec3, defender_pos: Vec3, attacker_pos: Vec3) -> bool {
        let to_attacker = (attacker_pos - cover_pos).normalize_or_zero();
        let to_defender = (defender_pos - cover_pos).normalize_or_zero();

        // 檢查防禦者是否在掩體的背面
        let defender_dot = self.cover_direction.dot(to_defender);
        let attacker_dot = self.cover_direction.dot(to_attacker);

        // 防禦者應該在掩體方向的反面，攻擊者在正面
        defender_dot < -0.3 && attacker_dot > 0.3
    }
}

/// AI 掩體尋找組件
/// 附加到可以使用掩體的 AI 實體上
#[derive(Component, Debug)]
pub struct CoverSeeker {
    /// 尋找掩體的觸發血量比例 (0.0-1.0)
    pub seek_cover_health_threshold: f32,
    /// 當前目標掩體實體
    pub target_cover: Option<Entity>,
    /// 是否正在掩體後面
    pub is_in_cover: bool,
    /// 在掩體後等待的時間
    pub cover_time: f32,
    /// 從掩體探出射擊的間隔
    pub peek_interval: f32,
    /// 探出計時器
    pub peek_timer: f32,
    /// 是否正在探出
    pub is_peeking: bool,
    /// 最大掩體距離（超過此距離不會尋找掩體）
    pub max_cover_distance: f32,
}

impl Default for CoverSeeker {
    fn default() -> Self {
        Self {
            seek_cover_health_threshold: 0.5, // 血量低於 50% 開始找掩體
            target_cover: None,
            is_in_cover: false,
            cover_time: 0.0,
            peek_interval: 2.0, // 每 2 秒探出一次
            peek_timer: 0.0,
            is_peeking: false,
            max_cover_distance: 15.0,
        }
    }
}

impl CoverSeeker {
    /// 是否應該尋找掩體
    pub fn should_seek_cover(&self, health_percentage: f32) -> bool {
        health_percentage <= self.seek_cover_health_threshold && self.target_cover.is_none()
    }

    /// 到達掩體
    pub fn enter_cover(&mut self, cover_entity: Entity) {
        self.target_cover = Some(cover_entity);
        self.is_in_cover = true;
        self.cover_time = 0.0;
        self.peek_timer = self.peek_interval;
    }

    /// 離開掩體
    pub fn leave_cover(&mut self) {
        self.target_cover = None;
        self.is_in_cover = false;
        self.is_peeking = false;
    }

    /// 更新掩體計時器
    pub fn tick(&mut self, dt: f32) {
        if self.is_in_cover {
            self.cover_time += dt;
            self.peek_timer -= dt;

            // 週期性探出射擊
            if self.peek_timer <= 0.0 {
                self.is_peeking = true;
                self.peek_timer = self.peek_interval;
            }
        }
    }

    /// 結束探出
    pub fn end_peek(&mut self) {
        self.is_peeking = false;
    }
}
