//! 暫停選單系統
//!
//! 處理遊戲暫停、繼續、退出以及按鈕動畫效果

use bevy::prelude::*;
use bevy::time::Real;

use super::components::{ButtonScaleState, PauseMenu, QuitButton, ResumeButton, UiState};
use super::constants::{
    BUTTON_BASE_HEIGHT, BUTTON_BASE_WIDTH, BUTTON_HOVER_SCALE, BUTTON_PRESSED_SCALE,
    BUTTON_SCALE_SPEED, QUIT_BTN_HOVER, QUIT_BTN_NORMAL, QUIT_BTN_PRESSED, RESUME_BTN_HOVER,
    RESUME_BTN_NORMAL, RESUME_BTN_PRESSED,
};
use crate::core::AppState;

/// 檢查按鈕是否被按下
fn is_button_pressed(query: &Query<&Interaction, impl bevy::ecs::query::QueryFilter>) -> bool {
    query.iter().any(|i| *i == Interaction::Pressed)
}

/// 設置暫停狀態
fn set_pause_state(
    paused: bool,
    ui_state: &mut UiState,
    pause_menu_query: &mut Query<&mut Visibility, With<PauseMenu>>,
    time: &mut Time<Virtual>,
) {
    ui_state.paused = paused;
    if let Ok(mut visibility) = pause_menu_query.single_mut() {
        if paused {
            *visibility = Visibility::Visible;
            time.pause();
        } else {
            *visibility = Visibility::Hidden;
            time.unpause();
        }
    }
}

/// 進入暫停狀態
pub fn on_enter_pause(
    mut ui_state: ResMut<UiState>,
    mut pause_menu_query: Query<&mut Visibility, With<PauseMenu>>,
    mut time: ResMut<Time<Virtual>>,
) {
    set_pause_state(true, &mut ui_state, &mut pause_menu_query, &mut time);
}

/// 離開暫停狀態
pub fn on_exit_pause(
    mut ui_state: ResMut<UiState>,
    mut pause_menu_query: Query<&mut Visibility, With<PauseMenu>>,
    mut time: ResMut<Time<Virtual>>,
) {
    set_pause_state(false, &mut ui_state, &mut pause_menu_query, &mut time);
}

/// 暫停選單切換（ESC 鍵）和退出遊戲（暫停時按 Q）
pub fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    resume_query: Query<&Interaction, (Changed<Interaction>, With<ResumeButton>)>,
    quit_query: Query<&Interaction, (Changed<Interaction>, With<QuitButton>)>,
    ui_state: Res<UiState>,
) {
    // 存檔槽 UI 開啟時，ESC 由該系統處理，此處跳過
    if ui_state.show_save_slots {
        return;
    }
    // 檢查「繼續遊戲」按鈕點擊
    if is_button_pressed(&resume_query) {
        next_state.set(AppState::InGame);
        return;
    }

    // 檢查「退出遊戲」按鈕點擊或暫停時按 Q
    let quit_pressed = is_button_pressed(&quit_query)
        || (*state == AppState::Paused && keyboard.just_pressed(KeyCode::KeyQ));
    if quit_pressed {
        std::process::exit(0);
    }

    // ESC 切換暫停
    if keyboard.just_pressed(KeyCode::Escape) {
        let next = if *state == AppState::Paused {
            AppState::InGame
        } else {
            AppState::Paused
        };
        next_state.set(next);
    }
}

// ============================================================================
// 輔助函數
// ============================================================================
/// 更新單個按鈕的互動效果（顏色與縮放動畫）
fn update_single_button(
    interaction: &Interaction,
    bg: &mut BackgroundColor,
    node: &mut Node,
    scale_state: &mut ButtonScaleState,
    lerp_speed: f32,
    colors: (Color, Color, Color), // (Normal, Hover, Pressed)
) {
    let (normal_color, hover_color, pressed_color) = colors;

    // 設定目標縮放與顏色
    let new_target = match *interaction {
        Interaction::Pressed => {
            *bg = BackgroundColor(pressed_color);
            BUTTON_PRESSED_SCALE
        }
        Interaction::Hovered => {
            *bg = BackgroundColor(hover_color);
            BUTTON_HOVER_SCALE
        }
        Interaction::None => {
            *bg = BackgroundColor(normal_color);
            1.0
        }
    };
    scale_state.target = new_target;

    // 平滑動畫更新
    let diff = scale_state.target - scale_state.current;
    if diff.abs() > 0.001 {
        scale_state.current += diff * lerp_speed.min(1.0);
        node.width = Val::Px(BUTTON_BASE_WIDTH * scale_state.current);
        node.height = Val::Px(BUTTON_BASE_HEIGHT * scale_state.current);
    }
}

/// 按鈕懸停效果 - GTA 風格（顏色 + Node 尺寸動畫）
pub fn button_hover_effect(
    time: Res<Time<Real>>, // 使用真實時間，不受 Virtual time 暫停影響
    mut resume_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut Node,
            &mut ButtonScaleState,
        ),
        With<ResumeButton>,
    >,
    mut quit_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut Node,
            &mut ButtonScaleState,
        ),
        (With<QuitButton>, Without<ResumeButton>),
    >,
) {
    let dt = time.delta_secs();
    let lerp_speed = dt * BUTTON_SCALE_SPEED;

    // 繼續遊戲按鈕
    for (interaction, mut bg, mut node, mut scale_state) in resume_query.iter_mut() {
        update_single_button(
            interaction,
            &mut bg,
            &mut node,
            &mut scale_state,
            lerp_speed,
            (RESUME_BTN_NORMAL, RESUME_BTN_HOVER, RESUME_BTN_PRESSED),
        );
    }

    // 退出遊戲按鈕
    for (interaction, mut bg, mut node, mut scale_state) in quit_query.iter_mut() {
        update_single_button(
            interaction,
            &mut bg,
            &mut node,
            &mut scale_state,
            lerp_speed,
            (QUIT_BTN_NORMAL, QUIT_BTN_HOVER, QUIT_BTN_PRESSED),
        );
    }
}

/// 按鈕縮放動畫（已整合到 button_hover_effect，保留空函數以避免修改 mod.rs）
pub fn animate_button_scale() {
    // 動畫邏輯已整合到 button_hover_effect
}

pub(super) struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(crate::core::AppState::Paused), on_enter_pause)
            .add_systems(OnExit(crate::core::AppState::Paused), on_exit_pause)
            .add_systems(
                Update,
                (
                    toggle_pause,
                    button_hover_effect,
                    animate_button_scale.after(button_hover_effect),
                )
                    .in_set(super::UiActive),
            );
    }
}
