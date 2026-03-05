//! 對話動作系統（GTA 5 風格）
//!
//! 處理對話期間的攝影機聚焦、動畫、特效等動作

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

use super::dialogue::{DialogueAction, DialogueDatabase, DialogueState};
use super::story_data::DialogueId;

// ============================================================================
// 對話動作事件與狀態
// ============================================================================

/// 對話動作事件（用於跨系統觸發）
#[derive(Message, Debug, Clone)]
pub enum DialogueActionEvent {
    /// 攝影機聚焦（GTA 風格對話特寫）
    CameraFocus { target: Vec3, duration: f32 },
    /// 播放動畫
    PlayAnimation { target_npc: u32, animation: String },
    /// 生成特效
    SpawnEffect { effect: String, position: Vec3 },
    /// 播放音效
    PlaySound(String),
    /// 設置天氣
    SetWeather(String),
    /// 設置時間
    SetTime(f32),
}

/// 對話動作執行狀態
#[derive(Resource, Default)]
pub struct DialogueActionState {
    /// 當前攝影機聚焦目標
    pub camera_focus: Option<CameraFocusData>,
    /// 等待計時器
    pub wait_timer: f32,
    /// 是否正在等待
    pub is_waiting: bool,
    /// 上一個處理的節點 ID（用於檢測節點變化）
    last_processed_node: Option<(DialogueId, u32)>,
}

/// 攝影機聚焦資料
#[derive(Clone)]
pub struct CameraFocusData {
    /// 目標位置
    pub target: Vec3,
    /// 持續時間
    pub duration: f32,
    /// 已經過時間
    pub elapsed: f32,
    /// 原始攝影機位置
    pub original_position: Option<Vec3>,
}

// ============================================================================
// 對話動作觸發系統
// ============================================================================

/// 觸發對話節點動作
pub fn dialogue_action_trigger_system(
    dialogue_state: Res<DialogueState>,
    database: Res<DialogueDatabase>,
    mut action_state: ResMut<DialogueActionState>,
    mut action_events: MessageWriter<DialogueActionEvent>,
) {
    let Some(active) = &dialogue_state.active_dialogue else {
        // 對話結束，清理狀態
        action_state.last_processed_node = None;
        action_state.camera_focus = None;
        action_state.is_waiting = false;
        return;
    };

    let current_key = (active.dialogue_id, active.current_node);

    // 檢查是否已處理過這個節點
    if action_state.last_processed_node == Some(current_key) {
        return;
    }

    // 標記為已處理
    action_state.last_processed_node = Some(current_key);

    // 取得節點
    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 執行節點的所有動作
    for action in &node.actions {
        match action {
            DialogueAction::None => {}
            DialogueAction::CameraFocus { target, duration } => {
                action_events.write(DialogueActionEvent::CameraFocus {
                    target: *target,
                    duration: *duration,
                });
                // 設置攝影機聚焦狀態
                action_state.camera_focus = Some(CameraFocusData {
                    target: *target,
                    duration: *duration,
                    elapsed: 0.0,
                    original_position: None,
                });
                info!("對話動作: 攝影機聚焦 {:?}", target);
            }
            DialogueAction::PlayAnimation { target, animation } => {
                action_events.write(DialogueActionEvent::PlayAnimation {
                    target_npc: *target,
                    animation: animation.clone(),
                });
                info!("對話動作: 播放動畫 {} on NPC {}", animation, target);
            }
            DialogueAction::SpawnEffect { effect, position } => {
                action_events.write(DialogueActionEvent::SpawnEffect {
                    effect: effect.clone(),
                    position: *position,
                });
                info!("對話動作: 生成特效 {} at {:?}", effect, position);
            }
            DialogueAction::PlaySound(sound) => {
                action_events.write(DialogueActionEvent::PlaySound(sound.clone()));
                info!("對話動作: 播放音效 {}", sound);
            }
            DialogueAction::Wait(duration) => {
                action_state.is_waiting = true;
                action_state.wait_timer = *duration;
                info!("對話動作: 等待 {} 秒", duration);
            }
            DialogueAction::SetWeather(weather) => {
                action_events.write(DialogueActionEvent::SetWeather(weather.clone()));
                info!("對話動作: 設置天氣 {}", weather);
            }
            DialogueAction::SetTime(time) => {
                action_events.write(DialogueActionEvent::SetTime(*time));
                info!("對話動作: 設置時間 {}", time);
            }
        }
    }
}

// ============================================================================
// 對話動作執行系統
// ============================================================================

/// 執行對話動作（攝影機聚焦等）
pub fn dialogue_action_executor_system(
    mut action_state: ResMut<DialogueActionState>,
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
) {
    // 處理等待計時器
    if action_state.is_waiting {
        action_state.wait_timer -= time.delta_secs();
        if action_state.wait_timer <= 0.0 {
            action_state.is_waiting = false;
            action_state.wait_timer = 0.0;
        }
    }

    // 處理攝影機聚焦（GTA 5 風格平滑過渡）
    if let Some(focus) = &mut action_state.camera_focus {
        focus.elapsed += time.delta_secs();

        for mut camera_transform in &mut camera_query {
            // 記錄原始位置
            if focus.original_position.is_none() {
                focus.original_position = Some(camera_transform.translation);
            }

            let progress = (focus.elapsed / focus.duration).clamp(0.0, 1.0);

            // 使用 ease-out 曲線平滑過渡（GTA 風格）
            let eased_progress = 1.0 - (1.0 - progress).powi(3);

            if let Some(original) = focus.original_position {
                // 計算聚焦位置（在目標前方偏移以獲得好的視角）
                let focus_offset = Vec3::new(0.0, 2.0, 5.0);
                let focus_position = focus.target + focus_offset;

                // 插值攝影機位置
                camera_transform.translation = original.lerp(focus_position, eased_progress);

                // 讓攝影機看向目標
                camera_transform.look_at(focus.target + Vec3::Y * 1.5, Vec3::Y);
            }
        }

        // 聚焦完成後清理
        if focus.elapsed >= focus.duration {
            action_state.camera_focus = None;
        }
    }
}
