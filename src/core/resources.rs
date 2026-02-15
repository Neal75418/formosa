//! 遊戲核心資源（狀態、互動、碰撞群組、工具函數）

// 部分資源欄位為將來擴展預留，個別標記 #[allow(dead_code)]

use bevy::prelude::*;

/// 遊戲狀態
#[derive(Resource, Default)]
pub struct GameState {
    pub player_in_vehicle: bool,
    pub current_vehicle: Option<Entity>,
}

/// 世界時間
#[derive(Resource)]
pub struct WorldTime {
    pub hour: f32,
    pub time_scale: f32,
}

impl Default for WorldTime {
    fn default() -> Self {
        Self {
            hour: 8.0,
            time_scale: 1.0,
        }
    }
}

/// 玩家狀態（HUD 顯示用）
#[derive(Resource)]
pub struct PlayerStats {
    #[allow(dead_code)]
    pub health: f32,
    #[allow(dead_code)]
    pub max_health: f32,
    pub money: u32,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            health: 100.0,
            max_health: 100.0,
            money: 5000,
        }
    }
}

/// 互動輸入狀態（F 鍵）
#[derive(Resource, Default)]
pub struct InteractionState {
    pub pressed: bool,
    pub consumed: bool,
}

impl InteractionState {
    /// 更新狀態
    pub fn update(&mut self, pressed: bool) {
        self.pressed = pressed;
        self.consumed = false;
    }

    /// 是否可互動
    pub fn can_interact(&self) -> bool {
        self.pressed && !self.consumed
    }

    /// 消耗冷卻
    pub fn consume(&mut self) {
        self.consumed = true;
    }
}

/// 每幀更新互動輸入狀態
pub fn update_interaction_state(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut interaction: ResMut<InteractionState>,
) {
    interaction.update(keyboard.just_pressed(KeyCode::KeyF));
}

/// 互動系統優先序
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum InteractionSet {
    Vehicle,
    Mission,
    Economy,
    Interior,
}

/// Debug 設定（F3 切換）
#[derive(Resource, Default)]
pub struct DebugSettings {
    /// 顯示 AI 視野範圍
    pub show_ai_ranges: bool,
}

// ============================================================================
// 碰撞群組定義
// ============================================================================
// 使用 bevy_rapier3d 的 Group 類型
// 統一定義以確保所有實體使用一致的碰撞規則

use bevy_rapier3d::prelude::Group;

/// 碰撞群組：角色（玩家、敵人）
pub const COLLISION_GROUP_CHARACTER: Group = Group::GROUP_1;

/// 碰撞群組：載具（機車、汽車、公車）
pub const COLLISION_GROUP_VEHICLE: Group = Group::GROUP_2;

/// 碰撞群組：靜態物體（建築、街道傢俱）
pub const COLLISION_GROUP_STATIC: Group = Group::GROUP_3;

/// 碰撞群組：子彈/投射物（預留）
#[allow(dead_code)]
pub const COLLISION_GROUP_PROJECTILE: Group = Group::GROUP_4;

// ============================================================================
// 通用工具函數
// ============================================================================

/// 計算基於生命週期的淡出透明度
///
/// # 參數
/// * `progress` - 當前進度 (0.0 = 開始, 1.0 = 結束)
/// * `fade_start` - 開始淡出的進度比例 (例如 0.7 表示在 70% 時開始淡出)
///
/// # 返回
/// 透明度值 (0.0 = 完全透明, 1.0 = 完全不透明)
#[inline]
pub fn calculate_fade_alpha(progress: f32, fade_start: f32) -> f32 {
    if progress < fade_start {
        1.0
    } else {
        1.0 - (progress - fade_start) / (1.0 - fade_start)
    }
}

/// 從 lifetime/max_lifetime 計算淡出透明度
///
/// 組合 progress 計算與 `calculate_fade_alpha`，避免重複的 progress 計算程式碼。
#[inline]
pub fn lifetime_fade_alpha(lifetime: f32, max_lifetime: f32, fade_start: f32) -> f32 {
    let progress = if max_lifetime > 0.0 {
        (lifetime / max_lifetime).clamp(0.0, 1.0)
    } else {
        1.0
    };
    calculate_fade_alpha(progress, fade_start)
}

/// 從 lifetime/max_lifetime 計算線性淡出透明度（1.0 → 0.0）
#[inline]
pub fn lifetime_linear_alpha(lifetime: f32, max_lifetime: f32) -> f32 {
    let progress = if max_lifetime > 0.0 {
        (lifetime / max_lifetime).clamp(0.0, 1.0)
    } else {
        1.0
    };
    (1.0 - progress).max(0.0)
}

// ============================================================================
// 緩動函數 (Easing Functions)
// ============================================================================

/// 二次緩動 - 加速
#[inline]
pub fn ease_in_quad(t: f32) -> f32 {
    t * t
}

/// 二次緩動 - 減速
#[inline]
pub fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

/// 二次緩動 - 先加速後減速
#[inline]
pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// 三次緩動 - 加速
#[inline]
pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

/// 三次緩動 - 減速
#[inline]
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// 三次緩動 - 先加速後減速
#[inline]
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}
