//! 爆炸事件處理與火焰區域
//!
//! 處理爆炸傷害分配、視覺效果生成、火焰區域持續傷害。

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};

use super::effects::{spawn_fire_particles, spawn_smoke_particles};
use super::*;
use super::super::health::*;

/// 處理爆炸事件
pub fn handle_explosion_event_system(
    mut commands: Commands,
    mut explosion_events: MessageReader<ExplosionEvent>,
    visuals: Option<Res<ExplosiveVisuals>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut damage_events: MessageWriter<DamageEvent>,
    damageable_query: Query<(Entity, &Transform), With<Damageable>>,
    rapier_context: ReadRapierContext,
) {
    let Some(visuals) = visuals else {
        return;
    };
    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    for event in explosion_events.read() {
        let position = event.position;

        // 範圍傷害（含牆壁遮擋檢測）
        for (target, target_transform) in &damageable_query {
            let target_pos = target_transform.translation;
            let distance = position.distance(target_pos);

            if distance < event.radius && distance > 0.1 {
                // 檢查是否有牆壁遮擋（raycast）
                // 從稍微上方發射射線（避免地面干擾）
                let ray_origin = position + Vec3::Y * 0.5;
                let ray_target = target_pos + Vec3::Y * 0.5;
                let ray_dir = (ray_target - ray_origin).normalize();
                let max_toi = distance as RapierReal;

                // 使用 solid=true 檢測第一個障礙物
                let filter = QueryFilter::default().exclude_collider(target); // 排除目標本身

                let has_obstacle = rapier
                    .cast_ray(ray_origin, ray_dir, max_toi, true, filter)
                    .is_some();

                // 只有沒有障礙物時才造成傷害
                if !has_obstacle {
                    // 傷害隨距離衰減（平方根曲線，中距離傷害更高）
                    let damage_ratio = (1.0 - (distance / event.radius).sqrt()).max(0.0);
                    let damage = event.max_damage * damage_ratio;

                    damage_events.write(DamageEvent {
                        target,
                        amount: damage,
                        source: DamageSource::Explosion,
                        attacker: event.source,
                        hit_position: Some(target_pos),
                        is_headshot: false,
                    });
                }
            }
        }

        // 生成爆炸視覺效果
        match event.explosive_type {
            ExplosiveType::Molotov => {
                // 燃燒瓶：生成火焰區域 + 煙霧發射器
                commands.spawn((
                    Mesh3d(visuals.fire_mesh.clone()),
                    MeshMaterial3d(visuals.fire_material.clone()),
                    Transform::from_translation(position).with_scale(Vec3::new(
                        event.radius,
                        1.0,
                        event.radius,
                    )),
                    FireZone::default(),
                    SmokeEmitter {
                        particles_per_second: 8.0, // 火焰產生較多煙霧
                        remaining_time: MOLOTOV_FIRE_DURATION,
                        radius: event.radius * 0.8,
                        ..default()
                    },
                ));

                // 生成初始火焰粒子
                spawn_fire_particles(
                    &mut commands,
                    &visuals,
                    &mut materials,
                    position,
                    event.radius,
                    5,
                );
            }
            _ => {
                // 手榴彈/黏性炸彈：生成爆炸效果
                commands.spawn((
                    Mesh3d(visuals.explosion_mesh.clone()),
                    MeshMaterial3d(visuals.explosion_material.clone()),
                    Transform::from_translation(position),
                    ExplosionEffect::new(event.radius, event.max_damage, 0.5),
                ));

                // 生成衝擊波效果（GTA5 風格的擴散環）
                // 每個衝擊波需要獨立的材質實例，避免多個衝擊波共享材質導致視覺錯誤
                let shockwave_material = {
                    let base_mat = materials.get(&visuals.shockwave_material).cloned();
                    base_mat
                        .map(|m| materials.add(m))
                        .unwrap_or_else(|| visuals.shockwave_material.clone())
                };

                commands.spawn((
                    Mesh3d(visuals.shockwave_mesh.clone()),
                    MeshMaterial3d(shockwave_material),
                    Transform::from_translation(position + Vec3::Y * 0.1)  // 稍微抬高避免地面穿透
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),  // 水平放置
                    ShockwaveEffect::new(event.radius * 1.5),  // 衝擊波比爆炸半徑大 50%
                ));

                // 生成爆炸煙霧粒子（GTA5 風格）
                spawn_smoke_particles(
                    &mut commands,
                    &visuals,
                    &mut materials,
                    position,
                    event.radius,
                    8,
                );
            }
        }

        info!("{} 爆炸於 {:?}", event.explosive_type.name(), position);
    }
}

/// 火焰區域更新系統
pub fn fire_zone_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut fire_query: Query<(Entity, &Transform, &mut FireZone)>,
    mut damage_events: MessageWriter<DamageEvent>,
    damageable_query: Query<(Entity, &Transform), With<Damageable>>,
) {
    for (fire_entity, fire_transform, mut fire) in &mut fire_query {
        fire.remaining_time -= time.delta_secs();
        fire.damage_tick -= time.delta_secs();

        if fire.remaining_time <= 0.0 {
            commands.entity(fire_entity).despawn();
            continue;
        }

        // 每 0.5 秒造成一次傷害
        if fire.damage_tick <= 0.0 {
            fire.damage_tick = 0.5;

            let fire_pos = fire_transform.translation;
            let radius_sq = fire.radius * fire.radius;
            let damage = fire.damage_per_second * 0.5; // 半秒傷害（預計算）

            for (target, target_transform) in &damageable_query {
                // 使用距離平方避免 sqrt 計算
                let distance_sq = fire_pos.distance_squared(target_transform.translation);
                if distance_sq < radius_sq {
                    damage_events.write(DamageEvent {
                        target,
                        amount: damage,
                        source: DamageSource::Fire,
                        attacker: None,
                        hit_position: Some(target_transform.translation),
                        is_headshot: false,
                    });
                }
            }
        }
    }
}
