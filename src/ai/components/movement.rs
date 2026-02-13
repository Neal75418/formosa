//! AI 移動和巡邏組件

use bevy::prelude::*;

/// 巡邏路徑組件
#[derive(Component, Debug)]
pub struct PatrolPath {
    /// 巡邏點列表
    pub waypoints: Vec<Vec3>,
    /// 當前目標點索引
    pub current_index: usize,
    /// 是否往返巡邏（否則循環）
    pub ping_pong: bool,
    /// 往返方向（true = 正向）
    pub forward: bool,
    /// 到達巡邏點後等待時間
    pub wait_time: f32,
    /// 當前等待計時器
    pub wait_timer: f32,
}

impl Default for PatrolPath {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            current_index: 0,
            ping_pong: false,
            forward: true,
            wait_time: 2.0,
            wait_timer: 0.0,
        }
    }
}

impl PatrolPath {
    /// 建立新實例
    pub fn new(waypoints: Vec<Vec3>) -> Self {
        Self {
            waypoints,
            ..default()
        }
    }

    /// 取得當前目標點
    pub fn current_waypoint(&self) -> Option<Vec3> {
        self.waypoints.get(self.current_index).copied()
    }

    /// 前進到下一個巡邏點
    pub fn advance(&mut self) {
        if self.waypoints.is_empty() {
            return;
        }

        if self.ping_pong {
            if self.forward {
                if self.current_index + 1 >= self.waypoints.len() {
                    self.forward = false;
                    self.current_index = self.current_index.saturating_sub(1);
                } else {
                    self.current_index += 1;
                }
            } else if self.current_index == 0 {
                self.forward = true;
                self.current_index = 1.min(self.waypoints.len() - 1);
            } else {
                self.current_index -= 1;
            }
        } else {
            self.current_index = (self.current_index + 1) % self.waypoints.len();
        }
    }
}

/// AI 移動組件
#[derive(Component, Debug)]
pub struct AiMovement {
    /// 行走速度
    pub walk_speed: f32,
    /// 跑步速度
    pub run_speed: f32,
    /// 是否正在跑步
    pub is_running: bool,
    /// 到達目標的距離閾值
    pub arrival_threshold: f32,
    /// 當前移動目標
    pub move_target: Option<Vec3>,
}

impl Default for AiMovement {
    fn default() -> Self {
        Self {
            walk_speed: 2.0,
            run_speed: 5.0,
            is_running: false,
            arrival_threshold: 1.0,
            move_target: None,
        }
    }
}

impl AiMovement {
    /// 取得目前速度
    pub fn current_speed(&self) -> f32 {
        if self.is_running {
            self.run_speed
        } else {
            self.walk_speed
        }
    }

    /// 檢查是否到達目標
    pub fn has_arrived(&self, current_pos: Vec3) -> bool {
        if let Some(target) = self.move_target {
            let arrival_threshold_sq = self.arrival_threshold * self.arrival_threshold;
            current_pos.distance_squared(target) <= arrival_threshold_sq
        } else {
            true
        }
    }
}
