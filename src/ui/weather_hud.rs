//! 天氣 HUD 系統
//!
//! 顯示當前天氣狀態和圖示

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use super::components::{
    ChineseFont, RainDropIcon, SunRay, WeatherHudContainer, WeatherIconContainer,
    WeatherIconElement, WeatherIconType, WeatherNameText,
};
use crate::core::{WeatherState, WeatherType};

// ============================================================================
// 天氣 HUD 顏色常數
// ============================================================================
const WEATHER_HUD_BG: Color = Color::srgba(0.05, 0.08, 0.12, 0.9);
const WEATHER_HUD_BORDER: Color = Color::srgba(0.3, 0.45, 0.55, 0.7);

// 天氣圖示顏色
const SUN_COLOR: Color = Color::srgb(1.0, 0.85, 0.2);
const SUN_GLOW: Color = Color::srgba(1.0, 0.9, 0.4, 0.5);
const CLOUD_COLOR: Color = Color::srgb(0.85, 0.88, 0.92);
const CLOUD_DARK: Color = Color::srgb(0.6, 0.65, 0.7);
const RAIN_COLOR: Color = Color::srgb(0.4, 0.7, 0.95);

/// 設置天氣 HUD
pub fn setup_weather_hud(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // 天氣 HUD 容器（金錢顯示下方，避免重疊）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(420.0), // 金錢下方 (365 + 約 55)
                right: Val::Px(10.0),
                width: Val::Px(150.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BackgroundColor(WEATHER_HUD_BG),
            BorderColor::all(WEATHER_HUD_BORDER),
            BorderRadius::all(Val::Px(8.0)),
            WeatherHudContainer,
        ))
        .with_children(|parent| {
            // 天氣圖示容器（固定大小）
            parent
                .spawn((
                    Node {
                        width: Val::Px(40.0),
                        height: Val::Px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    WeatherIconContainer,
                ))
                .with_children(|icon_parent| {
                    // === 太陽圖示 ===
                    spawn_sun_icon(icon_parent);
                    // === 雲圖示 ===
                    spawn_cloud_icon(icon_parent);
                    // === 雨圖示 ===
                    spawn_rain_icon(icon_parent);
                    // === 霧圖示 ===
                    spawn_fog_icon(icon_parent);
                });

            // 天氣名稱和按鍵提示
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },))
                .with_children(|col| {
                    // 天氣名稱
                    col.spawn((
                        Text::new("晴天"),
                        TextFont {
                            font_size: 18.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        WeatherNameText,
                    ));
                    // 按鍵提示
                    col.spawn((
                        Text::new("[F1] 切換"),
                        TextFont {
                            font_size: 11.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.55, 0.65, 0.9)),
                    ));
                });
        });
}

/// 生成圓形天氣元素（用於太陽、雲朵、雨雲）
fn spawn_weather_circle(
    parent: &mut ChildSpawnerCommands,
    size: f32,
    left: Val,
    top: Val,
    color: Color,
) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(size),
            height: Val::Px(size),
            left,
            top,
            ..default()
        },
        BackgroundColor(color),
        BorderRadius::all(Val::Percent(50.0)),
    ));
}

/// 生成矩形天氣元素（用於雲朵底部、雲層）
fn spawn_weather_rect(
    parent: &mut ChildSpawnerCommands,
    width: f32,
    height: f32,
    left: Val,
    top: Val,
    color: Color,
    radius: f32,
) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(width),
            height: Val::Px(height),
            left,
            top,
            ..default()
        },
        BackgroundColor(color),
        BorderRadius::all(Val::Px(radius)),
    ));
}

/// 生成天氣圖示容器
fn spawn_weather_icon_container<'a>(
    parent: &'a mut ChildSpawnerCommands,
    ty: WeatherIconType,
    visible: bool,
    layout: Option<(FlexDirection, JustifyContent, AlignItems, Val)>,
) -> EntityCommands<'a> {
    let (flex_direction, justify_content, align_items, row_gap) = layout.unwrap_or((
        FlexDirection::Row,
        JustifyContent::FlexStart,
        AlignItems::FlexStart,
        Val::Px(0.0),
    ));

    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(40.0),
            height: Val::Px(40.0),
            flex_direction,
            justify_content,
            align_items,
            row_gap,
            ..default()
        },
        if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        },
        WeatherIconElement { weather_type: ty },
    ))
}

/// 生成太陽圖示
fn spawn_sun_icon(parent: &mut ChildSpawnerCommands) {
    spawn_weather_icon_container(
        parent,
        WeatherIconType::Sun,
        true, // 預設顯示
        Some((
            FlexDirection::Row,
            JustifyContent::Center,
            AlignItems::Center,
            Val::Px(0.0),
        )),
    )
    .with_children(|sun| {
        // 外發光 (無指定位置，默認靠 Flex 居中或左上?)
        // 原代碼無 left/top，這裡傳入 Auto 保持一致，但注意原代碼依賴父容器佈局
        spawn_weather_circle(sun, 32.0, Val::Auto, Val::Auto, SUN_GLOW);
        // 太陽核心
        spawn_weather_circle(sun, 20.0, Val::Px(10.0), Val::Px(10.0), SUN_COLOR);
        // 光芒（8 條）
        for i in 0..8 {
            let angle = i as f32 * 45.0_f32.to_radians();
            let ray_len = 6.0;
            let offset = 14.0;
            let x = 20.0 + angle.cos() * offset - 1.5;
            let y = 20.0 + angle.sin() * offset - ray_len / 2.0;
            sun.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(3.0),
                    height: Val::Px(ray_len),
                    left: Val::Px(x),
                    top: Val::Px(y),
                    ..default()
                },
                BackgroundColor(SUN_COLOR),
                BorderRadius::all(Val::Px(1.5)),
                Transform::from_rotation(Quat::from_rotation_z(-angle)),
                SunRay { index: i },
            ));
        }
    });
}

/// 生成雲圖示
fn spawn_cloud_icon(parent: &mut ChildSpawnerCommands) {
    spawn_weather_icon_container(
        parent,
        WeatherIconType::Cloud,
        false,
        None, // 預設佈局（子元素絕對定位）
    )
    .with_children(|cloud| {
        // 雲朵由多個圓組成
        // 左側小圓
        spawn_weather_circle(cloud, 14.0, Val::Px(4.0), Val::Px(18.0), CLOUD_COLOR);
        // 中間大圓
        spawn_weather_circle(cloud, 20.0, Val::Px(10.0), Val::Px(10.0), CLOUD_COLOR);
        // 右側中圓
        spawn_weather_circle(cloud, 16.0, Val::Px(22.0), Val::Px(16.0), CLOUD_COLOR);
        // 底部連接
        spawn_weather_rect(
            cloud,
            28.0,
            10.0,
            Val::Px(6.0),
            Val::Px(22.0),
            CLOUD_COLOR,
            4.0,
        );
    });
}

/// 生成雨圖示
fn spawn_rain_icon(parent: &mut ChildSpawnerCommands) {
    spawn_weather_icon_container(parent, WeatherIconType::Rain, false, None).with_children(
        |rain| {
            // 深色雲
            spawn_weather_circle(rain, 12.0, Val::Px(4.0), Val::Px(6.0), CLOUD_DARK);
            spawn_weather_circle(rain, 16.0, Val::Px(12.0), Val::Px(2.0), CLOUD_DARK);
            spawn_weather_circle(rain, 12.0, Val::Px(24.0), Val::Px(6.0), CLOUD_DARK);
            spawn_weather_rect(
                rain,
                26.0,
                8.0,
                Val::Px(7.0),
                Val::Px(12.0),
                CLOUD_DARK,
                3.0,
            );
            // 雨滴
            for i in 0..3 {
                let x = 8.0 + i as f32 * 10.0;
                rain.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(2.5),
                        height: Val::Px(10.0),
                        left: Val::Px(x),
                        top: Val::Px(24.0),
                        ..default()
                    },
                    BackgroundColor(RAIN_COLOR),
                    BorderRadius::all(Val::Px(1.5)),
                    RainDropIcon {
                        index: i,
                        offset: i as f32 * 0.3,
                    },
                ));
            }
        },
    );
}

/// 生成霧圖示
fn spawn_fog_icon(parent: &mut ChildSpawnerCommands) {
    spawn_weather_icon_container(
        parent,
        WeatherIconType::Fog,
        false,
        Some((
            FlexDirection::Column,
            JustifyContent::Center,
            AlignItems::Center,
            Val::Px(5.0),
        )),
    )
    .with_children(|fog| {
        // 三條橫線代表霧
        for i in 0..3 {
            let width = match i {
                0 => 28.0,
                1 => 34.0,
                _ => 24.0,
            };
            let alpha = match i {
                0 => 0.9,
                1 => 0.7,
                _ => 0.5,
            };
            fog.spawn((
                Node {
                    width: Val::Px(width),
                    height: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.75, 0.78, 0.82, alpha)),
                BorderRadius::all(Val::Px(2.0)),
            ));
        }
    });
}

/// 更新天氣 HUD 顯示
pub fn update_weather_hud(
    weather: Res<WeatherState>,
    mut icon_query: Query<(&WeatherIconElement, &mut Visibility)>,
    mut name_query: Query<&mut Text, With<WeatherNameText>>,
) {
    // 取得當前顯示的天氣類型
    let display_weather = if weather.is_transitioning {
        // 過渡期間根據進度顯示
        if weather.transition_progress > 0.5 {
            weather.target_weather
        } else {
            weather.weather_type
        }
    } else {
        weather.weather_type
    };

    // 將 WeatherType 轉換為 WeatherIconType
    let target_icon = match display_weather {
        WeatherType::Clear => WeatherIconType::Sun,
        WeatherType::Cloudy => WeatherIconType::Cloud,
        WeatherType::Rainy => WeatherIconType::Rain,
        WeatherType::Foggy => WeatherIconType::Fog,
        WeatherType::Stormy => WeatherIconType::Rain, // 暴風雨用雨天圖示
        WeatherType::Sandstorm => WeatherIconType::Fog, // 沙塵暴用霧天圖示（TODO: 專用圖示）
    };

    // 更新圖示可見性
    for (icon_element, mut visibility) in icon_query.iter_mut() {
        *visibility = if icon_element.weather_type == target_icon {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    // 更新名稱
    if let Ok(mut text) = name_query.single_mut() {
        // 如果正在過渡，顯示過渡提示
        if weather.is_transitioning {
            **text = format!("{}...", weather.target_weather.name());
        } else {
            **text = display_weather.name().to_string();
        }
    }
}
