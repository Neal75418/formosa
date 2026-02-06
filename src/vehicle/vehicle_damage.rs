//! 載具損壞系統（碰撞傷害、火災、爆炸）

use super::*;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

// ============================================================================
// 車輛損壞系統（GTA 5 風格）
// ============================================================================

use super::{
    VehicleDamageSmoke, VehicleDamageState, VehicleDamageVisuals, VehicleExplosion, VehicleFire,
    VehicleHealth,
};
use crate::combat::{DamageEvent, DamageSource, Enemy};
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::wanted::PoliceOfficer;

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

// ============================================================================
// 車輛損壞視覺效果常數
// ============================================================================
/// 中度損壞煙霧生成率（每秒）
const MODERATE_SMOKE_RATE: f32 = 1.2;
/// 嚴重損壞煙霧生成率（每秒）
const HEAVY_SMOKE_RATE: f32 = 6.0;
/// 瀕臨爆炸煙霧生成率（每秒）
const CRITICAL_SMOKE_RATE: f32 = 9.0;
/// 瀕臨爆炸火焰生成率（每秒）
const CRITICAL_FIRE_RATE: f32 = 6.0;

/// 車輛損壞視覺效果系統
/// 根據損壞狀態生成煙霧和火焰粒子
/// 使用時間基準的生成率，確保效果與幀率無關
pub fn vehicle_damage_effect_system(
    mut commands: Commands,
    time: Res<Time>,
    damage_visuals: Option<Res<VehicleDamageVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle, &VehicleHealth)>,
) {
    let Some(visuals) = damage_visuals else {
        return;
    };
    let dt = time.delta_secs();
    let mut rng = rand::rng();

    for (transform, vehicle, health) in vehicle_query.iter() {
        // 計算引擎蓋位置（車頭）
        let hood_offset = match vehicle.vehicle_type {
            VehicleType::Scooter => Vec3::new(0.0, 0.3, -0.6),
            VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.5, -1.5),
            VehicleType::Bus => Vec3::new(0.0, 1.0, -4.0),
        };
        let hood_pos = transform.translation + transform.rotation * hood_offset;

        // 根據損壞狀態生成效果
        // 使用時間基準的機率：rate * dt 使生成與幀率無關
        match health.damage_state {
            VehicleDamageState::Moderate => {
                // 中度損壞：偶爾冒白煙
                if rng.random::<f32>() < MODERATE_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, false, &mut rng);
                }
            }
            VehicleDamageState::Heavy => {
                // 嚴重損壞：持續冒黑煙
                if rng.random::<f32>() < HEAVY_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, true, &mut rng);
                }
            }
            VehicleDamageState::Critical => {
                // 瀕臨爆炸：冒黑煙 + 火焰
                if rng.random::<f32>() < CRITICAL_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, true, &mut rng);
                }
                if rng.random::<f32>() < CRITICAL_FIRE_RATE * dt {
                    spawn_vehicle_fire(&mut commands, &visuals, hood_pos, &mut rng);
                }
            }
            _ => {}
        }
    }
}

/// 生成損壞煙霧粒子
fn spawn_damage_smoke(
    commands: &mut Commands,
    visuals: &VehicleDamageVisuals,
    position: Vec3,
    is_heavy: bool,
    rng: &mut rand::prelude::ThreadRng,
) {
    let spread = Vec3::new(
        rng.random_range(-0.3..0.3),
        rng.random_range(0.5..1.5),
        rng.random_range(-0.3..0.3),
    );

    let material = if is_heavy {
        visuals.heavy_smoke_material.clone()
    } else {
        visuals.light_smoke_material.clone()
    };

    commands.spawn((
        Mesh3d(visuals.smoke_mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(position).with_scale(Vec3::splat(0.2)),
        VehicleDamageSmoke::new(spread, is_heavy),
    ));
}

/// 生成車輛火焰粒子
fn spawn_vehicle_fire(
    commands: &mut Commands,
    visuals: &VehicleDamageVisuals,
    position: Vec3,
    rng: &mut rand::prelude::ThreadRng,
) {
    let spread = Vec3::new(
        rng.random_range(-0.2..0.2),
        rng.random_range(0.8..1.5),
        rng.random_range(-0.2..0.2),
    );

    commands.spawn((
        Mesh3d(visuals.fire_mesh.clone()),
        MeshMaterial3d(visuals.fire_material.clone()),
        Transform::from_translation(position + Vec3::Y * 0.1)
            .with_scale(Vec3::splat(rng.random_range(0.3..0.6))),
        VehicleFire::new(spread),
    ));
}

/// 對範圍內的目標造成爆炸傷害
fn apply_explosion_damage_to_targets<'a>(
    targets: impl Iterator<Item = (Entity, &'a Transform)>,
    explosion_center: Vec3,
    explosion_radius: f32,
    explosion_damage: f32,
    damage_events: &mut MessageWriter<DamageEvent>,
    exclude_entity: Option<Entity>,
) {
    for (target_entity, target_transform) in targets {
        if Some(target_entity) == exclude_entity {
            continue;
        }
        let distance = explosion_center.distance(target_transform.translation);
        if distance < explosion_radius {
            let damage_factor = 1.0 - (distance / explosion_radius);
            damage_events.write(
                DamageEvent::new(
                    target_entity,
                    explosion_damage * damage_factor,
                    DamageSource::Explosion,
                )
                .with_position(explosion_center),
            );
        }
    }
}

/// 觸發爆炸攝影機震動效果
fn trigger_explosion_camera_shake(
    explosion_center: Vec3,
    explosion_radius: f32,
    player_pos: Vec3,
    camera_shake: &mut crate::core::CameraShake,
) {
    let distance_to_player = explosion_center.distance(player_pos);
    let max_shake_distance = explosion_radius * 3.0;

    if distance_to_player < max_shake_distance {
        let falloff = 1.0 - distance_to_player / max_shake_distance;
        camera_shake.trigger(0.5 * falloff, 0.4 + 0.3 * falloff);
    }
}

/// 車輛爆炸系統
/// 處理爆炸效果和範圍傷害
/// 對範圍內的所有可傷害實體（玩家、敵人、行人、警察、其他車輛）造成傷害
#[allow(clippy::type_complexity)]
pub fn vehicle_explosion_system(
    mut commands: Commands,
    time: Res<Time>,
    mut camera_shake: ResMut<crate::core::CameraShake>,
    mut explosion_query: Query<(Entity, &mut VehicleExplosion, &mut Transform)>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<VehicleExplosion>)>,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<VehicleExplosion>)>,
    pedestrian_query: Query<
        (Entity, &Transform),
        (
            With<Pedestrian>,
            Without<Player>,
            Without<Enemy>,
            Without<VehicleExplosion>,
        ),
    >,
    police_query: Query<
        (Entity, &Transform),
        (
            With<PoliceOfficer>,
            Without<Player>,
            Without<Enemy>,
            Without<VehicleExplosion>,
        ),
    >,
    vehicle_query: Query<(Entity, &Transform), (With<VehicleHealth>, Without<VehicleExplosion>)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let dt = time.delta_secs();

    for (entity, mut explosion, mut transform) in explosion_query.iter_mut() {
        explosion.lifetime += dt;
        transform.scale = Vec3::splat(explosion.scale());

        if !explosion.has_damage_dealt {
            explosion.has_damage_dealt = true;

            if let Ok((_, player_transform)) = player_query.single() {
                trigger_explosion_camera_shake(
                    explosion.center,
                    explosion.radius,
                    player_transform.translation,
                    &mut camera_shake,
                );
            }

            apply_explosion_damage_to_targets(
                player_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                enemy_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                pedestrian_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                police_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                None,
            );
            apply_explosion_damage_to_targets(
                vehicle_query.iter(),
                explosion.center,
                explosion.radius,
                explosion.damage,
                &mut damage_events,
                Some(entity),
            );
        }

        if explosion.lifetime >= explosion.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
        }
    }
}

/// 車輛損壞粒子更新系統
/// 處理煙霧和火焰粒子的移動和刪除
pub fn vehicle_damage_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut smoke_query: Query<(Entity, &mut VehicleDamageSmoke, &mut Transform)>,
    mut fire_query: Query<(Entity, &mut VehicleFire, &mut Transform), Without<VehicleDamageSmoke>>,
) {
    let dt = time.delta_secs();

    // 更新煙霧粒子
    for (entity, mut smoke, mut transform) in smoke_query.iter_mut() {
        smoke.lifetime += dt;

        if smoke.lifetime >= smoke.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 煙霧上飄並減速
        smoke.velocity *= 1.0 - dt * 1.5;
        transform.translation += smoke.velocity * dt;

        // 擴散變大
        let progress = smoke.lifetime / smoke.max_lifetime;
        let scale = 0.2 + progress * 0.6;
        transform.scale = Vec3::splat(scale);
    }

    // 更新火焰粒子
    for (entity, mut fire, mut transform) in fire_query.iter_mut() {
        fire.lifetime += dt;

        if fire.lifetime >= fire.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 火焰快速上飄
        transform.translation += fire.velocity * dt;

        // 閃爍效果
        let flicker = (fire.lifetime * 20.0).sin() * 0.1 + 1.0;
        transform.scale = Vec3::splat(fire.scale() * flicker);
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
