//! AI 狀態機

use bevy::prelude::*;

/// AI 狀態
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AiState {
    #[default]
    Idle,       // 閒置：站在原地
    Patrol,     // 巡邏：沿路徑移動
    Alert,      // 警戒：聽到聲音，搜索中
    Chase,      // 追逐：看到目標，追趕中
    Attack,     // 攻擊：在攻擊範圍內，開火
    Flee,       // 逃跑：血量過低
    TakingCover, // 躲掩體：血量低時尋找掩體
}

impl AiState {
    /// 檢查是否可以從當前狀態轉換到目標狀態
    pub fn can_transition_to(&self, target: &AiState) -> bool {
        if self == target {
            return true;
        }
        matches!(
            (self, target),
            // Idle 可以進入巡邏、警戒，或直接看到目標時追逐/逃跑
            (AiState::Idle, AiState::Patrol | AiState::Alert | AiState::Chase | AiState::Flee)
            // Patrol 可以回到閒置、進入警戒，或直接看到目標時追逐/逃跑
            | (AiState::Patrol, AiState::Idle | AiState::Alert | AiState::Chase | AiState::Flee)
            // Alert 是中樞狀態，可以轉換到大部分狀態
            | (AiState::Alert, AiState::Chase | AiState::Attack | AiState::Flee | AiState::Idle | AiState::Patrol)
            // Chase 可以進入攻擊、回到警戒或開始逃跑
            | (AiState::Chase, AiState::Attack | AiState::Alert | AiState::Flee)
            // Attack 可以追逐、警戒、逃跑或躲掩體
            | (AiState::Attack, AiState::Chase | AiState::Alert | AiState::Flee | AiState::TakingCover)
            // Flee 可以回到閒置或警戒（脫離威脅後）
            | (AiState::Flee, AiState::Idle | AiState::Alert)
            // TakingCover 可以回到警戒、攻擊或逃跑
            | (AiState::TakingCover, AiState::Alert | AiState::Attack | AiState::Flee)
        )
    }
}

/// AI 行為組件
#[derive(Component, Debug)]
pub struct AiBehavior {
    /// 當前狀態
    pub state: AiState,
    /// 狀態計時器
    pub state_timer: f32,
    /// 上次狀態變更時間
    pub last_state_change: f32,
    /// 目標實體
    pub target: Option<Entity>,
    /// 上次看到目標的位置
    pub last_known_target_pos: Option<Vec3>,
    /// 上次看到目標的時間
    pub last_seen_time: f32,
    /// 逃跑血量閾值 (百分比)
    pub flee_threshold: f32,
    /// 是否已開始逃跑
    pub is_fleeing: bool,
    /// 生成保護時間（剛生成的敵人不會立即攻擊）
    pub spawn_protection: f32,
}

impl Default for AiBehavior {
    fn default() -> Self {
        Self {
            state: AiState::Idle,
            state_timer: 0.0,
            last_state_change: 0.0,
            target: None,
            last_known_target_pos: None,
            last_seen_time: 0.0,
            flee_threshold: 0.2, // 血量低於 20% 逃跑
            is_fleeing: false,
            spawn_protection: 2.0, // 生成後 2 秒內不會攻擊
        }
    }
}

impl AiBehavior {
    /// 設定新狀態
    pub fn set_state(&mut self, new_state: AiState, time: f32) {
        if self.state != new_state {
            debug_assert!(
                self.state.can_transition_to(&new_state),
                "Invalid AI state transition: {:?} -> {:?}",
                self.state,
                new_state
            );
            self.state = new_state;
            self.state_timer = 0.0;
            self.last_state_change = time;
        }
    }

    /// 更新狀態計時器
    pub fn tick(&mut self, dt: f32) {
        self.state_timer += dt;
        // 更新生成保護時間
        if self.spawn_protection > 0.0 {
            self.spawn_protection -= dt;
        }
    }

    /// 檢查是否還在生成保護期
    pub fn is_spawn_protected(&self) -> bool {
        self.spawn_protection > 0.0
    }

    /// 記錄看到目標
    pub fn see_target(&mut self, target: Entity, position: Vec3, time: f32) {
        self.target = Some(target);
        self.last_known_target_pos = Some(position);
        self.last_seen_time = time;
    }

    /// 失去目標（超過時間沒看到）
    pub fn lose_target(&mut self, current_time: f32, timeout: f32) -> bool {
        current_time - self.last_seen_time > timeout
    }
}
