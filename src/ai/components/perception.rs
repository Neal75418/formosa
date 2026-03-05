//! AI 感知系統

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use crate::core::{clamp_dot, safe_normalize};
use bevy::prelude::*;

/// AI 感知組件（視覺、聽覺）
#[derive(Component, Debug)]
pub struct AiPerception {
    /// 視野角度（度）
    pub fov: f32,
    /// 視野距離
    pub sight_range: f32,
    /// 聽覺距離
    pub hearing_range: f32,
    /// 是否能看到目標
    pub can_see_target: bool,
    /// 是否聽到聲音
    pub has_heard_noise: bool,
    /// 聽到聲音的位置
    pub noise_position: Option<Vec3>,
}

impl Default for AiPerception {
    fn default() -> Self {
        Self {
            fov: 60.0,           // 60 度視野
            sight_range: 30.0,   // 30 公尺視距
            hearing_range: 50.0, // 50 公尺聽力
            can_see_target: false,
            has_heard_noise: false,
            noise_position: None,
        }
    }
}

impl AiPerception {
    /// 設定感知範圍（視距與聽力距離）
    pub fn with_range(mut self, sight: f32, hearing: f32) -> Self {
        self.sight_range = sight;
        self.hearing_range = hearing;
        self
    }

    /// 設定視野角度
    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    /// 檢查目標是否在視野內
    pub fn is_in_fov(&self, my_pos: Vec3, my_forward: Vec3, target_pos: Vec3) -> bool {
        let to_target = safe_normalize(target_pos - my_pos);
        let forward = safe_normalize(my_forward);
        let dot = clamp_dot(forward.dot(to_target));
        let angle = dot.acos().to_degrees();
        angle <= self.fov / 2.0
    }

    /// 檢查目標是否在視距內
    pub fn is_in_sight_range(&self, my_pos: Vec3, target_pos: Vec3) -> bool {
        let sight_range_sq = self.sight_range * self.sight_range;
        my_pos.distance_squared(target_pos) <= sight_range_sq
    }

    /// 檢查聲音是否在聽力範圍內
    pub fn is_in_hearing_range(&self, my_pos: Vec3, sound_pos: Vec3) -> bool {
        let hearing_range_sq = self.hearing_range * self.hearing_range;
        my_pos.distance_squared(sound_pos) <= hearing_range_sq
    }
}
