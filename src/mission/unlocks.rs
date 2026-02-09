//! 解鎖內容管理（物品、區域）
#![allow(dead_code)]


use std::collections::HashSet;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::story_data::AreaId;

/// 解鎖內容管理器
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct UnlockManager {
    /// 解鎖的物品 ID 集合
    pub unlocked_items: HashSet<String>,
    /// 解鎖的區域 ID 集合
    pub unlocked_areas: HashSet<AreaId>,
}

impl Default for UnlockManager {
    fn default() -> Self {
        Self {
            unlocked_items: HashSet::new(),
            unlocked_areas: HashSet::from([1]), // 初始解鎖第一個區域
        }
    }
}

impl UnlockManager {
    /// 解鎖物品
    pub fn unlock_item(&mut self, item_id: impl Into<String>) {
        self.unlocked_items.insert(item_id.into());
    }

    /// 檢查物品是否已解鎖
    pub fn is_item_unlocked(&self, item_id: &str) -> bool {
        self.unlocked_items.contains(item_id)
    }

    /// 解鎖區域
    pub fn unlock_area(&mut self, area_id: AreaId) {
        self.unlocked_areas.insert(area_id);
    }

    /// 檢查區域是否已解鎖
    pub fn is_area_unlocked(&self, area_id: AreaId) -> bool {
        self.unlocked_areas.contains(&area_id)
    }
}
