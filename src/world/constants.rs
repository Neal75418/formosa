//! 世界地圖常數定義
//!
//! 西門町地圖參考真實 Google Maps 街道佈局
//! 原點 (0,0) = 漢中街與峨嵋街交叉口
//! 比例：1 game unit ≈ 1 公尺

use bevy::prelude::*;

// ============================================================================
// 西門町地圖常數定義 (Game Units)
// ============================================================================
/// 道路 Y 軸高度
pub const ROAD_Y: f32 = 0.05;

// 道路 X 軸位置 (南北向道路，由東到西)
/// 中華路 X 座標
pub const X_ZHONGHUA: f32 = 80.0;   // 中華路 (東邊界，主幹道)
/// 漢中街 X 座標
pub const X_HAN: f32 = 0.0;         // 漢中街 (徒步區中軸)
/// 西寧南路 X 座標
pub const X_XINING: f32 = -55.0;    // 西寧南路
/// 康定路 X 座標
pub const X_KANGDING: f32 = -100.0; // 康定路 (西邊界)

// 道路 Z 軸位置 (東西向道路，由北到南)
/// 漢口街 Z 座標
pub const Z_HANKOU: f32 = -80.0;  // 漢口街 (北邊界)
/// 武昌街 Z 座標
pub const Z_WUCHANG: f32 = -50.0; // 武昌街二段 (徒步區北)
/// 昆明街 Z 座標
pub const Z_KUNMING: f32 = -25.0; // 昆明街 (武昌與峨嵋之間)
/// 峨眉街 Z 座標
pub const Z_EMEI: f32 = 0.0;      // 峨嵋街 (徒步區中軸)
/// 成都路 Z 座標
pub const Z_CHENGDU: f32 = 50.0;  // 成都路 (南邊界，主幹道)

// 道路寬度 (按真實比例)
/// 中華路路寬
pub const W_ZHONGHUA: f32 = 40.0;   // 中華路 (6-8 車道)
/// 主要道路路寬
pub const W_MAIN: f32 = 16.0;       // 成都路, 康定路 (2 車道 + 寬人行道)
/// 次要道路路寬
pub const W_SECONDARY: f32 = 12.0;  // 西寧南路, 漢口街 (2-4 車道)
/// 行人專用區路寬
pub const W_PEDESTRIAN: f32 = 15.0; // 漢中街, 峨嵋街, 武昌街 (徒步區)
/// 小巷路寬
pub const W_ALLEY: f32 = 8.0;       // 昆明街 (小巷)

/// 建築物與道路之間的緩衝距離（公尺）
pub const BUILDING_ROAD_BUFFER: f32 = 1.5;

// 玩家出生/重生位置（漢中街與峨嵋街交叉口）
/// 玩家出生 X 座標
pub const PLAYER_SPAWN_X: f32 = 5.0;
/// 玩家出生 Z 座標
pub const PLAYER_SPAWN_Z: f32 = -5.0;
/// 重生時角色 Y 軸高度（含角色自身高度偏移）
pub const PLAYER_RESPAWN_Y: f32 = 0.7;

// 斑馬線
/// 斑馬線與道路中心線的偏移距離（公尺）
pub const ZEBRA_CROSSING_OFFSET: f32 = 2.5;
/// 路面標線 Y 軸偏移（避免 Z-fighting）
pub const ROAD_MARKING_Y_OFFSET: f32 = 0.01;

/// 地圖邊界（XZ 平面），用於限制 NPC/車輛不駛出地圖
/// 值略小於邊界牆位置，確保實體在可見區域內
#[derive(Resource, Clone, Debug)]
pub struct MapBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

impl Default for MapBounds {
    fn default() -> Self {
        Self {
            min_x: -119.0, // 康定路外側
            max_x: 109.0,  // 中華路外側
            min_z: -94.0,  // 漢口街外側
            max_z: 64.0,   // 成都路外側
        }
    }
}

impl MapBounds {
    /// 將座標夾持在邊界內
    pub fn clamp_position(&self, x: f32, z: f32) -> (f32, f32) {
        (x.clamp(self.min_x, self.max_x), z.clamp(self.min_z, self.max_z))
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
    pub fn try_record(&mut self, pos: Vec3, width: f32, _height: f32, depth: f32, name: &str) -> bool {
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
