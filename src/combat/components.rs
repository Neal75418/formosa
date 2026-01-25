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

/// 射擊輸入緩衝
#[derive(Resource, Default)]
pub struct ShootingInput {
    pub fire_pressed: bool,           // 射擊鍵按下
    pub fire_held: bool,              // 射擊鍵持續按住
    pub aim_pressed: bool,            // 瞄準鍵按住
    pub reload_pressed: bool,         // 換彈鍵按下
    pub weapon_switch: Option<usize>, // 切換武器 (1-4)
    pub mouse_wheel: f32,             // 滑鼠滾輪
}

// ============================================================================
// 敵人與玩家基礎組件
// ============================================================================

/// 敵人標記
#[derive(Component)]
#[allow(dead_code)]
pub struct Enemy {
    pub enemy_type: EnemyType,
}

/// 敵人類型
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnemyType {
    Gangster, // 小混混
    Thug,     // 打手
    Boss,     // 老大
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

/// 玩家手臂標記（用於揮拳動畫）
#[derive(Component, Debug)]
pub struct PlayerArm {
    pub is_right: bool,      // 是否為右手臂
    pub rest_position: Vec3, // 靜止位置
    pub rest_rotation: Quat, // 靜止旋轉
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

/// 揮拳動畫組件
#[derive(Component, Debug)]
#[allow(dead_code)]
pub struct PunchAnimation {
    pub timer: f32,        // 動畫計時器
    pub duration: f32,     // 總時長
    pub phase: PunchPhase, // 當前階段
}

impl Default for PunchAnimation {
    fn default() -> Self {
        Self {
            timer: 0.0,
            duration: 0.3, // 0.3 秒完成
            phase: PunchPhase::WindUp,
        }
    }
}

#[allow(dead_code)]
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

/// 敵人手臂標記（用於揮拳動畫）
#[derive(Component, Debug)]
pub struct EnemyArm {
    pub is_right: bool,      // 是否為右手臂
    pub rest_position: Vec3, // 靜止位置
    pub rest_rotation: Quat, // 靜止旋轉
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
    pub timer: f32,               // 動畫計時器
    pub duration: f32,            // 總時長
    pub phase: PunchPhase,        // 當前階段
    pub target: Option<Entity>,   // 攻擊目標
    pub attacker: Option<Entity>, // 攻擊者
    pub damage_dealt: bool,       // 是否已造成傷害
}

impl Default for EnemyPunchAnimation {
    fn default() -> Self {
        Self {
            timer: 0.0,
            duration: 0.35, // 敵人出拳稍慢一點
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
    None, // 無反應
    Flinch,    // 畏縮（輕傷）
    Stagger,   // 踉蹌（中傷）
    Knockback, // 擊退（重傷）
    Recovery,  // 恢復中
}

/// 受傷反應組件
/// 當實體受到傷害時，根據傷害量觸發不同的反應動畫
#[derive(Component, Debug)]
#[allow(dead_code)]
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

#[allow(dead_code)]
impl HitReaction {
    /// 傷害門檻常數
    pub const FLINCH_THRESHOLD: f32 = 10.0; // 10+ 傷害觸發畏縮
    pub const STAGGER_THRESHOLD: f32 = 25.0; // 25+ 傷害觸發踉蹌
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
