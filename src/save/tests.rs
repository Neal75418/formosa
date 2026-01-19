//! 存檔系統單元測試

use super::components::*;
use std::path::PathBuf;

// ============================================================================
// SaveManager 測試
// ============================================================================

#[test]
fn test_save_manager_default() {
    let manager = SaveManager::default();

    assert_eq!(manager.max_slots, 10);
    assert_eq!(manager.current_slot, 0);
    assert!(manager.auto_save_enabled);
    assert_eq!(manager.auto_save_interval, 300.0);
    assert!(!manager.is_busy);
    assert_eq!(manager.slot_info.len(), 10);
}

#[test]
fn test_save_manager_get_save_path() {
    let mut manager = SaveManager::default();
    manager.save_directory = PathBuf::from("/test/saves");

    let path = manager.get_save_path(0);
    assert_eq!(path, PathBuf::from("/test/saves/save_00.json"));

    let path = manager.get_save_path(5);
    assert_eq!(path, PathBuf::from("/test/saves/save_05.json"));

    let path = manager.get_save_path(99);
    assert_eq!(path, PathBuf::from("/test/saves/save_99.json"));
}

#[test]
fn test_save_manager_get_auto_save_path() {
    let mut manager = SaveManager::default();
    manager.save_directory = PathBuf::from("/test/saves");

    let path = manager.get_auto_save_path();
    assert_eq!(path, PathBuf::from("/test/saves/autosave.json"));
}

#[test]
fn test_save_manager_get_quick_save_path() {
    let mut manager = SaveManager::default();
    manager.save_directory = PathBuf::from("/test/saves");

    let path = manager.get_quick_save_path();
    assert_eq!(path, PathBuf::from("/test/saves/quicksave.json"));
}

// ============================================================================
// SaveData 測試
// ============================================================================

#[test]
fn test_save_data_default() {
    let save_data = SaveData::default();

    assert_eq!(save_data.version, SAVE_VERSION);
    assert_eq!(save_data.timestamp, 0);
    assert_eq!(save_data.play_time_secs, 0.0);
}

#[test]
fn test_save_data_serialization() {
    let save_data = SaveData {
        version: SAVE_VERSION,
        timestamp: 1234567890,
        play_time_secs: 3600.0,
        player: PlayerSaveData {
            position: [100.0, 0.0, 200.0],
            rotation_y: 1.5,
            health: 75.0,
            max_health: 100.0,
            armor: 50.0,
            cash: 5000,
            bank: 10000,
            weapons: vec![],
            current_weapon_index: 0,
            in_vehicle: false,
            current_vehicle_id: None,
        },
        world: WorldSaveData::default(),
        missions: MissionSaveData::default(),
        stats: GameStatistics::default(),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&save_data).expect("Serialization failed");

    // Deserialize back
    let loaded: SaveData = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(loaded.version, save_data.version);
    assert_eq!(loaded.timestamp, save_data.timestamp);
    assert_eq!(loaded.play_time_secs, save_data.play_time_secs);
    assert_eq!(loaded.player.position, save_data.player.position);
    assert_eq!(loaded.player.cash, save_data.player.cash);
}

// ============================================================================
// PlayerSaveData 測試
// ============================================================================

#[test]
fn test_player_save_data_default() {
    let player_data = PlayerSaveData::default();

    assert_eq!(player_data.position, [0.0, 0.0, 0.0]);
    assert_eq!(player_data.rotation_y, 0.0);
    assert_eq!(player_data.health, 0.0);
    assert!(!player_data.in_vehicle);
    assert!(player_data.current_vehicle_id.is_none());
}

#[test]
fn test_player_save_data_with_weapons() {
    let player_data = PlayerSaveData {
        weapons: vec![
            WeaponSaveData {
                weapon_type: "pistol".to_string(),
                current_ammo: 12,
                reserve_ammo: 60,
            },
            WeaponSaveData {
                weapon_type: "smg".to_string(),
                current_ammo: 30,
                reserve_ammo: 120,
            },
        ],
        ..Default::default()
    };

    let json = serde_json::to_string(&player_data).expect("Serialization failed");
    let loaded: PlayerSaveData = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(loaded.weapons.len(), 2);
    assert_eq!(loaded.weapons[0].weapon_type, "pistol");
    assert_eq!(loaded.weapons[0].current_ammo, 12);
    assert_eq!(loaded.weapons[1].weapon_type, "smg");
}

// ============================================================================
// WorldSaveData 測試
// ============================================================================

#[test]
fn test_world_save_data_default() {
    let world_data = WorldSaveData::default();

    assert_eq!(world_data.world_hour, 8.0);
    assert_eq!(world_data.weather, "Clear");
    assert_eq!(world_data.weather_intensity, 1.0);
    assert!(!world_data.unlocked_safehouses.is_empty());
}

#[test]
fn test_world_save_data_with_vehicles() {
    let world_data = WorldSaveData {
        owned_vehicles: vec![1, 2, 3],
        vehicle_modifications: vec![
            VehicleModificationSaveData {
                vehicle_index: 1,
                engine_level: 2,
                transmission_level: 1,
                suspension_level: 0,
                brakes_level: 1,
                tires_level: 1,
                armor_level: 0,
                has_nitro: true,
                nitro_charge: 0.75,
            },
        ],
        ..Default::default()
    };

    let json = serde_json::to_string(&world_data).expect("Serialization failed");
    let loaded: WorldSaveData = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(loaded.owned_vehicles.len(), 3);
    assert_eq!(loaded.vehicle_modifications.len(), 1);
    assert_eq!(loaded.vehicle_modifications[0].engine_level, 2);
    assert!(loaded.vehicle_modifications[0].has_nitro);
}

// ============================================================================
// MissionSaveData 測試
// ============================================================================

#[test]
fn test_mission_save_data_default() {
    let mission_data = MissionSaveData::default();

    assert!(mission_data.completed_missions.is_empty());
    assert!(mission_data.active_mission.is_none());
    assert!(mission_data.unlocked_missions.is_empty());
}

#[test]
fn test_mission_save_data_with_progress() {
    let mission_data = MissionSaveData {
        completed_missions: vec!["mission_01".to_string(), "mission_02".to_string()],
        active_mission: Some("mission_03".to_string()),
        mission_progress: vec![("mission_03".to_string(), 2)],
        mission_ratings: vec![
            ("mission_01".to_string(), 3),
            ("mission_02".to_string(), 2),
        ],
        unlocked_missions: vec!["mission_03".to_string(), "mission_04".to_string()],
        npc_relationships: vec![("npc_01".to_string(), 50)],
    };

    let json = serde_json::to_string(&mission_data).expect("Serialization failed");
    let loaded: MissionSaveData = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(loaded.completed_missions.len(), 2);
    assert_eq!(loaded.active_mission, Some("mission_03".to_string()));
    assert_eq!(loaded.mission_ratings.len(), 2);
}

// ============================================================================
// GameStatistics 測試
// ============================================================================

#[test]
fn test_game_statistics_default() {
    let stats = GameStatistics::default();

    assert_eq!(stats.enemies_killed, 0);
    assert_eq!(stats.headshots, 0);
    assert_eq!(stats.distance_driven, 0.0);
    assert_eq!(stats.missions_completed, 0);
    assert_eq!(stats.max_wanted_level_reached, 0);
}

#[test]
fn test_game_statistics_serialization() {
    let stats = GameStatistics {
        enemies_killed: 100,
        headshots: 25,
        distance_driven: 50000.0,
        distance_walked: 10000.0,
        missions_completed: 10,
        missions_failed: 2,
        total_money_earned: 500000,
        total_money_spent: 300000,
        max_wanted_level_reached: 4,
        police_killed: 15,
        vehicles_destroyed: 8,
    };

    let json = serde_json::to_string(&stats).expect("Serialization failed");
    let loaded: GameStatistics = serde_json::from_str(&json).expect("Deserialization failed");

    assert_eq!(loaded.enemies_killed, 100);
    assert_eq!(loaded.headshots, 25);
    assert_eq!(loaded.distance_driven, 50000.0);
    assert_eq!(loaded.max_wanted_level_reached, 4);
}

// ============================================================================
// VehicleModificationSaveData 測試
// ============================================================================

#[test]
fn test_vehicle_modification_save_data_default() {
    let mod_data = VehicleModificationSaveData::default();

    assert_eq!(mod_data.vehicle_index, 0);
    assert_eq!(mod_data.engine_level, 0);
    assert!(!mod_data.has_nitro);
    assert_eq!(mod_data.nitro_charge, 0.0);
}

// ============================================================================
// SaveType / LoadType 測試
// ============================================================================

#[test]
fn test_save_type_equality() {
    assert_eq!(SaveType::Slot, SaveType::Slot);
    assert_eq!(SaveType::QuickSave, SaveType::QuickSave);
    assert_eq!(SaveType::AutoSave, SaveType::AutoSave);
    assert_ne!(SaveType::Slot, SaveType::QuickSave);
}

#[test]
fn test_load_type_equality() {
    assert_eq!(LoadType::Slot, LoadType::Slot);
    assert_eq!(LoadType::QuickLoad, LoadType::QuickLoad);
    assert_eq!(LoadType::AutoSave, LoadType::AutoSave);
    assert_ne!(LoadType::Slot, LoadType::QuickLoad);
}
