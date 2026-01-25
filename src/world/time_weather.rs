//! 日夜循環和天氣系統

use bevy::prelude::*;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy_rapier3d::prelude::*;
use crate::core::{WorldTime, WeatherState, WeatherType};
use crate::camera::GameCamera;
use super::{StreetLight, NeonSign, BuildingWindow, Sun, Moon};

/// 窗戶更新計時器（效能優化：避免每幀更新）
#[derive(Resource)]
pub struct WindowUpdateTimer(pub Timer);

impl Default for WindowUpdateTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}

/// 更新世界時間
pub fn update_world_time(
    time: Res<Time>,
    mut world_time: ResMut<WorldTime>,
) {
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
    for mut sun in sun_query.iter_mut() {
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
        WeatherType::Stormy => 0.25,     // 暴風雨極暗
        WeatherType::Sandstorm => 0.35,  // 沙塵暴遮蔽陽光
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
        WeatherType::Cloudy => 2.0,      // 提早 2 小時
        WeatherType::Rainy => 3.0,       // 提早 3 小時
        WeatherType::Foggy => 2.5,       // 提早 2.5 小時
        WeatherType::Stormy => 4.0,      // 暴風雨更早開燈
        WeatherType::Sandstorm => 3.5,   // 沙塵暴影響能見度
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
fn update_ambient_light_with_weather(ambient: &mut AmbientLight, day_intensity: f32, weather_factor: f32) {
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
        (hour - 6.0) / 2.0  // 日出漸亮
    } else if hour > 16.0 {
        (18.0 - hour) / 2.0  // 日落漸暗
    } else {
        1.0  // 正午最亮
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

    // 太陽在 6:00 從東方升起，18:00 從西方落下

    // 更新太陽方向
    for mut transform in sun_query.iter_mut() {
        // 計算太陽高度角
        // 6:00 和 18:00 時在地平線，12:00 時在最高點
        let elevation = if (6.0..=18.0).contains(&hour) {
            // 白天：正弦曲線從 0 到 π
            let day_progress = (hour - 6.0) / 12.0;
            (day_progress * std::f32::consts::PI).sin() * 1.2  // 1.2 rad ≈ 69° 最大仰角
        } else {
            // 夜晚：太陽在地平線下
            -0.5  // 略低於地平線
        };

        // 方位角：東（6:00）→ 南（12:00）→ 西（18:00）
        // 只在白天計算有效方位角，夜間保持最後位置
        let azimuth = if (6.0..=18.0).contains(&hour) {
            (hour - 6.0) / 12.0 * std::f32::consts::PI
        } else if hour > 18.0 {
            std::f32::consts::PI  // 日落後固定在西方
        } else {
            0.0  // 日出前固定在東方
        };

        // 設定太陽方向
        // DirectionalLight 指向 -Z，所以需要旋轉使其指向地面
        *transform = Transform::from_rotation(
            Quat::from_euler(EulerRot::XYZ, -elevation, azimuth, 0.0)
        );
    }

    // 月亮位置（與太陽大致相對）
    // 月亮在天空中的軌道半徑
    const MOON_ORBIT_RADIUS: f32 = 500.0;
    const MOON_HEIGHT_OFFSET: f32 = 100.0;

    for (mut moon_transform, mut moon) in moon_query.iter_mut() {
        // 月亮與太陽相差約 12 小時（簡化模型）
        let moon_hour = (hour + 12.0) % 24.0;

        // 計算月亮在天空中的位置
        let moon_elevation = if (6.0..=18.0).contains(&moon_hour) {
            let night_progress = (moon_hour - 6.0) / 12.0;
            (night_progress * std::f32::consts::PI).sin()
        } else {
            -0.3  // 白天在地平線下但不完全消失（可選：完全隱藏）
        };

        // 計算月亮 3D 位置
        let moon_azimuth = (moon_hour - 6.0) / 12.0 * std::f32::consts::PI;
        let moon_x = -MOON_ORBIT_RADIUS * moon_azimuth.sin();
        let moon_z = -MOON_ORBIT_RADIUS * moon_azimuth.cos();
        let moon_y = MOON_HEIGHT_OFFSET + MOON_ORBIT_RADIUS * moon_elevation * 0.5;

        moon_transform.translation = Vec3::new(moon_x, moon_y, moon_z);

        // 月亮始終面向原點（玩家通常在原點附近）
        moon_transform.look_at(Vec3::ZERO, Vec3::Y);

        // 更新月亮發光強度（夜間更亮）
        let night_intensity = if (18.0..=24.0).contains(&hour) || (0.0..=6.0).contains(&hour) {
            1.0 + 0.5 * moon_elevation.max(0.0)  // 夜間增強
        } else {
            0.2  // 白天減弱
        };
        moon.emissive_intensity = night_intensity;

        // 更新月亮材質發光強度
        if let Ok(material_handle) = moon_material_query.single() {
            if let Some(material) = materials.get_mut(&material_handle.0) {
                let base_emissive = LinearRgba::new(0.8, 0.8, 0.9, 1.0);
                material.emissive = base_emissive * night_intensity;
            }
        }

        // 簡單月相模擬（每 30 遊戲天循環一次）
        // 使用 dt 確保幀率無關
        // 30 天 = 30 * 24 * 3600 秒（遊戲時間）
        // 但我們用 time_scale 調整，所以直接用 dt
        let phase_speed = 1.0 / (30.0 * 24.0 * 60.0);  // 每遊戲分鐘的相位變化
        moon.phase = (moon.phase + phase_speed * dt * world_time.time_scale) % 1.0;
    }
}

/// 更新路燈開關
fn update_street_lights(
    street_lights: &mut Query<(&mut PointLight, &mut StreetLight)>,
    hour: f32,
) {
    let should_be_on = !(6.0..=18.0).contains(&hour);

    for (mut light, mut street_light) in street_lights.iter_mut() {
        if street_light.is_on != should_be_on {
            street_light.is_on = should_be_on;
            light.intensity = if should_be_on { 80000.0 } else { 0.0 };
        }
    }
}

// === 霓虹燈輔助函數 ===

/// 計算夜晚亮度加成
#[inline]
fn get_night_boost(hour: f32) -> f32 {
    if (6.0..=18.0).contains(&hour) { 0.8 } else { 1.5 }
}

/// 計算波形閃爍值
#[inline]
fn calculate_wave(t: f32, speed: f32, phase: f32) -> f32 {
    ((t * speed + phase).sin() + 1.0) * 0.5
}

/// 計算故障燈隨機閃爍
fn calculate_broken_flicker(rng: &mut impl rand::Rng, t: f32, neon: &NeonSign) -> f32 {
    let random_flicker = if rng.random::<f32>() < 0.02 {
        rng.random::<f32>() * 0.5 // 偶爾完全熄滅
    } else {
        1.0
    };
    let wave = calculate_wave(t, neon.flicker_speed, neon.phase_offset);
    random_flicker * (1.0 - neon.flicker_amount + neon.flicker_amount * wave)
}

/// 計算霓虹燈閃爍強度
fn calculate_neon_flicker(rng: &mut impl rand::Rng, t: f32, neon: &NeonSign) -> f32 {
    if neon.is_broken {
        calculate_broken_flicker(rng, t, neon)
    } else if neon.flicker_speed > 0.0 {
        let wave = calculate_wave(t, neon.flicker_speed, neon.phase_offset);
        1.0 - neon.flicker_amount + neon.flicker_amount * wave
    } else {
        1.0
    }
}

/// 更新霓虹燈招牌閃爍效果
/// 每幀更新所有霓虹燈的發光強度
pub fn update_neon_signs(
    time: Res<Time>,
    world_time: Res<WorldTime>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    neon_query: Query<(&NeonSign, &MeshMaterial3d<StandardMaterial>)>,
) {
    let t = time.elapsed_secs();
    let night_boost = get_night_boost(world_time.hour);
    let mut rng = rand::rng();

    for (neon, material_handle) in &neon_query {
        let Some(material) = materials.get_mut(&material_handle.0) else { continue };

        let flicker = calculate_neon_flicker(&mut rng, t, neon);
        let intensity = neon.base_intensity * flicker * night_boost;
        let color = neon.color.to_linear();

        material.emissive = LinearRgba::new(
            color.red * intensity,
            color.green * intensity,
            color.blue * intensity,
            1.0,
        );
    }
}

// === 建築窗戶輔助函數 ===

/// 計算各時段窗戶點亮的基礎機率
#[inline]
fn calculate_base_lit_chance(hour: f32) -> f32 {
    match () {
        _ if (6.0..18.0).contains(&hour) => 0.1,   // 日間：10%
        _ if (18.0..20.0).contains(&hour) => 0.6,  // 傍晚：60%
        _ if (0.0..2.0).contains(&hour) || (22.0..24.0).contains(&hour) => 0.2, // 深夜：20%
        _ => 0.4, // 一般夜晚：40%
    }
}

/// 判斷窗戶是否應該點亮
#[inline]
fn should_window_be_lit(
    window: &BuildingWindow,
    base_chance: f32,
    shop_closed: bool,
    rng: &mut impl rand::Rng,
) -> bool {
    if window.is_shop && shop_closed {
        return false;
    }
    let effective_chance = base_chance * window.light_probability;
    rng.random::<f32>() < effective_chance
}

/// 設置窗戶發光材質
#[inline]
fn set_window_emissive(material: &mut StandardMaterial, window: &BuildingWindow, lit: bool) {
    material.emissive = if lit {
        let color = window.base_color.to_linear();
        LinearRgba::new(
            color.red * window.lit_emissive,
            color.green * window.lit_emissive,
            color.blue * window.lit_emissive,
            1.0,
        )
    } else {
        LinearRgba::new(0.0, 0.0, 0.0, 1.0)
    };
}

/// 更新建築窗戶燈光（根據時間變化）
/// 日間窗戶暗淡，夜間隨機點亮
/// 效能優化：每 5 秒更新一次（而非每幀）
pub fn update_building_windows(
    time: Res<Time>,
    mut timer: ResMut<WindowUpdateTimer>,
    world_time: Res<WorldTime>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut window_query: Query<(&mut BuildingWindow, &MeshMaterial3d<StandardMaterial>)>,
) {
    // 效能優化：僅在計時器觸發時更新
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let hour = world_time.hour;
    let base_lit_chance = calculate_base_lit_chance(hour);
    let shop_closed = (0.0..6.0).contains(&hour);
    let mut rng = rand::rng();

    for (mut window, material_handle) in window_query.iter_mut() {
        let should_be_lit = should_window_be_lit(&window, base_lit_chance, shop_closed, &mut rng);

        // 只在狀態改變時更新材質
        if window.is_lit != should_be_lit {
            window.is_lit = should_be_lit;
            if let Some(material) = materials.get_mut(&material_handle.0) {
                set_window_emissive(material, &window, should_be_lit);
            }
        }
    }
}

/// 生成霓虹燈招牌
/// 在指定位置生成一個發光的招牌
pub fn spawn_neon_sign(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    size: Vec3,        // (width, height, depth)
    text: &str,        // 招牌文字（目前僅用於識別）
    neon_config: NeonSign,
) {
    let color = neon_config.color;
    let intensity = neon_config.base_intensity;

    // 霓虹燈材質（發光）
    let neon_mat = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * intensity,
        ..default()
    });

    // 招牌底板（深色背景）
    let back_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.08),
        perceptual_roughness: 0.9,
        ..default()
    });

    // 生成招牌實體
    commands.spawn((
        Transform::from_translation(position),
        GlobalTransform::default(),
        Visibility::default(),
        Name::new(format!("NeonSign_{}", text)),
        // 招牌碰撞體
        Collider::cuboid(size.x / 2.0 + 0.1, size.y / 2.0 + 0.05, size.z / 2.0 + 0.05),
        RigidBody::Fixed,
    ))
    .with_children(|parent| {
        // 底板（放在後面）
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x + 0.2, size.y + 0.1, size.z))),
            MeshMaterial3d(back_mat),
            Transform::from_xyz(0.0, 0.0, -0.05),
            GlobalTransform::default(),
        ));

        // 發光文字區（放在前面，與底板保持足夠距離）
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, 0.05))),
            MeshMaterial3d(neon_mat),
            Transform::from_xyz(0.0, 0.0, size.z / 2.0 + 0.05),
            GlobalTransform::default(),
            neon_config,
        ));

        // 招牌光源（照亮周圍）
        parent.spawn((
            PointLight {
                color,
                intensity: 50000.0 * intensity / 8.0,
                range: 15.0,
                radius: 1.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -1.0),
            GlobalTransform::default(),
        ));
    });
}

// === 天氣系統 ===

/// 雨滴組件
#[derive(Component)]
pub struct RainDrop {
    pub velocity: Vec3,
    pub lifetime: f32,
}

/// 天氣過渡更新
pub fn update_weather_transition(
    time: Res<Time>,
    mut weather: ResMut<WeatherState>,
) {
    weather.update_transition(time.delta_secs());
}

/// 天氣切換輸入（F1 鍵）
pub fn weather_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut weather: ResMut<WeatherState>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        let next_weather = weather.weather_type.next();
        weather.start_transition(next_weather);
        info!("🌤️ 天氣切換中: {} → {}", weather.weather_type.name(), next_weather.name());
    }
}

/// 根據時間和天氣更新天空顏色
pub fn update_sky_color(
    world_time: Res<WorldTime>,
    weather: Res<WeatherState>,
    mut clear_color: ResMut<ClearColor>,
) {
    let hour = world_time.hour;

    // 基礎天空顏色根據時間（晴天）
    let base_sky_color = calculate_sky_color_by_time(hour);

    // 根據天氣調整天空顏色
    let final_color = apply_weather_to_sky(base_sky_color, &weather);

    // 平滑過渡到新顏色
    let current_linear = clear_color.0.to_linear();
    let target_linear = final_color.to_linear();
    let lerped = LinearRgba::new(
        current_linear.red + (target_linear.red - current_linear.red) * 0.02,
        current_linear.green + (target_linear.green - current_linear.green) * 0.02,
        current_linear.blue + (target_linear.blue - current_linear.blue) * 0.02,
        1.0,
    );
    clear_color.0 = Color::from(lerped);
}

/// 根據時間計算基礎天空顏色
fn calculate_sky_color_by_time(hour: f32) -> Color {
    match () {
        // 深夜 (0-5)：深藍色夜空
        _ if (0.0..5.0).contains(&hour) => Color::srgb(0.02, 0.02, 0.08),
        // 日出 (5-7)：橙紅漸變
        _ if (5.0..7.0).contains(&hour) => {
            let t = (hour - 5.0) / 2.0;
            Color::srgb(
                0.02 + 0.5 * t,
                0.02 + 0.2 * t,
                0.08 + 0.2 * t,
            )
        }
        // 清晨 (7-9)：淡藍色
        _ if (7.0..9.0).contains(&hour) => {
            let t = (hour - 7.0) / 2.0;
            Color::srgb(
                0.52 - 0.17 * t,
                0.22 + 0.38 * t,
                0.28 + 0.52 * t,
            )
        }
        // 白天 (9-16)：天藍色
        _ if (9.0..16.0).contains(&hour) => Color::srgb(0.35, 0.60, 0.80),
        // 黃昏 (16-18)：金橙色
        _ if (16.0..18.0).contains(&hour) => {
            let t = (hour - 16.0) / 2.0;
            Color::srgb(
                0.35 + 0.45 * t,
                0.60 - 0.25 * t,
                0.80 - 0.45 * t,
            )
        }
        // 傍晚 (18-20)：深紫紅色
        _ if (18.0..20.0).contains(&hour) => {
            let t = (hour - 18.0) / 2.0;
            Color::srgb(
                0.80 - 0.55 * t,
                0.35 - 0.25 * t,
                0.35 - 0.15 * t,
            )
        }
        // 夜晚 (20-24)：逐漸變深
        _ => {
            let t = (hour - 20.0) / 4.0;
            Color::srgb(
                0.25 - 0.23 * t,
                0.10 - 0.08 * t,
                0.20 - 0.12 * t,
            )
        }
    }
}

/// 根據天氣類型調整天空顏色
fn apply_weather_to_sky(base_color: Color, weather: &WeatherState) -> Color {
    let base_linear = base_color.to_linear();

    // 計算當前天氣的影響程度（考慮過渡）
    let current_factor = if weather.is_transitioning {
        1.0 - weather.transition_progress
    } else {
        1.0
    };
    let target_factor = if weather.is_transitioning {
        weather.transition_progress
    } else {
        0.0
    };

    // 計算當前和目標天氣的顏色修正
    let current_modifier = get_weather_color_modifier(weather.weather_type);
    let target_modifier = get_weather_color_modifier(weather.target_weather);

    // 混合顏色修正
    let modifier = (
        current_modifier.0 * current_factor + target_modifier.0 * target_factor,
        current_modifier.1 * current_factor + target_modifier.1 * target_factor,
        current_modifier.2 * current_factor + target_modifier.2 * target_factor,
    );

    Color::from(LinearRgba::new(
        base_linear.red * modifier.0,
        base_linear.green * modifier.1,
        base_linear.blue * modifier.2,
        1.0,
    ))
}

/// 取得天氣類型的顏色修正係數
fn get_weather_color_modifier(weather_type: WeatherType) -> (f32, f32, f32) {
    match weather_type {
        WeatherType::Clear => (1.0, 1.0, 1.0),       // 晴天：無修正
        WeatherType::Cloudy => (0.7, 0.7, 0.75),     // 陰天：整體變灰
        WeatherType::Rainy => (0.5, 0.5, 0.6),       // 雨天：更灰暗
        WeatherType::Foggy => (0.8, 0.8, 0.85),      // 霧天：淡白灰
        WeatherType::Stormy => (0.35, 0.35, 0.45),   // 暴風雨：深灰藍
        WeatherType::Sandstorm => (0.7, 0.55, 0.4),  // 沙塵暴：黃褐色調
    }
}

/// 更新霧效果
pub fn update_fog_effect(
    weather: Res<WeatherState>,
    mut camera_query: Query<&mut DistanceFog, With<GameCamera>>,
) {
    let Ok(mut fog) = camera_query.single_mut() else { return };

    // 計算目標霧參數
    let (target_density, target_color) = match weather.weather_type {
        WeatherType::Clear => (0.0, Color::srgba(0.5, 0.5, 0.6, 0.0)),
        WeatherType::Cloudy => (0.003, Color::srgba(0.6, 0.6, 0.65, 0.5)),
        WeatherType::Rainy => (0.008, Color::srgba(0.4, 0.42, 0.5, 0.7)),
        WeatherType::Foggy => (0.025, Color::srgba(0.75, 0.75, 0.8, 0.9)),
        WeatherType::Stormy => (0.015, Color::srgba(0.3, 0.32, 0.4, 0.85)),   // 暴風雨：深藍灰霧
        WeatherType::Sandstorm => (0.04, Color::srgba(0.75, 0.6, 0.4, 0.95)), // 沙塵暴：濃密黃沙
    };

    // 考慮過渡
    let transition = if weather.is_transitioning {
        weather.transition_progress
    } else {
        1.0
    };

    // 平滑過渡霧密度
    if let FogFalloff::Exponential { density } = &mut fog.falloff {
        let current_density = *density;
        *density = current_density + (target_density - current_density) * 0.02 * transition;
    }

    // 平滑過渡霧顏色
    let current_linear = fog.color.to_linear();
    let target_linear = target_color.to_linear();
    fog.color = Color::from(LinearRgba::new(
        current_linear.red + (target_linear.red - current_linear.red) * 0.02 * transition,
        current_linear.green + (target_linear.green - current_linear.green) * 0.02 * transition,
        current_linear.blue + (target_linear.blue - current_linear.blue) * 0.02 * transition,
        current_linear.alpha + (target_linear.alpha - current_linear.alpha) * 0.02 * transition,
    ));
}

/// 雨滴標記組件（用於識別雨滴父容器）
#[derive(Component)]
pub struct RainSystem;

/// 生成雨滴系統
pub fn spawn_rain_drops(
    mut commands: Commands,
    weather: Res<WeatherState>,
    rain_query: Query<Entity, With<RainSystem>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let should_rain = weather.weather_type.has_rain()
        || (weather.is_transitioning && weather.target_weather.has_rain());

    // 清理邏輯移至 cleanup_rain 系統，避免重複 despawn
    if !should_rain {
        return;
    }

    // 如果已經有雨滴系統，不重複生成
    if !rain_query.is_empty() {
        return;
    }

    // 生成雨滴系統（暴風雨時更大的雨滴）
    let rain_intensity = if matches!(weather.weather_type, WeatherType::Stormy) { "⛈️ 暴風雨" } else { "🌧️" };
    info!("{} 開始下雨...", rain_intensity);

    // 雨滴材質（半透明藍白色）
    let rain_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.8, 0.95, 0.4),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // 雨滴網格（細長圓柱）
    let rain_mesh = meshes.add(Cylinder::new(0.02, 0.5));

    // 生成 300 個雨滴
    commands.spawn((
        RainSystem,
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        Name::new("RainSystem"),
    )).with_children(|parent| {
        let mut rng = rand::rng();
        use rand::Rng;
        for _ in 0..300 {
            let x = rng.random::<f32>() * 200.0 - 100.0;
            let y = rng.random::<f32>() * 50.0 + 10.0;
            let z = rng.random::<f32>() * 200.0 - 100.0;

            parent.spawn((
                Mesh3d(rain_mesh.clone()),
                MeshMaterial3d(rain_mat.clone()),
                Transform::from_xyz(x, y, z)
                    .with_rotation(Quat::from_rotation_x(-0.1)), // 稍微傾斜
                GlobalTransform::default(),
                RainDrop {
                    velocity: Vec3::new(
                        rng.random::<f32>() * 2.0 - 1.0,  // 風的影響
                        -20.0 - rng.random::<f32>() * 10.0,  // 下落速度
                        rng.random::<f32>() * 2.0 - 1.0,
                    ),
                    lifetime: rng.random::<f32>() * 3.0,
                },
            ));
        }
    });
}

/// 更新雨滴位置
pub fn update_rain_drops(
    time: Res<Time>,
    weather: Res<WeatherState>,
    mut rain_query: Query<(&mut Transform, &mut RainDrop)>,
) {
    if !weather.weather_type.has_rain() {
        return;
    }

    let dt = time.delta_secs();
    let mut rng = rand::rng();
    use rand::Rng;

    for (mut transform, mut drop) in rain_query.iter_mut() {
        // 更新位置
        transform.translation += drop.velocity * dt;

        // 重置超出範圍的雨滴
        if transform.translation.y < 0.0 {
            transform.translation.x = rng.random::<f32>() * 200.0 - 100.0;
            transform.translation.y = 50.0 + rng.random::<f32>() * 10.0;
            transform.translation.z = rng.random::<f32>() * 200.0 - 100.0;
            drop.lifetime = rng.random::<f32>() * 3.0;
        }
    }
}

/// 清理雨滴系統
pub fn cleanup_rain(
    mut commands: Commands,
    weather: Res<WeatherState>,
    rain_system_query: Query<Entity, With<RainSystem>>,
) {
    // 只有在天氣不是雨天且不在過渡到雨天時才清理
    let should_rain = weather.weather_type.has_rain()
        || (weather.is_transitioning && weather.target_weather.has_rain());

    if !should_rain {
        for entity in rain_system_query.iter() {
            // Bevy 0.17: get_entity 返回 Option，安全處理已不存在的實體
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                info!("🌤️ 雨停了");
            }
        }
    }
}

// ============================================================================
// GTA 5 風格天氣增強：雨水積水 + 雷暴閃電
// ============================================================================

/// 雨水積水組件
#[derive(Component)]
pub struct RainPuddle {
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間（雨停後多久消失）
    pub max_lifetime: f32,
    /// 水坑大小
    pub size: f32,
}

impl Default for RainPuddle {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 30.0,  // 雨停後 30 秒消失
            size: 2.0,
        }
    }
}

/// 積水系統標記
#[derive(Component)]
pub struct PuddleSystem;

/// 閃電狀態資源
#[derive(Resource)]
pub struct LightningState {
    /// 下次閃電時間
    pub next_flash_time: f32,
    /// 閃電持續時間
    pub flash_duration: f32,
    /// 當前閃電進度 (0.0 = 無閃電, 1.0 = 最亮)
    pub flash_intensity: f32,
    /// 是否正在閃電中
    pub is_flashing: bool,
    /// 閃電間隔範圍（秒）
    pub min_interval: f32,
    pub max_interval: f32,
}

impl Default for LightningState {
    fn default() -> Self {
        Self {
            next_flash_time: 5.0,  // 5 秒後第一次閃電
            flash_duration: 0.15,  // 閃電持續 0.15 秒
            flash_intensity: 0.0,
            is_flashing: false,
            min_interval: 8.0,   // 最少 8 秒一次
            max_interval: 20.0,  // 最多 20 秒一次
        }
    }
}

/// 生成雨水積水系統
pub fn spawn_rain_puddles(
    mut commands: Commands,
    weather: Res<WeatherState>,
    puddle_query: Query<Entity, With<PuddleSystem>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let should_have_puddles = weather.weather_type.has_rain()
        || (weather.is_transitioning && weather.target_weather.has_rain());

    // 如果不是雨天，不生成水坑
    if !should_have_puddles {
        return;
    }

    // 如果已經有水坑系統，不重複生成
    if !puddle_query.is_empty() {
        return;
    }

    info!("💧 生成雨水積水...");

    // 水坑材質（半透明反射）
    let puddle_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.25, 0.35, 0.6),  // 深藍灰色
        alpha_mode: AlphaMode::Blend,
        metallic: 0.9,  // 高金屬度模擬反射
        perceptual_roughness: 0.1,  // 光滑表面
        reflectance: 0.8,  // 高反射率
        ..default()
    });

    // 水坑網格（扁平圓柱）
    let puddle_mesh = meshes.add(Cylinder::new(1.0, 0.02));

    // 生成水坑系統
    commands.spawn((
        PuddleSystem,
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        Name::new("PuddleSystem"),
    )).with_children(|parent| {
        let mut rng = rand::rng();
        use rand::Rng;

        // 在徒步區和道路上生成水坑
        // 徒步區: X ∈ [-30, 30], Z ∈ [-50, 20]
        // 道路: 中華路 X=75, 西寧南路 X=-50
        let puddle_positions = [
            // 徒步區水坑
            Vec3::new(5.0, 0.01, -10.0),
            Vec3::new(-8.0, 0.01, 5.0),
            Vec3::new(15.0, 0.01, -25.0),
            Vec3::new(-12.0, 0.01, 10.0),
            Vec3::new(0.0, 0.01, -35.0),
            Vec3::new(20.0, 0.01, 0.0),
            // 道路水坑
            Vec3::new(70.0, 0.01, 20.0),
            Vec3::new(80.0, 0.01, -30.0),
            Vec3::new(-48.0, 0.01, 15.0),
            Vec3::new(-52.0, 0.01, -20.0),
        ];

        for base_pos in puddle_positions {
            // 隨機偏移
            let offset = Vec3::new(
                rng.random::<f32>() * 4.0 - 2.0,
                0.0,
                rng.random::<f32>() * 4.0 - 2.0,
            );
            let pos = base_pos + offset;

            // 隨機大小
            let size = 1.5 + rng.random::<f32>() * 2.0;
            let scale = Vec3::new(size, 1.0, size * (0.7 + rng.random::<f32>() * 0.6));

            parent.spawn((
                Mesh3d(puddle_mesh.clone()),
                MeshMaterial3d(puddle_mat.clone()),
                Transform::from_translation(pos).with_scale(scale),
                GlobalTransform::default(),
                RainPuddle {
                    size,
                    ..Default::default()
                },
            ));
        }
    });
}

/// 處理水坑乾涸邏輯，回傳是否應該移除
fn handle_puddle_drying(
    puddle: &mut RainPuddle,
    transform: &mut Transform,
    dt: f32,
) -> bool {
    puddle.lifetime += dt;

    // 根據生命時間縮小水坑
    let half_lifetime = puddle.max_lifetime * 0.5;
    if puddle.lifetime > half_lifetime {
        let shrink_progress = (puddle.lifetime - half_lifetime) / half_lifetime;
        let scale_factor = (1.0 - shrink_progress).max(0.1);
        transform.scale.x = puddle.size * scale_factor;
        transform.scale.z = puddle.size * scale_factor * 0.8;
    }

    // 回傳是否應該消失
    puddle.lifetime >= puddle.max_lifetime
}

/// 清理水坑父實體
fn cleanup_puddle_system(
    commands: &mut Commands,
    puddle_system_query: &Query<Entity, With<PuddleSystem>>,
) {
    for entity in puddle_system_query.iter() {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
            info!("💧 水坑已乾涸");
        }
    }
}

/// 更新雨水積水（雨停後漸漸消失）
pub fn update_rain_puddles(
    mut commands: Commands,
    time: Res<Time>,
    weather: Res<WeatherState>,
    mut puddle_query: Query<(Entity, &mut RainPuddle, &mut Transform)>,
    puddle_system_query: Query<Entity, With<PuddleSystem>>,
) {
    let is_raining = weather.weather_type.has_rain();
    let dt = time.delta_secs();

    for (entity, mut puddle, mut transform) in puddle_query.iter_mut() {
        if is_raining {
            puddle.lifetime = 0.0;
        } else if handle_puddle_drying(&mut puddle, &mut transform, dt) {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
        }
    }

    // 如果所有水坑都消失了，清理父實體
    if puddle_query.is_empty() && !is_raining {
        cleanup_puddle_system(&mut commands, &puddle_system_query);
    }
}

/// 閃電更新系統
pub fn update_lightning(
    time: Res<Time>,
    weather: Res<WeatherState>,
    mut lightning: ResMut<LightningState>,
    mut ambient: ResMut<AmbientLight>,
) {
    // 只在雨天/暴風雨有閃電
    if !weather.weather_type.has_rain() {
        lightning.is_flashing = false;
        lightning.flash_intensity = 0.0;
        return;
    }

    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    // 檢查是否該閃電了
    if !lightning.is_flashing && current_time >= lightning.next_flash_time {
        // 開始閃電
        lightning.is_flashing = true;
        lightning.flash_intensity = 1.0;

        // 計算下次閃電時間
        let mut rng = rand::rng();
        use rand::Rng;
        let interval = lightning.min_interval + rng.random::<f32>() * (lightning.max_interval - lightning.min_interval);
        lightning.next_flash_time = current_time + interval;

        info!("⚡ 閃電！");
    }

    // 更新閃電強度
    if lightning.is_flashing {
        // 快速衰減
        lightning.flash_intensity -= dt / lightning.flash_duration;

        if lightning.flash_intensity <= 0.0 {
            lightning.flash_intensity = 0.0;
            lightning.is_flashing = false;
        }

        // 應用到環境光（閃電時瞬間變亮）
        let flash_boost = lightning.flash_intensity * 2.0;
        ambient.brightness += flash_boost * 500.0;
    }
}

/// 閃電視覺效果系統
/// 在閃電時瞬間增亮整個場景
pub fn lightning_visual_effect(
    lightning: Res<LightningState>,
    mut sun_query: Query<&mut DirectionalLight>,
) {
    if !lightning.is_flashing {
        return;
    }

    // 閃電時太陽光瞬間增強
    for mut sun in sun_query.iter_mut() {
        sun.illuminance += lightning.flash_intensity * 30000.0;
    }
}
