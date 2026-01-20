//! 遊戲資源

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

/// 第三人稱攝影機跟隨目標
#[derive(Component)]
pub struct ThirdPersonCameraTarget;

/// 攝影機設定
#[derive(Resource)]
pub struct CameraSettings {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub sensitivity: f32,
    // 瞄準模式參數
    pub aim_shoulder_offset: f32,  // 過肩偏移（正值=右肩）
    pub aim_distance: f32,         // 瞄準時攝影機距離
    pub aim_pitch: f32,            // 瞄準時俯仰角
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.5,           // 初始俯視角度稍高
            distance: 18.0,       // 距離稍近
            sensitivity: 0.006,   // 提高靈敏度
            // 瞄準模式
            aim_shoulder_offset: 1.5,  // 向右肩偏移 1.5 公尺
            aim_distance: 8.0,         // 瞄準時拉近攝影機
            aim_pitch: 0.2,            // 瞄準時降低俯角
        }
    }
}

/// 玩家狀態（HUD 顯示用）
#[derive(Resource)]
pub struct PlayerStats {
    pub health: f32,
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

/// Debug 設定（F3 切換）
#[derive(Resource, Default)]
pub struct DebugSettings {
    /// 顯示 AI 視野範圍
    pub show_ai_ranges: bool,
}

// === 射擊回饋系統 ===

/// 後座力狀態（影響攝影機抖動）
#[derive(Resource, Default)]
pub struct RecoilState {
    /// 當前累積的後座力偏移（X=水平，Y=垂直）
    pub current_offset: Vec2,
    /// 是否正在恢復中
    pub is_recovering: bool,
}

impl RecoilState {
    /// 添加後座力
    pub fn add_recoil(&mut self, vertical: f32, horizontal: f32) {
        // 垂直後座力累加
        self.current_offset.y += vertical;
        // 水平後座力隨機左右偏移
        let h_dir = if rand::random::<bool>() { 1.0 } else { -1.0 };
        self.current_offset.x += horizontal * h_dir;
        // 限制最大後座力
        self.current_offset.y = self.current_offset.y.min(0.5);
        self.current_offset.x = self.current_offset.x.clamp(-0.3, 0.3);
        self.is_recovering = false;
    }

    /// 更新後座力恢復
    pub fn update_recovery(&mut self, recovery_rate: f32, dt: f32) {
        if self.current_offset.length_squared() < 0.0001 {
            self.current_offset = Vec2::ZERO;
            return;
        }

        self.is_recovering = true;
        // 平滑恢復到零點
        let recovery = recovery_rate * dt;
        self.current_offset.y = (self.current_offset.y - recovery).max(0.0);
        self.current_offset.x *= 1.0 - recovery * 2.0;
    }
}

/// 攝影機震動狀態
#[derive(Resource, Default)]
pub struct CameraShake {
    /// 震動強度
    pub intensity: f32,
    /// 震動持續時間
    pub duration: f32,
    /// 剩餘時間
    pub timer: f32,
}

impl CameraShake {
    /// 觸發攝影機震動
    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        // 如果新震動更強，覆蓋舊的
        if intensity > self.intensity || self.timer <= 0.0 {
            self.intensity = intensity;
            self.duration = duration;
            self.timer = duration;
        }
    }

    /// 取得當前震動偏移
    pub fn get_offset(&self, time: f32) -> Vec3 {
        if self.timer <= 0.0 {
            return Vec3::ZERO;
        }

        let progress = self.timer / self.duration;
        let decay = progress * progress; // 平方衰減更自然

        // 使用高頻正弦波產生震動
        let shake_x = (time * 50.0).sin() * self.intensity * decay;
        let shake_y = (time * 60.0).cos() * self.intensity * decay * 0.5;
        let shake_z = (time * 40.0).sin() * self.intensity * decay * 0.3;

        Vec3::new(shake_x, shake_y, shake_z)
    }

    /// 更新震動計時器
    pub fn update(&mut self, dt: f32) {
        if self.timer > 0.0 {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.intensity = 0.0;
            }
        }
    }
}

// === 碰撞群組定義 ===
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
pub const COLLISION_GROUP_PROJECTILE: Group = Group::GROUP_4;

// === 天氣系統 ===

/// 天氣類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeatherType {
    #[default]
    Clear,      // 晴天
    Cloudy,     // 陰天
    Rainy,      // 雨天
    Foggy,      // 霧天
    Stormy,     // 暴風雨（強風+大雨+閃電）
    Sandstorm,  // 沙塵暴（能見度極低+沙粒效果）
}

impl WeatherType {
    /// 切換到下一個天氣類型
    pub fn next(&self) -> Self {
        match self {
            WeatherType::Clear => WeatherType::Cloudy,
            WeatherType::Cloudy => WeatherType::Rainy,
            WeatherType::Rainy => WeatherType::Stormy,
            WeatherType::Stormy => WeatherType::Foggy,
            WeatherType::Foggy => WeatherType::Sandstorm,
            WeatherType::Sandstorm => WeatherType::Clear,
        }
    }

    /// 取得天氣名稱
    pub fn name(&self) -> &'static str {
        match self {
            WeatherType::Clear => "晴天",
            WeatherType::Cloudy => "陰天",
            WeatherType::Rainy => "雨天",
            WeatherType::Foggy => "霧天",
            WeatherType::Stormy => "暴風雨",
            WeatherType::Sandstorm => "沙塵暴",
        }
    }

    /// 是否為降雨類型
    pub fn has_rain(&self) -> bool {
        matches!(self, WeatherType::Rainy | WeatherType::Stormy)
    }

    /// 是否為惡劣天氣（影響駕駛和視線）
    pub fn is_severe(&self) -> bool {
        matches!(self, WeatherType::Stormy | WeatherType::Sandstorm)
    }
}

/// 天氣狀態資源
#[derive(Resource)]
pub struct WeatherState {
    /// 當前天氣類型
    pub weather_type: WeatherType,
    /// 天氣強度 (0.0 - 1.0)
    pub intensity: f32,
    /// 天氣過渡進度 (0.0 = 舊天氣, 1.0 = 新天氣)
    pub transition_progress: f32,
    /// 是否正在過渡
    pub is_transitioning: bool,
    /// 目標天氣（過渡中使用）
    pub target_weather: WeatherType,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            weather_type: WeatherType::Clear,
            intensity: 1.0,
            transition_progress: 1.0,
            is_transitioning: false,
            target_weather: WeatherType::Clear,
        }
    }
}

impl WeatherState {
    /// 開始切換天氣
    pub fn start_transition(&mut self, target: WeatherType) {
        if self.weather_type != target && !self.is_transitioning {
            self.target_weather = target;
            self.is_transitioning = true;
            self.transition_progress = 0.0;
        }
    }

    /// 更新天氣過渡
    pub fn update_transition(&mut self, delta_secs: f32) {
        if !self.is_transitioning {
            return;
        }

        // 過渡速度：5 秒完成
        let transition_speed = 0.2;
        self.transition_progress += delta_secs * transition_speed;

        if self.transition_progress >= 1.0 {
            self.transition_progress = 1.0;
            self.weather_type = self.target_weather;
            self.is_transitioning = false;
        }
    }

    /// 取得當前有效強度（考慮過渡）
    pub fn effective_intensity(&self) -> f32 {
        if self.is_transitioning {
            // 過渡期間使用 smooth step
            let t = self.transition_progress;
            let smooth_t = t * t * (3.0 - 2.0 * t);
            smooth_t * self.intensity
        } else {
            self.intensity
        }
    }
}

// ============================================================================
// 通用實體池（物件池模式）
// ============================================================================

/// 通用實體物件池
///
/// 避免頻繁的 spawn/despawn 造成記憶體分配開銷。
/// 實體結束生命週期時歸還池中重用，而非銷毀。
///
/// # 使用範例
/// ```ignore
/// let mut pool = EntityPool::new(100);
/// // 取得實體
/// if let Some(entity) = pool.acquire() {
///     // 使用實體...
///     pool.confirm_acquire(entity);
/// }
/// // 歸還實體
/// pool.release(entity);
/// ```
#[derive(Default, Clone)]
pub struct EntityPool {
    /// 可用的實體（已隱藏/閒置）
    pub available: Vec<Entity>,
    /// 正在使用的實體
    pub in_use: Vec<Entity>,
    /// 池最大大小
    pub max_size: usize,
}

impl EntityPool {
    /// 創建指定大小的實體池
    pub fn new(max_size: usize) -> Self {
        Self {
            available: Vec::with_capacity(max_size),
            in_use: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// 從池中取得一個實體（僅標記為候選）
    pub fn acquire(&mut self) -> Option<Entity> {
        self.available.pop()
    }

    /// 確認取得實體（將實體加入使用中列表）
    pub fn confirm_acquire(&mut self, entity: Entity) {
        self.in_use.push(entity);
    }

    /// 取消取得（將實體退回可用列表）
    pub fn cancel_acquire(&mut self, entity: Entity) {
        self.available.push(entity);
    }

    /// 歸還實體到池中
    ///
    /// 使用 swap_remove 保持 O(1) 移除，搜索為 O(n)
    pub fn release(&mut self, entity: Entity) {
        if let Some(idx) = self.in_use.iter().position(|&e| e == entity) {
            self.in_use.swap_remove(idx);
            if self.available.len() < self.max_size {
                self.available.push(entity);
            }
        }
    }

    /// 清理無效實體（當外部系統刪除了池中的實體時使用）
    pub fn cleanup_invalid(&mut self, is_valid: impl Fn(Entity) -> bool) {
        self.in_use.retain(|&e| is_valid(e));
        self.available.retain(|&e| is_valid(e));
    }

    /// 取得目前使用中的實體數量
    pub fn active_count(&self) -> usize {
        self.in_use.len()
    }

    /// 取得池中可用的實體數量
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// 檢查是否可以生成更多實體
    pub fn can_spawn(&self) -> bool {
        !self.available.is_empty() || self.in_use.len() < self.max_size
    }

    /// 檢查池是否還有空間
    pub fn has_capacity(&self) -> bool {
        self.available.len() < self.max_size || !self.available.is_empty()
    }
}

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
