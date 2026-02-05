use bevy::prelude::*;

/// AI 配置資源
#[derive(Resource, Debug, Clone, Reflect)]
#[reflect(Resource)]
pub struct AiConfig {
    // === 感知相關 ===
    /// AI 眼睛高度（發射視線的起點）
    pub eye_height: f32,
    /// 玩家身體中心高度（視線目標）
    pub player_body_height: f32,
    /// 視線遮擋容差（95% 距離內無遮擋視為可見）
    pub line_of_sight_tolerance: f32,

    // === 天氣影響 ===
    /// 晴天視野乘數
    pub weather_clear_sight: f32,
    /// 陰天視野乘數
    pub weather_cloudy_sight: f32,
    /// 雨天基礎視野乘數
    pub weather_rainy_sight_base: f32,
    /// 雨天強度衰減
    pub weather_rainy_sight_decay: f32,
    /// 霧天基礎視野乘數
    pub weather_foggy_sight_base: f32,
    /// 霧天強度衰減
    pub weather_foggy_sight_decay: f32,

    // === 行為閾值 ===
    /// 逃跑時的移動距離
    pub flee_distance: f32,
    /// 警戒距離（保持安全距離）
    pub alert_distance: f32,
    /// 巡邏待機計時器閾值
    pub patrol_idle_threshold: f32,
    /// 警戒狀態超時（秒）
    pub alert_timeout: f32,
    /// 失去目標超時（秒）
    pub lose_target_timeout: f32,
    /// 低血量閾值（觸發撤退）
    pub low_health_threshold: f32,

    // === 射擊精度 ===
    /// 最小射擊精度
    pub min_accuracy: f32,
    /// 最大距離懲罰
    pub max_range_penalty: f32,
    /// 射擊散佈範圍 X
    pub miss_spread_x: f32,
    /// 射擊散佈範圍 Y (上/下)
    pub miss_spread_y_min: f32,
    pub miss_spread_y_max: f32,
    /// 射擊散佈範圍 Z
    pub miss_spread_z: f32,

    // === 生成相關 ===
    /// 最小生成距離
    pub min_spawn_distance: f32,
    /// 最小生成距離備用（防止過近）
    pub min_spawn_radius_buffer: f32,

    // === 槍口位置 ===
    /// 槍口前方偏移
    pub muzzle_forward_offset: f32,
    /// 槍口高度偏移
    pub muzzle_height_offset: f32,

    // === 小隊角色分配閾值 ===
    /// Gangster 衝鋒者機率
    pub gangster_rusher_threshold: f32,
    /// Gangster 側翼者機率（累積）
    pub gangster_flanker_threshold: f32,
    /// Thug 衝鋒者機率
    pub thug_rusher_threshold: f32,
    /// Thug 側翼者機率（累積）
    pub thug_flanker_threshold: f32,

    // === 距離平方常數 ===
    /// 掩體到達距離平方 (1.5²)
    pub cover_arrival_sq: f32,
    /// 包抄到達距離平方 (2.0²)
    pub flank_arrival_sq: f32,
    /// 包抄自我過濾距離平方 (0.5²)
    pub flank_self_filter_distance_sq: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            eye_height: 1.5,
            player_body_height: 1.0,
            line_of_sight_tolerance: 0.95,
            weather_clear_sight: 1.0,
            weather_cloudy_sight: 0.95,
            weather_rainy_sight_base: 0.8,
            weather_rainy_sight_decay: 0.2,
            weather_foggy_sight_base: 0.5,
            weather_foggy_sight_decay: 0.2,
            flee_distance: 30.0,
            alert_distance: 40.0,
            patrol_idle_threshold: 3.0,
            alert_timeout: 5.0,
            lose_target_timeout: 5.0,
            low_health_threshold: 0.7,
            min_accuracy: 0.1,
            max_range_penalty: 0.5,
            miss_spread_x: 2.0,
            miss_spread_y_min: -1.0,
            miss_spread_y_max: 1.5,
            miss_spread_z: 2.0,
            min_spawn_distance: 45.0,
            min_spawn_radius_buffer: 5.0,
            muzzle_forward_offset: 0.5,
            muzzle_height_offset: 0.3,
            gangster_rusher_threshold: 0.5,
            gangster_flanker_threshold: 0.9,
            thug_rusher_threshold: 0.7,
            thug_flanker_threshold: 0.9, // 70% Rusher, 20% Flanker, 10% Defender
            cover_arrival_sq: 2.25,
            flank_arrival_sq: 4.0,
            flank_self_filter_distance_sq: 0.25,
        }
    }
}

impl AiConfig {
    /// 取得警戒距離平方 (效能優化)
    pub fn alert_distance_sq(&self) -> f32 {
        self.alert_distance * self.alert_distance
    }
}
