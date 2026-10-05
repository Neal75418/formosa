//! 從路網推算位置的規則與常數（純函式，換地圖時直接沿用）

/// 車行道兩側人行道的寬度（全路網共用一個值）
pub const SIDEWALK_WIDTH: f32 = 4.0;

/// 行人逃跑目標離最外圍道路中線的距離
pub const FLEE_INSET: f32 = 5.0;
