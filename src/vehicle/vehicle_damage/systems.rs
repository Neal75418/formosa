//! 車輛損壞系統（碰撞、火焰、傷害事件處理）

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use super::super::{Vehicle, VehicleType};
use super::health::{TireDamage, VehicleDamageState, VehicleHealth};
use super::explosion::VehicleExplosion;
use super::visuals::VehicleDamageVisuals;
use crate::combat::{DamageEvent, DamageSource};

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

/// 子彈命中輪胎的判定距離平方（0.8m²）
const TIRE_HIT_RADIUS_SQ: f32 = 0.8 * 0.8;
/// 子彈爆胎機率（每次命中輪胎區域）
const TIRE_POP_CHANCE: f32 = 0.3;

/// 取得車輛各輪胎的局部位置（左前、右前、左後、右後）
fn get_tire_local_positions(vehicle_type: VehicleType) -> [Vec3; 4] {
    let (half_width, wheel_offset_z) = match vehicle_type {
        VehicleType::Car | VehicleType::Taxi => (1.0, 1.2),
        VehicleType::Bus => (1.4, 2.5),
        VehicleType::Scooter => (0.3, 0.6),
    };
    let wheel_y = -0.3;
    [
        Vec3::new(-half_width, wheel_y, -wheel_offset_z), // 左前
        Vec3::new(half_width, wheel_y, -wheel_offset_z),  // 右前
        Vec3::new(-half_width, wheel_y, wheel_offset_z),  // 左後
        Vec3::new(half_width, wheel_y, wheel_offset_z),   // 右後
    ]
}

/// 車輛傷害事件處理系統
/// 監聽 DamageEvent 並對車輛 VehicleHealth 造成傷害
/// 子彈命中輪胎附近時有機率爆胎
pub fn vehicle_damage_event_system(
    time: Res<Time>,
    mut damage_events: MessageReader<DamageEvent>,
    mut vehicle_query: Query<(&Transform, &Vehicle, &mut VehicleHealth, Option<&mut TireDamage>)>,
) {
    let current_time = time.elapsed_secs();

    for event in damage_events.read() {
        // 檢查目標是否是有 VehicleHealth 的車輛
        let Ok((transform, vehicle, mut health, tire_damage)) =
            vehicle_query.get_mut(event.target)
        else {
            continue;
        };

        // 已爆炸的車輛不處理
        if health.damage_state == VehicleDamageState::Destroyed {
            continue;
        }

        // 對車輛造成傷害（無敵車輛回傳 0）
        let damage_dealt = health.take_damage(event.amount, current_time);

        // 子彈命中時檢查輪胎爆胎（僅在實際造成傷害時才判定）
        if damage_dealt > 0.0 && event.source == DamageSource::Bullet {
            if let (Some(hit_pos), Some(mut td)) = (event.hit_position, tire_damage) {
                try_pop_tire_at_hit(
                    &mut td,
                    hit_pos,
                    transform,
                    vehicle.vehicle_type,
                );
            }
        }
    }
}

/// 嘗試在命中位置爆破最近的輪胎
fn try_pop_tire_at_hit(
    tire_damage: &mut TireDamage,
    hit_pos: Vec3,
    vehicle_transform: &Transform,
    vehicle_type: VehicleType,
) {
    let tire_positions = get_tire_local_positions(vehicle_type);

    // 車輛不應有非單位 scale（若有會導致局部座標計算錯誤）
    debug_assert!(
        vehicle_transform.scale.abs_diff_eq(Vec3::ONE, 0.01),
        "Vehicle scale must be ~1.0, got {:?}",
        vehicle_transform.scale
    );

    let inv_rotation = vehicle_transform.rotation.inverse();

    // 將命中點轉為車輛局部座標
    let local_hit = inv_rotation * (hit_pos - vehicle_transform.translation);

    // 找最近的輪胎
    let mut closest_idx = 0;
    let mut closest_dist_sq = f32::MAX;

    for (i, tire_pos) in tire_positions.iter().enumerate() {
        let dist_sq = local_hit.distance_squared(*tire_pos);
        if dist_sq < closest_dist_sq {
            closest_dist_sq = dist_sq;
            closest_idx = i;
        }
    }

    // 命中輪胎附近且該輪胎尚未爆破 → 機率爆胎
    if closest_dist_sq < TIRE_HIT_RADIUS_SQ
        && !tire_damage.flat_tires[closest_idx]
        && rand::random::<f32>() < TIRE_POP_CHANCE
    {
        tire_damage.pop_tire(closest_idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // get_tire_local_positions tests
    // ========================================================================

    #[test]
    fn tire_positions_car_symmetric() {
        let pos = get_tire_local_positions(VehicleType::Car);
        // 左右對稱（X 軸）
        assert_eq!(pos[0].x, -pos[1].x);
        assert_eq!(pos[2].x, -pos[3].x);
        // 前後對稱（Z 軸）
        assert_eq!(pos[0].z, -pos[2].z);
    }

    #[test]
    fn tire_positions_bus_wider_than_car() {
        let car = get_tire_local_positions(VehicleType::Car);
        let bus = get_tire_local_positions(VehicleType::Bus);
        // 巴士比轎車更寬
        assert!(bus[0].x.abs() > car[0].x.abs());
        // 巴士前後輪距更大
        assert!(bus[0].z.abs() > car[0].z.abs());
    }

    #[test]
    fn tire_positions_scooter_narrowest() {
        let scooter = get_tire_local_positions(VehicleType::Scooter);
        let car = get_tire_local_positions(VehicleType::Car);
        assert!(scooter[0].x.abs() < car[0].x.abs());
    }

    #[test]
    fn tire_positions_all_types_have_four() {
        for vt in [VehicleType::Scooter, VehicleType::Car, VehicleType::Taxi, VehicleType::Bus] {
            let pos = get_tire_local_positions(vt);
            assert_eq!(pos.len(), 4);
        }
    }

    // ========================================================================
    // try_pop_tire_at_hit tests
    // ========================================================================

    fn make_transform(pos: Vec3, yaw: f32) -> Transform {
        Transform {
            translation: pos,
            rotation: Quat::from_rotation_y(yaw),
            scale: Vec3::ONE,
        }
    }

    #[test]
    fn hit_directly_on_tire_pops_it() {
        let mut td = TireDamage::default();
        let transform = make_transform(Vec3::ZERO, 0.0);
        let tire_positions = get_tire_local_positions(VehicleType::Car);

        // 直接命中第一個輪胎位置（多次嘗試確保機率觸發）
        for _ in 0..100 {
            if td.flat_tires[0] {
                break;
            }
            try_pop_tire_at_hit(&mut td, tire_positions[0], &transform, VehicleType::Car);
        }
        // 30% 機率 × 100 次，應該至少觸發一次
        assert!(td.flat_tires[0], "直接命中輪胎 100 次後應該爆破");
    }

    #[test]
    fn hit_far_from_tires_does_not_pop() {
        let mut td = TireDamage::default();
        let transform = make_transform(Vec3::ZERO, 0.0);

        // 在遠離所有輪胎的位置（車頂）
        let far_hit = Vec3::new(0.0, 5.0, 0.0);
        for _ in 0..100 {
            try_pop_tire_at_hit(&mut td, far_hit, &transform, VehicleType::Car);
        }
        assert_eq!(td.flat_count(), 0, "遠離輪胎的命中不應爆胎");
    }

    #[test]
    fn already_popped_tire_not_popped_again() {
        let mut td = TireDamage::default();
        td.pop_tire(0); // 預先爆破
        let original_count = td.flat_count();

        let transform = make_transform(Vec3::ZERO, 0.0);
        let tire_positions = get_tire_local_positions(VehicleType::Car);

        // 多次命中已爆輪胎
        for _ in 0..100 {
            try_pop_tire_at_hit(&mut td, tire_positions[0], &transform, VehicleType::Car);
        }
        assert_eq!(td.flat_count(), original_count, "已爆輪胎不應再被爆");
    }

    #[test]
    fn rotated_vehicle_tire_detection() {
        let mut td = TireDamage::default();
        // 車輛旋轉 90 度（面向 X 正方向）
        let transform = make_transform(Vec3::new(10.0, 0.0, 5.0), std::f32::consts::FRAC_PI_2);
        let tire_positions = get_tire_local_positions(VehicleType::Car);

        // 將局部輪胎位置轉為世界座標
        let world_tire = transform.translation + transform.rotation * tire_positions[2];

        for _ in 0..100 {
            if td.flat_tires[2] {
                break;
            }
            try_pop_tire_at_hit(&mut td, world_tire, &transform, VehicleType::Car);
        }
        assert!(td.flat_tires[2], "旋轉後的輪胎座標轉換應正確");
    }

    // ========================================================================
    // Explosion helpers tests
    // ========================================================================

    #[test]
    fn explosion_radius_scales_with_vehicle_size() {
        assert!(get_explosion_radius(VehicleType::Scooter) < get_explosion_radius(VehicleType::Car));
        assert!(get_explosion_radius(VehicleType::Car) < get_explosion_radius(VehicleType::Bus));
    }

    #[test]
    fn explosion_damage_scales_with_vehicle_size() {
        assert!(get_explosion_damage(VehicleType::Scooter) < get_explosion_damage(VehicleType::Car));
        assert!(get_explosion_damage(VehicleType::Car) < get_explosion_damage(VehicleType::Bus));
    }

    #[test]
    fn taxi_matches_car_explosion() {
        assert_eq!(get_explosion_radius(VehicleType::Taxi), get_explosion_radius(VehicleType::Car));
        assert_eq!(get_explosion_damage(VehicleType::Taxi), get_explosion_damage(VehicleType::Car));
    }
}
