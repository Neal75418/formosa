//! 行人資源（配置、視覺、路徑等）

use bevy::prelude::*;
use std::collections::VecDeque;

/// 行人生成配置
#[derive(Resource)]
pub struct PedestrianConfig {
    /// 最大行人數量
    pub max_count: usize,
    /// 生成半徑（玩家周圍）
    pub spawn_radius: f32,
    /// 消失半徑（超過此距離移除）
    pub despawn_radius: f32,
    /// 生成間隔（秒）
    pub spawn_interval: f32,
    /// 生成計時器
    pub spawn_timer: f32,
    /// 行走速度
    pub walk_speed: f32,
    /// 逃跑速度
    pub flee_speed: f32,
    /// 聽到槍聲的範圍
    pub hearing_range: f32,
}

impl Default for PedestrianConfig {
    fn default() -> Self {
        Self {
            max_count: 12,          // 減少：避免太多行人漫無目的
            spawn_radius: 40.0,     // 縮小：只在玩家附近生成
            despawn_radius: 60.0,   // 縮小：更快清除遠處行人
            spawn_interval: 4.0,    // 增加：減緩生成速度
            spawn_timer: 0.0,
            walk_speed: 2.0,
            flee_speed: 5.0,
            hearing_range: 30.0,
        }
    }
}

/// 行人路徑資源
#[derive(Resource)]
#[derive(Default)]
pub struct PedestrianPaths {
    /// 人行道路徑列表
    pub sidewalk_paths: Vec<SidewalkPath>,
}

/// 單條人行道路徑
#[derive(Clone, Debug)]
pub struct SidewalkPath {
    /// 路徑名稱（用於調試）
    pub name: String,
    /// 路點列表
    pub waypoints: Vec<Vec3>,
    /// 是否往返（否則循環）
    pub ping_pong: bool,
}

impl SidewalkPath {
    /// 建立新實例
    pub fn new(name: &str, waypoints: Vec<Vec3>, ping_pong: bool) -> Self {
        Self {
            name: name.to_string(),
            waypoints,
            ping_pong,
        }
    }
}

/// 槍擊事件追蹤（用於行人反應）
#[derive(Resource, Default)]
pub struct GunshotTracker {
    /// 最近的槍擊位置和時間（使用 VecDeque 以 O(1) 移除舊記錄）
    pub recent_shots: VecDeque<(Vec3, f32)>,
}

impl GunshotTracker {
    /// 記錄槍擊事件
    pub fn record_shot(&mut self, position: Vec3, time: f32) {
        self.recent_shots.push_back((position, time));
        // 只保留最近 10 次 - O(1) 移除
        if self.recent_shots.len() > 10 {
            self.recent_shots.pop_front();
        }
    }

    /// 清理過期的槍擊記錄（超過 5 秒）
    pub fn cleanup(&mut self, current_time: f32) {
        self.recent_shots.retain(|(_, t)| current_time - *t < 5.0);
    }

    /// 檢查附近是否有最近的槍擊
    pub fn has_nearby_shot(&self, position: Vec3, range: f32, current_time: f32) -> Option<Vec3> {
        let range_sq = range * range;
        for (shot_pos, shot_time) in self.recent_shots.iter().rev() {
            // 只考慮 3 秒內的槍擊
            if current_time - *shot_time > 3.0 {
                continue;
            }
            if position.distance_squared(*shot_pos) <= range_sq {
                return Some(*shot_pos);
            }
        }
        None
    }
}

/// 行人視覺資源（預創建的 mesh 和 material）
#[derive(Resource)]
pub struct PedestrianVisuals {
    // Meshes
    pub head_mesh: Handle<Mesh>,
    pub hair_mesh: Handle<Mesh>,
    pub torso_mesh: Handle<Mesh>,
    pub leg_mesh: Handle<Mesh>,
    pub arm_mesh: Handle<Mesh>,
    pub shoe_mesh: Handle<Mesh>,
    // 預定義材質（常用顏色）
    pub skin_materials: Vec<Handle<StandardMaterial>>,
    pub shirt_materials: Vec<Handle<StandardMaterial>>,
    pub pants_materials: Vec<Handle<StandardMaterial>>,
    pub shoe_materials: Vec<Handle<StandardMaterial>>,
    pub hair_materials: Vec<Handle<StandardMaterial>>,
}

impl PedestrianVisuals {
    /// 建立新實例
    pub fn new(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        // 人體尺寸
        let head_radius = 0.12;
        let torso_height = 0.5;
        let leg_height = 0.45;
        let arm_length = 0.35;

        // 創建共用 meshes
        let head_mesh = meshes.add(Sphere::new(head_radius));
        let hair_mesh = meshes.add(Sphere::new(head_radius * 1.05));
        let torso_mesh = meshes.add(Capsule3d::new(0.15, torso_height));
        let leg_mesh = meshes.add(Capsule3d::new(0.06, leg_height));
        let arm_mesh = meshes.add(Capsule3d::new(0.04, arm_length));
        let shoe_mesh = meshes.add(Cuboid::new(0.08, 0.05, 0.15));

        // 預定義膚色
        let skin_tones = [0.65, 0.75, 0.85];
        let skin_materials: Vec<_> = skin_tones.iter().map(|&tone| {
            materials.add(StandardMaterial {
                base_color: Color::srgb(tone, tone * 0.8, tone * 0.7),
                perceptual_roughness: 0.8,
                ..default()
            })
        }).collect();

        // 預定義上衣顏色
        let shirt_colors = [
            Color::srgb(0.2, 0.3, 0.6),   // 藍色
            Color::srgb(0.6, 0.2, 0.2),   // 紅色
            Color::srgb(0.2, 0.5, 0.3),   // 綠色
            Color::srgb(0.8, 0.8, 0.8),   // 白色
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.6, 0.5, 0.2),   // 黃褐色
            Color::srgb(0.5, 0.3, 0.5),   // 紫色
            Color::srgb(0.9, 0.9, 0.9),   // 白襯衫
            Color::srgb(0.7, 0.8, 0.9),   // 淺藍
        ];
        let shirt_materials: Vec<_> = shirt_colors.iter().map(|&color| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.7,
                ..default()
            })
        }).collect();

        // 預定義褲子顏色
        let pants_colors = [
            Color::srgb(0.1, 0.1, 0.2),   // 深藍牛仔
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.4, 0.35, 0.3),  // 卡其色
            Color::srgb(0.3, 0.3, 0.3),   // 灰色
            Color::srgb(0.25, 0.25, 0.25),// 深灰
        ];
        let pants_materials: Vec<_> = pants_colors.iter().map(|&color| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.6,
                ..default()
            })
        }).collect();

        // 預定義鞋子顏色
        let shoe_colors = [
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.8, 0.8, 0.8),   // 白色
            Color::srgb(0.4, 0.2, 0.1),   // 棕色
        ];
        let shoe_materials: Vec<_> = shoe_colors.iter().map(|&color| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.5,
                ..default()
            })
        }).collect();

        // 預定義頭髮顏色
        let hair_colors = [
            Color::srgb(0.05, 0.05, 0.05), // 黑色
            Color::srgb(0.2, 0.1, 0.05),   // 深棕
            Color::srgb(0.4, 0.3, 0.2),    // 棕色
        ];
        let hair_materials: Vec<_> = hair_colors.iter().map(|&color| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.9,
                ..default()
            })
        }).collect();

        Self {
            head_mesh,
            hair_mesh,
            torso_mesh,
            leg_mesh,
            arm_mesh,
            shoe_mesh,
            skin_materials,
            shirt_materials,
            pants_materials,
            shoe_materials,
            hair_materials,
        }
    }

    /// 隨機選擇材質索引
    pub fn random_indices(&self) -> PedestrianMaterialIndices {
        use rand::Rng;
        let mut rng = rand::rng();
        PedestrianMaterialIndices {
            skin: rng.random_range(0..self.skin_materials.len()),
            shirt: rng.random_range(0..self.shirt_materials.len()),
            pants: rng.random_range(0..self.pants_materials.len()),
            shoe: rng.random_range(0..self.shoe_materials.len()),
            hair: rng.random_range(0..self.hair_materials.len()),
        }
    }
}

/// 材質索引（用於隨機選擇外觀）
pub struct PedestrianMaterialIndices {
    pub skin: usize,
    pub shirt: usize,
    pub pants: usize,
    pub shoe: usize,
    pub hair: usize,
}

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- GunshotTracker ---

    #[test]
    fn gunshot_tracker_record_and_query() {
        let mut gt = GunshotTracker::default();
        gt.record_shot(Vec3::new(10.0, 0.0, 10.0), 1.0);
        assert!(gt.has_nearby_shot(Vec3::new(11.0, 0.0, 10.0), 5.0, 2.0).is_some());
        assert!(gt.has_nearby_shot(Vec3::new(100.0, 0.0, 100.0), 5.0, 2.0).is_none());
    }

    #[test]
    fn gunshot_tracker_expires_old_shots() {
        let mut gt = GunshotTracker::default();
        gt.record_shot(Vec3::ZERO, 1.0);
        gt.cleanup(10.0);
        assert!(gt.recent_shots.is_empty());
    }

    #[test]
    fn gunshot_tracker_time_window() {
        let mut gt = GunshotTracker::default();
        gt.record_shot(Vec3::ZERO, 1.0);
        // 恰好 3 秒（差值 = 3.0，不嚴格大於 3.0）→ 仍可找到
        assert!(gt.has_nearby_shot(Vec3::ZERO, 10.0, 4.0).is_some());
        // 超過 3 秒 → 過期
        assert!(gt.has_nearby_shot(Vec3::ZERO, 10.0, 4.01).is_none());
    }
}
