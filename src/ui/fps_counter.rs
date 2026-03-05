//! FPS 計數器（開發工具）

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

/// FPS 計數器文字標記組件
#[derive(Component)]
pub struct FpsCounterText;

/// 初始化 FPS 計數器 UI
pub fn setup_fps_counter(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0), // 改到左上角，避免被小地圖擋住
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("FPS: --"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                FpsCounterText,
            ));
        });
}

/// 更新 FPS 顯示
pub fn update_fps_counter(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<(&mut Text, &mut TextColor), With<FpsCounterText>>,
) {
    for (mut text, mut color) in &mut query {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                // 依據 FPS 變色：綠色 (>60)、黃色 (30-60)、紅色 (<30)
                color.0 = if value >= 60.0 {
                    Color::srgb(0.0, 1.0, 0.0) // 綠色
                } else if value >= 30.0 {
                    Color::srgb(1.0, 1.0, 0.0) // 黃色
                } else {
                    Color::srgb(1.0, 0.0, 0.0) // 紅色
                };

                **text = format!("FPS: {value:.1}");
            }
        }
    }
}
