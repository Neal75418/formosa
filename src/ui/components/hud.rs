//! HUD 組件 — 玩家狀態、控制提示、動畫狀態、文字陰影

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

/// 控制提示文字
#[derive(Component)]
pub struct UiText;

/// 時間顯示
#[derive(Component)]
pub struct TimeDisplay;

/// 金錢顯示
#[derive(Component)]
pub struct MoneyDisplay;

/// 血量條背景
#[derive(Component)]
pub struct HealthBarBg;

/// 血量條
#[derive(Component)]
pub struct HealthBar;

/// 任務資訊
#[derive(Component)]
pub struct MissionInfo;

// ============================================================================
// GTA 風格 HUD 組件
// ============================================================================

/// 玩家狀態容器（左下角）
#[derive(Component)]
pub struct PlayerStatusContainer;

/// 血量條填充
#[derive(Component)]
pub struct HealthBarFill;

/// 血量條高光（模擬漸層效果）
#[derive(Component)]
pub struct HealthBarHighlight;

/// 血量數值標籤
#[derive(Component)]
pub struct HealthLabel;

/// 護甲區塊（可切換可見性）
#[derive(Component)]
pub struct ArmorSection;

/// 護甲條填充
#[derive(Component)]
pub struct ArmorBarFill;

/// 護甲數值標籤
#[derive(Component)]
pub struct ArmorLabel;

// ============================================================================
// GTA 風格控制提示組件
// ============================================================================

/// 控制提示容器
#[derive(Component)]
pub struct ControlHintContainer;

/// 控制提示狀態標籤（步行/駕駛）
#[derive(Component)]
pub struct ControlStatusTag;

/// 控制提示速度顯示
#[derive(Component)]
pub struct ControlSpeedDisplay;

/// 控制提示按鍵區域
#[derive(Component)]
pub struct ControlKeyArea;

// ============================================================================
// GTA 風格動畫組件
// ============================================================================

/// HUD 動畫狀態資源
#[derive(Resource)]
pub struct HudAnimationState {
    /// 低血量脈衝相位 (0.0 ~ TAU)
    pub low_health_pulse_phase: f32,
    /// 小地圖掃描線位置 (0.0 ~ 1.0)
    pub minimap_scan_position: f32,
    /// 玩家標記脈衝相位
    pub player_marker_pulse_phase: f32,
}

impl Default for HudAnimationState {
    fn default() -> Self {
        Self {
            low_health_pulse_phase: 0.0,
            minimap_scan_position: 0.0,
            player_marker_pulse_phase: 0.0,
        }
    }
}

/// 血量條外發光層（用於低血量脈衝）
#[derive(Component)]
pub struct HealthBarGlow;

// ============================================================================
// 文字陰影組件
// ============================================================================

/// 血量標籤陰影
#[derive(Component)]
pub struct HealthLabelShadow;

/// 護甲標籤陰影
#[derive(Component)]
pub struct ArmorLabelShadow;

/// 武器名稱陰影
#[derive(Component)]
pub struct WeaponDisplayShadow;

/// 當前彈藥陰影
#[derive(Component)]
pub struct CurrentAmmoShadow;

/// 後備彈藥陰影
#[derive(Component)]
pub struct ReserveAmmoShadow;

// ============================================================================
// GTA 風格電台 UI 組件（右上角）
// ============================================================================

/// 電台顯示容器（右上角，小地圖下方）
#[derive(Component)]
pub struct RadioDisplayContainer;

/// 電台圖示
#[derive(Component)]
pub struct RadioIcon;

/// 電台名稱文字
#[derive(Component)]
pub struct RadioStationName;

/// 電台頻率標籤
#[derive(Component)]
pub struct RadioFrequency;

/// 電台描述文字
#[derive(Component)]
pub struct RadioDescription;

/// 音量條背景
#[derive(Component)]
pub struct RadioVolumeBarBg;

/// 音量條填充
#[derive(Component)]
pub struct RadioVolumeBarFill;
