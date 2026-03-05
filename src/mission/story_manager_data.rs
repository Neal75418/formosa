//! 範例任務資料
//!
//! 包含劇情任務的範例資料建構和單元測試

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

use super::story_data::{
    Difficulty, FailCondition, MissionObjective, MissionPhase, MissionRewards, ObjectiveType,
    StoryMission, StoryMissionType,
};
use super::story_manager::StoryMissionDatabase;
use crate::combat::WeaponType;

// ============================================================================
// 範例任務建構
// ============================================================================

/// 創建範例任務（用於測試）
#[allow(clippy::too_many_lines)]
pub fn create_sample_missions(database: &mut StoryMissionDatabase) {
    // 第一章第一個任務：對話任務
    let mission1 = StoryMission::new(1, "初來乍到", "在酒吧與神秘人交談，了解這座島嶼的情況")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(50.0, 0.0, 50.0))
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Dialogue, "找到神秘人")
                .with_objective(MissionObjective::new(
                    1,
                    ObjectiveType::ReachLocation(Vec3::new(55.0, 0.0, 55.0), 3.0),
                    "前往酒吧",
                ))
                .with_start_dialogue(1),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Dialogue, "與老王交談").with_objective(
                MissionObjective::new(
                    2,
                    ObjectiveType::TalkToNpc("mysterious_man".to_string()),
                    "與神秘人交談",
                ),
            ),
        )
        .with_rewards(
            MissionRewards::money(100)
                .with_respect(10)
                .unlock_mission(2),
        );

    // 第一章第二個任務：戰鬥任務
    let mission2 = StoryMission::new(2, "收債", "幫老王去向一個欠錢的人討債")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(100.0, 0.0, 100.0))
        .requires_mission(1) // 需要先完成任務 1
        .difficulty(Difficulty::Normal)
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Dialogue, "前往目標地點")
                .with_objective(MissionObjective::new(
                    1,
                    ObjectiveType::ReachLocation(Vec3::new(150.0, 0.0, 120.0), 5.0),
                    "前往工業區倉庫",
                ))
                .with_start_dialogue(2),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Elimination, "消滅守衛")
                .with_objective(
                    MissionObjective::new(2, ObjectiveType::KillCount(3), "消滅守衛").with_count(3),
                )
                .with_time_limit(180.0),
        )
        .with_phase(
            MissionPhase::new(3, StoryMissionType::Dialogue, "找到目標").with_objective(
                MissionObjective::new(
                    3,
                    ObjectiveType::TalkToNpc("debtor".to_string()),
                    "找到欠債人",
                ),
            ),
        )
        .with_rewards(
            MissionRewards::money(500)
                .with_respect(25)
                .unlock_mission(3),
        );

    // 第一章第三個任務：追車任務
    let mission3 = StoryMission::new(3, "追蹤線索", "追蹤一輛可疑車輛，找出幕後老闆")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(80.0, 0.0, -50.0))
        .requires_mission(2) // 需要先完成任務 2
        .difficulty(Difficulty::Normal)
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Chase, "等待目標出現")
                .with_objective(MissionObjective::new(
                    1,
                    ObjectiveType::ReachLocation(Vec3::new(100.0, 0.0, -80.0), 5.0),
                    "前往監視點",
                ))
                .with_start_dialogue(3),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Chase, "追蹤可疑車輛")
                .with_objective(MissionObjective::new(
                    2,
                    ObjectiveType::FollowTarget("suspect_vehicle".to_string(), 50.0),
                    "追蹤車輛",
                ))
                .with_time_limit(120.0)
                .with_fail_condition(FailCondition::TargetEscaped),
        )
        .with_phase(
            MissionPhase::new(3, StoryMissionType::Dialogue, "記下地點").with_objective(
                MissionObjective::new(
                    3,
                    ObjectiveType::ReachLocation(Vec3::new(200.0, 0.0, -150.0), 5.0),
                    "到達目的地",
                ),
            ),
        )
        .with_rewards(
            MissionRewards::money(300)
                .with_respect(20)
                .unlock_mission(4)
                .set_flag("found_hideout".to_string()),
        );

    // 第一章第四個任務：潛入任務
    let mission4 = StoryMission::new(4, "夜間行動", "潛入老闆的秘密據點，取得證據")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(200.0, 0.0, -150.0))
        .requires_mission(3)
        .requires_flag("found_hideout")
        .difficulty(Difficulty::Hard)
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Stealth, "潛入大樓")
                .with_objective(MissionObjective::new(
                    1,
                    ObjectiveType::ReachLocation(Vec3::new(210.0, 0.0, -160.0), 3.0),
                    "找到側門入口",
                ))
                .with_start_dialogue(4)
                .with_fail_condition(FailCondition::Detected),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Retrieve, "取得證據")
                .with_objective(MissionObjective::new(
                    2,
                    ObjectiveType::CollectItem("evidence_files".to_string()),
                    "找到機密文件",
                ))
                .with_objective(MissionObjective::new(
                    3,
                    ObjectiveType::CollectItem("financial_records".to_string()),
                    "找到財務記錄",
                )),
        )
        .with_phase(
            MissionPhase::new(3, StoryMissionType::Stealth, "離開建築")
                .with_objective(MissionObjective::new(
                    4,
                    ObjectiveType::ReachLocation(Vec3::new(180.0, 0.0, -140.0), 5.0),
                    "安全撤離",
                ))
                .with_fail_condition(FailCondition::Detected),
        )
        .with_rewards(
            MissionRewards::money(800)
                .with_respect(40)
                .unlock_mission(5)
                .set_flag("has_evidence".to_string()),
        );

    // 第一章最終任務：刺殺老闆
    let mission5 = StoryMission::new(5, "清算日", "帶著證據找老闆算帳，結束這一切")
        .chapter(1)
        .with_quest_giver(100)
        .at_location(Vec3::new(0.0, 0.0, 200.0))
        .requires_mission(4)
        .requires_flag("has_evidence")
        .difficulty(Difficulty::Hard)
        .with_phase(
            MissionPhase::new(1, StoryMissionType::Elimination, "殺進去")
                .with_objective(
                    MissionObjective::new(1, ObjectiveType::KillCount(5), "消滅門衛").with_count(5),
                )
                .with_start_dialogue(5),
        )
        .with_phase(
            MissionPhase::new(2, StoryMissionType::Assassination, "找到老闆")
                .with_objective(MissionObjective::new(
                    2,
                    ObjectiveType::KillTarget("boss".to_string()),
                    "消滅老闆",
                ))
                .with_time_limit(300.0),
        )
        .with_phase(
            MissionPhase::new(3, StoryMissionType::Dialogue, "任務完成")
                .with_objective(MissionObjective::new(
                    3,
                    ObjectiveType::ReachLocation(Vec3::new(50.0, 0.0, 50.0), 5.0),
                    "回去向老王回報",
                ))
                .with_end_dialogue(6),
        )
        .with_rewards(
            MissionRewards::money(2000)
                .with_respect(100)
                .unlock_weapon(WeaponType::Rifle)
                .set_flag("chapter1_complete".to_string()),
        );

    database.register(mission1);
    database.register(mission2);
    database.register(mission3);
    database.register(mission4);
    database.register(mission5);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::WorldTime;
    use crate::economy::PlayerWallet;
    use crate::mission::economy::RespectManager;
    use crate::mission::story_manager::{CheckpointError, StoryMissionManager};
    use crate::mission::unlocks::UnlockManager;

    fn setup_test_env() -> (StoryMissionManager, StoryMissionDatabase) {
        let mut database = StoryMissionDatabase::default();
        create_sample_missions(&mut database);
        let mut manager = StoryMissionManager::default();
        manager.unlock_mission(1);
        (manager, database)
    }

    #[test]
    fn test_create_checkpoint() {
        let (mut manager, database) = setup_test_env();
        let mission = database.get(1).unwrap();
        let wallet = PlayerWallet::default();
        let respect = RespectManager::default();
        let unlocks = UnlockManager::default();

        manager
            .start_mission(mission, &wallet, &respect, &unlocks, &WorldTime::default())
            .unwrap();
        manager.create_checkpoint(Vec3::new(10.0, 0.0, 20.0), 1);

        let checkpoint = manager.load_checkpoint().expect("應有檢查點");
        assert_eq!(checkpoint.mission_id, 1);
        assert_eq!(checkpoint.phase, 1);
        assert_eq!(checkpoint.player_position, Vec3::new(10.0, 0.0, 20.0));
    }

    #[test]
    fn test_clear_checkpoint() {
        let (mut manager, database) = setup_test_env();
        let mission = database.get(1).unwrap();
        let wallet = PlayerWallet::default();
        let respect = RespectManager::default();
        let unlocks = UnlockManager::default();

        manager
            .start_mission(mission, &wallet, &respect, &unlocks, &WorldTime::default())
            .unwrap();
        manager.create_checkpoint(Vec3::ZERO, 0);
        assert!(manager.load_checkpoint().is_some());

        manager.clear_checkpoint();
        assert!(manager.load_checkpoint().is_none());
    }

    #[test]
    fn test_retry_from_checkpoint() {
        let (mut manager, database) = setup_test_env();
        let mission = database.get(1).unwrap();
        let wallet = PlayerWallet::default();
        let respect = RespectManager::default();
        let unlocks = UnlockManager::default();

        manager
            .start_mission(mission, &wallet, &respect, &unlocks, &WorldTime::default())
            .unwrap();
        manager.create_checkpoint(Vec3::new(50.0, 0.0, 50.0), 1);

        // 模擬失敗
        manager.current_mission = None;

        let position = manager.retry_from_checkpoint(&database).unwrap();
        assert_eq!(position, Vec3::new(50.0, 0.0, 50.0));
        assert!(manager.current_mission.is_some());
    }

    #[test]
    fn test_retry_without_checkpoint_fails() {
        let (mut manager, database) = setup_test_env();
        assert!(manager.retry_from_checkpoint(&database).is_err());
    }

    #[test]
    fn test_validate_checkpoint() {
        let (mut manager, database) = setup_test_env();
        let mission = database.get(1).unwrap();
        let wallet = PlayerWallet::default();
        let respect = RespectManager::default();
        let unlocks = UnlockManager::default();

        manager
            .start_mission(mission, &wallet, &respect, &unlocks, &WorldTime::default())
            .unwrap();
        manager.create_checkpoint(Vec3::ZERO, 0);

        let result = manager.validate_and_load_checkpoint(&database);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_checkpoint_no_checkpoint() {
        let (manager, database) = setup_test_env();
        let result = manager.validate_and_load_checkpoint(&database);
        assert!(matches!(result, Err(CheckpointError::NoCheckpoint)));
    }
}
