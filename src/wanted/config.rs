//! 通緝系統常數
//!
//! 警察生成、戰鬥、巡邏、無線電等可調參數。

// ============================================================================
// 視野與搜索
// ============================================================================

/// 視野檢測距離（公尺）
pub const VISION_RANGE: f32 = 40.0;

/// 射線原點高度偏移（避免地面干擾）
pub const RAYCAST_ORIGIN_HEIGHT: f32 = 1.5;

/// 射線命中容許距離
pub const RAYCAST_HIT_TOLERANCE: f32 = 1.0;

// ============================================================================
// 無線電與搜索
// ============================================================================

/// 無線電呼叫範圍（公尺）
pub const RADIO_CALL_RANGE: f32 = 45.0;

/// 無線電呼叫冷卻時間（秒）
pub const RADIO_CALL_COOLDOWN: f32 = 5.0;

/// 犯罪搜索半徑
pub const CRIME_SEARCH_RADIUS: f32 = 30.0;

/// 犯罪搜索逾時（秒）
pub const CRIME_SEARCH_TIMEOUT: f32 = 45.0;

/// 警察搜索後返回閾值（秒）— 超過此時間且通緝歸零則撤離
pub const POLICE_SEARCH_RETURN_THRESHOLD: f32 = 30.0;

// ============================================================================
// 巡邏
// ============================================================================

/// 巡邏移動速度
pub const PATROL_SPEED: f32 = 2.5;

/// 到達巡邏點的距離閾值
pub const PATROL_WAYPOINT_THRESHOLD: f32 = 1.5;

/// 巡邏路線偏移半徑
pub const PATROL_OFFSET_RADIUS: f32 = 15.0;

// ============================================================================
// 警察生成參數
// ============================================================================

/// 警察初始血量
pub const POLICE_OFFICER_HEALTH: f32 = 100.0;

/// 巡邏員步行速度
pub const OFFICER_WALK_SPEED: f32 = 3.0;

/// 巡邏員跑步速度
pub const OFFICER_RUN_SPEED: f32 = 5.5;

/// SWAT 跑步速度
pub const SWAT_RUN_SPEED: f32 = 7.0;

/// SWAT 出現的最低星數
pub const SWAT_STAR_THRESHOLD: u8 = 3;

/// 軍人出現的最低星數
pub const MILITARY_STAR_THRESHOLD: u8 = 5;

/// 軍人血量（高於普通警察）
pub const MILITARY_HEALTH: f32 = 150.0;

/// 軍人跑步速度
pub const MILITARY_RUN_SPEED: f32 = 8.0;

/// 軍人傷害值（步槍）
pub const MILITARY_DAMAGE: f32 = 25.0;

/// 軍人攻擊冷卻（秒）
pub const MILITARY_ATTACK_COOLDOWN: f32 = 1.0;

/// 軍人基礎命中率
pub const MILITARY_HIT_CHANCE: f32 = 0.35;

/// 警察膠囊碰撞器半高
pub const OFFICER_CAPSULE_HALF_HEIGHT: f32 = 0.4;

/// 警察膠囊碰撞器半徑
pub const OFFICER_CAPSULE_RADIUS: f32 = 0.25;

/// 角色控制器偏移
pub const OFFICER_CONTROLLER_OFFSET: f32 = 0.1;

/// 生成高度偏移
pub const OFFICER_SPAWN_HEIGHT: f32 = 0.9;

// ============================================================================
// 戰鬥
// ============================================================================

/// 未命中偏移範圍（XZ 軸）
pub const MISS_OFFSET_RANGE: f32 = 2.0;

/// 未命中偏移範圍（Y 軸）
pub const MISS_OFFSET_Y_RANGE: f32 = 1.5;

/// 目標高度偏移
pub const TARGET_HEIGHT_OFFSET: f32 = 1.0;

/// 槍口閃光高度偏移
pub const MUZZLE_FLASH_HEIGHT: f32 = 1.5;

/// 槍口前方偏移
pub const MUZZLE_FORWARD_OFFSET: f32 = 0.5;
