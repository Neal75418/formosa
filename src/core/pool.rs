//! 通用實體池（物件池模式）

use bevy::prelude::*;

/// 通用實體物件池
///
/// 避免頻繁的 spawn/despawn 造成記憶體分配開銷。
/// 實體結束生命週期時歸還池中重用，而非銷毀。
///
/// # 使用範例
/// ```ignore
/// let mut pool = EntityPool::new(100);
/// // 取得實體
/// if let Some(entity) = pool.acquire() {
///     // 使用實體...
///     pool.confirm_acquire(entity);
/// }
/// // 歸還實體
/// pool.release(entity);
/// ```
#[derive(Default, Clone)]
pub struct EntityPool {
    /// 可用的實體（已隱藏/閒置）
    pub available: Vec<Entity>,
    /// 正在使用的實體
    pub in_use: Vec<Entity>,
    /// 池最大大小
    pub max_size: usize,
}

impl EntityPool {
    /// 創建指定大小的實體池
    pub fn new(max_size: usize) -> Self {
        Self {
            available: Vec::with_capacity(max_size),
            in_use: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// 從池中取得一個實體（僅標記為候選）
    pub fn acquire(&mut self) -> Option<Entity> {
        self.available.pop()
    }

    /// 確認取得實體（將實體加入使用中列表）
    pub fn confirm_acquire(&mut self, entity: Entity) {
        self.in_use.push(entity);
    }

    /// 取消取得（將實體退回可用列表）
    pub fn cancel_acquire(&mut self, entity: Entity) {
        self.available.push(entity);
    }

    /// 歸還實體到池中
    ///
    /// 使用 swap_remove 保持 O(1) 移除，搜索為 O(n)
    pub fn release(&mut self, entity: Entity) {
        if let Some(idx) = self.in_use.iter().position(|&e| e == entity) {
            self.in_use.swap_remove(idx);
            if self.available.len() < self.max_size {
                self.available.push(entity);
            }
        }
    }

    /// 清理無效實體（當外部系統刪除了池中的實體時使用）
    pub fn cleanup_invalid(&mut self, is_valid: impl Fn(Entity) -> bool) {
        self.in_use.retain(|&e| is_valid(e));
        self.available.retain(|&e| is_valid(e));
    }

    /// 取得目前使用中的實體數量
    pub fn active_count(&self) -> usize {
        self.in_use.len()
    }

    /// 取得池中可用的實體數量
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// 檢查是否可以生成更多實體
    pub fn can_spawn(&self) -> bool {
        !self.available.is_empty() || self.in_use.len() < self.max_size
    }

    /// 檢查池是否還有空間
    pub fn has_capacity(&self) -> bool {
        self.available.len() < self.max_size || !self.available.is_empty()
    }
}
