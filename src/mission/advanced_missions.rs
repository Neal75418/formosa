//! 進階任務類型
//!
//! 暗殺、護送、飛車追逐、拍照任務的資料結構和生成邏輯。

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use rand::Rng;

use super::data::{MissionData, MissionManager, MissionType};

// ============================================================================
// 暗殺任務
// ============================================================================

/// 暗殺目標難度
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetDifficulty {
    /// 普通目標（無護衛）
    Easy,
    /// 有 1-2 名護衛
    Medium,
    /// 有 3-4 名護衛 + 裝甲車
    Hard,
}

impl TargetDifficulty {
    /// 獎勵乘數
    pub fn reward_multiplier(&self) -> f32 {
        match self {
            TargetDifficulty::Easy => 1.0,
            TargetDifficulty::Medium => 1.5,
            TargetDifficulty::Hard => 2.5,
        }
    }

    /// 護衛數量
    pub fn guard_count(&self) -> u32 {
        match self {
            TargetDifficulty::Easy => 0,
            TargetDifficulty::Medium => 2,
            TargetDifficulty::Hard => 4,
        }
    }
}

/// 暗殺任務資料
#[derive(Clone, Debug)]
pub struct AssassinationData {
    /// 目標名稱
    pub target_name: String,
    /// 目標位置
    pub target_pos: Vec3,
    /// 難度
    pub difficulty: TargetDifficulty,
    /// 是否需要無聲擊殺（額外獎勵）
    pub silent_kill_bonus: bool,
    /// 時限（秒），None = 無時限
    pub time_limit: Option<f32>,
}

// ============================================================================
// 護送任務
// ============================================================================

/// 護送任務資料
#[derive(Clone, Debug)]
pub struct EscortData {
    /// 被護送者名稱
    pub vip_name: String,
    /// 出發位置
    pub start_pos: Vec3,
    /// 目的地
    pub destination: Vec3,
    /// VIP 最大 HP
    pub vip_max_hp: f32,
    /// VIP 當前 HP
    pub vip_hp: f32,
    /// 途中伏擊波數
    pub ambush_waves: u32,
    /// 是否使用車輛護送
    pub use_vehicle: bool,
}

impl EscortData {
    pub fn new(vip_name: String, start: Vec3, dest: Vec3, waves: u32) -> Self {
        Self {
            vip_name,
            start_pos: start,
            destination: dest,
            vip_max_hp: 100.0,
            vip_hp: 100.0,
            ambush_waves: waves,
            use_vehicle: false,
        }
    }

    /// VIP 受傷
    pub fn damage_vip(&mut self, amount: f32) {
        self.vip_hp = (self.vip_hp - amount).max(0.0);
    }

    /// VIP 是否存活
    pub fn vip_alive(&self) -> bool {
        self.vip_hp > 0.0
    }

    /// VIP HP 百分比
    pub fn vip_hp_ratio(&self) -> f32 {
        self.vip_hp / self.vip_max_hp
    }
}

// ============================================================================
// 飛車追逐
// ============================================================================

/// 飛車追逐資料
#[derive(Clone, Debug)]
pub struct ChaseData {
    /// 逃跑車輛描述
    pub target_vehicle: String,
    /// 追逐起點
    pub chase_start: Vec3,
    /// 逃跑路線途經點
    pub escape_route: Vec<Vec3>,
    /// 需要在多近距離攔截（米）
    pub intercept_radius: f32,
    /// 追逐時限（秒）
    pub time_limit: f32,
    /// 目標車速
    pub target_speed: f32,
}

impl ChaseData {
    /// 檢查是否成功攔截
    pub fn is_intercepted(&self, player_pos: Vec3, target_pos: Vec3) -> bool {
        player_pos.distance_squared(target_pos) <= self.intercept_radius * self.intercept_radius
    }
}

// ============================================================================
// 拍照任務
// ============================================================================

/// 拍照目標
#[derive(Clone, Debug)]
pub struct PhotoTarget {
    /// 目標描述
    pub description: String,
    /// 目標位置
    pub position: Vec3,
    /// 拍攝需要距離多近
    pub required_distance: f32,
    /// 是否需要特定角度（面朝方向）
    pub required_facing: Option<Vec3>,
    /// 是否已拍攝完成
    pub captured: bool,
}

/// 拍照任務資料
#[derive(Clone, Debug)]
pub struct PhotographyData {
    /// 任務主題
    pub theme: String,
    /// 需要拍攝的目標列表
    pub targets: Vec<PhotoTarget>,
    /// 已完成的拍照數量
    pub captured_count: u32,
}

impl PhotographyData {
    /// 建立新拍照任務
    pub fn new(theme: String, targets: Vec<PhotoTarget>) -> Self {
        Self {
            theme,
            targets,
            captured_count: 0,
        }
    }

    /// 嘗試拍攝目標（檢查距離）
    pub fn try_capture(&mut self, player_pos: Vec3) -> Option<String> {
        for target in &mut self.targets {
            if target.captured {
                continue;
            }
            let dist_sq = player_pos.distance_squared(target.position);
            if dist_sq <= target.required_distance * target.required_distance {
                target.captured = true;
                self.captured_count += 1;
                return Some(target.description.clone());
            }
        }
        None
    }

    /// 是否全部完成
    pub fn all_captured(&self) -> bool {
        self.targets.iter().all(|t| t.captured)
    }

    /// 完成進度（已拍/總數）
    pub fn progress(&self) -> (u32, u32) {
        (self.captured_count, self.targets.len() as u32)
    }
}

// ============================================================================
// MissionManager 擴展
// ============================================================================

impl MissionManager {
    /// 生成暗殺任務
    pub fn generate_assassination_mission(&mut self) -> MissionData {
        let mut rng = rand::rng();

        let targets = create_assassination_targets();
        let idx = rng.random_range(0..targets.len());
        let (name, pos, difficulty, base_reward) = &targets[idx];

        let id = self.next_mission_id;
        self.next_mission_id += 1;

        let reward = (*base_reward as f32 * difficulty.reward_multiplier()) as u32;
        let time_limit = match difficulty {
            TargetDifficulty::Easy => Some(120.0),
            TargetDifficulty::Medium => Some(180.0),
            TargetDifficulty::Hard => None, // 高難度無時限
        };

        MissionData {
            id,
            mission_type: MissionType::Assassination,
            title: format!("暗殺: {}", name),
            description: format!(
                "消滅目標 {}（護衛 {} 人）",
                name,
                difficulty.guard_count()
            ),
            start_pos: Vec3::ZERO,
            end_pos: *pos,
            reward,
            time_limit,
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        }
    }

    /// 生成護送任務
    pub fn generate_escort_mission(&mut self) -> MissionData {
        let mut rng = rand::rng();

        let vips = create_escort_vips();
        let idx = rng.random_range(0..vips.len());
        let (name, start, dest, waves, reward) = &vips[idx];

        let id = self.next_mission_id;
        self.next_mission_id += 1;

        let _escort = EscortData::new(name.clone(), *start, *dest, *waves);
        let distance = start.distance(*dest);
        let time_limit = (distance / 8.0).max(60.0); // 每 8 米 1 秒，最少 60 秒

        MissionData {
            id,
            mission_type: MissionType::Escort,
            title: format!("護送: {}", name),
            description: format!("護送 {} 安全抵達目的地（{} 波伏擊）", name, waves),
            start_pos: *start,
            end_pos: *dest,
            reward: *reward,
            time_limit: Some(time_limit),
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        }
    }

    /// 生成飛車追逐任務
    pub fn generate_chase_mission(&mut self) -> MissionData {
        let mut rng = rand::rng();

        let chases = create_chase_scenarios();
        let idx = rng.random_range(0..chases.len());
        let (desc, start, route, reward) = &chases[idx];

        let id = self.next_mission_id;
        self.next_mission_id += 1;

        let end_pos = route.last().copied().unwrap_or(*start);

        MissionData {
            id,
            mission_type: MissionType::ChaseDown,
            title: format!("追逐: {}", desc),
            description: format!("追上並攔截 {}", desc),
            start_pos: *start,
            end_pos,
            reward: *reward,
            time_limit: Some(90.0),
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        }
    }

    /// 生成拍照任務
    pub fn generate_photography_mission(&mut self) -> MissionData {
        let mut rng = rand::rng();

        let missions = create_photo_missions();
        let idx = rng.random_range(0..missions.len());
        let (theme, targets, reward) = &missions[idx];

        let id = self.next_mission_id;
        self.next_mission_id += 1;

        let start_pos = targets
            .first()
            .map(|t| t.position)
            .unwrap_or(Vec3::ZERO);

        MissionData {
            id,
            mission_type: MissionType::Photography,
            title: format!("拍照: {}", theme),
            description: format!("拍攝 {} 個指定場景（{}）", targets.len(), theme),
            start_pos,
            end_pos: start_pos,
            reward: *reward,
            time_limit: None, // 拍照任務無時限
            delivery_order: None,
            race_data: None,
            taxi_data: None,
        }
    }

    /// 刷新進階任務列表
    pub fn refresh_advanced_missions(&mut self) {
        // 各生成 1-2 個
        let assassination = self.generate_assassination_mission();
        self.available_missions.push(assassination);

        let escort = self.generate_escort_mission();
        self.available_missions.push(escort);

        let chase = self.generate_chase_mission();
        self.available_missions.push(chase);

        let photo = self.generate_photography_mission();
        self.available_missions.push(photo);
    }
}

// ============================================================================
// 預定義資料
// ============================================================================

/// 暗殺目標列表
fn create_assassination_targets() -> Vec<(String, Vec3, TargetDifficulty, u32)> {
    vec![
        (
            "黑道會計師".to_string(),
            Vec3::new(-30.0, 0.5, -40.0),
            TargetDifficulty::Easy,
            3000,
        ),
        (
            "毒品批發商".to_string(),
            Vec3::new(60.0, 0.5, 20.0),
            TargetDifficulty::Medium,
            5000,
        ),
        (
            "軍火走私頭目".to_string(),
            Vec3::new(-50.0, 0.5, 50.0),
            TargetDifficulty::Hard,
            8000,
        ),
        (
            "貪腐議員".to_string(),
            Vec3::new(40.0, 0.5, -30.0),
            TargetDifficulty::Medium,
            6000,
        ),
        (
            "仿冒品大王".to_string(),
            Vec3::new(-20.0, 0.5, 60.0),
            TargetDifficulty::Easy,
            2500,
        ),
    ]
}

/// 護送 VIP 列表
fn create_escort_vips() -> Vec<(String, Vec3, Vec3, u32, u32)> {
    vec![
        (
            "證人張先生".to_string(),
            Vec3::new(-40.0, 0.5, -20.0),
            Vec3::new(50.0, 0.5, 40.0),
            2, // 波數
            4000,
        ),
        (
            "外交官李小姐".to_string(),
            Vec3::new(30.0, 0.5, -50.0),
            Vec3::new(-60.0, 0.5, 30.0),
            3,
            6000,
        ),
        (
            "藝人陳大哥".to_string(),
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(70.0, 0.5, 60.0),
            1,
            2500,
        ),
        (
            "科學家王博士".to_string(),
            Vec3::new(-60.0, 0.5, -60.0),
            Vec3::new(40.0, 0.5, 50.0),
            4,
            8000,
        ),
    ]
}

/// 飛車追逐場景
fn create_chase_scenarios() -> Vec<(String, Vec3, Vec<Vec3>, u32)> {
    vec![
        (
            "黑色轎車".to_string(),
            Vec3::new(0.0, 0.5, 0.0),
            vec![
                Vec3::new(30.0, 0.5, 10.0),
                Vec3::new(60.0, 0.5, 30.0),
                Vec3::new(40.0, 0.5, 60.0),
            ],
            3500,
        ),
        (
            "改裝機車".to_string(),
            Vec3::new(-20.0, 0.5, -30.0),
            vec![
                Vec3::new(-40.0, 0.5, -10.0),
                Vec3::new(-60.0, 0.5, 20.0),
                Vec3::new(-30.0, 0.5, 50.0),
            ],
            2500,
        ),
        (
            "運鈔車".to_string(),
            Vec3::new(50.0, 0.5, 50.0),
            vec![
                Vec3::new(30.0, 0.5, 30.0),
                Vec3::new(0.0, 0.5, 10.0),
                Vec3::new(-30.0, 0.5, -20.0),
                Vec3::new(-60.0, 0.5, -50.0),
            ],
            5000,
        ),
    ]
}

/// 拍照任務資料
fn create_photo_missions() -> Vec<(String, Vec<PhotoTarget>, u32)> {
    vec![
        (
            "西門町地標巡禮".to_string(),
            vec![
                PhotoTarget {
                    description: "西門紅樓".to_string(),
                    position: Vec3::new(50.0, 0.5, 69.0),
                    required_distance: 15.0,
                    required_facing: None,
                    captured: false,
                },
                PhotoTarget {
                    description: "電影街".to_string(),
                    position: Vec3::new(-10.0, 0.5, -20.0),
                    required_distance: 15.0,
                    required_facing: None,
                    captured: false,
                },
                PhotoTarget {
                    description: "萬年大樓".to_string(),
                    position: Vec3::new(-68.0, 0.5, -12.0),
                    required_distance: 15.0,
                    required_facing: None,
                    captured: false,
                },
            ],
            1500,
        ),
        (
            "街頭美食攝影".to_string(),
            vec![
                PhotoTarget {
                    description: "阿宗麵線攤位".to_string(),
                    position: Vec3::new(-10.0, 0.5, -20.0),
                    required_distance: 8.0,
                    required_facing: None,
                    captured: false,
                },
                PhotoTarget {
                    description: "成都楊桃冰".to_string(),
                    position: Vec3::new(-15.0, 0.5, 30.0),
                    required_distance: 8.0,
                    required_facing: None,
                    captured: false,
                },
            ],
            1000,
        ),
        (
            "夜景特輯".to_string(),
            vec![
                PhotoTarget {
                    description: "霓虹招牌街".to_string(),
                    position: Vec3::new(0.0, 0.5, 0.0),
                    required_distance: 20.0,
                    required_facing: None,
                    captured: false,
                },
                PhotoTarget {
                    description: "紅樓夜景".to_string(),
                    position: Vec3::new(50.0, 0.5, 69.0),
                    required_distance: 25.0,
                    required_facing: None,
                    captured: false,
                },
                PhotoTarget {
                    description: "獅子林霓虹".to_string(),
                    position: Vec3::new(-69.0, 0.5, -66.0),
                    required_distance: 20.0,
                    required_facing: None,
                    captured: false,
                },
                PhotoTarget {
                    description: "漢中街入口".to_string(),
                    position: Vec3::new(-45.0, 0.5, 10.0),
                    required_distance: 15.0,
                    required_facing: None,
                    captured: false,
                },
            ],
            2000,
        ),
    ]
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_difficulty_reward_multiplier() {
        assert!((TargetDifficulty::Easy.reward_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!((TargetDifficulty::Medium.reward_multiplier() - 1.5).abs() < f32::EPSILON);
        assert!((TargetDifficulty::Hard.reward_multiplier() - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn target_difficulty_guard_count() {
        assert_eq!(TargetDifficulty::Easy.guard_count(), 0);
        assert_eq!(TargetDifficulty::Medium.guard_count(), 2);
        assert_eq!(TargetDifficulty::Hard.guard_count(), 4);
    }

    #[test]
    fn escort_data_damage_and_alive() {
        let mut escort = EscortData::new(
            "Test VIP".to_string(),
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            2,
        );

        assert!(escort.vip_alive());
        assert!((escort.vip_hp_ratio() - 1.0).abs() < f32::EPSILON);

        escort.damage_vip(30.0);
        assert!(escort.vip_alive());
        assert!((escort.vip_hp_ratio() - 0.7).abs() < f32::EPSILON);

        escort.damage_vip(80.0); // 超過剩餘 HP
        assert!(!escort.vip_alive());
        assert!(escort.vip_hp_ratio().abs() < f32::EPSILON);
    }

    #[test]
    fn chase_data_intercept() {
        let chase = ChaseData {
            target_vehicle: "Test Car".to_string(),
            chase_start: Vec3::ZERO,
            escape_route: vec![Vec3::new(100.0, 0.0, 0.0)],
            intercept_radius: 5.0,
            time_limit: 60.0,
            target_speed: 20.0,
        };

        // 太遠
        assert!(!chase.is_intercepted(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)));
        // 足夠近
        assert!(chase.is_intercepted(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)));
    }

    #[test]
    fn photography_try_capture() {
        let mut photo = PhotographyData::new(
            "Test".to_string(),
            vec![
                PhotoTarget {
                    description: "Target A".to_string(),
                    position: Vec3::new(10.0, 0.0, 0.0),
                    required_distance: 5.0,
                    required_facing: None,
                    captured: false,
                },
                PhotoTarget {
                    description: "Target B".to_string(),
                    position: Vec3::new(50.0, 0.0, 0.0),
                    required_distance: 5.0,
                    required_facing: None,
                    captured: false,
                },
            ],
        );

        assert_eq!(photo.progress(), (0, 2));
        assert!(!photo.all_captured());

        // 太遠
        let result = photo.try_capture(Vec3::ZERO);
        assert!(result.is_none());

        // 靠近 Target A
        let result = photo.try_capture(Vec3::new(8.0, 0.0, 0.0));
        assert_eq!(result, Some("Target A".to_string()));
        assert_eq!(photo.progress(), (1, 2));

        // 重複拍攝不算
        let result = photo.try_capture(Vec3::new(8.0, 0.0, 0.0));
        assert!(result.is_none());

        // 拍攝 Target B
        let result = photo.try_capture(Vec3::new(48.0, 0.0, 0.0));
        assert_eq!(result, Some("Target B".to_string()));
        assert!(photo.all_captured());
    }

    #[test]
    fn generate_assassination_mission() {
        let mut manager = MissionManager::default();
        let mission = manager.generate_assassination_mission();

        assert_eq!(mission.mission_type, MissionType::Assassination);
        assert!(mission.reward > 0);
    }

    #[test]
    fn generate_escort_mission() {
        let mut manager = MissionManager::default();
        let mission = manager.generate_escort_mission();

        assert_eq!(mission.mission_type, MissionType::Escort);
        assert!(mission.time_limit.is_some());
    }

    #[test]
    fn generate_chase_mission() {
        let mut manager = MissionManager::default();
        let mission = manager.generate_chase_mission();

        assert_eq!(mission.mission_type, MissionType::ChaseDown);
        assert_eq!(mission.time_limit, Some(90.0));
    }

    #[test]
    fn generate_photography_mission() {
        let mut manager = MissionManager::default();
        let mission = manager.generate_photography_mission();

        assert_eq!(mission.mission_type, MissionType::Photography);
        assert!(mission.time_limit.is_none());
    }

    #[test]
    fn refresh_advanced_missions_adds_four() {
        let mut manager = MissionManager::default();
        let initial = manager.available_missions.len();
        manager.refresh_advanced_missions();
        assert_eq!(manager.available_missions.len(), initial + 4);
    }

    #[test]
    fn mission_type_labels() {
        let record = super::super::data::CompletedMissionRecord {
            title: "Test".to_string(),
            mission_type: MissionType::Assassination,
            reward: 1000,
            stars: 3,
            rating_label: "Good".to_string(),
        };
        assert_eq!(record.type_label(), "暗殺");
    }
}
