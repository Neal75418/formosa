//! 實體命名系統（開發工具）
//!
//! 為場景中的重要實體加上有意義的名字，方便在 Inspector 中識別

use super::components::Building;
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::vehicle::{NpcVehicle, Vehicle, VehicleType};
use crate::wanted::PoliceOfficer;
use bevy::prelude::*;

/// 更新實體命名計時器（每秒執行一次命名檢查即可）
pub fn update_entity_naming_timer(time: Res<Time>, mut timer: ResMut<super::EntityNamingTimer>) {
    timer.timer.tick(time.delta());
}

/// 為未命名的玩家實體加上名字
pub fn name_player_entities(
    mut commands: Commands,
    query: Query<Entity, (With<Player>, Without<Name>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(Name::new("Player"));
    }
}

/// 為未命名的車輛實體加上名字
pub fn name_vehicle_entities(
    mut commands: Commands,
    query: Query<(Entity, &Vehicle, Option<&NpcVehicle>), Without<Name>>,
) {
    for (entity, vehicle, npc) in &query {
        let name = if npc.is_some() {
            match vehicle.vehicle_type {
                VehicleType::Scooter => "NPC Scooter",
                VehicleType::Car => "NPC Car",
                VehicleType::Taxi => "NPC Taxi",
                VehicleType::Bus => "NPC Bus",
            }
        } else {
            match vehicle.vehicle_type {
                VehicleType::Scooter => "Player Scooter",
                VehicleType::Car => "Player Car",
                VehicleType::Taxi => "Player Taxi",
                VehicleType::Bus => "Player Bus",
            }
        };
        commands.entity(entity).insert(Name::new(name));
    }
}

/// 為未命名的警察實體加上名字
pub fn name_police_entities(
    mut commands: Commands,
    query: Query<Entity, (With<PoliceOfficer>, Without<Name>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(Name::new("Police"));
    }
}

/// 為未命名的行人實體加上名字
pub fn name_pedestrian_entities(
    mut commands: Commands,
    query: Query<Entity, (With<Pedestrian>, Without<Name>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(Name::new("Pedestrian"));
    }
}

/// 為警車加上名字（根據 VehicleId）
pub fn name_police_car_entities(
    mut commands: Commands,
    query: Query<(Entity, &crate::wanted::PoliceCar), Without<Name>>,
) {
    for (entity, _) in &query {
        commands.entity(entity).insert(Name::new("Police Car"));
    }
}

/// 為建築物加上名字（Building + 類型，避免中文亂碼）
pub fn name_building_entities(
    mut commands: Commands,
    query: Query<(Entity, &Building), Without<Name>>,
) {
    for (entity, building) in &query {
        // 簡化名稱：Building + 類型
        let display_name = match building.building_type {
            super::components::BuildingType::Shop => "Shop",
            super::components::BuildingType::ConvenienceStore => "Convenience Store",
            super::components::BuildingType::Restaurant => "Restaurant",
            super::components::BuildingType::Cinema => "Cinema",
            super::components::BuildingType::Office => "Office Building",
            super::components::BuildingType::Residential => "Residential",
            super::components::BuildingType::Temple => "Temple",
            super::components::BuildingType::Other => "Building",
        };
        commands.entity(entity).insert(Name::new(display_name));
    }
}
