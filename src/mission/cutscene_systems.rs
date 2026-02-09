//! 過場動畫系統邏輯
//!
//! 處理過場動畫播放、攝影機控制、淡入淡出等
#![allow(dead_code)]


use bevy::prelude::*;

use super::cutscene::*;
use super::story_data::CutsceneId;

/// 過場動畫系統 Plugin
pub struct CutsceneSystemPlugin;

impl Plugin for CutsceneSystemPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<CutsceneState>()
            .init_resource::<CutsceneDatabase>()
            // 事件
            .add_message::<CutsceneEvent>()
            // 系統
            .add_systems(Startup, setup_cutscene_ui)
            .add_systems(
                Update,
                (
                    cutscene_event_handler,
                    cutscene_playback_system,
                    cutscene_camera_system,
                    cutscene_timeline_system,
                    cutscene_fade_system,
                    cutscene_letterbox_system,
                    cutscene_skip_system,
                    cutscene_cleanup_system, // 清理系統放最後，等待淡出完成
                )
                    .chain(),
            );
    }
}

// ============================================================================
// UI 設置
// ============================================================================

/// 設置過場動畫 UI
fn setup_cutscene_ui(mut commands: Commands) {
    // 淡入淡出遮罩
    commands.spawn((
        FadeOverlay,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        Visibility::Hidden,
        ZIndex(100), // 最上層
    ));

    // 上方黑邊
    commands.spawn((
        LetterboxBar { is_top: true },
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(0.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        Visibility::Hidden,
        ZIndex(90),
    ));

    // 下方黑邊
    commands.spawn((
        LetterboxBar { is_top: false },
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(0.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        Visibility::Hidden,
        ZIndex(90),
    ));

    // 字幕容器
    commands.spawn((
        CutsceneSubtitle,
        Text::new(""),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(100.0),
            left: Val::Percent(50.0),
            ..default()
        },
        Visibility::Hidden,
        ZIndex(95),
    ));

    // 跳過提示
    commands.spawn((
        SkipPrompt,
        Text::new("長按 空白鍵 跳過"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgba(0.7, 0.7, 0.7, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(30.0),
            right: Val::Px(30.0),
            ..default()
        },
        Visibility::Hidden,
        ZIndex(95),
    ));
}

// ============================================================================
// 事件處理
// ============================================================================

// ============================================================================
// 過場動畫事件處理輔助函數
// ============================================================================
/// 處理過場動畫開始事件
fn handle_cutscene_start(
    cutscene_id: CutsceneId,
    cutscene_state: &mut CutsceneState,
    database: &CutsceneDatabase,
) {
    let Some(cutscene) = database.get(cutscene_id) else {
        warn!("找不到過場動畫: {}", cutscene_id);
        return;
    };

    cutscene_state.active_cutscene = Some(ActiveCutscene::new(cutscene_id));

    // 設置黑邊
    if cutscene.letterbox {
        cutscene_state.letterbox_visible = true;
    }

    // 設置淡入
    if cutscene.fade_in_on_start {
        cutscene_state.fade_state = FadeState::fade_in(1.0, Color::BLACK);
    }

    info!("🎬 過場動畫開始: {} (ID: {})", cutscene.name, cutscene_id);
}

/// 處理過場動畫跳過事件
fn handle_cutscene_skip(cutscene_state: &mut CutsceneState) {
    if let Some(active) = &mut cutscene_state.active_cutscene {
        active.completed = true;
        info!("過場動畫被跳過");
    }
}

/// 設定過場動畫暫停狀態
fn set_cutscene_paused(cutscene_state: &mut CutsceneState, paused: bool) {
    if let Some(active) = &mut cutscene_state.active_cutscene {
        active.paused = paused;
    }
}

/// 處理過場動畫事件
fn cutscene_event_handler(
    mut events: MessageReader<CutsceneEvent>,
    mut cutscene_state: ResMut<CutsceneState>,
    database: Res<CutsceneDatabase>,
) {
    for event in events.read() {
        match event {
            CutsceneEvent::Start(cutscene_id) => {
                handle_cutscene_start(*cutscene_id, &mut cutscene_state, &database);
            }
            CutsceneEvent::Skip => {
                handle_cutscene_skip(&mut cutscene_state);
            }
            CutsceneEvent::Pause => {
                set_cutscene_paused(&mut cutscene_state, true);
            }
            CutsceneEvent::Resume => {
                set_cutscene_paused(&mut cutscene_state, false);
            }
            CutsceneEvent::Completed(_) | CutsceneEvent::ExecuteAction(_) => {
                // 由其他系統處理
            }
        }
    }
}

// ============================================================================
// 播放系統
// ============================================================================

/// 過場動畫播放系統
fn cutscene_playback_system(
    mut cutscene_state: ResMut<CutsceneState>,
    database: Res<CutsceneDatabase>,
    time: Res<Time>,
    mut events: MessageWriter<CutsceneEvent>,
) {
    // 先檢查並取得需要的資訊，避免借用衝突
    let (should_complete, cutscene_id, should_fade_out) = {
        let Some(active) = &mut cutscene_state.active_cutscene else {
            return;
        };

        if active.completed || active.paused {
            return;
        }

        let Some(cutscene) = database.get(active.cutscene_id) else {
            return;
        };

        // 更新時間
        active.current_time += time.delta_secs();

        // 檢查是否完成
        if active.current_time >= cutscene.duration {
            active.completed = true;
            (true, active.cutscene_id, cutscene.fade_out_on_end)
        } else {
            (false, active.cutscene_id, false)
        }
    };

    // 在借用結束後設置淡出狀態
    if should_complete {
        if should_fade_out {
            cutscene_state.fade_state = FadeState::fade_out(0.5, Color::BLACK);
        }
        events.write(CutsceneEvent::Completed(cutscene_id));
        info!("🎬 過場動畫完成: {}", cutscene_id);
    }
}

/// 過場動畫完成清理
pub fn cutscene_cleanup_system(
    mut cutscene_state: ResMut<CutsceneState>,
    mut commands: Commands,
) {
    let Some(active) = &cutscene_state.active_cutscene else {
        return;
    };

    if !active.completed {
        return;
    }

    // 淡出完成後清理
    if cutscene_state.fade_state.active {
        return;
    }

    // 清理生成的實體
    for entity in &active.spawned_entities {
        if let Ok(mut cmd) = commands.get_entity(*entity) {
            cmd.despawn();
        }
    }

    // 隱藏黑邊
    cutscene_state.letterbox_visible = false;

    // 清除活動過場動畫
    cutscene_state.active_cutscene = None;
}

// ============================================================================
// 攝影機系統
// ============================================================================

/// 過場動畫攝影機系統
fn cutscene_camera_system(
    cutscene_state: Res<CutsceneState>,
    database: Res<CutsceneDatabase>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
) {
    let Some(active) = &cutscene_state.active_cutscene else {
        return;
    };

    let Some(cutscene) = database.get(active.cutscene_id) else {
        return;
    };

    let Some((position, target, fov)) = cutscene.interpolate_camera(active.current_time) else {
        return;
    };

    for (mut transform, mut projection) in &mut camera_query {
        // 更新位置和朝向
        transform.translation = position;
        transform.look_at(target, Vec3::Y);

        // 更新視野角度
        if let Projection::Perspective(ref mut persp) = *projection {
            persp.fov = fov.to_radians();
        }
    }
}

// ============================================================================
// 時間軸系統
// ============================================================================

/// 過場動畫時間軸系統
fn cutscene_timeline_system(
    mut cutscene_state: ResMut<CutsceneState>,
    database: Res<CutsceneDatabase>,
    mut subtitle_query: Query<(&mut Text, &mut Visibility), With<CutsceneSubtitle>>,
    mut action_events: MessageWriter<CutsceneEvent>,
) {
    // 先收集需要執行的動作，避免借用衝突
    let actions_to_execute: Vec<CutsceneAction> = {
        let Some(active) = &mut cutscene_state.active_cutscene else {
            return;
        };

        let Some(cutscene) = database.get(active.cutscene_id) else {
            return;
        };

        let mut actions = Vec::new();

        // 執行到達時間點的動作
        for (index, entry) in cutscene.timeline.iter().enumerate() {
            if active.executed_indices.contains(&index) {
                continue;
            }

            if entry.time <= active.current_time {
                // 標記為已執行
                active.executed_indices.push(index);
                actions.push(entry.action.clone());
            }
        }

        actions
    };

    // 在借用結束後執行動作
    for action in actions_to_execute {
        execute_cutscene_action(
            &action,
            &mut cutscene_state,
            &mut subtitle_query,
            &mut action_events,
        );
    }
}

/// 執行過場動畫動作
fn execute_cutscene_action(
    action: &CutsceneAction,
    cutscene_state: &mut ResMut<CutsceneState>,
    subtitle_query: &mut Query<(&mut Text, &mut Visibility), With<CutsceneSubtitle>>,
    _events: &mut MessageWriter<CutsceneEvent>,
) {
    match action {
        CutsceneAction::ShowSubtitle { text, duration: _ } => {
            if let Ok((mut subtitle_text, mut vis)) = subtitle_query.single_mut() {
                subtitle_text.0 = text.clone();
                *vis = Visibility::Visible;
            }
        }
        CutsceneAction::HideSubtitle => {
            if let Ok((_, mut vis)) = subtitle_query.single_mut() {
                *vis = Visibility::Hidden;
            }
        }
        CutsceneAction::FadeIn { duration, color } => {
            cutscene_state.fade_state = FadeState::fade_in(*duration, *color);
        }
        CutsceneAction::FadeOut { duration, color } => {
            cutscene_state.fade_state = FadeState::fade_out(*duration, *color);
        }
        CutsceneAction::ShowLetterbox => {
            cutscene_state.letterbox_visible = true;
        }
        CutsceneAction::HideLetterbox => {
            cutscene_state.letterbox_visible = false;
        }
        // 其他動作由專門的系統處理
        _ => {}
    }
}

// ============================================================================
// 淡入淡出系統
// ============================================================================

/// 淡入淡出系統
fn cutscene_fade_system(
    mut cutscene_state: ResMut<CutsceneState>,
    time: Res<Time>,
    mut fade_query: Query<(&mut BackgroundColor, &mut Visibility), With<FadeOverlay>>,
) {
    let Ok((mut bg_color, mut vis)) = fade_query.single_mut() else {
        return;
    };

    let fade = &mut cutscene_state.fade_state;

    if !fade.active {
        *vis = Visibility::Hidden;
        return;
    }

    // 更新進度
    if fade.duration > 0.0 {
        fade.progress += time.delta_secs() / fade.duration;
    } else {
        fade.progress = 1.0;
    }

    if fade.progress >= 1.0 {
        fade.progress = 1.0;
        fade.active = false;
    }

    // 計算當前不透明度
    let opacity = fade.current_opacity();

    if opacity > 0.0 {
        *vis = Visibility::Visible;
        *bg_color = BackgroundColor(fade.color.with_alpha(opacity));
    } else {
        *vis = Visibility::Hidden;
    }
}

// ============================================================================
// 黑邊系統
// ============================================================================

/// 黑邊（電影模式）系統
fn cutscene_letterbox_system(
    cutscene_state: Res<CutsceneState>,
    time: Res<Time>,
    mut letterbox_query: Query<(&LetterboxBar, &mut Node, &mut Visibility)>,
    mut current_height: Local<f32>,
) {
    let target_height = if cutscene_state.letterbox_visible {
        80.0 // 黑邊高度
    } else {
        0.0
    };

    // 平滑過渡
    let speed = 200.0; // 每秒像素
    if *current_height < target_height {
        *current_height = (*current_height + speed * time.delta_secs()).min(target_height);
    } else if *current_height > target_height {
        *current_height = (*current_height - speed * time.delta_secs()).max(target_height);
    }

    for (_bar, mut node, mut vis) in &mut letterbox_query {
        if *current_height > 0.0 {
            *vis = Visibility::Visible;
            node.height = Val::Px(*current_height);
        } else {
            *vis = Visibility::Hidden;
        }
    }
}

// ============================================================================
// 跳過系統
// ============================================================================

/// 跳過過場動畫系統
fn cutscene_skip_system(
    mut cutscene_state: ResMut<CutsceneState>,
    database: Res<CutsceneDatabase>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut skip_prompt_query: Query<&mut Visibility, With<SkipPrompt>>,
    mut events: MessageWriter<CutsceneEvent>,
) {
    let Some(active) = &mut cutscene_state.active_cutscene else {
        // 隱藏跳過提示
        if let Ok(mut vis) = skip_prompt_query.single_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    };

    let Some(cutscene) = database.get(active.cutscene_id) else {
        return;
    };

    // 顯示跳過提示
    if cutscene.skippable {
        if let Ok(mut vis) = skip_prompt_query.single_mut() {
            *vis = Visibility::Visible;
        }
    }

    // 處理跳過輸入
    if cutscene.skippable && keyboard.pressed(KeyCode::Space) {
        active.skip_hold_time += time.delta_secs();

        if active.skip_hold_time >= cutscene.skip_hold_time {
            events.write(CutsceneEvent::Skip);
        }
    } else {
        active.skip_hold_time = 0.0;
    }
}

// ============================================================================
// 便利函數
// ============================================================================

/// 開始過場動畫
pub fn start_cutscene(
    cutscene_id: CutsceneId,
    events: &mut MessageWriter<CutsceneEvent>,
) {
    events.write(CutsceneEvent::Start(cutscene_id));
}

/// 檢查是否有過場動畫進行中
pub fn is_cutscene_active(cutscene_state: &CutsceneState) -> bool {
    cutscene_state.active_cutscene.is_some()
}

/// 取得當前過場動畫進度（0.0 - 1.0）
pub fn get_cutscene_progress(cutscene_state: &CutsceneState, database: &CutsceneDatabase) -> f32 {
    let Some(active) = &cutscene_state.active_cutscene else {
        return 0.0;
    };

    let Some(cutscene) = database.get(active.cutscene_id) else {
        return 0.0;
    };

    if cutscene.duration > 0.0 {
        (active.current_time / cutscene.duration).clamp(0.0, 1.0)
    } else {
        1.0
    }
}
