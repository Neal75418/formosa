//! UI 系統

use super::{AmmoBulletIcon, AmmoVisualGrid};
use super::{
    ArmorBarFill, ArmorLabel, ArmorSection, ControlHintContainer, ControlKeyArea,
    ControlSpeedDisplay, ControlStatusTag, CurrentAmmoText, HealthBarFill, HealthBarHighlight,
    HealthLabel, PlayerStatusContainer, ReserveAmmoText, WeaponAreaContainer, WeaponSlot,
};
use super::{
    ArmorLabelShadow, CurrentAmmoShadow, HealthLabelShadow, ReserveAmmoShadow, WeaponDisplayShadow,
};
use super::{
    ChineseFont, DamageEdge, DamageIndicator, DamageIndicatorEdge, DamageIndicatorState,
    EnemyHealthBar, EnemyHealthBarFill, EnemyHealthBarGlow, EnemyHealthBarHighlight,
    FullMapContainer, FullMapPlayerMarker, HealthBarBg, MinimapContainer, MinimapPlayerMarker,
    MissionInfo, MoneyDisplay, PauseMenu, QuitButton, ResumeButton, TimeDisplay, UiState, UiText,
};
use super::{
    CrosshairDynamics, HealthBarGlow, HudAnimationState, MinimapPlayerGlow, MinimapScanLine,
    WeaponSwitchAnimation,
};
use super::{
    RainDropIcon, SunRay, WeatherHudContainer, WeatherIconContainer, WeatherIconElement,
    WeatherIconType, WeatherNameText,
};
use super::{
    StoryMissionHud, StoryMissionObjectiveCheck, StoryMissionObjectiveItem,
    StoryMissionObjectiveList, StoryMissionObjectiveText, StoryMissionPhaseText, StoryMissionTimer,
    StoryMissionTitle,
};
use super::{
    WeaponWheel, WeaponWheelAmmo, WeaponWheelBackground, WeaponWheelCenterInfo, WeaponWheelIcon,
    WeaponWheelName, WeaponWheelSelector, WeaponWheelSlot, WeaponWheelState,
};
use crate::combat::{Armor, Enemy, Health, WeaponInventory};
use crate::core::{AppState, GameState, WeatherState, WeatherType, WorldTime};
use crate::mission::{
    get_current_mission_info, MissionManager, MissionType, StoryMissionDatabase,
    StoryMissionManager,
};
use crate::player::Player;
use crate::vehicle::{Vehicle, VehicleType};
use crate::world::Building;
use bevy::prelude::*;

/// 載入中文字體
pub fn setup_chinese_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/STHeiti.ttc");
    commands.insert_resource(ChineseFont { font });
}

// === GTA 風格 HUD 顏色常數 ===

/// HUD 背景色（深藍黑）
const HUD_BG: Color = Color::srgba(0.05, 0.05, 0.1, 0.75);
/// HUD 邊框色
const HUD_BORDER: Color = Color::srgba(0.3, 0.3, 0.4, 0.5);

/// 血量圖示色（GTA 5 風格：較低飽和度）
const HEALTH_ICON: Color = Color::srgb(0.85, 0.2, 0.15);
/// 血量條背景
const HEALTH_BAR_BG: Color = Color::srgba(0.12, 0.04, 0.04, 0.85);
/// 血量條填充（GTA 5 風格：較深沉的紅）
const HEALTH_BAR_FILL_COLOR: Color = Color::srgb(0.75, 0.18, 0.12);
/// 血量條高光
const HEALTH_BAR_HIGHLIGHT_COLOR: Color = Color::srgb(0.9, 0.3, 0.2);

/// 護甲圖示色（GTA 5 風格：較低飽和度）
const ARMOR_ICON: Color = Color::srgb(0.35, 0.6, 0.85);
/// 護甲條背景
const ARMOR_BAR_BG: Color = Color::srgba(0.04, 0.08, 0.15, 0.85);
/// 護甲條填充（GTA 5 風格：較深沉的藍）
const ARMOR_BAR_FILL_COLOR: Color = Color::srgb(0.22, 0.5, 0.78);
/// 護甲條高光
const ARMOR_BAR_HIGHLIGHT_COLOR: Color = Color::srgb(0.4, 0.68, 0.88);

/// 金錢文字色（GTA 5 風格：較低飽和度的綠）
const MONEY_TEXT_COLOR: Color = Color::srgb(0.25, 0.85, 0.35);
/// 金錢背景色
const MONEY_BG: Color = Color::srgba(0.04, 0.08, 0.04, 0.85);

/// 彈藥正常色（GTA 5 風格：較柔和的金黃）
const AMMO_NORMAL: Color = Color::srgb(0.95, 0.88, 0.45);
/// 彈藥低量色（GTA 5 風格：較深沉的橙紅）
const AMMO_LOW: Color = Color::srgb(0.92, 0.4, 0.3);
/// 後備彈藥色
const AMMO_RESERVE: Color = Color::srgba(0.75, 0.75, 0.78, 0.85);

/// 子彈圖示填充色（金黃色）
const BULLET_FILLED: Color = Color::srgb(0.95, 0.85, 0.35);
/// 子彈圖示空彈色（暗灰）
const BULLET_EMPTY: Color = Color::srgba(0.25, 0.25, 0.3, 0.5);
/// 子彈圖示低彈量閃爍色（警示橙紅）
const BULLET_LOW_WARN: Color = Color::srgb(0.95, 0.4, 0.3);

/// 武器槽選中色
const SLOT_ACTIVE: Color = Color::srgba(0.4, 0.6, 0.9, 0.9);
/// 武器槽未選中色
const SLOT_INACTIVE: Color = Color::srgba(0.2, 0.2, 0.25, 0.6);

// === GTA 風格地圖顏色常數 ===

/// 小地圖外框發光色（雷達感）
const MINIMAP_GLOW: Color = Color::srgba(0.2, 0.5, 0.3, 0.4);
/// 小地圖主邊框色
const MINIMAP_BORDER: Color = Color::srgba(0.15, 0.35, 0.2, 0.95);
/// 小地圖內邊框色（陰影效果）
const MINIMAP_INNER_BORDER: Color = Color::srgba(0.05, 0.15, 0.08, 0.9);
/// 小地圖背景色（深綠軍事風）
const MINIMAP_BG: Color = Color::srgba(0.08, 0.12, 0.08, 0.92);
/// 小地圖內層背景（稍亮）
const MINIMAP_BG_INNER: Color = Color::srgba(0.1, 0.15, 0.1, 0.95);
/// 玩家標記發光色
const PLAYER_MARKER_GLOW: Color = Color::srgba(1.0, 0.9, 0.3, 0.6);
/// 玩家標記核心色
const PLAYER_MARKER_CORE: Color = Color::srgb(1.0, 0.15, 0.1);
/// 方位標示背景色
const COMPASS_BG: Color = Color::srgba(0.1, 0.1, 0.1, 0.7);
/// 北方標示色（紅色更醒目）
const COMPASS_NORTH: Color = Color::srgb(1.0, 0.3, 0.2);

/// 大地圖背景色
const FULLMAP_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.88);
/// 大地圖主體背景
const FULLMAP_MAIN_BG: Color = Color::srgb(0.12, 0.15, 0.1);
/// 大地圖邊框色
const FULLMAP_BORDER: Color = Color::srgb(0.35, 0.4, 0.3);
/// 大地圖標題背景
const FULLMAP_TITLE_BG: Color = Color::srgba(0.1, 0.15, 0.1, 0.9);

// === GTA 風格暫停選單顏色常數 ===

/// 暫停選單外層背景（毛玻璃效果第一層）
const PAUSE_BG_OUTER: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
/// 暫停選單內層背景（毛玻璃效果第二層）
const PAUSE_BG_INNER: Color = Color::srgba(0.02, 0.02, 0.05, 0.4);
/// 暫停選單面板外發光
const PAUSE_PANEL_GLOW: Color = Color::srgba(0.3, 0.35, 0.4, 0.3);
/// 暫停選單面板主邊框
const PAUSE_PANEL_BORDER: Color = Color::srgba(0.4, 0.45, 0.5, 0.8);
/// 暫停選單面板內邊框
const PAUSE_PANEL_INNER_BORDER: Color = Color::srgba(0.15, 0.15, 0.2, 0.9);
/// 暫停選單面板背景
const PAUSE_PANEL_BG: Color = Color::srgba(0.08, 0.08, 0.12, 0.95);
/// 暫停選單標題色
const PAUSE_TITLE_COLOR: Color = Color::srgb(0.95, 0.95, 0.98);
/// 繼續按鈕正常色
const RESUME_BTN_NORMAL: Color = Color::srgb(0.15, 0.45, 0.25);
/// 繼續按鈕懸停色
const RESUME_BTN_HOVER: Color = Color::srgb(0.2, 0.6, 0.3);
/// 繼續按鈕按下色
const RESUME_BTN_PRESSED: Color = Color::srgb(0.1, 0.35, 0.18);
/// 繼續按鈕邊框色
const RESUME_BTN_BORDER: Color = Color::srgba(0.3, 0.7, 0.4, 0.8);
/// 退出按鈕正常色
const QUIT_BTN_NORMAL: Color = Color::srgb(0.5, 0.15, 0.15);
/// 退出按鈕懸停色
const QUIT_BTN_HOVER: Color = Color::srgb(0.7, 0.2, 0.2);
/// 退出按鈕按下色
const QUIT_BTN_PRESSED: Color = Color::srgb(0.4, 0.1, 0.1);
/// 退出按鈕邊框色
const QUIT_BTN_BORDER: Color = Color::srgba(0.8, 0.3, 0.3, 0.8);
/// 暫停選單提示文字色
const PAUSE_HINT_COLOR: Color = Color::srgba(0.6, 0.6, 0.65, 0.8);
/// 暫停選單副標題色
const PAUSE_SUBTITLE_COLOR: Color = Color::srgba(0.5, 0.5, 0.55, 0.7);

// === GTA 風格準星顏色常數 ===

/// 準星主色（白色微透明）
const CROSSHAIR_MAIN: Color = Color::srgba(1.0, 1.0, 1.0, 0.9);
/// 準星陰影色（輪廓）
const CROSSHAIR_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
/// 準星外圈色
const CROSSHAIR_OUTER_RING: Color = Color::srgba(1.0, 1.0, 1.0, 0.3);
/// 命中標記色（亮紅）
const HIT_MARKER_COLOR: Color = Color::srgba(1.0, 0.2, 0.2, 0.95);
/// 爆頭標記色（金黃）
const HEADSHOT_MARKER_COLOR: Color = Color::srgba(1.0, 0.85, 0.2, 1.0);
/// 準星瞄準時色（收縮變亮）
const CROSSHAIR_AIM: Color = Color::srgba(0.9, 1.0, 0.95, 0.95);

// === GTA 風格敵人血條顏色常數 ===

/// 敵人血條外發光（紅色輝光）
const ENEMY_BAR_GLOW: Color = Color::srgba(0.8, 0.2, 0.2, 0.3);
/// 敵人血條邊框色
const ENEMY_BAR_BORDER: Color = Color::srgba(0.1, 0.1, 0.12, 0.95);
/// 敵人血條背景色
const ENEMY_BAR_BG: Color = Color::srgba(0.05, 0.05, 0.08, 0.9);
/// 敵人血條滿血色（綠色）
const ENEMY_HEALTH_FULL: Color = Color::srgb(0.2, 0.8, 0.3);
/// 敵人血條中血色（黃色）
const ENEMY_HEALTH_MID: Color = Color::srgb(0.9, 0.8, 0.2);
/// 敵人血條低血色（紅色）
const ENEMY_HEALTH_LOW: Color = Color::srgb(0.9, 0.2, 0.2);
/// 敵人血條高光色
const ENEMY_BAR_HIGHLIGHT: Color = Color::srgba(1.0, 1.0, 1.0, 0.2);

// === 受傷指示器顏色常數 ===

/// 受傷指示器主色（血紅色暈影）
const DAMAGE_INDICATOR_COLOR: Color = Color::srgba(0.6, 0.0, 0.0, 0.0); // 基礎透明
/// 受傷指示器最大透明度
const DAMAGE_INDICATOR_MAX_ALPHA: f32 = 0.5;
/// 受傷指示器邊緣寬度
const DAMAGE_EDGE_WIDTH: f32 = 150.0;
/// 受傷指示器淡出速度
const DAMAGE_FADE_RATE: f32 = 2.0;

// === GTA 風格外送 App 顏色常數 ===

/// 外送 App 外發光色（橘色輝光）
const DELIVERY_APP_GLOW: Color = Color::srgba(0.9, 0.4, 0.1, 0.25);
/// 外送 App 主邊框色
const DELIVERY_APP_BORDER: Color = Color::srgb(0.9, 0.4, 0.1);
/// 外送 App 內邊框色
const DELIVERY_APP_INNER_BORDER: Color = Color::srgba(0.4, 0.2, 0.05, 0.9);
/// 外送 App 背景色
const DELIVERY_APP_BG: Color = Color::srgba(0.08, 0.06, 0.1, 0.95);
/// 外送 App 標題色
const DELIVERY_APP_TITLE: Color = Color::srgb(1.0, 0.5, 0.15);
/// 外送 App 副標題色
const DELIVERY_APP_SUBTITLE: Color = Color::srgba(0.7, 0.7, 0.7, 0.9);
/// 訂單卡片外發光
const ORDER_CARD_GLOW: Color = Color::srgba(0.5, 0.3, 0.1, 0.15);
/// 訂單卡片背景
const ORDER_CARD_BG: Color = Color::srgba(0.12, 0.1, 0.15, 0.92);
/// 訂單卡片邊框
const ORDER_CARD_BORDER: Color = Color::srgba(0.6, 0.35, 0.15, 0.6);
/// 訂單卡片懸停邊框
const ORDER_CARD_HOVER_BORDER: Color = Color::srgba(0.9, 0.5, 0.2, 0.9);
/// 餐廳名稱色
const RESTAURANT_NAME_COLOR: Color = Color::srgb(1.0, 0.85, 0.6);
/// 地址文字色
const ADDRESS_TEXT_COLOR: Color = Color::srgba(0.75, 0.75, 0.8, 0.9);
/// 報酬金額色（綠色）
const REWARD_TEXT_COLOR: Color = Color::srgb(0.3, 0.95, 0.4);
/// 評價星星色（金黃）
const RATING_STAR_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);
/// 連擊數字色（橘紅）
const STREAK_COLOR: Color = Color::srgb(1.0, 0.5, 0.2);

// === GTA 風格控制提示顏色常數 ===

/// 控制提示背景色
const CONTROL_HINT_BG: Color = Color::srgba(0.05, 0.05, 0.08, 0.85);
/// 控制提示邊框色
const CONTROL_HINT_BORDER: Color = Color::srgba(0.25, 0.25, 0.3, 0.7);
/// 按鍵圖示背景色
const KEY_ICON_BG: Color = Color::srgba(0.15, 0.15, 0.2, 0.95);
/// 按鍵圖示邊框色
const KEY_ICON_BORDER: Color = Color::srgba(0.4, 0.4, 0.5, 0.9);
/// 按鍵文字色
const KEY_TEXT_COLOR: Color = Color::srgb(0.95, 0.95, 0.98);
/// 動作說明文字色
const ACTION_TEXT_COLOR: Color = Color::srgba(0.75, 0.75, 0.8, 0.9);
/// 狀態標籤背景色（步行/駕駛）
const STATUS_TAG_BG: Color = Color::srgba(0.2, 0.4, 0.25, 0.9);
/// 速度顯示色
const SPEED_TEXT_COLOR: Color = Color::srgb(0.4, 0.9, 0.5);

/// 文字陰影色（GTA 風格深色陰影）
const TEXT_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.65);
/// 文字陰影偏移量（像素）
const TEXT_SHADOW_OFFSET: f32 = 1.5;

// === 多層邊框效果色系 ===
/// HUD 容器外發光色（藍色微光）
const HUD_GLOW_OUTER: Color = Color::srgba(0.3, 0.5, 0.8, 0.2);
/// HUD 邊框高亮色
const HUD_BORDER_HIGHLIGHT: Color = Color::srgba(0.5, 0.6, 0.75, 0.6);

// === 通用 UI 顏色常數（減少內聯重複）===
/// 深黑半透明背景（90%）
const OVERLAY_BLACK_90: Color = Color::srgba(0.0, 0.0, 0.0, 0.9);
/// 深黑半透明背景（70%）
const OVERLAY_BLACK_70: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
/// 深灰按鈕背景
const BUTTON_BG_DARK: Color = Color::srgba(0.2, 0.2, 0.25, 0.8);
/// 深灰按鈕邊框（70%）
const BUTTON_BORDER_GRAY_70: Color = Color::srgba(0.4, 0.4, 0.45, 0.7);
/// 深灰按鈕邊框（60%）
const BUTTON_BORDER_GRAY_60: Color = Color::srgba(0.4, 0.4, 0.45, 0.6);
/// 灰色文字色（90%）
const TEXT_GRAY_90: Color = Color::srgba(0.7, 0.7, 0.7, 0.9);
/// 淺灰文字色
const TEXT_LIGHT_GRAY: Color = Color::srgba(0.8, 0.8, 0.85, 0.9);
/// 次要文字色
const TEXT_SECONDARY: Color = Color::srgba(0.65, 0.65, 0.7, 0.9);
/// 低飽和灰色
const TEXT_MUTED: Color = Color::srgba(0.5, 0.5, 0.55, 0.8);
/// 面板邊框灰
const PANEL_BORDER_GRAY: Color = Color::srgba(0.3, 0.3, 0.35, 0.4);
/// 綠色地圖區塊
const MAP_AREA_GREEN: Color = Color::srgba(0.3, 0.4, 0.3, 0.1);
/// 亮白色
const TEXT_WHITE: Color = Color::srgb(0.95, 0.95, 0.95);
/// 金黃標題色
const TITLE_GOLD: Color = Color::srgb(1.0, 0.85, 0.0);
/// 誠品綠
const ESLITE_GREEN: Color = Color::srgb(0.2, 0.35, 0.25);

/// 設置 UI（使用中文字體）
pub fn setup_ui(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // === 地圖常數定義 ===
    // 常數已移至 world/setup.rs 且通過 spawn_map_layer 引用

    // === 左下角：GTA 風格玩家狀態區（多層邊框）===
    // 外發光層
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(56.0), // 留空間給控制提示
                left: Val::Px(16.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(HUD_GLOW_OUTER),
            BorderRadius::all(Val::Px(12.0)),
        ))
        .with_children(|glow| {
            // 主容器
            glow.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(HUD_BG),
                BorderColor::all(HUD_BORDER_HIGHLIGHT),
                BorderRadius::all(Val::Px(8.0)),
                PlayerStatusContainer,
            ))
            .with_children(|parent| {
                // === 血量區塊 ===
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        ..default()
                    },))
                    .with_children(|row| {
                        // 血量圖示（紅色圓角方塊 + 內圈模擬愛心）
                        row.spawn((
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(HEALTH_ICON),
                            BorderRadius::all(Val::Px(4.0)),
                        ))
                        .with_children(|icon| {
                            // 內圈高光
                            icon.spawn((
                                Node {
                                    width: Val::Px(8.0),
                                    height: Val::Px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
                                BorderRadius::all(Val::Px(4.0)),
                            ));
                        });

                        // 血量條外發光層（低血量時脈衝）
                        row.spawn((
                            Node {
                                width: Val::Px(186.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                            BorderRadius::all(Val::Px(6.0)),
                            HealthBarGlow,
                        ))
                        .with_children(|glow| {
                            // 血量條容器
                            glow.spawn((
                                Node {
                                    width: Val::Px(180.0),
                                    height: Val::Px(18.0),
                                    ..default()
                                },
                                BackgroundColor(HEALTH_BAR_BG),
                                BorderRadius::all(Val::Px(4.0)),
                                HealthBarBg,
                            ))
                            .with_children(|bar_bg| {
                                // 血量條填充
                                bar_bg.spawn((
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        position_type: PositionType::Absolute,
                                        ..default()
                                    },
                                    BackgroundColor(HEALTH_BAR_FILL_COLOR),
                                    BorderRadius::all(Val::Px(4.0)),
                                    HealthBarFill,
                                ));
                                // 血量條高光（模擬漸層）
                                bar_bg.spawn((
                                    Node {
                                        width: Val::Percent(100.0),
                                        height: Val::Px(6.0),
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(0.0),
                                        ..default()
                                    },
                                    BackgroundColor(HEALTH_BAR_HIGHLIGHT_COLOR),
                                    BorderRadius::top(Val::Px(4.0)),
                                    HealthBarHighlight,
                                ));
                            }); // 結束 bar_bg
                        }); // 結束 glow

                        // 血量數值標籤（帶陰影）
                        row.spawn((Node { ..default() },))
                            .with_children(|label_container| {
                                // 陰影層
                                label_container.spawn((
                                    Text::new("100/100"),
                                    TextFont {
                                        font_size: 14.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(TEXT_SHADOW_OFFSET),
                                        top: Val::Px(TEXT_SHADOW_OFFSET),
                                        ..default()
                                    },
                                    HealthLabelShadow,
                                ));
                                // 主文字
                                label_container.spawn((
                                    Text::new("100/100"),
                                    TextFont {
                                        font_size: 14.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    HealthLabel,
                                ));
                            });
                    });

                // === 護甲區塊（有護甲時才顯示）===
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },
                        Visibility::Hidden, // 預設隱藏
                        ArmorSection,
                    ))
                    .with_children(|row| {
                        // 護甲圖示（藍色圓角方塊 + 盾牌樣式）
                        row.spawn((
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(ARMOR_ICON),
                            BorderColor::all(Color::srgba(0.6, 0.85, 1.0, 0.8)),
                            BorderRadius::all(Val::Px(4.0)),
                        ))
                        .with_children(|icon| {
                            // 內部深色區塊（模擬盾牌）
                            icon.spawn((
                                Node {
                                    width: Val::Px(6.0),
                                    height: Val::Px(8.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.1, 0.3, 0.5, 0.6)),
                                BorderRadius::all(Val::Px(2.0)),
                            ));
                        });

                        // 護甲條容器
                        row.spawn((
                            Node {
                                width: Val::Px(180.0),
                                height: Val::Px(18.0),
                                ..default()
                            },
                            BackgroundColor(ARMOR_BAR_BG),
                            BorderRadius::all(Val::Px(4.0)),
                        ))
                        .with_children(|bar_bg| {
                            // 護甲條填充
                            bar_bg.spawn((
                                Node {
                                    width: Val::Percent(50.0), // 預設 50%
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                BackgroundColor(ARMOR_BAR_FILL_COLOR),
                                BorderRadius::all(Val::Px(4.0)),
                                ArmorBarFill,
                            ));
                            // 護甲條高光
                            bar_bg.spawn((
                                Node {
                                    width: Val::Percent(50.0),
                                    height: Val::Px(6.0),
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(0.0),
                                    ..default()
                                },
                                BackgroundColor(ARMOR_BAR_HIGHLIGHT_COLOR),
                                BorderRadius::top(Val::Px(4.0)),
                            ));
                        });

                        // 護甲數值標籤（帶陰影）
                        row.spawn((Node { ..default() },))
                            .with_children(|label_container| {
                                // 陰影層
                                label_container.spawn((
                                    Text::new("50/100"),
                                    TextFont {
                                        font_size: 14.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(TEXT_SHADOW_OFFSET),
                                        top: Val::Px(TEXT_SHADOW_OFFSET),
                                        ..default()
                                    },
                                    ArmorLabelShadow,
                                ));
                                // 主文字
                                label_container.spawn((
                                    Text::new("50/100"),
                                    TextFont {
                                        font_size: 14.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    ArmorLabel,
                                ));
                            });
                    });
            }); // 結束 PlayerStatusContainer
        }); // 結束外發光層

    // === 右上角：小地圖 (GTA 風格多層邊框) ===
    // 外層發光框
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(316.0), // 300 + 8*2 邊框
                height: Val::Px(316.0),
                padding: UiRect::all(Val::Px(4.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(MINIMAP_GLOW),
            BorderRadius::all(Val::Px(12.0)),
        ))
        .with_children(|glow| {
            // 主邊框層
            glow.spawn((
                Node {
                    width: Val::Px(308.0),
                    height: Val::Px(308.0),
                    padding: UiRect::all(Val::Px(3.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(MINIMAP_BORDER),
                BorderColor::all(MINIMAP_INNER_BORDER),
                BorderRadius::all(Val::Px(10.0)),
            ))
            .with_children(|frame| {
                // 實際地圖容器
                frame
                    .spawn((
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(300.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(MINIMAP_BG),
                        BorderRadius::all(Val::Px(6.0)),
                        MinimapContainer,
                    ))
                    .with_children(|parent| {
                        // 內層漸層效果（四角較亮模擬）
                        // 左上角高光
                        parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(0.0),
                                left: Val::Px(0.0),
                                width: Val::Px(60.0),
                                height: Val::Px(60.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.2, 0.3, 0.2, 0.15)),
                            BorderRadius::top_left(Val::Px(6.0)),
                        ));
                        // 右上角高光
                        parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(0.0),
                                right: Val::Px(0.0),
                                width: Val::Px(60.0),
                                height: Val::Px(60.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.2, 0.3, 0.2, 0.1)),
                            BorderRadius::top_right(Val::Px(6.0)),
                        ));

                        // 小地圖標題（帶背景）
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(4.0),
                                    left: Val::Px(4.0),
                                    padding: UiRect::new(
                                        Val::Px(6.0),
                                        Val::Px(6.0),
                                        Val::Px(2.0),
                                        Val::Px(2.0),
                                    ),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                                BorderRadius::all(Val::Px(4.0)),
                            ))
                            .with_children(|title_bg| {
                                title_bg.spawn((
                                    Text::new("西門町"),
                                    TextFont {
                                        font_size: 10.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::srgba(0.8, 0.95, 0.8, 0.9)),
                                ));
                            });

                        // === 地圖內容層 ===
                        let mm_scale = 0.9;
                        let mm_off_x = 150.0;
                        let mm_off_y = 150.0;
                        let mw_fac = 0.7;

                        spawn_map_layer(
                            parent,
                            mm_scale,
                            mm_off_x,
                            mm_off_y,
                            mw_fac,
                            false,
                            font.clone(),
                        );

                        // === 雷達掃描線（GTA 風格）===
                        parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Px(3.0),
                                top: Val::Percent(0.0),
                                left: Val::Px(0.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.4, 0.9, 0.5, 0.25)),
                            MinimapScanLine,
                        ));

                        // === 玩家標記（簡潔圓形+箭頭指針）===
                        // 注意：容器需要包含整個箭頭，旋轉才能正確工作
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(20.0),
                                    height: Val::Px(34.0), // 增加高度以包含箭頭
                                    left: Val::Px(140.0),
                                    top: Val::Px(133.0), // 調整位置，讓圓心在地圖中央
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    overflow: Overflow::visible(), // 允許子元素溢出
                                    ..default()
                                },
                                Transform::default(),
                                GlobalTransform::default(),
                                MinimapPlayerMarker,
                            ))
                            .with_children(|marker| {
                                // 淡白色外圈（脈衝動畫用）- 定位在容器下半部
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(18.0),
                                        height: Val::Px(18.0),
                                        left: Val::Px(1.0),
                                        top: Val::Px(15.0), // 在容器下半部
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
                                    BorderRadius::all(Val::Px(9.0)),
                                    MinimapPlayerGlow,
                                ));
                                // 黑色描邊圓
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(14.0),
                                        height: Val::Px(14.0),
                                        left: Val::Px(3.0),
                                        top: Val::Px(17.0),
                                        ..default()
                                    },
                                    BackgroundColor(OVERLAY_BLACK_90),
                                    BorderRadius::all(Val::Px(7.0)),
                                ));
                                // 白色主圓
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(10.0),
                                        height: Val::Px(10.0),
                                        left: Val::Px(5.0),
                                        top: Val::Px(19.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::WHITE),
                                    BorderRadius::all(Val::Px(5.0)),
                                ));
                                // 方向指示三角（黑色描邊）
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(10.0),
                                        height: Val::Px(16.0),
                                        left: Val::Px(5.0),
                                        top: Val::Px(2.0),
                                        ..default()
                                    },
                                    BackgroundColor(OVERLAY_BLACK_90),
                                    BorderRadius::top(Val::Px(5.0)),
                                ));
                                // 方向指示三角（白色內部）
                                marker.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(6.0),
                                        height: Val::Px(14.0),
                                        left: Val::Px(7.0),
                                        top: Val::Px(4.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::WHITE),
                                    BorderRadius::top(Val::Px(3.0)),
                                ));
                            });

                        // === 方位標示（帶圓角背景）===
                        // 北（紅色更醒目）
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(6.0),
                                    left: Val::Px(140.0),
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(COMPASS_BG),
                                BorderRadius::all(Val::Px(10.0)),
                            ))
                            .with_children(|bg| {
                                bg.spawn((
                                    Text::new("N"),
                                    TextFont {
                                        font_size: 13.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(COMPASS_NORTH),
                                ));
                            });
                        // 南
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    bottom: Val::Px(6.0),
                                    left: Val::Px(140.0),
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(COMPASS_BG),
                                BorderRadius::all(Val::Px(10.0)),
                            ))
                            .with_children(|bg| {
                                bg.spawn((
                                    Text::new("S"),
                                    TextFont {
                                        font_size: 13.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        // 東
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(140.0),
                                    right: Val::Px(6.0),
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(COMPASS_BG),
                                BorderRadius::all(Val::Px(10.0)),
                            ))
                            .with_children(|bg| {
                                bg.spawn((
                                    Text::new("E"),
                                    TextFont {
                                        font_size: 13.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        // 西
                        parent
                            .spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(140.0),
                                    left: Val::Px(6.0),
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(COMPASS_BG),
                                BorderRadius::all(Val::Px(10.0)),
                            ))
                            .with_children(|bg| {
                                bg.spawn((
                                    Text::new("W"),
                                    TextFont {
                                        font_size: 13.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });

                        // === 掃描線效果（模擬雷達感）===
                        for i in 0..6 {
                            parent.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    top: Val::Px(50.0 * i as f32),
                                    left: Val::Px(0.0),
                                    width: Val::Percent(100.0),
                                    height: Val::Px(1.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.3, 0.5, 0.3, 0.08)),
                            ));
                        }
                    });
            });
        });

    // === 小地圖下方：時間（帶背景）===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(332.0),   // 316 + 6 + 10
                right: Val::Px(110.0), // 對齊小地圖中央
                padding: UiRect::new(Val::Px(10.0), Val::Px(10.0), Val::Px(4.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.08, 0.05, 0.7)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("18:00"),
                TextFont {
                    font_size: 20.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.95, 0.9, 0.95)),
                TimeDisplay,
            ));
        });

    // === 小地圖下方：金錢顯示（GTA 風格）===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(365.0), // 時間下方
                right: Val::Px(10.0),
                padding: UiRect::new(Val::Px(12.0), Val::Px(12.0), Val::Px(6.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(MONEY_BG),
            BorderColor::all(HUD_BORDER),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("$ 5,000"),
                TextFont {
                    font_size: 28.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(MONEY_TEXT_COLOR),
                MoneyDisplay,
            ));
        });

    // === 任務資訊（小地圖下方） ===
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 16.0,
            font: font.clone(),
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.8, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(190.0),
            right: Val::Px(10.0),
            ..default()
        },
        MissionInfo,
    ));

    // === 左下角：控制提示（GTA 風格）===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::new(Val::Px(10.0), Val::Px(10.0), Val::Px(6.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(CONTROL_HINT_BG),
            BorderColor::all(CONTROL_HINT_BORDER),
            BorderRadius::all(Val::Px(4.0)),
            ControlHintContainer,
        ))
        .with_children(|parent| {
            // 狀態標籤（步行/駕駛）
            parent
                .spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(8.0),
                            Val::Px(8.0),
                            Val::Px(4.0),
                            Val::Px(4.0),
                        ),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(STATUS_TAG_BG),
                    BorderColor::all(Color::srgba(0.3, 0.6, 0.35, 0.8)),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|tag| {
                    tag.spawn((
                        Text::new("步行"),
                        TextFont {
                            font_size: 12.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(Color::srgb(0.85, 0.95, 0.85)),
                        ControlStatusTag,
                    ));
                });

            // 速度顯示（駕駛時顯示）
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(SPEED_TEXT_COLOR),
                ControlSpeedDisplay,
                Visibility::Hidden,
            ));

            // 按鍵提示區域
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(4.0),
                        ..default()
                    },
                    ControlKeyArea,
                ))
                .with_children(|keys| {
                    // 按鍵圖示 helper
                    let spawn_key = |keys: &mut ChildSpawnerCommands,
                                     key: &str,
                                     action: &str,
                                     font: Handle<Font>| {
                        // 按鍵背景
                        keys.spawn((
                            Node {
                                padding: UiRect::new(
                                    Val::Px(6.0),
                                    Val::Px(6.0),
                                    Val::Px(3.0),
                                    Val::Px(3.0),
                                ),
                                border: UiRect::all(Val::Px(1.0)),
                                min_width: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(KEY_ICON_BG),
                            BorderColor::all(KEY_ICON_BORDER),
                            BorderRadius::all(Val::Px(3.0)),
                        ))
                        .with_children(|key_bg| {
                            key_bg.spawn((
                                Text::new(key),
                                TextFont {
                                    font_size: 11.0,
                                    font: font.clone(),
                                    ..default()
                                },
                                TextColor(KEY_TEXT_COLOR),
                            ));
                        });
                        // 動作說明
                        keys.spawn((
                            Text::new(action),
                            TextFont {
                                font_size: 11.0,
                                font: font.clone(),
                                ..default()
                            },
                            TextColor(ACTION_TEXT_COLOR),
                            Node {
                                margin: UiRect::right(Val::Px(6.0)),
                                ..default()
                            },
                        ));
                    };

                    spawn_key(keys, "WASD", "移動", font.clone());
                    spawn_key(keys, "R", "射擊", font.clone());
                    spawn_key(keys, "1-4", "武器", font.clone());
                    spawn_key(keys, "ESC", "暫停", font.clone());
                });
        });

    // 舊版簡單文字提示（保留作為備用更新目標）
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            font: font.clone(),
            ..default()
        },
        TextColor(Color::NONE),
        Node {
            display: Display::None,
            ..default()
        },
        UiText,
    ));

    // === 暫停選單（初始隱藏）- GTA 風格毛玻璃效果 ===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(PAUSE_BG_OUTER),
            Visibility::Hidden,
            PauseMenu,
        ))
        .with_children(|parent| {
            // 內層毛玻璃效果層
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(PAUSE_BG_INNER),
            ));

            // 面板外發光層
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(PAUSE_PANEL_GLOW),
                    BorderRadius::all(Val::Px(16.0)),
                ))
                .with_children(|glow| {
                    // 面板主邊框層
                    glow.spawn((
                        Node {
                            padding: UiRect::all(Val::Px(3.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(PAUSE_PANEL_BORDER),
                        BorderColor::all(Color::srgba(0.5, 0.55, 0.6, 0.6)),
                        BorderRadius::all(Val::Px(12.0)),
                    ))
                    .with_children(|border| {
                        // 面板內邊框層
                        border
                            .spawn((
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::new(
                                        Val::Px(50.0),
                                        Val::Px(50.0),
                                        Val::Px(35.0),
                                        Val::Px(35.0),
                                    ),
                                    row_gap: Val::Px(18.0),
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(PAUSE_PANEL_BG),
                                BorderColor::all(PAUSE_PANEL_INNER_BORDER),
                                BorderRadius::all(Val::Px(8.0)),
                            ))
                            .with_children(|menu| {
                                // 標題區
                                menu.spawn((Node {
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    row_gap: Val::Px(5.0),
                                    margin: UiRect::bottom(Val::Px(10.0)),
                                    ..default()
                                },))
                                    .with_children(|title_area| {
                                        // 暫停圖示（用方塊模擬）
                                        title_area
                                            .spawn((Node {
                                                flex_direction: FlexDirection::Row,
                                                column_gap: Val::Px(8.0),
                                                margin: UiRect::bottom(Val::Px(8.0)),
                                                ..default()
                                            },))
                                            .with_children(|icon_row| {
                                                // 左豎條
                                                icon_row.spawn((
                                                    Node {
                                                        width: Val::Px(8.0),
                                                        height: Val::Px(28.0),
                                                        ..default()
                                                    },
                                                    BackgroundColor(PAUSE_TITLE_COLOR),
                                                    BorderRadius::all(Val::Px(2.0)),
                                                ));
                                                // 右豎條
                                                icon_row.spawn((
                                                    Node {
                                                        width: Val::Px(8.0),
                                                        height: Val::Px(28.0),
                                                        ..default()
                                                    },
                                                    BackgroundColor(PAUSE_TITLE_COLOR),
                                                    BorderRadius::all(Val::Px(2.0)),
                                                ));
                                            });

                                        // 標題文字
                                        title_area.spawn((
                                            Text::new("遊戲暫停"),
                                            TextFont {
                                                font_size: 32.0,
                                                font: font.clone(),
                                                ..default()
                                            },
                                            TextColor(PAUSE_TITLE_COLOR),
                                        ));
                                    });

                                // 分隔線
                                menu.spawn((
                                    Node {
                                        width: Val::Px(220.0),
                                        height: Val::Px(1.0),
                                        margin: UiRect::new(
                                            Val::Px(0.0),
                                            Val::Px(0.0),
                                            Val::Px(5.0),
                                            Val::Px(10.0),
                                        ),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.4, 0.4, 0.45, 0.5)),
                                ));

                                // 繼續遊戲按鈕（帶邊框）
                                menu.spawn((
                                    Node {
                                        padding: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(RESUME_BTN_BORDER),
                                    BorderRadius::all(Val::Px(8.0)),
                                ))
                                .with_children(|btn_border| {
                                    btn_border
                                        .spawn((
                                            Button,
                                            Node {
                                                width: Val::Px(220.0),
                                                height: Val::Px(48.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BackgroundColor(RESUME_BTN_NORMAL),
                                            BorderRadius::all(Val::Px(6.0)),
                                            ResumeButton,
                                        ))
                                        .with_children(|btn| {
                                            btn.spawn((
                                                Text::new("繼續遊戲"),
                                                TextFont {
                                                    font_size: 20.0,
                                                    font: font.clone(),
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                });

                                // 退出遊戲按鈕（帶邊框）
                                menu.spawn((
                                    Node {
                                        padding: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(QUIT_BTN_BORDER),
                                    BorderRadius::all(Val::Px(8.0)),
                                ))
                                .with_children(|btn_border| {
                                    btn_border
                                        .spawn((
                                            Button,
                                            Node {
                                                width: Val::Px(220.0),
                                                height: Val::Px(48.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            BackgroundColor(QUIT_BTN_NORMAL),
                                            BorderRadius::all(Val::Px(6.0)),
                                            QuitButton,
                                        ))
                                        .with_children(|btn| {
                                            btn.spawn((
                                                Text::new("退出遊戲"),
                                                TextFont {
                                                    font_size: 20.0,
                                                    font: font.clone(),
                                                    ..default()
                                                },
                                                TextColor(Color::WHITE),
                                            ));
                                        });
                                });

                                // 分隔線
                                menu.spawn((
                                    Node {
                                        width: Val::Px(220.0),
                                        height: Val::Px(1.0),
                                        margin: UiRect::new(
                                            Val::Px(0.0),
                                            Val::Px(0.0),
                                            Val::Px(10.0),
                                            Val::Px(5.0),
                                        ),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.3, 0.3, 0.35, 0.4)),
                                ));

                                // 快捷鍵提示
                                menu.spawn((Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: Val::Px(20.0),
                                    ..default()
                                },))
                                    .with_children(|hint_row| {
                                        // ESC 提示
                                        hint_row
                                            .spawn((Node {
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                column_gap: Val::Px(6.0),
                                                ..default()
                                            },))
                                            .with_children(|hint| {
                                                // ESC 按鍵框
                                                hint.spawn((
                                                    Node {
                                                        padding: UiRect::new(
                                                            Val::Px(6.0),
                                                            Val::Px(6.0),
                                                            Val::Px(3.0),
                                                            Val::Px(3.0),
                                                        ),
                                                        border: UiRect::all(Val::Px(1.0)),
                                                        ..default()
                                                    },
                                                    BackgroundColor(BUTTON_BG_DARK),
                                                    BorderColor::all(BUTTON_BORDER_GRAY_70),
                                                    BorderRadius::all(Val::Px(4.0)),
                                                ))
                                                .with_children(|key| {
                                                    key.spawn((
                                                        Text::new("ESC"),
                                                        TextFont {
                                                            font_size: 11.0,
                                                            font: font.clone(),
                                                            ..default()
                                                        },
                                                        TextColor(TEXT_LIGHT_GRAY),
                                                    ));
                                                });
                                                hint.spawn((
                                                    Text::new("繼續"),
                                                    TextFont {
                                                        font_size: 13.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                    TextColor(PAUSE_HINT_COLOR),
                                                ));
                                            });

                                        // Q 提示
                                        hint_row
                                            .spawn((Node {
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                column_gap: Val::Px(6.0),
                                                ..default()
                                            },))
                                            .with_children(|hint| {
                                                // Q 按鍵框
                                                hint.spawn((
                                                    Node {
                                                        padding: UiRect::new(
                                                            Val::Px(8.0),
                                                            Val::Px(8.0),
                                                            Val::Px(3.0),
                                                            Val::Px(3.0),
                                                        ),
                                                        border: UiRect::all(Val::Px(1.0)),
                                                        ..default()
                                                    },
                                                    BackgroundColor(BUTTON_BG_DARK),
                                                    BorderColor::all(BUTTON_BORDER_GRAY_70),
                                                    BorderRadius::all(Val::Px(4.0)),
                                                ))
                                                .with_children(|key| {
                                                    key.spawn((
                                                        Text::new("Q"),
                                                        TextFont {
                                                            font_size: 11.0,
                                                            font: font.clone(),
                                                            ..default()
                                                        },
                                                        TextColor(TEXT_LIGHT_GRAY),
                                                    ));
                                                });
                                                hint.spawn((
                                                    Text::new("退出"),
                                                    TextFont {
                                                        font_size: 13.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                    TextColor(PAUSE_HINT_COLOR),
                                                ));
                                            });
                                    });

                                // 遊戲標題
                                menu.spawn((
                                    Text::new("ISLAND RAMPAGE"),
                                    TextFont {
                                        font_size: 11.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(PAUSE_SUBTITLE_COLOR),
                                    Node {
                                        margin: UiRect::top(Val::Px(8.0)),
                                        ..default()
                                    },
                                ));
                            });
                    });
                });
        });

    // === 2. 大地圖 (Full Map) - GTA 風格 ===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(FULLMAP_BG),
            Visibility::Hidden,
            FullMapContainer,
        ))
        .with_children(|parent| {
            // === 標題區（帶邊框背景）===
            parent
                .spawn((Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(25.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },))
                .with_children(|title_row| {
                    title_row
                        .spawn((
                            Node {
                                padding: UiRect::new(
                                    Val::Px(30.0),
                                    Val::Px(30.0),
                                    Val::Px(10.0),
                                    Val::Px(10.0),
                                ),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(FULLMAP_TITLE_BG),
                            BorderColor::all(FULLMAP_BORDER),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|bg| {
                            bg.spawn((
                                Text::new("西門町地圖"),
                                TextFont {
                                    font_size: 28.0,
                                    font: font.clone(),
                                    ..default()
                                },
                                TextColor(Color::srgba(0.9, 0.95, 0.9, 1.0)),
                            ));
                        });
                });

            // === 地圖主體（多層邊框）===
            // 外層發光框
            parent
                .spawn((
                    Node {
                        width: Val::Px(1220.0),
                        height: Val::Px(820.0),
                        padding: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.35, 0.2, 0.4)),
                    BorderRadius::all(Val::Px(12.0)),
                ))
                .with_children(|glow| {
                    // 主邊框層
                    glow.spawn((
                        Node {
                            width: Val::Px(1210.0),
                            height: Val::Px(810.0),
                            padding: UiRect::all(Val::Px(4.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(FULLMAP_BORDER),
                        BorderColor::all(Color::srgba(0.15, 0.2, 0.15, 0.9)),
                        BorderRadius::all(Val::Px(10.0)),
                    ))
                    .with_children(|frame| {
                        // 實際地圖容器
                        frame
                            .spawn((
                                Node {
                                    width: Val::Px(1200.0),
                                    height: Val::Px(800.0),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                BackgroundColor(FULLMAP_MAIN_BG),
                                BorderRadius::all(Val::Px(6.0)),
                            ))
                            .with_children(|map| {
                                // 角落高光效果
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(0.0),
                                        left: Val::Px(0.0),
                                        width: Val::Px(150.0),
                                        height: Val::Px(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.25, 0.35, 0.25, 0.12)),
                                    BorderRadius::top_left(Val::Px(6.0)),
                                ));
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(0.0),
                                        right: Val::Px(0.0),
                                        width: Val::Px(150.0),
                                        height: Val::Px(100.0),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.25, 0.35, 0.25, 0.08)),
                                    BorderRadius::top_right(Val::Px(6.0)),
                                ));

                                // 網格線（增加地圖質感）
                                for i in 0..9 {
                                    // 水平線
                                    map.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            top: Val::Px(100.0 * i as f32),
                                            left: Val::Px(0.0),
                                            width: Val::Percent(100.0),
                                            height: Val::Px(1.0),
                                            ..default()
                                        },
                                        BackgroundColor(MAP_AREA_GREEN),
                                    ));
                                }
                                for i in 0..13 {
                                    // 垂直線
                                    map.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            top: Val::Px(0.0),
                                            left: Val::Px(100.0 * i as f32),
                                            width: Val::Px(1.0),
                                            height: Val::Percent(100.0),
                                            ..default()
                                        },
                                        BackgroundColor(MAP_AREA_GREEN),
                                    ));
                                }

                                // 地圖內容
                                let fm_scale = 2.0;
                                let fm_off_x = 600.0;
                                let fm_off_y = 400.0;
                                let fw_fac = 1.0;

                                spawn_map_layer(
                                    map,
                                    fm_scale,
                                    fm_off_x,
                                    fm_off_y,
                                    fw_fac,
                                    true,
                                    font.clone(),
                                );

                                // === 玩家標記（簡潔圓形+箭頭指針）===
                                // 注意：容器需要包含整個箭頭，旋轉才能正確工作
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Px(30.0),
                                        height: Val::Px(52.0), // 增加高度以包含箭頭
                                        left: Val::Px(585.0),
                                        top: Val::Px(374.0), // 調整位置
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        overflow: Overflow::visible(),
                                        ..default()
                                    },
                                    Transform::default(),
                                    GlobalTransform::default(),
                                    FullMapPlayerMarker,
                                ))
                                .with_children(|marker| {
                                    // 淡白色外圈（脈衝動畫用）- 定位在容器下半部
                                    marker.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Px(28.0),
                                            height: Val::Px(28.0),
                                            left: Val::Px(1.0),
                                            top: Val::Px(23.0), // 在容器下半部
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.25)),
                                        BorderRadius::all(Val::Px(14.0)),
                                    ));
                                    // 黑色描邊圓
                                    marker.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Px(22.0),
                                            height: Val::Px(22.0),
                                            left: Val::Px(4.0),
                                            top: Val::Px(26.0),
                                            ..default()
                                        },
                                        BackgroundColor(OVERLAY_BLACK_90),
                                        BorderRadius::all(Val::Px(11.0)),
                                    ));
                                    // 白色主圓
                                    marker.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Px(16.0),
                                            height: Val::Px(16.0),
                                            left: Val::Px(7.0),
                                            top: Val::Px(29.0),
                                            ..default()
                                        },
                                        BackgroundColor(Color::WHITE),
                                        BorderRadius::all(Val::Px(8.0)),
                                    ));
                                    // 方向指示三角（黑色描邊）- 向上指
                                    marker.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Px(14.0),
                                            height: Val::Px(24.0),
                                            left: Val::Px(8.0),
                                            top: Val::Px(2.0),
                                            ..default()
                                        },
                                        BackgroundColor(OVERLAY_BLACK_90),
                                        BorderRadius::top(Val::Px(7.0)),
                                    ));
                                    // 方向指示三角（白色內部）
                                    marker.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Px(10.0),
                                            height: Val::Px(22.0),
                                            left: Val::Px(10.0),
                                            top: Val::Px(4.0),
                                            ..default()
                                        },
                                        BackgroundColor(Color::WHITE),
                                        BorderRadius::top(Val::Px(5.0)),
                                    ));
                                });

                                // === 方位標示（帶圓角背景）===
                                // 北（紅色更醒目）
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(15.0),
                                        left: Val::Px(582.0),
                                        width: Val::Px(36.0),
                                        height: Val::Px(36.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(COMPASS_BG),
                                    BorderRadius::all(Val::Px(18.0)),
                                ))
                                .with_children(|bg| {
                                    bg.spawn((
                                        Text::new("N"),
                                        TextFont {
                                            font_size: 22.0,
                                            font: font.clone(),
                                            ..default()
                                        },
                                        TextColor(COMPASS_NORTH),
                                    ));
                                });
                                // 南
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        bottom: Val::Px(15.0),
                                        left: Val::Px(582.0),
                                        width: Val::Px(36.0),
                                        height: Val::Px(36.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(COMPASS_BG),
                                    BorderRadius::all(Val::Px(18.0)),
                                ))
                                .with_children(|bg| {
                                    bg.spawn((
                                        Text::new("S"),
                                        TextFont {
                                            font_size: 22.0,
                                            font: font.clone(),
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                                // 東
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(382.0),
                                        right: Val::Px(15.0),
                                        width: Val::Px(36.0),
                                        height: Val::Px(36.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(COMPASS_BG),
                                    BorderRadius::all(Val::Px(18.0)),
                                ))
                                .with_children(|bg| {
                                    bg.spawn((
                                        Text::new("E"),
                                        TextFont {
                                            font_size: 22.0,
                                            font: font.clone(),
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                                // 西
                                map.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Px(382.0),
                                        left: Val::Px(15.0),
                                        width: Val::Px(36.0),
                                        height: Val::Px(36.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(COMPASS_BG),
                                    BorderRadius::all(Val::Px(18.0)),
                                ))
                                .with_children(|bg| {
                                    bg.spawn((
                                        Text::new("W"),
                                        TextFont {
                                            font_size: 22.0,
                                            font: font.clone(),
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            });
                    });
                });

            // === 圖例（帶背景容器）===
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(25.0),
                        padding: UiRect::new(
                            Val::Px(20.0),
                            Val::Px(20.0),
                            Val::Px(8.0),
                            Val::Px(8.0),
                        ),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.1, 0.08, 0.85)),
                    BorderColor::all(Color::srgba(0.3, 0.35, 0.3, 0.5)),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|legend| {
                    spawn_legend_item(legend, Color::srgb(0.5, 0.5, 0.55), "道路", font.clone());
                    spawn_legend_item(legend, Color::srgb(0.8, 0.25, 0.2), "地標", font.clone());
                    spawn_legend_item(legend, PLAYER_MARKER_CORE, "你", font.clone());
                });

            // === 操作提示 ===
            parent
                .spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(15.0),
                            Val::Px(15.0),
                            Val::Px(6.0),
                            Val::Px(6.0),
                        ),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.6)),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .with_children(|bg| {
                    bg.spawn((
                        Text::new("[M] 關閉地圖"),
                        TextFont {
                            font_size: 14.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(TEXT_GRAY_90),
                    ));
                });
        });
}

/// 生成圖例項目
fn spawn_legend_item(
    parent: &mut ChildSpawnerCommands,
    color: Color,
    label: &str,
    font: Handle<Font>,
) {
    parent
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(5.0),
            ..default()
        },))
        .with_children(|item| {
            item.spawn((
                Node {
                    width: Val::Px(15.0),
                    height: Val::Px(15.0),
                    ..default()
                },
                BackgroundColor(color),
            ));
            item.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.0,
                    font,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// 取得載具類型中文名稱
fn get_vehicle_type_name(vehicle_type: VehicleType) -> &'static str {
    match vehicle_type {
        VehicleType::Scooter => "機車",
        VehicleType::Car => "汽車",
        VehicleType::Taxi => "計程車",
        VehicleType::Bus => "公車",
    }
}

/// 更新 UI
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_ui(
    game_state: Res<GameState>,
    world_time: Res<WorldTime>,
    vehicle_query: Query<&Vehicle>,
    mission_manager: Res<MissionManager>,
    mut text_query: Query<
        &mut Text,
        (
            With<UiText>,
            Without<TimeDisplay>,
            Without<ControlStatusTag>,
            Without<ControlSpeedDisplay>,
        ),
    >,
    mut time_query: Query<&mut Text, With<TimeDisplay>>,
    mut status_tag_query: Query<
        &mut Text,
        (
            With<ControlStatusTag>,
            Without<TimeDisplay>,
            Without<UiText>,
            Without<ControlSpeedDisplay>,
        ),
    >,
    mut speed_display_query: Query<
        (&mut Text, &mut Visibility),
        (
            With<ControlSpeedDisplay>,
            Without<TimeDisplay>,
            Without<UiText>,
            Without<ControlStatusTag>,
        ),
    >,
) {
    // 更新舊版控制提示文字（保留兼容）
    if let Ok(mut text) = text_query.single_mut() {
        **text = get_control_hint_text(&game_state, &vehicle_query, &mission_manager);
    }

    // 更新時間顯示
    if let Ok(mut text) = time_query.single_mut() {
        **text = format_world_time(&world_time);
    }

    // 更新 GTA 風格狀態標籤
    if let Ok(mut status_text) = status_tag_query.single_mut() {
        let name = if game_state.player_in_vehicle {
            vehicle_query
                .iter()
                .next()
                .map(|v| get_vehicle_type_name(v.vehicle_type))
                .unwrap_or("駕駛")
        } else {
            "步行"
        };
        **status_text = name.to_string();
    }

    // 更新速度顯示
    let Ok((mut speed_text, mut visibility)) = speed_display_query.single_mut() else {
        return;
    };
    if !game_state.player_in_vehicle {
        *visibility = Visibility::Hidden;
        return;
    }
    if let Some(vehicle) = vehicle_query.iter().next() {
        let speed_kmh = (vehicle.current_speed * 3.6).abs() as i32;
        **speed_text = format!("{} km/h", speed_kmh);
        *visibility = Visibility::Visible;
    }
}

/// 取得控制提示文字
fn get_control_hint_text(
    game_state: &GameState,
    vehicle_query: &Query<&Vehicle>,
    mission_manager: &MissionManager,
) -> String {
    if !game_state.player_in_vehicle {
        return if mission_manager.active_mission.is_some() {
            "[步行] WASD移動 | Q/E旋轉 | R射擊 T換彈 | 1-4武器 | Tab上車".to_string()
        } else {
            "[步行] WASD移動 | Q/E旋轉 | R射擊 T換彈 | 1-4武器 | F接任務".to_string()
        };
    }

    let Some(vehicle_entity) = game_state.current_vehicle else {
        return String::new();
    };

    let Ok(vehicle) = vehicle_query.get(vehicle_entity) else {
        return String::new();
    };

    let speed_kmh = (vehicle.current_speed * 3.6).abs() as i32;
    format!(
        "[{}] {} km/h | WASD駕駛 | Space煞車 | Tab下車",
        get_vehicle_type_name(vehicle.vehicle_type),
        speed_kmh
    )
}

/// 格式化世界時間
fn format_world_time(world_time: &WorldTime) -> String {
    let hour = world_time.hour as u32;
    let minute = ((world_time.hour - hour as f32) * 60.0) as u32;
    let day_night = if (6..18).contains(&hour) { "D" } else { "N" };
    format!("[{}] {:02}:{:02}", day_night, hour, minute)
}

/// 任務 UI 更新
pub fn update_mission_ui(
    mission_manager: Res<MissionManager>,
    player_query: Query<&Transform, With<Player>>,
    mut mission_info_query: Query<&mut Text, With<MissionInfo>>,
) {
    if let Ok(mut text) = mission_info_query.single_mut() {
        if let Some(ref active) = mission_manager.active_mission {
            if let Ok(player_transform) = player_query.single() {
                let distance = player_transform.translation.distance(active.data.end_pos);

                if let Some(limit) = active.data.time_limit {
                    let remaining = (limit - active.time_elapsed).max(0.0);
                    **text = format!(
                        "[任務] {} | {:.0}m | {:.0}s",
                        active.data.title, distance, remaining
                    );
                } else {
                    **text = format!("[任務] {} | {:.0}m", active.data.title, distance);
                }
            }
        } else {
            **text = "".to_string();
        }
    }
}

/// 更新護甲區顯示
fn update_armor_section(
    armor_opt: Option<&Armor>,
    armor_section_query: &mut Query<&mut Visibility, With<ArmorSection>>,
    armor_fill_query: &mut Query<
        &mut Node,
        (
            With<ArmorBarFill>,
            Without<HealthBarFill>,
            Without<HealthBarHighlight>,
        ),
    >,
    armor_label_query: &mut Query<
        &mut Text,
        (
            With<ArmorLabel>,
            Without<HealthLabel>,
            Without<MoneyDisplay>,
            Without<ArmorLabelShadow>,
        ),
    >,
    armor_shadow_query: &mut Query<
        &mut Text,
        (
            With<ArmorLabelShadow>,
            Without<ArmorLabel>,
            Without<HealthLabel>,
            Without<HealthLabelShadow>,
        ),
    >,
) {
    let should_show = armor_opt.map(|a| a.current > 0.0).unwrap_or(false);

    if let Ok(mut visibility) = armor_section_query.single_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Some(armor) = armor_opt.filter(|a| a.current > 0.0) {
        let armor_percent = (armor.current / armor.max * 100.0).clamp(0.0, 100.0);

        if let Ok(mut node) = armor_fill_query.single_mut() {
            node.width = Val::Percent(armor_percent);
        }

        let armor_text = format!("{:.0}/{:.0}", armor.current, armor.max);
        if let Ok(mut text) = armor_label_query.single_mut() {
            **text = armor_text.clone();
        }
        if let Ok(mut text) = armor_shadow_query.single_mut() {
            **text = armor_text;
        }
    }
}

/// 更新 HUD（血量條、護甲條、金錢）- GTA 風格
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_hud(
    player_query: Query<(&Health, Option<&Armor>, &Player)>,
    mut health_fill_query: Query<
        &mut Node,
        (
            With<HealthBarFill>,
            Without<HealthBarHighlight>,
            Without<ArmorBarFill>,
        ),
    >,
    mut health_highlight_query: Query<
        &mut Node,
        (
            With<HealthBarHighlight>,
            Without<HealthBarFill>,
            Without<ArmorBarFill>,
        ),
    >,
    mut health_label_query: Query<
        &mut Text,
        (
            With<HealthLabel>,
            Without<MoneyDisplay>,
            Without<ArmorLabel>,
            Without<HealthLabelShadow>,
        ),
    >,
    mut health_shadow_query: Query<
        &mut Text,
        (
            With<HealthLabelShadow>,
            Without<HealthLabel>,
            Without<ArmorLabel>,
            Without<ArmorLabelShadow>,
        ),
    >,
    mut armor_section_query: Query<&mut Visibility, With<ArmorSection>>,
    mut armor_fill_query: Query<
        &mut Node,
        (
            With<ArmorBarFill>,
            Without<HealthBarFill>,
            Without<HealthBarHighlight>,
        ),
    >,
    mut armor_label_query: Query<
        &mut Text,
        (
            With<ArmorLabel>,
            Without<HealthLabel>,
            Without<MoneyDisplay>,
            Without<ArmorLabelShadow>,
        ),
    >,
    mut armor_shadow_query: Query<
        &mut Text,
        (
            With<ArmorLabelShadow>,
            Without<ArmorLabel>,
            Without<HealthLabel>,
            Without<HealthLabelShadow>,
        ),
    >,
    mut money_query: Query<
        &mut Text,
        (
            With<MoneyDisplay>,
            Without<HealthLabel>,
            Without<ArmorLabel>,
            Without<HealthLabelShadow>,
            Without<ArmorLabelShadow>,
        ),
    >,
) {
    let Ok((health, armor_opt, player)) = player_query.single() else {
        return;
    };

    let health_percent = health.percentage() * 100.0;

    // 更新血量條填充寬度
    if let Ok(mut node) = health_fill_query.single_mut() {
        node.width = Val::Percent(health_percent);
    }

    // 更新血量條高光寬度（跟隨填充）
    if let Ok(mut node) = health_highlight_query.single_mut() {
        node.width = Val::Percent(health_percent);
    }

    // 更新血量數值標籤和陰影
    let health_text = format!("{:.0}/{:.0}", health.current, health.max);
    if let Ok(mut text) = health_label_query.single_mut() {
        **text = health_text.clone();
    }
    if let Ok(mut text) = health_shadow_query.single_mut() {
        **text = health_text;
    }

    // 更新護甲區
    update_armor_section(
        armor_opt,
        &mut armor_section_query,
        &mut armor_fill_query,
        &mut armor_label_query,
        &mut armor_shadow_query,
    );

    // 更新金錢顯示
    if let Ok(mut text) = money_query.single_mut() {
        **text = format!("$ {}", player.money);
    }
}

/// 檢查按鈕是否被按下
fn is_button_pressed(query: &Query<&Interaction, impl bevy::ecs::query::QueryFilter>) -> bool {
    query.iter().any(|i| *i == Interaction::Pressed)
}

/// 設置暫停狀態
fn set_pause_state(
    paused: bool,
    ui_state: &mut UiState,
    pause_menu_query: &mut Query<&mut Visibility, With<PauseMenu>>,
    time: &mut Time<Virtual>,
) {
    ui_state.paused = paused;
    if let Ok(mut visibility) = pause_menu_query.single_mut() {
        if paused {
            *visibility = Visibility::Visible;
            time.pause();
        } else {
            *visibility = Visibility::Hidden;
            time.unpause();
        }
    }
}

/// 進入暫停狀態
pub fn on_enter_pause(
    mut ui_state: ResMut<UiState>,
    mut pause_menu_query: Query<&mut Visibility, With<PauseMenu>>,
    mut time: ResMut<Time<Virtual>>,
) {
    set_pause_state(true, &mut ui_state, &mut pause_menu_query, &mut time);
}

/// 離開暫停狀態
pub fn on_exit_pause(
    mut ui_state: ResMut<UiState>,
    mut pause_menu_query: Query<&mut Visibility, With<PauseMenu>>,
    mut time: ResMut<Time<Virtual>>,
) {
    set_pause_state(false, &mut ui_state, &mut pause_menu_query, &mut time);
}

/// 暫停選單切換（ESC 鍵）和退出遊戲（暫停時按 Q）
pub fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    resume_query: Query<&Interaction, (Changed<Interaction>, With<ResumeButton>)>,
    quit_query: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
) {
    // 檢查「繼續遊戲」按鈕點擊
    if is_button_pressed(&resume_query) {
        next_state.set(AppState::InGame);
        return;
    }

    // 檢查「退出遊戲」按鈕點擊或暫停時按 Q
    let quit_pressed = is_button_pressed(&quit_query)
        || (*state == AppState::Paused && keyboard.just_pressed(KeyCode::KeyQ));
    if quit_pressed {
        std::process::exit(0);
    }

    // ESC 切換暫停
    if keyboard.just_pressed(KeyCode::Escape) {
        let next = if *state == AppState::Paused {
            AppState::InGame
        } else {
            AppState::Paused
        };
        next_state.set(next);
    }
}

/// 按鈕懸停效果 - GTA 風格
#[allow(clippy::type_complexity)]
pub fn button_hover_effect(
    mut resume_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ResumeButton>),
    >,
    mut quit_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<QuitButton>,
            Without<ResumeButton>,
        ),
    >,
) {
    // 繼續遊戲按鈕
    for (interaction, mut bg) in resume_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(RESUME_BTN_PRESSED);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(RESUME_BTN_HOVER);
            }
            Interaction::None => {
                *bg = BackgroundColor(RESUME_BTN_NORMAL);
            }
        }
    }

    // 退出遊戲按鈕
    for (interaction, mut bg) in quit_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg = BackgroundColor(QUIT_BTN_PRESSED);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(QUIT_BTN_HOVER);
            }
            Interaction::None => {
                *bg = BackgroundColor(QUIT_BTN_NORMAL);
            }
        }
    }
}

/// 大地圖切換（M 鍵）
pub fn toggle_map(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut full_map_query: Query<&mut Visibility, With<FullMapContainer>>,
) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        ui_state.show_full_map = !ui_state.show_full_map;

        if let Ok(mut visibility) = full_map_query.single_mut() {
            *visibility = if ui_state.show_full_map {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// 更新小地圖（同步玩家真實位置和方向）
#[allow(clippy::type_complexity)]
pub fn update_minimap(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<MinimapPlayerMarker>)>,
    mut player_marker_query: Query<
        (&mut Node, &mut Transform),
        (With<MinimapPlayerMarker>, Without<Player>),
    >,
) {
    // 獲取玩家在 3D 世界的位置和方向
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let pos = player_transform.translation;
    let forward = player_transform.forward();

    // 將 3D 世界座標轉換為小地圖 UI 座標
    // 小地圖尺寸: 300x300
    let map_scale = 0.9;
    let offset_x = 150.0;
    let offset_y = 150.0;

    let minimap_x = (pos.x * map_scale + offset_x).clamp(10.0, 290.0);
    // Z 軸翻轉：讓北方（正 Z）在上方
    let minimap_y = (-pos.z * map_scale + offset_y).clamp(10.0, 290.0);

    // 計算旋轉角度（基於玩家面向方向）
    // ▲ 預設朝上（北），需要根據玩家朝向旋轉
    // forward.x = 東西方向, forward.z = 南北方向
    // 地圖上北方在上，所以 forward.z > 0 時箭頭朝上
    let rotation_angle = forward.x.atan2(forward.z);
    let target_rotation = Quat::from_rotation_z(-rotation_angle);

    // 更新玩家標記位置和旋轉
    // 容器: 20x34, 圓心在 (10, 24)（從容器左上角算）
    if let Ok((mut node, mut transform)) = player_marker_query.single_mut() {
        node.left = Val::Px(minimap_x - 10.0); // 置中調整 (20/2)
        node.top = Val::Px(minimap_y - 24.0); // 圓心偏移 (19 + 10/2)
                                              // 平滑旋轉插值（每秒旋轉速度約 10 倍，讓旋轉看起來平滑）
        let rotation_speed = 10.0;
        let t = (rotation_speed * time.delta_secs()).min(1.0);
        transform.rotation = transform.rotation.slerp(target_rotation, t);
    }
}

/// 更新大地圖玩家標記位置和方向
#[allow(clippy::type_complexity)]
pub fn update_fullmap(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, Without<FullMapPlayerMarker>)>,
    mut fullmap_marker_query: Query<
        (&mut Node, &mut Transform),
        (With<FullMapPlayerMarker>, Without<Player>),
    >,
) {
    // 獲取玩家在 3D 世界的位置和方向
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let pos = player_transform.translation;
    let forward = player_transform.forward();

    // 將 3D 世界座標轉換為大地圖 UI 座標
    // Full Map: 1200x800
    let fm_scale = 2.0;
    let fm_off_x = 600.0;
    let fm_off_y = 400.0;

    // Z 軸翻轉：讓北方在上方
    let map_x = (pos.x * fm_scale + fm_off_x).clamp(20.0, 1180.0);
    let map_y = (-pos.z * fm_scale + fm_off_y).clamp(20.0, 780.0);

    // 計算旋轉角度
    let rotation_angle = forward.x.atan2(forward.z);
    let target_rotation = Quat::from_rotation_z(-rotation_angle);

    // 更新玩家標記位置和旋轉
    // 容器: 30x52, 圓心在 (15, 37)（從容器左上角算）
    if let Ok((mut node, mut transform)) = fullmap_marker_query.single_mut() {
        node.left = Val::Px(map_x - 15.0); // 置中調整 (30/2)
        node.top = Val::Px(map_y - 37.0); // 圓心偏移 (29 + 16/2)
                                          // 平滑旋轉插值
        let rotation_speed = 10.0;
        let t = (rotation_speed * time.delta_secs()).min(1.0);
        transform.rotation = transform.rotation.slerp(target_rotation, t);
    }
}

/// 小地圖縮放控制（+/- 鍵）
pub fn minimap_zoom_control(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut minimap_query: Query<&mut Node, With<MinimapContainer>>,
) {
    let mut changed = false;

    // + 鍵放大
    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        ui_state.minimap_zoom = (ui_state.minimap_zoom + 0.25).min(2.0);
        changed = true;
    }
    // - 鍵縮小
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        ui_state.minimap_zoom = (ui_state.minimap_zoom - 0.25).max(0.5);
        changed = true;
    }

    // 更新小地圖大小 (基準為 setup_ui 中設定的 300x300)
    if changed {
        if let Ok(mut node) = minimap_query.single_mut() {
            let base_size = 300.0;
            let new_size = base_size * ui_state.minimap_zoom;
            node.width = Val::Px(new_size);
            node.height = Val::Px(new_size);
        }
    }
}
// ... (existing code)

/// 3D 世界名稱標籤組件
#[derive(Component)]
pub struct WorldNameTag {
    pub target_entity: Entity,
    pub offset: Vec3,
}

/// 為所有有名字的建築生成世界標籤 UI
pub fn setup_world_name_tags(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    buildings: Query<(Entity, &GlobalTransform, &Building), Added<Building>>,
    vehicles: Query<(Entity, &GlobalTransform, &Vehicle), Added<Vehicle>>,
    missions: Query<
        (Entity, &GlobalTransform, &crate::mission::MissionMarker),
        Added<crate::mission::MissionMarker>,
    >,
) {
    let font = asset_server.load("fonts/STHeiti.ttc");

    // 建築物標籤 (白色)
    for (entity, _transform, building) in &buildings {
        if building.name.is_empty() {
            continue;
        }

        commands.spawn((
            Text::new(&building.name),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            TextLayout::default(),
            WorldNameTag {
                target_entity: entity,
                offset: Vec3::new(0.0, 10.0, 0.0),
            },
        ));
    }

    // 載具標籤 (黃色)
    for (entity, _transform, vehicle) in &vehicles {
        let name = match vehicle.vehicle_type {
            VehicleType::Scooter => "[機車]",
            VehicleType::Car => "[汽車]",
            VehicleType::Taxi => "[計程車]",
            VehicleType::Bus => "[公車]",
        };

        commands.spawn((
            Text::new(name),
            TextFont {
                font: font.clone(),
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.3)), // 黃色
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            TextLayout::default(),
            WorldNameTag {
                target_entity: entity,
                offset: Vec3::new(0.0, 3.0, 0.0), // 載具較矮，偏移量小
            },
        ));
    }

    // 任務標記標籤 (綠色)
    for (entity, _transform, _marker) in &missions {
        commands.spawn((
            Text::new("[!] 任務"),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.3, 1.0, 0.4)), // 綠色
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            TextLayout::default(),
            WorldNameTag {
                target_entity: entity,
                offset: Vec3::new(0.0, 4.0, 0.0),
            },
        ));
    }
}

/// 更新世界標籤位置 (World to Screen)
pub fn update_world_name_tags(
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::camera::GameCamera>>,
    mut tag_query: Query<(&mut Node, &mut Visibility, &WorldNameTag)>,
    target_query: Query<&GlobalTransform>,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    for (mut node, mut visibility, tag) in tag_query.iter_mut() {
        if let Ok(target_transform) = target_query.get(tag.target_entity) {
            let world_position = target_transform.translation() + tag.offset;

            // World to Screen position
            if let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_position) {
                // 檢查是否在相機前方
                let forward = camera_transform.forward();
                let direction = (world_position - camera_transform.translation()).normalize();

                if forward.dot(direction) > 0.0 {
                    *visibility = Visibility::Visible;
                    node.left = Val::Px(screen_pos.x);
                    node.top = Val::Px(screen_pos.y);
                } else {
                    *visibility = Visibility::Hidden;
                }
            } else {
                *visibility = Visibility::Hidden;
            }
        } else {
            // 目標實體不存在，隱藏標籤（稍後由清理系統移除）
            *visibility = Visibility::Hidden;
        }
    }
}

/// 清理孤立的世界標籤（目標實體已被銷毀）
pub fn cleanup_orphaned_world_tags(
    mut commands: Commands,
    tag_query: Query<(Entity, &WorldNameTag)>,
    target_query: Query<Entity>,
) {
    for (tag_entity, tag) in &tag_query {
        // 如果目標實體不存在，清理標籤
        if target_query.get(tag.target_entity).is_err() {
            commands.entity(tag_entity).despawn();
        }
    }
}

// === 地圖生成通用邏輯 ===

/// 地標數據結構
struct MapLandmark {
    name: &'static str,
    world_x: f32, // World X center
    world_z: f32, // World Z center
    w: f32,       // World Width
    d: f32,       // World Depth
    color: Color,
}

/// 統一生成地圖內容（道路 + 地標）
fn spawn_map_layer(
    parent: &mut ChildSpawnerCommands,
    scale: f32,
    off_x: f32,
    off_y: f32,
    road_width_factor: f32, // 道路寬度縮放係數
    is_fullmap: bool,       // true: 大地圖(顯示路名、完整方塊), false: 小地圖(簡化)
    font: Handle<Font>,
) {
    // 引用世界常數 (更新為新的道路佈局)
    use crate::world::{
        W_ALLEY, W_MAIN, W_PEDESTRIAN, W_SECONDARY, W_ZHONGHUA, X_HAN, X_KANGDING, X_XINING,
        X_ZHONGHUA, Z_CHENGDU, Z_EMEI, Z_HANKOU, Z_KUNMING, Z_WUCHANG,
    };

    // 1. 繪製道路 (Roads) - 完整西門町道路網格
    let v_len_main = 180.0;
    let h_center_x = -10.0; // 水平道路中心點

    // 南北向道路 (Vertical)
    draw_road_rect(
        parent,
        X_ZHONGHUA,
        -15.0,
        W_ZHONGHUA * road_width_factor,
        v_len_main,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "中華路",
        font.clone(),
    );
    draw_road_rect(
        parent,
        X_XINING,
        -15.0,
        W_SECONDARY * road_width_factor,
        v_len_main,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "西寧南路",
        font.clone(),
    );
    draw_road_rect(
        parent,
        X_KANGDING,
        -15.0,
        W_MAIN * road_width_factor,
        v_len_main,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "康定路",
        font.clone(),
    );
    draw_road_rect(
        parent,
        X_HAN,
        0.0,
        W_PEDESTRIAN * road_width_factor,
        100.0,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "漢中街",
        font.clone(),
    );

    // 東西向道路 (Horizontal)
    let h_len = 200.0;
    draw_road_rect(
        parent,
        h_center_x,
        Z_HANKOU,
        h_len,
        W_SECONDARY * road_width_factor,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "漢口街",
        font.clone(),
    );
    draw_road_rect(
        parent,
        h_center_x,
        Z_WUCHANG,
        h_len,
        W_PEDESTRIAN * road_width_factor,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "武昌街",
        font.clone(),
    );
    draw_road_rect(
        parent,
        h_center_x,
        Z_KUNMING,
        h_len,
        W_ALLEY * road_width_factor,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "昆明街",
        font.clone(),
    );
    draw_road_rect(
        parent,
        h_center_x,
        Z_EMEI,
        h_len,
        W_PEDESTRIAN * road_width_factor,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "峨嵋街",
        font.clone(),
    );
    draw_road_rect(
        parent,
        h_center_x,
        Z_CHENGDU,
        h_len,
        W_MAIN * road_width_factor,
        scale,
        off_x,
        off_y,
        is_fullmap,
        "成都路",
        font.clone(),
    );

    // 2. 繪製地標 (Landmarks) - 根據新的建築位置更新
    let landmarks = [
        // 西寧南路沿線
        MapLandmark {
            name: "萬年",
            world_x: X_XINING - 16.0,
            world_z: Z_EMEI - 17.5,
            w: 20.0,
            d: 15.0,
            color: Color::srgb(0.5, 0.5, 0.7),
        },
        MapLandmark {
            name: "獅子林",
            world_x: X_XINING - 17.0,
            world_z: Z_WUCHANG - 18.5,
            w: 22.0,
            d: 22.0,
            color: Color::srgb(0.5, 0.4, 0.3),
        },
        MapLandmark {
            name: "Donki",
            world_x: X_XINING + 20.0,
            world_z: Z_WUCHANG + 18.5,
            w: 28.0,
            d: 22.0,
            color: Color::srgb(1.0, 0.85, 0.0),
        },
        MapLandmark {
            name: "電影公園",
            world_x: X_XINING - 18.5,
            world_z: Z_KUNMING - 14.0,
            w: 25.0,
            d: 20.0,
            color: Color::srgb(0.25, 0.4, 0.25),
        },
        // 漢中街沿線
        MapLandmark {
            name: "誠品西門",
            world_x: X_HAN - 16.5,
            world_z: Z_EMEI - 15.5,
            w: 18.0,
            d: 16.0,
            color: ESLITE_GREEN,
        },
        MapLandmark {
            name: "誠品武昌",
            world_x: X_HAN - 14.5,
            world_z: Z_WUCHANG + 14.5,
            w: 14.0,
            d: 14.0,
            color: ESLITE_GREEN,
        },
        MapLandmark {
            name: "UQ",
            world_x: X_HAN + 13.5,
            world_z: Z_EMEI - 15.0,
            w: 12.0,
            d: 12.0,
            color: Color::srgb(0.85, 0.15, 0.15),
        },
        MapLandmark {
            name: "H&M",
            world_x: X_HAN + 14.5,
            world_z: Z_CHENGDU - 15.0,
            w: 14.0,
            d: 14.0,
            color: Color::srgb(0.85, 0.85, 0.85),
        },
        // 中華路沿線
        MapLandmark {
            name: "捷運6號",
            world_x: X_ZHONGHUA - 26.0,
            world_z: Z_CHENGDU - 14.0,
            w: 12.0,
            d: 12.0,
            color: Color::srgb(0.2, 0.35, 0.65),
        },
        MapLandmark {
            name: "紅樓",
            world_x: X_ZHONGHUA - 31.0,
            world_z: Z_CHENGDU + 19.0,
            w: 22.0,
            d: 22.0,
            color: Color::srgb(0.7, 0.22, 0.18),
        },
        MapLandmark {
            name: "錢櫃",
            world_x: X_ZHONGHUA + 28.0,
            world_z: Z_CHENGDU - 16.0,
            w: 16.0,
            d: 16.0,
            color: Color::srgb(0.75, 0.45, 0.55),
        },
        MapLandmark {
            name: "鴨肉扁",
            world_x: X_ZHONGHUA - 25.0,
            world_z: Z_WUCHANG + 12.5,
            w: 10.0,
            d: 10.0,
            color: Color::srgb(0.85, 0.65, 0.35),
        },
        // 康定路沿線
        MapLandmark {
            name: "西門國小",
            world_x: X_KANGDING + 23.0,
            world_z: Z_WUCHANG - 20.0,
            w: 30.0,
            d: 25.0,
            color: Color::srgb(0.7, 0.65, 0.55),
        },
    ];

    for lm in landmarks.iter() {
        if is_fullmap {
            // 大地圖：顯示完整矩形
            // Size mapping based on scale 1.2 adjusted:
            // 為了保持與之前手動調整的一致性 (UI Size ~= World Size * 1.2)
            // 這裡我們直接使用 (World Size * Scale)
            draw_building_rect(
                parent,
                lm.world_x,
                lm.world_z,
                lm.w,
                lm.d,
                scale,
                off_x,
                off_y,
                lm.color,
                lm.name,
                font.clone(),
            );
        } else {
            // 小地圖：顯示簡化點
            draw_minimap_point(
                parent,
                lm.world_x,
                lm.world_z,
                scale,
                off_x,
                off_y,
                lm.color,
                lm.name,
                font.clone(),
            );
        }
    }
}

// --- 底層繪圖 Helpers ---

#[allow(clippy::too_many_arguments)]
fn draw_road_rect(
    parent: &mut ChildSpawnerCommands,
    x: f32,
    z: f32,
    width: f32,
    length: f32, // w=thickness, l=length
    scale: f32,
    off_x: f32,
    off_y: f32,
    show_name: bool,
    name: &str,
    font: Handle<Font>,
) {
    // 判斷方向：如果 (Width < Length) 則是垂直路？
    // 不，這裡調用時已經區分了。我們在 spawn_map_layer 裡傳入的 width 已經是 "Thickness"。
    // 垂直路：Rect Width = Thickness, Rect Height = Length
    // 水平路：Rect Width = Length, Rect Height = Thickness
    // 上面的 spawn_map_layer 用法：
    // draw(X_ZIONG, 0, W*fac, v_len) -> width < length -> Vertical.
    // draw(h_center, Z, h_len, W*fac) -> width > length -> Horizontal.
    // 讓我們簡單點，直接判斷長寬比來決定形狀 (UI Width/Height)

    // 如果是垂直路 (Thickness, Length) -> UI (T*scale, L*scale)
    // 如果是水平路 (Length, Thickness) -> UI (L*scale, T*scale)
    // 這裡 parent 調用時：
    // Vert: x=pos, z=0, w=thick, l=len.
    // Horz: x=0, z=pos, w=len, l=thick. (Wait, let's fix logic in spawn_map_layer)

    // To simplify: I'll assume usage is always: (Center X, Center Z, Dimension X, Dimension Z)
    // Vert: (X, 0, Thickness, Length) -> UI W=Thick, H=Len
    // Horz: (0, Z, Length, Thickness) -> UI W=Len, H=Thick
    // No, logic in spawn_map_layer was:
    // Vert: draw(X, 0, W, len) -> W is width (thickness).
    // Horz: draw(22.5, Z, h_len, W) -> h_len is width (length).
    // so `width` parameter here is always X-dimension, `length` parameter is Z-dimension?

    // Let's redefine `draw_road_rect` params to be `w_world`, `h_world`.
    let ui_w = width * scale;
    let ui_h = length * scale;
    let ui_x = x * scale + off_x;
    let ui_y = -z * scale + off_y; // Z 軸翻轉

    spawn_centered_rect(
        parent,
        ui_x,
        ui_y,
        ui_w,
        ui_h,
        Color::srgba(0.5, 0.5, 0.55, 0.6),
        if show_name { name } else { "" },
        14.0,
        font,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_building_rect(
    parent: &mut ChildSpawnerCommands,
    x: f32,
    z: f32,
    w: f32,
    d: f32, // World dims
    scale: f32,
    off_x: f32,
    off_y: f32,
    color: Color,
    name: &str,
    font: Handle<Font>,
) {
    let ui_w = w * scale;
    let ui_h = d * scale;
    let ui_x = x * scale + off_x;
    let ui_y = -z * scale + off_y; // Z 軸翻轉

    spawn_centered_rect(parent, ui_x, ui_y, ui_w, ui_h, color, name, 10.0, font);
}

/// 生成置中矩形 (共用 helper)
#[allow(clippy::too_many_arguments)]
fn spawn_centered_rect(
    parent: &mut ChildSpawnerCommands,
    ui_x: f32,
    ui_y: f32,
    ui_w: f32,
    ui_h: f32,
    color: Color,
    name: &str,
    font_size: f32,
    font: Handle<Font>,
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(ui_x - ui_w / 2.0),
                top: Val::Px(ui_y - ui_h / 2.0),
                width: Val::Px(ui_w),
                height: Val::Px(ui_h),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(color),
        ))
        .with_children(|bg| {
            if !name.is_empty() {
                bg.spawn((
                    Text::new(name),
                    TextFont {
                        font_size,
                        font,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn draw_minimap_point(
    parent: &mut ChildSpawnerCommands,
    x: f32,
    z: f32,
    scale: f32,
    off_x: f32,
    off_y: f32,
    color: Color,
    name: &str,
    font: Handle<Font>,
) {
    let ui_x = x * scale + off_x;
    let ui_y = -z * scale + off_y; // Z 軸翻轉
    let size = 10.0; // Fixed point size for minimap

    // Point
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(ui_x - size / 2.0),
            top: Val::Px(ui_y - size / 2.0),
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        },
        BackgroundColor(color),
    ));

    // Label (Offset)
    parent.spawn((
        Text::new(name),
        TextFont {
            font_size: 8.0,
            font,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(ui_x + 6.0),
            top: Val::Px(ui_y - 4.0),
            ..default()
        },
    ));
}

// === 外送 App UI 系統 ===

/// 設置外送 App UI（GTA 風格）
pub fn setup_delivery_app(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // 外送 App 外發光層
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(46.0),
                top: Val::Px(96.0),
                width: Val::Px(362.0),
                height: Val::Auto,
                max_height: Val::Px(516.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(DELIVERY_APP_GLOW),
            BorderRadius::all(Val::Px(10.0)),
            Visibility::Hidden,
            super::DeliveryAppContainer,
        ))
        .with_children(|glow| {
            // 主邊框層
            glow.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(DELIVERY_APP_BORDER),
                BorderColor::all(DELIVERY_APP_BORDER),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|border| {
                // 內邊框層
                border
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(2.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(DELIVERY_APP_INNER_BORDER),
                        BorderColor::all(DELIVERY_APP_INNER_BORDER),
                        BorderRadius::all(Val::Px(6.0)),
                    ))
                    .with_children(|inner| {
                        // 內容區
                        inner
                            .spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    padding: UiRect::all(Val::Px(12.0)),
                                    row_gap: Val::Px(10.0),
                                    ..default()
                                },
                                BackgroundColor(DELIVERY_APP_BG),
                                BorderRadius::all(Val::Px(4.0)),
                            ))
                            .with_children(|content| {
                                // 標題列
                                content
                                    .spawn((
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            justify_content: JustifyContent::SpaceBetween,
                                            align_items: AlignItems::Center,
                                            padding: UiRect::bottom(Val::Px(8.0)),
                                            border: UiRect::bottom(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BorderColor::all(DELIVERY_APP_INNER_BORDER),
                                    ))
                                    .with_children(|header| {
                                        // App 圖示和名稱
                                        header
                                            .spawn((Node {
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                column_gap: Val::Px(8.0),
                                                ..default()
                                            },))
                                            .with_children(|title| {
                                                // 圖示背景
                                                title
                                                    .spawn((
                                                        Node {
                                                            width: Val::Px(28.0),
                                                            height: Val::Px(28.0),
                                                            justify_content: JustifyContent::Center,
                                                            align_items: AlignItems::Center,
                                                            border: UiRect::all(Val::Px(1.0)),
                                                            ..default()
                                                        },
                                                        BackgroundColor(Color::srgba(
                                                            0.9, 0.4, 0.1, 0.3,
                                                        )),
                                                        BorderColor::all(DELIVERY_APP_BORDER),
                                                        BorderRadius::all(Val::Px(4.0)),
                                                    ))
                                                    .with_children(|icon| {
                                                        icon.spawn((
                                                            Text::new("🛵"),
                                                            TextFont {
                                                                font_size: 16.0,
                                                                font: font.clone(),
                                                                ..default()
                                                            },
                                                        ));
                                                    });
                                                // App 名稱
                                                title.spawn((
                                                    Text::new("西門快送"),
                                                    TextFont {
                                                        font_size: 22.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                    TextColor(DELIVERY_APP_TITLE),
                                                ));
                                            });
                                        // 關閉提示（按鍵圖示風格）
                                        header
                                            .spawn((Node {
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                column_gap: Val::Px(4.0),
                                                ..default()
                                            },))
                                            .with_children(|close| {
                                                close
                                                    .spawn((
                                                        Node {
                                                            padding: UiRect::new(
                                                                Val::Px(6.0),
                                                                Val::Px(6.0),
                                                                Val::Px(2.0),
                                                                Val::Px(2.0),
                                                            ),
                                                            border: UiRect::all(Val::Px(1.0)),
                                                            ..default()
                                                        },
                                                        BackgroundColor(KEY_ICON_BG),
                                                        BorderColor::all(KEY_ICON_BORDER),
                                                        BorderRadius::all(Val::Px(3.0)),
                                                    ))
                                                    .with_children(|key| {
                                                        key.spawn((
                                                            Text::new("O"),
                                                            TextFont {
                                                                font_size: 10.0,
                                                                font: font.clone(),
                                                                ..default()
                                                            },
                                                            TextColor(KEY_TEXT_COLOR),
                                                        ));
                                                    });
                                                close.spawn((
                                                    Text::new("關閉"),
                                                    TextFont {
                                                        font_size: 11.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                    TextColor(DELIVERY_APP_SUBTITLE),
                                                ));
                                            });
                                    });

                                // 統計資訊列
                                content
                                    .spawn((
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            justify_content: JustifyContent::SpaceBetween,
                                            align_items: AlignItems::Center,
                                            padding: UiRect::new(
                                                Val::Px(8.0),
                                                Val::Px(8.0),
                                                Val::Px(6.0),
                                                Val::Px(6.0),
                                            ),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgba(0.1, 0.08, 0.12, 0.8)),
                                        BorderColor::all(Color::srgba(0.3, 0.25, 0.2, 0.5)),
                                        BorderRadius::all(Val::Px(4.0)),
                                    ))
                                    .with_children(|stats| {
                                        // 評價顯示
                                        stats
                                            .spawn((Node {
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                column_gap: Val::Px(4.0),
                                                ..default()
                                            },))
                                            .with_children(|rating| {
                                                rating.spawn((
                                                    Text::new("⭐"),
                                                    TextFont {
                                                        font_size: 14.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                ));
                                                rating.spawn((
                                                    Text::new("4.8"),
                                                    TextFont {
                                                        font_size: 16.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                    TextColor(RATING_STAR_COLOR),
                                                    super::DeliveryRatingDisplay,
                                                ));
                                            });
                                        // 連擊顯示
                                        stats
                                            .spawn((Node {
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                column_gap: Val::Px(4.0),
                                                ..default()
                                            },))
                                            .with_children(|streak| {
                                                streak.spawn((
                                                    Text::new("🔥"),
                                                    TextFont {
                                                        font_size: 14.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                ));
                                                streak.spawn((
                                                    Text::new("x0 連擊"),
                                                    TextFont {
                                                        font_size: 14.0,
                                                        font: font.clone(),
                                                        ..default()
                                                    },
                                                    TextColor(STREAK_COLOR),
                                                    super::DeliveryStreakDisplay,
                                                ));
                                            });
                                    });

                                // 訂單列表區域
                                content.spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(8.0),
                                        overflow: Overflow::clip(),
                                        max_height: Val::Px(300.0),
                                        ..default()
                                    },
                                    super::DeliveryOrderList,
                                ));

                                // 提示文字
                                content
                                    .spawn((
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            align_items: AlignItems::Center,
                                            column_gap: Val::Px(6.0),
                                            padding: UiRect::top(Val::Px(4.0)),
                                            border: UiRect::top(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BorderColor::all(Color::srgba(0.3, 0.3, 0.3, 0.3)),
                                    ))
                                    .with_children(|hint| {
                                        hint.spawn((
                                            Text::new("💡"),
                                            TextFont {
                                                font_size: 12.0,
                                                font: font.clone(),
                                                ..default()
                                            },
                                        ));
                                        hint.spawn((
                                            Text::new("靠近餐廳按 F 接單"),
                                            TextFont {
                                                font_size: 12.0,
                                                font: font.clone(),
                                                ..default()
                                            },
                                            TextColor(DELIVERY_APP_SUBTITLE),
                                        ));
                                    });
                            });
                    });
            });
        });
}

/// 切換外送 App 顯示（O 鍵）
pub fn toggle_delivery_app(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut app_query: Query<&mut Visibility, With<super::DeliveryAppContainer>>,
    mut mission_manager: ResMut<MissionManager>,
) {
    if keyboard.just_pressed(KeyCode::KeyO) {
        ui_state.show_delivery_app = !ui_state.show_delivery_app;

        if let Ok(mut visibility) = app_query.single_mut() {
            if ui_state.show_delivery_app {
                *visibility = Visibility::Visible;
                // 開啟時刷新訂單
                mission_manager.refresh_delivery_orders();
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// 更新評分顯示
fn update_rating_display(
    rating_query: &mut Query<
        &mut Text,
        (
            With<super::DeliveryRatingDisplay>,
            Without<super::DeliveryStreakDisplay>,
        ),
    >,
    mission_manager: &MissionManager,
) {
    let Ok(mut text) = rating_query.single_mut() else {
        return;
    };
    let avg = if mission_manager.total_deliveries > 0 {
        mission_manager.average_rating
    } else {
        5.0 // 新手預設滿星
    };
    **text = format!("[*] {:.1}", avg);
}

/// 更新連擊顯示
fn update_streak_display(
    streak_query: &mut Query<
        &mut Text,
        (
            With<super::DeliveryStreakDisplay>,
            Without<super::DeliveryRatingDisplay>,
        ),
    >,
    streak: u32,
) {
    let Ok(mut text) = streak_query.single_mut() else {
        return;
    };
    **text = if streak > 0 {
        format!("x{} 連擊", streak)
    } else {
        "x0 連擊".to_string()
    };
}

/// 生成空訂單提示（GTA 風格）
fn spawn_empty_order_hint(list: &mut ChildSpawnerCommands, font: Handle<Font>) {
    list.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(Val::Px(25.0)),
            row_gap: Val::Px(8.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.08, 0.1, 0.6)),
        BorderColor::all(PANEL_BORDER_GRAY),
        BorderRadius::all(Val::Px(6.0)),
    ))
    .with_children(|hint_box| {
        hint_box.spawn((
            Text::new("📭"),
            TextFont {
                font_size: 32.0,
                font: font.clone(),
                ..default()
            },
        ));
        hint_box.spawn((
            Text::new("暫無訂單"),
            TextFont {
                font_size: 16.0,
                font: font.clone(),
                ..default()
            },
            TextColor(TEXT_GRAY_90),
        ));
        hint_box.spawn((
            Text::new("稍後再試..."),
            TextFont {
                font_size: 12.0,
                font,
                ..default()
            },
            TextColor(TEXT_MUTED),
        ));
    });
}

/// 更新外送 App 訂單列表
/// 優化：只在訂單實際變更時重建 UI，避免每幀 despawn/spawn
#[allow(clippy::too_many_arguments)]
pub fn update_delivery_app(
    mut mission_manager: ResMut<MissionManager>,
    chinese_font: Res<ChineseFont>,
    ui_state: Res<UiState>,
    mut commands: Commands,
    order_list_query: Query<Entity, With<super::DeliveryOrderList>>,
    existing_cards: Query<Entity, With<super::DeliveryOrderCard>>,
    mut rating_query: Query<
        &mut Text,
        (
            With<super::DeliveryRatingDisplay>,
            Without<super::DeliveryStreakDisplay>,
        ),
    >,
    mut streak_query: Query<
        &mut Text,
        (
            With<super::DeliveryStreakDisplay>,
            Without<super::DeliveryRatingDisplay>,
        ),
    >,
) {
    if !ui_state.show_delivery_app {
        return;
    }

    // 更新統計資訊
    update_rating_display(&mut rating_query, &mission_manager);
    update_streak_display(&mut streak_query, mission_manager.delivery_streak);

    // 只在訂單變更時重建卡片
    if !mission_manager.delivery_orders_changed {
        return;
    }
    mission_manager.delivery_orders_changed = false;

    let font = chinese_font.font.clone();

    // 清除舊卡片
    for entity in &existing_cards {
        commands.entity(entity).despawn();
    }

    // 生成新卡片
    let Ok(list_entity) = order_list_query.single() else {
        return;
    };
    commands.entity(list_entity).with_children(|list| {
        if mission_manager.delivery_orders.is_empty() {
            spawn_empty_order_hint(list, font);
        } else {
            for (idx, order) in mission_manager.delivery_orders.iter().enumerate() {
                spawn_delivery_order_card(list, idx, order, font.clone());
            }
        }
    });
}

/// 生成單個訂單卡片（GTA 風格）
fn spawn_delivery_order_card(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    order: &crate::mission::MissionData,
    font: Handle<Font>,
) {
    let delivery_info = order.delivery_order.as_ref();

    // 外層發光
    parent
        .spawn((
            Node {
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(ORDER_CARD_GLOW),
            BorderRadius::all(Val::Px(6.0)),
            super::DeliveryOrderCard { order_index: index },
        ))
        .with_children(|glow| {
            // 主卡片
            glow.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(1.0)),
                    width: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(ORDER_CARD_BG),
                BorderColor::all(ORDER_CARD_BORDER),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|card| {
                // 餐廳名稱和餐點
                let restaurant_name = delivery_info
                    .map(|d| d.restaurant_name.as_str())
                    .unwrap_or("未知餐廳");
                let food_item = delivery_info
                    .map(|d| d.food_item.as_str())
                    .unwrap_or("外送品項");

                card.spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },))
                    .with_children(|title_row| {
                        // 食物圖示
                        title_row.spawn((
                            Text::new("🍜"),
                            TextFont {
                                font_size: 16.0,
                                font: font.clone(),
                                ..default()
                            },
                        ));
                        // 餐廳名稱
                        title_row.spawn((
                            Text::new(restaurant_name),
                            TextFont {
                                font_size: 14.0,
                                font: font.clone(),
                                ..default()
                            },
                            TextColor(RESTAURANT_NAME_COLOR),
                        ));
                        // 分隔
                        title_row.spawn((
                            Text::new("-"),
                            TextFont {
                                font_size: 14.0,
                                font: font.clone(),
                                ..default()
                            },
                            TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                        ));
                        // 餐點
                        title_row.spawn((
                            Text::new(food_item),
                            TextFont {
                                font_size: 13.0,
                                font: font.clone(),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    });

                // 送達地址
                let address = delivery_info
                    .map(|d| d.customer_address.as_str())
                    .unwrap_or("未知地址");

                card.spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },))
                    .with_children(|addr_row| {
                        addr_row.spawn((
                            Text::new("📍"),
                            TextFont {
                                font_size: 12.0,
                                font: font.clone(),
                                ..default()
                            },
                        ));
                        addr_row.spawn((
                            Text::new(address),
                            TextFont {
                                font_size: 12.0,
                                font: font.clone(),
                                ..default()
                            },
                            TextColor(ADDRESS_TEXT_COLOR),
                        ));
                    });

                // 報酬和距離/時間
                let distance = delivery_info.map(|d| d.distance).unwrap_or(0.0);
                let time_limit = order.time_limit.unwrap_or(0.0);

                card.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::top(Val::Px(4.0)),
                        border: UiRect::top(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.3, 0.25, 0.2, 0.4)),
                ))
                .with_children(|info_row| {
                    // 報酬（醒目綠色）
                    info_row
                        .spawn((Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(4.0),
                            ..default()
                        },))
                        .with_children(|reward| {
                            reward.spawn((
                                Text::new("💰"),
                                TextFont {
                                    font_size: 14.0,
                                    font: font.clone(),
                                    ..default()
                                },
                            ));
                            reward.spawn((
                                Text::new(format!("${}", order.reward)),
                                TextFont {
                                    font_size: 15.0,
                                    font: font.clone(),
                                    ..default()
                                },
                                TextColor(REWARD_TEXT_COLOR),
                            ));
                        });
                    // 距離和時間
                    info_row
                        .spawn((Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(8.0),
                            ..default()
                        },))
                        .with_children(|meta| {
                            meta.spawn((
                                Text::new(format!("🗺 {:.0}m", distance)),
                                TextFont {
                                    font_size: 11.0,
                                    font: font.clone(),
                                    ..default()
                                },
                                TextColor(TEXT_SECONDARY),
                            ));
                            meta.spawn((
                                Text::new(format!("⏱ {:.0}s", time_limit)),
                                TextFont {
                                    font_size: 11.0,
                                    font: font.clone(),
                                    ..default()
                                },
                                TextColor(TEXT_SECONDARY),
                            ));
                        });
                });
            });
        });
}

// === 戰鬥 UI 系統 ===

use super::{
    Crosshair, CrosshairDirection, CrosshairDot, CrosshairHitMarker, CrosshairLine,
    CrosshairOuterRing, HitMarkerLine, WeaponDisplay,
};
use crate::combat::CombatState;

/// 設置準星和彈藥 UI
pub fn setup_crosshair(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // === 螢幕中央準星 - GTA 風格 ===
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Crosshair,
        ))
        .with_children(|parent| {
            // 準星容器（增加尺寸以容納外圈）
            parent
                .spawn((Node {
                    width: Val::Px(60.0),
                    height: Val::Px(60.0),
                    position_type: PositionType::Relative,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|crosshair| {
                    // 外圈（動態擴散時使用）
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(50.0),
                            height: Val::Px(50.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(CROSSHAIR_OUTER_RING),
                        BorderRadius::all(Val::Percent(50.0)),
                        CrosshairOuterRing,
                    ));

                    // 中心點陰影（輪廓效果）
                    crosshair.spawn((
                        Node {
                            width: Val::Px(6.0),
                            height: Val::Px(6.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_SHADOW),
                        BorderRadius::all(Val::Percent(50.0)),
                    ));

                    // 中心點
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(4.0),
                            height: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_MAIN),
                        BorderRadius::all(Val::Percent(50.0)),
                        CrosshairDot,
                    ));

                    // 上方線條（帶陰影）
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(4.0),
                            height: Val::Px(12.0),
                            top: Val::Px(3.0),
                            left: Val::Px(28.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_SHADOW),
                        BorderRadius::all(Val::Px(2.0)),
                    ));
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(2.0),
                            height: Val::Px(10.0),
                            top: Val::Px(4.0),
                            left: Val::Px(29.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_MAIN),
                        BorderRadius::all(Val::Px(1.0)),
                        CrosshairLine {
                            direction: CrosshairDirection::Top,
                        },
                    ));

                    // 下方線條（帶陰影）
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(4.0),
                            height: Val::Px(12.0),
                            bottom: Val::Px(3.0),
                            left: Val::Px(28.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_SHADOW),
                        BorderRadius::all(Val::Px(2.0)),
                    ));
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(2.0),
                            height: Val::Px(10.0),
                            bottom: Val::Px(4.0),
                            left: Val::Px(29.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_MAIN),
                        BorderRadius::all(Val::Px(1.0)),
                        CrosshairLine {
                            direction: CrosshairDirection::Bottom,
                        },
                    ));

                    // 左方線條（帶陰影）
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(12.0),
                            height: Val::Px(4.0),
                            left: Val::Px(3.0),
                            top: Val::Px(28.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_SHADOW),
                        BorderRadius::all(Val::Px(2.0)),
                    ));
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(10.0),
                            height: Val::Px(2.0),
                            left: Val::Px(4.0),
                            top: Val::Px(29.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_MAIN),
                        BorderRadius::all(Val::Px(1.0)),
                        CrosshairLine {
                            direction: CrosshairDirection::Left,
                        },
                    ));

                    // 右方線條（帶陰影）
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(12.0),
                            height: Val::Px(4.0),
                            right: Val::Px(3.0),
                            top: Val::Px(28.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_SHADOW),
                        BorderRadius::all(Val::Px(2.0)),
                    ));
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(10.0),
                            height: Val::Px(2.0),
                            right: Val::Px(4.0),
                            top: Val::Px(29.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_MAIN),
                        BorderRadius::all(Val::Px(1.0)),
                        CrosshairLine {
                            direction: CrosshairDirection::Right,
                        },
                    ));

                    // 命中標記（X 形，初始隱藏）
                    crosshair
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Px(20.0),
                                height: Val::Px(20.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            Visibility::Hidden,
                            CrosshairHitMarker,
                        ))
                        .with_children(|hit_marker| {
                            // X 的四條線（斜向）- 使用四個小方塊模擬
                            // 左上到中
                            hit_marker.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(3.0),
                                    height: Val::Px(10.0),
                                    top: Val::Px(0.0),
                                    left: Val::Px(3.0),
                                    ..default()
                                },
                                BackgroundColor(HIT_MARKER_COLOR),
                                BorderRadius::all(Val::Px(1.0)),
                                Transform::from_rotation(Quat::from_rotation_z(
                                    std::f32::consts::FRAC_PI_4,
                                )),
                                HitMarkerLine,
                            ));
                            // 右上到中
                            hit_marker.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(3.0),
                                    height: Val::Px(10.0),
                                    top: Val::Px(0.0),
                                    right: Val::Px(3.0),
                                    ..default()
                                },
                                BackgroundColor(HIT_MARKER_COLOR),
                                BorderRadius::all(Val::Px(1.0)),
                                Transform::from_rotation(Quat::from_rotation_z(
                                    -std::f32::consts::FRAC_PI_4,
                                )),
                                HitMarkerLine,
                            ));
                            // 左下到中
                            hit_marker.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(3.0),
                                    height: Val::Px(10.0),
                                    bottom: Val::Px(0.0),
                                    left: Val::Px(3.0),
                                    ..default()
                                },
                                BackgroundColor(HIT_MARKER_COLOR),
                                BorderRadius::all(Val::Px(1.0)),
                                Transform::from_rotation(Quat::from_rotation_z(
                                    -std::f32::consts::FRAC_PI_4,
                                )),
                                HitMarkerLine,
                            ));
                            // 右下到中
                            hit_marker.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    width: Val::Px(3.0),
                                    height: Val::Px(10.0),
                                    bottom: Val::Px(0.0),
                                    right: Val::Px(3.0),
                                    ..default()
                                },
                                BackgroundColor(HIT_MARKER_COLOR),
                                BorderRadius::all(Val::Px(1.0)),
                                Transform::from_rotation(Quat::from_rotation_z(
                                    std::f32::consts::FRAC_PI_4,
                                )),
                                HitMarkerLine,
                            ));
                        });
                });
        });

    // === 右下角：GTA 風格武器區（多層邊框）===
    // 外發光層
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                right: Val::Px(16.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(HUD_GLOW_OUTER),
            BorderRadius::all(Val::Px(12.0)),
        ))
        .with_children(|glow| {
            // 主容器
            glow.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(HUD_BG),
                BorderColor::all(HUD_BORDER_HIGHLIGHT),
                BorderRadius::all(Val::Px(8.0)),
                WeaponAreaContainer,
            ))
            .with_children(|parent| {
                // 武器名稱區（圖示 + 名稱）
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                    .with_children(|row| {
                        // 武器圖示（金色子彈形狀）
                        row.spawn((
                            Node {
                                width: Val::Px(6.0),
                                height: Val::Px(16.0),
                                ..default()
                            },
                            BackgroundColor(AMMO_NORMAL),
                            BorderRadius::top(Val::Px(3.0)),
                        ));
                        // 武器名稱（帶陰影）
                        row.spawn((Node { ..default() },))
                            .with_children(|label_container| {
                                // 陰影層
                                label_container.spawn((
                                    Text::new("手槍"),
                                    TextFont {
                                        font_size: 22.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(TEXT_SHADOW_OFFSET),
                                        top: Val::Px(TEXT_SHADOW_OFFSET),
                                        ..default()
                                    },
                                    WeaponDisplayShadow,
                                ));
                                // 主文字
                                label_container.spawn((
                                    Text::new("手槍"),
                                    TextFont {
                                        font_size: 22.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    WeaponDisplay,
                                ));
                            });
                    });

                // 彈藥數量區
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Baseline,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                    .with_children(|ammo_row| {
                        // 當前彈藥（大字，帶陰影）
                        ammo_row
                            .spawn((Node { ..default() },))
                            .with_children(|label_container| {
                                // 陰影層
                                label_container.spawn((
                                    Text::new("12"),
                                    TextFont {
                                        font_size: 36.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(2.0), // 大字用更大的偏移
                                        top: Val::Px(2.0),
                                        ..default()
                                    },
                                    CurrentAmmoShadow,
                                ));
                                // 主文字
                                label_container.spawn((
                                    Text::new("12"),
                                    TextFont {
                                        font_size: 36.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(AMMO_NORMAL),
                                    CurrentAmmoText,
                                ));
                            });
                        // 分隔線（帶陰影）
                        ammo_row
                            .spawn((Node { ..default() },))
                            .with_children(|sep_container| {
                                sep_container.spawn((
                                    Text::new("/"),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(TEXT_SHADOW_OFFSET),
                                        top: Val::Px(TEXT_SHADOW_OFFSET),
                                        ..default()
                                    },
                                ));
                                sep_container.spawn((
                                    Text::new("/"),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(AMMO_RESERVE),
                                ));
                            });
                        // 後備彈藥（小字，帶陰影）
                        ammo_row
                            .spawn((Node { ..default() },))
                            .with_children(|label_container| {
                                // 陰影層
                                label_container.spawn((
                                    Text::new("120"),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(TEXT_SHADOW_OFFSET),
                                        top: Val::Px(TEXT_SHADOW_OFFSET),
                                        ..default()
                                    },
                                    ReserveAmmoShadow,
                                ));
                                // 主文字
                                label_container.spawn((
                                    Text::new("120"),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(AMMO_RESERVE),
                                    ReserveAmmoText,
                                ));
                            });
                    });

                // 彈藥視覺化網格（子彈圖示）
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::FlexEnd,
                            column_gap: Val::Px(2.0),
                            row_gap: Val::Px(2.0),
                            max_width: Val::Px(140.0),
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },
                        AmmoVisualGrid,
                    ))
                    .with_children(|grid| {
                        // 初始生成 12 個子彈圖示（預設手槍彈匣）
                        for i in 0..12 {
                            grid.spawn((
                                Node {
                                    width: Val::Px(4.0),
                                    height: Val::Px(10.0),
                                    ..default()
                                },
                                BackgroundColor(BULLET_FILLED),
                                BorderRadius::top(Val::Px(2.0)),
                                AmmoBulletIcon { index: i },
                            ));
                        }
                    });

                // 武器槽位指示器 [1][2][3][4]
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        margin: UiRect::top(Val::Px(5.0)),
                        ..default()
                    },))
                    .with_children(|slots| {
                        for i in 0..4 {
                            let is_active = i == 0; // 預設第一格選中
                            slots
                                .spawn((
                                    Node {
                                        width: Val::Px(28.0),
                                        height: Val::Px(28.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(if is_active {
                                        SLOT_ACTIVE
                                    } else {
                                        SLOT_INACTIVE
                                    }),
                                    BorderColor::all(Color::srgba(0.5, 0.5, 0.5, 0.5)),
                                    BorderRadius::all(Val::Px(4.0)),
                                    WeaponSlot { slot_index: i },
                                ))
                                .with_children(|slot| {
                                    slot.spawn((
                                        Text::new(format!("{}", i + 1)),
                                        TextFont {
                                            font_size: 14.0,
                                            font: font.clone(),
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                        }
                    });
            });
        });
}

/// 判斷是否應該顯示準星（不在車上、正在瞄準、持有遠程武器）
fn should_show_crosshair(
    game_state: &GameState,
    combat_state: &CombatState,
    player_query: &Query<&WeaponInventory, With<Player>>,
) -> bool {
    if game_state.player_in_vehicle || !combat_state.is_aiming {
        return false;
    }
    player_query
        .single()
        .ok()
        .and_then(|inv| inv.current_weapon())
        .map(|w| w.stats.magazine_size > 0)
        .unwrap_or(false)
}

/// 更新準星擴散值（逐漸恢復）
fn update_crosshair_bloom(combat_state: &mut CombatState, dt: f32) {
    if combat_state.crosshair_bloom > 0.0 {
        let recovery_rate = if combat_state.is_aiming { 5.0 } else { 2.0 };
        combat_state.crosshair_bloom = (combat_state.crosshair_bloom - dt * recovery_rate).max(0.0);
    }
}

/// 計算準星偏移量
fn calculate_crosshair_offset(bloom: f32, is_aiming: bool) -> f32 {
    let bloom_offset = bloom.min(1.0) * 12.0;
    let aim_shrink = if is_aiming { 3.0 } else { 0.0 };
    (bloom_offset - aim_shrink).max(-3.0)
}

/// 計算外圈大小
fn calculate_outer_ring_size(bloom: f32, is_aiming: bool) -> f32 {
    let base_size = if is_aiming { 40.0 } else { 50.0 };
    let bloom_expand = bloom.min(1.0) * 20.0;
    base_size + bloom_expand
}

/// 應用準星線條偏移（新版 GTA 風格，基於 4.0 的基礎位置）
fn apply_crosshair_line_offset(node: &mut Node, direction: CrosshairDirection, offset: f32) {
    // 基礎位置 4.0 + 偏移
    let base = 4.0;
    let val = Val::Px(base - offset);
    match direction {
        CrosshairDirection::Top => node.top = val,
        CrosshairDirection::Bottom => node.bottom = val,
        CrosshairDirection::Left => node.left = val,
        CrosshairDirection::Right => node.right = val,
    }
}

/// 更新準星 UI（根據射擊狀態調整準星大小）- GTA 風格
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_crosshair(
    time: Res<Time>,
    mut combat_state: ResMut<CombatState>,
    game_state: Res<GameState>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut crosshair_query: Query<&mut Visibility, With<Crosshair>>,
    mut line_query: Query<(&mut Node, &CrosshairLine), Without<CrosshairOuterRing>>,
    mut outer_ring_query: Query<&mut Node, (With<CrosshairOuterRing>, Without<CrosshairLine>)>,
    mut dot_query: Query<&mut BackgroundColor, With<CrosshairDot>>,
) {
    // 更新可見性
    let should_show = should_show_crosshair(&game_state, &combat_state, &player_query);
    for mut visibility in crosshair_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 更新擴散
    update_crosshair_bloom(&mut combat_state, time.delta_secs());

    let bloom = combat_state.crosshair_bloom;
    let is_aiming = combat_state.is_aiming;

    // 更新線條位置
    let offset = calculate_crosshair_offset(bloom, is_aiming);
    for (mut node, line) in line_query.iter_mut() {
        apply_crosshair_line_offset(&mut node, line.direction, offset);
    }

    // 更新外圈大小
    let ring_size = calculate_outer_ring_size(bloom, is_aiming);
    for mut node in outer_ring_query.iter_mut() {
        node.width = Val::Px(ring_size);
        node.height = Val::Px(ring_size);
    }

    // 瞄準時中心點變亮
    let dot_color = if is_aiming {
        CROSSHAIR_AIM
    } else {
        CROSSHAIR_MAIN
    };
    for mut bg in dot_query.iter_mut() {
        *bg = BackgroundColor(dot_color);
    }
}

/// 更新命中標記（X 形回饋）
#[allow(clippy::type_complexity)]
pub fn update_hit_marker(
    time: Res<Time>,
    mut combat_state: ResMut<CombatState>,
    mut hit_marker_query: Query<&mut Visibility, With<CrosshairHitMarker>>,
    mut hit_marker_line_query: Query<&mut BackgroundColor, With<HitMarkerLine>>,
) {
    // 更新計時器
    if combat_state.hit_marker_timer > 0.0 {
        combat_state.hit_marker_timer -= time.delta_secs();
    }

    // 更新命中標記可見性
    let should_show = combat_state.hit_marker_timer > 0.0;
    for mut visibility in hit_marker_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 更新命中標記顏色（爆頭為金色，普通為紅色）
    if should_show {
        let color = if combat_state.hit_marker_headshot {
            HEADSHOT_MARKER_COLOR
        } else {
            HIT_MARKER_COLOR
        };
        // 根據剩餘時間計算透明度（淡出效果）
        let alpha = (combat_state.hit_marker_timer / 0.2).min(1.0);
        let faded_color = Color::srgba(
            color.to_srgba().red,
            color.to_srgba().green,
            color.to_srgba().blue,
            color.to_srgba().alpha * alpha,
        );
        for mut bg in hit_marker_line_query.iter_mut() {
            *bg = BackgroundColor(faded_color);
        }
    }
}

/// 格式化當前彈藥文字
fn format_current_ammo_text(magazine_size: u32, current_ammo: u32, is_reloading: bool) -> String {
    if magazine_size == 0 {
        "∞".to_string()
    } else if is_reloading {
        "...".to_string()
    } else {
        format!("{}", current_ammo)
    }
}

/// 取得當前彈藥顏色
fn get_current_ammo_color(magazine_size: u32, is_reloading: bool, is_low_ammo: bool) -> Color {
    if magazine_size == 0 {
        AMMO_NORMAL
    } else if is_reloading {
        AMMO_RESERVE
    } else if is_low_ammo {
        AMMO_LOW
    } else {
        AMMO_NORMAL
    }
}

/// 格式化後備彈藥文字
fn format_reserve_ammo_text(magazine_size: u32, reserve_ammo: u32, is_reloading: bool) -> String {
    if magazine_size == 0 {
        String::new()
    } else if is_reloading {
        "換彈中".to_string()
    } else {
        format!("{}", reserve_ammo)
    }
}

/// 更新彈藥顯示（GTA 風格，支援低彈藥變色）
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ammo_display(
    player_query: Query<&WeaponInventory, With<Player>>,
    mut current_ammo_query: Query<
        (&mut Text, &mut TextColor),
        (
            With<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<WeaponDisplay>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut current_ammo_shadow_query: Query<
        &mut Text,
        (
            With<CurrentAmmoShadow>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<ReserveAmmoShadow>,
            Without<WeaponDisplay>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut reserve_ammo_query: Query<
        &mut Text,
        (
            With<ReserveAmmoText>,
            Without<CurrentAmmoText>,
            Without<WeaponDisplay>,
            Without<ReserveAmmoShadow>,
            Without<CurrentAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut reserve_ammo_shadow_query: Query<
        &mut Text,
        (
            With<ReserveAmmoShadow>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<WeaponDisplay>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut weapon_query: Query<
        &mut Text,
        (
            With<WeaponDisplay>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<WeaponDisplayShadow>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
        ),
    >,
    mut weapon_shadow_query: Query<
        &mut Text,
        (
            With<WeaponDisplayShadow>,
            Without<WeaponDisplay>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
        ),
    >,
    mut slot_query: Query<(&mut BackgroundColor, &WeaponSlot)>,
) {
    let Ok(inventory) = player_query.single() else {
        return;
    };
    let Some(weapon) = inventory.current_weapon() else {
        return;
    };

    let magazine_size = weapon.stats.magazine_size;
    let is_low_ammo = magazine_size > 0
        && weapon.current_ammo < (magazine_size as f32 * 0.25) as u32
        && !weapon.is_reloading;

    // 當前彈藥顯示
    let current_text =
        format_current_ammo_text(magazine_size, weapon.current_ammo, weapon.is_reloading);
    let current_color = get_current_ammo_color(magazine_size, weapon.is_reloading, is_low_ammo);

    for (mut text, mut color) in current_ammo_query.iter_mut() {
        **text = current_text.clone();
        color.0 = current_color;
    }
    for mut text in current_ammo_shadow_query.iter_mut() {
        **text = current_text.clone();
    }

    // 後備彈藥顯示
    let reserve_text =
        format_reserve_ammo_text(magazine_size, weapon.reserve_ammo, weapon.is_reloading);
    for mut text in reserve_ammo_query.iter_mut() {
        **text = reserve_text.clone();
    }
    for mut text in reserve_ammo_shadow_query.iter_mut() {
        **text = reserve_text.clone();
    }

    // 武器名稱
    let weapon_name = weapon.stats.weapon_type.name().to_string();
    for mut text in weapon_query.iter_mut() {
        **text = weapon_name.clone();
    }
    for mut text in weapon_shadow_query.iter_mut() {
        **text = weapon_name.clone();
    }

    // 武器槽位高亮
    let current_slot = inventory.current_index;
    for (mut bg, slot) in slot_query.iter_mut() {
        *bg = BackgroundColor(if slot.slot_index == current_slot {
            SLOT_ACTIVE
        } else {
            SLOT_INACTIVE
        });
    }
}

/// 計算低彈藥時的閃爍 alpha
fn calculate_low_ammo_blink_alpha(elapsed_secs: f32, is_low_ammo: bool) -> f32 {
    if is_low_ammo {
        let phase = elapsed_secs * 8.0;
        0.5 + 0.5 * phase.sin()
    } else {
        1.0
    }
}

/// 取得子彈圖示顏色
fn get_bullet_icon_color(is_filled: bool, is_low_ammo: bool, blink_alpha: f32) -> Color {
    if !is_filled {
        return BULLET_EMPTY;
    }
    if !is_low_ammo {
        return BULLET_FILLED;
    }
    let base = BULLET_LOW_WARN.to_srgba();
    Color::srgba(base.red, base.green, base.blue, blink_alpha)
}

/// 檢查是否為低彈藥狀態
fn is_low_ammo_state(current: usize, max: usize, is_reloading: bool) -> bool {
    max > 0 && current < (max as f32 * 0.25).ceil() as usize && !is_reloading
}

/// 更新彈藥視覺化網格（子彈圖示）
#[allow(clippy::type_complexity)]
pub fn update_ammo_visual_grid(
    time: Res<Time>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut bullet_query: Query<(&mut BackgroundColor, &AmmoBulletIcon)>,
    grid_query: Query<Entity, With<AmmoVisualGrid>>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    let Ok(inventory) = player_query.single() else {
        return;
    };
    let Some(weapon) = inventory.current_weapon() else {
        return;
    };

    let current_ammo = weapon.current_ammo as usize;
    let magazine_size = weapon.stats.magazine_size as usize;
    let is_low_ammo = is_low_ammo_state(current_ammo, magazine_size, weapon.is_reloading);
    let blink_alpha = calculate_low_ammo_blink_alpha(time.elapsed_secs(), is_low_ammo);

    // 獲取網格中現有的子彈圖示數量
    let Ok(grid_entity) = grid_query.single() else {
        return;
    };
    let Ok(children) = children_query.get(grid_entity) else {
        return;
    };
    let existing_count = children.len();

    // 如果彈匣大小改變（切換武器），需要重建子彈圖示
    if existing_count != magazine_size && magazine_size > 0 {
        // 刪除所有現有子彈圖示
        for child in children.iter() {
            commands.entity(child).despawn();
        }

        // 生成新的子彈圖示
        commands.entity(grid_entity).with_children(|grid| {
            for i in 0..magazine_size {
                let is_filled = i < current_ammo;
                let color = if is_filled {
                    BULLET_FILLED
                } else {
                    BULLET_EMPTY
                };
                grid.spawn((
                    Node {
                        width: Val::Px(4.0),
                        height: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(color),
                    BorderRadius::top(Val::Px(2.0)),
                    AmmoBulletIcon { index: i },
                ));
            }
        });
        return;
    }

    // 更新現有子彈圖示顏色
    for (mut bg, bullet) in bullet_query.iter_mut() {
        let is_filled = bullet.index < current_ammo;
        *bg = BackgroundColor(get_bullet_icon_color(is_filled, is_low_ammo, blink_alpha));
    }
}

/// 計算武器切換動畫的兩階段淡入淡出透明度
fn calculate_switch_opacity(progress: f32) -> f32 {
    if progress < 0.5 {
        1.0 - (progress * 2.0)
    } else {
        (progress - 0.5) * 2.0
    }
}

/// 計算武器槽位的亮度脈衝
fn calculate_slot_brightness(ease_out: f32) -> f32 {
    0.7 + (ease_out * std::f32::consts::PI).sin() * 0.3
}

/// 應用亮度到顏色
fn apply_brightness_to_color(color: Color, brightness: f32) -> Color {
    let base = color.to_srgba();
    Color::srgba(
        (base.red * brightness).min(1.0),
        (base.green * brightness).min(1.0),
        (base.blue * brightness).min(1.0),
        base.alpha,
    )
}

/// 更新切換動畫進度，回傳 (是否繼續動畫, 透明度, ease_out)
fn update_switch_animation_progress(
    anim: &mut WeaponSwitchAnimation,
    dt: f32,
) -> Option<(f32, f32)> {
    anim.progress += dt / anim.duration;
    if anim.progress >= 1.0 {
        anim.is_switching = false;
        anim.progress = 1.0;
        return None;
    }
    let p = anim.progress;
    Some((calculate_switch_opacity(p), 1.0 - (1.0 - p).powi(2)))
}

/// 更新武器切換動畫（簡化版：只做透明度和縮放，不做位置變化）
/// 注意：此系統需要在 update_ammo_display 之後執行，避免 Query 衝突導致 SIGSEGV
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_weapon_switch_animation(
    time: Res<Time>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut switch_anim: ResMut<WeaponSwitchAnimation>,
    mut weapon_display_query: Query<
        &mut TextColor,
        (
            With<WeaponDisplay>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<WeaponDisplayShadow>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
        ),
    >,
    mut weapon_shadow_query: Query<
        &mut TextColor,
        (
            With<WeaponDisplayShadow>,
            Without<WeaponDisplay>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
        ),
    >,
    mut current_ammo_query: Query<
        &mut TextColor,
        (
            With<CurrentAmmoText>,
            Without<WeaponDisplay>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut current_shadow_query: Query<
        &mut TextColor,
        (
            With<CurrentAmmoShadow>,
            Without<CurrentAmmoText>,
            Without<WeaponDisplay>,
            Without<ReserveAmmoText>,
            Without<ReserveAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut reserve_ammo_query: Query<
        &mut TextColor,
        (
            With<ReserveAmmoText>,
            Without<CurrentAmmoText>,
            Without<WeaponDisplay>,
            Without<ReserveAmmoShadow>,
            Without<CurrentAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut reserve_shadow_query: Query<
        &mut TextColor,
        (
            With<ReserveAmmoShadow>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<WeaponDisplay>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut slot_query: Query<
        &mut BackgroundColor,
        (
            With<WeaponSlot>,
            Without<WeaponDisplay>,
            Without<WeaponDisplayShadow>,
        ),
    >,
) {
    let Ok(inventory) = player_query.single() else {
        return;
    };

    // 檢測武器切換
    if inventory.current_index != switch_anim.last_weapon_index {
        switch_anim.is_switching = true;
        switch_anim.progress = 0.0;
        switch_anim.last_weapon_index = inventory.current_index;
    }

    if !switch_anim.is_switching {
        return;
    }

    let Some((opacity, ease_out)) =
        update_switch_animation_progress(&mut switch_anim, time.delta_secs())
    else {
        return;
    };

    // 應用透明度到文字和陰影
    let shadow_alpha = opacity * 0.65;
    for mut c in weapon_display_query.iter_mut() {
        c.0 = c.0.with_alpha(opacity);
    }
    for mut c in weapon_shadow_query.iter_mut() {
        c.0 = c.0.with_alpha(shadow_alpha);
    }
    for mut c in current_ammo_query.iter_mut() {
        c.0 = c.0.with_alpha(opacity);
    }
    for mut c in current_shadow_query.iter_mut() {
        c.0 = c.0.with_alpha(shadow_alpha);
    }
    for mut c in reserve_ammo_query.iter_mut() {
        c.0 = c.0.with_alpha(opacity);
    }
    for mut c in reserve_shadow_query.iter_mut() {
        c.0 = c.0.with_alpha(shadow_alpha);
    }

    // 武器槽位亮度脈衝
    let brightness = calculate_slot_brightness(ease_out);
    for mut bg in slot_query.iter_mut() {
        bg.0 = apply_brightness_to_color(bg.0, brightness);
    }
}

// ============================================================================
// 敵人血條 UI 系統
// ============================================================================

/// 血條尺寸常數
const ENEMY_HEALTH_BAR_WIDTH: f32 = 70.0;
const ENEMY_HEALTH_BAR_HEIGHT: f32 = 10.0;
const ENEMY_HEALTH_BAR_GLOW_PADDING: f32 = 3.0;

/// 為新生成的敵人創建血條 UI - GTA 風格
pub fn setup_enemy_health_bars(mut commands: Commands, new_enemies: Query<Entity, Added<Enemy>>) {
    for enemy_entity in &new_enemies {
        // 外發光層
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(ENEMY_HEALTH_BAR_WIDTH + ENEMY_HEALTH_BAR_GLOW_PADDING * 2.0),
                    height: Val::Px(ENEMY_HEALTH_BAR_HEIGHT + ENEMY_HEALTH_BAR_GLOW_PADDING * 2.0),
                    padding: UiRect::all(Val::Px(ENEMY_HEALTH_BAR_GLOW_PADDING)),
                    ..default()
                },
                BackgroundColor(ENEMY_BAR_GLOW),
                BorderRadius::all(Val::Px(6.0)),
                EnemyHealthBar { enemy_entity },
                EnemyHealthBarGlow { enemy_entity },
                Visibility::Hidden,
            ))
            .with_children(|glow| {
                // 邊框層
                glow.spawn((
                    Node {
                        width: Val::Px(ENEMY_HEALTH_BAR_WIDTH),
                        height: Val::Px(ENEMY_HEALTH_BAR_HEIGHT),
                        padding: UiRect::all(Val::Px(2.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(ENEMY_BAR_BORDER),
                    BorderColor::all(Color::srgba(0.3, 0.3, 0.35, 0.6)),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .with_children(|border| {
                    // 血條背景
                    border
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(ENEMY_BAR_BG),
                            BorderRadius::all(Val::Px(2.0)),
                        ))
                        .with_children(|bg| {
                            // 血條填充
                            bg.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(ENEMY_HEALTH_FULL),
                                BorderRadius::all(Val::Px(2.0)),
                                EnemyHealthBarFill { enemy_entity },
                            ))
                            .with_children(|fill| {
                                // 高光效果（頂部亮條）
                                fill.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        width: Val::Percent(100.0),
                                        height: Val::Px(3.0),
                                        top: Val::Px(0.0),
                                        left: Val::Px(0.0),
                                        ..default()
                                    },
                                    BackgroundColor(ENEMY_BAR_HIGHLIGHT),
                                    BorderRadius::top(Val::Px(2.0)),
                                    EnemyHealthBarHighlight { enemy_entity },
                                ));
                            });
                        });
                });
            });
    }
}

/// 根據血量百分比計算血條顏色
fn get_health_bar_color(percentage: f32) -> Color {
    if percentage > 0.6 {
        // 60%+ 綠色
        ENEMY_HEALTH_FULL
    } else if percentage > 0.3 {
        // 30-60% 黃色漸變到紅色
        let t = (percentage - 0.3) / 0.3; // 0.0 ~ 1.0
        Color::srgb(
            ENEMY_HEALTH_LOW.to_srgba().red * (1.0 - t) + ENEMY_HEALTH_MID.to_srgba().red * t,
            ENEMY_HEALTH_LOW.to_srgba().green * (1.0 - t) + ENEMY_HEALTH_MID.to_srgba().green * t,
            ENEMY_HEALTH_LOW.to_srgba().blue * (1.0 - t) + ENEMY_HEALTH_MID.to_srgba().blue * t,
        )
    } else {
        // 30% 以下紅色
        ENEMY_HEALTH_LOW
    }
}

/// 更新敵人血條位置和填充 - GTA 風格（含變色）
#[allow(clippy::type_complexity)]
pub fn update_enemy_health_bars(
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::camera::GameCamera>>,
    enemy_query: Query<(&GlobalTransform, &Health), With<Enemy>>,
    mut bar_query: Query<
        (&mut Node, &mut Visibility, &EnemyHealthBar),
        Without<EnemyHealthBarFill>,
    >,
    mut fill_query: Query<
        (&mut Node, &mut BackgroundColor, &EnemyHealthBarFill),
        Without<EnemyHealthBar>,
    >,
) {
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    // 收集每個敵人的血量百分比
    let mut enemy_health_map = std::collections::HashMap::new();

    for (mut node, mut visibility, health_bar) in bar_query.iter_mut() {
        // 取得對應敵人的位置和血量
        let Ok((enemy_transform, health)) = enemy_query.get(health_bar.enemy_entity) else {
            // 敵人已不存在，隱藏血條
            *visibility = Visibility::Hidden;
            continue;
        };

        let percentage = health.percentage();
        enemy_health_map.insert(health_bar.enemy_entity, percentage);

        // 血條位置：敵人頭頂上方
        let world_pos = enemy_transform.translation() + Vec3::new(0.0, 2.5, 0.0);

        // 世界座標轉螢幕座標
        let total_width = ENEMY_HEALTH_BAR_WIDTH + ENEMY_HEALTH_BAR_GLOW_PADDING * 2.0;
        let total_height = ENEMY_HEALTH_BAR_HEIGHT + ENEMY_HEALTH_BAR_GLOW_PADDING * 2.0;

        if let Ok(screen_pos) = camera.world_to_viewport(camera_transform, world_pos) {
            // 檢查是否在攝影機前方
            let forward = camera_transform.forward();
            let direction = (world_pos - camera_transform.translation()).normalize();
            let distance = world_pos.distance(camera_transform.translation());

            // 只在一定距離內且在攝影機前方顯示
            if forward.dot(direction) > 0.0 && distance < 50.0 {
                *visibility = Visibility::Visible;
                // 置中血條（考慮外發光層的額外尺寸）
                node.left = Val::Px(screen_pos.x - total_width / 2.0);
                node.top = Val::Px(screen_pos.y - total_height / 2.0);
            } else {
                *visibility = Visibility::Hidden;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    // 更新所有填充條的寬度和顏色
    for (mut fill_node, mut fill_bg, fill) in fill_query.iter_mut() {
        if let Some(&percentage) = enemy_health_map.get(&fill.enemy_entity) {
            fill_node.width = Val::Percent(percentage * 100.0);
            *fill_bg = BackgroundColor(get_health_bar_color(percentage));
        }
    }
}

/// 清理已死亡敵人的血條
pub fn cleanup_enemy_health_bars(
    mut commands: Commands,
    bar_query: Query<(Entity, &EnemyHealthBar)>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    for (bar_entity, health_bar) in &bar_query {
        // 如果敵人不存在了，移除血條（包含子實體）
        if enemy_query.get(health_bar.enemy_entity).is_err() {
            commands.entity(bar_entity).despawn();
        }
    }
}

// ============================================================================
// 受傷指示器 UI 系統
// ============================================================================

/// 設置受傷指示器 UI（螢幕邊緣暈影）
pub fn setup_damage_indicator(mut commands: Commands) {
    // 受傷指示器容器（全螢幕）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            // 使用 PickingBehavior::IGNORE 讓點擊穿透
            Visibility::Hidden,
            DamageIndicator,
        ))
        .with_children(|parent| {
            // 頂部邊緣
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Px(DAMAGE_EDGE_WIDTH),
                    ..default()
                },
                BackgroundColor(DAMAGE_INDICATOR_COLOR),
                DamageIndicatorEdge {
                    edge: DamageEdge::Top,
                },
            ));

            // 底部邊緣
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Px(DAMAGE_EDGE_WIDTH),
                    ..default()
                },
                BackgroundColor(DAMAGE_INDICATOR_COLOR),
                DamageIndicatorEdge {
                    edge: DamageEdge::Bottom,
                },
            ));

            // 左側邊緣
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(0.0),
                    width: Val::Px(DAMAGE_EDGE_WIDTH),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(DAMAGE_INDICATOR_COLOR),
                DamageIndicatorEdge {
                    edge: DamageEdge::Left,
                },
            ));

            // 右側邊緣
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    right: Val::Px(0.0),
                    width: Val::Px(DAMAGE_EDGE_WIDTH),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(DAMAGE_INDICATOR_COLOR),
                DamageIndicatorEdge {
                    edge: DamageEdge::Right,
                },
            ));

            // 中心暈影（額外的傷害效果）
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(30.0),
                    left: Val::Percent(30.0),
                    width: Val::Percent(40.0),
                    height: Val::Percent(40.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.4, 0.0, 0.0, 0.0)),
                BorderRadius::all(Val::Percent(50.0)),
            ));
        });
}

/// 更新受傷指示器（根據傷害強度和方向）
pub fn update_damage_indicator(
    time: Res<Time>,
    mut damage_state: ResMut<DamageIndicatorState>,
    mut indicator_query: Query<&mut Visibility, With<DamageIndicator>>,
    mut edge_query: Query<(&DamageIndicatorEdge, &mut BackgroundColor)>,
) {
    // 淡出傷害指示器
    if damage_state.intensity > 0.0 {
        damage_state.intensity -= DAMAGE_FADE_RATE * time.delta_secs();
        damage_state.intensity = damage_state.intensity.max(0.0);
    }

    // 更新可見性
    let should_show = damage_state.intensity > 0.01;
    for mut visibility in indicator_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 更新透明度（根據方向）
    if should_show {
        let base_alpha = damage_state.intensity * DAMAGE_INDICATOR_MAX_ALPHA;

        for (edge, mut bg) in edge_query.iter_mut() {
            let alpha = if let Some(dir) = damage_state.damage_direction {
                // 根據傷害方向計算每個邊的強度
                // dir 是從玩家指向攻擊者的方向（螢幕座標系）
                let edge_factor = match edge.edge {
                    DamageEdge::Top => (-dir.y).max(0.0), // 傷害從上方來 (dir.y < 0)
                    DamageEdge::Bottom => dir.y.max(0.0), // 傷害從下方來 (dir.y > 0)
                    DamageEdge::Left => (-dir.x).max(0.0), // 傷害從左方來 (dir.x < 0)
                    DamageEdge::Right => dir.x.max(0.0),  // 傷害從右方來 (dir.x > 0)
                };
                // 保留最小 0.15 的基礎強度，加上方向性的額外強度
                base_alpha * (0.15 + edge_factor * 0.85)
            } else {
                // 無方向時，所有邊緣均勻顯示
                base_alpha
            };
            *bg = BackgroundColor(Color::srgba(0.6, 0.0, 0.0, alpha));
        }
    }
}

/// 觸發受傷指示器（在傷害系統中調用）
///
/// # 參數
/// - `damage_state`: 傷害指示器狀態
/// - `damage_amount`: 傷害量
/// - `direction`: 從玩家指向攻擊者的方向（世界座標 XZ 平面）
pub fn trigger_damage_indicator(
    damage_state: &mut DamageIndicatorState,
    damage_amount: f32,
    direction: Option<Vec2>,
) {
    // 根據傷害量設置強度（最大 1.0）
    let intensity_boost = (damage_amount / 30.0).min(1.0);
    damage_state.intensity = (damage_state.intensity + intensity_boost).min(1.0);

    // 設置傷害方向（新傷害覆蓋舊方向）
    if direction.is_some() {
        damage_state.damage_direction = direction;
    }
}

// === GTA 風格動畫常數 ===

/// 低血量脈衝閾值（血量百分比）
const LOW_HEALTH_THRESHOLD: f32 = 0.3;
/// 低血量脈衝速度（每秒弧度）
const LOW_HEALTH_PULSE_SPEED: f32 = 4.0;
/// 低血量脈衝發光最大強度
const LOW_HEALTH_GLOW_MAX: f32 = 0.6;
/// 低血量脈衝發光最小強度
const LOW_HEALTH_GLOW_MIN: f32 = 0.15;

/// 小地圖掃描線速度（每秒完成一次掃描）
const MINIMAP_SCAN_SPEED: f32 = 0.5;

/// 玩家標記脈衝速度
const PLAYER_MARKER_PULSE_SPEED: f32 = 3.0;

/// 準星動態：射擊後展開量
const CROSSHAIR_FIRE_EXPAND: f32 = 1.8;
/// 準星動態：瞄準時收縮量
const CROSSHAIR_AIM_SHRINK: f32 = 0.6;
/// 準星動態：恢復速度
const CROSSHAIR_RECOVERY_SPEED: f32 = 5.0;
/// 準星動態：命中反彈縮放
const CROSSHAIR_HIT_BOUNCE_SCALE: f32 = 1.3;
/// 準星動態：命中反彈恢復速度
const CROSSHAIR_HIT_BOUNCE_RECOVERY: f32 = 8.0;

// === GTA 風格 HUD 動畫系統 ===

/// 將動畫相位環繞到 0..TAU 範圍
fn wrap_animation_phase(phase: &mut f32) {
    if *phase > std::f32::consts::TAU {
        *phase -= std::f32::consts::TAU;
    }
}

/// 計算低血量發光強度
fn calculate_low_health_glow_intensity(pulse_phase: f32, health_percent: f32) -> f32 {
    let pulse = (pulse_phase.sin() + 1.0) * 0.5;
    let glow_intensity = LOW_HEALTH_GLOW_MIN + (LOW_HEALTH_GLOW_MAX - LOW_HEALTH_GLOW_MIN) * pulse;
    let health_factor = 1.0 - (health_percent / LOW_HEALTH_THRESHOLD);
    glow_intensity * health_factor
}

/// 更新 HUD 動畫狀態（低血量脈衝、小地圖掃描）
pub fn update_hud_animations(
    time: Res<Time>,
    mut anim_state: ResMut<HudAnimationState>,
    player_query: Query<&Health, With<Player>>,
    mut health_glow_query: Query<&mut BackgroundColor, With<HealthBarGlow>>,
    mut minimap_scan_query: Query<&mut Node, With<MinimapScanLine>>,
    mut player_glow_query: Query<
        &mut BackgroundColor,
        (With<MinimapPlayerGlow>, Without<HealthBarGlow>),
    >,
) {
    let dt = time.delta_secs();

    // === 低血量脈衝動畫 ===
    if let Ok(health) = player_query.single() {
        let health_percent = health.percentage();
        let is_low_health = health_percent < LOW_HEALTH_THRESHOLD && !health.is_dead();

        if is_low_health {
            anim_state.low_health_pulse_phase += LOW_HEALTH_PULSE_SPEED * dt;
            wrap_animation_phase(&mut anim_state.low_health_pulse_phase);
            let final_intensity = calculate_low_health_glow_intensity(
                anim_state.low_health_pulse_phase,
                health_percent,
            );
            for mut bg in health_glow_query.iter_mut() {
                *bg = BackgroundColor(Color::srgba(0.8, 0.15, 0.1, final_intensity));
            }
        } else {
            anim_state.low_health_pulse_phase = 0.0;
            for mut bg in health_glow_query.iter_mut() {
                *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
            }
        }
    }

    // === 小地圖掃描線動畫 ===
    anim_state.minimap_scan_position += MINIMAP_SCAN_SPEED * dt;
    if anim_state.minimap_scan_position > 1.0 {
        anim_state.minimap_scan_position -= 1.0;
    }
    for mut node in minimap_scan_query.iter_mut() {
        node.top = Val::Percent(anim_state.minimap_scan_position * 100.0);
    }

    // === 玩家標記脈衝動畫 ===
    anim_state.player_marker_pulse_phase += PLAYER_MARKER_PULSE_SPEED * dt;
    wrap_animation_phase(&mut anim_state.player_marker_pulse_phase);
    let marker_pulse = (anim_state.player_marker_pulse_phase.sin() + 1.0) * 0.5;
    let marker_glow_alpha = 0.15 + marker_pulse * 0.2;
    for mut bg in player_glow_query.iter_mut() {
        *bg = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, marker_glow_alpha));
    }
}

/// 更新準星動態效果（射擊展開、瞄準收縮、命中反彈）
pub fn update_crosshair_dynamics(
    time: Res<Time>,
    combat_state: Res<CombatState>,
    mut dynamics: ResMut<CrosshairDynamics>,
) {
    let dt = time.delta_secs();

    // 根據瞄準狀態設置目標散佈值
    dynamics.target_spread = if combat_state.is_aiming {
        CROSSHAIR_AIM_SHRINK
    } else {
        1.0
    };

    // 平滑過渡到目標散佈值
    let spread_diff = dynamics.target_spread - dynamics.current_spread;
    dynamics.current_spread += spread_diff * CROSSHAIR_RECOVERY_SPEED * dt;

    // 命中反彈恢復
    if dynamics.hit_bounce_scale > 1.0 {
        dynamics.hit_bounce_scale -=
            (dynamics.hit_bounce_scale - 1.0) * CROSSHAIR_HIT_BOUNCE_RECOVERY * dt;
        if dynamics.hit_bounce_scale < 1.01 {
            dynamics.hit_bounce_scale = 1.0;
        }
    }
}

/// 觸發準星射擊展開效果
pub fn trigger_crosshair_fire_expand(dynamics: &mut CrosshairDynamics) {
    dynamics.current_spread = (dynamics.current_spread * CROSSHAIR_FIRE_EXPAND).min(2.5);
}

/// 觸發準星命中反彈效果
pub fn trigger_crosshair_hit_bounce(dynamics: &mut CrosshairDynamics) {
    dynamics.hit_bounce_scale = CROSSHAIR_HIT_BOUNCE_SCALE;
}

// === 天氣 HUD 系統 ===

/// 天氣 HUD 顏色常數
const WEATHER_HUD_BG: Color = Color::srgba(0.05, 0.08, 0.12, 0.9);
const WEATHER_HUD_BORDER: Color = Color::srgba(0.3, 0.45, 0.55, 0.7);

// 天氣圖示顏色
const SUN_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);
const SUN_GLOW: Color = Color::srgba(1.0, 0.9, 0.4, 0.5);
const CLOUD_COLOR: Color = Color::srgb(0.85, 0.88, 0.92);
const CLOUD_DARK: Color = Color::srgb(0.6, 0.65, 0.7);
const RAIN_COLOR: Color = Color::srgb(0.4, 0.7, 0.95);
const FOG_COLOR: Color = Color::srgba(0.75, 0.78, 0.82, 0.8);

/// 設置天氣 HUD
pub fn setup_weather_hud(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // 天氣 HUD 容器（金錢顯示下方，避免重疊）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(420.0), // 金錢下方 (365 + 約 55)
                right: Val::Px(10.0),
                width: Val::Px(150.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BackgroundColor(WEATHER_HUD_BG),
            BorderColor::all(WEATHER_HUD_BORDER),
            BorderRadius::all(Val::Px(8.0)),
            WeatherHudContainer,
        ))
        .with_children(|parent| {
            // 天氣圖示容器（固定大小）
            parent
                .spawn((
                    Node {
                        width: Val::Px(40.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    WeatherIconContainer,
                ))
                .with_children(|icon_parent| {
                    // === 太陽圖示 ===
                    spawn_sun_icon(icon_parent);
                    // === 雲圖示 ===
                    spawn_cloud_icon(icon_parent);
                    // === 雨圖示 ===
                    spawn_rain_icon(icon_parent);
                    // === 霧圖示 ===
                    spawn_fog_icon(icon_parent);
                });

            // 天氣名稱和按鍵提示
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },))
                .with_children(|col| {
                    // 天氣名稱
                    col.spawn((
                        Text::new("晴天"),
                        TextFont {
                            font_size: 18.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        WeatherNameText,
                    ));
                    // 按鍵提示
                    col.spawn((
                        Text::new("[F1] 切換"),
                        TextFont {
                            font_size: 11.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.55, 0.65, 0.9)),
                    ));
                });
        });
}

/// 生成太陽圖示
fn spawn_sun_icon(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            WeatherIconElement {
                weather_type: WeatherIconType::Sun,
            },
        ))
        .with_children(|sun| {
            // 外發光
            sun.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(32.0),
                    height: Val::Px(32.0),
                    ..default()
                },
                BackgroundColor(SUN_GLOW),
                BorderRadius::all(Val::Percent(50.0)),
            ));
            // 太陽核心
            sun.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    left: Val::Px(10.0),
                    top: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(SUN_COLOR),
                BorderRadius::all(Val::Percent(50.0)),
            ));
            // 光芒（8 條）
            for i in 0..8 {
                let angle = i as f32 * 45.0_f32.to_radians();
                let ray_len = 6.0;
                let offset = 14.0;
                let x = 20.0 + angle.cos() * offset - 1.5;
                let y = 20.0 + angle.sin() * offset - ray_len / 2.0;
                sun.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(3.0),
                        height: Val::Px(ray_len),
                        left: Val::Px(x),
                        top: Val::Px(y),
                        ..default()
                    },
                    BackgroundColor(SUN_COLOR),
                    BorderRadius::all(Val::Px(1.5)),
                    Transform::from_rotation(Quat::from_rotation_z(-angle)),
                    SunRay { index: i },
                ));
            }
        });
}

/// 生成雲圖示
fn spawn_cloud_icon(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                ..default()
            },
            Visibility::Hidden,
            WeatherIconElement {
                weather_type: WeatherIconType::Cloud,
            },
        ))
        .with_children(|cloud| {
            // 雲朵由多個圓組成
            // 左側小圓
            cloud.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(14.0),
                    height: Val::Px(14.0),
                    left: Val::Px(4.0),
                    top: Val::Px(18.0),
                    ..default()
                },
                BackgroundColor(CLOUD_COLOR),
                BorderRadius::all(Val::Percent(50.0)),
            ));
            // 中間大圓
            cloud.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    left: Val::Px(10.0),
                    top: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(CLOUD_COLOR),
                BorderRadius::all(Val::Percent(50.0)),
            ));
            // 右側中圓
            cloud.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    left: Val::Px(22.0),
                    top: Val::Px(16.0),
                    ..default()
                },
                BackgroundColor(CLOUD_COLOR),
                BorderRadius::all(Val::Percent(50.0)),
            ));
            // 底部連接
            cloud.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(28.0),
                    height: Val::Px(10.0),
                    left: Val::Px(6.0),
                    top: Val::Px(22.0),
                    ..default()
                },
                BackgroundColor(CLOUD_COLOR),
                BorderRadius::all(Val::Px(4.0)),
            ));
        });
}

/// 生成雨圖示
fn spawn_rain_icon(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                ..default()
            },
            Visibility::Hidden,
            WeatherIconElement {
                weather_type: WeatherIconType::Rain,
            },
        ))
        .with_children(|rain| {
            // 深色雲
            rain.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(12.0),
                    height: Val::Px(12.0),
                    left: Val::Px(4.0),
                    top: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(CLOUD_DARK),
                BorderRadius::all(Val::Percent(50.0)),
            ));
            rain.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(16.0),
                    height: Val::Px(16.0),
                    left: Val::Px(12.0),
                    top: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(CLOUD_DARK),
                BorderRadius::all(Val::Percent(50.0)),
            ));
            rain.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(12.0),
                    height: Val::Px(12.0),
                    left: Val::Px(24.0),
                    top: Val::Px(6.0),
                    ..default()
                },
                BackgroundColor(CLOUD_DARK),
                BorderRadius::all(Val::Percent(50.0)),
            ));
            rain.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(26.0),
                    height: Val::Px(8.0),
                    left: Val::Px(7.0),
                    top: Val::Px(12.0),
                    ..default()
                },
                BackgroundColor(CLOUD_DARK),
                BorderRadius::all(Val::Px(3.0)),
            ));
            // 雨滴
            for i in 0..3 {
                let x = 8.0 + i as f32 * 10.0;
                rain.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(2.5),
                        height: Val::Px(10.0),
                        left: Val::Px(x),
                        top: Val::Px(24.0),
                        ..default()
                    },
                    BackgroundColor(RAIN_COLOR),
                    BorderRadius::all(Val::Px(1.5)),
                    RainDropIcon {
                        index: i,
                        offset: i as f32 * 0.3,
                    },
                ));
            }
        });
}

/// 生成霧圖示
fn spawn_fog_icon(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(5.0),
                ..default()
            },
            Visibility::Hidden,
            WeatherIconElement {
                weather_type: WeatherIconType::Fog,
            },
        ))
        .with_children(|fog| {
            // 三條橫線代表霧
            for i in 0..3 {
                let width = match i {
                    0 => 28.0,
                    1 => 34.0,
                    _ => 24.0,
                };
                let alpha = match i {
                    0 => 0.9,
                    1 => 0.7,
                    _ => 0.5,
                };
                fog.spawn((
                    Node {
                        width: Val::Px(width),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.75, 0.78, 0.82, alpha)),
                    BorderRadius::all(Val::Px(2.0)),
                ));
            }
        });
}

/// 更新天氣 HUD 顯示
pub fn update_weather_hud(
    weather: Res<WeatherState>,
    mut icon_query: Query<(&WeatherIconElement, &mut Visibility)>,
    mut name_query: Query<&mut Text, With<WeatherNameText>>,
) {
    // 取得當前顯示的天氣類型
    let display_weather = if weather.is_transitioning {
        // 過渡期間根據進度顯示
        if weather.transition_progress > 0.5 {
            weather.target_weather
        } else {
            weather.weather_type
        }
    } else {
        weather.weather_type
    };

    // 將 WeatherType 轉換為 WeatherIconType
    let target_icon = match display_weather {
        WeatherType::Clear => WeatherIconType::Sun,
        WeatherType::Cloudy => WeatherIconType::Cloud,
        WeatherType::Rainy => WeatherIconType::Rain,
        WeatherType::Foggy => WeatherIconType::Fog,
        WeatherType::Stormy => WeatherIconType::Rain, // 暴風雨用雨天圖示
        WeatherType::Sandstorm => WeatherIconType::Fog, // 沙塵暴用霧天圖示（TODO: 專用圖示）
    };

    // 更新圖示可見性
    for (icon_element, mut visibility) in icon_query.iter_mut() {
        *visibility = if icon_element.weather_type == target_icon {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // 更新名稱
    if let Ok(mut text) = name_query.single_mut() {
        // 如果正在過渡，顯示過渡提示
        if weather.is_transitioning {
            **text = format!("{}...", weather.target_weather.name());
        } else {
            **text = display_weather.name().to_string();
        }
    }
}

// ============================================================================
// 武器輪盤系統 (GTA 5 風格)
// ============================================================================

/// 武器輪盤顏色常數
const WEAPON_WHEEL_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const WEAPON_WHEEL_SLOT_NORMAL: Color = Color::srgba(0.2, 0.2, 0.25, 0.8);
const WEAPON_WHEEL_SLOT_SELECTED: Color = Color::srgba(0.85, 0.75, 0.3, 0.9);
const WEAPON_WHEEL_SLOT_EMPTY: Color = Color::srgba(0.15, 0.15, 0.18, 0.5);
const WEAPON_WHEEL_TEXT: Color = Color::srgb(0.95, 0.95, 0.95);
const WEAPON_WHEEL_AMMO: Color = Color::srgb(0.85, 0.85, 0.3);

/// 設置武器輪盤 UI
pub fn setup_weapon_wheel(mut commands: Commands, font: Option<Res<ChineseFont>>) {
    let Some(font) = font else { return };

    // 主容器（隱藏狀態開始）
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(WEAPON_WHEEL_BG),
            Visibility::Hidden,
            WeaponWheel,
            Name::new("WeaponWheel"),
        ))
        .with_children(|parent| {
            // 武器輪盤容器
            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(400.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    WeaponWheelBackground,
                ))
                .with_children(|wheel| {
                    // 6 個武器槽位
                    for i in 0..6 {
                        let angle = WeaponWheelState::slot_angle(i);
                        let radius = 140.0;
                        let x = angle.cos() * radius;
                        let y = angle.sin() * radius;

                        wheel
                            .spawn((
                                Node {
                                    width: Val::Px(70.0),
                                    height: Val::Px(70.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(200.0 + x - 35.0),
                                    top: Val::Px(200.0 + y - 35.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(WEAPON_WHEEL_SLOT_NORMAL),
                                BorderColor::all(BUTTON_BORDER_GRAY_60),
                                BorderRadius::all(Val::Px(35.0)), // 圓形
                                WeaponWheelSlot {
                                    index: i,
                                    angle,
                                    is_selected: false,
                                },
                            ))
                            .with_children(|slot| {
                                // 武器圖示
                                slot.spawn((
                                    Text::new(weapon_slot_icon(i)),
                                    TextFont {
                                        font: font.font.clone(),
                                        font_size: 28.0,
                                        ..default()
                                    },
                                    TextColor(WEAPON_WHEEL_TEXT),
                                    WeaponWheelIcon { slot_index: i },
                                ));
                            });
                    }

                    // 中央資訊區域
                    wheel
                        .spawn((
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(80.0),
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            WeaponWheelCenterInfo,
                        ))
                        .with_children(|center| {
                            // 武器名稱
                            center.spawn((
                                Text::new("拳頭"),
                                TextFont {
                                    font: font.font.clone(),
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(WEAPON_WHEEL_TEXT),
                                WeaponWheelName,
                            ));
                            // 彈藥資訊
                            center.spawn((
                                Text::new("∞"),
                                TextFont {
                                    font: font.font.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(WEAPON_WHEEL_AMMO),
                                WeaponWheelAmmo,
                            ));
                        });

                    // 選擇指示器
                    wheel.spawn((
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(80.0),
                            position_type: PositionType::Absolute,
                            left: Val::Px(160.0),
                            top: Val::Px(160.0),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(WEAPON_WHEEL_SLOT_SELECTED),
                        BorderRadius::all(Val::Px(40.0)),
                        WeaponWheelSelector,
                    ));
                });
        });
}

/// 取得武器槽位圖示
fn weapon_slot_icon(slot: usize) -> &'static str {
    match slot {
        0 => "👊", // 拳頭
        1 => "🔫", // 手槍
        2 => "🔫", // 衝鋒槍
        3 => "🎯", // 霰彈槍
        4 => "🎯", // 步槍
        5 => "💣", // 空槽位
        _ => "❓",
    }
}

/// 取得武器輪盤顯示資訊（名稱、彈藥）
fn get_wheel_weapon_info(inventory: &WeaponInventory, slot_index: usize) -> (String, String) {
    if slot_index < inventory.weapons.len() {
        let weapon = &inventory.weapons[slot_index];
        let name = weapon.stats.weapon_type.name().to_string();
        let ammo = if weapon.stats.magazine_size == 0 {
            "∞".to_string()
        } else {
            format!("{} / {}", weapon.current_ammo, weapon.reserve_ammo)
        };
        (name, ammo)
    } else {
        ("空槽位".to_string(), "-".to_string())
    }
}

/// 確認武器選擇並切換
fn confirm_weapon_selection(inventory: &mut WeaponInventory, selected_index: usize) {
    if selected_index < inventory.weapons.len() {
        inventory.current_index = selected_index;
    }
}

/// 武器輪盤輸入系統
pub fn weapon_wheel_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut wheel_state: ResMut<WeaponWheelState>,
    mut wheel_query: Query<&mut Visibility, With<WeaponWheel>>,
    windows: Query<&Window>,
    mut player_query: Query<&mut WeaponInventory, With<Player>>,
) {
    // Tab 鍵打開武器輪盤
    if keyboard.just_pressed(KeyCode::Tab) {
        ui_state.show_weapon_wheel = true;
        wheel_state.is_animating = true;
        wheel_state.open_progress = 0.0;
        for mut vis in wheel_query.iter_mut() {
            *vis = Visibility::Visible;
        }
    }

    // Tab 鍵釋放關閉並確認選擇
    if keyboard.just_released(KeyCode::Tab) {
        if ui_state.show_weapon_wheel {
            if let Ok(mut inventory) = player_query.single_mut() {
                confirm_weapon_selection(&mut inventory, wheel_state.selected_index);
            }
        }
        ui_state.show_weapon_wheel = false;
        for mut vis in wheel_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }

    // 更新滑鼠位置選擇
    if !ui_state.show_weapon_wheel {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let offset = cursor_pos - center;
    wheel_state.update_selection(Vec2::new(offset.x, -offset.y));
}

/// 武器輪盤更新系統
pub fn weapon_wheel_update_system(
    time: Res<Time>,
    ui_state: Res<UiState>,
    mut wheel_state: ResMut<WeaponWheelState>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut slot_query: Query<(&mut WeaponWheelSlot, &mut BackgroundColor, &mut BorderColor)>,
    mut selector_query: Query<&mut Node, With<WeaponWheelSelector>>,
    mut name_query: Query<&mut Text, (With<WeaponWheelName>, Without<WeaponWheelAmmo>)>,
    mut ammo_query: Query<&mut Text, (With<WeaponWheelAmmo>, Without<WeaponWheelName>)>,
) {
    if !ui_state.show_weapon_wheel {
        return;
    }

    let dt = time.delta_secs();

    // 更新打開動畫
    if wheel_state.is_animating {
        wheel_state.open_progress = (wheel_state.open_progress + dt * 5.0).min(1.0);
        if wheel_state.open_progress >= 1.0 {
            wheel_state.is_animating = false;
        }
    }

    let selected = wheel_state.selected_index;

    // 更新槽位高亮
    for (mut slot, mut bg, mut border) in slot_query.iter_mut() {
        slot.is_selected = slot.index == selected;
        if slot.is_selected {
            *bg = BackgroundColor(WEAPON_WHEEL_SLOT_SELECTED);
            *border = BorderColor::all(Color::srgba(1.0, 0.9, 0.5, 0.9));
        } else {
            *bg = BackgroundColor(WEAPON_WHEEL_SLOT_NORMAL);
            *border = BorderColor::all(BUTTON_BORDER_GRAY_60);
        }
    }

    // 更新選擇指示器位置
    let angle = WeaponWheelState::slot_angle(selected);
    let radius = 140.0;
    let x = angle.cos() * radius;
    let y = angle.sin() * radius;
    for mut node in selector_query.iter_mut() {
        node.left = Val::Px(200.0 + x - 40.0);
        node.top = Val::Px(200.0 + y - 40.0);
    }

    // 更新中央資訊
    if let Ok(inventory) = player_query.single() {
        let (weapon_name, ammo_text) = get_wheel_weapon_info(inventory, selected);
        if let Ok(mut text) = name_query.single_mut() {
            **text = weapon_name;
        }
        if let Ok(mut text) = ammo_query.single_mut() {
            **text = ammo_text;
        }
    }
}

/// 武器輪盤圖示更新系統
pub fn weapon_wheel_icon_update_system(
    ui_state: Res<UiState>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut icon_query: Query<(&WeaponWheelIcon, &mut Text)>,
) {
    if !ui_state.show_weapon_wheel {
        return;
    }

    if let Ok(inventory) = player_query.single() {
        for (icon, mut text) in icon_query.iter_mut() {
            let slot = icon.slot_index;
            if slot < inventory.weapons.len() {
                let weapon_type = inventory.weapons[slot].stats.weapon_type;
                **text = weapon_type.icon().to_string();
            } else {
                **text = "—".to_string(); // 空槽位
            }
        }
    }
}

// ============================================================================
// 互動提示 UI 系統
// ============================================================================

use super::{
    InteractionPromptContainer, InteractionPromptKey, InteractionPromptState, InteractionPromptText,
};
use crate::mission::{Trigger as MissionTrigger, TriggerType};

/// 互動提示背景色
const INTERACTION_PROMPT_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
/// 互動提示按鍵背景色
const INTERACTION_KEY_BG: Color = Color::srgb(0.95, 0.85, 0.2);
/// 互動提示文字色
const INTERACTION_TEXT_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);

/// 設置互動提示 UI
pub fn setup_interaction_prompt(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    // 外層容器（全寬，用於居中）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(180.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
            InteractionPromptContainer,
            Name::new("InteractionPromptWrapper"),
        ))
        .with_children(|wrapper| {
            // 內層提示框
            wrapper
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                    BackgroundColor(INTERACTION_PROMPT_BG),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|parent| {
                    // 按鍵框
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(36.0),
                                height: Val::Px(36.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(INTERACTION_KEY_BG),
                            BorderRadius::all(Val::Px(4.0)),
                            InteractionPromptKey,
                        ))
                        .with_children(|key_parent| {
                            key_parent.spawn((
                                Text::new("F"),
                                TextFont {
                                    font: chinese_font.font.clone(),
                                    font_size: 22.0,
                                    ..default()
                                },
                                TextColor(Color::BLACK),
                            ));
                        });

                    // 提示文字
                    parent.spawn((
                        Text::new("按 F 互動"),
                        TextFont {
                            font: chinese_font.font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(INTERACTION_TEXT_COLOR),
                        InteractionPromptText,
                    ));
                });
        });

    // 初始化狀態資源
    commands.insert_resource(InteractionPromptState::default());
}

/// 更新互動提示狀態（檢測玩家是否靠近觸發點）
pub fn update_interaction_prompt_state(
    player_query: Query<&Transform, With<Player>>,
    trigger_query: Query<(&Transform, &MissionTrigger)>,
    mut prompt_state: ResMut<InteractionPromptState>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.single() else {
        prompt_state.hide();
        prompt_state.update(time.delta_secs());
        return;
    };

    let player_pos = player_transform.translation;

    // 尋找最近的可互動觸發點 (OnInteract)
    let mut closest_trigger: Option<(&MissionTrigger, f32)> = None;

    for (trigger_transform, trigger) in &trigger_query {
        if !trigger.enabled {
            continue;
        }

        // 只處理需要互動的觸發點
        if trigger.trigger_type != TriggerType::OnInteract {
            continue;
        }

        // 檢查玩家是否在觸發範圍內
        let in_range = trigger
            .shape
            .contains(trigger_transform.translation, player_pos);
        if !in_range {
            continue;
        }

        // 計算距離
        let distance = player_pos.distance(trigger_transform.translation);

        // 保留最近的觸發點
        let is_closer = closest_trigger.map_or(true, |(_, d)| distance < d);
        if is_closer {
            closest_trigger = Some((trigger, distance));
        }
    }

    // 更新提示狀態
    if let Some((trigger, _)) = closest_trigger {
        let prompt_text = trigger
            .prompt_text
            .clone()
            .unwrap_or_else(|| "按 F 互動".to_string());
        prompt_state.show(prompt_text, "F");
    } else {
        prompt_state.hide();
    }

    prompt_state.update(time.delta_secs());
}

/// 更新互動提示 UI 顯示
pub fn update_interaction_prompt_ui(
    prompt_state: Res<InteractionPromptState>,
    mut container_query: Query<&mut Visibility, With<InteractionPromptContainer>>,
    mut text_query: Query<&mut Text, With<InteractionPromptText>>,
) {
    // 更新容器可見性
    for mut visibility in container_query.iter_mut() {
        let should_show = prompt_state.fade_progress > 0.01;
        let new_visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if *visibility != new_visibility {
            *visibility = new_visibility;
        }
    }

    // 只在狀態變更時更新文字內容
    if prompt_state.is_changed() {
        for mut text in text_query.iter_mut() {
            if **text != prompt_state.text {
                **text = prompt_state.text.clone();
            }
        }
    }
}

// ============================================================================
// GPS 導航系統 (GTA5 風格)
// ============================================================================

use super::{GpsDirectionArrow, GpsDistanceDisplay, GpsNavigationState, MinimapGpsMarker};

/// GPS 顏色常數
const GPS_ROUTE_COLOR: Color = Color::srgba(0.4, 0.8, 1.0, 0.8); // 淡藍色路線
const GPS_MARKER_COLOR: Color = Color::srgba(1.0, 0.85, 0.0, 0.9); // 黃色目標點

/// 設置 GPS UI 元素
pub fn setup_gps_ui(mut commands: Commands, font: Option<Res<ChineseFont>>) {
    let Some(font) = font else { return };

    // 屏幕頂部的方向指示箭頭（在小地圖上方）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(330.0),   // 小地圖下方
                right: Val::Px(145.0), // 居中於小地圖
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            BorderRadius::all(Val::Px(20.0)),
            Visibility::Hidden,
            GpsDirectionArrow,
            Name::new("GPS_DirectionArrow"),
        ))
        .with_children(|parent| {
            // 箭頭符號 ▲
            parent.spawn((
                Text::new("▲"),
                TextFont {
                    font: font.font.clone(),
                    font_size: 24.0,
                    ..default()
                },
                TextColor(GPS_MARKER_COLOR),
            ));
        });

    // 距離顯示（在方向箭頭下方）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(375.0),
                right: Val::Px(120.0),
                width: Val::Px(90.0),
                height: Val::Px(24.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            BorderRadius::all(Val::Px(4.0)),
            Visibility::Hidden,
            GpsDistanceDisplay,
            Name::new("GPS_DistanceDisplay"),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("0 m"),
                TextFont {
                    font: font.font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// 計算玩家面向方向與目標方向的夾角
fn calculate_gps_direction_angle(player_forward: Vec3, to_dest: Vec3) -> f32 {
    let to_dest_normalized = Vec3::new(to_dest.x, 0.0, to_dest.z).normalize_or_zero();
    let player_forward_xz = Vec3::new(player_forward.x, 0.0, player_forward.z).normalize_or_zero();
    player_forward_xz.x.atan2(player_forward_xz.z)
        - to_dest_normalized.x.atan2(to_dest_normalized.z)
}

/// 格式化 GPS 距離顯示
fn format_gps_distance(distance_xz: f32) -> String {
    if distance_xz >= 1000.0 {
        format!("{:.1} km", distance_xz / 1000.0)
    } else {
        format!("{:.0} m", distance_xz)
    }
}

/// 更新 GPS 導航狀態
pub fn update_gps_navigation(
    time: Res<Time>,
    mut gps: ResMut<GpsNavigationState>,
    player_query: Query<&Transform, With<Player>>,
    mut arrow_query: Query<
        (&mut Visibility, &mut Transform, &Children),
        (With<GpsDirectionArrow>, Without<Player>),
    >,
    mut distance_query: Query<
        (&mut Visibility, &Children),
        (
            With<GpsDistanceDisplay>,
            Without<GpsDirectionArrow>,
            Without<Player>,
        ),
    >,
    mut text_query: Query<&mut Text>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let player_forward = player_transform.forward().as_vec3();

    // 更新冷卻計時器
    if gps.route_recalc_cooldown > 0.0 {
        gps.route_recalc_cooldown -= time.delta_secs();
    }

    // 如果導航未啟用，隱藏 UI
    let should_hide = gps.destination.is_none() || !gps.active;
    if should_hide {
        for (mut vis, _, _) in arrow_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        for (mut vis, _) in distance_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let destination = gps.destination.unwrap();

    // 計算距離和方向
    let to_dest = destination - player_pos;
    let distance_xz = (to_dest.x.powi(2) + to_dest.z.powi(2)).sqrt();
    gps.distance_to_target = distance_xz;

    // 檢查是否到達目標
    if gps.is_at_destination(player_pos, 5.0) {
        gps.clear();
        return;
    }

    // 計算方向角度並更新箭頭
    let angle = calculate_gps_direction_angle(player_forward, to_dest);
    for (mut vis, mut transform, _children) in arrow_query.iter_mut() {
        *vis = Visibility::Visible;
        transform.rotation = Quat::from_rotation_z(angle);
    }

    // 更新距離顯示
    let distance_str = format_gps_distance(distance_xz);
    for (mut vis, children) in distance_query.iter_mut() {
        *vis = Visibility::Visible;
        for child in children.iter() {
            let Ok(mut text) = text_query.get_mut(child) else {
                continue;
            };
            **text = distance_str.clone();
        }
    }
}

/// 更新小地圖上的 GPS 目標標記
/// 優化：只在目標變化時重建標記，避免每幀 despawn/spawn 造成抖動
pub fn update_minimap_gps_marker(
    mut commands: Commands,
    gps: Res<GpsNavigationState>,
    minimap_query: Query<Entity, With<MinimapContainer>>,
    mut marker_query: Query<(Entity, &mut Node, &mut Visibility), With<MinimapGpsMarker>>,
) {
    // 如果 GPS 未啟用或無目標，隱藏現有標記
    if !gps.active || gps.destination.is_none() {
        for (_, _, mut vis) in marker_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let destination = gps.destination.unwrap();

    // 將世界座標轉換為小地圖座標
    let map_scale = 0.9;
    let offset_x = 150.0;
    let offset_y = 150.0;

    let minimap_x = (destination.x * map_scale + offset_x).clamp(5.0, 295.0);
    let minimap_y = (-destination.z * map_scale + offset_y).clamp(5.0, 295.0);

    // 收集現有標記
    let markers: Vec<_> = marker_query.iter_mut().collect();

    // 如果標記已存在，只更新位置和可見性（避免每幀重建）
    if markers.len() >= 2 {
        let mut iter = markers.into_iter();
        // 外圈脈衝（第一個標記）
        if let Some((_, mut node, mut vis)) = iter.next() {
            node.left = Val::Px(minimap_x - 8.0);
            node.top = Val::Px(minimap_y - 8.0);
            *vis = Visibility::Visible;
        }
        // 核心點（第二個標記）
        if let Some((_, mut node, mut vis)) = iter.next() {
            node.left = Val::Px(minimap_x - 4.0);
            node.top = Val::Px(minimap_y - 4.0);
            *vis = Visibility::Visible;
        }
        return;
    }

    // 標記不存在，需要創建（只在首次或標記被清理後）
    // 先清理可能的殘留
    for (entity, _, _) in marker_query.iter() {
        commands.entity(entity).despawn();
    }

    // 找到小地圖容器
    let Ok(minimap_entity) = minimap_query.single() else {
        return;
    };

    // 在小地圖上生成目標標記（黃色圓點 + 脈衝效果）
    commands.entity(minimap_entity).with_children(|parent| {
        // 外圈脈衝
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(minimap_x - 8.0),
                top: Val::Px(minimap_y - 8.0),
                width: Val::Px(16.0),
                height: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 0.85, 0.0, 0.3)),
            BorderRadius::all(Val::Px(8.0)),
            Visibility::Visible,
            MinimapGpsMarker,
        ));

        // 核心點
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(minimap_x - 4.0),
                top: Val::Px(minimap_y - 4.0),
                width: Val::Px(8.0),
                height: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(GPS_MARKER_COLOR),
            BorderRadius::all(Val::Px(4.0)),
            Visibility::Visible,
            MinimapGpsMarker,
        ));
    });
}

/// 根據任務類型設置 GPS 目標
fn set_gps_for_mission(gps: &mut GpsNavigationState, mission: &crate::mission::ActiveMission) {
    let data = &mission.data;

    match data.mission_type {
        MissionType::Delivery => {
            gps.set_destination(data.end_pos, "送貨目的地");
        }
        MissionType::Taxi => {
            if let Some(taxi_data) = &data.taxi_data {
                let (pos, name) = if taxi_data.passenger_picked_up {
                    (data.end_pos, taxi_data.destination_name.as_str())
                } else {
                    (data.start_pos, "接乘客")
                };
                gps.set_destination(pos, name);
            }
        }
        MissionType::Race => {
            if let Some(race_data) = &data.race_data {
                if let Some(cp) = race_data.current_checkpoint_pos() {
                    gps.set_destination(
                        cp,
                        &format!("檢查點 {}", race_data.current_checkpoint + 1),
                    );
                }
            }
        }
        MissionType::Explore => {
            gps.set_destination(data.end_pos, "目標位置");
        }
    }
}

/// 檢查是否應該清除任務導航
fn should_clear_mission_gps(destination_name: &str) -> bool {
    destination_name.contains("目的地")
        || destination_name.contains("檢查點")
        || destination_name.contains("乘客")
}

/// 處理任務開始時自動設置 GPS 目標
pub fn gps_mission_integration(
    mut gps: ResMut<GpsNavigationState>,
    mission_manager: Res<MissionManager>,
) {
    if let Some(mission) = &mission_manager.active_mission {
        if !gps.active {
            set_gps_for_mission(&mut gps, mission);
        }
    } else if gps.active && should_clear_mission_gps(&gps.destination_name) {
        gps.clear();
    }
}

// ============================================================================
// 劇情任務 HUD 系統 (GTA 5 風格)
// ============================================================================

/// 劇情任務 HUD 顏色常數
const STORY_HUD_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const STORY_HUD_TITLE_COLOR: Color = Color::srgb(1.0, 0.85, 0.0); // 金黃色標題
const STORY_HUD_PHASE_COLOR: Color = Color::srgb(0.9, 0.9, 0.9); // 白色描述
const STORY_HUD_OBJECTIVE_COLOR: Color = Color::srgb(0.7, 0.7, 0.7); // 灰白色目標
const STORY_HUD_OBJECTIVE_DONE: Color = Color::srgb(0.3, 0.8, 0.3); // 綠色完成
const STORY_HUD_TIMER_COLOR: Color = Color::srgb(1.0, 0.5, 0.3); // 橙色計時
const STORY_HUD_CHECK_EMPTY: &str = "○";
const STORY_HUD_CHECK_DONE: &str = "●";

/// 設置劇情任務 HUD
pub fn setup_story_mission_hud(mut commands: Commands, font: Option<Res<ChineseFont>>) {
    let Some(font) = font else { return };

    // 主容器（右上角，小地圖下方）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(220.0),
                right: Val::Px(10.0),
                width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(STORY_HUD_BG),
            BorderRadius::all(Val::Px(8.0)),
            Visibility::Hidden,
            StoryMissionHud,
            Name::new("StoryMissionHud"),
        ))
        .with_children(|parent| {
            // 任務標題
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(STORY_HUD_TITLE_COLOR),
                StoryMissionTitle,
            ));

            // 階段描述
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(STORY_HUD_PHASE_COLOR),
                StoryMissionPhaseText,
            ));

            // 計時器（如果有時限）
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(STORY_HUD_TIMER_COLOR),
                StoryMissionTimer,
            ));

            // 目標列表容器
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                    StoryMissionObjectiveList,
                ))
                .with_children(|list| {
                    // 預先創建 5 個目標槽位（動態顯示/隱藏）
                    for i in 0..5 {
                        list.spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            Visibility::Hidden,
                            StoryMissionObjectiveItem { index: i },
                        ))
                        .with_children(|item| {
                            // 勾選框
                            item.spawn((
                                Text::new(STORY_HUD_CHECK_EMPTY),
                                TextFont {
                                    font: font.font.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(STORY_HUD_OBJECTIVE_COLOR),
                                StoryMissionObjectiveCheck { index: i },
                            ));

                            // 目標文字
                            item.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.font.clone(),
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(STORY_HUD_OBJECTIVE_COLOR),
                                StoryMissionObjectiveText { index: i },
                            ));
                        });
                    }
                });
        });
}

/// 格式化任務計時器文字
fn format_mission_timer(time_remaining: Option<f32>) -> String {
    match time_remaining {
        Some(remaining) => {
            let mins = (remaining / 60.0).floor() as u32;
            let secs = (remaining % 60.0).floor() as u32;
            format!("⏱ {:02}:{:02}", mins, secs)
        }
        None => String::new(),
    }
}

/// 取得目標勾選框狀態
fn get_objective_check_state(is_completed: bool) -> (&'static str, Color) {
    if is_completed {
        (STORY_HUD_CHECK_DONE, STORY_HUD_OBJECTIVE_DONE)
    } else {
        (STORY_HUD_CHECK_EMPTY, STORY_HUD_OBJECTIVE_COLOR)
    }
}

/// 格式化目標文字
fn format_objective_text(description: &str, current_count: u32, target_count: u32) -> String {
    if target_count > 1 {
        format!("{} ({}/{})", description, current_count, target_count)
    } else {
        description.to_string()
    }
}

/// 取得目標文字顏色
fn get_objective_text_color(is_completed: bool) -> Color {
    if is_completed {
        STORY_HUD_OBJECTIVE_DONE
    } else {
        STORY_HUD_OBJECTIVE_COLOR
    }
}

/// 根據條件取得可見度
fn visibility_from_bool(visible: bool) -> Visibility {
    if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

/// 更新劇情任務 HUD
pub fn update_story_mission_hud(
    story_manager: Res<StoryMissionManager>,
    story_database: Res<StoryMissionDatabase>,
    mut hud_query: Query<&mut Visibility, With<StoryMissionHud>>,
    mut title_query: Query<
        &mut Text,
        (
            With<StoryMissionTitle>,
            Without<StoryMissionPhaseText>,
            Without<StoryMissionTimer>,
            Without<StoryMissionObjectiveCheck>,
            Without<StoryMissionObjectiveText>,
        ),
    >,
    mut phase_query: Query<
        &mut Text,
        (
            With<StoryMissionPhaseText>,
            Without<StoryMissionTitle>,
            Without<StoryMissionTimer>,
            Without<StoryMissionObjectiveCheck>,
            Without<StoryMissionObjectiveText>,
        ),
    >,
    mut timer_query: Query<
        &mut Text,
        (
            With<StoryMissionTimer>,
            Without<StoryMissionTitle>,
            Without<StoryMissionPhaseText>,
            Without<StoryMissionObjectiveCheck>,
            Without<StoryMissionObjectiveText>,
        ),
    >,
    mut item_query: Query<(&mut Visibility, &StoryMissionObjectiveItem), Without<StoryMissionHud>>,
    mut check_query: Query<
        (&mut Text, &mut TextColor, &StoryMissionObjectiveCheck),
        (
            Without<StoryMissionTitle>,
            Without<StoryMissionPhaseText>,
            Without<StoryMissionTimer>,
            Without<StoryMissionObjectiveText>,
        ),
    >,
    mut text_query: Query<
        (&mut Text, &mut TextColor, &StoryMissionObjectiveText),
        (
            Without<StoryMissionTitle>,
            Without<StoryMissionPhaseText>,
            Without<StoryMissionTimer>,
            Without<StoryMissionObjectiveCheck>,
        ),
    >,
) {
    let mission_info = get_current_mission_info(&story_manager, &story_database);
    let hud_visible = visibility_from_bool(mission_info.is_some());
    for mut visibility in &mut hud_query {
        *visibility = hud_visible;
    }

    let Some(info) = mission_info else { return };

    // 更新標題、階段、計時器
    if let Ok(mut t) = title_query.single_mut() {
        **t = info.title;
    }
    if let Ok(mut t) = phase_query.single_mut() {
        **t = info.phase_description;
    }
    if let Ok(mut t) = timer_query.single_mut() {
        **t = format_mission_timer(info.time_remaining);
    }

    // 更新目標列表可見度
    let obj_count = info.objectives.len();
    for (mut vis, item) in &mut item_query {
        *vis = visibility_from_bool(item.index < obj_count);
    }

    // 更新勾選框
    for (mut check_text, mut check_color, check) in &mut check_query {
        if let Some(obj) = info.objectives.get(check.index) {
            let (text, color) = get_objective_check_state(obj.is_completed);
            **check_text = text.to_string();
            check_color.0 = color;
        }
    }

    // 更新目標文字
    for (mut obj_text, mut obj_color, text_comp) in &mut text_query {
        if let Some(obj) = info.objectives.get(text_comp.index) {
            **obj_text =
                format_objective_text(&obj.description, obj.current_count, obj.target_count);
            obj_color.0 = get_objective_text_color(obj.is_completed);
        }
    }
}
