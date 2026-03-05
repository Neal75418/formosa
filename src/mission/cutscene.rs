//! 過場動畫系統資料結構
//!
//! 定義過場動畫、攝影機路徑、演出指令等資料結構

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::story_data::{CutsceneId, NpcId};

/// 插值緩動類型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EasingType {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
}

// ============================================================================
// 緩動輔助函數
// ============================================================================
/// 二次/三次緩動 in-out 共用邏輯
#[inline]
fn ease_in_out_power(t: f32, power: i32) -> f32 {
    if t < 0.5 {
        (2.0_f32.powi(power - 1)) * t.powi(power)
    } else {
        1.0 - (-2.0 * t + 2.0).powi(power) / 2.0
    }
}

/// 指數緩動 in
#[inline]
#[allow(clippy::float_cmp)]
fn ease_in_expo(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else {
        2.0_f32.powf(10.0 * t - 10.0)
    }
}

/// 指數緩動 out
#[inline]
#[allow(clippy::float_cmp)]
fn ease_out_expo(t: f32) -> f32 {
    if t == 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

/// 指數緩動 in-out
#[inline]
#[allow(clippy::float_cmp)]
fn ease_in_out_expo(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else if t < 0.5 {
        2.0_f32.powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
    }
}

impl EasingType {
    /// 計算緩動值（輸入 0.0-1.0，輸出 0.0-1.0）
    pub fn evaluate(&self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::EaseIn | Self::EaseInQuad => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseOutQuad => 1.0 - (1.0 - t).powi(2),
            Self::EaseInOut | Self::EaseInOutQuad => ease_in_out_power(t, 2),
            Self::EaseInCubic => t * t * t,
            Self::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOutCubic => ease_in_out_power(t, 3),
            Self::EaseInExpo => ease_in_expo(t),
            Self::EaseOutExpo => ease_out_expo(t),
            Self::EaseInOutExpo => ease_in_out_expo(t),
        }
    }
}

/// 攝影機關鍵幀
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraKeyframe {
    /// 時間點（秒）
    pub time: f32,
    /// 攝影機位置
    pub position: Vec3,
    /// 攝影機目標點
    pub target: Vec3,
    /// 視野角度
    pub fov: f32,
    /// 插值緩動類型
    pub easing: EasingType,
}

impl Default for CameraKeyframe {
    fn default() -> Self {
        Self {
            time: 0.0,
            position: Vec3::new(0.0, 5.0, 10.0),
            target: Vec3::ZERO,
            fov: 60.0,
            easing: EasingType::EaseInOut,
        }
    }
}

impl CameraKeyframe {
    /// 創建新的攝影機關鍵幀
    pub fn new(time: f32, position: Vec3, target: Vec3) -> Self {
        Self {
            time,
            position,
            target,
            ..Default::default()
        }
    }

    /// 設置視野角度
    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    /// 設置緩動類型
    pub fn with_easing(mut self, easing: EasingType) -> Self {
        self.easing = easing;
        self
    }
}

/// 過場動畫指令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CutsceneAction {
    /// 無操作（佔位）
    None,
    /// 播放對話
    PlayDialogue { dialogue_id: u32 },
    /// 顯示字幕
    ShowSubtitle { text: String, duration: f32 },
    /// 隱藏字幕
    HideSubtitle,
    /// 播放音效
    PlaySound { sound: String, volume: f32 },
    /// 播放音樂
    PlayMusic { music: String, fade_in: f32 },
    /// 停止音樂
    StopMusic { fade_out: f32 },
    /// 生成 NPC
    SpawnNpc {
        npc_id: NpcId,
        position: Vec3,
        rotation: f32,
    },
    /// 移除 NPC
    DespawnNpc { npc_id: NpcId },
    /// NPC 移動到位置
    MoveNpc {
        npc_id: NpcId,
        target: Vec3,
        speed: f32,
    },
    /// NPC 播放動畫
    NpcAnimation { npc_id: NpcId, animation: String },
    /// NPC 面向目標
    NpcLookAt { npc_id: NpcId, target: Vec3 },
    /// 淡入
    FadeIn { duration: f32, color: Color },
    /// 淡出
    FadeOut { duration: f32, color: Color },
    /// 等待
    Wait { duration: f32 },
    /// 設置天氣
    SetWeather { weather: String, transition: f32 },
    /// 設置時間
    SetTime { hour: f32, transition: f32 },
    /// 生成特效
    SpawnEffect { effect: String, position: Vec3 },
    /// 搖晃攝影機
    CameraShake { intensity: f32, duration: f32 },
    /// 暫停遊戲時間
    PauseGameTime,
    /// 恢復遊戲時間
    ResumeGameTime,
    /// 顯示黑邊（電影模式）
    ShowLetterbox,
    /// 隱藏黑邊
    HideLetterbox,
    /// 設置劇情旗標
    SetStoryFlag { flag: String, value: bool },
    /// 解鎖任務
    UnlockMission { mission_id: u32 },
    /// 自定義事件
    CustomEvent { event_name: String, data: String },
}

/// 過場動畫時間軸條目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutsceneTimelineEntry {
    /// 觸發時間（秒）
    pub time: f32,
    /// 要執行的動作
    pub action: CutsceneAction,
    /// 是否已執行
    #[serde(skip)]
    pub executed: bool,
}

impl CutsceneTimelineEntry {
    /// 創建新的時間軸條目
    pub fn new(time: f32, action: CutsceneAction) -> Self {
        Self {
            time,
            action,
            executed: false,
        }
    }
}

/// 完整的過場動畫定義
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cutscene {
    /// 過場動畫 ID
    pub id: CutsceneId,
    /// 名稱（用於編輯器）
    pub name: String,
    /// 總時長（秒）
    pub duration: f32,
    /// 攝影機關鍵幀
    pub camera_keyframes: Vec<CameraKeyframe>,
    /// 時間軸動作
    pub timeline: Vec<CutsceneTimelineEntry>,
    /// 是否可跳過
    pub skippable: bool,
    /// 跳過前需要長按的時間（秒）
    pub skip_hold_time: f32,
    /// 是否顯示電影黑邊
    pub letterbox: bool,
    /// 是否在開始時淡入
    pub fade_in_on_start: bool,
    /// 是否在結束時淡出
    pub fade_out_on_end: bool,
}

impl Default for Cutscene {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            duration: 10.0,
            camera_keyframes: Vec::new(),
            timeline: Vec::new(),
            skippable: true,
            skip_hold_time: 1.5,
            letterbox: true,
            fade_in_on_start: true,
            fade_out_on_end: true,
        }
    }
}

impl Cutscene {
    /// 創建新的過場動畫
    pub fn new(id: CutsceneId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            ..Default::default()
        }
    }

    /// 設置時長
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// 添加攝影機關鍵幀
    pub fn add_keyframe(&mut self, keyframe: CameraKeyframe) {
        self.camera_keyframes.push(keyframe);
        // 保持按時間排序（使用 total_cmp 避免 NaN panic）
        self.camera_keyframes
            .sort_by(|a, b| a.time.total_cmp(&b.time));
    }

    /// 鏈式添加關鍵幀
    pub fn with_keyframe(mut self, keyframe: CameraKeyframe) -> Self {
        self.add_keyframe(keyframe);
        self
    }

    /// 添加時間軸動作
    pub fn add_action(&mut self, time: f32, action: CutsceneAction) {
        self.timeline.push(CutsceneTimelineEntry::new(time, action));
        // 保持按時間排序（使用 total_cmp 避免 NaN panic）
        self.timeline.sort_by(|a, b| a.time.total_cmp(&b.time));
    }

    /// 鏈式添加動作
    pub fn with_action(mut self, time: f32, action: CutsceneAction) -> Self {
        self.add_action(time, action);
        self
    }

    /// 設置是否可跳過
    pub fn with_skippable(mut self, skippable: bool) -> Self {
        self.skippable = skippable;
        self
    }

    /// 設置黑邊模式
    pub fn with_letterbox(mut self, letterbox: bool) -> Self {
        self.letterbox = letterbox;
        self
    }

    /// 計算指定時間的攝影機狀態
    pub fn interpolate_camera(&self, time: f32) -> Option<(Vec3, Vec3, f32)> {
        if self.camera_keyframes.is_empty() {
            return None;
        }

        // 找到當前時間所在的關鍵幀區間
        let mut prev_keyframe = &self.camera_keyframes[0];
        let mut next_keyframe = prev_keyframe;

        for keyframe in &self.camera_keyframes {
            if keyframe.time <= time {
                prev_keyframe = keyframe;
            } else {
                next_keyframe = keyframe;
                break;
            }
        }

        // 如果超過最後一幀，使用最後一幀
        if let Some(last) = self.camera_keyframes.last() {
            if time >= last.time {
                return Some((last.position, last.target, last.fov));
            }
        }

        // 如果在第一幀之前，使用第一幀
        if time <= self.camera_keyframes[0].time {
            let first = &self.camera_keyframes[0];
            return Some((first.position, first.target, first.fov));
        }

        // 計算插值 t 值
        let segment_duration = next_keyframe.time - prev_keyframe.time;
        let t = if segment_duration > 0.0 {
            ((time - prev_keyframe.time) / segment_duration).clamp(0.0, 1.0)
        } else {
            1.0
        };

        // 應用緩動
        let eased_t = next_keyframe.easing.evaluate(t);

        // 插值計算
        let position = prev_keyframe.position.lerp(next_keyframe.position, eased_t);
        let target = prev_keyframe.target.lerp(next_keyframe.target, eased_t);
        let fov = prev_keyframe.fov + (next_keyframe.fov - prev_keyframe.fov) * eased_t;

        Some((position, target, fov))
    }
}

/// 過場動畫系統狀態資源
#[derive(Resource, Default)]
pub struct CutsceneState {
    /// 當前進行中的過場動畫
    pub active_cutscene: Option<ActiveCutscene>,
    /// 淡入淡出狀態
    pub fade_state: FadeState,
    /// 是否顯示黑邊
    pub letterbox_visible: bool,
    /// 黑邊動畫進度（0.0-1.0）
    pub letterbox_progress: f32,
}

/// 進行中的過場動畫
#[derive(Debug, Clone)]
pub struct ActiveCutscene {
    /// 過場動畫 ID
    pub cutscene_id: CutsceneId,
    /// 當前播放時間
    pub current_time: f32,
    /// 是否已完成
    pub completed: bool,
    /// 跳過按鈕按住時間
    pub skip_hold_time: f32,
    /// 已執行的時間軸索引
    pub executed_indices: Vec<usize>,
    /// 生成的臨時實體
    pub spawned_entities: Vec<Entity>,
    /// 是否暫停
    pub paused: bool,
}

impl ActiveCutscene {
    /// 創建新的進行中過場動畫
    pub fn new(cutscene_id: CutsceneId) -> Self {
        Self {
            cutscene_id,
            current_time: 0.0,
            completed: false,
            skip_hold_time: 0.0,
            executed_indices: Vec::new(),
            spawned_entities: Vec::new(),
            paused: false,
        }
    }
}

/// 淡入淡出狀態
#[derive(Debug, Clone, Default)]
pub struct FadeState {
    /// 是否正在淡入淡出
    pub active: bool,
    /// 是否是淡入（否則淡出）
    pub fading_in: bool,
    /// 淡入淡出進度（0.0-1.0）
    pub progress: f32,
    /// 淡入淡出持續時間
    pub duration: f32,
    /// 淡入淡出顏色
    pub color: Color,
}

impl FadeState {
    /// 開始淡入
    pub fn fade_in(duration: f32, color: Color) -> Self {
        Self {
            active: true,
            fading_in: true,
            progress: 0.0,
            duration,
            color,
        }
    }

    /// 開始淡出
    pub fn fade_out(duration: f32, color: Color) -> Self {
        Self {
            active: true,
            fading_in: false,
            progress: 0.0,
            duration,
            color,
        }
    }

    /// 計算當前不透明度
    pub fn current_opacity(&self) -> f32 {
        if !self.active {
            return 0.0;
        }

        if self.fading_in {
            1.0 - self.progress // 淡入：從不透明到透明
        } else {
            self.progress // 淡出：從透明到不透明
        }
    }
}

/// 過場動畫事件
#[derive(Message, Debug, Clone)]
pub enum CutsceneEvent {
    /// 開始過場動畫
    Start(CutsceneId),
    /// 跳過過場動畫
    Skip,
    /// 暫停過場動畫
    Pause,
    /// 恢復過場動畫
    Resume,
    /// 過場動畫完成
    Completed(CutsceneId),
    /// 執行過場動畫動作
    ExecuteAction(CutsceneAction),
}

/// 過場動畫資料庫資源
#[derive(Resource, Default)]
pub struct CutsceneDatabase {
    /// 所有過場動畫
    pub cutscenes: HashMap<CutsceneId, Cutscene>,
}

impl CutsceneDatabase {
    /// 註冊過場動畫
    pub fn register(&mut self, cutscene: Cutscene) {
        self.cutscenes.insert(cutscene.id, cutscene);
    }

    /// 取得過場動畫
    pub fn get(&self, id: CutsceneId) -> Option<&Cutscene> {
        self.cutscenes.get(&id)
    }
}

/// 過場動畫 UI 組件標記
#[derive(Component)]
pub struct CutsceneUIRoot;

/// 黑邊組件
#[derive(Component)]
pub struct LetterboxBar {
    /// 是否為上方黑邊
    pub is_top: bool,
}

/// 淡入淡出遮罩組件
#[derive(Component)]
pub struct FadeOverlay;

/// 字幕組件
#[derive(Component)]
pub struct CutsceneSubtitle;

/// 跳過提示組件
#[derive(Component)]
pub struct SkipPrompt;

// ============================================================================
// 範例過場動畫建構
// ============================================================================

/// 創建範例過場動畫（用於測試）
pub fn create_sample_cutscene() -> Cutscene {
    Cutscene::new(1, "第一章開場")
        .with_duration(15.0)
        // 攝影機關鍵幀
        .with_keyframe(
            CameraKeyframe::new(0.0, Vec3::new(100.0, 50.0, 100.0), Vec3::new(0.0, 0.0, 0.0))
                .with_fov(45.0),
        )
        .with_keyframe(
            CameraKeyframe::new(5.0, Vec3::new(50.0, 20.0, 50.0), Vec3::new(0.0, 5.0, 0.0))
                .with_fov(60.0)
                .with_easing(EasingType::EaseInOut),
        )
        .with_keyframe(
            CameraKeyframe::new(10.0, Vec3::new(10.0, 5.0, 10.0), Vec3::new(0.0, 2.0, 0.0))
                .with_fov(75.0)
                .with_easing(EasingType::EaseOutCubic),
        )
        .with_keyframe(
            CameraKeyframe::new(15.0, Vec3::new(5.0, 3.0, 5.0), Vec3::new(0.0, 1.5, 0.0))
                .with_fov(60.0)
                .with_easing(EasingType::EaseInOutCubic),
        )
        // 時間軸動作
        .with_action(
            0.0,
            CutsceneAction::FadeIn {
                duration: 2.0,
                color: Color::BLACK,
            },
        )
        .with_action(
            0.5,
            CutsceneAction::PlayMusic {
                music: "bgm_intro.ogg".to_string(),
                fade_in: 1.0,
            },
        )
        .with_action(
            2.0,
            CutsceneAction::ShowSubtitle {
                text: "熱帶島嶼「天堂灣」".to_string(),
                duration: 3.0,
            },
        )
        .with_action(5.0, CutsceneAction::HideSubtitle)
        .with_action(
            6.0,
            CutsceneAction::SpawnNpc {
                npc_id: 100,
                position: Vec3::new(0.0, 0.0, 0.0),
                rotation: 0.0,
            },
        )
        .with_action(
            7.0,
            CutsceneAction::ShowSubtitle {
                text: "這裡，機會與危險並存...".to_string(),
                duration: 3.0,
            },
        )
        .with_action(10.0, CutsceneAction::HideSubtitle)
        .with_action(
            12.0,
            CutsceneAction::ShowSubtitle {
                text: "而你，將改變一切".to_string(),
                duration: 2.5,
            },
        )
        .with_action(
            14.5,
            CutsceneAction::FadeOut {
                duration: 0.5,
                color: Color::BLACK,
            },
        )
        .with_skippable(true)
        .with_letterbox(true)
}
