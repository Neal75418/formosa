//! 車輛損壞系統（碰撞、火焰、傷害事件處理）

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use super::super::{Vehicle, VehicleType};
use super::health::{VehicleDamageState, VehicleHealth};
use super::explosion::VehicleExplosion;
use super::visuals::VehicleDamageVisuals;
use crate::combat::DamageEvent;

/// 初始化車輛損壞視覺效果資源
pub fn setup_vehicle_damage_effects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(VehicleDamageVisuals::new(&mut meshes, &mut materials));
    info!("💥 車輛損壞系統已初始化");
}

// ============================================================================
// 車輛碰撞傷害常數
// ============================================================================

/// 碰撞傷害冷卻時間（秒）- 防止持續接觸時每幀扣血
const COLLISION_DAMAGE_COOLDOWN: f32 = 0.5;
/// 造成傷害的最低速度門檻（m/s）
const COLLISION_DAMAGE_SPEED_THRESHOLD: f32 = 10.0;
/// 傷害倍率：每超過門檻 1 m/s 造成此數值傷害
const COLLISION_DAMAGE_MULTIPLIER: f32 = 5.0;

/// 車輛碰撞傷害系統
/// 根據碰撞速度計算車輛傷害
pub fn vehicle_collision_damage_system(
    time: Res<Time>,
    rapier_context: ReadRapierContext,
    mut vehicle_query: Query<(Entity, &Transform, &Vehicle, &mut VehicleHealth)>,
) {
    let current_time = time.elapsed_secs();

    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    for (entity, _transform, vehicle, mut health) in vehicle_query.iter_mut() {
        // 已爆炸的車輛不處理
        if health.is_destroyed() {
            continue;
        }

        // 碰撞傷害冷卻：防止持續接觸時每幀扣血
        if current_time - health.last_damage_time < COLLISION_DAMAGE_COOLDOWN {
            continue;
        }

        // 檢查碰撞事件
        for contact_pair in rapier.contact_pairs_with(entity) {
            // 只處理有接觸的碰撞
            if !contact_pair.has_any_active_contact() {
                continue;
            }

            // 根據速度計算傷害
            // 速度越快，傷害越高
            let speed = vehicle.current_speed.abs();
            if speed < COLLISION_DAMAGE_SPEED_THRESHOLD {
                continue; // 低速碰撞不造成傷害
            }

            // 傷害公式：(速度 - 門檻) * 倍率
            // 例如：30 m/s = (30-10) * 5 = 100 傷害
            let damage = (speed - COLLISION_DAMAGE_SPEED_THRESHOLD) * COLLISION_DAMAGE_MULTIPLIER;
            health.take_damage(damage, current_time);
            break; // 一次碰撞只計算一次傷害
        }
    }
}

/// 取得車輛類型對應的爆炸半徑
fn get_explosion_radius(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 5.0,
        VehicleType::Car | VehicleType::Taxi => 8.0,
        VehicleType::Bus => 12.0,
    }
}

/// 取得車輛類型對應的爆炸傷害
fn get_explosion_damage(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 100.0,
        VehicleType::Car | VehicleType::Taxi => 200.0,
        VehicleType::Bus => 300.0,
    }
}

/// 車輛火焰系統
/// 處理著火狀態和爆炸倒計時
pub fn vehicle_fire_system(
    mut commands: Commands,
    time: Res<Time>,
    mut vehicle_query: Query<(Entity, &Transform, &Vehicle, &mut VehicleHealth)>,
    damage_visuals: Option<Res<VehicleDamageVisuals>>,
) {
    let dt = time.delta_secs();

    for (entity, transform, vehicle, mut health) in vehicle_query.iter_mut() {
        if health.is_destroyed() {
            continue;
        }

        if !health.tick_fire(dt) {
            continue;
        }

        // 爆炸！
        let explosion_pos = transform.translation + Vec3::Y * 0.5;
        let explosion_radius = get_explosion_radius(vehicle.vehicle_type);
        let explosion_damage = get_explosion_damage(vehicle.vehicle_type);

        if let Some(ref visuals) = damage_visuals {
            commands.spawn((
                Mesh3d(visuals.explosion_mesh.clone()),
                MeshMaterial3d(visuals.explosion_material.clone()),
                Transform::from_translation(explosion_pos),
                VehicleExplosion::new(explosion_pos, explosion_radius, explosion_damage),
            ));
        }

        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
        }
    }
}

/// 車輛傷害事件處理系統
/// 監聽 DamageEvent 並對車輛 VehicleHealth 造成傷害
/// 這使車輛可以被子彈、爆炸等傷害
pub fn vehicle_damage_event_system(
    time: Res<Time>,
    mut damage_events: MessageReader<DamageEvent>,
    mut vehicle_query: Query<&mut VehicleHealth>,
) {
    let current_time = time.elapsed_secs();

    for event in damage_events.read() {
        // 檢查目標是否是有 VehicleHealth 的車輛
        if let Ok(mut health) = vehicle_query.get_mut(event.target) {
            // 已爆炸的車輛不處理
            if health.damage_state == VehicleDamageState::Destroyed {
                continue;
            }

            // 對車輛造成傷害
            let damage_dealt = health.take_damage(event.amount, current_time);

            if damage_dealt > 0.0 {
                // 可以在這裡添加車輛受傷的視覺/音效回饋
                // 例如金屬撞擊音效
            }
        }
    }
}
