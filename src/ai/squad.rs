//! AI 小隊系統
//!
//! 實現 GTA 5 風格的 AI 包抄戰術，敵人會協調進攻。

#![allow(dead_code)]

use bevy::prelude::*;
use std::collections::HashMap;

// ============================================================================
// 小隊角色
// ============================================================================

/// 小隊成員角色
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SquadRole {
    /// 正面進攻者（預設）
    #[default]
    Rusher,
    /// 側翼包抄者
    Flanker,
    /// 遠距離狙擊/壓制
    Suppressor,
    /// 小隊長（協調指揮）
    Leader,
}

impl SquadRole {
    /// 取得角色的理想進攻角度（相對於目標正面）
    pub fn ideal_attack_angle(&self) -> f32 {
        match self {
            SquadRole::Rusher => 0.0,             // 正面
            SquadRole::Flanker => 90.0_f32.to_radians(),  // 側翼 90 度
            SquadRole::Suppressor => 10.0_f32.to_radians(), // 略偏正面，保持距離
            SquadRole::Leader => 20.0_f32.to_radians(),    // 稍偏，觀察全局
        }
    }

    /// 取得角色的理想攻擊距離
    pub fn ideal_attack_distance(&self) -> f32 {
        match self {
            SquadRole::Rusher => 5.0,      // 近距離突擊
            SquadRole::Flanker => 10.0,    // 中距離側翼
            SquadRole::Suppressor => 20.0, // 遠距離壓制
            SquadRole::Leader => 12.0,     // 中距離指揮
        }
    }

    /// 取得移動優先級（數字越高越優先移動）
    pub fn movement_priority(&self) -> u8 {
        match self {
            SquadRole::Flanker => 3,    // 側翼最先移動
            SquadRole::Rusher => 2,     // 正面其次
            SquadRole::Leader => 1,     // 隊長觀察
            SquadRole::Suppressor => 0, // 壓制者固定位置
        }
    }
}

// ============================================================================
// 小隊成員組件
// ============================================================================

/// 小隊成員組件
#[derive(Component, Debug)]
pub struct SquadMember {
    /// 小隊 ID（相同 ID 的敵人會協調行動）
    pub squad_id: u32,
    /// 在小隊中的角色
    pub role: SquadRole,
    /// 包抄目標位置（由小隊協調系統計算）
    pub flank_target: Option<Vec3>,
    /// 是否正在執行包抄
    pub is_flanking: bool,
    /// 包抄計時器（避免頻繁切換）
    pub flank_cooldown: f32,
    /// 與其他隊友的最小距離（避免擠在一起）
    pub min_ally_distance: f32,
}

impl Default for SquadMember {
    fn default() -> Self {
        Self {
            squad_id: 0,
            role: SquadRole::Rusher,
            flank_target: None,
            is_flanking: false,
            flank_cooldown: 0.0,
            min_ally_distance: 3.0,
        }
    }
}

impl SquadMember {
    /// 創建指定角色的小隊成員
    pub fn with_role(role: SquadRole) -> Self {
        Self {
            role,
            ..default()
        }
    }

    /// 設定小隊 ID
    pub fn in_squad(mut self, squad_id: u32) -> Self {
        self.squad_id = squad_id;
        self
    }

    /// 開始包抄
    pub fn start_flank(&mut self, target_pos: Vec3) {
        self.flank_target = Some(target_pos);
        self.is_flanking = true;
        self.flank_cooldown = 5.0; // 5 秒內不會重新計算包抄
    }

    /// 結束包抄
    pub fn end_flank(&mut self) {
        self.flank_target = None;
        self.is_flanking = false;
    }

    /// 更新計時器
    pub fn tick(&mut self, dt: f32) {
        if self.flank_cooldown > 0.0 {
            self.flank_cooldown -= dt;
        }
    }

    /// 是否可以開始新的包抄
    pub fn can_flank(&self) -> bool {
        self.flank_cooldown <= 0.0 && !self.is_flanking
    }
}

// ============================================================================
// 小隊管理器
// ============================================================================

/// 小隊管理器資源
/// 追蹤所有活躍小隊及其成員
#[derive(Resource, Default, Debug)]
pub struct SquadManager {
    /// 下一個可用的小隊 ID
    next_squad_id: u32,
    /// 小隊目標（小隊 ID -> 目標位置）
    pub squad_targets: HashMap<u32, Vec3>,
    /// 協調更新計時器
    pub coordination_timer: Timer,
}

impl SquadManager {
    pub fn new() -> Self {
        Self {
            next_squad_id: 1,
            squad_targets: HashMap::new(),
            coordination_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        }
    }

    /// 分配新的小隊 ID
    pub fn allocate_squad_id(&mut self) -> u32 {
        let id = self.next_squad_id;
        self.next_squad_id += 1;
        id
    }

    /// 設定小隊的主要目標
    pub fn set_squad_target(&mut self, squad_id: u32, target: Vec3) {
        self.squad_targets.insert(squad_id, target);
    }

    /// 清除小隊目標
    pub fn clear_squad_target(&mut self, squad_id: u32) {
        self.squad_targets.remove(&squad_id);
    }
}

// ============================================================================
// 包抄計算
// ============================================================================

/// 計算側翼進攻位置
///
/// 根據目標位置、隊友位置、角色類型計算最佳進攻位置
pub fn calculate_flank_position(
    my_pos: Vec3,
    target_pos: Vec3,
    role: SquadRole,
    ally_positions: &[Vec3],
    min_ally_distance: f32,
) -> Vec3 {
    // 計算從目標到自己的基礎方向
    let to_me = (my_pos - target_pos).normalize_or_zero();
    let distance = role.ideal_attack_distance();
    let angle = role.ideal_attack_angle();

    // 根據角色計算兩個可能的包抄位置（左右兩側）
    let left_dir = rotate_y(to_me, angle);
    let right_dir = rotate_y(to_me, -angle);

    let left_pos = target_pos + left_dir * distance;
    let right_pos = target_pos + right_dir * distance;

    // 選擇離隊友較遠的位置
    let left_min_dist = ally_positions
        .iter()
        .map(|p| left_pos.distance(*p))
        .fold(f32::MAX, f32::min);
    let right_min_dist = ally_positions
        .iter()
        .map(|p| right_pos.distance(*p))
        .fold(f32::MAX, f32::min);

    // 如果兩邊都太擠，選擇更遠的那個
    if left_min_dist > right_min_dist && left_min_dist >= min_ally_distance {
        left_pos
    } else if right_min_dist >= min_ally_distance {
        right_pos
    } else {
        // 都太擠了，直接走向目標
        target_pos + to_me * distance
    }
}

/// 繞 Y 軸旋轉向量
fn rotate_y(v: Vec3, angle: f32) -> Vec3 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    Vec3::new(
        v.x * cos_a - v.z * sin_a,
        v.y,
        v.x * sin_a + v.z * cos_a,
    )
}

/// 根據小隊大小分配角色
pub fn assign_squad_roles(member_count: usize) -> Vec<SquadRole> {
    match member_count {
        0 => vec![],
        1 => vec![SquadRole::Rusher],
        2 => vec![SquadRole::Rusher, SquadRole::Flanker],
        3 => vec![SquadRole::Leader, SquadRole::Rusher, SquadRole::Flanker],
        4 => vec![
            SquadRole::Leader,
            SquadRole::Rusher,
            SquadRole::Flanker,
            SquadRole::Flanker,
        ],
        _ => {
            // 5+ 人：1 隊長, 2 突擊, 2 側翼, 其餘壓制
            let mut roles = vec![
                SquadRole::Leader,
                SquadRole::Rusher,
                SquadRole::Rusher,
                SquadRole::Flanker,
                SquadRole::Flanker,
            ];
            for _ in 5..member_count {
                roles.push(SquadRole::Suppressor);
            }
            roles
        }
    }
}

// ============================================================================
// 包抄戰術品質評估
// ============================================================================

/// 評估當前包抄態勢的品質
/// 回傳 0.0-1.0 的分數，1.0 表示完美包抄
pub fn evaluate_flank_quality(
    target_pos: Vec3,
    ally_positions: &[Vec3],
) -> f32 {
    // 安全檢查：防止空陣列或單人情況
    // 至少需要 2 人才能形成包抄態勢
    if ally_positions.is_empty() || ally_positions.len() < 2 {
        return 0.0;
    }

    // 計算每個隊友相對於目標的角度
    let angles: Vec<f32> = ally_positions
        .iter()
        .map(|pos| {
            let dir = (*pos - target_pos).normalize_or_zero();
            dir.z.atan2(dir.x)
        })
        .collect();

    // 計算角度覆蓋範圍（理想情況是均勻分布在目標周圍）
    let mut sorted_angles = angles.clone();
    sorted_angles.sort_by(|a, b| a.total_cmp(b));

    // 計算相鄰角度之間的差異
    let mut max_gap = 0.0_f32;
    for i in 0..sorted_angles.len() {
        let next = (i + 1) % sorted_angles.len();
        let gap = if next == 0 {
            // 處理環繞
            (std::f32::consts::TAU - sorted_angles[i]) + sorted_angles[0]
        } else {
            sorted_angles[next] - sorted_angles[i]
        };
        max_gap = max_gap.max(gap);
    }

    // 理想的最大間隙是 2π / n
    let ideal_gap = std::f32::consts::TAU / ally_positions.len() as f32;

    // 品質分數：間隙越接近理想值越好
    (1.0 - (max_gap - ideal_gap).abs() / std::f32::consts::PI).clamp(0.0, 1.0)
}
