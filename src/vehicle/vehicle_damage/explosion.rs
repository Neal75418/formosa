//! 車輛爆炸效果和系統

use bevy::prelude::*;
use crate::core::lifetime_linear_alpha;
use crate::combat::{DamageEvent, DamageSource, Enemy};
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::wanted::PoliceOfficer;
use super::health::VehicleHealth;

/// 車輛爆炸效果
#[derive(Component)]
pub struct VehicleExplosion {
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 爆炸中心
    pub center: Vec3,
    /// 爆炸範圍
    pub radius: f32,
    /// 傷害
    pub damage: f32,
    /// 是否已造成傷害
    pub has_damage_dealt: bool,
}

impl VehicleExplosion {
    /// 建立新實例
    pub fn new(center: Vec3, radius: f32, damage: f32) -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 1.0,
            center,
            radius,
            damage,
            has_damage_dealt: false,
        }
    }

    /// 計算當前縮放（先擴大後縮小）
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if progress < 0.3 {
            // 快速擴大
            progress / 0.3 * 1.5
        } else {
            // 緩慢縮小
            1.5 - (progress - 0.3) / 0.7 * 1.5
        }
    }

    /// 計算透明度
    pub fn alpha(&self) -> f32 {
        lifetime_linear_alpha(self.lifetime, self.max_lifetime)
    }
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

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explosion_scale_expand_then_shrink() {
        let mut e = VehicleExplosion::new(Vec3::ZERO, 5.0, 100.0);
        assert!((e.scale() - 0.0).abs() < 0.01);
        e.lifetime = 0.3;
        assert!((e.scale() - 1.5).abs() < 0.01);
        e.lifetime = 1.0;
        assert!((e.scale() - 0.0).abs() < 0.01);
    }

    #[test]
    fn explosion_alpha_fades() {
        let mut e = VehicleExplosion::new(Vec3::ZERO, 5.0, 100.0);
        assert!((e.alpha() - 1.0).abs() < f32::EPSILON);
        e.lifetime = 0.5;
        assert!((e.alpha() - 0.5).abs() < f32::EPSILON);
        e.lifetime = 1.0;
        assert!((e.alpha() - 0.0).abs() < f32::EPSILON);
    }
}
