//! 通緝系統組件定義

#![allow(dead_code)] // Phase 5+ 預留功能

use bevy::prelude::*;

/// 全局通緝等級資源
#[derive(Resource, Default)]
pub struct WantedLevel {
    /// 當前星級 (0-5)
    pub stars: u8,
    /// 熱度累積 (0.0-100.0)，決定星級
    pub heat: f32,
    /// 場上警察數量
    pub police_count: u32,
    /// 消退計時器（秒）
    pub cooldown_timer: f32,
    /// 最後犯罪時間
    pub last_crime_time: f32,
    /// 最後被警察看到的位置
    pub player_last_seen_pos: Option<Vec3>,
    /// 是否被警察看到
    pub player_visible: bool,
    /// 搜索區域中心
    pub search_center: Option<Vec3>,
    /// 搜索區域半徑
    pub search_radius: f32,
    /// 搜索區域剩餘時間（秒）- 超時後清除搜索區域
    pub search_timer: f32,
}

impl WantedLevel {
    /// 根據熱度計算星級
    pub fn calculate_stars(&self) -> u8 {
        match self.heat as u32 {
            0..=19 => 0,
            20..=39 => 1,
            40..=59 => 2,
            60..=79 => 3,
            80..=99 => 4,
            _ => 5,
        }
    }

    /// 增加熱度
    pub fn add_heat(&mut self, amount: f32) {
        self.heat = (self.heat + amount).min(100.0);
        self.stars = self.calculate_stars();
    }

    /// 減少熱度
    pub fn reduce_heat(&mut self, amount: f32) {
        self.heat = (self.heat - amount).max(0.0);
        self.stars = self.calculate_stars();
    }

    /// 根據星級獲取目標警察數量
    pub fn target_police_count(&self) -> u32 {
        match self.stars {
            0 => 0,
            1 => 2,
            2 => 4,
            3 => 6,
            4 => 8,
            5 => 10,
            _ => 0,
        }
    }

    /// 獲取消退所需時間（秒）
    /// 高星級需要更長時間才能消退，增加挑戰性
    pub fn cooldown_duration(&self) -> f32 {
        match self.stars {
            1 => 10.0,
            2 => 15.0,
            3 => 20.0,
            4 => 40.0,  // 提高：4星應該更難消退
            5 => 60.0,  // 提高：5星需要很長時間
            _ => 5.0,
        }
    }
}

/// 警察配置資源
#[derive(Resource)]
pub struct PoliceConfig {
    /// 警察生成間隔（秒）
    pub spawn_interval: f32,
    /// 上次生成時間
    pub last_spawn_time: f32,
    /// 警察生成距離範圍（距玩家）
    pub spawn_distance_min: f32,
    pub spawn_distance_max: f32,
    /// 警察消失距離
    pub despawn_distance: f32,
    /// 警察視野範圍
    pub vision_range: f32,
    /// 警察視野角度（弧度）
    pub vision_fov: f32,
    /// 警察攻擊範圍
    pub attack_range: f32,
    /// 警察移動速度
    pub walk_speed: f32,
    pub run_speed: f32,
    // === 戰鬥配置 ===
    /// 警察傷害值
    pub damage: f32,
    /// 攻擊冷卻時間（秒）
    pub attack_cooldown: f32,
    /// 基礎命中率 (0.0-1.0)
    pub base_hit_chance: f32,
    /// 距離命中率懲罰係數
    pub distance_hit_penalty: f32,
}

impl Default for PoliceConfig {
    fn default() -> Self {
        Self {
            spawn_interval: 3.0,
            last_spawn_time: 0.0,
            spawn_distance_min: 30.0,
            spawn_distance_max: 50.0,
            despawn_distance: 80.0,
            vision_range: 40.0,
            vision_fov: std::f32::consts::PI / 3.0, // 60 度
            attack_range: 25.0,
            walk_speed: 3.0,
            run_speed: 6.0,
            // 戰鬥配置
            damage: 15.0,
            attack_cooldown: 1.5,
            base_hit_chance: 0.28,        // 降低：避免 6 警察秒殺玩家
            distance_hit_penalty: 0.2,    // 降低：讓距離衰減更平滑
        }
    }
}

/// 警察 NPC 組件
#[derive(Component)]
pub struct PoliceOfficer {
    /// 警察狀態
    pub state: PoliceState,
    /// 巡邏路徑（TODO: 尚未實現，警察目前使用隨機遊走）
    pub patrol_route: Vec<Vec3>,
    /// 當前巡邏點索引（TODO: 配合 patrol_route 實現）
    pub patrol_index: usize,
    /// 是否正在追捕玩家
    pub target_player: bool,
    /// 搜索計時器
    pub search_timer: f32,
    /// 攻擊冷卻
    pub attack_cooldown: f32,
    /// 警察類型
    pub officer_type: PoliceType,
    /// 無線電呼叫冷卻（秒）
    pub radio_cooldown: f32,
    /// 是否已收到無線電通知
    pub radio_alerted: bool,
    /// 無線電通知的玩家位置
    pub radio_alert_position: Option<Vec3>,
}

impl Default for PoliceOfficer {
    fn default() -> Self {
        Self {
            state: PoliceState::Patrolling,
            patrol_route: Vec::new(),
            patrol_index: 0,
            target_player: false,
            search_timer: 0.0,
            attack_cooldown: 0.0,
            officer_type: PoliceType::Patrol,
            radio_cooldown: 0.0,
            radio_alerted: false,
            radio_alert_position: None,
        }
    }
}

/// 警察狀態
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PoliceState {
    /// 正常巡邏 (0星)
    #[default]
    Patrolling,
    /// 警覺狀態（聽到槍聲）
    Alerted,
    /// 追捕中（看到玩家）
    Pursuing,
    /// 搜索區域（失去視線）
    Searching,
    /// 戰鬥中（攻擊距離內）
    Engaging,
    /// 返回巡邏
    Returning,
}

/// 警察類型
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PoliceType {
    /// 巡邏警察（步行）
    #[default]
    Patrol,
    /// 快速反應（跑步）
    Swat,
    /// 警車警察
    Vehicular,
}

/// 警察視覺資源（預加載的 Mesh 和 Material）
#[derive(Resource)]
pub struct PoliceVisuals {
    pub body_mesh: Handle<Mesh>,
    pub head_mesh: Handle<Mesh>,
    pub arm_mesh: Handle<Mesh>,
    pub leg_mesh: Handle<Mesh>,
    pub uniform_material: Handle<StandardMaterial>,
    pub skin_material: Handle<StandardMaterial>,
    pub badge_material: Handle<StandardMaterial>,
}

/// 搜索區域標記組件
#[derive(Component)]
pub struct SearchZone {
    pub center: Vec3,
    pub radius: f32,
    pub lifetime: f32,
}

/// 通緝等級 HUD 組件
#[derive(Component)]
pub struct WantedHud;

/// 單個星星組件
#[derive(Component)]
pub struct WantedStar {
    pub index: u8,
}
