//! 戰鬥系統組件
//!
//! 定義剩餘的戰鬥組件（玩家動畫、敵人基礎、全域狀態）。
//! 武器、生命值、視覺效果已拆分至 weapon.rs, health.rs, visuals.rs。

use super::weapon::WeaponStats;
use bevy::prelude::*;

// ============================================================================
// 戰鬥狀態資源
// ============================================================================

// 其他常數
/// 近戰基礎傷害
pub const MELEE_DAMAGE: f32 = 15.0;

/// 戰鬥狀態（全域資源）
#[derive(Resource, Default)]
pub struct CombatState {
    pub is_aiming: bool,           // 是否正在瞄準
    pub crosshair_bloom: f32,      // 準星擴散程度
    pub last_shot_time: f32,       // 上次射擊時間
    pub hit_marker_timer: f32,     // 命中標記顯示計時器
    pub hit_marker_headshot: bool, // 是否為爆頭（影響顏色）
    // === 車上射擊相關 ===
    pub can_fire_in_vehicle: bool,  // 是否可在車上射擊
    pub vehicle_aim_valid: bool,    // 車上瞄準角度是否有效
    pub last_hit_time: Option<f32>, // 上次命中時間
}

/// 自動瞄準/鎖定狀態（GTA 5 風格）
///
/// 瞄準時自動鎖定最近的敵人，提供瞄準吸附和目標追蹤。
/// Tab 鍵切換目標，停止瞄準或目標死亡/超出距離時解除鎖定。
#[derive(Resource)]
pub struct LockOnState {
    /// 當前鎖定的目標實體
    pub locked_target: Option<Entity>,
    /// 鎖定搜索範圍（公尺）
    pub lock_range: f32,
    /// 最大保持距離（超出則解鎖）
    pub max_range: f32,
    /// 鎖定視野半角（弧度）
    pub fov_half_angle: f32,
    /// 瞄準吸附強度 (0.0 = 無吸附, 1.0 = 完全鎖定)
    pub snap_strength: f32,
    /// 失去視線計時器
    pub los_lost_timer: f32,
    /// 失去視線容忍時間（秒）
    pub los_timeout: f32,
}

impl Default for LockOnState {
    fn default() -> Self {
        Self {
            locked_target: None,
            lock_range: 30.0,
            max_range: 40.0,
            fov_half_angle: 0.52, // ~30°
            snap_strength: 0.85,
            los_lost_timer: 0.0,
            los_timeout: 2.0,
        }
    }
}

// ============================================================================
// 近戰連擊系統
// ============================================================================

/// 連擊階段（GTA 5 風格四段連擊鏈）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ComboStep {
    #[default]
    Jab,       // 第 1 擊：直拳
    Hook,      // 第 2 擊：鉤拳
    Uppercut,  // 第 3 擊：上勾拳
    Finisher,  // 第 4 擊：重拳（強制擊退）
}

impl ComboStep {
    /// 傷害倍率
    pub fn damage_multiplier(self) -> f32 {
        match self {
            ComboStep::Jab => 1.0,
            ComboStep::Hook => 1.2,
            ComboStep::Uppercut => 1.5,
            ComboStep::Finisher => 2.0,
        }
    }

    /// 動畫時長（秒）— 終結技較慢
    pub fn animation_duration(self) -> f32 {
        match self {
            ComboStep::Jab => 0.28,
            ComboStep::Hook => 0.30,
            ComboStep::Uppercut => 0.32,
            ComboStep::Finisher => 0.40,
        }
    }

    /// 下一階段（Finisher 後回到 Jab）
    pub fn next(self) -> Self {
        match self {
            ComboStep::Jab => ComboStep::Hook,
            ComboStep::Hook => ComboStep::Uppercut,
            ComboStep::Uppercut => ComboStep::Finisher,
            ComboStep::Finisher => ComboStep::Jab,
        }
    }

    /// 是否為終結技
    pub fn is_finisher(self) -> bool {
        self == ComboStep::Finisher
    }
}

/// 連擊窗口超時時間（秒）
pub const COMBO_WINDOW: f32 = 0.6;

/// 近戰連擊狀態（全域資源）
///
/// 追蹤玩家的連擊鏈：每次近戰命中在窗口內推進到下一階段，
/// 超時或切換武器則重置。終結技（第 4 擊）自帶擊退效果。
#[derive(Resource)]
pub struct MeleeComboState {
    /// 當前連擊階段
    pub current_step: ComboStep,
    /// 上次命中時間（用於判斷連擊窗口）
    pub last_hit_time: f32,
    /// 連擊是否啟動中
    pub active: bool,
}

impl Default for MeleeComboState {
    fn default() -> Self {
        Self {
            current_step: ComboStep::Jab,
            last_hit_time: 0.0,
            active: false,
        }
    }
}

impl MeleeComboState {
    /// 註冊一次近戰命中 — 推進連擊階段
    pub fn register_hit(&mut self, time: f32) {
        if self.active && (time - self.last_hit_time) <= COMBO_WINDOW {
            self.current_step = self.current_step.next();
        } else {
            // 超時或首次攻擊 → 從 Jab 開始
            self.current_step = ComboStep::Jab;
            self.active = true;
        }
        self.last_hit_time = time;
    }

    /// 重置連擊（切換武器、被擊中等）
    pub fn reset(&mut self) {
        self.current_step = ComboStep::Jab;
        self.active = false;
    }

    /// 取得當前傷害倍率
    pub fn damage_multiplier(&self) -> f32 {
        if self.active {
            self.current_step.damage_multiplier()
        } else {
            1.0
        }
    }
}

// ============================================================================
// 格擋/反擊系統
// ============================================================================

/// 精準格擋窗口（秒）— 按下格擋後的前 0.2 秒
pub const PARRY_WINDOW: f32 = 0.2;
/// 格擋傷害減免比例（60%）
pub const BLOCK_DAMAGE_REDUCTION: f32 = 0.6;
/// 反擊傷害倍率（精準格擋後下次攻擊）
pub const COUNTER_DAMAGE_MULTIPLIER: f32 = 2.0;
/// 格擋每次吸收消耗體力
pub const BLOCK_STAMINA_COST: f32 = 5.0;
/// 精準格擋每次消耗體力
pub const PARRY_STAMINA_COST: f32 = 2.0;
/// 反擊加成有效時間（秒）
pub const COUNTER_WINDOW: f32 = 2.0;

/// 格擋/反擊狀態（全域資源）
///
/// 近戰武器時按住右鍵 = 格擋；格擋啟動瞬間（0.2s 內）受擊 = 精準格擋，
/// 觸發反擊加成（下一次近戰攻擊 2x 傷害）。
#[derive(Resource)]
pub struct BlockState {
    /// 是否正在格擋
    pub is_blocking: bool,
    /// 格擋開始時間（用於判斷精準格擋窗口）
    pub block_start_time: f32,
    /// 反擊準備就緒（精準格擋成功後）
    pub counter_ready: bool,
    /// 反擊啟動時間（超過 COUNTER_WINDOW 秒後失效）
    pub counter_start_time: f32,
    /// 精準格擋次數（統計用）
    pub parry_count: u32,
}

impl Default for BlockState {
    fn default() -> Self {
        Self {
            is_blocking: false,
            block_start_time: 0.0,
            counter_ready: false,
            counter_start_time: 0.0,
            parry_count: 0,
        }
    }
}

impl BlockState {
    /// 開始格擋
    pub fn start_block(&mut self, time: f32) {
        if !self.is_blocking {
            self.is_blocking = true;
            self.block_start_time = time;
        }
    }

    /// 結束格擋
    pub fn stop_block(&mut self) {
        self.is_blocking = false;
    }

    /// 判斷當前是否在精準格擋窗口內
    pub fn is_parry_window(&self, current_time: f32) -> bool {
        self.is_blocking && (current_time - self.block_start_time) <= PARRY_WINDOW
    }

    /// 觸發反擊準備（精準格擋成功時呼叫）
    pub fn activate_counter(&mut self, time: f32) {
        self.counter_ready = true;
        self.counter_start_time = time;
        self.parry_count += 1;
    }

    /// 消耗反擊加成，回傳傷害倍率
    pub fn consume_counter(&mut self) -> f32 {
        if self.counter_ready {
            self.counter_ready = false;
            COUNTER_DAMAGE_MULTIPLIER
        } else {
            1.0
        }
    }

    /// 更新：反擊超時失效
    pub fn update_counter_timeout(&mut self, current_time: f32) {
        if self.counter_ready && (current_time - self.counter_start_time) > COUNTER_WINDOW {
            self.counter_ready = false;
        }
    }
}

/// 射擊輸入緩衝
#[derive(Resource, Default)]
pub struct ShootingInput {
    pub is_fire_pressed: bool,           // 射擊鍵按下
    pub is_fire_held: bool,              // 射擊鍵持續按住
    pub is_aim_pressed: bool,            // 瞄準鍵按住
    pub is_block_pressed: bool,          // 格擋鍵按住（近戰右鍵）
    pub is_reload_pressed: bool,         // 換彈鍵按下
    pub weapon_switch: Option<usize>, // 切換武器 (1-4)
    pub mouse_wheel: f32,             // 滑鼠滾輪
}

// ============================================================================
// 敵人與玩家基礎組件
// ============================================================================

/// 敵人標記
#[derive(Component)]
pub struct Enemy {
    #[allow(dead_code)]
    pub enemy_type: EnemyType,
}

/// 敵人類型
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EnemyType {
    Gangster, // 小混混
    Thug,     // 打手
    Boss,     // 老大
    Military, // 軍人（5 星通緝出現）
}

impl EnemyType {
    /// 取得生命值
    pub fn health(&self) -> f32 {
        match self {
            EnemyType::Gangster => 50.0,
            EnemyType::Thug => 80.0,
            EnemyType::Boss => 200.0,
            EnemyType::Military => 150.0,
        }
    }

    /// 取得武器資訊
    pub fn weapon(&self) -> WeaponStats {
        match self {
            EnemyType::Gangster => WeaponStats::pistol(),
            EnemyType::Thug => WeaponStats::smg(),
            EnemyType::Boss => WeaponStats::shotgun(),
            EnemyType::Military => WeaponStats::rifle(),
        }
    }
}

/// 玩家手臂標記（用於揮拳動畫）
#[derive(Component, Debug)]
pub struct PlayerArm {
    pub is_right: bool,      // 是否為右手臂
    pub rest_position: Vec3, // 靜止位置
    pub rest_rotation: Quat, // 靜止旋轉
}

impl PlayerArm {
    /// 左側部件
    pub fn left(position: Vec3, rotation: Quat) -> Self {
        Self {
            is_right: false,
            rest_position: position,
            rest_rotation: rotation,
        }
    }

    /// 右側部件
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

// ============================================================================
// 揮拳動畫系統
// ============================================================================

/// 揮拳動畫階段
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum PunchPhase {
    #[default]
    WindUp, // 蓄力（向後拉）
    Strike, // 出拳（向前伸）
    Return, // 收回
}

/// 揮拳動畫通用特徵
pub trait PunchAnimatable {
    fn get_timer(&self) -> f32;
    fn get_duration(&self) -> f32;
    fn set_phase(&mut self, phase: PunchPhase);

    /// 取得各階段時間佔比
    fn phase_times(&self) -> (f32, f32, f32) {
        let duration = self.get_duration();
        (duration * 0.33, duration * 0.66, duration)
    }

    /// 檢查動畫是否完成
    fn is_finished(&self) -> bool {
        self.get_timer() >= self.get_duration()
    }

    /// 根據計時器更新動畫階段
    fn update_phase(&mut self) {
        let (wind_up_end, strike_end, duration) = self.phase_times();
        let t = self.get_timer();
        if t < wind_up_end {
            self.set_phase(PunchPhase::WindUp);
        } else if t < strike_end {
            self.set_phase(PunchPhase::Strike);
        } else if t < duration {
            self.set_phase(PunchPhase::Return);
        }
    }
}

/// 揮拳動畫組件
#[derive(Component, Debug)]
pub struct PunchAnimation {
    pub timer: f32,            // 動畫計時器
    pub duration: f32,         // 總時長
    pub phase: PunchPhase,     // 當前階段
    pub combo_step: ComboStep, // 連擊階段（影響動畫軌跡）
}

impl Default for PunchAnimation {
    fn default() -> Self {
        Self {
            timer: 0.0,
            duration: 0.3, // 0.3 秒完成
            phase: PunchPhase::WindUp,
            combo_step: ComboStep::Jab,
        }
    }
}

impl PunchAnimation {
    /// 根據連擊階段建立動畫
    pub fn for_combo_step(step: ComboStep) -> Self {
        Self {
            timer: 0.0,
            duration: step.animation_duration(),
            phase: PunchPhase::WindUp,
            combo_step: step,
        }
    }
}

impl PunchAnimatable for PunchAnimation {
    fn get_timer(&self) -> f32 {
        self.timer
    }

    fn get_duration(&self) -> f32 {
        self.duration
    }

    fn set_phase(&mut self, phase: PunchPhase) {
        self.phase = phase;
    }
}

/// 敵人手臂標記（用於揮拳動畫）
#[derive(Component, Debug)]
pub struct EnemyArm {
    pub is_right: bool,      // 是否為右手臂
    pub rest_position: Vec3, // 靜止位置
    pub rest_rotation: Quat, // 靜止旋轉
}

impl EnemyArm {
    /// 左側部件
    pub fn left(position: Vec3, rotation: Quat) -> Self {
        Self {
            is_right: false,
            rest_position: position,
            rest_rotation: rotation,
        }
    }

    /// 右側部件
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
    pub timer: f32,               // 動畫計時器
    pub duration: f32,            // 總時長
    pub phase: PunchPhase,        // 當前階段
    pub target: Option<Entity>,   // 攻擊目標
    pub attacker: Option<Entity>, // 攻擊者
    pub has_damage_dealt: bool,       // 是否已造成傷害
}

impl Default for EnemyPunchAnimation {
    fn default() -> Self {
        Self {
            timer: 0.0,
            duration: 0.35, // 敵人出拳稍慢一點
            phase: PunchPhase::WindUp,
            target: None,
            attacker: None,
            has_damage_dealt: false,
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

impl PunchAnimatable for EnemyPunchAnimation {
    fn get_timer(&self) -> f32 {
        self.timer
    }

    fn get_duration(&self) -> f32 {
        self.duration
    }

    fn set_phase(&mut self, phase: PunchPhase) {
        self.phase = phase;
    }
}

// ============================================================================
// 受傷反應系統 (GTA 5 風格)
// ============================================================================

/// 受傷反應階段
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum HitReactionPhase {
    #[default]
    None, // 無反應
    Flinch,    // 畏縮（輕傷）
    Stagger,   // 踉蹌（中傷）
    Knockback, // 擊退（重傷）
    Recovery,  // 恢復中
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
    pub const FLINCH_THRESHOLD: f32 = 10.0; // 10+ 傷害觸發畏縮
    /// 觸發硬直的傷害閾值
    pub const STAGGER_THRESHOLD: f32 = 25.0; // 25+ 傷害觸發踉蹌
    /// 觸發擊退的傷害閾值
    pub const KNOCKBACK_THRESHOLD: f32 = 40.0; // 40+ 傷害觸發擊退

    /// 反應持續時間常數
    pub const FLINCH_DURATION: f32 = 0.15;
    /// 硬直持續時間（秒）
    pub const STAGGER_DURATION: f32 = 0.3;
    /// 擊退持續時間（秒）
    pub const KNOCKBACK_DURATION: f32 = 0.5;
    /// 恢復持續時間（秒）
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
        let (phase, duration, knockback_speed) =
            if is_headshot || damage >= Self::KNOCKBACK_THRESHOLD {
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
                let intensity = if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                };
                self.visual_rotation = Quat::slerp(self.visual_rotation, Quat::IDENTITY, intensity);
            }
            HitReactionPhase::None => {}
        }

        // 檢查是否完成當前階段
        if self.timer >= self.duration {
            match self.phase {
                HitReactionPhase::Flinch
                | HitReactionPhase::Stagger
                | HitReactionPhase::Knockback => {
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
    #[allow(dead_code)]
    pub fn is_reacting(&self) -> bool {
        self.phase != HitReactionPhase::None
    }

    /// 取得當前擊退速度（用於移動系統）
    pub fn get_knockback_velocity(&self) -> Vec3 {
        if matches!(
            self.phase,
            HitReactionPhase::Knockback | HitReactionPhase::Stagger
        ) {
            self.knockback_velocity
        } else {
            Vec3::ZERO
        }
    }
}
