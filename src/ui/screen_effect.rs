//! WASTED / BUSTED 全螢幕效果系統 (GTA 5 風格)
//!
//! 玩家死亡時顯示紅色色調 + "WASTED" 文字 + 慢動作 + 淡出黑幕，
//! 被捕時顯示藍色色調 + "逮捕" 文字（Phase 2 實現）。

use bevy::prelude::*;
use bevy::time::Real;

use crate::combat::{killcam_update_system, KillCamState, RespawnState};
use crate::core::{ease_in_quad, ease_out_quad, AppState};
use crate::wanted::{handle_arrest_event_system, ArrestEvent, ArrestType};

use super::components::ChineseFont;
use super::constants::*;

// ============================================================================
// 常數
// ============================================================================

/// 慢動作階段持續時間（秒）
const SLOWDOWN_DURATION: f32 = 0.5;
/// Hold 階段結束時間
const HOLD_END: f32 = 3.5;
/// 淡出黑幕階段結束時間（效果總時長）
const FADE_END: f32 = 4.5;
/// 慢動作目標時間縮放（5 倍慢動作）
const SLOW_MOTION_SCALE: f32 = 0.2;
/// 文字縮放動畫起始倍率
const TEXT_SCALE_START: f32 = 1.5;

// ============================================================================
// 類型定義
// ============================================================================

/// 全螢幕效果類型
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScreenEffectType {
    Wasted,
    Busted,
}

/// 全螢幕效果階段
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum ScreenEffectPhase {
    #[default]
    Inactive,
    /// 慢動作淡入（0.0–0.5s）
    SlowDown,
    /// 慢動作維持（0.5–3.5s）
    Hold,
    /// 淡出至黑幕（3.5–4.5s）
    FadeToBlack,
    /// 完成，觸發重生/逮捕
    Complete,
}

/// 全螢幕效果狀態（WASTED / BUSTED 共用）
#[derive(Resource)]
pub struct ScreenEffectState {
    /// 效果類型
    pub effect_type: Option<ScreenEffectType>,
    /// 是否凍結重生計時器
    pub respawn_timer_frozen: bool,
    /// 暫存的逮捕事件（BUSTED 動畫完成後重新發送）
    pending_arrest: Option<ArrestEvent>,
    /// BUSTED 剛完成標記（防止重新觸發）
    busted_just_completed: bool,
    /// 當前階段
    phase: ScreenEffectPhase,
    /// 已經過時間（牆鐘時間）
    elapsed: f32,
    /// 當前時間縮放
    time_scale: f32,
    /// 上一幀玩家是否死亡（用於偵測 false→true 轉變）
    was_dead_last_frame: bool,
}

impl Default for ScreenEffectState {
    fn default() -> Self {
        Self {
            effect_type: None,
            phase: ScreenEffectPhase::Inactive,
            elapsed: 0.0,
            time_scale: 1.0,
            respawn_timer_frozen: false,
            pending_arrest: None,
            busted_just_completed: false,
            was_dead_last_frame: false,
        }
    }
}

impl ScreenEffectState {
    /// 是否正在播放效果
    pub fn is_active(&self) -> bool {
        self.phase != ScreenEffectPhase::Inactive
    }

    fn reset(&mut self) {
        self.effect_type = None;
        self.phase = ScreenEffectPhase::Inactive;
        self.elapsed = 0.0;
        self.time_scale = 1.0;
        self.pending_arrest = None;
    }
}

// ============================================================================
// UI 組件標記
// ============================================================================

#[derive(Component)]
struct ScreenEffectRoot;

#[derive(Component)]
struct ScreenEffectTint;

#[derive(Component)]
struct ScreenEffectLabel;

#[derive(Component)]
struct ScreenEffectBlackout;

// ============================================================================
// 系統
// ============================================================================

/// 建立 WASTED/BUSTED UI 節點（Startup，初始隱藏）
fn setup_screen_effect_ui(mut commands: Commands, font: Res<ChineseFont>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ZIndex(90),
            Visibility::Hidden,
            ScreenEffectRoot,
        ))
        .with_children(|root| {
            // 色調疊層（紅色 / 藍色）
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ScreenEffectTint,
            ));

            // WASTED / 逮捕 文字
            root.spawn((
                Text::new(""),
                TextFont {
                    font_size: SCREEN_EFFECT_TEXT_SIZE,
                    font: font.font.clone(),
                    ..default()
                },
                TextColor(Color::NONE),
                ScreenEffectLabel,
            ));

            // 黑幕疊層（fade to black）
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                ScreenEffectBlackout,
            ));
        });
}

/// 偵測逮捕事件 → 觸發 BUSTED
fn detect_busted_trigger(
    mut arrest_events: MessageReader<ArrestEvent>,
    mut state: ResMut<ScreenEffectState>,
) {
    // BUSTED 剛完成的幀：跳過以避免重新觸發（re-sent 的 ArrestEvent 仍在緩衝區）
    if state.busted_just_completed {
        state.busted_just_completed = false;
        for _ in arrest_events.read() {} // 消耗事件
        return;
    }

    for event in arrest_events.read() {
        if event.arrest_type == ArrestType::PlayerSurrender && state.effect_type.is_none() {
            state.effect_type = Some(ScreenEffectType::Busted);
            state.phase = ScreenEffectPhase::SlowDown;
            state.elapsed = 0.0;
            state.time_scale = 1.0;
            state.pending_arrest = Some(event.clone());
            info!("🚔 BUSTED 效果觸發");
            break;
        }
    }
}

/// 偵測玩家死亡 → 觸發 WASTED
fn detect_wasted_trigger(
    respawn_state: Res<RespawnState>,
    mut state: ResMut<ScreenEffectState>,
) {
    let is_dead = respawn_state.is_dead;

    // 偵測 is_dead 從 false → true 的轉變
    if is_dead && !state.was_dead_last_frame && state.effect_type.is_none() {
        state.effect_type = Some(ScreenEffectType::Wasted);
        state.phase = ScreenEffectPhase::SlowDown;
        state.elapsed = 0.0;
        state.time_scale = 1.0;
        state.respawn_timer_frozen = true;
        info!("💀 WASTED 效果觸發");
    }

    state.was_dead_last_frame = is_dead;
}

/// 核心狀態機：管理階段轉換與時間縮放
fn screen_effect_phase_machine(
    real_time: Res<Time<Real>>,
    mut state: ResMut<ScreenEffectState>,
    mut killcam: ResMut<KillCamState>,
    mut time_strategy: ResMut<bevy::time::TimeUpdateStrategy>,
    mut respawn_state: ResMut<RespawnState>,
    mut arrest_events: MessageWriter<ArrestEvent>,
) {
    if state.phase == ScreenEffectPhase::Inactive {
        return;
    }

    // 強制結束 Kill Cam（WASTED/BUSTED 優先）
    if killcam.active {
        killcam.active = false;
        killcam.time_scale = 1.0;
    }

    // 使用真實時間（不受 ManualDuration 影響）避免時間縮放漂移
    let real_dt = real_time.delta_secs();
    state.elapsed += real_dt;

    // 階段轉換
    match state.phase {
        ScreenEffectPhase::SlowDown => {
            if state.elapsed >= SLOWDOWN_DURATION {
                state.phase = ScreenEffectPhase::Hold;
            }
            let t = (state.elapsed / SLOWDOWN_DURATION).min(1.0);
            state.time_scale = 1.0 - (1.0 - SLOW_MOTION_SCALE) * ease_out_quad(t);
        }
        ScreenEffectPhase::Hold => {
            if state.elapsed >= HOLD_END {
                state.phase = ScreenEffectPhase::FadeToBlack;
            }
            state.time_scale = SLOW_MOTION_SCALE;
        }
        ScreenEffectPhase::FadeToBlack => {
            if state.elapsed >= FADE_END {
                state.phase = ScreenEffectPhase::Complete;
            }
            let t = ((state.elapsed - HOLD_END) / (FADE_END - HOLD_END)).min(1.0);
            state.time_scale =
                SLOW_MOTION_SCALE + (1.0 - SLOW_MOTION_SCALE) * ease_in_quad(t);
        }
        ScreenEffectPhase::Complete => {
            match state.effect_type {
                Some(ScreenEffectType::Wasted) => {
                    // 觸發即時重生（設 respawn_timer=0 讓 player_respawn_system 在下一幀執行）
                    respawn_state.respawn_timer = 0.0;
                    state.respawn_timer_frozen = false;
                    info!("💀 WASTED 效果結束，觸發重生");
                }
                Some(ScreenEffectType::Busted) => {
                    // 重新發送逮捕事件（讓 handle_arrest_event_system 在下一幀執行）
                    if let Some(arrest_data) = state.pending_arrest.take() {
                        arrest_events.write(arrest_data);
                    }
                    state.busted_just_completed = true;
                    info!("🚔 BUSTED 效果結束，執行逮捕");
                }
                None => {}
            }
            state.reset();
        }
        ScreenEffectPhase::Inactive => {}
    }

    // 設定全局時間縮放（使用真實 dt 乘以縮放比例）
    if state.phase != ScreenEffectPhase::Inactive {
        let scaled_dt = (real_dt * state.time_scale).max(0.0001);
        *time_strategy = bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f32(scaled_dt),
        );
    } else {
        *time_strategy = bevy::time::TimeUpdateStrategy::Automatic;
    }
}

/// 更新 UI 視覺效果（色調、文字、黑幕）
fn screen_effect_visual_update(
    state: Res<ScreenEffectState>,
    mut root_query: Query<&mut Visibility, With<ScreenEffectRoot>>,
    mut tint_query: Query<&mut BackgroundColor, With<ScreenEffectTint>>,
    mut text_query: Query<
        (&mut TextColor, &mut TextFont, &mut Text),
        With<ScreenEffectLabel>,
    >,
    mut blackout_query: Query<
        &mut BackgroundColor,
        (With<ScreenEffectBlackout>, Without<ScreenEffectTint>),
    >,
) {
    let Ok(mut root_vis) = root_query.single_mut() else {
        return;
    };

    if state.phase == ScreenEffectPhase::Inactive {
        *root_vis = Visibility::Hidden;
        return;
    }

    *root_vis = Visibility::Inherited;
    let is_wasted = matches!(state.effect_type, Some(ScreenEffectType::Wasted));

    // --- 色調疊層 ---
    if let Ok(mut bg) = tint_query.single_mut() {
        let alpha = match state.phase {
            ScreenEffectPhase::SlowDown => {
                let t = (state.elapsed / SLOWDOWN_DURATION).min(1.0);
                ease_out_quad(t) * SCREEN_EFFECT_TINT_ALPHA
            }
            ScreenEffectPhase::Hold | ScreenEffectPhase::FadeToBlack => {
                SCREEN_EFFECT_TINT_ALPHA
            }
            _ => 0.0,
        };
        *bg = if is_wasted {
            BackgroundColor(Color::srgba(0.5, 0.0, 0.0, alpha))
        } else {
            BackgroundColor(Color::srgba(0.0, 0.1, 0.5, alpha))
        };
    }

    // --- 文字 ---
    if let Ok((mut color, mut font, mut text)) = text_query.single_mut() {
        match state.phase {
            ScreenEffectPhase::SlowDown
            | ScreenEffectPhase::Hold
            | ScreenEffectPhase::FadeToBlack => {
                // 透明度（SlowDown 期間淡入）
                let text_alpha = match state.phase {
                    ScreenEffectPhase::SlowDown => {
                        let t = (state.elapsed / SLOWDOWN_DURATION).min(1.0);
                        ease_out_quad(t)
                    }
                    _ => 1.0,
                };

                // 縮放動畫：1.5x → 1.0x（SlowDown 期間）
                let scale_factor = match state.phase {
                    ScreenEffectPhase::SlowDown => {
                        let t = (state.elapsed / SLOWDOWN_DURATION).min(1.0);
                        TEXT_SCALE_START - (TEXT_SCALE_START - 1.0) * ease_out_quad(t)
                    }
                    _ => 1.0,
                };
                font.font_size = SCREEN_EFFECT_TEXT_SIZE * scale_factor;

                if is_wasted {
                    *color = TextColor(WASTED_TEXT_COLOR.with_alpha(text_alpha));
                    *text = Text::new("WASTED");
                } else {
                    *color = TextColor(BUSTED_TEXT_COLOR.with_alpha(text_alpha));
                    *text = Text::new("逮捕");
                }
            }
            _ => {
                *color = TextColor(Color::NONE);
            }
        }
    }

    // --- 黑幕 ---
    if let Ok(mut bg) = blackout_query.single_mut() {
        let alpha = match state.phase {
            ScreenEffectPhase::FadeToBlack => {
                let t = ((state.elapsed - HOLD_END) / (FADE_END - HOLD_END)).min(1.0);
                ease_in_quad(t)
            }
            _ => 0.0,
        };
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, alpha));
    }
}

// ============================================================================
// Plugin
// ============================================================================

pub(super) struct ScreenEffectPlugin;

impl Plugin for ScreenEffectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenEffectState>()
            .add_systems(
                Startup,
                setup_screen_effect_ui.in_set(super::UiSetup),
            )
            .add_systems(
                Update,
                (
                    detect_busted_trigger
                        .before(handle_arrest_event_system),
                    detect_wasted_trigger,
                    screen_effect_phase_machine
                        .after(detect_wasted_trigger)
                        .after(detect_busted_trigger)
                        .after(killcam_update_system),
                    screen_effect_visual_update.after(screen_effect_phase_machine),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
