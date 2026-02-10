//! 武器組件 — 彈藥顯示、武器槽、武器輪盤

#![allow(dead_code)]

use bevy::prelude::*;

/// 彈藥顯示
#[derive(Component)]
pub struct AmmoDisplay;

/// 彈藥視覺化網格容器
#[derive(Component)]
pub struct AmmoVisualGrid;

/// 彈藥子彈圖示
#[derive(Component)]
pub struct AmmoBulletIcon {
    /// 在彈匣中的索引位置
    pub index: usize,
}

/// 當前武器顯示
#[derive(Component)]
pub struct WeaponDisplay;

/// 武器槽位指示器
#[derive(Component)]
pub struct WeaponSlot {
    pub slot_index: usize,
}

/// 當前彈藥數文字
#[derive(Component)]
pub struct CurrentAmmoText;

/// 後備彈藥數文字
#[derive(Component)]
pub struct ReserveAmmoText;

/// 武器區容器（右下角）
#[derive(Component)]
pub struct WeaponAreaContainer;

/// 武器切換動畫狀態資源
#[derive(Resource)]
pub struct WeaponSwitchAnimation {
    /// 是否正在切換中
    pub is_switching: bool,
    /// 切換進度 (0.0 ~ 1.0)
    pub progress: f32,
    /// 切換動畫持續時間（秒）
    pub duration: f32,
    /// 上一把武器索引（用於檢測切換）
    pub last_weapon_index: usize,
}

impl Default for WeaponSwitchAnimation {
    fn default() -> Self {
        Self {
            is_switching: false,
            progress: 0.0,
            duration: 0.25,
            last_weapon_index: 0,
        }
    }
}

// ============================================================================
// 武器輪盤組件 (GTA 5 風格)
// ============================================================================

/// 武器輪盤容器
#[derive(Component)]
pub struct WeaponWheel;

/// 武器輪盤背景
#[derive(Component)]
pub struct WeaponWheelBackground;

/// 武器輪盤中心資訊
#[derive(Component)]
pub struct WeaponWheelCenterInfo;

/// 武器名稱文字（中央）
#[derive(Component)]
pub struct WeaponWheelName;

/// 武器彈藥文字（中央）
#[derive(Component)]
pub struct WeaponWheelAmmo;

/// 武器輪盤槽位
#[derive(Component)]
pub struct WeaponWheelSlot {
    /// 槽位索引 (0-5)
    pub index: usize,
    /// 槽位角度（弧度）
    pub angle: f32,
    /// 是否選中
    pub is_selected: bool,
}

/// 武器輪盤圖示
#[derive(Component)]
pub struct WeaponWheelIcon {
    pub slot_index: usize,
}

/// 武器輪盤選擇指示器
#[derive(Component)]
pub struct WeaponWheelSelector;

/// 武器輪盤狀態資源
#[derive(Resource)]
pub struct WeaponWheelState {
    /// 當前選中的槽位索引
    pub selected_index: usize,
    /// 滑鼠相對中心的角度（弧度）
    pub mouse_angle: f32,
    /// 滑鼠相對中心的距離
    pub mouse_distance: f32,
    /// 選擇死區半徑（像素）
    pub dead_zone: f32,
    /// 輪盤半徑（像素）
    pub radius: f32,
    /// 時間縮放（慢動作效果）
    pub time_scale: f32,
    /// 輪盤打開動畫進度 (0.0 ~ 1.0)
    pub open_progress: f32,
    /// 是否正在打開/關閉動畫中
    pub is_animating: bool,
}

impl Default for WeaponWheelState {
    fn default() -> Self {
        Self {
            selected_index: 0,
            mouse_angle: 0.0,
            mouse_distance: 0.0,
            dead_zone: 40.0,
            radius: 180.0,
            time_scale: 0.3,  // 武器輪盤時減慢到 30%
            open_progress: 0.0,
            is_animating: false,
        }
    }
}

impl WeaponWheelState {
    /// 根據角度計算選中的槽位
    /// 6 個槽位，每個佔 60 度 (π/3 弧度)
    /// 從正上方開始 (角度 = -π/2)
    pub fn angle_to_slot(&self, angle: f32) -> usize {
        // 調整角度使正上方為 0
        let adjusted = angle + std::f32::consts::FRAC_PI_2;
        // 標準化到 0 ~ 2π
        let normalized = if adjusted < 0.0 {
            adjusted + std::f32::consts::TAU
        } else {
            adjusted
        };
        // 計算槽位
        let slot = (normalized / (std::f32::consts::TAU / 6.0)).floor() as usize;
        slot.min(5)  // 確保不超過 5
    }

    /// 取得槽位的中心角度
    pub fn slot_angle(index: usize) -> f32 {
        // 從正上方開始，順時針分佈
        -std::f32::consts::FRAC_PI_2 + (index as f32 * std::f32::consts::TAU / 6.0)
    }

    /// 更新滑鼠位置和選中槽位
    pub fn update_selection(&mut self, mouse_offset: Vec2) {
        self.mouse_distance = mouse_offset.length();
        if self.mouse_distance > self.dead_zone {
            self.mouse_angle = mouse_offset.y.atan2(mouse_offset.x);
            self.selected_index = self.angle_to_slot(self.mouse_angle);
        }
    }
}
