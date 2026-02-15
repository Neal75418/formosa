//! 攝影機系統常數

/// 攝影機自動跟隨速度（越大越快跟上玩家）
pub const CAMERA_FOLLOW_SPEED: f32 = 3.0;

/// 俯仰角最小值（約 -17°，微微仰望）
pub const PITCH_MIN: f32 = -0.3;
/// 俯仰角最大值 — 正常輸入範圍（約 69°）
pub const PITCH_MAX_INPUT: f32 = 1.2;
/// 俯仰角最大值 — 含後座力影響（約 86°）
/// 比 PITCH_MAX_INPUT 寬，允許後座力暫時超過正常輸入上限
pub const PITCH_MAX_WITH_RECOIL: f32 = 1.5;

/// 距離調整係數（非瞄準時滑鼠 Y 軸每像素的距離變化）
pub const DISTANCE_MOUSE_FACTOR: f32 = 0.1;
/// 滾輪距離調整步長
pub const DISTANCE_SCROLL_STEP: f32 = 0.4;
/// 攝影機距離最小值
pub const DISTANCE_MIN: f32 = 5.0;
/// 攝影機距離最大值
pub const DISTANCE_MAX: f32 = 80.0;

/// 瞄準時跟隨插值速度
pub const AIM_FOLLOW_LERP_SPEED: f32 = 15.0;
/// 非瞄準時跟隨插值速度
pub const NORMAL_FOLLOW_LERP_SPEED: f32 = 8.0;
/// 瞄準注視點 Y 偏移
pub const AIM_LOOK_TARGET_Y_OFFSET: f32 = 1.5;
/// 非瞄準時後座力影響係數
pub const TPS_RECOIL_FACTOR: f32 = 0.3;

/// 車內視角俯仰角最大值
pub const VEHICLE_INTERIOR_PITCH_MAX: f32 = 0.8;

/// 電影模式俯仰角限制（±80°）
pub const CINEMATIC_PITCH_LIMIT: f32 = 1.4;

/// 鎖定目標注視混合比例
pub const LOCK_ON_LOOK_BLEND: f32 = 0.3;
/// 鎖定目標 Y 偏移
pub const LOCK_ON_Y_OFFSET: f32 = 1.0;
