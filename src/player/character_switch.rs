//! 角色切換系統
//!
//! 2-3 位可操作主角，各自有獨立技能、金錢、位置。
//! 按 F5/F6/F7 切換角色。

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]
// Bevy 系統需要 Res<T> 按值傳遞
#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::skills::PlayerSkills;

// ============================================================================
// 角色定義
// ============================================================================

/// 可操作角色 ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CharacterId {
    /// 主角 A：阿龍（街頭混混出身，近戰強）
    ALong,
    /// 主角 B：小美（前警察，射擊/駕駛強）
    XiaoMei,
    /// 主角 C：阿財（商人，初始資金高）
    ACai,
}

impl CharacterId {
    pub const ALL: [CharacterId; 3] = [CharacterId::ALong, CharacterId::XiaoMei, CharacterId::ACai];

    /// 角色中文名
    pub fn name(&self) -> &'static str {
        match self {
            CharacterId::ALong => "阿龍",
            CharacterId::XiaoMei => "小美",
            CharacterId::ACai => "阿財",
        }
    }

    /// 角色背景描述
    pub fn description(&self) -> &'static str {
        match self {
            CharacterId::ALong => "街頭混混出身，拳頭就是道理。近戰傷害 +30%",
            CharacterId::XiaoMei => "前警察，精準射擊和高速駕駛。射擊/駕駛經驗 +20%",
            CharacterId::ACai => "夜市老闆之子，人脈廣。初始資金 $50,000",
        }
    }

    /// 初始技能加成
    pub fn initial_skills(&self) -> PlayerSkills {
        let mut skills = PlayerSkills::default();
        match self {
            CharacterId::ALong => {
                // 近戰特長：體力技能初始較高
                skills.stamina.add_xp(500.0);
            }
            CharacterId::XiaoMei => {
                // 射擊和駕駛特長
                skills.shooting.add_xp(400.0);
                skills.driving.add_xp(400.0);
            }
            CharacterId::ACai => {
                // 潛行特長（商場手腕）
                skills.stealth.add_xp(300.0);
            }
        }
        skills
    }

    /// 初始現金
    pub fn initial_cash(&self) -> i32 {
        match self {
            CharacterId::ALong => 5_000,
            CharacterId::XiaoMei => 10_000,
            CharacterId::ACai => 50_000,
        }
    }

    /// 特殊能力倍率
    pub fn combat_multiplier(&self) -> f32 {
        match self {
            CharacterId::ALong => 1.3,   // 近戰 +30%
            CharacterId::XiaoMei => 1.0,
            CharacterId::ACai => 1.0,
        }
    }

    /// 射擊經驗加成
    pub fn shooting_xp_multiplier(&self) -> f32 {
        match self {
            CharacterId::ALong => 1.0,
            CharacterId::XiaoMei => 1.2, // +20%
            CharacterId::ACai => 1.0,
        }
    }

    /// 駕駛經驗加成
    pub fn driving_xp_multiplier(&self) -> f32 {
        match self {
            CharacterId::ALong => 1.0,
            CharacterId::XiaoMei => 1.2, // +20%
            CharacterId::ACai => 1.0,
        }
    }
}

// ============================================================================
// 角色狀態快照
// ============================================================================

/// 角色狀態快照（切換時保存/恢復）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterSnapshot {
    /// 角色 ID
    pub id: CharacterId,
    /// 位置
    pub position: Vec3,
    /// 面朝方向（Y 軸旋轉）
    pub rotation_y: f32,
    /// 技能
    pub skills: PlayerSkills,
    /// 現金
    pub cash: i32,
    /// 銀行存款
    pub bank: i32,
    /// HP
    pub hp: f32,
    /// 是否已解鎖
    pub unlocked: bool,
}

impl CharacterSnapshot {
    /// 建立新角色快照
    pub fn new(id: CharacterId) -> Self {
        Self {
            id,
            position: id.default_position(),
            rotation_y: 0.0,
            skills: id.initial_skills(),
            cash: id.initial_cash(),
            bank: 0,
            hp: 100.0,
            unlocked: id == CharacterId::ALong, // 只有阿龍初始解鎖
        }
    }
}

impl CharacterId {
    /// 角色預設位置（各自的「家」）
    pub fn default_position(&self) -> Vec3 {
        match self {
            CharacterId::ALong => Vec3::new(-20.0, 1.0, -30.0),  // 西門町巷弄
            CharacterId::XiaoMei => Vec3::new(40.0, 1.0, 20.0),   // 警察局附近
            CharacterId::ACai => Vec3::new(-50.0, 1.0, 10.0),     // 夜市入口
        }
    }
}

// ============================================================================
// 角色管理資源
// ============================================================================

/// 角色切換管理器
#[derive(Resource)]
pub struct CharacterManager {
    /// 當前操作的角色
    pub active: CharacterId,
    /// 所有角色的快照
    pub snapshots: Vec<CharacterSnapshot>,
    /// 切換冷卻（防止快速連按）
    pub switch_cooldown: f32,
}

impl Default for CharacterManager {
    fn default() -> Self {
        Self {
            active: CharacterId::ALong,
            snapshots: CharacterId::ALL
                .iter()
                .map(|&id| CharacterSnapshot::new(id))
                .collect(),
            switch_cooldown: 0.0,
        }
    }
}

impl CharacterManager {
    /// 取得角色快照
    pub fn get(&self, id: CharacterId) -> Option<&CharacterSnapshot> {
        self.snapshots.iter().find(|s| s.id == id)
    }

    /// 取得角色可變快照
    pub fn get_mut(&mut self, id: CharacterId) -> Option<&mut CharacterSnapshot> {
        self.snapshots.iter_mut().find(|s| s.id == id)
    }

    /// 取得當前角色快照
    pub fn active_snapshot(&self) -> Option<&CharacterSnapshot> {
        self.get(self.active)
    }

    /// 是否可以切換（冷卻完畢且目標已解鎖）
    pub fn can_switch_to(&self, id: CharacterId) -> bool {
        if self.switch_cooldown > 0.0 {
            return false;
        }
        if id == self.active {
            return false;
        }
        self.get(id).is_some_and(|s| s.unlocked)
    }

    /// 解鎖角色
    pub fn unlock(&mut self, id: CharacterId) {
        if let Some(snapshot) = self.get_mut(id) {
            snapshot.unlocked = true;
        }
    }

    /// 執行切換：保存當前角色狀態、切換到目標角色
    pub fn switch_to(
        &mut self,
        target: CharacterId,
        current_pos: Vec3,
        current_rotation_y: f32,
        current_hp: f32,
        current_cash: i32,
        current_bank: i32,
        current_skills: &PlayerSkills,
    ) -> Option<&CharacterSnapshot> {
        if !self.can_switch_to(target) {
            return None;
        }

        // 保存當前角色狀態
        if let Some(current) = self.get_mut(self.active) {
            current.position = current_pos;
            current.rotation_y = current_rotation_y;
            current.hp = current_hp;
            current.cash = current_cash;
            current.bank = current_bank;
            current.skills = current_skills.clone();
        }

        // 切換
        self.active = target;
        self.switch_cooldown = 1.0; // 1 秒冷卻

        self.get(target)
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 角色切換冷卻計時系統
pub fn character_switch_cooldown_system(
    time: Res<Time>,
    mut manager: ResMut<CharacterManager>,
) {
    if manager.switch_cooldown > 0.0 {
        manager.switch_cooldown = (manager.switch_cooldown - time.delta_secs()).max(0.0);
    }
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_ids() {
        assert_eq!(CharacterId::ALL.len(), 3);
        assert_eq!(CharacterId::ALong.name(), "阿龍");
        assert_eq!(CharacterId::XiaoMei.name(), "小美");
        assert_eq!(CharacterId::ACai.name(), "阿財");
    }

    #[test]
    fn character_initial_cash() {
        assert_eq!(CharacterId::ALong.initial_cash(), 5_000);
        assert_eq!(CharacterId::ACai.initial_cash(), 50_000);
    }

    #[test]
    fn character_combat_multiplier() {
        assert!((CharacterId::ALong.combat_multiplier() - 1.3).abs() < f32::EPSILON);
        assert!((CharacterId::XiaoMei.combat_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn snapshot_default_only_along_unlocked() {
        let manager = CharacterManager::default();
        assert!(manager.get(CharacterId::ALong).unwrap().unlocked);
        assert!(!manager.get(CharacterId::XiaoMei).unwrap().unlocked);
        assert!(!manager.get(CharacterId::ACai).unwrap().unlocked);
    }

    #[test]
    fn manager_can_switch() {
        let mut manager = CharacterManager::default();

        // 不能切換到未解鎖的角色
        assert!(!manager.can_switch_to(CharacterId::XiaoMei));

        // 解鎖後可以切換
        manager.unlock(CharacterId::XiaoMei);
        assert!(manager.can_switch_to(CharacterId::XiaoMei));

        // 不能切換到自己
        assert!(!manager.can_switch_to(CharacterId::ALong));
    }

    #[test]
    fn manager_switch_saves_state() {
        let mut manager = CharacterManager::default();
        manager.unlock(CharacterId::XiaoMei);

        let skills = PlayerSkills::default();
        let result = manager.switch_to(
            CharacterId::XiaoMei,
            Vec3::new(10.0, 1.0, 20.0),
            45.0,
            80.0,
            3000,
            5000,
            &skills,
        );

        assert!(result.is_some());
        assert_eq!(manager.active, CharacterId::XiaoMei);

        // 確認阿龍的狀態已保存
        let along = manager.get(CharacterId::ALong).unwrap();
        assert!((along.position.x - 10.0).abs() < f32::EPSILON);
        assert!((along.hp - 80.0).abs() < f32::EPSILON);
        assert_eq!(along.cash, 3000);
    }

    #[test]
    fn manager_switch_cooldown() {
        let mut manager = CharacterManager::default();
        manager.unlock(CharacterId::XiaoMei);
        manager.unlock(CharacterId::ACai);

        let skills = PlayerSkills::default();
        manager.switch_to(
            CharacterId::XiaoMei,
            Vec3::ZERO,
            0.0,
            100.0,
            0,
            0,
            &skills,
        );

        // 冷卻中不能再切換
        assert!(!manager.can_switch_to(CharacterId::ACai));
        assert!(manager.switch_cooldown > 0.0);

        // 冷卻結束後可以
        manager.switch_cooldown = 0.0;
        assert!(manager.can_switch_to(CharacterId::ACai));
    }

    #[test]
    fn character_default_positions() {
        for &id in &CharacterId::ALL {
            let pos = id.default_position();
            assert!(pos.y > 0.0, "Position Y should be above ground");
        }
    }

    #[test]
    fn character_initial_skills_differ() {
        let along_skills = CharacterId::ALong.initial_skills();
        let mei_skills = CharacterId::XiaoMei.initial_skills();

        // 阿龍體力技能初始較高
        assert!(along_skills.stamina.xp > 0.0);
        // 小美射擊和駕駛較高
        assert!(mei_skills.shooting.xp > 0.0);
        assert!(mei_skills.driving.xp > 0.0);
    }

    #[test]
    fn unlock_character() {
        let mut manager = CharacterManager::default();
        assert!(!manager.get(CharacterId::ACai).unwrap().unlocked);

        manager.unlock(CharacterId::ACai);
        assert!(manager.get(CharacterId::ACai).unwrap().unlocked);
    }
}
