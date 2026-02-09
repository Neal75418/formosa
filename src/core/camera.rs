//! 攝影機相關資源
#![allow(dead_code)]

use bevy::prelude::*;

/// 第三人稱攝影機跟隨目標
#[derive(Component)]
pub struct ThirdPersonCameraTarget;

/// 攝影機設定
#[derive(Resource)]
pub struct CameraSettings {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub sensitivity: f32,
    // 瞄準模式參數
    pub aim_shoulder_offset: f32,  // 過肩偏移（正值=右肩）
    pub aim_distance: f32,         // 瞄準時攝影機距離
    pub aim_pitch: f32,            // 瞄準時俯仰角
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.5,           // 初始俯視角度稍高
            distance: 18.0,       // 距離稍近
            sensitivity: 0.006,   // 提高靈敏度
            // 瞄準模式
            aim_shoulder_offset: 1.5,  // 向右肩偏移 1.5 公尺
            aim_distance: 8.0,         // 瞄準時拉近攝影機
            aim_pitch: 0.2,            // 瞄準時降低俯角
        }
    }
}

/// 後座力狀態（影響攝影機抖動）
#[derive(Resource, Default)]
pub struct RecoilState {
    /// 當前累積的後座力偏移（X=水平，Y=垂直）
    pub current_offset: Vec2,
    /// 是否正在恢復中
    pub is_recovering: bool,
}

impl RecoilState {
    /// 垂直後座力最大值
    const MAX_VERTICAL_RECOIL: f32 = 0.5;
    /// 水平後座力最大值
    const MAX_HORIZONTAL_RECOIL: f32 = 0.3;
    /// 後座力恢復完成閾值（小於此值視為歸零）
    const RECOVERY_THRESHOLD_SQ: f32 = 0.0001;

    /// 添加後座力
    pub fn add_recoil(&mut self, vertical: f32, horizontal: f32) {
        // 垂直後座力累加
        self.current_offset.y += vertical;
        // 水平後座力隨機左右偏移
        let h_dir = if rand::random::<bool>() { 1.0 } else { -1.0 };
        self.current_offset.x += horizontal * h_dir;
        // 限制最大後座力
        self.current_offset.y = self.current_offset.y.min(Self::MAX_VERTICAL_RECOIL);
        self.current_offset.x = self.current_offset.x.clamp(-Self::MAX_HORIZONTAL_RECOIL, Self::MAX_HORIZONTAL_RECOIL);
        self.is_recovering = false;
    }

    /// 更新後座力恢復
    pub fn update_recovery(&mut self, recovery_rate: f32, dt: f32) {
        if self.current_offset.length_squared() < Self::RECOVERY_THRESHOLD_SQ {
            self.current_offset = Vec2::ZERO;
            return;
        }

        self.is_recovering = true;
        // 平滑恢復到零點
        let recovery = recovery_rate * dt;
        self.current_offset.y = (self.current_offset.y - recovery).max(0.0);
        self.current_offset.x *= 1.0 - recovery * 2.0;
    }
}

/// 攝影機震動狀態
#[derive(Resource, Default)]
pub struct CameraShake {
    /// 震動強度
    pub intensity: f32,
    /// 震動持續時間
    pub duration: f32,
    /// 剩餘時間
    pub timer: f32,
}

impl CameraShake {
    /// 觸發攝影機震動
    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        // 如果新震動更強，覆蓋舊的
        if intensity > self.intensity || self.timer <= 0.0 {
            self.intensity = intensity;
            self.duration = duration;
            self.timer = duration;
        }
    }

    /// 取得當前震動偏移
    pub fn get_offset(&self, time: f32) -> Vec3 {
        if self.timer <= 0.0 {
            return Vec3::ZERO;
        }

        let progress = self.timer / self.duration;
        let decay = progress * progress; // 平方衰減更自然

        // 使用高頻正弦波產生震動
        let shake_x = (time * 50.0).sin() * self.intensity * decay;
        let shake_y = (time * 60.0).cos() * self.intensity * decay * 0.5;
        let shake_z = (time * 40.0).sin() * self.intensity * decay * 0.3;

        Vec3::new(shake_x, shake_y, shake_z)
    }

    /// 更新震動計時器
    pub fn update(&mut self, dt: f32) {
        if self.timer > 0.0 {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.intensity = 0.0;
            }
        }
    }
}
