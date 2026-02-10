//! 空間哈希網格系統
//!
//! 將 O(n²) 的鄰近查詢優化為 O(1)。
//! 用於：行人/車輛碰撞、警察視野檢測、爆炸範圍檢測等。
#![allow(dead_code)]


use bevy::prelude::*;
use std::collections::HashMap;

// ============================================================================
// 常數
// ============================================================================

/// 預設網格大小（米）- 應大於最大查詢半徑
pub const DEFAULT_CELL_SIZE: f32 = 10.0;

// ============================================================================
// 空間哈希網格
// ============================================================================

/// 網格座標
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellCoord {
    pub x: i32,
    pub z: i32,
}

impl CellCoord {
    /// 建立新實例
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// 從世界座標計算網格座標
    pub fn from_world(pos: Vec3, cell_size: f32) -> Self {
        Self {
            x: (pos.x / cell_size).floor() as i32,
            z: (pos.z / cell_size).floor() as i32,
        }
    }
}

/// 空間哈希網格資源
///
/// 使用方式：
/// 1. 每幀開始時呼叫 `clear()` 清空
/// 2. 插入所有需要追蹤的實體 `insert(entity, position)`
/// 3. 查詢時呼叫 `query_radius()` 或 `query_cells()`
#[derive(Resource)]
pub struct SpatialHashGrid<T: Clone + Copy + PartialEq + Eq + std::hash::Hash = Entity> {
    /// 網格大小（米）
    pub cell_size: f32,
    /// 哈希表：網格座標 -> 實體列表
    cells: HashMap<CellCoord, Vec<(T, Vec3)>>,
    /// 快取：實體 -> 網格座標（用於快速移除）
    entity_cells: HashMap<T, CellCoord>,
}

impl<T: Clone + Copy + PartialEq + Eq + std::hash::Hash> Default for SpatialHashGrid<T> {
    fn default() -> Self {
        Self::with_default_size()
    }
}

impl<T: Clone + Copy + PartialEq + Eq + std::hash::Hash> SpatialHashGrid<T> {
    /// 建立指定大小的空間哈希網格
    ///
    /// # Panics
    /// 如果 `cell_size <= 0` 會 panic
    pub fn new(cell_size: f32) -> Self {
        assert!(
            cell_size > 0.0,
            "cell_size must be positive, got {}",
            cell_size
        );
        Self {
            cell_size,
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    /// 建立預設大小（10m）的空間哈希網格
    pub fn with_default_size() -> Self {
        Self::new(DEFAULT_CELL_SIZE)
    }

    /// 建立指定大小和預期容量的空間哈希網格
    pub fn with_capacity(cell_size: f32, capacity: usize) -> Self {
        assert!(
            cell_size > 0.0,
            "cell_size must be positive, got {}",
            cell_size
        );
        Self {
            cell_size,
            cells: HashMap::with_capacity(capacity / 4), // 假設每個 cell 平均 4 個實體
            entity_cells: HashMap::with_capacity(capacity),
        }
    }

    /// 清空所有資料（每幀開始時呼叫）
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }

    /// 插入實體到網格
    pub fn insert(&mut self, entity: T, position: Vec3) {
        let coord = CellCoord::from_world(position, self.cell_size);

        self.cells
            .entry(coord)
            .or_default()
            .push((entity, position));

        self.entity_cells.insert(entity, coord);
    }

    /// 批量插入（效能更好）
    ///
    /// 對於已知數量的實體，會預先分配記憶體。
    pub fn insert_batch<I>(&mut self, entities: I)
    where
        I: IntoIterator<Item = (T, Vec3)>,
    {
        let entities = entities.into_iter();
        // 嘗試預先分配（如果迭代器提供 size_hint）
        let (lower, _) = entities.size_hint();
        if lower > 0 {
            self.entity_cells.reserve(lower);
        }

        for (entity, position) in entities {
            self.insert(entity, position);
        }
    }

    /// 移除實體
    pub fn remove(&mut self, entity: T) {
        if let Some(coord) = self.entity_cells.remove(&entity) {
            if let Some(cell) = self.cells.get_mut(&coord) {
                cell.retain(|(e, _)| *e != entity);
            }
        }
    }

    // === 查詢輔助函數 ===

    /// 處理單個網格中的實體，對在半徑內的實體調用處理函數
    #[inline]
    fn process_cell_in_radius<R>(
        entities: &[(T, Vec3)],
        center: Vec3,
        radius_sq: f32,
        f: &mut impl FnMut(T, Vec3, f32) -> Option<R>,
    ) -> Option<R> {
        for &(entity, pos) in entities {
            let dist_sq = center.distance_squared(pos);
            if dist_sq > radius_sq {
                continue;
            }
            if let Some(result) = f(entity, pos, dist_sq) {
                return Some(result);
            }
        }
        None
    }

    /// 遍歷指定半徑內的所有實體，對每個實體調用處理函數
    ///
    /// 這是所有半徑查詢的核心邏輯，避免重複的巢狀迴圈。
    /// 處理函數返回 `Some(result)` 時會立即返回該結果（用於提前終止）。
    #[inline]
    pub fn for_each_in_radius<R>(
        &self,
        center: Vec3,
        radius: f32,
        mut f: impl FnMut(T, Vec3, f32) -> Option<R>,
    ) -> Option<R> {
        let radius_sq = radius * radius;
        let cells_needed = (radius / self.cell_size).ceil() as i32;
        let center_coord = CellCoord::from_world(center, self.cell_size);

        for dx in -cells_needed..=cells_needed {
            for dz in -cells_needed..=cells_needed {
                let coord = CellCoord::new(center_coord.x + dx, center_coord.z + dz);
                let Some(entities) = self.cells.get(&coord) else {
                    continue;
                };

                if let Some(result) =
                    Self::process_cell_in_radius(entities, center, radius_sq, &mut f)
                {
                    return Some(result);
                }
            }
        }
        None
    }

    /// 查詢指定半徑內的所有實體
    ///
    /// 時間複雜度：O(k)，k = 查詢區域內的實體數量
    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<(T, Vec3, f32)> {
        let mut results = Vec::new();
        self.for_each_in_radius(center, radius, |entity, pos, dist_sq| {
            results.push((entity, pos, dist_sq));
            None::<()>
        });
        results
    }

    /// 查詢指定半徑內的所有實體（不含距離平方）
    pub fn query_radius_entities(&self, center: Vec3, radius: f32) -> Vec<T> {
        self.query_radius(center, radius)
            .into_iter()
            .map(|(e, _, _)| e)
            .collect()
    }

    /// 查詢最近的實體
    pub fn query_nearest(&self, center: Vec3, radius: f32) -> Option<(T, Vec3, f32)> {
        self.query_radius(center, radius)
            .into_iter()
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// 檢查指定半徑內是否有任何實體（不分配記憶體）
    ///
    /// 比 `!query_radius().is_empty()` 更有效率，因為找到第一個就返回。
    pub fn has_entity_in_radius(&self, center: Vec3, radius: f32) -> bool {
        self.for_each_in_radius(center, radius, |_, _, _| Some(()))
            .is_some()
    }

    /// 計算指定半徑內的實體數量（不分配記憶體）
    pub fn count_in_radius(&self, center: Vec3, radius: f32) -> usize {
        let mut count = 0;
        self.for_each_in_radius(center, radius, |_, _, _| {
            count += 1;
            None::<()>
        });
        count
    }

    /// 查詢指定網格及其鄰居
    pub fn query_cell_with_neighbors(&self, coord: CellCoord) -> impl Iterator<Item = &(T, Vec3)> {
        let coords: Vec<CellCoord> = (-1..=1)
            .flat_map(|dx| (-1..=1).map(move |dz| CellCoord::new(coord.x + dx, coord.z + dz)))
            .collect();

        coords
            .into_iter()
            .filter_map(|c| self.cells.get(&c))
            .flatten()
    }

    /// 取得網格內的實體數量
    pub fn len(&self) -> usize {
        self.cells.values().map(|v| v.len()).sum()
    }

    /// 檢查是否為空
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// 取得佔用的網格數量
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

// ============================================================================
// 預定義的空間哈希資源
// ============================================================================

/// 生成空間哈希 wrapper 類型的巨集
macro_rules! define_spatial_hash {
    ($(#[$attr:meta])* $name:ident, $cell_size:expr) => {
        $(#[$attr])*
        #[derive(Resource)]
        pub struct $name(pub SpatialHashGrid<Entity>);

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            /// 建立新實例
            pub fn new() -> Self {
                Self(SpatialHashGrid::new($cell_size))
            }
        }

        impl std::ops::Deref for $name {
            type Target = SpatialHashGrid<Entity>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

define_spatial_hash!(
    /// 車輛空間哈希（用於行人碰撞檢測）
    VehicleSpatialHash,
    15.0
);

define_spatial_hash!(
    /// 行人空間哈希（用於恐慌傳播、爆炸範圍等）
    PedestrianSpatialHash,
    10.0
);

define_spatial_hash!(
    /// 警察空間哈希（用於玩家視野檢測）
    PoliceSpatialHash,
    20.0
);

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_coord_from_world() {
        let coord = CellCoord::from_world(Vec3::new(15.0, 0.0, 25.0), 10.0);
        assert_eq!(coord.x, 1);
        assert_eq!(coord.z, 2);

        let coord = CellCoord::from_world(Vec3::new(-5.0, 0.0, -15.0), 10.0);
        assert_eq!(coord.x, -1);
        assert_eq!(coord.z, -2);
    }

    #[test]
    fn test_insert_and_query() {
        let mut grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(10.0);

        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);
        let e3 = Entity::from_bits(3);

        grid.insert(e1, Vec3::new(0.0, 0.0, 0.0));
        grid.insert(e2, Vec3::new(5.0, 0.0, 5.0));
        grid.insert(e3, Vec3::new(100.0, 0.0, 100.0));

        // 查詢半徑 10 內的實體
        let results = grid.query_radius(Vec3::ZERO, 10.0);
        assert_eq!(results.len(), 2); // e1 和 e2

        // e3 應該不在範圍內
        let entities: Vec<Entity> = results.iter().map(|(e, _, _)| *e).collect();
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
        assert!(!entities.contains(&e3));
    }

    #[test]
    fn test_query_nearest() {
        let mut grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(10.0);

        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);

        grid.insert(e1, Vec3::new(5.0, 0.0, 0.0));
        grid.insert(e2, Vec3::new(3.0, 0.0, 0.0));

        let nearest = grid.query_nearest(Vec3::ZERO, 10.0);
        assert!(nearest.is_some());
        assert_eq!(nearest.unwrap().0, e2); // e2 更近
    }

    #[test]
    fn test_clear() {
        let mut grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(10.0);

        grid.insert(Entity::from_bits(1), Vec3::ZERO);
        assert_eq!(grid.len(), 1);

        grid.clear();
        assert_eq!(grid.len(), 0);
        assert!(grid.is_empty());
    }

    #[test]
    fn test_remove() {
        let mut grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(10.0);

        let e1 = Entity::from_bits(1);
        let e2 = Entity::from_bits(2);

        grid.insert(e1, Vec3::ZERO);
        grid.insert(e2, Vec3::new(5.0, 0.0, 0.0));
        assert_eq!(grid.len(), 2);

        grid.remove(e1);
        assert_eq!(grid.len(), 1);

        let results = grid.query_radius(Vec3::ZERO, 10.0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, e2);
    }

    #[test]
    fn test_batch_insert() {
        let mut grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(10.0);

        let entities = vec![
            (Entity::from_bits(1), Vec3::new(0.0, 0.0, 0.0)),
            (Entity::from_bits(2), Vec3::new(10.0, 0.0, 0.0)),
            (Entity::from_bits(3), Vec3::new(20.0, 0.0, 0.0)),
        ];

        grid.insert_batch(entities);
        assert_eq!(grid.len(), 3);
    }

    #[test]
    fn test_cross_cell_boundary() {
        let mut grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(10.0);

        // 實體在邊界附近
        let e1 = Entity::from_bits(1);
        grid.insert(e1, Vec3::new(9.5, 0.0, 0.0)); // 接近網格邊界

        // 從另一個網格查詢應該能找到
        let results = grid.query_radius(Vec3::new(10.5, 0.0, 0.0), 5.0);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_has_entity_in_radius() {
        let mut grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(10.0);

        grid.insert(Entity::from_bits(1), Vec3::new(5.0, 0.0, 5.0));

        assert!(grid.has_entity_in_radius(Vec3::ZERO, 10.0));
        assert!(!grid.has_entity_in_radius(Vec3::ZERO, 3.0)); // 太遠
        assert!(!grid.has_entity_in_radius(Vec3::new(100.0, 0.0, 100.0), 10.0));
        // 完全不同區域
    }

    #[test]
    fn test_count_in_radius() {
        let mut grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(10.0);

        grid.insert(Entity::from_bits(1), Vec3::new(1.0, 0.0, 0.0));
        grid.insert(Entity::from_bits(2), Vec3::new(2.0, 0.0, 0.0));
        grid.insert(Entity::from_bits(3), Vec3::new(3.0, 0.0, 0.0));
        grid.insert(Entity::from_bits(4), Vec3::new(100.0, 0.0, 0.0)); // 很遠

        assert_eq!(grid.count_in_radius(Vec3::ZERO, 5.0), 3);
        assert_eq!(grid.count_in_radius(Vec3::ZERO, 1.5), 1);
        assert_eq!(grid.count_in_radius(Vec3::ZERO, 0.5), 0);
    }

    #[test]
    fn test_with_capacity() {
        let grid: SpatialHashGrid<Entity> = SpatialHashGrid::with_capacity(10.0, 100);
        assert!(grid.is_empty());
        assert_eq!(grid.cell_size, 10.0);
    }

    #[test]
    #[should_panic(expected = "cell_size must be positive")]
    fn test_invalid_cell_size() {
        let _grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(0.0);
    }

    #[test]
    #[should_panic(expected = "cell_size must be positive")]
    fn test_negative_cell_size() {
        let _grid: SpatialHashGrid<Entity> = SpatialHashGrid::new(-5.0);
    }
}
