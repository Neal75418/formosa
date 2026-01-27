//! UI 組件

use bevy::prelude::*;
use crate::core::calculate_fade_alpha;

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

/// 小地圖容器
#[derive(Component)]
pub struct MinimapContainer;

/// 小地圖玩家標記
#[derive(Component)]
pub struct MinimapPlayerMarker;

/// 暫停選單
#[derive(Component)]
pub struct PauseMenu;

/// 繼續遊戲按鈕
#[derive(Component)]
pub struct ResumeButton;

/// 退出遊戲按鈕
#[derive(Component)]
pub struct QuitButton;

/// 按鈕縮放動畫狀態
#[derive(Component)]
pub struct ButtonScaleState {
    /// 目標縮放值
    pub target: f32,
    /// 當前縮放值
    pub current: f32,
}

impl Default for ButtonScaleState {
    fn default() -> Self {
        Self {
            target: 1.0,
            current: 1.0,
        }
    }
}

/// 大地圖容器
#[derive(Component)]
pub struct FullMapContainer;

/// 大地圖玩家標記
#[derive(Component)]
pub struct FullMapPlayerMarker;

// === 外送 App UI 組件 ===

/// 外送 App 容器
#[derive(Component)]
pub struct DeliveryAppContainer;

/// 外送訂單卡片
#[derive(Component)]
pub struct DeliveryOrderCard {
    pub order_index: usize,
}

/// 外送訂單列表容器
#[derive(Component)]
pub struct DeliveryOrderList;

/// 外送狀態面板（進行中的訂單）
#[derive(Component)]
pub struct DeliveryStatusPanel;

/// 外送評價顯示
#[derive(Component)]
pub struct DeliveryRatingDisplay;

/// 外送連擊顯示
#[derive(Component)]
pub struct DeliveryStreakDisplay;

/// UI 遊戲狀態
#[derive(Resource)]
pub struct UiState {
    pub paused: bool,
    pub show_full_map: bool,
    pub minimap_zoom: f32,       // 小地圖縮放倍率 (0.5 ~ 2.0)
    pub show_delivery_app: bool, // 是否顯示外送 App
    pub show_weapon_wheel: bool, // 是否顯示武器輪盤
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            paused: false,
            show_full_map: false,
            minimap_zoom: 1.0,
            show_delivery_app: false,
            show_weapon_wheel: false,
        }
    }
}

/// 中文字體資源
#[derive(Resource)]
pub struct ChineseFont {
    pub font: Handle<Font>,
}

// === 戰鬥 UI 組件 ===

/// 準星容器
#[derive(Component)]
pub struct Crosshair;

/// 準星中心點
#[derive(Component)]
pub struct CrosshairDot;

/// 準星線條
#[derive(Component)]
pub struct CrosshairLine {
    pub direction: CrosshairDirection,
}

/// 準星線條方向
#[derive(Clone, Copy)]
pub enum CrosshairDirection {
    Top,
    Bottom,
    Left,
    Right,
}

/// 準星外圈（GTA 風格）
#[derive(Component)]
pub struct CrosshairOuterRing;

/// 準星命中標記（X 形）
#[derive(Component)]
pub struct CrosshairHitMarker;

/// 準星命中標記線條
#[derive(Component)]
pub struct HitMarkerLine;

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

// === 敵人 UI 組件 ===

/// 敵人血條（世界空間）
#[derive(Component)]
pub struct EnemyHealthBar {
    /// 對應的敵人實體
    pub enemy_entity: Entity,
}

/// 敵人血條填充
#[derive(Component)]
pub struct EnemyHealthBarFill {
    /// 對應的敵人實體
    pub enemy_entity: Entity,
}

/// 敵人血條高光
#[derive(Component)]
pub struct EnemyHealthBarHighlight {
    /// 對應的敵人實體
    pub enemy_entity: Entity,
}

/// 敵人血條外框（用於外發光）
#[derive(Component)]
pub struct EnemyHealthBarGlow {
    /// 對應的敵人實體
    pub enemy_entity: Entity,
}

// === 受傷指示器組件 ===

/// 受傷指示器容器（螢幕邊緣暈影）
#[derive(Component)]
pub struct DamageIndicator;

/// 受傷指示器邊緣（上下左右）
#[derive(Component)]
pub struct DamageIndicatorEdge {
    pub edge: DamageEdge,
}

/// 傷害邊緣方向
#[derive(Clone, Copy)]
pub enum DamageEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// 受傷指示器狀態
#[derive(Resource)]
pub struct DamageIndicatorState {
    /// 當前顯示強度 (0.0 ~ 1.0)
    pub intensity: f32,
    /// 上次受傷時間
    pub last_damage_time: f32,
    /// 傷害來源方向（如果有）
    pub damage_direction: Option<Vec2>,
}

impl Default for DamageIndicatorState {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            last_damage_time: 0.0,
            damage_direction: None,
        }
    }
}

// === GTA 風格 HUD 組件 ===

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

// === GTA 風格控制提示組件 ===

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

// === GTA 風格動畫組件 ===

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

/// 準星動態狀態資源
#[derive(Resource)]
pub struct CrosshairDynamics {
    /// 當前散佈值 (射擊時增加，恢復時減少)
    pub current_spread: f32,
    /// 目標散佈值 (瞄準時較小)
    pub target_spread: f32,
    /// 命中反彈縮放
    pub hit_bounce_scale: f32,
}

impl Default for CrosshairDynamics {
    fn default() -> Self {
        Self {
            current_spread: 1.0,
            target_spread: 1.0,
            hit_bounce_scale: 1.0,
        }
    }
}

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

/// 文字陰影標記（用於陰影層）
#[derive(Component)]
pub struct TextShadowLayer;

/// 血量條外發光層（用於低血量脈衝）
#[derive(Component)]
pub struct HealthBarGlow;

/// 小地圖掃描線
#[derive(Component)]
pub struct MinimapScanLine;

/// 小地圖玩家標記外發光
#[derive(Component)]
pub struct MinimapPlayerGlow;

// === GPS 導航系統組件 (GTA5 風格) ===

/// GPS 導航狀態資源
#[derive(Resource, Default)]
pub struct GpsNavigationState {
    /// 是否啟用導航
    pub active: bool,
    /// 目標位置（3D 世界座標）
    pub destination: Option<Vec3>,
    /// 目標名稱（顯示用）
    pub destination_name: String,
    /// 計算出的路線點
    pub route_waypoints: Vec<Vec3>,
    /// 到目標的預估距離
    pub distance_to_target: f32,
    /// 下一個轉彎點
    pub next_turn_point: Option<Vec3>,
    /// 路線計算冷卻（避免每幀重算）
    pub route_recalc_cooldown: f32,
}

impl GpsNavigationState {
    /// 設置新目標
    pub fn set_destination(&mut self, position: Vec3, name: &str) {
        self.active = true;
        self.destination = Some(position);
        self.destination_name = name.to_string();
        self.route_waypoints.clear();
        self.route_recalc_cooldown = 0.0;
    }

    /// 清除導航
    pub fn clear(&mut self) {
        self.active = false;
        self.destination = None;
        self.destination_name.clear();
        self.route_waypoints.clear();
        self.distance_to_target = 0.0;
        self.next_turn_point = None;
    }

    /// 檢查是否到達目標
    pub fn is_at_destination(&self, current_pos: Vec3, threshold: f32) -> bool {
        if let Some(dest) = self.destination {
            let dist_xz = ((current_pos.x - dest.x).powi(2) + (current_pos.z - dest.z).powi(2)).sqrt();
            dist_xz < threshold
        } else {
            false
        }
    }
}

/// 小地圖上的 GPS 目標標記
#[derive(Component)]
pub struct MinimapGpsMarker;

/// 小地圖上的 GPS 路線段
#[derive(Component)]
pub struct MinimapGpsRouteLine {
    pub segment_index: usize,
}

/// 屏幕上的方向指示箭頭
#[derive(Component)]
pub struct GpsDirectionArrow;

/// 屏幕上的距離顯示
#[derive(Component)]
pub struct GpsDistanceDisplay;

// === 文字陰影組件 ===

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

// === 天氣 HUD 組件 ===

/// 天氣 HUD 容器
#[derive(Component)]
pub struct WeatherHudContainer;

/// 天氣圖示容器
#[derive(Component)]
pub struct WeatherIconContainer;

/// 天氣圖示元素（根據類型顯示/隱藏）
#[derive(Component)]
pub struct WeatherIconElement {
    pub weather_type: WeatherIconType,
}

/// 天氣圖示類型
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WeatherIconType {
    Sun,
    Cloud,
    Rain,
    Fog,
}

/// 太陽光芒（動畫用）
#[derive(Component)]
pub struct SunRay {
    pub index: usize,
}

/// 雨滴圖示（動畫用）
#[derive(Component)]
pub struct RainDropIcon {
    pub index: usize,
    pub offset: f32,
}

/// 天氣名稱文字
#[derive(Component)]
pub struct WeatherNameText;

// === 浮動傷害數字組件 ===

/// 浮動傷害數字（世界空間）
/// GTA 5 風格：傷害數字從敵人位置向上飄動並淡出
#[derive(Component)]
pub struct FloatingDamageNumber {
    /// 初始位置
    pub start_position: Vec3,
    /// 傷害數值
    pub damage: f32,
    /// 是否為爆頭
    pub is_headshot: bool,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 初始縮放
    pub initial_scale: f32,
    /// 向上漂浮速度
    pub float_speed: f32,
    /// 水平偏移（避免重疊）
    pub horizontal_offset: f32,
}

impl FloatingDamageNumber {
    pub fn new(position: Vec3, damage: f32, is_headshot: bool) -> Self {
        Self {
            start_position: position,
            damage,
            is_headshot,
            lifetime: 0.0,
            max_lifetime: 1.2,  // 1.2 秒後消失
            initial_scale: if is_headshot { 1.4 } else { 1.0 },  // 爆頭更大
            float_speed: 2.0,  // 每秒上升 2 米
            horizontal_offset: 0.0,
        }
    }

    /// 設置水平偏移（避免多個數字重疊）
    pub fn with_offset(mut self, offset: f32) -> Self {
        self.horizontal_offset = offset;
        self
    }

    /// 計算當前透明度 (0.0 ~ 1.0)
    pub fn alpha(&self) -> f32 {
        let progress = self.lifetime / self.max_lifetime;
        calculate_fade_alpha(progress, 0.3)
    }

    /// 計算當前縮放
    pub fn scale(&self) -> f32 {
        let progress = self.lifetime / self.max_lifetime;
        // 彈出效果：開始時放大，然後縮小
        if progress < 0.1 {
            self.initial_scale * (1.0 + progress * 3.0)  // 快速放大
        } else {
            self.initial_scale * (1.3 - progress * 0.3)  // 緩慢縮小
        }
    }

    /// 計算當前 Y 偏移
    pub fn y_offset(&self) -> f32 {
        self.lifetime * self.float_speed
    }
}

/// 浮動傷害數字追蹤器（控制同時顯示的數量）
#[derive(Resource, Default)]
pub struct FloatingDamageTracker {
    /// 當前顯示的傷害數字數量
    pub active_count: usize,
    /// 最大同時顯示數量
    pub max_count: usize,
    /// 上次生成時的偏移方向（用於交替左右）
    pub last_offset_direction: f32,
}

impl FloatingDamageTracker {
    pub fn new() -> Self {
        Self {
            active_count: 0,
            max_count: 15,  // 最多同時 15 個
            last_offset_direction: 1.0,
        }
    }

    /// 取得下一個偏移方向並切換
    pub fn next_offset(&mut self) -> f32 {
        self.last_offset_direction *= -1.0;
        self.last_offset_direction * 0.3  // 左右偏移 0.3 米
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

// ============================================================================
// 互動提示 UI 組件
// ============================================================================

/// 互動提示容器（螢幕中央偏下）
#[derive(Component)]
pub struct InteractionPromptContainer;

/// 互動提示按鍵框
#[derive(Component)]
pub struct InteractionPromptKey;

/// 互動提示文字
#[derive(Component)]
pub struct InteractionPromptText;

/// 互動提示狀態資源
#[derive(Resource)]
pub struct InteractionPromptState {
    /// 是否顯示提示
    pub visible: bool,
    /// 當前提示文字
    pub text: String,
    /// 按鍵提示
    pub key: String,
    /// 淡入淡出進度 (0.0 ~ 1.0)
    pub fade_progress: f32,
    /// 目標可見度
    pub target_visibility: f32,
}

impl Default for InteractionPromptState {
    fn default() -> Self {
        Self {
            visible: false,
            text: String::new(),
            key: "F".to_string(),
            fade_progress: 0.0,
            target_visibility: 0.0,
        }
    }
}

impl InteractionPromptState {
    /// 顯示提示
    pub fn show(&mut self, text: impl Into<String>, key: impl Into<String>) {
        self.visible = true;
        self.text = text.into();
        self.key = key.into();
        self.target_visibility = 1.0;
    }

    /// 隱藏提示
    pub fn hide(&mut self) {
        self.target_visibility = 0.0;
    }

    /// 更新淡入淡出
    pub fn update(&mut self, delta: f32) {
        let speed = 8.0;
        if self.fade_progress < self.target_visibility {
            self.fade_progress = (self.fade_progress + speed * delta).min(self.target_visibility);
        } else if self.fade_progress > self.target_visibility {
            self.fade_progress = (self.fade_progress - speed * delta).max(self.target_visibility);
        }

        // 完全淡出後設為不可見
        if self.fade_progress <= 0.01 && self.target_visibility <= 0.0 {
            self.visible = false;
        }
    }
}

// ============================================================================
// 劇情任務 HUD 組件 (GTA 5 風格)
// ============================================================================

/// 劇情任務 HUD 容器（右上角）
#[derive(Component)]
pub struct StoryMissionHud;

/// 任務標題文字
#[derive(Component)]
pub struct StoryMissionTitle;

/// 任務階段描述文字
#[derive(Component)]
pub struct StoryMissionPhaseText;

/// 任務目標列表容器
#[derive(Component)]
pub struct StoryMissionObjectiveList;

/// 任務目標項目
#[derive(Component)]
pub struct StoryMissionObjectiveItem {
    /// 目標索引
    pub index: usize,
}

/// 任務目標勾選框
#[derive(Component)]
pub struct StoryMissionObjectiveCheck {
    pub index: usize,
}

/// 任務目標文字
#[derive(Component)]
pub struct StoryMissionObjectiveText {
    pub index: usize,
}

/// 任務計時器顯示
#[derive(Component)]
pub struct StoryMissionTimer;

/// 任務進度條背景
#[derive(Component)]
pub struct StoryMissionProgressBg;

/// 任務進度條填充
#[derive(Component)]
pub struct StoryMissionProgressFill;

// ============================================================================
// 任務失敗/結果 UI
// ============================================================================

/// 任務失敗 UI 狀態
#[derive(Resource, Default)]
pub struct MissionFailState {
    /// 是否顯示失敗畫面
    pub is_showing: bool,
    /// 失敗原因
    pub fail_reason: Option<String>,
    /// 是否可從檢查點重試
    pub can_retry: bool,
    /// 當前選項（0 = 重試, 1 = 放棄）
    pub selected_option: usize,
    /// 顯示計時器（用於淡入動畫）
    pub show_timer: f32,
}

impl MissionFailState {
    pub fn show(&mut self, reason: String, can_retry: bool) {
        self.is_showing = true;
        self.fail_reason = Some(reason);
        self.can_retry = can_retry;
        self.selected_option = 0;
        self.show_timer = 0.0;
    }

    pub fn hide(&mut self) {
        self.is_showing = false;
        self.fail_reason = None;
        self.can_retry = false;
        self.show_timer = 0.0;
    }
}

/// 任務完成結果 UI 狀態
#[derive(Resource, Default)]
pub struct MissionResultState {
    /// 是否顯示結果畫面
    pub is_showing: bool,
    /// 顯示計時器（用於動畫）
    pub show_timer: f32,
    /// 是否已確認（按下任意鍵後）
    pub confirmed: bool,
}

impl MissionResultState {
    pub fn show(&mut self) {
        self.is_showing = true;
        self.show_timer = 0.0;
        self.confirmed = false;
    }

    pub fn hide(&mut self) {
        self.is_showing = false;
        self.show_timer = 0.0;
        self.confirmed = false;
    }
}

/// 任務失敗 UI 容器標記
#[derive(Component)]
pub struct MissionFailUI;

/// 任務失敗標題
#[derive(Component)]
pub struct MissionFailTitle;

/// 任務失敗原因文字
#[derive(Component)]
pub struct MissionFailReason;

/// 任務失敗選項（重試/放棄）
#[derive(Component)]
pub struct MissionFailOption {
    pub index: usize,
}

/// 任務結果 UI 容器標記
#[derive(Component)]
pub struct MissionResultUI;

/// 任務結果標題
#[derive(Component)]
pub struct MissionResultTitle;

/// 任務結果評分星星
#[derive(Component)]
pub struct MissionResultStars;

/// 任務結果統計
#[derive(Component)]
pub struct MissionResultStats;

/// 任務結果獎勵
#[derive(Component)]
pub struct MissionResultReward;

