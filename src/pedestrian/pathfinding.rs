//! A* 尋路系統
//!
//! 提供基於網格的 A* 尋路功能，用於行人導航。

use bevy::prelude::*;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

// ============================================================================
// A* 尋路系統
// ============================================================================

/// A* 節點（用於優先佇列）
#[derive(Clone, Copy, Eq, PartialEq)]
struct AStarNode {
    pos: (usize, usize),
    f_cost: i32,
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_cost.cmp(&self.f_cost) // 反向以取得最小堆
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A* 四方向與對角線移動
const ASTAR_DIRECTIONS: [(i32, i32); 8] = [
    (-1, 0), (1, 0), (0, -1), (0, 1),  // 四方向
    (-1, -1), (-1, 1), (1, -1), (1, 1), // 對角線
];

/// 計算曼哈頓距離啟發式
fn astar_heuristic(pos: (usize, usize), goal: (usize, usize)) -> i32 {
    let dx = (pos.0 as i32 - goal.0 as i32).abs();
    let dz = (pos.1 as i32 - goal.1 as i32).abs();
    (dx + dz) * 10
}

/// 簡化路徑（移除共線點）
fn simplify_path(path: Vec<Vec3>) -> Vec<Vec3> {
    if path.len() <= 2 {
        return path;
    }

    let mut simplified = vec![path[0]];
    for i in 1..path.len() - 1 {
        let prev_dir = (path[i] - path[i - 1]).normalize_or_zero();
        let next_dir = (path[i + 1] - path[i]).normalize_or_zero();
        if prev_dir.dot(next_dir) < 0.99 {
            simplified.push(path[i]);
        }
    }
    if let Some(&last) = path.last() {
        simplified.push(last);
    }
    simplified
}

/// 重建 A* 路徑
fn reconstruct_path(
    came_from: &HashMap<(usize, usize), (usize, usize)>,
    goal_grid: (usize, usize),
    grid: &PathfindingGrid,
) -> Vec<Vec3> {
    let mut path = Vec::new();
    let mut pos = goal_grid;
    path.push(grid.grid_to_world(pos.0, pos.1));

    while let Some(&prev) = came_from.get(&pos) {
        pos = prev;
        path.push(grid.grid_to_world(pos.0, pos.1));
    }

    path.reverse();
    simplify_path(path)
}

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

    /// 檢查對角線移動是否有效（鄰近格子需可通行）
    fn is_diagonal_valid(&self, current: (usize, usize), neighbor: (usize, usize)) -> bool {
        self.is_walkable(current.0, neighbor.1) && self.is_walkable(neighbor.0, current.1)
    }

    /// 計算鄰居座標（若有效）
    fn get_neighbor(&self, current: (usize, usize), dx: i32, dz: i32) -> Option<(usize, usize)> {
        let nx = current.0 as i32 + dx;
        let nz = current.1 as i32 + dz;

        if nx < 0 || nz < 0 {
            return None;
        }

        let neighbor = (nx as usize, nz as usize);

        if !self.is_walkable(neighbor.0, neighbor.1) {
            return None;
        }

        // 對角線移動需要鄰近格子也可通行
        let is_diagonal = dx != 0 && dz != 0;
        if is_diagonal && !self.is_diagonal_valid(current, neighbor) {
            return None;
        }

        Some(neighbor)
    }

    /// A* 尋路
    pub fn find_path(&self, start: Vec3, goal: Vec3) -> Option<Vec<Vec3>> {
        let start_grid = self.world_to_grid(start)?;
        let goal_grid = self.world_to_grid(goal)?;

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<(usize, usize), (usize, usize)> = HashMap::new();
        let mut g_score: HashMap<(usize, usize), i32> = HashMap::new();

        g_score.insert(start_grid, 0);
        open_set.push(AStarNode {
            pos: start_grid,
            f_cost: astar_heuristic(start_grid, goal_grid),
        });

        while let Some(current) = open_set.pop() {
            if current.pos == goal_grid {
                return Some(reconstruct_path(&came_from, goal_grid, self));
            }

            self.explore_neighbors(
                current.pos,
                goal_grid,
                &mut open_set,
                &mut came_from,
                &mut g_score,
            );
        }

        None
    }

    /// 探索當前節點的所有鄰居
    fn explore_neighbors(
        &self,
        current_pos: (usize, usize),
        goal_grid: (usize, usize),
        open_set: &mut BinaryHeap<AStarNode>,
        came_from: &mut HashMap<(usize, usize), (usize, usize)>,
        g_score: &mut HashMap<(usize, usize), i32>,
    ) {
        let current_g = *g_score.get(&current_pos).unwrap_or(&i32::MAX);

        for (dx, dz) in ASTAR_DIRECTIONS.iter() {
            let Some(neighbor) = self.get_neighbor(current_pos, *dx, *dz) else {
                continue;
            };

            let is_diagonal = *dx != 0 && *dz != 0;
            let move_cost = if is_diagonal { 14 } else { 10 };
            let tentative_g = current_g + move_cost;

            if tentative_g >= *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                continue;
            }

            came_from.insert(neighbor, current_pos);
            g_score.insert(neighbor, tentative_g);
            open_set.push(AStarNode {
                pos: neighbor,
                f_cost: tentative_g + astar_heuristic(neighbor, goal_grid),
            });
        }
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
    /// 建立新實例
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
