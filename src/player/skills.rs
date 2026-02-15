//! 技能成長系統 (GTA 5 風格)
//!
//! 4 項技能透過實際使用自動升級：
//! - 射擊：射擊命中敵人時獲得 XP → 降低後座力、加快換彈速度
//! - 駕駛：駕車時累積里程 → 提高操控性、降低碰撞傷害
//! - 體力：衝刺/攀爬時累積時間 → 增加體力上限、加快恢復
//! - 潛行：潛行狀態移動時累積 → 降低偵測距離、靜默擊殺傷害加成

// 部分技能效果尚未完全整合，個別標記 #[allow(dead_code)]

use bevy::prelude::*;

// ============================================================================
// 技能定義
// ============================================================================

/// 技能類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SkillType {
    /// 射擊：降低後座力、加快換彈
    Shooting,
    /// 駕駛：提高操控、降低碰撞傷害
    Driving,
    /// 體力：增加體力上限、加快恢復
    Stamina,
    /// 潛行：降低偵測距離、靜默傷害加成
    Stealth,
}

impl SkillType {
    /// 所有技能類型
    #[allow(dead_code)]
    pub const ALL: [SkillType; 4] = [
        SkillType::Shooting,
        SkillType::Driving,
        SkillType::Stamina,
        SkillType::Stealth,
    ];

    /// 技能中文名稱
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            SkillType::Shooting => "射擊",
            SkillType::Driving => "駕駛",
            SkillType::Stamina => "體力",
            SkillType::Stealth => "潛行",
        }
    }
}

// ============================================================================
// 單一技能資料
// ============================================================================

/// 單一技能的等級和經驗值
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    /// 當前等級 (0-100)
    pub level: u32,
    /// 當前經驗值
    pub xp: f32,
    /// 此技能類型
    pub skill_type: SkillType,
}

/// 最大等級
pub const MAX_SKILL_LEVEL: u32 = 100;

/// 每級所需 XP（等級越高越慢）
/// level N → level N+1 需要 base * (1 + N * 0.15) XP
const XP_BASE: f32 = 100.0;
const XP_SCALE: f32 = 0.15;

impl Skill {
    pub fn new(skill_type: SkillType) -> Self {
        Self {
            level: 0,
            xp: 0.0,
            skill_type,
        }
    }

    /// 升到下一級所需的 XP
    pub fn xp_to_next_level(&self) -> f32 {
        if self.level >= MAX_SKILL_LEVEL {
            return f32::MAX;
        }
        XP_BASE * (1.0 + self.level as f32 * XP_SCALE)
    }

    /// 當前等級進度百分比 (0.0-1.0)
    #[allow(dead_code)]
    pub fn progress(&self) -> f32 {
        if self.level >= MAX_SKILL_LEVEL {
            return 1.0;
        }
        (self.xp / self.xp_to_next_level()).clamp(0.0, 1.0)
    }

    /// 增加 XP，自動升級
    /// 回傳是否升級了
    pub fn add_xp(&mut self, amount: f32) -> bool {
        if self.level >= MAX_SKILL_LEVEL {
            return false;
        }

        self.xp += amount;
        let mut leveled_up = false;

        while self.level < MAX_SKILL_LEVEL && self.xp >= self.xp_to_next_level() {
            self.xp -= self.xp_to_next_level();
            self.level += 1;
            leveled_up = true;
        }

        // 滿級時歸零殘餘 XP
        if self.level >= MAX_SKILL_LEVEL {
            self.xp = 0.0;
        }

        leveled_up
    }

    /// 等級效果倍率 (0.0 → 1.0 隨等級線性成長)
    #[allow(dead_code)]
    pub fn effect_ratio(&self) -> f32 {
        self.level as f32 / MAX_SKILL_LEVEL as f32
    }
}

// ============================================================================
// 玩家技能資源
// ============================================================================

/// 玩家技能資源（全域，因為只有一個玩家）
#[derive(Resource, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerSkills {
    pub shooting: Skill,
    pub driving: Skill,
    pub stamina: Skill,
    pub stealth: Skill,
}

impl Default for PlayerSkills {
    fn default() -> Self {
        Self {
            shooting: Skill::new(SkillType::Shooting),
            driving: Skill::new(SkillType::Driving),
            stamina: Skill::new(SkillType::Stamina),
            stealth: Skill::new(SkillType::Stealth),
        }
    }
}

impl PlayerSkills {
    /// 取得指定技能的引用
    #[allow(dead_code)]
    pub fn get(&self, skill_type: SkillType) -> &Skill {
        match skill_type {
            SkillType::Shooting => &self.shooting,
            SkillType::Driving => &self.driving,
            SkillType::Stamina => &self.stamina,
            SkillType::Stealth => &self.stealth,
        }
    }

    /// 取得指定技能的可變引用
    #[allow(dead_code)]
    pub fn get_mut(&mut self, skill_type: SkillType) -> &mut Skill {
        match skill_type {
            SkillType::Shooting => &mut self.shooting,
            SkillType::Driving => &mut self.driving,
            SkillType::Stamina => &mut self.stamina,
            SkillType::Stealth => &mut self.stealth,
        }
    }

    // ========================================================================
    // 技能效果查詢（各系統呼叫）
    // ========================================================================

    /// 射擊後座力倍率 (1.0 → 0.5 隨等級降低)
    #[allow(dead_code)]
    pub fn recoil_multiplier(&self) -> f32 {
        1.0 - self.shooting.effect_ratio() * 0.5
    }

    /// 換彈速度倍率 (1.0 → 1.5 隨等級加快)
    #[allow(dead_code)]
    pub fn reload_speed_multiplier(&self) -> f32 {
        1.0 + self.shooting.effect_ratio() * 0.5
    }

    /// 駕駛操控加成 (0.0 → 0.3 隨等級增加)
    #[allow(dead_code)]
    pub fn driving_handling_bonus(&self) -> f32 {
        self.driving.effect_ratio() * 0.3
    }

    /// 碰撞傷害減免 (0% → 30%)
    #[allow(dead_code)]
    pub fn collision_damage_reduction(&self) -> f32 {
        self.driving.effect_ratio() * 0.3
    }

    /// 體力上限加成 (100 → 150)
    #[allow(dead_code)]
    pub fn stamina_max_bonus(&self) -> f32 {
        self.stamina.effect_ratio() * 50.0
    }

    /// 體力恢復速度倍率 (1.0 → 1.8)
    #[allow(dead_code)]
    pub fn stamina_regen_multiplier(&self) -> f32 {
        1.0 + self.stamina.effect_ratio() * 0.8
    }

    /// 潛行偵測距離倍率 (1.0 → 0.5 隨等級降低)
    #[allow(dead_code)]
    pub fn stealth_detection_multiplier(&self) -> f32 {
        1.0 - self.stealth.effect_ratio() * 0.5
    }

    /// 靜默擊殺傷害倍率 (2.0 → 4.0)
    #[allow(dead_code)]
    pub fn stealth_kill_multiplier(&self) -> f32 {
        2.0 + self.stealth.effect_ratio() * 2.0
    }
}

// ============================================================================
// 技能成長系統
// ============================================================================

use crate::core::GameState;
use crate::vehicle::Vehicle;

/// 技能 XP 獲取常數
#[allow(dead_code)]
const SHOOTING_XP_PER_HIT: f32 = 5.0;
#[allow(dead_code)]
const SHOOTING_XP_HEADSHOT_BONUS: f32 = 15.0;
const DRIVING_XP_PER_SECOND: f32 = 1.0;
const STAMINA_XP_PER_SPRINT_SECOND: f32 = 2.0;
#[allow(dead_code)]
const STAMINA_XP_PER_CLIMB: f32 = 10.0;
const STEALTH_XP_PER_SECOND: f32 = 3.0;
#[allow(dead_code)]
const STEALTH_XP_PER_SILENT_KILL: f32 = 25.0;

/// 駕駛技能累積系統
/// 駕車期間每秒累積 XP
pub fn driving_skill_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    mut skills: ResMut<PlayerSkills>,
    vehicle_query: Query<&Vehicle>,
) {
    if !game_state.player_in_vehicle {
        return;
    }

    // 確認有車輛且在移動中
    let has_moving_vehicle = vehicle_query
        .iter()
        .any(|v| v.current_speed.abs() > 1.0);

    if !has_moving_vehicle {
        return;
    }

    let xp = DRIVING_XP_PER_SECOND * time.delta_secs();
    skills.driving.add_xp(xp);
}

/// 體力技能累積系統
/// 衝刺中每秒累積 XP
pub fn stamina_skill_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    mut skills: ResMut<PlayerSkills>,
    player_query: Query<&super::Player>,
) {
    if game_state.player_in_vehicle {
        return;
    }

    let Ok(player) = player_query.single() else {
        return;
    };

    if player.is_sprinting {
        let xp = STAMINA_XP_PER_SPRINT_SECOND * time.delta_secs();
        skills.stamina.add_xp(xp);
    }
}

/// 潛行技能累積系統
/// 蹲伏移動期間每秒累積 XP
pub fn stealth_skill_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    mut skills: ResMut<PlayerSkills>,
    player_query: Query<&super::Player>,
) {
    if game_state.player_in_vehicle {
        return;
    }

    let Ok(player) = player_query.single() else {
        return;
    };

    // 蹲伏且有移動才算潛行
    if player.is_crouching && player.current_speed > 0.5 {
        let xp = STEALTH_XP_PER_SECOND * time.delta_secs();
        skills.stealth.add_xp(xp);
    }
}

/// 射擊技能 XP 獎勵（由戰鬥系統呼叫）
#[allow(dead_code)]
pub fn award_shooting_xp(skills: &mut PlayerSkills, is_headshot: bool) {
    let xp = SHOOTING_XP_PER_HIT + if is_headshot { SHOOTING_XP_HEADSHOT_BONUS } else { 0.0 };
    skills.shooting.add_xp(xp);
}

/// 攀爬完成時的體力 XP 獎勵（由攀爬系統呼叫）
#[allow(dead_code)]
pub fn award_climb_xp(skills: &mut PlayerSkills) {
    skills.stamina.add_xp(STAMINA_XP_PER_CLIMB);
}

/// 靜默擊殺的潛行 XP 獎勵（由戰鬥系統呼叫）
#[allow(dead_code)]
pub fn award_stealth_kill_xp(skills: &mut PlayerSkills) {
    skills.stealth.add_xp(STEALTH_XP_PER_SILENT_KILL);
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_starts_at_level_zero() {
        let skill = Skill::new(SkillType::Shooting);
        assert_eq!(skill.level, 0);
        assert_eq!(skill.xp, 0.0);
    }

    #[test]
    fn skill_xp_to_next_level_scales() {
        let skill = Skill::new(SkillType::Driving);
        let xp_0 = skill.xp_to_next_level(); // base * (1 + 0) = 100

        let mut skill_10 = Skill::new(SkillType::Driving);
        skill_10.level = 10;
        let xp_10 = skill_10.xp_to_next_level(); // base * (1 + 10 * 0.15) = 250

        assert!((xp_0 - 100.0).abs() < 0.01);
        assert!((xp_10 - 250.0).abs() < 0.01);
        assert!(xp_10 > xp_0, "高等級應需要更多 XP");
    }

    #[test]
    fn skill_add_xp_levels_up() {
        let mut skill = Skill::new(SkillType::Stamina);
        // Level 0 → 1 需要 100 XP
        let leveled = skill.add_xp(100.0);
        assert!(leveled);
        assert_eq!(skill.level, 1);
        assert!(skill.xp < 1.0, "應無殘餘 XP");
    }

    #[test]
    fn skill_add_xp_multiple_levels() {
        let mut skill = Skill::new(SkillType::Stealth);
        // 一次給大量 XP，應跨多級
        skill.add_xp(500.0);
        assert!(skill.level > 1, "500 XP 應至少升到 Lv.2+");
    }

    #[test]
    fn skill_max_level_caps() {
        let mut skill = Skill::new(SkillType::Shooting);
        skill.level = MAX_SKILL_LEVEL;
        let leveled = skill.add_xp(9999.0);
        assert!(!leveled, "滿級不應再升級");
        assert_eq!(skill.level, MAX_SKILL_LEVEL);
    }

    #[test]
    fn skill_progress_0_to_1() {
        let mut skill = Skill::new(SkillType::Driving);
        assert!((skill.progress() - 0.0).abs() < 0.01);

        skill.xp = 50.0; // 50/100 = 0.5
        assert!((skill.progress() - 0.5).abs() < 0.01);

        skill.level = MAX_SKILL_LEVEL;
        assert!((skill.progress() - 1.0).abs() < 0.01);
    }

    #[test]
    fn player_skills_default() {
        let skills = PlayerSkills::default();
        assert_eq!(skills.shooting.level, 0);
        assert_eq!(skills.driving.level, 0);
        assert_eq!(skills.stamina.level, 0);
        assert_eq!(skills.stealth.level, 0);
    }

    #[test]
    fn skill_effects_scale_with_level() {
        let mut skills = PlayerSkills::default();

        // 等級 0：無加成
        assert!((skills.recoil_multiplier() - 1.0).abs() < 0.01);
        assert!((skills.stamina_max_bonus() - 0.0).abs() < 0.01);

        // 滿級射擊
        skills.shooting.level = MAX_SKILL_LEVEL;
        assert!((skills.recoil_multiplier() - 0.5).abs() < 0.01);
        assert!((skills.reload_speed_multiplier() - 1.5).abs() < 0.01);

        // 滿級駕駛
        skills.driving.level = MAX_SKILL_LEVEL;
        assert!((skills.driving_handling_bonus() - 0.3).abs() < 0.01);
        assert!((skills.collision_damage_reduction() - 0.3).abs() < 0.01);

        // 滿級體力
        skills.stamina.level = MAX_SKILL_LEVEL;
        assert!((skills.stamina_max_bonus() - 50.0).abs() < 0.01);
        assert!((skills.stamina_regen_multiplier() - 1.8).abs() < 0.01);

        // 滿級潛行
        skills.stealth.level = MAX_SKILL_LEVEL;
        assert!((skills.stealth_detection_multiplier() - 0.5).abs() < 0.01);
        assert!((skills.stealth_kill_multiplier() - 4.0).abs() < 0.01);
    }

    #[test]
    fn skill_get_and_get_mut() {
        let mut skills = PlayerSkills::default();
        assert_eq!(skills.get(SkillType::Shooting).level, 0);

        skills.get_mut(SkillType::Shooting).level = 50;
        assert_eq!(skills.get(SkillType::Shooting).level, 50);
    }

    #[test]
    fn skill_type_all_has_four() {
        assert_eq!(SkillType::ALL.len(), 4);
    }

    #[test]
    fn award_shooting_xp_normal_and_headshot() {
        let mut skills = PlayerSkills::default();
        award_shooting_xp(&mut skills, false);
        assert!(skills.shooting.xp > 0.0);

        let xp_after_normal = skills.shooting.xp;
        award_shooting_xp(&mut skills, true);
        let xp_after_headshot = skills.shooting.xp;
        assert!(
            xp_after_headshot - xp_after_normal > SHOOTING_XP_PER_HIT,
            "爆頭應獎勵更多 XP"
        );
    }
}
