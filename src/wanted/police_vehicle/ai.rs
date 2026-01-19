//! 警車 AI 行為系統

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::player::Player;
use crate::core::GameState;
use crate::vehicle::{Vehicle, VehicleHealth};
use crate::combat::{DamageEvent, DamageSource};
use crate::wanted::WantedLevel;

use super::{
    PoliceCar, PoliceCarState, PoliceCarConfig, SirenLight,
    PIT_MANEUVER_DISTANCE_SQ, PIT_MANEUVER_ANGLE, PIT_ABANDON_DISTANCE_SQ,
    CHASE_SWITCH_DISTANCE_SQ, CHASE_SPEED_MULTIPLIER,
    INTERCEPT_DISTANCE_SQ, INTERCEPT_ABANDON_DISTANCE_SQ,
    FRONT_DOT_THRESHOLD, POLICE_CAR_COLLISION_DAMAGE,
};

// ============================================================================
// AI 系統
// ============================================================================

/// 警車 AI 系統
pub fn police_car_ai_system(
    mut police_car_query: Query<(
        Entity,
        &Transform,
        &mut PoliceCar,
        &mut Vehicle,
        &mut ExternalForce,
        &VehicleHealth,
    )>,
    player_query: Query<(&Transform, &Velocity), (With<Player>, Without<PoliceCar>)>,
    game_state: Res<GameState>,
    wanted: Res<WantedLevel>,
    config: Res<PoliceCarConfig>,
    time: Res<Time>,
) {
    // 玩家不在車上或沒有通緝，警車停止追逐
    if !game_state.player_in_vehicle || wanted.stars < 2 {
        return;
    }

    let Ok((player_transform, player_velocity)) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation;
    let player_vel = player_velocity.linvel;
    let dt = time.delta_secs();

    for (_entity, transform, mut police_car, mut _vehicle, mut force, health) in &mut police_car_query {
        // 車輛已損壞，停止 AI
        if health.is_destroyed() {
            police_car.state = PoliceCarState::Disabled;
            continue;
        }

        let car_pos = transform.translation;
        let car_forward = transform.forward().as_vec3();
        let to_player = player_pos - car_pos;
        let distance_sq = to_player.length_squared();
        let direction = to_player.normalize_or_zero();
        let direction = if direction == Vec3::ZERO { car_forward } else { direction };

        // 更新冷卻計時器
        if police_car.pit_cooldown > 0.0 {
            police_car.pit_cooldown -= dt;
        }

        // 根據狀態執行行為（使用 distance_sq 避免 sqrt 運算）
        match police_car.state {
            PoliceCarState::Responding => {
                // 前往玩家位置
                if distance_sq < CHASE_SWITCH_DISTANCE_SQ {
                    police_car.state = PoliceCarState::Chasing;
                } else {
                    // 直線前往
                    let target_speed = config.chase_speed * CHASE_SPEED_MULTIPLIER;
                    apply_driving_force(&mut force, transform, direction, target_speed, dt);
                }
            }

            PoliceCarState::Chasing => {
                // 追逐玩家車輛
                police_car.chase_timer += dt;

                // 預測玩家位置
                let predicted_pos = player_pos + player_vel * 1.0;
                let to_predicted = (predicted_pos - car_pos).normalize_or_zero();

                // 檢查是否可以執行 PIT 機動（距離夠近）
                if distance_sq < PIT_MANEUVER_DISTANCE_SQ * 4.0
                    && police_car.pit_cooldown <= 0.0
                    && wanted.stars >= 3
                {
                    // 檢查角度
                    let angle = car_forward.dot(to_player).acos();
                    if angle < PIT_MANEUVER_ANGLE {
                        police_car.state = PoliceCarState::PitManeuver;
                        continue;
                    }
                }

                // 檢查是否應該攔截
                if distance_sq > INTERCEPT_DISTANCE_SQ && police_car.chase_timer > 5.0 {
                    // 每 5 秒有機會嘗試攔截
                    if rand::random::<f32>() < 0.3 {
                        police_car.state = PoliceCarState::Intercepting;
                        police_car.chase_timer = 0.0;
                        continue;
                    }
                }

                // 追逐
                let target_speed = config.chase_speed;
                apply_driving_force(&mut force, transform, to_predicted, target_speed, dt);
            }

            PoliceCarState::PitManeuver => {
                // PIT 機動：撞擊玩家車輛後側
                let target_offset = player_pos - car_forward * 2.0 + car_forward.cross(Vec3::Y) * 1.5;
                let to_target = (target_offset - car_pos).normalize_or_zero();

                apply_driving_force(&mut force, transform, to_target, config.pit_speed, dt);

                // 撞擊後重置狀態
                if distance_sq < PIT_MANEUVER_DISTANCE_SQ {
                    police_car.state = PoliceCarState::Chasing;
                    police_car.pit_cooldown = 10.0; // 10 秒冷卻
                }

                // 如果距離太遠，放棄 PIT
                if distance_sq > PIT_ABANDON_DISTANCE_SQ {
                    police_car.state = PoliceCarState::Chasing;
                }
            }

            PoliceCarState::Intercepting => {
                // 嘗試擋在玩家前方
                let intercept_pos = player_pos + player_vel.normalize_or_zero() * 30.0;
                let to_intercept = (intercept_pos - car_pos).normalize_or_zero();

                apply_driving_force(&mut force, transform, to_intercept, config.intercept_speed, dt);

                // 如果已經在前方或太遠，恢復追逐
                let dot = car_forward.dot(-to_player);
                if dot > FRONT_DOT_THRESHOLD || distance_sq > INTERCEPT_ABANDON_DISTANCE_SQ {
                    police_car.state = PoliceCarState::Chasing;
                }
            }

            PoliceCarState::Disabled => {
                // 停止所有力
                force.force = Vec3::ZERO;
                force.torque = Vec3::ZERO;
            }
        }
    }
}

/// 應用駕駛力到車輛
fn apply_driving_force(
    force: &mut ExternalForce,
    transform: &Transform,
    target_direction: Vec3,
    target_speed: f32,
    dt: f32,
) {
    let car_forward = transform.forward().as_vec3();

    // 計算目標方向與當前方向的夾角
    let dot = car_forward.dot(target_direction);
    let cross = car_forward.cross(target_direction).y;

    // 轉向力矩
    let turn_strength = cross.signum() * (1.0 - dot.abs()).min(1.0) * 50000.0;
    force.torque = Vec3::new(0.0, turn_strength * dt, 0.0);

    // 前進力
    let forward_force = car_forward * target_speed * 1000.0;
    force.force = forward_force * dt;
}

// ============================================================================
// 碰撞系統
// ============================================================================

/// 警車碰撞處理系統
pub fn police_car_collision_system(
    mut collision_events: MessageReader<CollisionEvent>,
    mut police_car_query: Query<(&Transform, &mut PoliceCar, &mut VehicleHealth)>,
    player_vehicle_query: Query<Entity, (With<Player>, With<Vehicle>)>,
    mut damage_events: MessageWriter<DamageEvent>,
    time: Res<Time>,
) {
    let Ok(player_vehicle) = player_vehicle_query.single() else {
        return;
    };

    for event in collision_events.read() {
        let CollisionEvent::Started(entity1, entity2, _) = event else {
            continue;
        };

        // 檢查是否是警車與玩家車輛的碰撞
        let (police_entity, is_player_collision) = if *entity1 == player_vehicle {
            (*entity2, true)
        } else if *entity2 == player_vehicle {
            (*entity1, true)
        } else {
            continue;
        };

        if !is_player_collision {
            continue;
        }

        // 獲取警車資料
        let Ok((transform, mut police_car, mut health)) = police_car_query.get_mut(police_entity) else {
            continue;
        };

        // 碰撞冷卻
        let elapsed = time.elapsed_secs();
        if elapsed - police_car.last_collision_time < 1.0 {
            continue;
        }
        police_car.last_collision_time = elapsed;

        // 警車受傷
        health.take_damage(50.0, elapsed);

        // 對玩家造成傷害
        damage_events.write(DamageEvent {
            target: player_vehicle,
            amount: POLICE_CAR_COLLISION_DAMAGE,
            source: DamageSource::Explosion, // 使用爆炸類型表示碰撞
            attacker: Some(police_entity),
            hit_position: Some(transform.translation),
            is_headshot: false,
        });

        info!("警車碰撞！警車血量: {:.0}", health.current);
    }
}

// ============================================================================
// 視覺效果系統
// ============================================================================

/// 警笛燈閃爍系統
pub fn siren_light_system(
    mut siren_query: Query<(&mut SirenLight, &mut Visibility)>,
    _police_car_query: Query<&PoliceCar>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let flash_rate = 4.0; // 每秒閃爍 4 次

    for (mut siren, mut visibility) in &mut siren_query {
        siren.flash_timer += dt * flash_rate;

        // 紅藍燈交替閃爍
        let phase = (siren.flash_timer * std::f32::consts::TAU).sin();
        siren.is_on = if siren.is_red {
            phase > 0.0
        } else {
            phase <= 0.0
        };

        *visibility = if siren.is_on {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
