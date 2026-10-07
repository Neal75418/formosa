//! WASTED / BUSTED 全螢幕效果系統 (GTA 5 風格)
//!
//! 玩家死亡時顯示紅色色調 + "WASTED" 文字 + 慢動作 + 淡出黑幕，
//! 被捕時顯示藍色色調 + "逮捕" 文字（Phase 2 實現）。

use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
use bevy::time::{Real, Virtual};

use crate::combat::{killcam_update_system, KillCamState, RespawnState};
use crate::core::{ease_in_quad, ease_out_quad, AppState};
use crate::wanted::{handle_arrest_event_system, police_arrest_system, ArrestEvent, ArrestType};

use super::components::ChineseFont;
#[allow(clippy::wildcard_imports)]
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
fn detect_wasted_trigger(respawn_state: Res<RespawnState>, mut state: ResMut<ScreenEffectState>) {
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
    mut virtual_time: ResMut<Time<Virtual>>,
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

    // 用真實時間計時：慢動作只改遊戲時間（Time<Virtual>）的速度，真實時間不受影響
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
            state.time_scale = SLOW_MOTION_SCALE + (1.0 - SLOW_MOTION_SCALE) * ease_in_quad(t);
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

    // 慢動作：調整遊戲時間的速度。不能改用 TimeUpdateStrategy::ManualDuration，
    // 它連真實時間也一起換成那段時長，每幀的時間越縮越短、計時幾乎停住
    let speed = if state.phase == ScreenEffectPhase::Inactive {
        1.0
    } else {
        state.time_scale
    };
    virtual_time.set_relative_speed(speed);
}

/// 更新 UI 視覺效果（色調、文字、黑幕）
fn screen_effect_visual_update(
    state: Res<ScreenEffectState>,
    mut root_query: Query<&mut Visibility, With<ScreenEffectRoot>>,
    mut tint_query: Query<&mut BackgroundColor, With<ScreenEffectTint>>,
    mut text_query: Query<(&mut TextColor, &mut TextFont, &mut Text), With<ScreenEffectLabel>>,
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
            ScreenEffectPhase::Hold | ScreenEffectPhase::FadeToBlack => SCREEN_EFFECT_TINT_ALPHA,
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
            .add_systems(Startup, setup_screen_effect_ui.in_set(super::UiSetup))
            .add_systems(
                Update,
                screen_effect_systems().run_if(in_state(AppState::InGame)),
            );
    }
}

/// WASTED／BUSTED 的系統。狀態機排在 kill cam 之後：兩者都會設遊戲時間的速度，
/// kill cam 沒在播時每幀設回 1，WASTED 的慢動作要後設才算數
fn screen_effect_systems() -> ScheduleConfigs<ScheduleSystem> {
    (
        // 逮捕完成那一幀就要開始 BUSTED，處理逮捕才會等 BUSTED 播完（排在逮捕前面會處理兩次）
        detect_busted_trigger
            .after(police_arrest_system)
            .before(handle_arrest_event_system),
        detect_wasted_trigger,
        screen_effect_phase_machine
            .after(detect_wasted_trigger)
            .after(detect_busted_trigger)
            .after(killcam_update_system),
        screen_effect_visual_update.after(screen_effect_phase_machine),
    )
        .into_configs()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::{TimeUpdateStrategy, Virtual};

    use super::*;

    #[test]
    fn busted_detection_runs_between_the_arrest_and_its_processing() {
        // 逮捕完成那一幀送出逮捕事件：偵測 BUSTED 要排在逮捕之後、處理逮捕之前，同一幀就開始 BUSTED，
        // 處理逮捕才會等 BUSTED 播完；不然逮捕當幀就被處理，BUSTED 播完又處理一次
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems((crate::wanted::arrest_systems(), screen_effect_systems()));
        schedule.initialize(&mut world).expect("排得出來");
        let order: Vec<(bevy::ecs::schedule::SystemKey, String)> = schedule
            .systems()
            .expect("已經初始化")
            .map(|(key, system)| (key, system.name().to_string()))
            .collect();
        let find = |name: &str| {
            order
                .iter()
                .position(|(_, system)| system.ends_with(name))
                .unwrap_or_else(|| panic!("排程裡沒有 {name}"))
        };
        // 兩個系統之間有排序（不在「會互相影響卻沒排序」的清單裡），而且 first 排在前面
        let assert_runs_before = |first: usize, second: usize| {
            let unordered = schedule
                .graph()
                .conflicting_systems()
                .iter()
                .any(|(a, b, _)| {
                    let pair = [*a, *b];
                    pair.contains(&order[first].0) && pair.contains(&order[second].0)
                });
            let (first_name, second_name) = (&order[first].1, &order[second].1);
            assert!(!unordered, "{first_name} 和 {second_name} 之間沒有排序");
            assert!(first < second, "{second_name} 排在 {first_name} 之前");
        };
        let detect = find("detect_busted_trigger");
        assert_runs_before(find("police_arrest_system"), detect);
        assert_runs_before(detect, find("handle_arrest_event_system"));
    }

    #[test]
    fn wasted_finishes_and_triggers_respawn() {
        // 真實時間每幀走 0.1 秒（第一幀 0）：WASTED 要在效果總長 4.5 秒後播完、觸發重生；
        // 播放中遊戲時間變慢（kill cam 系統同時在跑、沒在播時每幀把速度設回 1），播完恢復正常速度
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                100,
            )))
            .init_resource::<ScreenEffectState>()
            .init_resource::<KillCamState>()
            .insert_resource(RespawnState {
                is_dead: true,
                respawn_timer: 3.0,
                death_position: Vec3::ZERO,
            })
            .add_message::<ArrestEvent>()
            .add_systems(Update, (killcam_update_system, screen_effect_systems()));

        // 約 1.9 秒：在慢動作維持階段（0.5～3.5 秒）
        for _ in 0..20 {
            app.update();
        }
        let speed_while_wasted = app.world().resource::<Time<Virtual>>().relative_speed();
        assert!(
            app.world()
                .resource::<ScreenEffectState>()
                .respawn_timer_frozen
        );
        // 約 4.4 秒：還沒到 4.5 秒，還在播
        for _ in 0..25 {
            app.update();
        }
        assert!(
            app.world()
                .resource::<ScreenEffectState>()
                .respawn_timer_frozen,
            "WASTED 提早播完"
        );
        // 約 5.9 秒
        for _ in 0..15 {
            app.update();
        }

        let state = app.world().resource::<ScreenEffectState>();
        assert!(!state.is_active(), "WASTED 沒播完");
        assert!(!state.respawn_timer_frozen);
        assert!(app.world().resource::<RespawnState>().respawn_timer <= 0.0);
        assert!(
            (speed_while_wasted - SLOW_MOTION_SCALE).abs() < 1e-4,
            "播放中要慢動作，遊戲時間速度 {speed_while_wasted}"
        );
        assert_eq!(
            app.world().resource::<Time<Virtual>>().relative_speed(),
            1.0
        );
    }
}
