//! 世界常數：路面高度與標線偏移、重生高度、地圖邊界型別、建築重疊追蹤
//!
//! 道路位置、寬度、邊界與出生點在地圖資料檔 `assets/levels/ximending.ron`

use bevy::prelude::*;

/// 道路 Y 軸高度
pub const ROAD_Y: f32 = 0.05;

// 玩家重生（出生點在地圖資料檔）
/// 重生時角色 Y 軸高度（含角色自身高度偏移）
pub const PLAYER_RESPAWN_Y: f32 = 0.7;

// 路面標線
/// 路面標線 Y 軸偏移（避免 Z-fighting）
pub const ROAD_MARKING_Y_OFFSET: f32 = 0.01;

/// 地圖邊界（XZ 平面），由地圖資料檔決定；用於限制 NPC／車輛不駛出地圖
/// 值略小於邊界牆位置，確保實體在可見區域內
#[derive(Resource, Clone, Debug)]
pub struct MapBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl MapBounds {
    /// 將座標夾持在邊界內
    pub fn clamp_position(&self, x: f32, z: f32) -> (f32, f32) {
        (
            x.clamp(self.min_x, self.max_x),
            z.clamp(self.min_z, self.max_z),
        )
    }
}

/// 建築物重疊追蹤器 - 記錄已生成建築的包圍盒，防止重疊
pub struct BuildingTracker {
    bounds: Vec<(Vec3, Vec3, String)>, // (min, max, name)
}

impl BuildingTracker {
    /// 建立新實例
    pub fn new() -> Self {
        Self { bounds: Vec::new() }
    }

    /// 檢查新建築是否與已有建築重疊，若無重疊則記錄並返回 true
    pub fn try_record(
        &mut self,
        pos: Vec3,
        width: f32,
        _height: f32,
        depth: f32,
        name: &str,
    ) -> bool {
        // 只檢查 XZ 平面重疊（Y 軸高度不考慮，建築都在地面）
        let half_w = width / 2.0;
        let half_d = depth / 2.0;
        let min = Vec3::new(pos.x - half_w, 0.0, pos.z - half_d);
        let max = Vec3::new(pos.x + half_w, 1.0, pos.z + half_d);

        // 檢查與已有建築是否重疊
        for (existing_min, existing_max, existing_name) in &self.bounds {
            if Self::aabb_overlap_xz(min, max, *existing_min, *existing_max) {
                info!("🚫 跳過建築 \"{}\" (與 \"{}\" 重疊)", name, existing_name);
                return false;
            }
        }

        // 無重疊，記錄此建築
        self.bounds.push((min, max, name.to_string()));
        true
    }

    /// 檢查兩個 AABB 在 XZ 平面是否重疊
    fn aabb_overlap_xz(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3) -> bool {
        a_min.x < b_max.x && a_max.x > b_min.x && a_min.z < b_max.z && a_max.z > b_min.z
    }

    /// 檢查建築是否已成功生成（用於招牌檢查）
    pub fn is_spawned(&self, name: &str) -> bool {
        self.bounds.iter().any(|(_, _, n)| n == name)
    }

    /// 檢查建築名稱是否包含指定關鍵字（模糊匹配）
    pub fn is_spawned_contains(&self, keyword: &str) -> bool {
        self.bounds.iter().any(|(_, _, n)| n.contains(keyword))
    }
}

impl Default for BuildingTracker {
    fn default() -> Self {
        Self::new()
    }
}
