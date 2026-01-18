//! AI 組件
//!
//! 定義 AI 狀態機、感知、巡邏等組件。

#![allow(dead_code)] // 部分函數供未來擴展使用

use bevy::prelude::*;

// ============================================================================
// AI 狀態機
// ============================================================================

/// AI 狀態
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AiState {
    #[default]
    Idle,       // 閒置：站在原地
    Patrol,     // 巡邏：沿路徑移動
    Alert,      // 警戒：聽到聲音，搜索中
    Chase,      // 追逐：看到目標，追趕中
    Attack,     // 攻擊：在攻擊範圍內，開火
    Flee,       // 逃跑：血量過低
    TakingCover, // 躲掩體：血量低時尋找掩體
}

/// AI 行為組件
#[derive(Component, Debug)]
pub struct AiBehavior {
    /// 當前狀態
    pub state: AiState,
    /// 狀態計時器
    pub state_timer: f32,
    /// 上次狀態變更時間
    pub last_state_change: f32,
    /// 目標實體
    pub target: Option<Entity>,
    /// 上次看到目標的位置
    pub last_known_target_pos: Option<Vec3>,
    /// 上次看到目標的時間
    pub last_seen_time: f32,
    /// 逃跑血量閾值 (百分比)
    pub flee_threshold: f32,
    /// 是否已開始逃跑
    pub is_fleeing: bool,
    /// 生成保護時間（剛生成的敵人不會立即攻擊）
    pub spawn_protection: f32,
}

impl Default for AiBehavior {
    fn default() -> Self {
        Self {
            state: AiState::Idle,
            state_timer: 0.0,
            last_state_change: 0.0,
            target: None,
            last_known_target_pos: None,
            last_seen_time: 0.0,
            flee_threshold: 0.2, // 血量低於 20% 逃跑
            is_fleeing: false,
            spawn_protection: 2.0, // 生成後 2 秒內不會攻擊
        }
    }
}

impl AiBehavior {
    /// 設定新狀態
    pub fn set_state(&mut self, new_state: AiState, time: f32) {
        if self.state != new_state {
            self.state = new_state;
            self.state_timer = 0.0;
            self.last_state_change = time;
        }
    }

    /// 更新狀態計時器
    pub fn tick(&mut self, dt: f32) {
        self.state_timer += dt;
        // 更新生成保護時間
        if self.spawn_protection > 0.0 {
            self.spawn_protection -= dt;
        }
    }

    /// 檢查是否還在生成保護期
    pub fn is_spawn_protected(&self) -> bool {
        self.spawn_protection > 0.0
    }

    /// 記錄看到目標
    pub fn see_target(&mut self, target: Entity, position: Vec3, time: f32) {
        self.target = Some(target);
        self.last_known_target_pos = Some(position);
        self.last_seen_time = time;
    }

    /// 失去目標（超過時間沒看到）
    pub fn lose_target(&mut self, current_time: f32, timeout: f32) -> bool {
        current_time - self.last_seen_time > timeout
    }
}

// ============================================================================
// 感知組件
// ============================================================================

/// AI 感知組件（視覺、聽覺）
#[derive(Component, Debug)]
pub struct AiPerception {
    /// 視野角度（度）
    pub fov: f32,
    /// 視野距離
    pub sight_range: f32,
    /// 聽覺距離
    pub hearing_range: f32,
    /// 是否能看到目標
    pub can_see_target: bool,
    /// 是否聽到聲音
    pub heard_noise: bool,
    /// 聽到聲音的位置
    pub noise_position: Option<Vec3>,
}

impl Default for AiPerception {
    fn default() -> Self {
        Self {
            fov: 60.0,              // 60 度視野
            sight_range: 30.0,       // 30 公尺視距
            hearing_range: 50.0,     // 50 公尺聽力
            can_see_target: false,
            heard_noise: false,
            noise_position: None,
        }
    }
}

impl AiPerception {
    pub fn with_range(mut self, sight: f32, hearing: f32) -> Self {
        self.sight_range = sight;
        self.hearing_range = hearing;
        self
    }

    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    /// 檢查目標是否在視野內
    pub fn is_in_fov(&self, my_pos: Vec3, my_forward: Vec3, target_pos: Vec3) -> bool {
        let to_target = (target_pos - my_pos).normalize_or_zero();
        let dot = my_forward.dot(to_target);
        let angle = dot.acos().to_degrees();
        angle <= self.fov / 2.0
    }

    /// 檢查目標是否在視距內
    pub fn is_in_sight_range(&self, my_pos: Vec3, target_pos: Vec3) -> bool {
        my_pos.distance(target_pos) <= self.sight_range
    }

    /// 檢查聲音是否在聽力範圍內
    pub fn is_in_hearing_range(&self, my_pos: Vec3, sound_pos: Vec3) -> bool {
        my_pos.distance(sound_pos) <= self.hearing_range
    }
}

// ============================================================================
// 巡邏組件
// ============================================================================

/// 巡邏路徑組件
#[derive(Component, Debug)]
pub struct PatrolPath {
    /// 巡邏點列表
    pub waypoints: Vec<Vec3>,
    /// 當前目標點索引
    pub current_index: usize,
    /// 是否往返巡邏（否則循環）
    pub ping_pong: bool,
    /// 往返方向（true = 正向）
    pub forward: bool,
    /// 到達巡邏點後等待時間
    pub wait_time: f32,
    /// 當前等待計時器
    pub wait_timer: f32,
}

impl Default for PatrolPath {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            current_index: 0,
            ping_pong: false,
            forward: true,
            wait_time: 2.0,
            wait_timer: 0.0,
        }
    }
}

impl PatrolPath {
    pub fn new(waypoints: Vec<Vec3>) -> Self {
        Self {
            waypoints,
            ..default()
        }
    }

    /// 取得當前目標點
    pub fn current_waypoint(&self) -> Option<Vec3> {
        self.waypoints.get(self.current_index).copied()
    }

    /// 前進到下一個巡邏點
    pub fn advance(&mut self) {
        if self.waypoints.is_empty() {
            return;
        }

        if self.ping_pong {
            if self.forward {
                if self.current_index + 1 >= self.waypoints.len() {
                    self.forward = false;
                    self.current_index = self.current_index.saturating_sub(1);
                } else {
                    self.current_index += 1;
                }
            } else if self.current_index == 0 {
                self.forward = true;
                self.current_index = 1.min(self.waypoints.len() - 1);
            } else {
                self.current_index -= 1;
            }
        } else {
            self.current_index = (self.current_index + 1) % self.waypoints.len();
        }
    }
}

// ============================================================================
// 移動組件
// ============================================================================

/// AI 移動組件
#[derive(Component, Debug)]
pub struct AiMovement {
    /// 行走速度
    pub walk_speed: f32,
    /// 跑步速度
    pub run_speed: f32,
    /// 是否正在跑步
    pub is_running: bool,
    /// 到達目標的距離閾值
    pub arrival_threshold: f32,
    /// 當前移動目標
    pub move_target: Option<Vec3>,
}

impl Default for AiMovement {
    fn default() -> Self {
        Self {
            walk_speed: 2.0,
            run_speed: 5.0,
            is_running: false,
            arrival_threshold: 1.0,
            move_target: None,
        }
    }
}

impl AiMovement {
    pub fn current_speed(&self) -> f32 {
        if self.is_running {
            self.run_speed
        } else {
            self.walk_speed
        }
    }

    /// 檢查是否到達目標
    pub fn has_arrived(&self, current_pos: Vec3) -> bool {
        if let Some(target) = self.move_target {
            let dist = current_pos.distance(target);
            dist <= self.arrival_threshold
        } else {
            true
        }
    }
}

// ============================================================================
// 攻擊組件
// ============================================================================

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
    pub fn can_attack(&self) -> bool {
        self.cooldown_timer <= 0.0 && self.burst_fired == 0
    }

    pub fn is_in_range(&self, my_pos: Vec3, target_pos: Vec3) -> bool {
        my_pos.distance(target_pos) <= self.attack_range
    }

    pub fn start_attack(&mut self) {
        self.burst_fired = 0;
        self.burst_timer = 0.0;
    }

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

    pub fn should_fire_next(&self) -> bool {
        self.burst_fired > 0 && self.burst_fired < self.burst_count && self.burst_timer <= 0.0
    }
}

// ============================================================================
// 計時器資源
// ============================================================================

/// AI 更新計時器（降低 CPU 負載）
#[derive(Resource)]
pub struct AiUpdateTimer {
    pub perception_timer: Timer,  // 感知更新
    pub decision_timer: Timer,    // 決策更新
}

impl Default for AiUpdateTimer {
    fn default() -> Self {
        Self {
            perception_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            decision_timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }
}

/// 敵人生成計時器
#[derive(Resource)]
pub struct EnemySpawnTimer {
    pub timer: Timer,
    pub max_enemies: usize,
    pub spawn_radius: f32,
}

impl Default for EnemySpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            max_enemies: 10,
            spawn_radius: 70.0,  // 增加到 70m，配合最小生成距離 45m
        }
    }
}

// ============================================================================
// 掩體系統 (GTA 5 風格)
// ============================================================================

/// 掩體類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverType {
    Low,    // 低掩體（蹲姿射擊）：汽車、矮牆
    High,   // 高掩體（站姿射擊）：柱子、牆角
    Full,   // 完全掩護（無法射擊）：大型障礙物
}

/// 掩體點組件
/// 標記世界中可以作為掩護的位置
#[derive(Component, Debug)]
pub struct CoverPoint {
    /// 掩體類型
    pub cover_type: CoverType,
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
            cover_type: CoverType::Low,
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
            cover_type: CoverType::Low,
            cover_direction: direction.normalize_or_zero(),
            height: 0.8,
            damage_reduction: 0.5,
            ..default()
        }
    }

    /// 創建高掩體
    pub fn high(direction: Vec3) -> Self {
        Self {
            cover_type: CoverType::High,
            cover_direction: direction.normalize_or_zero(),
            height: 1.8,
            damage_reduction: 0.7,
            ..default()
        }
    }

    /// 創建完全掩體
    pub fn full(direction: Vec3) -> Self {
        Self {
            cover_type: CoverType::Full,
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
