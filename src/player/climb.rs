//! 攀爬/翻越系統
//!
//! GTA5 風格的攀爬和翻越機制。支援三種類型：
//! - Vault（翻越）：低矮障礙物（0.3-1.0m），快速跨越
//! - Climb（攀爬）：中等高度牆面（1.0-1.8m），抓住邊緣往上爬
//! - HighClimb（高位攀爬）：較高牆面（1.8-2.5m），需要更長時間


use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};
use crate::core::{ease_out_cubic, ease_in_out_quad, rapier_real_to_f32};
use super::{PlayerSkills, skills::award_climb_xp};

// ============================================================================
// 常數定義
// ============================================================================

/// 前方障礙物檢測距離
pub const DETECTION_DISTANCE: f32 = 1.5;
/// 射線起點高度（胸口）
pub const CHEST_HEIGHT: f32 = 0.8;
/// 最大可攀爬高度
pub const MAX_CLIMB_HEIGHT: f32 = 2.5;
/// 最小翻越高度
pub const MIN_VAULT_HEIGHT: f32 = 0.3;
/// 翻越閾值（低於此高度為 Vault）
pub const VAULT_THRESHOLD: f32 = 1.0;
/// 中等攀爬閾值（低於此高度為 Climb）
pub const MEDIUM_CLIMB_THRESHOLD: f32 = 1.8;
/// 著地點檢測深度
pub const LANDING_CHECK_DEPTH: f32 = 0.8;
/// 邊緣高度掃描步進
pub const EDGE_SCAN_STEP: f32 = 0.1;

// ============================================================================
// 動畫位置偏移常數
// ============================================================================
/// 接近障礙物時的垂直偏移（玩家身體低於邊緣）
pub const APPROACH_OFFSET: f32 = 0.3;
/// 手抓邊緣時的垂直偏移（手在邊緣下方）
pub const GRAB_OFFSET: f32 = 0.5;
/// 站在邊緣上的垂直偏移
pub const STANDING_OFFSET: f32 = 0.7;
/// 翻越弧線的頂點額外高度
pub const VAULT_PEAK_OFFSET: f32 = 0.4;

// ============================================================================
// 攀爬階段
// ============================================================================

/// 攀爬/翻越動畫階段
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ClimbPhase {
    #[default]
    None,
    /// 接近障礙物（短暫）
    Approaching,
    /// 抓住邊緣（僅 Climb/HighClimb）
    GrabbingEdge,
    /// 向上攀升/翻越
    Ascending,
    /// 著地
    Landing,
}

// ============================================================================
// 攀爬類型
// ============================================================================

/// 攀爬類型（根據障礙物高度決定）
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ClimbType {
    #[default]
    None,
    /// 低矮翻越（< 1.0m）- 快速跨越
    Vault,
    /// 中等攀爬（1.0 - 1.8m）- 抓住邊緣往上爬
    Climb,
    /// 高位攀爬（1.8 - 2.5m）- 較慢的攀爬
    HighClimb,
}

impl ClimbType {
    /// 根據障礙物高度決定攀爬類型
    pub fn from_height(height: f32) -> Self {
        if height < MIN_VAULT_HEIGHT {
            Self::None
        } else if height < VAULT_THRESHOLD {
            Self::Vault
        } else if height < MEDIUM_CLIMB_THRESHOLD {
            Self::Climb
        } else if height <= MAX_CLIMB_HEIGHT {
            Self::HighClimb
        } else {
            Self::None
        }
    }
}

// ============================================================================
// 攀爬狀態組件
// ============================================================================

/// 攀爬/翻越狀態組件
#[derive(Component, Default)]
pub struct ClimbState {
    /// 當前階段
    pub phase: ClimbPhase,
    /// 階段進度（0.0 ~ 1.0）
    pub progress: f32,
    /// 攀爬類型
    pub climb_type: ClimbType,
    /// 起始位置
    pub start_position: Vec3,
    /// 障礙物邊緣位置（頂部）
    pub edge_position: Vec3,
    /// 著地位置
    pub landing_position: Vec3,
    /// 障礙物高度
    pub obstacle_height: f32,
    /// 攀爬方向（面向障礙物的方向）
    pub climb_direction: Vec3,
}

impl ClimbState {
    // === 動畫時間常數 ===
    /// 接近階段持續時間
    pub const APPROACH_DURATION: f32 = 0.15;
    /// 抓住邊緣階段持續時間
    pub const GRAB_DURATION: f32 = 0.2;
    /// 翻越上升階段持續時間
    pub const ASCEND_DURATION_VAULT: f32 = 0.25;
    /// 攀爬上升階段持續時間
    pub const ASCEND_DURATION_CLIMB: f32 = 0.4;
    /// 高位攀爬上升階段持續時間
    pub const ASCEND_DURATION_HIGH: f32 = 0.6;
    /// 著地階段持續時間
    pub const LANDING_DURATION: f32 = 0.15;

    /// 是否正在攀爬中
    pub fn is_climbing(&self) -> bool {
        self.phase != ClimbPhase::None
    }

    /// 開始攀爬/翻越
    pub fn start(
        &mut self,
        climb_type: ClimbType,
        start_pos: Vec3,
        edge_pos: Vec3,
        landing_pos: Vec3,
        height: f32,
        direction: Vec3,
    ) {
        self.phase = ClimbPhase::Approaching;
        self.progress = 0.0;
        self.climb_type = climb_type;
        self.start_position = start_pos;
        self.edge_position = edge_pos;
        self.landing_position = landing_pos;
        self.obstacle_height = height;
        self.climb_direction = if direction.length_squared() > 1e-6 {
            direction.normalize()
        } else {
            Vec3::Z // 預設朝前
        };
    }

    /// 取得當前階段的持續時間
    pub fn current_phase_duration(&self) -> f32 {
        match self.phase {
            ClimbPhase::None => 0.0,
            ClimbPhase::Approaching => Self::APPROACH_DURATION,
            ClimbPhase::GrabbingEdge => Self::GRAB_DURATION,
            ClimbPhase::Ascending => match self.climb_type {
                ClimbType::Vault => Self::ASCEND_DURATION_VAULT,
                ClimbType::Climb => Self::ASCEND_DURATION_CLIMB,
                ClimbType::HighClimb => Self::ASCEND_DURATION_HIGH,
                ClimbType::None => 0.0,
            },
            ClimbPhase::Landing => Self::LANDING_DURATION,
        }
    }

    /// 取得總動畫時間
    #[allow(dead_code)]
    pub fn total_duration(&self) -> f32 {
        match self.climb_type {
            ClimbType::None => 0.0,
            ClimbType::Vault => {
                Self::APPROACH_DURATION + Self::ASCEND_DURATION_VAULT + Self::LANDING_DURATION
            }
            ClimbType::Climb => {
                Self::APPROACH_DURATION
                    + Self::GRAB_DURATION
                    + Self::ASCEND_DURATION_CLIMB
                    + Self::LANDING_DURATION
            }
            ClimbType::HighClimb => {
                Self::APPROACH_DURATION
                    + Self::GRAB_DURATION
                    + Self::ASCEND_DURATION_HIGH
                    + Self::LANDING_DURATION
            }
        }
    }

    /// 更新動畫進度，返回是否需要切換到下一階段
    pub fn update(&mut self, dt: f32) -> bool {
        if self.phase == ClimbPhase::None {
            return false;
        }

        let duration = self.current_phase_duration();
        if duration > 0.0 {
            self.progress += dt / duration;
        }

        self.progress >= 1.0
    }

    /// 切換到下一階段
    pub fn advance_phase(&mut self) {
        self.progress = 0.0;
        self.phase = match (self.phase, self.climb_type) {
            // Vault：跳過 GrabbingEdge
            (ClimbPhase::Approaching, ClimbType::Vault) => ClimbPhase::Ascending,
            // Climb/HighClimb：需要抓住邊緣
            (ClimbPhase::Approaching, _) => ClimbPhase::GrabbingEdge,
            (ClimbPhase::GrabbingEdge, _) => ClimbPhase::Ascending,
            (ClimbPhase::Ascending, _) => ClimbPhase::Landing,
            (ClimbPhase::Landing, _) => ClimbPhase::None,
            (ClimbPhase::None, _) => ClimbPhase::None,
        };
    }

    /// 重置狀態
    pub fn reset(&mut self) {
        self.phase = ClimbPhase::None;
        self.progress = 0.0;
        self.climb_type = ClimbType::None;
        self.climb_direction = Vec3::ZERO;
    }
}

// ============================================================================
// 檢測結果
// ============================================================================

/// 障礙物檢測結果
#[derive(Default)]
pub struct ClimbDetectionResult {
    /// 是否檢測到可攀爬障礙物
    pub detected: bool,
    /// 攀爬類型
    pub climb_type: ClimbType,
    /// 障礙物高度
    pub obstacle_height: f32,
    /// 障礙物邊緣位置（頂部）
    pub edge_position: Vec3,
    /// 著地位置
    pub landing_position: Vec3,
}


// ============================================================================
// 障礙物檢測函數
// ============================================================================

/// 檢測前方可攀爬的障礙物
///
/// 使用多重射線檢測：
/// 1. 前方射線（胸高）檢測障礙物存在
/// 2. 向上掃描找到障礙物頂部邊緣
/// 3. 著地點檢測確認翻越後的落腳處
pub fn detect_climbable_obstacle(
    player_pos: Vec3,
    player_forward: Vec3,
    player_entity: Entity,
    rapier: &RapierContext,
) -> ClimbDetectionResult {
    let filter = QueryFilter::default().exclude_collider(player_entity);
    let forward = player_forward.normalize_or_zero();
    if forward == Vec3::ZERO {
        return ClimbDetectionResult::default();
    }

    // Step 1: 前方射線檢測障礙物（從胸口高度發射）
    let chest_origin = player_pos + Vec3::Y * CHEST_HEIGHT;
    let forward_hit = rapier.cast_ray(chest_origin, forward, DETECTION_DISTANCE as RapierReal, true, filter);

    let Some((_, forward_toi)) = forward_hit else {
        return ClimbDetectionResult::default();
    };

    let obstacle_front = chest_origin + forward * rapier_real_to_f32(forward_toi);

    // Step 2: 向上掃描找到邊緣高度
    let scan_origin = obstacle_front + forward * 0.05; // 稍微往前一點
    let mut edge_height = 0.0;
    // 確保 max_steps 非負，防止常數配置錯誤時產生無限迴圈
    let max_steps = ((MAX_CLIMB_HEIGHT - CHEST_HEIGHT) / EDGE_SCAN_STEP).max(0.0) as i32;

    for i in 0..=max_steps {
        let h = i as f32 * EDGE_SCAN_STEP;
        let check_origin = scan_origin + Vec3::Y * h;

        // 如果這個高度沒有碰撞，說明找到了邊緣
        if rapier
            .cast_ray(check_origin, forward, 0.3 as RapierReal, true, filter)
            .is_none()
        {
            edge_height = CHEST_HEIGHT + h;
            break;
        }
    }

    // 檢查高度是否在有效範圍內
    if !(MIN_VAULT_HEIGHT..=MAX_CLIMB_HEIGHT).contains(&edge_height) {
        return ClimbDetectionResult::default();
    }

    // Step 3: 確認頂部表面存在（向下射線）
    let above_edge = player_pos + forward * (rapier_real_to_f32(forward_toi) + 0.3) + Vec3::Y * (edge_height + 0.3);
    let down_hit = rapier.cast_ray(above_edge, -Vec3::Y, 0.6 as RapierReal, true, filter);

    let Some((_, down_toi)) = down_hit else {
        return ClimbDetectionResult::default();
    };

    let edge_position = above_edge - Vec3::Y * rapier_real_to_f32(down_toi);

    // Step 4: 檢測著地點（翻越後的落腳處）
    let landing_check_origin = edge_position + forward * LANDING_CHECK_DEPTH + Vec3::Y * 1.5;
    let landing_hit = rapier.cast_ray(landing_check_origin, -Vec3::Y, 3.0 as RapierReal, true, filter);

    let landing_position = match landing_hit {
        Some((_, toi)) => landing_check_origin - Vec3::Y * rapier_real_to_f32(toi) + Vec3::Y * 0.1,
        None => edge_position + forward * LANDING_CHECK_DEPTH, // 沒有地面就站在邊緣上
    };

    // 確定攀爬類型
    let climb_type = ClimbType::from_height(edge_height);

    if climb_type == ClimbType::None {
        return ClimbDetectionResult::default();
    }

    ClimbDetectionResult {
        detected: true,
        climb_type,
        obstacle_height: edge_height,
        edge_position,
        landing_position,
    }
}

// ============================================================================
// 位置計算函數
// ============================================================================

/// 計算翻越（Vault）軌跡位置
///
/// 使用拋物線軌跡跨越障礙物
pub fn calculate_vault_position(
    start: Vec3,
    edge: Vec3,
    landing: Vec3,
    progress: f32,
) -> Vec3 {
    let t = ease_in_out_quad(progress.clamp(0.0, 1.0));

    // 水平方向：線性插值
    let horizontal = Vec3::new(
        start.x.lerp(landing.x, t),
        0.0,
        start.z.lerp(landing.z, t),
    );

    // 垂直方向：拋物線弧度
    let peak_height = edge.y + VAULT_PEAK_OFFSET;
    let arc = 4.0 * t * (1.0 - t); // 拋物線：在 t=0.5 時最高
    let base_y = start.y.lerp(landing.y, t);
    let vertical = base_y + arc * (peak_height - base_y.max(start.y));

    Vec3::new(horizontal.x, vertical, horizontal.z)
}

/// 計算攀爬（Climb）軌跡位置
///
/// 兩階段動畫：
/// 1. GrabbingEdge：移動到邊緣下方（手抓位置）
/// 2. Ascending：向上拉起到邊緣上方
pub fn calculate_climb_position(
    start: Vec3,
    edge: Vec3,
    landing: Vec3,
    phase: ClimbPhase,
    progress: f32,
) -> Vec3 {
    let t = progress.clamp(0.0, 1.0);

    match phase {
        ClimbPhase::Approaching => {
            // 接近障礙物
            let approach_target = Vec3::new(edge.x, start.y, edge.z) - Vec3::Y * APPROACH_OFFSET;
            start.lerp(approach_target, ease_out_cubic(t))
        }
        ClimbPhase::GrabbingEdge => {
            // 移動到邊緣下方（手抓住邊緣的位置）
            let approach_pos = Vec3::new(edge.x, start.y, edge.z) - Vec3::Y * APPROACH_OFFSET;
            let grab_pos = edge - Vec3::Y * GRAB_OFFSET;
            approach_pos.lerp(grab_pos, ease_out_cubic(t))
        }
        ClimbPhase::Ascending => {
            // 向上拉起
            let grab_pos = edge - Vec3::Y * GRAB_OFFSET;
            let top_pos = edge + Vec3::Y * STANDING_OFFSET;
            grab_pos.lerp(top_pos, ease_in_out_quad(t))
        }
        ClimbPhase::Landing => {
            // 著地
            let top_pos = edge + Vec3::Y * STANDING_OFFSET;
            top_pos.lerp(landing, ease_out_cubic(t))
        }
        ClimbPhase::None => start,
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 攀爬檢測系統
///
/// 持續檢測玩家前方是否有可攀爬的障礙物
pub fn climb_detection_system(
    rapier_context: ReadRapierContext,
    keyboard: Res<ButtonInput<KeyCode>>,
    vehicle_transition: Res<super::VehicleTransitionState>,
    mut query: Query<(
        Entity,
        &Transform,
        &super::Player,
        &mut ClimbState,
        &super::DodgeState,
        &crate::combat::PlayerCoverState,
    )>,
) {
    let Ok(rapier) = rapier_context.single() else { return; };

    // 上下車動畫中禁止攀爬
    if vehicle_transition.is_animating() {
        return;
    }

    for (entity, transform, player, mut climb_state, dodge_state, cover_state) in &mut query {
        // 阻擋條件：攀爬中、閃避中、掩體中
        if climb_state.is_climbing() || dodge_state.is_dodging || cover_state.is_in_cover {
            continue;
        }

        // 取得玩家前方方向
        let forward = transform.forward().as_vec3();

        // 檢測障礙物
        let detection = detect_climbable_obstacle(
            transform.translation,
            forward,
            entity,
            &rapier,
        );

        if !detection.detected {
            continue;
        }

        // 觸發條件（需要站在地面上）：
        // 1. 高速移動 + 前進輸入 → 自動觸發（跑酷風格）
        //    - 衝刺時自動觸發
        //    - 或速度超過 8.0 m/s 時（接近衝刺門檻）
        // 2. 按下 Space 鍵 → 手動觸發（靠近障礙物時）
        if !player.is_grounded {
            continue;
        }

        // 放寬自動觸發條件：不只衝刺，高速走路也能觸發
        let is_moving_fast = player.is_sprinting || player.current_speed > 8.0;
        let is_moving_forward = keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp);
        let auto_trigger = is_moving_fast && is_moving_forward;
        let manual_trigger = keyboard.just_pressed(KeyCode::Space);

        if auto_trigger || manual_trigger {
            climb_state.start(
                detection.climb_type,
                transform.translation,
                detection.edge_position,
                detection.landing_position,
                detection.obstacle_height,
                forward,
            );
        }
    }
}

/// 攀爬動畫系統
///
/// 處理攀爬動畫的位置插值和階段切換
pub fn climb_animation_system(
    time: Res<Time>,
    mut skills: ResMut<PlayerSkills>,
    mut query: Query<(&mut Transform, &mut ClimbState, &mut super::Player)>,
) {
    let dt = time.delta_secs();

    for (mut transform, mut climb_state, mut player) in &mut query {
        if !climb_state.is_climbing() {
            continue;
        }

        // 禁用重力影響
        player.vertical_velocity = 0.0;
        player.is_grounded = false;

        // 更新進度
        let should_advance = climb_state.update(dt);

        // 計算位置
        let new_position = match climb_state.climb_type {
            ClimbType::Vault => calculate_vault_position(
                climb_state.start_position,
                climb_state.edge_position,
                climb_state.landing_position,
                climb_state.progress,
            ),
            ClimbType::Climb | ClimbType::HighClimb => calculate_climb_position(
                climb_state.start_position,
                climb_state.edge_position,
                climb_state.landing_position,
                climb_state.phase,
                climb_state.progress,
            ),
            ClimbType::None => transform.translation,
        };

        // 更新位置
        transform.translation = new_position;

        // 更新朝向（面向攀爬方向）
        if climb_state.climb_direction.length_squared() > 0.01 {
            let target_rotation = Quat::from_rotation_y(
                (-climb_state.climb_direction.z).atan2(-climb_state.climb_direction.x)
                    + std::f32::consts::FRAC_PI_2,
            );
            transform.rotation = transform.rotation.slerp(target_rotation, dt * 10.0);
        }

        // 切換階段
        if should_advance {
            let was_landing = climb_state.phase == ClimbPhase::Landing;
            climb_state.advance_phase();

            // 如果動畫結束
            if climb_state.phase == ClimbPhase::None {
                // 攀爬完成，獎勵體力 XP
                if was_landing {
                    award_climb_xp(&mut skills);
                }
                player.is_grounded = true;
                climb_state.reset();
            }
        }
    }
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_climb_type_from_height() {
        assert_eq!(ClimbType::from_height(0.2), ClimbType::None);
        assert_eq!(ClimbType::from_height(0.5), ClimbType::Vault);
        assert_eq!(ClimbType::from_height(1.2), ClimbType::Climb);
        assert_eq!(ClimbType::from_height(2.0), ClimbType::HighClimb);
        assert_eq!(ClimbType::from_height(3.0), ClimbType::None);
    }

    #[test]
    fn test_climb_state_phases_vault() {
        let mut state = ClimbState::default();
        state.start(
            ClimbType::Vault,
            Vec3::ZERO,
            Vec3::Y,
            Vec3::new(0.0, 0.0, 1.0),
            0.8,
            Vec3::Z,
        );

        assert_eq!(state.phase, ClimbPhase::Approaching);
        state.advance_phase();
        assert_eq!(state.phase, ClimbPhase::Ascending); // Vault 跳過 GrabbingEdge
        state.advance_phase();
        assert_eq!(state.phase, ClimbPhase::Landing);
        state.advance_phase();
        assert_eq!(state.phase, ClimbPhase::None);
    }

    #[test]
    fn test_climb_state_phases_climb() {
        let mut state = ClimbState::default();
        state.start(
            ClimbType::Climb,
            Vec3::ZERO,
            Vec3::Y * 1.5,
            Vec3::new(0.0, 1.5, 1.0),
            1.5,
            Vec3::Z,
        );

        assert_eq!(state.phase, ClimbPhase::Approaching);
        state.advance_phase();
        assert_eq!(state.phase, ClimbPhase::GrabbingEdge); // Climb 需要抓邊緣
        state.advance_phase();
        assert_eq!(state.phase, ClimbPhase::Ascending);
        state.advance_phase();
        assert_eq!(state.phase, ClimbPhase::Landing);
        state.advance_phase();
        assert_eq!(state.phase, ClimbPhase::None);
    }

    #[test]
    fn test_total_duration() {
        let mut state = ClimbState { climb_type: ClimbType::Vault, ..ClimbState::default() };
        let vault_duration = state.total_duration();
        assert!(vault_duration < 1.0);
        assert!(vault_duration > 0.0);

        state.climb_type = ClimbType::HighClimb;
        let high_climb_duration = state.total_duration();
        assert!(high_climb_duration > vault_duration); // HighClimb 應比 Vault 更長
    }

    #[test]
    fn test_easing_functions() {
        assert!((ease_out_cubic(0.0) - 0.0).abs() < 0.001);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 0.001);

        assert!((ease_in_out_quad(0.0) - 0.0).abs() < 0.001);
        assert!((ease_in_out_quad(0.5) - 0.5).abs() < 0.001);
        assert!((ease_in_out_quad(1.0) - 1.0).abs() < 0.001);
    }
}
