//! Kill Cam 系統 (GTA 5 風格死亡慢鏡頭)
//!
//! 在特定擊殺（爆頭、最後敵人、連殺）時觸發慢動作效果。

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use crate::core::{ease_in_quad, ease_out_quad};
use bevy::prelude::*;

// ============================================================================
// Kill Cam 常數
// ============================================================================

/// 最小持續時間（秒）- 防止除以零
const MIN_DURATION: f32 = 0.1;
/// 最小時間縮放：慢動作再慢，遊戲時間的速度也不能變成 0（會凍住）
const MIN_TIME_SCALE: f32 = 0.01;

// --- 階段轉換點 ---
/// 緩入階段結束點（前 20% 用於減速）
const PHASE_EASE_IN_END: f32 = 0.2;
/// 緩出階段開始點（後 20% 用於加速恢復）
const PHASE_EASE_OUT_START: f32 = 0.8;

// --- 爆頭擊殺參數 ---
const HEADSHOT_TIME_SCALE: f32 = 0.2; // 5 倍慢動作
const HEADSHOT_DURATION: f32 = 1.5; // 持續 1.5 秒
const HEADSHOT_ZOOM: f32 = 2.0; // 2 倍縮放

// --- 最後敵人參數 ---
const LAST_ENEMY_TIME_SCALE: f32 = 0.15; // 6.7 倍慢動作
const LAST_ENEMY_DURATION: f32 = 2.0; // 持續 2 秒
const LAST_ENEMY_ZOOM: f32 = 2.5; // 2.5 倍縮放

// --- 遠距離擊殺參數 ---
const LONG_RANGE_TIME_SCALE: f32 = 0.25; // 4 倍慢動作
const LONG_RANGE_DURATION: f32 = 1.2; // 持續 1.2 秒
const LONG_RANGE_ZOOM: f32 = 1.8; // 1.8 倍縮放

// --- 連殺參數 ---
const MULTI_KILL_BASE_TIME_SCALE: f32 = 0.3; // 基礎時間縮放
const MULTI_KILL_SCALE_PER_KILL: f32 = 0.05; // 每次連殺額外減速
const MULTI_KILL_MIN_TIME_SCALE: f32 = 0.1; // 最慢時間縮放
const MULTI_KILL_BASE_DURATION: f32 = 1.0; // 基礎持續時間
const MULTI_KILL_DURATION_PER_KILL: f32 = 0.3; // 每次連殺額外時間
const MULTI_KILL_BASE_ZOOM: f32 = 1.5; // 基礎縮放
const MULTI_KILL_ZOOM_PER_KILL: f32 = 0.2; // 每次連殺額外縮放

// ============================================================================
// Kill Cam 狀態
// ============================================================================

/// Kill Cam 觸發類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KillCamTrigger {
    /// 爆頭擊殺
    Headshot,
    /// 最後一個敵人
    LastEnemy,
    /// 連殺（3+ 在短時間內）
    MultiKill(u8),
    /// 遠距離擊殺
    LongRange,
}

/// Kill Cam 資源
/// 控制全局慢動作效果
#[derive(Resource, Debug)]
pub struct KillCamState {
    /// 是否啟用
    pub active: bool,
    /// 觸發類型
    pub trigger: Option<KillCamTrigger>,
    /// 目標實體（被擊殺者）
    pub target_entity: Option<Entity>,
    /// 目標位置
    pub target_position: Option<Vec3>,
    /// 當前時間縮放 (1.0 = 正常, 0.1 = 十倍慢)
    pub time_scale: f32,
    /// 目標時間縮放
    pub target_time_scale: f32,
    /// 持續時間
    pub duration: f32,
    /// 已經過時間
    pub elapsed: f32,
    /// 攝影機縮放目標
    pub camera_zoom: f32,
    /// 連殺計數
    pub kill_streak: u8,
    /// 最後擊殺時間
    pub last_kill_time: f32,
    /// 連殺時間窗口（秒）
    pub kill_streak_window: f32,
}

impl Default for KillCamState {
    fn default() -> Self {
        Self {
            active: false,
            trigger: None,
            target_entity: None,
            target_position: None,
            time_scale: 1.0,
            target_time_scale: 1.0,
            duration: 0.0,
            elapsed: 0.0,
            camera_zoom: 1.0,
            kill_streak: 0,
            last_kill_time: 0.0,
            kill_streak_window: 3.0, // 3 秒內連續擊殺算連殺
        }
    }
}

impl KillCamState {
    /// 觸發 Kill Cam
    pub fn trigger(
        &mut self,
        trigger_type: KillCamTrigger,
        target_entity: Entity,
        target_position: Vec3,
        current_time: f32,
    ) {
        // 如果已經在 Kill Cam 中，不重複觸發
        if self.active {
            return;
        }

        // 設定效果參數（使用常數定義）
        let (time_scale, duration, zoom) = match trigger_type {
            KillCamTrigger::Headshot => (HEADSHOT_TIME_SCALE, HEADSHOT_DURATION, HEADSHOT_ZOOM),
            KillCamTrigger::LastEnemy => {
                (LAST_ENEMY_TIME_SCALE, LAST_ENEMY_DURATION, LAST_ENEMY_ZOOM)
            }
            KillCamTrigger::MultiKill(count) => {
                let scale = (MULTI_KILL_BASE_TIME_SCALE
                    - f32::from(count) * MULTI_KILL_SCALE_PER_KILL)
                    .max(MULTI_KILL_MIN_TIME_SCALE);
                let dur =
                    MULTI_KILL_BASE_DURATION + f32::from(count) * MULTI_KILL_DURATION_PER_KILL;
                let z = MULTI_KILL_BASE_ZOOM + f32::from(count) * MULTI_KILL_ZOOM_PER_KILL;
                (scale, dur, z)
            }
            KillCamTrigger::LongRange => {
                (LONG_RANGE_TIME_SCALE, LONG_RANGE_DURATION, LONG_RANGE_ZOOM)
            }
        };

        // 持續時間不為零（進度要除以它）；時間縮放不為零（遊戲時間速度為 0 會凍住）
        let safe_duration = duration.max(MIN_DURATION);
        let safe_time_scale = time_scale.max(MIN_TIME_SCALE);

        self.active = true;
        self.trigger = Some(trigger_type);
        self.target_entity = Some(target_entity);
        self.target_position = Some(target_position);
        self.target_time_scale = safe_time_scale;
        self.duration = safe_duration;
        self.elapsed = 0.0;
        self.camera_zoom = zoom;
        self.last_kill_time = current_time;
    }

    /// 記錄擊殺（用於連殺計數）
    pub fn record_kill(&mut self, current_time: f32) {
        if current_time - self.last_kill_time < self.kill_streak_window {
            self.kill_streak += 1;
        } else {
            self.kill_streak = 1;
        }
        self.last_kill_time = current_time;
    }

    /// 重置連殺
    pub fn reset_kill_streak(&mut self) {
        self.kill_streak = 0;
    }

    /// 取得連殺數
    pub fn get_kill_streak(&self) -> u8 {
        self.kill_streak
    }

    /// 是否應該觸發連殺 Kill Cam
    pub fn should_trigger_multi_kill(&self) -> bool {
        self.kill_streak >= 3
    }

    /// 更新 Kill Cam 狀態；dt 是真實時間（不受慢動作影響）
    pub fn update(&mut self, dt: f32) {
        if !self.active {
            self.time_scale = 1.0;
            return;
        }

        self.elapsed += dt;

        // 安全檢查：確保 duration 不為零
        let safe_duration = self.duration.max(MIN_DURATION);

        // 計算時間縮放進度
        let progress = self.elapsed / safe_duration;

        if progress >= 1.0 {
            // Kill Cam 結束
            self.active = false;
            self.trigger = None;
            self.target_entity = None;
            self.target_position = None;
            self.time_scale = 1.0;
            self.camera_zoom = 1.0;
        } else {
            // 平滑過渡時間縮放
            // 前 PHASE_EASE_IN_END (20%)：快速減慢
            // 中間 60%：保持慢動作
            // 後 (1.0 - PHASE_EASE_OUT_START) (20%)：恢復正常
            if progress < PHASE_EASE_IN_END {
                let t = progress / PHASE_EASE_IN_END;
                self.time_scale = 1.0 - (1.0 - self.target_time_scale) * ease_out_quad(t);
            } else if progress < PHASE_EASE_OUT_START {
                self.time_scale = self.target_time_scale;
            } else {
                let t = (progress - PHASE_EASE_OUT_START) / (1.0 - PHASE_EASE_OUT_START);
                self.time_scale =
                    self.target_time_scale + (1.0 - self.target_time_scale) * ease_in_quad(t);
            }
        }
    }

    /// 取得當前攝影機縮放
    pub fn get_camera_zoom(&self) -> f32 {
        if !self.active {
            return 1.0;
        }

        // 安全檢查：確保 duration 不為零
        let safe_duration = self.duration.max(MIN_DURATION);
        let progress = self.elapsed / safe_duration;

        // 縮放曲線：快速縮放然後慢慢恢復
        if progress < PHASE_EASE_IN_END {
            let t = progress / PHASE_EASE_IN_END;
            1.0 + (self.camera_zoom - 1.0) * ease_out_quad(t)
        } else if progress < PHASE_EASE_OUT_START {
            self.camera_zoom
        } else {
            let t = (progress - PHASE_EASE_OUT_START) / (1.0 - PHASE_EASE_OUT_START);
            self.camera_zoom - (self.camera_zoom - 1.0) * ease_in_quad(t)
        }
    }
}

// ============================================================================
// 輔助函數
// ============================================================================

// ============================================================================
// 系統
// ============================================================================

/// Kill Cam 更新系統：用真實時間計時，慢動作只調整遊戲時間（Time<Virtual>）的速度。
/// 不能改用 TimeUpdateStrategy::ManualDuration：它連真實時間也一起換成那段時長，
/// 每幀的時間越縮越短，遊戲時間停住、kill cam 永遠不結束
pub fn killcam_update_system(
    real_time: Res<Time<Real>>,
    mut killcam: ResMut<KillCamState>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    killcam.update(real_time.delta_secs());
    // 沒在播時 update 已把 time_scale 設回 1
    virtual_time.set_relative_speed(killcam.time_scale);
}

/// Kill Cam 視覺效果系統
/// 添加慢動作時的視覺效果（模糊、暗角等）
pub fn killcam_visual_system(
    killcam: Res<KillCamState>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    // 如果 Kill Cam 啟用，微調攝影機
    if killcam.active {
        if let Some(target_pos) = killcam.target_position {
            for mut transform in &mut query {
                // 讓攝影機稍微看向目標
                let current_forward = transform.forward().as_vec3();
                let to_target = (target_pos - transform.translation).normalize_or_zero();

                // 緩慢轉向目標（每幀 1%）
                let blend = 0.01;
                let new_forward = current_forward.lerp(to_target, blend);
                if new_forward.length_squared() > 0.001 {
                    transform.look_to(Dir3::new(new_forward).unwrap_or(Dir3::NEG_Z), Vec3::Y);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::{TimeUpdateStrategy, Virtual};

    use super::*;

    #[test]
    fn headshot_kill_cam_slows_time_then_ends() {
        // 真實時間每幀走 0.1 秒：爆頭 kill cam（1.5 秒、0.2 倍）期間遊戲時間變慢但仍在走，
        // 時間到就結束、恢復正常速度，不能把遊戲時間凍住
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                100,
            )))
            .init_resource::<KillCamState>()
            .add_systems(Update, killcam_update_system);
        app.update();
        let target = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<KillCamState>().trigger(
            KillCamTrigger::Headshot,
            target,
            Vec3::ZERO,
            0.0,
        );

        // 0.8 秒：在慢動作中段
        for _ in 0..8 {
            app.update();
        }
        let time = app.world().resource::<Time<Virtual>>();
        let (speed, game_dt) = (time.relative_speed(), time.delta_secs());
        // 1.4 秒：還沒到 1.5 秒，還在播
        for _ in 0..6 {
            app.update();
        }
        assert!(
            app.world().resource::<KillCamState>().active,
            "kill cam 提早結束"
        );
        // 2.0 秒
        for _ in 0..6 {
            app.update();
        }

        assert!(
            !app.world().resource::<KillCamState>().active,
            "kill cam 沒結束"
        );
        assert!(
            game_dt > 0.01,
            "慢動作中遊戲時間要繼續走，這一幀只走了 {game_dt} 秒"
        );
        assert!(speed < 0.5, "慢動作中遊戲時間速度 {speed}");
        assert_eq!(
            app.world().resource::<Time<Virtual>>().relative_speed(),
            1.0
        );
    }
}
