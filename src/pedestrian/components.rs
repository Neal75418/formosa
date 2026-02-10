//! 行人組件
//!
//! 定義行人 NPC 的組件、狀態和資源。

#![allow(dead_code)]

use bevy::prelude::*;
use std::collections::VecDeque;

// ============================================================================
// 行人組件
// ============================================================================

/// 行人標記組件
#[derive(Component, Debug)]
pub struct Pedestrian {
    /// 行人類型
    pub ped_type: PedestrianType,
}

impl Default for Pedestrian {
    fn default() -> Self {
        Self {
            ped_type: PedestrianType::Casual,
        }
    }
}

/// 行人類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PedestrianType {
    #[default]
    Casual,     // 一般路人
    Business,   // 上班族
    Student,    // 學生
    Tourist,    // 觀光客
}

/// 行人狀態組件
#[derive(Component, Debug)]
pub struct PedestrianState {
    /// 當前狀態
    pub state: PedState,
    /// 恐懼程度 (0.0-1.0)
    pub fear_level: f32,
    /// 逃跑持續時間
    pub flee_timer: f32,
    /// 最後威脅位置
    pub last_threat_pos: Option<Vec3>,
    /// 卡住計時器（用於檢測行人是否卡在障礙物）
    pub stuck_timer: f32,
    /// 上一次記錄的位置（用於卡住檢測）
    pub last_recorded_pos: Vec3,
}

impl Default for PedestrianState {
    fn default() -> Self {
        Self {
            state: PedState::Walking,
            fear_level: 0.0,
            flee_timer: 0.0,
            last_threat_pos: None,
            stuck_timer: 0.0,
            last_recorded_pos: Vec3::ZERO,
        }
    }
}

/// 行人行為狀態
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PedState {
    Idle,           // 站著（等紅燈、看手機）
    #[default]
    Walking,        // 正常行走
    Fleeing,        // 逃跑中
    CallingPolice,  // 報警中（掏出手機打電話）
}

// ============================================================================
// 行人報警系統組件
// ============================================================================

/// 行人報警狀態組件
/// 當行人目擊犯罪時，會進入報警狀態
#[derive(Component, Debug)]
pub struct WitnessState {
    /// 是否已目擊犯罪
    pub witnessed_crime: bool,
    /// 目擊的犯罪類型
    pub crime_type: Option<WitnessedCrime>,
    /// 犯罪發生位置
    pub crime_position: Option<Vec3>,
    /// 報警進度（0.0 - 1.0，達到 1.0 時完成報警）
    pub call_progress: f32,
    /// 報警所需時間（秒）
    pub call_duration: f32,
    /// 報警冷卻（避免同一行人重複報警）
    pub report_cooldown: f32,
    /// 是否已完成報警
    pub has_reported: bool,
}

impl Default for WitnessState {
    fn default() -> Self {
        Self {
            witnessed_crime: false,
            crime_type: None,
            crime_position: None,
            call_progress: 0.0,
            call_duration: 3.0,  // 預設 3 秒完成報警
            report_cooldown: 0.0,
            has_reported: false,
        }
    }
}

impl WitnessState {
    /// 目擊犯罪
    pub fn witness_crime(&mut self, crime: WitnessedCrime, position: Vec3) {
        // 如果冷卻中或已報警，不重複目擊
        if self.report_cooldown > 0.0 || self.has_reported {
            return;
        }
        self.witnessed_crime = true;
        self.crime_type = Some(crime);
        self.crime_position = Some(position);
        self.call_progress = 0.0;
    }

    /// 更新報警進度
    /// 回傳 true 表示報警完成
    pub fn tick(&mut self, dt: f32) -> bool {
        // 更新冷卻
        if self.report_cooldown > 0.0 {
            self.report_cooldown -= dt;
        }

        // 如果正在報警
        if self.witnessed_crime && !self.has_reported {
            self.call_progress += dt / self.call_duration;
            if self.call_progress >= 1.0 {
                self.call_progress = 1.0;
                self.has_reported = true;
                self.report_cooldown = 60.0; // 60 秒內不會再報警
                return true;
            }
        }
        false
    }

    /// 重置狀態（被打斷或逃跑時）
    pub fn reset(&mut self) {
        self.witnessed_crime = false;
        self.crime_type = None;
        self.crime_position = None;
        self.call_progress = 0.0;
    }
}

/// 目擊的犯罪類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessedCrime {
    /// 開槍（聽到槍聲）
    Gunshot,
    /// 攻擊（看到玩家攻擊行人）
    Assault,
    /// 謀殺（看到玩家殺死行人）
    Murder,
    /// 搶車（看到玩家搶車）
    VehicleTheft,
    /// 撞人（看到玩家開車撞人）
    VehicleHit,
}

impl WitnessedCrime {
    /// 獲取犯罪的嚴重程度（影響報警速度）
    pub fn severity(&self) -> f32 {
        match self {
            WitnessedCrime::Gunshot => 0.8,
            WitnessedCrime::Assault => 0.5,
            WitnessedCrime::Murder => 1.0,
            WitnessedCrime::VehicleTheft => 0.6,
            WitnessedCrime::VehicleHit => 0.9,
        }
    }

    /// 獲取目擊範圍（視覺距離）
    pub fn witness_range(&self) -> f32 {
        match self {
            WitnessedCrime::Gunshot => 30.0,   // 聽覺範圍較大
            WitnessedCrime::Assault => 15.0,   // 視覺範圍
            WitnessedCrime::Murder => 20.0,    // 視覺範圍（較遠也能看到）
            WitnessedCrime::VehicleTheft => 12.0,
            WitnessedCrime::VehicleHit => 25.0, // 撞擊聲音較大
        }
    }
}

// ============================================================================
// 行人外觀
// ============================================================================

/// 行人外觀配置
#[derive(Clone, Debug)]
pub struct PedestrianAppearance {
    pub skin_color: Color,
    pub shirt_color: Color,
    pub pants_color: Color,
    pub shoe_color: Color,
    pub hair_color: Color,
}

impl PedestrianAppearance {
    /// 隨機生成休閒風格外觀
    pub fn random_casual() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();

        // 膚色變化
        let skin_tone = rng.random_range(0.6..0.9);
        let skin_color = Color::srgb(skin_tone, skin_tone * 0.8, skin_tone * 0.7);

        // 隨機上衣顏色
        let shirt_colors = [
            Color::srgb(0.2, 0.3, 0.6),   // 藍色
            Color::srgb(0.6, 0.2, 0.2),   // 紅色
            Color::srgb(0.2, 0.5, 0.3),   // 綠色
            Color::srgb(0.8, 0.8, 0.8),   // 白色
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.6, 0.5, 0.2),   // 黃褐色
            Color::srgb(0.5, 0.3, 0.5),   // 紫色
        ];
        let shirt_color = shirt_colors[rng.random_range(0..shirt_colors.len())];

        // 隨機褲子顏色
        let pants_colors = [
            Color::srgb(0.1, 0.1, 0.2),   // 深藍牛仔
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.4, 0.35, 0.3),  // 卡其色
            Color::srgb(0.3, 0.3, 0.3),   // 灰色
        ];
        let pants_color = pants_colors[rng.random_range(0..pants_colors.len())];

        // 鞋子顏色
        let shoe_colors = [
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.8, 0.8, 0.8),   // 白色
            Color::srgb(0.4, 0.2, 0.1),   // 棕色
        ];
        let shoe_color = shoe_colors[rng.random_range(0..shoe_colors.len())];

        // 頭髮顏色
        let hair_colors = [
            Color::srgb(0.05, 0.05, 0.05), // 黑色
            Color::srgb(0.2, 0.1, 0.05),   // 深棕
            Color::srgb(0.4, 0.3, 0.2),    // 棕色
        ];
        let hair_color = hair_colors[rng.random_range(0..hair_colors.len())];

        Self {
            skin_color,
            shirt_color,
            pants_color,
            shoe_color,
            hair_color,
        }
    }

    /// 生成上班族風格
    pub fn random_business() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();

        let skin_tone = rng.random_range(0.6..0.9);
        let skin_color = Color::srgb(skin_tone, skin_tone * 0.8, skin_tone * 0.7);

        // 上班族：白襯衫或淺色襯衫
        let shirt_colors = [
            Color::srgb(0.9, 0.9, 0.9),   // 白色
            Color::srgb(0.7, 0.8, 0.9),   // 淺藍
            Color::srgb(0.9, 0.85, 0.8),  // 米色
        ];
        let shirt_color = shirt_colors[rng.random_range(0..shirt_colors.len())];

        // 深色西褲
        let pants_colors = [
            Color::srgb(0.1, 0.1, 0.15),  // 深藍
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.25, 0.25, 0.25),// 深灰
        ];
        let pants_color = pants_colors[rng.random_range(0..pants_colors.len())];

        Self {
            skin_color,
            shirt_color,
            pants_color,
            shoe_color: Color::srgb(0.1, 0.1, 0.1),
            hair_color: Color::srgb(0.05, 0.05, 0.05),
        }
    }
}

// ============================================================================
// 資源
// ============================================================================

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

// ============================================================================
// 視覺資源（預創建以提升效能）
// ============================================================================

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
// 行走動畫組件
// ============================================================================

/// 行人腿部標記（用於行走動畫）
#[derive(Component)]
pub struct PedestrianLeg {
    /// 是左腿還是右腿
    pub is_left: bool,
}

/// 行人手臂標記（用於行走動畫）
#[derive(Component)]
pub struct PedestrianArm {
    /// 是左手還是右手
    pub is_left: bool,
}

/// 行走動畫狀態
#[derive(Component, Default)]
pub struct WalkingAnimation {
    /// 動畫週期計時器
    pub phase: f32,
    /// 動畫速度（與移動速度關聯）
    pub speed: f32,
}

// ============================================================================
// 車輛碰撞組件
// ============================================================================

/// 行人被車撞標記
#[derive(Component)]
pub struct HitByVehicle {
    /// 撞擊方向
    pub impact_direction: Vec3,
    /// 撞擊力度
    pub impact_force: f32,
    /// 撞擊時間
    pub hit_time: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- WitnessState ---

    #[test]
    fn witness_crime_sets_state() {
        let mut ws = WitnessState::default();
        ws.witness_crime(WitnessedCrime::Gunshot, Vec3::new(1.0, 0.0, 2.0));
        assert!(ws.witnessed_crime);
        assert_eq!(ws.crime_type, Some(WitnessedCrime::Gunshot));
        assert_eq!(ws.call_progress, 0.0);
    }

    #[test]
    fn witness_crime_ignored_on_cooldown() {
        let mut ws = WitnessState { report_cooldown: 30.0, ..WitnessState::default() };
        ws.witness_crime(WitnessedCrime::Assault, Vec3::ZERO);
        assert!(!ws.witnessed_crime);
    }

    #[test]
    fn witness_tick_completes_call() {
        let mut ws = WitnessState::default();
        ws.witness_crime(WitnessedCrime::Murder, Vec3::ZERO);
        assert!(!ws.tick(1.0));
        assert!(!ws.tick(1.0));
        assert!(ws.tick(1.0));
        assert!(ws.has_reported);
        assert!((ws.report_cooldown - 60.0).abs() < f32::EPSILON);
    }

    #[test]
    fn witness_reset_clears_state() {
        let mut ws = WitnessState::default();
        ws.witness_crime(WitnessedCrime::VehicleTheft, Vec3::ZERO);
        ws.tick(1.0);
        ws.reset();
        assert!(!ws.witnessed_crime);
        assert_eq!(ws.crime_type, None);
        assert_eq!(ws.call_progress, 0.0);
    }

    // --- WitnessedCrime ---

    #[test]
    fn crime_severity_ordered() {
        assert!(WitnessedCrime::Murder.severity() > WitnessedCrime::Assault.severity());
        assert!(WitnessedCrime::VehicleHit.severity() > WitnessedCrime::Gunshot.severity());
    }

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

