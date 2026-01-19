//! 行人組件
//!
//! 定義行人 NPC 的組件、狀態和資源。

#![allow(dead_code)] // Phase 5+ 預留功能

use bevy::prelude::*;

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
}

impl Default for PedestrianState {
    fn default() -> Self {
        Self {
            state: PedState::Walking,
            fear_level: 0.0,
            flee_timer: 0.0,
            last_threat_pos: None,
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
            max_count: 30,
            spawn_radius: 50.0,
            despawn_radius: 80.0,
            spawn_interval: 2.0,
            spawn_timer: 0.0,
            walk_speed: 2.0,
            flee_speed: 5.0,
            hearing_range: 30.0,
        }
    }
}

/// 行人路徑資源
#[derive(Resource)]
pub struct PedestrianPaths {
    /// 人行道路徑列表
    pub sidewalk_paths: Vec<SidewalkPath>,
}

impl Default for PedestrianPaths {
    fn default() -> Self {
        Self {
            sidewalk_paths: Vec::new(),
        }
    }
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
    /// 最近的槍擊位置和時間
    pub recent_shots: Vec<(Vec3, f32)>,
}

impl GunshotTracker {
    /// 記錄槍擊事件
    pub fn record_shot(&mut self, position: Vec3, time: f32) {
        self.recent_shots.push((position, time));
        // 只保留最近 10 次
        if self.recent_shots.len() > 10 {
            self.recent_shots.remove(0);
        }
    }

    /// 清理過期的槍擊記錄（超過 5 秒）
    pub fn cleanup(&mut self, current_time: f32) {
        self.recent_shots.retain(|(_, t)| current_time - *t < 5.0);
    }

    /// 檢查附近是否有最近的槍擊
    pub fn has_nearby_shot(&self, position: Vec3, range: f32, current_time: f32) -> Option<Vec3> {
        for (shot_pos, shot_time) in self.recent_shots.iter().rev() {
            // 只考慮 3 秒內的槍擊
            if current_time - *shot_time > 3.0 {
                continue;
            }
            if position.distance(*shot_pos) <= range {
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

// ============================================================================
// A* 尋路系統
// ============================================================================

/// A* 尋路網格配置
#[derive(Resource)]
pub struct PathfindingGrid {
    /// 網格原點（左下角）
    pub origin: Vec3,
    /// 網格尺寸（格數）
    pub width: usize,
    pub height: usize,
    /// 每格大小（米）
    pub cell_size: f32,
    /// 可通行性地圖 (true = 可通行)
    pub walkable: Vec<bool>,
}

impl Default for PathfindingGrid {
    fn default() -> Self {
        // 西門町區域: X ∈ [-70, 50], Z ∈ [-70, 60]
        // 使用 2m 格子
        let cell_size = 2.0;
        let width = 60;  // 120m / 2m = 60 格
        let height = 65; // 130m / 2m = 65 格
        let origin = Vec3::new(-70.0, 0.0, -70.0);

        // 預設全部可通行
        let walkable = vec![true; width * height];

        Self {
            origin,
            width,
            height,
            cell_size,
            walkable,
        }
    }
}

impl PathfindingGrid {
    /// 世界座標轉網格座標
    pub fn world_to_grid(&self, pos: Vec3) -> Option<(usize, usize)> {
        let local_x = pos.x - self.origin.x;
        let local_z = pos.z - self.origin.z;

        let grid_x = (local_x / self.cell_size).floor() as i32;
        let grid_z = (local_z / self.cell_size).floor() as i32;

        if grid_x >= 0 && grid_x < self.width as i32 && grid_z >= 0 && grid_z < self.height as i32 {
            Some((grid_x as usize, grid_z as usize))
        } else {
            None
        }
    }

    /// 網格座標轉世界座標（格子中心）
    pub fn grid_to_world(&self, x: usize, z: usize) -> Vec3 {
        Vec3::new(
            self.origin.x + (x as f32 + 0.5) * self.cell_size,
            0.25, // 人行道高度
            self.origin.z + (z as f32 + 0.5) * self.cell_size,
        )
    }

    /// 檢查格子是否可通行
    pub fn is_walkable(&self, x: usize, z: usize) -> bool {
        if x < self.width && z < self.height {
            self.walkable[z * self.width + x]
        } else {
            false
        }
    }

    /// 設置格子可通行性
    pub fn set_walkable(&mut self, x: usize, z: usize, walkable: bool) {
        if x < self.width && z < self.height {
            self.walkable[z * self.width + x] = walkable;
        }
    }

    /// A* 尋路
    pub fn find_path(&self, start: Vec3, goal: Vec3) -> Option<Vec<Vec3>> {
        let start_grid = self.world_to_grid(start)?;
        let goal_grid = self.world_to_grid(goal)?;

        // 使用簡單的 A* 實現
        use std::collections::{BinaryHeap, HashMap};
        use std::cmp::Ordering;

        #[derive(Clone, Copy, Eq, PartialEq)]
        struct Node {
            pos: (usize, usize),
            f_cost: i32,
        }

        impl Ord for Node {
            fn cmp(&self, other: &Self) -> Ordering {
                other.f_cost.cmp(&self.f_cost) // 反向以取得最小堆
            }
        }

        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let heuristic = |pos: (usize, usize)| -> i32 {
            let dx = (pos.0 as i32 - goal_grid.0 as i32).abs();
            let dz = (pos.1 as i32 - goal_grid.1 as i32).abs();
            (dx + dz) * 10 // 曼哈頓距離
        };

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut g_score: HashMap<(usize, usize), i32> = HashMap::new();

        g_score.insert(start_grid, 0);
        open_set.push(Node {
            pos: start_grid,
            f_cost: heuristic(start_grid),
        });

        let directions = [
            (-1i32, 0i32), (1, 0), (0, -1), (0, 1),  // 四方向
            (-1, -1), (-1, 1), (1, -1), (1, 1),      // 對角線
        ];

        while let Some(current) = open_set.pop() {
            if current.pos == goal_grid {
                // 重建路徑
                let mut path = Vec::new();
                let mut pos = goal_grid;
                path.push(self.grid_to_world(pos.0, pos.1));

                while let Some(&prev) = came_from.get(&pos) {
                    pos = prev;
                    path.push(self.grid_to_world(pos.0, pos.1));
                }

                path.reverse();

                // 簡化路徑（移除共線點）
                if path.len() > 2 {
                    let mut simplified = vec![path[0]];
                    for i in 1..path.len() - 1 {
                        let prev_dir = (path[i] - path[i - 1]).normalize_or_zero();
                        let next_dir = (path[i + 1] - path[i]).normalize_or_zero();
                        if prev_dir.dot(next_dir) < 0.99 {
                            simplified.push(path[i]);
                        }
                    }
                    simplified.push(*path.last().unwrap());
                    return Some(simplified);
                }

                return Some(path);
            }

            let current_g = *g_score.get(&current.pos).unwrap_or(&i32::MAX);

            for (dx, dz) in directions.iter() {
                let nx = current.pos.0 as i32 + dx;
                let nz = current.pos.1 as i32 + dz;

                if nx < 0 || nz < 0 {
                    continue;
                }

                let neighbor = (nx as usize, nz as usize);

                if !self.is_walkable(neighbor.0, neighbor.1) {
                    continue;
                }

                // 對角線移動需要鄰近格子也可通行
                if *dx != 0 && *dz != 0 {
                    if !self.is_walkable(current.pos.0, neighbor.1)
                        || !self.is_walkable(neighbor.0, current.pos.1) {
                        continue;
                    }
                }

                let move_cost = if *dx != 0 && *dz != 0 { 14 } else { 10 };
                let tentative_g = current_g + move_cost;

                if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current.pos);
                    g_score.insert(neighbor, tentative_g);
                    open_set.push(Node {
                        pos: neighbor,
                        f_cost: tentative_g + heuristic(neighbor),
                    });
                }
            }
        }

        None // 找不到路徑
    }
}

/// A* 路徑組件（用於動態尋路的行人）
#[derive(Component)]
pub struct AStarPath {
    /// 計算出的路徑點
    pub waypoints: Vec<Vec3>,
    /// 當前目標路徑點索引
    pub current_index: usize,
    /// 最終目標位置
    pub goal: Vec3,
    /// 是否需要重新計算路徑
    pub needs_recalc: bool,
    /// 路徑計算冷卻時間
    pub recalc_cooldown: f32,
}

impl AStarPath {
    pub fn new(goal: Vec3) -> Self {
        Self {
            waypoints: Vec::new(),
            current_index: 0,
            goal,
            needs_recalc: true,
            recalc_cooldown: 0.0,
        }
    }

    /// 取得當前目標點
    pub fn current_waypoint(&self) -> Option<Vec3> {
        self.waypoints.get(self.current_index).copied()
    }

    /// 前進到下一個路徑點
    pub fn advance(&mut self) -> bool {
        if self.current_index + 1 < self.waypoints.len() {
            self.current_index += 1;
            true
        } else {
            false
        }
    }

    /// 是否已到達終點
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.waypoints.len().saturating_sub(1) && !self.waypoints.is_empty()
    }
}

// ============================================================================
// 日常行為系統
// ============================================================================

/// 行人日常行為組件
#[derive(Component)]
pub struct DailyBehavior {
    /// 當前行為
    pub behavior: BehaviorType,
    /// 行為持續時間
    pub duration: f32,
    /// 行為計時器
    pub timer: f32,
    /// 下一個行為（隨機選擇用）
    pub next_behavior_cooldown: f32,
}

impl Default for DailyBehavior {
    fn default() -> Self {
        Self {
            behavior: BehaviorType::Walking,
            duration: 0.0,
            timer: 0.0,
            next_behavior_cooldown: 5.0,
        }
    }
}

/// 行人行為類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BehaviorType {
    #[default]
    Walking,        // 正常行走
    PhoneWatching,  // 看手機（原地站立，偶爾低頭）
    WindowShopping, // 逛櫥窗（緩慢移動，左右看）
    Chatting,       // 聊天（與另一行人面對面站立）
    Resting,        // 休息（靠牆站或坐在長椅上）
    TakingPhoto,    // 拍照（舉起手機拍照動作）
    SeekingShelter, // 躲雨（快速跑向遮蔽處）
}

impl BehaviorType {
    /// 取得行為的典型持續時間範圍（秒）
    pub fn duration_range(&self) -> (f32, f32) {
        match self {
            BehaviorType::Walking => (10.0, 30.0),
            BehaviorType::PhoneWatching => (5.0, 15.0),
            BehaviorType::WindowShopping => (8.0, 20.0),
            BehaviorType::Chatting => (15.0, 45.0),
            BehaviorType::Resting => (20.0, 60.0),
            BehaviorType::TakingPhoto => (3.0, 8.0),
            BehaviorType::SeekingShelter => (30.0, 120.0), // 躲到雨停為止
        }
    }

    /// 行為的行走速度倍率
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            BehaviorType::Walking => 1.0,
            BehaviorType::PhoneWatching => 0.0,  // 原地不動
            BehaviorType::WindowShopping => 0.3, // 緩慢移動
            BehaviorType::Chatting => 0.0,       // 原地不動
            BehaviorType::Resting => 0.0,        // 原地不動
            BehaviorType::TakingPhoto => 0.0,    // 原地不動
            BehaviorType::SeekingShelter => 2.0, // 快速奔跑
        }
    }
}

/// 興趣點類型（用於日常行為）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointOfInterestType {
    ShopWindow,     // 商店櫥窗
    Bench,          // 長椅
    PhotoSpot,      // 拍照點
    Crosswalk,      // 斑馬線（等紅燈）
    Shelter,        // 遮蔽處（躲雨用）
}

/// 庇護點位置匹配容差
const SHELTER_POSITION_TOLERANCE: f32 = 2.0;

/// 興趣點資源
#[derive(Resource, Default)]
pub struct PointsOfInterest {
    pub shop_windows: Vec<Vec3>,
    pub benches: Vec<Vec3>,
    pub photo_spots: Vec<Vec3>,
    pub shelters: Vec<ShelterPoint>,
}

/// 庇護點（躲雨用）
#[derive(Clone, Debug)]
pub struct ShelterPoint {
    /// 庇護點位置
    pub position: Vec3,
    /// 庇護點類型
    pub shelter_type: ShelterType,
    /// 容納人數上限
    pub capacity: usize,
    /// 當前佔用人數
    pub current_occupants: usize,
}

impl ShelterPoint {
    pub fn new(position: Vec3, shelter_type: ShelterType, capacity: usize) -> Self {
        Self {
            position,
            shelter_type,
            capacity,
            current_occupants: 0,
        }
    }

    /// 是否還有空位
    pub fn has_space(&self) -> bool {
        self.current_occupants < self.capacity
    }
}

/// 庇護點類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShelterType {
    Awning,      // 遮雨棚
    BusStop,     // 公車站
    Building,    // 建築物入口
    Overpass,    // 天橋下
}

impl PointsOfInterest {
    pub fn setup_ximending() -> Self {
        Self {
            // 商店櫥窗位置（沿街道兩側）
            shop_windows: vec![
                // 漢中街東側
                Vec3::new(12.0, 0.25, -40.0),
                Vec3::new(12.0, 0.25, -20.0),
                Vec3::new(12.0, 0.25, 0.0),
                Vec3::new(12.0, 0.25, 20.0),
                // 漢中街西側
                Vec3::new(-12.0, 0.25, -30.0),
                Vec3::new(-12.0, 0.25, -10.0),
                Vec3::new(-12.0, 0.25, 10.0),
                // 峨嵋街
                Vec3::new(-20.0, 0.25, 12.0),
                Vec3::new(0.0, 0.25, 12.0),
                Vec3::new(15.0, 0.25, 12.0),
            ],
            // 長椅位置
            benches: vec![
                Vec3::new(0.0, 0.25, -25.0),
                Vec3::new(0.0, 0.25, 15.0),
                Vec3::new(-25.0, 0.25, 0.0),
                Vec3::new(20.0, 0.25, 0.0),
            ],
            // 拍照點（地標附近）
            photo_spots: vec![
                Vec3::new(0.0, 0.25, 0.0),       // 徒步區中心
                Vec3::new(-30.0, 0.25, -30.0),  // 紅樓附近
                Vec3::new(25.0, 0.25, 25.0),    // 廣場
            ],
            // 庇護點（躲雨用）
            shelters: vec![
                // 公車站（有遮雨棚）- 移到人行道上
                ShelterPoint::new(Vec3::new(30.0, 0.25, 8.0), ShelterType::BusStop, 6),
                ShelterPoint::new(Vec3::new(-30.0, 0.25, -8.0), ShelterType::BusStop, 6),
                // 便利商店門口（有雨遮）
                ShelterPoint::new(Vec3::new(-55.0, 0.25, -52.0), ShelterType::Awning, 4),
                ShelterPoint::new(Vec3::new(55.0, 0.25, -52.0), ShelterType::Awning, 4),
                // 建築物入口
                ShelterPoint::new(Vec3::new(-55.0, 0.25, 52.0), ShelterType::Building, 3),
                ShelterPoint::new(Vec3::new(55.0, 0.25, 52.0), ShelterType::Building, 3),
                ShelterPoint::new(Vec3::new(0.0, 0.25, 52.0), ShelterType::Building, 5),
                // 商店騎樓
                ShelterPoint::new(Vec3::new(-25.0, 0.25, -52.0), ShelterType::Awning, 4),
                ShelterPoint::new(Vec3::new(25.0, 0.25, -52.0), ShelterType::Awning, 4),
                ShelterPoint::new(Vec3::new(0.0, 0.25, -52.0), ShelterType::Awning, 4),
                // 停車場入口
                ShelterPoint::new(Vec3::new(-40.0, 0.25, 22.0), ShelterType::Overpass, 8),
            ],
        }
    }

    /// 找到最近的興趣點
    pub fn find_nearest(&self, pos: Vec3, poi_type: PointOfInterestType, max_distance: f32) -> Option<Vec3> {
        match poi_type {
            PointOfInterestType::ShopWindow => self.find_nearest_point(&self.shop_windows, pos, max_distance),
            PointOfInterestType::Bench => self.find_nearest_point(&self.benches, pos, max_distance),
            PointOfInterestType::PhotoSpot => self.find_nearest_point(&self.photo_spots, pos, max_distance),
            PointOfInterestType::Crosswalk => None, // 暫不實作
            PointOfInterestType::Shelter => self.find_nearest_shelter(pos, max_distance),
        }
    }

    /// 找到最近的點位
    fn find_nearest_point(&self, points: &[Vec3], pos: Vec3, max_distance: f32) -> Option<Vec3> {
        points
            .iter()
            .filter(|p| p.distance(pos) < max_distance)
            .min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)))
            .copied()
    }

    /// 找到最近且有空位的庇護點
    /// 優化：先計算距離再排序，避免重複計算
    pub fn find_nearest_shelter(&self, pos: Vec3, max_distance: f32) -> Option<Vec3> {
        self.shelters
            .iter()
            .filter(|s| s.has_space())
            .map(|s| (s, s.position.distance_squared(pos)))
            .filter(|(_, dist_sq)| *dist_sq < max_distance * max_distance)
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(s, _)| s.position)
    }

    /// 佔用庇護點
    pub fn occupy_shelter(&mut self, pos: Vec3) -> bool {
        if let Some(shelter) = self.shelters.iter_mut()
            .find(|s| s.position.distance(pos) < SHELTER_POSITION_TOLERANCE && s.has_space())
        {
            shelter.current_occupants += 1;
            true
        } else {
            false
        }
    }

    /// 釋放庇護點
    pub fn release_shelter(&mut self, pos: Vec3) {
        if let Some(shelter) = self.shelters.iter_mut()
            .find(|s| s.position.distance(pos) < SHELTER_POSITION_TOLERANCE)
        {
            shelter.current_occupants = shelter.current_occupants.saturating_sub(1);
        }
    }
}

/// 聊天夥伴標記（用於雙人聊天行為）
#[derive(Component)]
pub struct ChattingPartner {
    pub partner_entity: Entity,
}

// ============================================================================
// 躲雨行為組件
// ============================================================================

/// 躲雨狀態組件
/// 追蹤行人是否正在躲雨以及目標庇護點
#[derive(Component)]
pub struct ShelterSeeker {
    /// 目標庇護點位置
    pub target_shelter: Option<Vec3>,
    /// 是否已到達庇護點
    pub is_sheltered: bool,
    /// 躲雨開始時間（用於計算等待時間）
    pub shelter_start_time: f32,
    /// 之前的行為（雨停後恢復）
    pub previous_behavior: BehaviorType,
}

impl Default for ShelterSeeker {
    fn default() -> Self {
        Self {
            target_shelter: None,
            is_sheltered: false,
            shelter_start_time: 0.0,
            previous_behavior: BehaviorType::Walking,
        }
    }
}

impl ShelterSeeker {
    /// 開始尋找庇護
    pub fn start_seeking(&mut self, shelter_pos: Vec3, current_behavior: BehaviorType) {
        self.target_shelter = Some(shelter_pos);
        self.is_sheltered = false;
        self.previous_behavior = current_behavior;
    }

    /// 到達庇護點
    pub fn arrive_at_shelter(&mut self, current_time: f32) {
        self.is_sheltered = true;
        self.shelter_start_time = current_time;
    }

    /// 停止躲雨（雨停了）
    pub fn stop_sheltering(&mut self) {
        self.target_shelter = None;
        self.is_sheltered = false;
    }

    /// 是否正在尋找庇護（移動中）
    pub fn is_seeking(&self) -> bool {
        self.target_shelter.is_some() && !self.is_sheltered
    }
}

// ============================================================================
// 群體恐慌傳播系統（GTA5 風格）
// ============================================================================

/// 恐慌波常數
const PANIC_WAVE_DEFAULT_MAX_RADIUS: f32 = 15.0;      // 預設最大傳播半徑（米）
const PANIC_WAVE_DEFAULT_SPEED: f32 = 8.0;            // 預設傳播速度（米/秒）
const PANIC_WAVE_GUNSHOT_MAX_RADIUS: f32 = 30.0;      // 槍聲恐慌波最大半徑
const PANIC_WAVE_GUNSHOT_SPEED: f32 = 15.0;           // 槍聲恐慌波傳播速度
const PANIC_WAVE_FRONT_WIDTH: f32 = 2.0;              // 恐慌波前緣寬度
const PANIC_SCREAM_COOLDOWN: f32 = 3.0;               // 尖叫冷卻時間（秒）
const PANIC_SPREAD_THRESHOLD: f32 = 0.7;              // 恐慌傳播閾值（panic_level）
const PANIC_IS_PANICKED_THRESHOLD: f32 = 0.3;         // 判斷「正在恐慌」的閾值

/// 恐慌波檢測結果
#[derive(Clone, Debug)]
pub struct PanicWaveHit {
    /// 恐慌強度
    pub intensity: f32,
    /// 恐慌源位置
    pub source: Vec3,
}

/// 恐慌波管理器資源
/// 管理場上所有活躍的恐慌波
#[derive(Resource, Default)]
pub struct PanicWaveManager {
    /// 活躍的恐慌波列表
    pub active_waves: Vec<PanicWave>,
}

impl PanicWaveManager {
    /// 添加新的恐慌波
    pub fn add_wave(&mut self, origin: Vec3, max_radius: f32, speed: f32, intensity: f32, spawn_time: f32) {
        self.active_waves.push(PanicWave {
            origin,
            current_radius: 0.0,
            max_radius,
            propagation_speed: speed,
            intensity,
            spawn_time,
        });
    }

    /// 從槍聲位置創建恐慌波
    pub fn create_from_gunshot(&mut self, position: Vec3, spawn_time: f32) {
        self.add_wave(
            position,
            PANIC_WAVE_GUNSHOT_MAX_RADIUS,
            PANIC_WAVE_GUNSHOT_SPEED,
            1.0,  // 槍聲恐慌強度最高
            spawn_time,
        );
    }

    /// 從行人尖叫位置創建恐慌波
    pub fn create_from_scream(&mut self, position: Vec3, intensity: f32, spawn_time: f32) {
        self.add_wave(
            position,
            PANIC_WAVE_DEFAULT_MAX_RADIUS,
            PANIC_WAVE_DEFAULT_SPEED,
            intensity * 0.8,  // 傳播會衰減
            spawn_time,
        );
    }

    /// 更新所有恐慌波（擴展半徑、清理過期）
    pub fn update(&mut self, delta_time: f32) {
        // 更新所有波的半徑
        for wave in &mut self.active_waves {
            wave.current_radius += wave.propagation_speed * delta_time;
        }

        // 清理已達最大半徑的波
        self.active_waves.retain(|w| w.current_radius < w.max_radius);
    }

    /// 檢查位置是否在任何恐慌波的前緣
    /// 返回最強的恐慌波命中資訊（強度 + 源位置）
    pub fn check_panic_at(&self, position: Vec3) -> Option<PanicWaveHit> {
        let mut best_hit: Option<PanicWaveHit> = None;

        for wave in &self.active_waves {
            let dist = position.distance(wave.origin);
            // 在恐慌波前緣範圍內
            if dist <= wave.current_radius && dist > wave.current_radius - PANIC_WAVE_FRONT_WIDTH {
                match &best_hit {
                    None => {
                        best_hit = Some(PanicWaveHit {
                            intensity: wave.intensity,
                            source: wave.origin,
                        });
                    }
                    Some(current) if wave.intensity > current.intensity => {
                        best_hit = Some(PanicWaveHit {
                            intensity: wave.intensity,
                            source: wave.origin,
                        });
                    }
                    _ => {}
                }
            }
        }

        best_hit
    }
}

/// 單個恐慌波
#[derive(Clone, Debug)]
pub struct PanicWave {
    /// 恐慌源位置
    pub origin: Vec3,
    /// 當前傳播半徑（米）
    pub current_radius: f32,
    /// 最大傳播半徑（米）
    pub max_radius: f32,
    /// 傳播速度（米/秒）
    pub propagation_speed: f32,
    /// 恐慌強度（0.0-1.0，影響逃跑速度和傳播）
    pub intensity: f32,
    /// 創建時間（用於調試）
    pub spawn_time: f32,
}

impl PanicWave {
    /// 創建新的恐慌波
    pub fn new(origin: Vec3, max_radius: f32, speed: f32, intensity: f32, spawn_time: f32) -> Self {
        Self {
            origin,
            current_radius: 0.0,
            max_radius,
            propagation_speed: speed,
            intensity: intensity.clamp(0.0, 1.0),
            spawn_time,
        }
    }

    /// 計算逃跑方向（遠離恐慌源）
    pub fn flee_direction(&self, position: Vec3) -> Vec3 {
        (position - self.origin).normalize_or_zero()
    }
}

/// 行人恐慌狀態組件
/// 追蹤個別行人的恐慌程度和傳播能力
#[derive(Component)]
pub struct PanicState {
    /// 恐慌程度（0.0-1.0）
    pub panic_level: f32,
    /// 恐慌來源方向（用於逃跑）
    pub panic_source: Option<Vec3>,
    /// 尖叫冷卻計時器
    pub scream_cooldown: f32,
    /// 是否可以傳播恐慌（尖叫過一次後設為 false）
    pub can_spread_panic: bool,
    /// 恐慌持續時間（累計被恐慌的時間）
    pub panic_duration: f32,
}

impl Default for PanicState {
    fn default() -> Self {
        Self {
            panic_level: 0.0,
            panic_source: None,
            scream_cooldown: 0.0,
            can_spread_panic: true,
            panic_duration: 0.0,
        }
    }
}

impl PanicState {
    /// 觸發恐慌
    pub fn trigger_panic(&mut self, intensity: f32, source: Vec3) {
        self.panic_level = (self.panic_level + intensity).min(1.0);
        self.panic_source = Some(source);
    }

    /// 更新冷卻計時器
    pub fn update(&mut self, delta_time: f32) {
        if self.scream_cooldown > 0.0 {
            self.scream_cooldown -= delta_time;
        }

        if self.panic_level > 0.0 {
            self.panic_duration += delta_time;
        }
    }

    /// 檢查是否可以尖叫傳播恐慌
    pub fn can_scream(&self) -> bool {
        self.panic_level >= PANIC_SPREAD_THRESHOLD
            && self.can_spread_panic
            && self.scream_cooldown <= 0.0
    }

    /// 執行尖叫（傳播恐慌後調用）
    pub fn do_scream(&mut self) {
        self.scream_cooldown = PANIC_SCREAM_COOLDOWN;
        self.can_spread_panic = false;
    }

    /// 逐漸平息恐慌
    pub fn calm_down(&mut self, rate: f32, delta_time: f32) {
        if self.panic_level > 0.0 {
            self.panic_level = (self.panic_level - rate * delta_time).max(0.0);
            if self.panic_level == 0.0 {
                self.panic_source = None;
                self.panic_duration = 0.0;
                // 重置傳播能力（下次恐慌時可以再尖叫）
                self.can_spread_panic = true;
            }
        }
    }

    /// 計算逃跑方向
    pub fn flee_direction(&self, current_pos: Vec3) -> Option<Vec3> {
        self.panic_source.map(|source| (current_pos - source).normalize_or_zero())
    }

    /// 是否處於恐慌狀態
    pub fn is_panicked(&self) -> bool {
        self.panic_level > PANIC_IS_PANICKED_THRESHOLD
    }
}
