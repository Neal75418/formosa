//! 任務經濟獎勵系統
#![allow(dead_code)]


use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// 聲望管理器
#[derive(Resource, Serialize, Deserialize, Default, Debug, Clone)]
pub struct RespectManager {
    /// 玩家聲望
    pub respect: i32,
}

impl RespectManager {
    /// 創建新的聲望管理器
    pub fn new() -> Self {
        Self { respect: 0 }
    }

    /// 增加聲望
    pub fn add_respect(&mut self, amount: i32) {
        self.respect = (self.respect + amount).max(0);
    }

    /// 取得聲望
    pub fn get_respect(&self) -> i32 {
        self.respect
    }
}
