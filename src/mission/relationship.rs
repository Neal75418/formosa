use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::story_data::NpcId;

/// NPC 關係管理器
#[derive(Resource, Serialize, Deserialize, Default, Debug, Clone)]
pub struct RelationshipManager {
    /// NPC 好感度
    pub relationships: HashMap<NpcId, i32>,
}

impl RelationshipManager {
    /// 取得 NPC 好感度
    pub fn get_relationship(&self, npc_id: NpcId) -> i32 {
        self.relationships.get(&npc_id).copied().unwrap_or(0)
    }

    /// 修改 NPC 好感度
    pub fn change_relationship(&mut self, npc_id: NpcId, delta: i32) {
        let current = self.get_relationship(npc_id);
        let new_value = (current + delta).clamp(-100, 100);
        self.relationships.insert(npc_id, new_value);
    }
}
