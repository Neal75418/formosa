//! 天空盒與距離霧
//!
//! 天空是 CPU 畫的漸層 cubemap（天頂 → 地平線，日出日落再加一圈暖色光暈），掛在主攝影機的 `Skybox` 上。
//! 霧色永遠等於地平線色：遠處的建築和地面會剛好融進天空，看不到地面的盡頭。

use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::color::ColorToPacked;
use bevy::core_pipeline::Skybox;
use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

use super::lighting::calculate_sun_rotation;
use crate::camera::GameCamera;
use crate::core::{WeatherState, WeatherType, WorldTime};

/// 天空 cubemap 每一面的邊長（像素）：只有大範圍漸層和光暈，32 加上線性過濾就夠平滑
pub const SKY_FACE_SIZE: u32 = 32;

/// 晴天的霧能見度（公尺）：晴天也有一層薄霧，遠處融進地平線
pub const CLEAR_FOG_VISIBILITY: f32 = 300.0;

/// 地平線色帶的高度（方向向量的 y）：比最靠近地平線那排像素（y ≈ 1/32）再高一點，
/// 地平線上下兩排都是純地平線色，線性過濾後地平線那一圈剛好等於霧色
const HORIZON_BAND: f32 = 0.04;

/// 光暈集中程度：越大越集中在太陽附近
const GLOW_EXPONENT: f32 = 6.0;

/// 光暈從地平線色帶頂端往上淡入的高度
const GLOW_FADE_HEIGHT: f32 = 0.1;

/// 光暈顏色（sRGB）與最大強度（線性，疊加在天空色上）
const GLOW_COLOR: [f32; 3] = [1.0, 0.55, 0.25];
const GLOW_INTENSITY: f32 = 0.6;

/// 光暈強度關鍵影格：(時刻, 強度)，日出日落前後最強，表外為 0
const GLOW_KEYFRAMES: [(f32, f32); 6] = [
    (5.5, 0.0),
    (6.25, 1.0),
    (7.5, 0.0),
    (16.25, 0.0),
    (17.75, 1.0),
    (18.75, 0.0),
];

/// 深夜天空：深藍黑天頂，地平線是城市燈光映出的暗橘紫（西門町看不到星星）
const NIGHT_ZENITH: [f32; 3] = [0.03, 0.04, 0.09];
const NIGHT_HORIZON: [f32; 3] = [0.20, 0.13, 0.14];

/// 天空關鍵影格：(時刻, 天頂 sRGB, 地平線 sRGB)，影格之間在線性空間內插，最後一格接回 24 點的第一格
const SKY_KEYFRAMES: [(f32, [f32; 3], [f32; 3]); 8] = [
    (0.0, NIGHT_ZENITH, NIGHT_HORIZON),
    (5.0, NIGHT_ZENITH, NIGHT_HORIZON),
    (6.0, [0.16, 0.18, 0.32], [0.66, 0.42, 0.32]), // 日出
    (8.0, [0.32, 0.46, 0.66], [0.66, 0.70, 0.74]), // 早晨：霧藍天頂、灰白地平線
    (16.0, [0.32, 0.46, 0.66], [0.68, 0.70, 0.72]), // 午後
    (17.5, [0.22, 0.24, 0.42], [0.86, 0.50, 0.30]), // 夕陽：深藍紫天頂、橘紅地平線
    (18.5, [0.08, 0.08, 0.20], [0.46, 0.24, 0.24]), // 暮色
    (20.0, NIGHT_ZENITH, NIGHT_HORIZON),           // 入夜
];

/// 天氣切換時，天空每前進 1/50 的進度重畫一次
const SKY_BLEND_STEPS: f32 = 50.0;

/// 天頂與地平線顏色（線性 RGB）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyColors {
    pub zenith: Vec3,
    pub horizon: Vec3,
}

/// 某個時刻、某種天氣下畫天空所需的全部資料（線性 RGB）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkySnapshot {
    pub zenith: Vec3,
    pub horizon: Vec3,
    /// 日出日落光暈（顏色 × 強度），平常是 0
    pub glow: Vec3,
    /// 光暈中心的方向（世界座標）
    pub glow_direction: Vec3,
}

fn srgb_to_linear(c: [f32; 3]) -> Vec3 {
    let linear = Color::srgb(c[0], c[1], c[2]).to_linear();
    Vec3::new(linear.red, linear.green, linear.blue)
}

/// 晴天下某個時刻的天頂與地平線顏色
pub fn sky_colors_at(hour: f32) -> SkyColors {
    let hour = hour.rem_euclid(24.0);
    let last = SKY_KEYFRAMES.len() - 1;
    // 第一格是 0 點，所以找到的下一格至少是第 1 格；找不到代表在最後一格之後，接回 24 點
    let (from, to, to_hour) = match SKY_KEYFRAMES.iter().position(|(h, ..)| *h > hour) {
        Some(next) => (
            SKY_KEYFRAMES[next - 1],
            SKY_KEYFRAMES[next],
            SKY_KEYFRAMES[next].0,
        ),
        None => (SKY_KEYFRAMES[last], SKY_KEYFRAMES[0], 24.0),
    };
    let t = (hour - from.0) / (to_hour - from.0);
    SkyColors {
        zenith: srgb_to_linear(from.1).lerp(srgb_to_linear(to.1), t),
        horizon: srgb_to_linear(from.2).lerp(srgb_to_linear(to.2), t),
    }
}

/// 天氣對天空的調色（線性 RGB 乘數）
fn weather_tint(weather_type: WeatherType) -> Vec3 {
    match weather_type {
        WeatherType::Clear => Vec3::ONE,                     // 晴天：無修正
        WeatherType::Cloudy => Vec3::new(0.7, 0.7, 0.75),    // 陰天：整體變灰
        WeatherType::Rainy => Vec3::new(0.5, 0.5, 0.6),      // 雨天：更灰暗
        WeatherType::Foggy => Vec3::new(0.8, 0.8, 0.85),     // 霧天：淡白灰
        WeatherType::Stormy => Vec3::new(0.35, 0.35, 0.45),  // 暴風雨：深灰藍
        WeatherType::Sandstorm => Vec3::new(0.7, 0.55, 0.4), // 沙塵暴：黃褐色調
    }
}

/// 天氣對天空的調色，切換中依進度混合
pub fn weather_sky_tint(weather: &WeatherState) -> Vec3 {
    let current = weather_tint(weather.weather_type);
    if weather.is_transitioning {
        current.lerp(
            weather_tint(weather.target_weather),
            weather.transition_progress,
        )
    } else {
        current
    }
}

/// 日出日落光暈（線性顏色 × 強度），其他時段為 0
pub fn sun_glow_at(hour: f32) -> Vec3 {
    let hour = hour.rem_euclid(24.0);
    let strength = GLOW_KEYFRAMES
        .windows(2)
        .find(|pair| (pair[0].0..pair[1].0).contains(&hour))
        .map_or(0.0, |pair| {
            let t = (hour - pair[0].0) / (pair[1].0 - pair[0].0);
            pair[0].1 + (pair[1].1 - pair[0].1) * t
        });
    srgb_to_linear(GLOW_COLOR) * GLOW_INTENSITY * strength
}

/// 光暈中心的方向：白天跟著太陽走；入夜後停在日落方向的地平線，不會跟著夜間光源跳到半空
pub fn glow_direction(hour: f32) -> Vec3 {
    calculate_sun_rotation(hour.clamp(6.0, 18.0)) * Vec3::Z
}

/// 某個時刻、某種天氣下的天空（光暈也跟著天氣變暗）
pub fn sky_snapshot(hour: f32, weather: &WeatherState) -> SkySnapshot {
    let colors = sky_colors_at(hour);
    let tint = weather_sky_tint(weather);
    SkySnapshot {
        zenith: colors.zenith * tint,
        horizon: colors.horizon * tint,
        glow: sun_glow_at(hour) * tint,
        glow_direction: glow_direction(hour),
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 天空某個方向（世界座標、單位向量）的顏色
///
/// 地平線色帶以下（含地平線）一律是地平線色，光暈也只從色帶上方開始：遠景的霧色才接得上天空
pub fn sky_pixel(direction: Vec3, snapshot: &SkySnapshot) -> Vec3 {
    let height = ((direction.y - HORIZON_BAND) / (1.0 - HORIZON_BAND)).max(0.0);
    // 1 - (1 - h)³：靠近地平線變化較快，俯視鏡頭看得到的那一小條天空也有漸層
    let t = 1.0 - (1.0 - height).powi(3);
    let base = snapshot.horizon.lerp(snapshot.zenith, t);
    let toward_sun = direction
        .dot(snapshot.glow_direction)
        .max(0.0)
        .powf(GLOW_EXPONENT);
    let lift = smoothstep(HORIZON_BAND, HORIZON_BAND + GLOW_FADE_HEIGHT, direction.y);
    base + snapshot.glow * toward_sun * lift
}

/// cubemap 第 `face` 面像素 (x, y) 指向的世界方向
///
/// 面的順序與 (u, v) 的取向照 wgpu／Vulkan 的 cubemap 慣例；
/// Bevy 的天空盒取樣時把 z 反過來（cubemap 是左手座標），所以這裡把 z 再反回世界座標
pub fn cube_texel_world_direction(face: u32, x: u32, y: u32) -> Vec3 {
    let size = SKY_FACE_SIZE as f32;
    let u = (x as f32 + 0.5) / size * 2.0 - 1.0;
    let v = (y as f32 + 0.5) / size * 2.0 - 1.0;
    let cube = match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    Vec3::new(cube.x, cube.y, -cube.z).normalize()
}

/// 畫出整張天空 cubemap（6 面，RGBA8 sRGB）
pub fn sky_cubemap_pixels(snapshot: &SkySnapshot) -> Vec<u8> {
    let size = SKY_FACE_SIZE;
    let mut data = Vec::with_capacity((6 * size * size * 4) as usize);
    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let c = sky_pixel(cube_texel_world_direction(face, x, y), snapshot);
                data.extend_from_slice(&Srgba::from(LinearRgba::rgb(c.x, c.y, c.z)).to_u8_array());
            }
        }
    }
    data
}

/// 包成 Bevy 的 cube 貼圖
fn sky_image(data: Vec<u8>) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: SKY_FACE_SIZE,
            height: SKY_FACE_SIZE,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        // 要保留 MAIN_WORLD：只標 RENDER_WORLD 的話，Bevy 送上 GPU 後會把它從 Assets 移除，之後就改不到了
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// 各天氣的霧能見度（公尺）
fn weather_visibility(weather_type: WeatherType) -> f32 {
    match weather_type {
        WeatherType::Clear => CLEAR_FOG_VISIBILITY,
        WeatherType::Cloudy => 240.0,
        WeatherType::Rainy => 160.0,
        WeatherType::Stormy => 130.0,
        WeatherType::Foggy => 80.0,
        WeatherType::Sandstorm => 55.0,
    }
}

/// 霧能見度（公尺），天氣切換中依進度混合
pub fn fog_visibility(weather: &WeatherState) -> f32 {
    let current = weather_visibility(weather.weather_type);
    if weather.is_transitioning {
        let target = weather_visibility(weather.target_weather);
        current + (target - current) * weather.transition_progress
    } else {
        current
    }
}

/// 能見度降到這裡（雨天）就看不到月亮
const MOON_HIDDEN_VISIBILITY: f32 = 160.0;

/// 月亮透出的程度（0 = 看不到、1 = 清楚）：晴天清楚、陰天半透明、雨霧沙塵看不到，天氣切換中漸變
///
/// 月亮掛在 ~500 m 外，吃距離霧的話晴天的薄霧就會把它整個蓋掉，所以月亮不開霧、改由這裡依天氣決定
pub fn moon_clearness(weather: &WeatherState) -> f32 {
    ((fog_visibility(weather) - MOON_HIDDEN_VISIBILITY)
        / (CLEAR_FOG_VISIBILITY - MOON_HIDDEN_VISIBILITY))
        .clamp(0.0, 1.0)
}

/// 天空貼圖畫的是哪一分鐘、哪種天氣：不同才重畫
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SkyKey {
    minute: u32,
    weather: WeatherType,
    target: WeatherType,
    blend_step: u32,
}

fn sky_key(hour: f32, weather: &WeatherState) -> SkyKey {
    let (target, blend_step) = if weather.is_transitioning {
        (
            weather.target_weather,
            (weather.transition_progress * SKY_BLEND_STEPS) as u32,
        )
    } else {
        (weather.weather_type, 0)
    };
    SkyKey {
        minute: (hour.rem_euclid(24.0) * 60.0) as u32,
        weather: weather.weather_type,
        target,
        blend_step,
    }
}

/// 天空 cubemap 與它目前畫的內容
#[derive(Resource)]
pub struct SkyCubemap {
    pub image: Handle<Image>,
    drawn: Option<SkyKey>,
    /// 目前貼圖的地平線色：霧色直接用它，天空和霧永遠出自同一份資料
    horizon: Vec3,
}

impl FromWorld for SkyCubemap {
    fn from_world(world: &mut World) -> Self {
        let snapshot = sky_snapshot(WorldTime::default().hour, &WeatherState::default());
        let image = world
            .resource_mut::<Assets<Image>>()
            .add(sky_image(sky_cubemap_pixels(&snapshot)));
        Self {
            image,
            drawn: None,
            horizon: snapshot.horizon,
        }
    }
}

/// 遊戲攝影機一生成就掛上天空盒
///
/// 亮度抵銷曝光：天空貼圖的值原樣上螢幕，跟（不乘曝光的）霧色同一個尺度
pub fn attach_skybox(
    mut commands: Commands,
    sky: Res<SkyCubemap>,
    cameras: Query<(Entity, Option<&Exposure>), (With<GameCamera>, Without<Skybox>)>,
) {
    for (entity, exposure) in &cameras {
        let exposure = exposure.copied().unwrap_or_default().exposure();
        commands.entity(entity).insert(Skybox {
            image: sky.image.clone(),
            brightness: exposure.recip(),
            ..default()
        });
    }
}

/// 依時間與天氣更新天空貼圖與霧
///
/// 天空每換一分鐘（或天氣切換前進一段）才重畫；霧色用貼圖的地平線色，霧的濃度每幀跟著天氣
pub fn update_sky_and_fog(
    world_time: Res<WorldTime>,
    weather: Res<WeatherState>,
    mut sky: ResMut<SkyCubemap>,
    mut images: ResMut<Assets<Image>>,
    mut fogs: Query<&mut DistanceFog, With<GameCamera>>,
) {
    let key = sky_key(world_time.hour, &weather);
    if sky.drawn != Some(key) {
        let snapshot = sky_snapshot(world_time.hour, &weather);
        if let Some(image) = images.get_mut(&sky.image) {
            image.data = Some(sky_cubemap_pixels(&snapshot));
            sky.horizon = snapshot.horizon;
            sky.drawn = Some(key);
        } else {
            warn_once!("天空貼圖不在 Assets 裡，天空與霧色停止更新");
        }
    }

    let Ok(mut fog) = fogs.single_mut() else {
        return;
    };
    fog.color = Color::LinearRgba(LinearRgba::rgb(sky.horizon.x, sky.horizon.y, sky.horizon.z));
    fog.falloff = FogFalloff::from_visibility_squared(fog_visibility(&weather));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::color::ColorToPacked;

    fn luminance(c: Vec3) -> f32 {
        c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
    }

    fn weather(kind: WeatherType) -> WeatherState {
        WeatherState {
            weather_type: kind,
            target_weather: kind,
            ..default()
        }
    }

    fn transitioning(from: WeatherType, to: WeatherType, progress: f32) -> WeatherState {
        WeatherState {
            weather_type: from,
            target_weather: to,
            is_transitioning: true,
            transition_progress: progress,
            ..default()
        }
    }

    fn srgb_u8(c: Vec3) -> [u8; 4] {
        Srgba::from(LinearRgba::rgb(c.x, c.y, c.z)).to_u8_array()
    }

    /// cubemap 某一面某個像素的 RGBA
    fn texel(pixels: &[u8], face: u32, x: u32, y: u32) -> [u8; 4] {
        let i = (((face * SKY_FACE_SIZE + y) * SKY_FACE_SIZE + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    }

    // --- 天空顏色 ---

    #[test]
    fn sky_colors_continuous_through_the_day() {
        // 每 0.01 小時取樣（含 23.99 → 0:00 的接縫），相鄰兩點的顏色不能跳
        let mut previous = sky_colors_at(0.0);
        for i in 1..=2400 {
            let current = sky_colors_at(i as f32 * 0.01);
            let jump = (current.zenith - previous.zenith)
                .abs()
                .max((current.horizon - previous.horizon).abs())
                .max_element();
            assert!(jump < 0.01, "{:.2}h 顏色跳了 {jump}", i as f32 * 0.01);
            previous = current;
        }
    }

    #[test]
    fn night_sky_darker_than_noon() {
        let noon = sky_colors_at(12.0);
        let night = sky_colors_at(23.0);
        assert!(luminance(night.zenith) < luminance(noon.zenith) * 0.2);
        assert!(luminance(night.horizon) < luminance(noon.horizon) * 0.2);
    }

    #[test]
    fn sunset_horizon_warmer_than_zenith() {
        let sunset = sky_colors_at(17.5);
        assert!(sunset.horizon.x > sunset.horizon.z, "夕陽地平線要偏紅橘");
        assert!(sunset.zenith.z > sunset.zenith.x, "夕陽天頂要偏藍紫");
    }

    #[test]
    fn night_horizon_glows_with_light_pollution() {
        // 西門町的夜空：地平線是城市燈光映出的暗橘紫，比天頂亮
        let night = sky_colors_at(1.0);
        assert!(luminance(night.horizon) > luminance(night.zenith) * 2.0);
        assert!(night.horizon.x > night.horizon.z, "光害地平線要偏暖");
    }

    #[test]
    fn weather_tint_identity_when_clear_and_darker_in_storm() {
        assert_eq!(weather_sky_tint(&weather(WeatherType::Clear)), Vec3::ONE);
        let storm = weather_sky_tint(&weather(WeatherType::Stormy));
        assert!(storm.max_element() < 0.6);
    }

    #[test]
    fn weather_tint_blends_during_transition() {
        let clear = weather_sky_tint(&weather(WeatherType::Clear));
        let storm = weather_sky_tint(&weather(WeatherType::Stormy));
        let half = weather_sky_tint(&transitioning(WeatherType::Clear, WeatherType::Stormy, 0.5));
        assert!(half.abs_diff_eq((clear + storm) / 2.0, 1e-5));
    }

    // --- 光暈 ---

    #[test]
    fn glow_only_around_sunrise_and_sunset() {
        assert_eq!(sun_glow_at(12.0), Vec3::ZERO);
        assert_eq!(sun_glow_at(23.0), Vec3::ZERO);
        assert!(sun_glow_at(6.25).max_element() > 0.0);
        assert!(sun_glow_at(17.75).max_element() > 0.0);
    }

    #[test]
    fn glow_direction_follows_sun_then_rests_on_horizon() {
        assert!(glow_direction(17.0).y > 0.0, "日落前光暈在地平線上方");
        // 入夜後停在日落方向的地平線，不會跟著夜間光源跳到半空
        let dusk = glow_direction(18.5);
        assert!(dusk.y.abs() < 1e-5);
        assert!(dusk.abs_diff_eq(glow_direction(18.0), 1e-5));
    }

    // --- cubemap 方向 ---

    #[test]
    fn cube_face_centers_point_along_axes() {
        // wgpu 的面順序 +X -X +Y -Y +Z -Z；Bevy 取樣時把 z 反過來，所以 +Z 面對應世界的 -Z
        let center = SKY_FACE_SIZE / 2;
        let expected = [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::NEG_Z,
            Vec3::Z,
        ];
        for (face, axis) in expected.into_iter().enumerate() {
            let dir = cube_texel_world_direction(face as u32, center, center);
            assert!(
                dir.dot(axis) > 0.99,
                "第 {face} 面中心是 {dir}，應接近 {axis}"
            );
        }
    }

    /// 照 Vulkan／wgpu 規格的主軸選面公式，從世界方向找出 cubemap 的面與像素
    /// （與產生貼圖的 `cube_texel_world_direction` 是兩套獨立寫法）
    fn select_texel(world: Vec3) -> (u32, u32, u32) {
        let c = Vec3::new(world.x, world.y, -world.z); // Bevy 取樣時把 z 反過來
        let a = c.abs();
        let (face, sc, tc, ma) = if a.x >= a.y && a.x >= a.z {
            if c.x > 0.0 {
                (0, -c.z, -c.y, a.x)
            } else {
                (1, c.z, -c.y, a.x)
            }
        } else if a.y >= a.z {
            if c.y > 0.0 {
                (2, c.x, c.z, a.y)
            } else {
                (3, c.x, -c.z, a.y)
            }
        } else if c.z > 0.0 {
            (4, c.x, -c.y, a.z)
        } else {
            (5, -c.x, -c.y, a.z)
        };
        let to_pixel = |s: f32| {
            ((f32::midpoint(s / ma, 1.0) * SKY_FACE_SIZE as f32) as u32).min(SKY_FACE_SIZE - 1)
        };
        (face, to_pixel(sc), to_pixel(tc))
    }

    #[test]
    fn cube_texels_match_spec_face_selection() {
        // 每個方向都要落回「指著它自己」的像素：任何一面左右鏡像或上下顛倒都會被抓到
        for i in 0..72 {
            for j in -9..=9 {
                let azimuth = (i as f32 * 5.0).to_radians();
                let elevation = (j as f32 * 10.0).to_radians();
                let d = Vec3::new(
                    elevation.cos() * azimuth.cos(),
                    elevation.sin(),
                    elevation.cos() * azimuth.sin(),
                );
                let (face, x, y) = select_texel(d);
                let back = cube_texel_world_direction(face, x, y);
                assert!(
                    back.dot(d) > 0.998,
                    "方向 {d} 選到第 {face} 面 ({x}, {y})，該像素卻指向 {back}"
                );
            }
        }
    }

    #[test]
    fn side_faces_are_not_upside_down() {
        // 側面最上排往上約 45°、最下排往下約 45°
        for face in [0, 1, 4, 5] {
            assert!(cube_texel_world_direction(face, 16, 0).y > 0.6);
            assert!(cube_texel_world_direction(face, 16, SKY_FACE_SIZE - 1).y < -0.6);
        }
    }

    // --- 天空像素 ---

    fn sample_snapshot() -> SkySnapshot {
        SkySnapshot {
            zenith: Vec3::new(0.05, 0.1, 0.4),
            horizon: Vec3::new(0.5, 0.4, 0.3),
            glow: Vec3::new(0.6, 0.2, 0.05),
            glow_direction: Vec3::NEG_Z,
        }
    }

    #[test]
    fn sky_at_and_below_horizon_is_exactly_horizon_color() {
        let s = sample_snapshot();
        // 包含朝著光暈的方向：地平線那一圈永遠等於霧色
        for dir in [
            Vec3::NEG_Z,
            Vec3::X,
            Vec3::new(0.0, -0.5, -0.8).normalize(),
            Vec3::NEG_Y,
        ] {
            assert_eq!(sky_pixel(dir, &s), s.horizon, "方向 {dir}");
        }
    }

    #[test]
    fn sky_straight_up_is_zenith_color() {
        let s = SkySnapshot {
            glow: Vec3::ZERO,
            ..sample_snapshot()
        };
        assert!(sky_pixel(Vec3::Y, &s).abs_diff_eq(s.zenith, 1e-5));
    }

    #[test]
    fn glow_brightens_the_sun_side_only() {
        let s = sample_snapshot();
        let toward = sky_pixel(Vec3::new(0.0, 0.2, -1.0).normalize(), &s);
        let away = sky_pixel(Vec3::new(0.0, 0.2, 1.0).normalize(), &s);
        assert!(luminance(toward) > luminance(away) + 0.05);
    }

    // --- cubemap 像素 ---

    #[test]
    fn cubemap_top_is_zenith_and_horizon_rows_are_horizon() {
        let s = SkySnapshot {
            glow: Vec3::ZERO,
            ..sample_snapshot()
        };
        let pixels = sky_cubemap_pixels(&s);
        assert_eq!(
            pixels.len(),
            (6 * SKY_FACE_SIZE * SKY_FACE_SIZE * 4) as usize
        );
        let mid = SKY_FACE_SIZE / 2;
        // 頂面中心接近天頂色（中心像素不是正上方，允許 1 階誤差）
        let top = texel(&pixels, 2, mid, mid);
        let zenith = srgb_u8(s.zenith);
        for c in 0..3 {
            assert!(
                top[c].abs_diff(zenith[c]) <= 1,
                "頂面中心 {top:?}，天頂 {zenith:?}"
            );
        }
        // 側面最靠近地平線的上下兩排都是純地平線色，線性過濾後地平線剛好等於霧色
        for face in [0, 1, 4, 5] {
            for y in [mid - 1, mid] {
                for x in 0..SKY_FACE_SIZE {
                    assert_eq!(
                        texel(&pixels, face, x, y),
                        srgb_u8(s.horizon),
                        "第 {face} 面 ({x}, {y})"
                    );
                }
            }
        }
        assert_eq!(texel(&pixels, 3, mid, mid), srgb_u8(s.horizon), "底面");
    }

    #[test]
    fn cubemap_glow_lands_on_the_sunset_side() {
        // 17:45 太陽在世界的 -Z 方向，對應 cubemap 的 +Z 面（第 4 面）
        let s = sky_snapshot(17.75, &weather(WeatherType::Clear));
        let pixels = sky_cubemap_pixels(&s);
        let row = SKY_FACE_SIZE / 2 - 4; // 地平線上方一點
        let sun_side = texel(&pixels, 4, SKY_FACE_SIZE / 2, row);
        let far_side = texel(&pixels, 5, SKY_FACE_SIZE / 2, row);
        assert!(
            u32::from(sun_side[0]) > u32::from(far_side[0]) + 20,
            "太陽那側 {sun_side:?} 應比背面 {far_side:?} 紅亮"
        );
    }

    // --- 霧 ---

    #[test]
    fn clear_weather_has_light_fog() {
        let v = fog_visibility(&weather(WeatherType::Clear));
        assert!((100.0..=1000.0).contains(&v), "晴天能見度 {v}");
    }

    #[test]
    fn fog_thickens_with_bad_weather() {
        let v = |w| fog_visibility(&weather(w));
        assert!(v(WeatherType::Cloudy) < v(WeatherType::Clear));
        assert!(v(WeatherType::Rainy) < v(WeatherType::Cloudy));
        assert!(v(WeatherType::Stormy) < v(WeatherType::Rainy));
        assert!(v(WeatherType::Foggy) < v(WeatherType::Stormy));
        assert!(v(WeatherType::Sandstorm) < v(WeatherType::Foggy));
    }

    #[test]
    fn fog_visibility_blends_during_transition() {
        let clear = fog_visibility(&weather(WeatherType::Clear));
        let foggy = fog_visibility(&weather(WeatherType::Foggy));
        let half = fog_visibility(&transitioning(WeatherType::Clear, WeatherType::Foggy, 0.5));
        assert!((half - f32::midpoint(clear, foggy)).abs() < 1e-3);
    }

    #[test]
    fn moon_fades_behind_bad_weather() {
        // 月亮不吃距離霧（在 ~500 m 外會整個不見），改依天氣淡出
        let m = |w| moon_clearness(&weather(w));
        assert_eq!(m(WeatherType::Clear), 1.0);
        assert!(
            m(WeatherType::Cloudy) > 0.0 && m(WeatherType::Cloudy) < 1.0,
            "陰天半透明"
        );
        for w in [
            WeatherType::Rainy,
            WeatherType::Stormy,
            WeatherType::Foggy,
            WeatherType::Sandstorm,
        ] {
            assert_eq!(m(w), 0.0, "{w:?} 看不到月亮");
        }
        let half = moon_clearness(&transitioning(WeatherType::Clear, WeatherType::Foggy, 0.5));
        assert!(half > 0.0 && half < 1.0, "切換中漸變：{half}");
    }

    // --- 系統 ---

    fn sky_app(hour: f32) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .insert_resource(WorldTime { hour, ..default() })
            .insert_resource(WeatherState::default())
            .init_resource::<SkyCubemap>()
            .add_systems(Update, (attach_skybox, update_sky_and_fog));
        app.world_mut().spawn((GameCamera, DistanceFog::default()));
        app
    }

    fn sky_pixels(app: &App) -> Vec<u8> {
        let handle = app.world().resource::<SkyCubemap>().image.clone();
        app.world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap()
            .data
            .clone()
            .unwrap()
    }

    /// 把貼圖內容換成全 0：之後若被重畫就看得出來
    fn blank_sky(app: &mut App) {
        let handle = app.world().resource::<SkyCubemap>().image.clone();
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let image = images.get_mut(&handle).unwrap();
        image.data = Some(vec![0; image.data.as_ref().unwrap().len()]);
    }

    fn fog(app: &mut App) -> DistanceFog {
        let mut q = app.world_mut().query::<&DistanceFog>();
        q.single(app.world()).unwrap().clone()
    }

    #[test]
    fn sky_image_stays_editable_after_upload() {
        // 只標 RENDER_WORLD 的話，Bevy 送上 GPU 後會把它從 Assets 移除，之後就改不到了
        let app = sky_app(8.0);
        let handle = app.world().resource::<SkyCubemap>().image.clone();
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .unwrap();
        assert!(image.asset_usage.contains(RenderAssetUsages::MAIN_WORLD));
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 6);
    }

    #[test]
    fn fog_color_matches_sky_horizon() {
        let mut app = sky_app(17.5);
        app.update();
        let fog = fog(&mut app);
        let pixels = sky_pixels(&app);
        let horizon_texel = texel(&pixels, 0, 3, SKY_FACE_SIZE / 2);
        let fog_u8 = Srgba::from(fog.color.to_linear()).to_u8_array();
        assert_eq!(fog_u8, horizon_texel, "霧色要等於天空地平線色");
        assert_eq!(fog.color.alpha(), 1.0, "霧要能完全蓋住遠景");
        let expected = FogFalloff::from_visibility_squared(CLEAR_FOG_VISIBILITY);
        match (fog.falloff, expected) {
            (
                FogFalloff::ExponentialSquared { density: a },
                FogFalloff::ExponentialSquared { density: b },
            ) => {
                assert!((a - b).abs() < 1e-6);
            }
            (other, _) => panic!("晴天霧應是 ExponentialSquared，實際 {other:?}"),
        }
    }

    #[test]
    fn sky_redraws_only_when_the_minute_changes() {
        let mut app = sky_app(8.0);
        app.update();
        blank_sky(&mut app);
        app.world_mut().resource_mut::<WorldTime>().hour = 8.01; // 同一分鐘
        app.update();
        assert!(
            sky_pixels(&app).iter().all(|&b| b == 0),
            "同一分鐘內不該重畫"
        );
        app.world_mut().resource_mut::<WorldTime>().hour = 8.02; // 下一分鐘
        app.update();
        assert!(sky_pixels(&app).iter().any(|&b| b != 0), "換分鐘要重畫");
    }

    #[test]
    fn sky_redraws_as_weather_transition_progresses() {
        let mut app = sky_app(8.0);
        app.update();
        blank_sky(&mut app);
        *app.world_mut().resource_mut::<WeatherState>() =
            transitioning(WeatherType::Clear, WeatherType::Rainy, 0.1);
        app.update();
        assert!(
            sky_pixels(&app).iter().any(|&b| b != 0),
            "天氣開始切換要重畫"
        );

        // 每 1/50 的進度重畫一次：同一段內不重畫，前進一段才重畫
        blank_sky(&mut app);
        app.world_mut()
            .resource_mut::<WeatherState>()
            .transition_progress = 0.11;
        app.update();
        assert!(
            sky_pixels(&app).iter().all(|&b| b == 0),
            "同一段進度內不該重畫"
        );
        app.world_mut()
            .resource_mut::<WeatherState>()
            .transition_progress = 0.13;
        app.update();
        assert!(
            sky_pixels(&app).iter().any(|&b| b != 0),
            "過渡前進一段要重畫"
        );
    }

    #[test]
    fn skybox_brightness_cancels_exposure() {
        let mut app = sky_app(8.0);
        let custom = app
            .world_mut()
            .spawn((GameCamera, Exposure { ev100: 12.0 }))
            .id();
        app.update();
        let mut q = app
            .world_mut()
            .query::<(Entity, &Skybox, Option<&Exposure>)>();
        let found: Vec<_> = q
            .iter(app.world())
            .map(|(e, sky, exp)| {
                (
                    e,
                    sky.brightness * exp.copied().unwrap_or_default().exposure(),
                )
            })
            .collect();
        assert_eq!(found.len(), 2, "兩台 GameCamera 都要掛上天空盒");
        for (entity, product) in found {
            assert!(
                (product - 1.0).abs() < 1e-4,
                "{entity:?} 亮度 × 曝光 = {product}"
            );
        }
        assert!(app.world().get::<Skybox>(custom).is_some());
    }
}
