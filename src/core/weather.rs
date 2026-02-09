//! 天氣系統資源
#![allow(dead_code)]

/// 天氣類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
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

    /// 取得穩定的存檔鍵值（不受 enum 重命名影響）
    pub fn save_key(&self) -> &'static str {
        match self {
            WeatherType::Clear => "Clear",
            WeatherType::Cloudy => "Cloudy",
            WeatherType::Rainy => "Rainy",
            WeatherType::Foggy => "Foggy",
            WeatherType::Stormy => "Stormy",
            WeatherType::Sandstorm => "Sandstorm",
        }
    }
}

/// 天氣狀態資源
#[derive(bevy::prelude::Resource)]
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
