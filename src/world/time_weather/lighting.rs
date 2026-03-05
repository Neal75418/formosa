//! 日夜光照系統（太陽月亮軌跡、環境光、路燈）

use bevy::prelude::*;

use crate::core::{WeatherState, WeatherType, WorldTime};
use crate::world::{Moon, StreetLight, Sun};

/// 更新世界時間
pub fn update_world_time(time: Res<Time>, mut world_time: ResMut<WorldTime>) {
    world_time.hour += time.delta_secs() * world_time.time_scale / 3600.0;
    if world_time.hour >= 24.0 {
        world_time.hour -= 24.0;
    }
}

/// 更新日夜光照（整合天氣）
pub fn update_lighting(
    world_time: Res<WorldTime>,
    weather: Res<WeatherState>,
    mut ambient: ResMut<AmbientLight>,
    mut sun_query: Query<&mut DirectionalLight>,
    mut street_lights: Query<(&mut PointLight, &mut StreetLight)>,
) {
    let hour = world_time.hour;
    let day_intensity = calculate_day_intensity(hour);

    // 天氣對光照的影響
    let weather_light_factor = get_weather_light_factor(&weather);

    // 更新環境光（考慮天氣）
    update_ambient_light_with_weather(&mut ambient, day_intensity, weather_light_factor);

    // 更新太陽光（考慮天氣）
    for mut sun in &mut sun_query {
        let base_illuminance = 2000.0 + 18000.0 * day_intensity;
        sun.illuminance = base_illuminance * weather_light_factor;
    }

    // 更新路燈（陰雨天提早開燈）
    let adjusted_hour = adjust_hour_for_weather(hour, &weather);
    update_street_lights(&mut street_lights, adjusted_hour);
}

/// 取得天氣對光照的影響係數
fn get_weather_light_factor(weather: &WeatherState) -> f32 {
    let current_factor = match weather.weather_type {
        WeatherType::Clear => 1.0,
        WeatherType::Cloudy => 0.6,
        WeatherType::Rainy => 0.4,
        WeatherType::Foggy => 0.5,
        WeatherType::Stormy => 0.25,    // 暴風雨極暗
        WeatherType::Sandstorm => 0.35, // 沙塵暴遮蔽陽光
    };

    if weather.is_transitioning {
        let target_factor = match weather.target_weather {
            WeatherType::Clear => 1.0,
            WeatherType::Cloudy => 0.6,
            WeatherType::Rainy => 0.4,
            WeatherType::Foggy => 0.5,
            WeatherType::Stormy => 0.25,
            WeatherType::Sandstorm => 0.35,
        };
        // 平滑過渡
        current_factor + (target_factor - current_factor) * weather.transition_progress
    } else {
        current_factor
    }
}

/// 根據天氣調整路燈開關時間
fn adjust_hour_for_weather(hour: f32, weather: &WeatherState) -> f32 {
    // 陰雨天路燈提早開啟（模擬下午變暗）
    let hour_shift = match weather.weather_type {
        WeatherType::Clear => 0.0,
        WeatherType::Cloudy => 2.0,    // 提早 2 小時
        WeatherType::Rainy => 3.0,     // 提早 3 小時
        WeatherType::Foggy => 2.5,     // 提早 2.5 小時
        WeatherType::Stormy => 4.0,    // 暴風雨更早開燈
        WeatherType::Sandstorm => 3.5, // 沙塵暴影響能見度
    };

    // 只在白天時段調整
    if (6.0..18.0).contains(&hour) {
        // 模擬天色變暗：讓時間「感覺」更晚
        hour + hour_shift
    } else {
        hour
    }
}

/// 更新環境光（考慮天氣）
fn update_ambient_light_with_weather(
    ambient: &mut AmbientLight,
    day_intensity: f32,
    weather_factor: f32,
) {
    ambient.color = Color::srgb(
        (0.1 + 0.4 * day_intensity) * weather_factor,
        (0.1 + 0.4 * day_intensity) * weather_factor,
        (0.2 + 0.35 * day_intensity) * weather_factor.powf(0.8), // 藍色稍微保留
    );
    ambient.brightness = (100.0 + 400.0 * day_intensity) * weather_factor;
}

/// 計算日光強度 (0.0 ~ 1.0)
fn calculate_day_intensity(hour: f32) -> f32 {
    if !(6.0..=18.0).contains(&hour) {
        return 0.0;
    }

    if hour < 8.0 {
        (hour - 6.0) / 2.0 // 日出漸亮
    } else if hour > 16.0 {
        (18.0 - hour) / 2.0 // 日落漸暗
    } else {
        1.0 // 正午最亮
    }
}

/// 太陽/月亮軌跡系統
/// 根據世界時間旋轉太陽（東升西落）和更新月亮位置
///
/// 太陽軌跡：
/// - 6:00 日出 (東方，+X)
/// - 12:00 正午 (頭頂，-Y 方向)
/// - 18:00 日落 (西方，-X)
pub fn sun_moon_rotation_system(
    time: Res<Time>,
    world_time: Res<WorldTime>,
    mut sun_query: Query<&mut Transform, With<Sun>>,
    mut moon_query: Query<(&mut Transform, &mut Moon), Without<Sun>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    moon_material_query: Query<&MeshMaterial3d<StandardMaterial>, With<Moon>>,
) {
    let hour = world_time.hour;
    let dt = time.delta_secs();

    // 1. 更新太陽
    for mut transform in &mut sun_query {
        *transform = Transform::from_rotation(calculate_sun_rotation(hour));
    }

    // 2. 更新月亮
    for (mut moon_transform, mut moon) in &mut moon_query {
        let (pos, elevation) = calculate_moon_position(hour);
        moon_transform.translation = pos;
        moon_transform.look_at(Vec3::ZERO, Vec3::Y);

        let night_intensity = calculate_moon_intensity(hour, elevation);
        moon.emissive_intensity = night_intensity;

        // 更新月亮材質
        if let Ok(material_handle) = moon_material_query.single() {
            if let Some(material) = materials.get_mut(&material_handle.0) {
                let base_emissive = LinearRgba::new(0.8, 0.8, 0.9, 1.0);
                material.emissive = base_emissive * night_intensity;
            }
        }

        // 更新月相
        update_moon_phase(&mut moon, dt, world_time.time_scale);
    }
}

// ============================================================================
// 日月輔助函數
// ============================================================================
/// 計算太陽旋轉 (基於時間)
fn calculate_sun_rotation(hour: f32) -> Quat {
    // 計算太陽高度角
    // 6:00 和 18:00 時在地平線，12:00 時在最高點
    let elevation = if (6.0..=18.0).contains(&hour) {
        // 白天：正弦曲線從 0 到 π
        let day_progress = (hour - 6.0) / 12.0;
        (day_progress * std::f32::consts::PI).sin() * 1.2 // 1.2 rad ≈ 69° 最大仰角
    } else {
        // 夜晚：太陽在地平線下
        -0.5 // 略低於地平線
    };

    // 方位角：東（6:00）→ 南（12:00）→ 西（18:00）
    // 只在白天計算有效方位角，夜間保持最後位置
    let azimuth = if (6.0..=18.0).contains(&hour) {
        (hour - 6.0) / 12.0 * std::f32::consts::PI
    } else if hour > 18.0 {
        std::f32::consts::PI // 日落後固定在西方
    } else {
        0.0 // 日出前固定在東方
    };

    Quat::from_euler(EulerRot::XYZ, -elevation, azimuth, 0.0)
}

/// 計算月亮位置與高度角
fn calculate_moon_position(hour: f32) -> (Vec3, f32) {
    const MOON_ORBIT_RADIUS: f32 = 500.0;
    const MOON_HEIGHT_OFFSET: f32 = 100.0;

    // 月亮與太陽相差約 12 小時（簡化模型）
    let moon_hour = (hour + 12.0) % 24.0;

    // 計算月亮在天空中的位置
    let moon_elevation = if (6.0..=18.0).contains(&moon_hour) {
        let night_progress = (moon_hour - 6.0) / 12.0;
        (night_progress * std::f32::consts::PI).sin()
    } else {
        -0.3 // 白天在地平線下但不完全消失
    };

    // 計算月亮 3D 位置
    let moon_azimuth = (moon_hour - 6.0) / 12.0 * std::f32::consts::PI;
    let moon_x = -MOON_ORBIT_RADIUS * moon_azimuth.sin();
    let moon_z = -MOON_ORBIT_RADIUS * moon_azimuth.cos();
    let moon_y = MOON_HEIGHT_OFFSET + MOON_ORBIT_RADIUS * moon_elevation * 0.5;

    (Vec3::new(moon_x, moon_y, moon_z), moon_elevation)
}

/// 計算月亮發光強度
fn calculate_moon_intensity(hour: f32, elevation: f32) -> f32 {
    if (18.0..=24.0).contains(&hour) || (0.0..=6.0).contains(&hour) {
        1.0 + 0.5 * elevation.max(0.0) // 夜間增強
    } else {
        0.2 // 白天減弱
    }
}

/// 更新月相
fn update_moon_phase(moon: &mut Moon, dt: f32, time_scale: f32) {
    // 簡單月相模擬（每 30 遊戲天循環一次）
    let phase_speed = 1.0 / (30.0 * 24.0 * 60.0);
    moon.phase = (moon.phase + phase_speed * dt * time_scale) % 1.0;
}

/// 更新路燈開關
fn update_street_lights(street_lights: &mut Query<(&mut PointLight, &mut StreetLight)>, hour: f32) {
    let should_be_on = !(6.0..=18.0).contains(&hour);

    for (mut light, mut street_light) in &mut *street_lights {
        if street_light.is_on != should_be_on {
            street_light.is_on = should_be_on;
            light.intensity = if should_be_on { 80000.0 } else { 0.0 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- calculate_day_intensity ---

    #[test]
    fn day_intensity_midnight_is_zero() {
        assert_eq!(calculate_day_intensity(0.0), 0.0);
        assert_eq!(calculate_day_intensity(3.0), 0.0);
        assert_eq!(calculate_day_intensity(5.9), 0.0);
    }

    #[test]
    fn day_intensity_noon_is_max() {
        assert_eq!(calculate_day_intensity(12.0), 1.0);
        assert_eq!(calculate_day_intensity(10.0), 1.0);
        assert_eq!(calculate_day_intensity(15.0), 1.0);
    }

    #[test]
    fn day_intensity_sunrise_ramps_up() {
        let dawn = calculate_day_intensity(7.0);
        assert!(dawn > 0.0 && dawn < 1.0);
        assert!((dawn - 0.5).abs() < f32::EPSILON);
    }

    // --- get_weather_light_factor ---

    #[test]
    fn weather_light_clear_is_full() {
        let ws = WeatherState::default(); // Clear
        assert_eq!(get_weather_light_factor(&ws), 1.0);
    }

    #[test]
    fn weather_light_stormy_is_darkest() {
        let ws = WeatherState {
            weather_type: WeatherType::Stormy,
            is_transitioning: false,
            ..Default::default()
        };
        assert!((get_weather_light_factor(&ws) - 0.25).abs() < f32::EPSILON);
    }

    // --- adjust_hour_for_weather ---

    #[test]
    fn adjust_hour_clear_no_shift() {
        let ws = WeatherState::default();
        assert_eq!(adjust_hour_for_weather(12.0, &ws), 12.0);
    }

    #[test]
    fn adjust_hour_stormy_shifts_4h_daytime() {
        let ws = WeatherState {
            weather_type: WeatherType::Stormy,
            ..Default::default()
        };
        assert!((adjust_hour_for_weather(12.0, &ws) - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn adjust_hour_nighttime_no_shift() {
        let ws = WeatherState {
            weather_type: WeatherType::Stormy,
            ..Default::default()
        };
        assert_eq!(adjust_hour_for_weather(22.0, &ws), 22.0);
    }

    // --- calculate_sun_rotation ---

    #[test]
    fn sun_rotation_changes_between_noon_and_night() {
        let noon = calculate_sun_rotation(12.0);
        let night = calculate_sun_rotation(0.0);
        // 正午和夜晚的旋轉應該不同
        let diff = noon.dot(night).abs();
        assert!(
            diff < 0.99,
            "noon and night rotations should differ, dot={diff}"
        );
    }

    // --- calculate_moon_position ---

    #[test]
    fn moon_position_midnight_is_high() {
        let (pos, elevation) = calculate_moon_position(0.0);
        // 午夜月亮應在高處
        assert!(
            pos.y > 100.0,
            "moon at midnight should be high, got {}",
            pos.y
        );
        assert!(elevation > 0.0);
    }
}
