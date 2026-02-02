//! UI 常數定義
//!
//! 所有 UI 系統共用的顏色、尺寸常數
#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

use bevy::prelude::*;

// ============================================================================
// 解析度與縮放
// ============================================================================

/// 基準解析度高度（所有 UI 以 1080p 設計）
pub const BASE_RESOLUTION_HEIGHT: f32 = 1080.0;

/// 按鈕基礎尺寸
pub const BUTTON_BASE_WIDTH: f32 = 220.0;
pub const BUTTON_BASE_HEIGHT: f32 = 48.0;
/// 按鈕懸停時的縮放比例 (1.08 = 放大 8%)
pub const BUTTON_HOVER_SCALE: f32 = 1.08;
/// 按鈕按下時的縮放比例
pub const BUTTON_PRESSED_SCALE: f32 = 0.96;
/// 按鈕縮放動畫速度
pub const BUTTON_SCALE_SPEED: f32 = 15.0;

// ============================================================================
// GTA 風格 HUD 顏色
// ============================================================================

/// HUD 背景色（深藍黑）
pub const HUD_BG: Color = Color::srgba(0.05, 0.05, 0.1, 0.75);
/// HUD 邊框色
pub const HUD_BORDER: Color = Color::srgba(0.3, 0.3, 0.4, 0.5);

/// 血量圖示色（GTA 5 風格：較低飽和度）
pub const HEALTH_ICON: Color = Color::srgb(0.85, 0.2, 0.15);
/// 血量條背景
pub const HEALTH_BAR_BG: Color = Color::srgba(0.12, 0.04, 0.04, 0.85);
/// 血量條填充（GTA 5 風格：較深沉的紅）
pub const HEALTH_BAR_FILL_COLOR: Color = Color::srgb(0.75, 0.18, 0.12);
/// 血量條高光
pub const HEALTH_BAR_HIGHLIGHT_COLOR: Color = Color::srgb(0.9, 0.3, 0.2);

/// 護甲圖示色（GTA 5 風格：較低飽和度）
pub const ARMOR_ICON: Color = Color::srgb(0.35, 0.6, 0.85);
/// 護甲條背景
pub const ARMOR_BAR_BG: Color = Color::srgba(0.04, 0.08, 0.15, 0.85);
/// 護甲條填充（GTA 5 風格：較深沉的藍）
pub const ARMOR_BAR_FILL_COLOR: Color = Color::srgb(0.22, 0.5, 0.78);
/// 護甲條高光
pub const ARMOR_BAR_HIGHLIGHT_COLOR: Color = Color::srgb(0.4, 0.68, 0.88);

/// 金錢文字色（GTA 5 風格：較低飽和度的綠）
pub const MONEY_TEXT_COLOR: Color = Color::srgb(0.25, 0.85, 0.35);
/// 金錢背景色
pub const MONEY_BG: Color = Color::srgba(0.04, 0.08, 0.04, 0.85);

/// 彈藥正常色（GTA 5 風格：較柔和的金黃）
pub const AMMO_NORMAL: Color = Color::srgb(0.95, 0.88, 0.45);
/// 彈藥低量色（GTA 5 風格：較深沉的橙紅）
pub const AMMO_LOW: Color = Color::srgb(0.92, 0.4, 0.3);
/// 後備彈藥色
pub const AMMO_RESERVE: Color = Color::srgba(0.75, 0.75, 0.78, 0.85);

/// 子彈圖示填充色（金黃色）
pub const BULLET_FILLED: Color = Color::srgb(0.95, 0.85, 0.35);
/// 子彈圖示空彈色（暗灰）
pub const BULLET_EMPTY: Color = Color::srgba(0.25, 0.25, 0.3, 0.5);
/// 子彈圖示低彈量閃爍色（警示橙紅）
pub const BULLET_LOW_WARN: Color = Color::srgb(0.95, 0.4, 0.3);

/// 武器槽選中色
pub const SLOT_ACTIVE: Color = Color::srgba(0.4, 0.6, 0.9, 0.9);
/// 武器槽未選中色
pub const SLOT_INACTIVE: Color = Color::srgba(0.2, 0.2, 0.25, 0.6);

// ============================================================================
// GTA 風格地圖顏色
// ============================================================================

/// 小地圖外框發光色（雷達感）
pub const MINIMAP_GLOW: Color = Color::srgba(0.2, 0.5, 0.3, 0.4);
/// 小地圖主邊框色
pub const MINIMAP_BORDER: Color = Color::srgba(0.15, 0.35, 0.2, 0.95);
/// 小地圖內邊框色（陰影效果）
pub const MINIMAP_INNER_BORDER: Color = Color::srgba(0.05, 0.15, 0.08, 0.9);
/// 小地圖背景色（深綠軍事風）
pub const MINIMAP_BG: Color = Color::srgba(0.08, 0.12, 0.08, 0.92);
/// 小地圖內層背景（稍亮）
pub const MINIMAP_BG_INNER: Color = Color::srgba(0.1, 0.15, 0.1, 0.95);
/// 玩家標記發光色
pub const PLAYER_MARKER_GLOW: Color = Color::srgba(1.0, 0.9, 0.3, 0.6);
/// 玩家標記核心色
pub const PLAYER_MARKER_CORE: Color = Color::srgb(1.0, 0.15, 0.1);
/// 方位標示背景色
pub const COMPASS_BG: Color = Color::srgba(0.1, 0.1, 0.1, 0.7);
/// 北方標示色（紅色更醒目）
pub const COMPASS_NORTH: Color = Color::srgb(1.0, 0.3, 0.2);

/// 大地圖背景色
pub const FULLMAP_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.88);
/// 大地圖主體背景
pub const FULLMAP_MAIN_BG: Color = Color::srgb(0.12, 0.15, 0.1);
/// 大地圖邊框色
pub const FULLMAP_BORDER: Color = Color::srgb(0.35, 0.4, 0.3);
/// 大地圖標題背景
pub const FULLMAP_TITLE_BG: Color = Color::srgba(0.1, 0.15, 0.1, 0.9);

// ============================================================================
// GTA 風格暫停選單顏色
// ============================================================================

/// 暫停選單外層背景（毛玻璃效果第一層）
pub const PAUSE_BG_OUTER: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
/// 暫停選單內層背景（毛玻璃效果第二層）
pub const PAUSE_BG_INNER: Color = Color::srgba(0.02, 0.02, 0.05, 0.4);
/// 暫停選單面板外發光
pub const PAUSE_PANEL_GLOW: Color = Color::srgba(0.3, 0.35, 0.4, 0.3);
/// 暫停選單面板主邊框
pub const PAUSE_PANEL_BORDER: Color = Color::srgba(0.4, 0.45, 0.5, 0.8);
/// 暫停選單面板內邊框
pub const PAUSE_PANEL_INNER_BORDER: Color = Color::srgba(0.15, 0.15, 0.2, 0.9);
/// 暫停選單面板背景
pub const PAUSE_PANEL_BG: Color = Color::srgba(0.08, 0.08, 0.12, 0.95);
/// 暫停選單標題色
pub const PAUSE_TITLE_COLOR: Color = Color::srgb(0.95, 0.95, 0.98);
/// 繼續按鈕正常色
pub const RESUME_BTN_NORMAL: Color = Color::srgb(0.15, 0.45, 0.25);
/// 繼續按鈕懸停色
pub const RESUME_BTN_HOVER: Color = Color::srgb(0.2, 0.6, 0.3);
/// 繼續按鈕按下色
pub const RESUME_BTN_PRESSED: Color = Color::srgb(0.1, 0.35, 0.18);
/// 繼續按鈕邊框色
pub const RESUME_BTN_BORDER: Color = Color::srgba(0.3, 0.7, 0.4, 0.8);
/// 退出按鈕正常色
pub const QUIT_BTN_NORMAL: Color = Color::srgb(0.5, 0.15, 0.15);
/// 退出按鈕懸停色
pub const QUIT_BTN_HOVER: Color = Color::srgb(0.7, 0.2, 0.2);
/// 退出按鈕按下色
pub const QUIT_BTN_PRESSED: Color = Color::srgb(0.4, 0.1, 0.1);
/// 退出按鈕邊框色
pub const QUIT_BTN_BORDER: Color = Color::srgba(0.8, 0.3, 0.3, 0.8);
/// 暫停選單提示文字色
pub const PAUSE_HINT_COLOR: Color = Color::srgba(0.6, 0.6, 0.65, 0.8);
/// 暫停選單副標題色
pub const PAUSE_SUBTITLE_COLOR: Color = Color::srgba(0.5, 0.5, 0.55, 0.7);

// ============================================================================
// GTA 風格準星顏色
// ============================================================================

/// 準星主色（白色微透明）
pub const CROSSHAIR_MAIN: Color = Color::srgba(1.0, 1.0, 1.0, 0.9);
/// 準星陰影色（輪廓）
pub const CROSSHAIR_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
/// 準星外圈色
pub const CROSSHAIR_OUTER_RING: Color = Color::srgba(1.0, 1.0, 1.0, 0.3);
/// 命中標記色（亮紅）
pub const HIT_MARKER_COLOR: Color = Color::srgba(1.0, 0.2, 0.2, 0.95);
/// 爆頭標記色（金黃）
pub const HEADSHOT_MARKER_COLOR: Color = Color::srgba(1.0, 0.85, 0.2, 1.0);
/// 準星瞄準時色（收縮變亮）
pub const CROSSHAIR_AIM: Color = Color::srgba(0.9, 1.0, 0.95, 0.95);

// ============================================================================
// GTA 風格敵人血條顏色
// ============================================================================

/// 敵人血條外發光（紅色輝光）
pub const ENEMY_BAR_GLOW: Color = Color::srgba(0.8, 0.2, 0.2, 0.3);
/// 敵人血條邊框色
pub const ENEMY_BAR_BORDER: Color = Color::srgba(0.1, 0.1, 0.12, 0.95);
/// 敵人血條背景色
pub const ENEMY_BAR_BG: Color = Color::srgba(0.05, 0.05, 0.08, 0.9);
/// 敵人血條滿血色（綠色）
pub const ENEMY_HEALTH_FULL: Color = Color::srgb(0.2, 0.8, 0.3);
/// 敵人血條中血色（黃色）
pub const ENEMY_HEALTH_MID: Color = Color::srgb(0.9, 0.8, 0.2);
/// 敵人血條低血色（紅色）
pub const ENEMY_HEALTH_LOW: Color = Color::srgb(0.9, 0.2, 0.2);
/// 敵人血條高光色
pub const ENEMY_BAR_HIGHLIGHT: Color = Color::srgba(1.0, 1.0, 1.0, 0.2);

// ============================================================================
// 受傷指示器常數
// ============================================================================

/// 受傷指示器主色（血紅色暈影）
pub const DAMAGE_INDICATOR_COLOR: Color = Color::srgba(0.6, 0.0, 0.0, 0.0);
/// 受傷指示器最大透明度
pub const DAMAGE_INDICATOR_MAX_ALPHA: f32 = 0.5;
/// 受傷指示器邊緣寬度
pub const DAMAGE_EDGE_WIDTH: f32 = 150.0;
/// 受傷指示器淡出速度
pub const DAMAGE_FADE_RATE: f32 = 2.0;

// ============================================================================
// GTA 風格外送 App 顏色
// ============================================================================

/// 外送 App 外發光色（橘色輝光）
pub const DELIVERY_APP_GLOW: Color = Color::srgba(0.9, 0.4, 0.1, 0.25);
/// 外送 App 主邊框色
pub const DELIVERY_APP_BORDER: Color = Color::srgb(0.9, 0.4, 0.1);
/// 外送 App 內邊框色
pub const DELIVERY_APP_INNER_BORDER: Color = Color::srgba(0.4, 0.2, 0.05, 0.9);
/// 外送 App 背景色
pub const DELIVERY_APP_BG: Color = Color::srgba(0.08, 0.06, 0.1, 0.95);
/// 外送 App 標題色
pub const DELIVERY_APP_TITLE: Color = Color::srgb(1.0, 0.5, 0.15);
/// 外送 App 副標題色
pub const DELIVERY_APP_SUBTITLE: Color = Color::srgba(0.7, 0.7, 0.7, 0.9);
/// 訂單卡片外發光
pub const ORDER_CARD_GLOW: Color = Color::srgba(0.5, 0.3, 0.1, 0.15);
/// 訂單卡片背景
pub const ORDER_CARD_BG: Color = Color::srgba(0.12, 0.1, 0.15, 0.92);
/// 訂單卡片邊框
pub const ORDER_CARD_BORDER: Color = Color::srgba(0.6, 0.35, 0.15, 0.6);
/// 訂單卡片懸停邊框
pub const ORDER_CARD_HOVER_BORDER: Color = Color::srgba(0.9, 0.5, 0.2, 0.9);
/// 餐廳名稱色
pub const RESTAURANT_NAME_COLOR: Color = Color::srgb(1.0, 0.85, 0.6);
/// 地址文字色
pub const ADDRESS_TEXT_COLOR: Color = Color::srgba(0.75, 0.75, 0.8, 0.9);
/// 報酬金額色（綠色）
pub const REWARD_TEXT_COLOR: Color = Color::srgb(0.3, 0.95, 0.4);
/// 評價星星色（金黃）
pub const RATING_STAR_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);
/// 連擊數字色（橘紅）
pub const STREAK_COLOR: Color = Color::srgb(1.0, 0.5, 0.2);

// ============================================================================
// GTA 風格控制提示顏色
// ============================================================================

/// 控制提示背景色
pub const CONTROL_HINT_BG: Color = Color::srgba(0.05, 0.05, 0.08, 0.85);
/// 控制提示邊框色
pub const CONTROL_HINT_BORDER: Color = Color::srgba(0.25, 0.25, 0.3, 0.7);
/// 按鍵圖示背景色
pub const KEY_ICON_BG: Color = Color::srgba(0.15, 0.15, 0.2, 0.95);
/// 按鍵圖示邊框色
pub const KEY_ICON_BORDER: Color = Color::srgba(0.4, 0.4, 0.5, 0.9);
/// 按鍵文字色
pub const KEY_TEXT_COLOR: Color = Color::srgb(0.95, 0.95, 0.98);
/// 動作說明文字色
pub const ACTION_TEXT_COLOR: Color = Color::srgba(0.75, 0.75, 0.8, 0.9);
/// 狀態標籤背景色（步行/駕駛）
pub const STATUS_TAG_BG: Color = Color::srgba(0.2, 0.4, 0.25, 0.9);
/// 速度顯示色
pub const SPEED_TEXT_COLOR: Color = Color::srgb(0.4, 0.9, 0.5);

// ============================================================================
// 文字與陰影
// ============================================================================

/// 文字陰影色（GTA 風格深色陰影）
pub const TEXT_SHADOW_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 0.65);
/// 文字陰影偏移量（像素）
pub const TEXT_SHADOW_OFFSET: f32 = 1.5;

// ============================================================================
// 多層邊框效果色系
// ============================================================================

/// HUD 容器外發光色（藍色微光）
pub const HUD_GLOW_OUTER: Color = Color::srgba(0.3, 0.5, 0.8, 0.2);
/// HUD 邊框高亮色
pub const HUD_BORDER_HIGHLIGHT: Color = Color::srgba(0.5, 0.6, 0.75, 0.6);

// ============================================================================
// 通用 UI 顏色（減少內聯重複）
// ============================================================================

/// 深黑半透明背景（90%）
pub const OVERLAY_BLACK_90: Color = Color::srgba(0.0, 0.0, 0.0, 0.9);
/// 深黑半透明背景（70%）
pub const OVERLAY_BLACK_70: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
/// 深灰按鈕背景
pub const BUTTON_BG_DARK: Color = Color::srgba(0.2, 0.2, 0.25, 0.8);
/// 深灰按鈕邊框（70%）
pub const BUTTON_BORDER_GRAY_70: Color = Color::srgba(0.4, 0.4, 0.45, 0.7);
/// 深灰按鈕邊框（60%）
pub const BUTTON_BORDER_GRAY_60: Color = Color::srgba(0.4, 0.4, 0.45, 0.6);
/// 灰色文字色（90%）
pub const TEXT_GRAY_90: Color = Color::srgba(0.7, 0.7, 0.7, 0.9);
/// 淺灰文字色
pub const TEXT_LIGHT_GRAY: Color = Color::srgba(0.8, 0.8, 0.85, 0.9);
/// 次要文字色
pub const TEXT_SECONDARY: Color = Color::srgba(0.65, 0.65, 0.7, 0.9);
/// 低飽和灰色
pub const TEXT_MUTED: Color = Color::srgba(0.5, 0.5, 0.55, 0.8);
/// 面板邊框灰
pub const PANEL_BORDER_GRAY: Color = Color::srgba(0.3, 0.3, 0.35, 0.4);
/// 綠色地圖區塊
pub const MAP_AREA_GREEN: Color = Color::srgba(0.3, 0.4, 0.3, 0.1);
/// 亮白色
pub const TEXT_WHITE: Color = Color::srgb(0.95, 0.95, 0.95);
/// 金黃標題色
pub const TITLE_GOLD: Color = Color::srgb(1.0, 0.85, 0.0);
/// 誠品綠
pub const ESLITE_GREEN: Color = Color::srgb(0.2, 0.35, 0.25);
