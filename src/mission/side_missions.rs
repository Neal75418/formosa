//! Strangers & Freaks 支線任務
//!
//! GTA5 風格的地圖上 ? 標記支線任務，獨立對話樹，
//! 以台灣在地特色為主題的 6 個支線任務。

use bevy::prelude::*;

use super::story_data::{
    Difficulty, MissionObjective, MissionPhase, MissionRewards, ObjectiveType, StoryMission,
    StoryMissionId, StoryMissionType,
};
use super::story_manager::StoryMissionDatabase;

/// 支線任務 ID 起始值（避免與主線 1-99 衝突）
pub const SIDE_MISSION_ID_START: StoryMissionId = 100;

/// 支線任務章節號（0 = 不屬於任何主線章節）
pub const SIDE_MISSION_CHAPTER: u32 = 0;

/// 註冊所有支線任務到任務資料庫
pub fn register_side_missions(database: &mut StoryMissionDatabase) {
    database.register(create_betel_nut_beauty());
    database.register(create_temple_fortune());
    database.register(create_night_market_chef());
    database.register(create_stray_dog_uncle());
    database.register(create_street_racer());
    database.register(create_conspiracy_blogger());

    info!(
        "支線任務系統初始化完成，共 {} 個 Strangers & Freaks 任務",
        6
    );
}

// ============================================================================
// 支線任務 #1：檳榔西施的煩惱
// ============================================================================

fn create_betel_nut_beauty() -> StoryMission {
    StoryMission::new(
        100,
        "檳榔西施的煩惱",
        "路邊檳榔攤的西施姐姐被流氓騷擾，幫她趕走那些人",
    )
    .chapter(SIDE_MISSION_CHAPTER)
    .with_quest_giver(200)
    .at_location(Vec3::new(-60.0, 0.0, 30.0))
    .difficulty(Difficulty::Easy)
    .with_phase(
        MissionPhase::new(1, StoryMissionType::Dialogue, "聽取請求")
            .with_objective(MissionObjective::new(
                1,
                ObjectiveType::TalkToNpc("betel_nut_beauty".to_string()),
                "與檳榔西施交談",
            ))
            .with_start_dialogue(200),
    )
    .with_phase(
        MissionPhase::new(2, StoryMissionType::Elimination, "趕走流氓")
            .with_objective(
                MissionObjective::new(2, ObjectiveType::KillCount(3), "教訓騷擾的流氓")
                    .with_count(3),
            )
            .with_time_limit(120.0),
    )
    .with_phase(
        MissionPhase::new(3, StoryMissionType::Dialogue, "回報")
            .with_objective(MissionObjective::new(
                3,
                ObjectiveType::TalkToNpc("betel_nut_beauty".to_string()),
                "回去找西施姐姐",
            ))
            .with_end_dialogue(201),
    )
    .with_rewards(
        MissionRewards::money(800)
            .with_respect(15)
            .set_flag("helped_betel_nut_beauty".to_string()),
    )
}

// ============================================================================
// 支線任務 #2：廟公的籤詩
// ============================================================================

fn create_temple_fortune() -> StoryMission {
    StoryMission::new(
        101,
        "廟公的籤詩",
        "土地公廟的廟公說你有劫數，要你去三個地方化解",
    )
    .chapter(SIDE_MISSION_CHAPTER)
    .with_quest_giver(201)
    .at_location(Vec3::new(120.0, 0.0, -30.0))
    .difficulty(Difficulty::Normal)
    .with_phase(
        MissionPhase::new(1, StoryMissionType::Dialogue, "求籤")
            .with_objective(MissionObjective::new(
                1,
                ObjectiveType::TalkToNpc("temple_keeper".to_string()),
                "向廟公求籤",
            ))
            .with_start_dialogue(202),
    )
    .with_phase(
        MissionPhase::new(2, StoryMissionType::Retrieve, "化解劫數")
            .with_objective(MissionObjective::new(
                2,
                ObjectiveType::ReachLocation(Vec3::new(80.0, 0.0, 60.0), 5.0),
                "前往東方水源地淨身",
            ))
            .with_objective(MissionObjective::new(
                3,
                ObjectiveType::ReachLocation(Vec3::new(-40.0, 0.0, -80.0), 5.0),
                "前往西方大榕樹祈福",
            ))
            .with_objective(MissionObjective::new(
                4,
                ObjectiveType::ReachLocation(Vec3::new(160.0, 0.0, 100.0), 5.0),
                "前往南方山頂拜拜",
            )),
    )
    .with_phase(
        MissionPhase::new(3, StoryMissionType::Dialogue, "回報廟公")
            .with_objective(MissionObjective::new(
                5,
                ObjectiveType::TalkToNpc("temple_keeper".to_string()),
                "回去找廟公",
            ))
            .with_end_dialogue(203),
    )
    .with_rewards(
        MissionRewards::money(500)
            .with_respect(20)
            .set_flag("temple_fortune_done".to_string()),
    )
}

// ============================================================================
// 支線任務 #3：夜市大廚的挑戰
// ============================================================================

fn create_night_market_chef() -> StoryMission {
    StoryMission::new(
        102,
        "夜市大廚的挑戰",
        "夜市蚵仔煎名店的老闆需要緊急送貨到三個地點",
    )
    .chapter(SIDE_MISSION_CHAPTER)
    .with_quest_giver(202)
    .at_location(Vec3::new(10.0, 0.0, -60.0))
    .difficulty(Difficulty::Normal)
    .with_phase(
        MissionPhase::new(1, StoryMissionType::Dialogue, "接受任務")
            .with_objective(MissionObjective::new(
                1,
                ObjectiveType::TalkToNpc("night_market_chef".to_string()),
                "與夜市老闆交談",
            ))
            .with_start_dialogue(204),
    )
    .with_phase(
        MissionPhase::new(2, StoryMissionType::Retrieve, "送貨")
            .with_objective(MissionObjective::new(
                2,
                ObjectiveType::ReachLocation(Vec3::new(70.0, 0.0, -40.0), 5.0),
                "送到第一個客人",
            ))
            .with_objective(MissionObjective::new(
                3,
                ObjectiveType::ReachLocation(Vec3::new(-30.0, 0.0, 50.0), 5.0),
                "送到第二個客人",
            ))
            .with_objective(MissionObjective::new(
                4,
                ObjectiveType::ReachLocation(Vec3::new(100.0, 0.0, 80.0), 5.0),
                "送到第三個客人",
            ))
            .with_time_limit(180.0),
    )
    .with_phase(
        MissionPhase::new(3, StoryMissionType::Dialogue, "回報")
            .with_objective(MissionObjective::new(
                5,
                ObjectiveType::TalkToNpc("night_market_chef".to_string()),
                "回去找夜市老闆",
            ))
            .with_end_dialogue(205),
    )
    .with_rewards(
        MissionRewards::money(600)
            .with_respect(15)
            .set_flag("night_market_chef_done".to_string()),
    )
}

// ============================================================================
// 支線任務 #4：流浪狗大叔
// ============================================================================

fn create_stray_dog_uncle() -> StoryMission {
    StoryMission::new(
        103,
        "流浪狗大叔",
        "公園裡餵流浪狗的大叔要你幫忙趕走虐待動物的混混",
    )
    .chapter(SIDE_MISSION_CHAPTER)
    .with_quest_giver(203)
    .at_location(Vec3::new(-100.0, 0.0, 70.0))
    .difficulty(Difficulty::Easy)
    .with_phase(
        MissionPhase::new(1, StoryMissionType::Dialogue, "聽取請求")
            .with_objective(MissionObjective::new(
                1,
                ObjectiveType::TalkToNpc("stray_dog_uncle".to_string()),
                "與大叔交談",
            ))
            .with_start_dialogue(206),
    )
    .with_phase(
        MissionPhase::new(2, StoryMissionType::Elimination, "趕走混混").with_objective(
            MissionObjective::new(2, ObjectiveType::KillCount(2), "教訓虐狗混混").with_count(2),
        ),
    )
    .with_phase(
        MissionPhase::new(3, StoryMissionType::Dialogue, "回報大叔")
            .with_objective(MissionObjective::new(
                3,
                ObjectiveType::TalkToNpc("stray_dog_uncle".to_string()),
                "回去找大叔",
            ))
            .with_end_dialogue(207),
    )
    .with_rewards(
        MissionRewards::money(300)
            .with_respect(25)
            .set_flag("helped_stray_dog_uncle".to_string()),
    )
}

// ============================================================================
// 支線任務 #5：飆車族
// ============================================================================

fn create_street_racer() -> StoryMission {
    StoryMission::new(
        104,
        "飆車族的賭注",
        "一群飆車族在海邊挑戰你賽車，贏了有獎金",
    )
    .chapter(SIDE_MISSION_CHAPTER)
    .with_quest_giver(204)
    .at_location(Vec3::new(-80.0, 0.0, -120.0))
    .difficulty(Difficulty::Hard)
    .with_phase(
        MissionPhase::new(1, StoryMissionType::Dialogue, "接受挑戰")
            .with_objective(MissionObjective::new(
                1,
                ObjectiveType::TalkToNpc("street_racer".to_string()),
                "與飆車族老大談話",
            ))
            .with_start_dialogue(208),
    )
    .with_phase(
        MissionPhase::new(2, StoryMissionType::Chase, "海岸線競速")
            .with_objective(MissionObjective::new(
                2,
                ObjectiveType::ReachLocation(Vec3::new(-100.0, 0.0, -150.0), 10.0),
                "通過第一個彎道",
            ))
            .with_objective(MissionObjective::new(
                3,
                ObjectiveType::ReachLocation(Vec3::new(-150.0, 0.0, -200.0), 10.0),
                "到達終點線",
            ))
            .with_time_limit(90.0),
    )
    .with_phase(
        MissionPhase::new(3, StoryMissionType::Dialogue, "領獎")
            .with_objective(MissionObjective::new(
                4,
                ObjectiveType::TalkToNpc("street_racer".to_string()),
                "回去找飆車族老大",
            ))
            .with_end_dialogue(209),
    )
    .with_rewards(
        MissionRewards::money(2000)
            .with_respect(30)
            .set_flag("street_racer_done".to_string()),
    )
}

// ============================================================================
// 支線任務 #6：陰謀論部落客
// ============================================================================

fn create_conspiracy_blogger() -> StoryMission {
    StoryMission::new(
        105,
        "陰謀論部落客",
        "一個戴著鋁箔帽的部落客說這座島有秘密，要你去三個地點拍照蒐證",
    )
    .chapter(SIDE_MISSION_CHAPTER)
    .with_quest_giver(205)
    .at_location(Vec3::new(40.0, 0.0, 140.0))
    .difficulty(Difficulty::Normal)
    .with_phase(
        MissionPhase::new(1, StoryMissionType::Dialogue, "聽取陰謀論")
            .with_objective(MissionObjective::new(
                1,
                ObjectiveType::TalkToNpc("conspiracy_blogger".to_string()),
                "聽部落客說明",
            ))
            .with_start_dialogue(210),
    )
    .with_phase(
        MissionPhase::new(2, StoryMissionType::Retrieve, "蒐集證據")
            .with_objective(MissionObjective::new(
                2,
                ObjectiveType::ReachLocation(Vec3::new(-50.0, 0.0, 180.0), 5.0),
                "調查廢棄工廠",
            ))
            .with_objective(MissionObjective::new(
                3,
                ObjectiveType::ReachLocation(Vec3::new(130.0, 0.0, 160.0), 5.0),
                "調查山上電塔",
            ))
            .with_objective(MissionObjective::new(
                4,
                ObjectiveType::ReachLocation(Vec3::new(200.0, 0.0, -50.0), 5.0),
                "調查港口貨櫃",
            )),
    )
    .with_phase(
        MissionPhase::new(3, StoryMissionType::Dialogue, "回報結果")
            .with_objective(MissionObjective::new(
                5,
                ObjectiveType::TalkToNpc("conspiracy_blogger".to_string()),
                "回去找部落客",
            ))
            .with_end_dialogue(211),
    )
    .with_rewards(
        MissionRewards::money(400)
            .with_respect(10)
            .set_flag("conspiracy_blogger_done".to_string()),
    )
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_mission_id_range() {
        assert_eq!(SIDE_MISSION_ID_START, 100);
    }

    #[test]
    fn test_all_side_missions_registered() {
        let mut database = StoryMissionDatabase::default();
        register_side_missions(&mut database);

        // 6 個支線任務
        for id in 100..=105 {
            assert!(database.get(id).is_some(), "支線任務 ID {id} 未註冊");
        }
    }

    #[test]
    fn test_side_missions_have_unique_locations() {
        let mut database = StoryMissionDatabase::default();
        register_side_missions(&mut database);

        let mut locations: Vec<Vec3> = Vec::new();
        for id in 100..=105 {
            let mission = database.get(id).unwrap();
            let loc = mission.trigger_location.expect("支線任務應有觸發位置");
            // 確保沒有重疊的觸發位置
            for existing in &locations {
                let dist = loc.distance(*existing);
                assert!(
                    dist > 10.0,
                    "任務 {id} 的位置與其他任務太近 (距離: {dist:.1})"
                );
            }
            locations.push(loc);
        }
    }

    #[test]
    fn test_side_missions_have_rewards() {
        let mut database = StoryMissionDatabase::default();
        register_side_missions(&mut database);

        for id in 100..=105 {
            let mission = database.get(id).unwrap();
            assert!(mission.rewards.money > 0, "任務 {id} 沒有金錢獎勵");
            assert!(mission.rewards.respect > 0, "任務 {id} 沒有聲望獎勵");
        }
    }

    #[test]
    fn test_side_missions_no_prerequisites() {
        let mut database = StoryMissionDatabase::default();
        register_side_missions(&mut database);

        // 支線任務不應該需要主線任務前置條件
        for id in 100..=105 {
            let mission = database.get(id).unwrap();
            assert!(
                mission.unlock_conditions.is_empty(),
                "支線任務 {id} 不應有前置條件"
            );
        }
    }

    #[test]
    fn test_side_missions_have_phases() {
        let mut database = StoryMissionDatabase::default();
        register_side_missions(&mut database);

        for id in 100..=105 {
            let mission = database.get(id).unwrap();
            assert!(
                mission.phases.len() >= 2,
                "任務 {} 至少需要 2 個階段，實際 {}",
                id,
                mission.phases.len()
            );
        }
    }
}
