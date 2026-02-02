//! 車上射擊系統
//!
//! 允許駕駛/乘客在車上射擊，限制武器類型和射擊角度

#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::audio::{play_weapon_fire_sound, AudioManager, WeaponSounds};
use crate::core::{CameraSettings, GameState};
use crate::player::Player;
use crate::vehicle::Vehicle;

use super::components::*;
use super::health::*;
use super::visuals::*;
use super::weapon::*;

// ============================================================================
// 常數
// ============================================================================

/// 駕駛左側射擊角度限制（度）
const DRIVER_LEFT_ANGLE_LIMIT: f32 = 90.0;
/// 駕駛右側射擊角度限制（度）
const DRIVER_RIGHT_ANGLE_LIMIT: f32 = 45.0;
/// 乘客左側射擊角度限制（度）
#[allow(dead_code)]
const PASSENGER_LEFT_ANGLE_LIMIT: f32 = 45.0;
/// 乘客右側射擊角度限制（度）
#[allow(dead_code)]
const PASSENGER_RIGHT_ANGLE_LIMIT: f32 = 90.0;

// ============================================================================
// 組件
// ============================================================================

/// 車上射擊狀態
#[derive(Component, Default)]
pub struct VehicleShootingState {
    /// 是否允許射擊
    pub can_shoot: bool,
    /// 當前瞄準角度（相對於車輛前方）
    pub aim_angle: f32,
    /// 射擊冷卻計時器
    pub cooldown_timer: f32,
    /// 座位類型
    pub seat_type: VehicleSeatType,
}

/// 車輛座位類型
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VehicleSeatType {
    #[default]
    Driver,
    FrontPassenger,
    BackLeft,
    BackRight,
}

// ============================================================================
// 系統
// ============================================================================

/// 車上射擊輸入處理
pub fn vehicle_shooting_input_system(
    game_state: Res<GameState>,
    camera_settings: Res<CameraSettings>,
    mut player_query: Query<&mut WeaponInventory, With<Player>>,
    vehicle_query: Query<&Transform, (With<Vehicle>, Without<Player>)>,
    mut combat_state: ResMut<CombatState>,
) {
    // 只有在車上時才處理
    if !game_state.player_in_vehicle {
        return;
    }

    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };

    let Ok(mut inventory) = player_query.single_mut() else {
        return;
    };

    let Ok(vehicle_transform) = vehicle_query.get(vehicle_entity) else {
        return;
    };

    // 檢查當前武器是否可在車上使用
    let Some(weapon) = inventory.current_weapon() else {
        return;
    };

    if !is_vehicle_compatible_weapon(weapon.stats.weapon_type) {
        // 自動切換到可用武器
        if let Some(compatible_index) = find_compatible_weapon_index(&inventory) {
            inventory.current_index = compatible_index;
            info!("自動切換到車上可用武器");
        } else {
            combat_state.can_fire_in_vehicle = false;
            return;
        }
    }

    combat_state.can_fire_in_vehicle = true;

    // 計算相對於車輛的瞄準角度
    let vehicle_forward = vehicle_transform.forward();
    let aim_dir = Vec3::new(camera_settings.yaw.cos(), 0.0, camera_settings.yaw.sin()).normalize();

    let angle = vehicle_forward.dot(aim_dir).acos().to_degrees();
    let cross = vehicle_forward.cross(aim_dir).y;
    let signed_angle = if cross >= 0.0 { angle } else { -angle };

    // 檢查角度是否在允許範圍內（假設駕駛座）
    let in_range =
        (-DRIVER_LEFT_ANGLE_LIMIT..=DRIVER_RIGHT_ANGLE_LIMIT).contains(&signed_angle);
    combat_state.vehicle_aim_valid = in_range;
}

/// 車上射擊執行
pub fn vehicle_shooting_fire_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    game_state: Res<GameState>,
    camera_settings: Res<CameraSettings>,
    rapier_context: ReadRapierContext,
    mut commands: Commands,
    combat_visuals: Option<Res<CombatVisuals>>,
    weapon_sounds: Option<Res<WeaponSounds>>,
    audio_manager: Res<AudioManager>,
    mut player_query: Query<(Entity, &mut WeaponInventory), With<Player>>,
    vehicle_query: Query<(&Transform, &Vehicle), Without<Player>>,
    mut combat_state: ResMut<CombatState>,
    mut damage_events: MessageWriter<DamageEvent>,
    damageable_query: Query<Entity, With<Damageable>>,
) {
    // 只有在車上時才處理
    if !game_state.player_in_vehicle || !combat_state.can_fire_in_vehicle {
        return;
    }

    // 檢查瞄準角度是否有效
    if !combat_state.vehicle_aim_valid {
        return;
    }

    // 射擊輸入：R 鍵
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }

    let Some(visuals) = combat_visuals else {
        return;
    };
    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };

    let Ok((player_entity, mut inventory)) = player_query.single_mut() else {
        return;
    };

    let Ok((vehicle_transform, _vehicle)) = vehicle_query.get(vehicle_entity) else {
        return;
    };

    let Some(weapon) = inventory.current_weapon_mut() else {
        return;
    };

    // 檢查冷卻和彈藥
    if weapon.fire_cooldown > 0.0 {
        return;
    }
    if weapon.current_ammo == 0 && weapon.stats.magazine_size > 0 {
        return;
    }

    // 計算射擊方向（從車窗向外）
    let muzzle_offset = Vec3::new(0.0, 1.2, 0.0); // 車窗高度
    let muzzle_pos = vehicle_transform.translation + muzzle_offset;

    let aim_dir = Vec3::new(
        camera_settings.yaw.cos(),
        -camera_settings.pitch.sin() * 0.3, // 限制垂直角度
        camera_settings.yaw.sin(),
    )
    .normalize();

    // 射線檢測
    let filter = QueryFilter::new()
        .exclude_rigid_body(player_entity)
        .exclude_rigid_body(vehicle_entity);

    let max_range = weapon.stats.range;
    let weapon_type = weapon.stats.weapon_type;

    // 執行射擊
    weapon.fire_cooldown = weapon.stats.fire_rate;
    if weapon.stats.magazine_size > 0 {
        weapon.current_ammo -= 1;
    }

    // 射線檢測目標
    if let Some((hit_entity, hit_dist)) = rapier.cast_ray(
        muzzle_pos,
        aim_dir,
        max_range as bevy_rapier3d::prelude::Real,
        true,
        filter,
    ) {
        let hit_pos = muzzle_pos + aim_dir * hit_dist;

        // 檢查是否命中可受傷目標
        if damageable_query.get(hit_entity).is_ok() {
            damage_events.write(DamageEvent {
                target: hit_entity,
                amount: weapon.stats.damage,
                source: DamageSource::Bullet,
                attacker: Some(player_entity),
                hit_position: Some(hit_pos),
                is_headshot: false, // 車上射擊不計爆頭
            });
            combat_state.last_hit_time = Some(time.elapsed_secs());
        }

        // 生成擊中效果
        spawn_impact_effect(&mut commands, &visuals, hit_pos, -aim_dir);
    }

    // 生成槍口火焰
    let muzzle_world = muzzle_pos + aim_dir * 0.5;
    spawn_muzzle_flash(&mut commands, &visuals, muzzle_world, aim_dir);

    // 播放音效
    if let Some(sounds) = weapon_sounds {
        play_weapon_fire_sound(&mut commands, &sounds, &audio_manager, weapon_type);
    }

    // 生成子彈曳光
    let tracer_end = muzzle_pos + aim_dir * max_range;
    spawn_bullet_tracer(&mut commands, &visuals, muzzle_pos, tracer_end, weapon_type);
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 檢查武器是否可在車上使用
fn is_vehicle_compatible_weapon(weapon_type: WeaponType) -> bool {
    matches!(
        weapon_type,
        WeaponType::Pistol | WeaponType::SMG | WeaponType::Fist
    )
}

/// 找到可在車上使用的武器索引
fn find_compatible_weapon_index(inventory: &WeaponInventory) -> Option<usize> {
    inventory
        .weapons
        .iter()
        .position(|w| is_vehicle_compatible_weapon(w.stats.weapon_type))
}

/// 生成槍口火焰
fn spawn_muzzle_flash(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    position: Vec3,
    direction: Vec3,
) {
    commands.spawn((
        Mesh3d(visuals.muzzle_mesh.clone()),
        MeshMaterial3d(visuals.muzzle_material.clone()),
        Transform::from_translation(position)
            .looking_to(direction, Vec3::Y)
            .with_scale(Vec3::splat(0.3)),
        MuzzleFlash { lifetime: 0.05 },
    ));
}

/// 生成擊中效果
fn spawn_impact_effect(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    position: Vec3,
    normal: Vec3,
) {
    commands.spawn((
        Mesh3d(visuals.impact_mesh.clone()),
        MeshMaterial3d(visuals.impact_material.clone()),
        Transform::from_translation(position)
            .looking_to(normal, Vec3::Y)
            .with_scale(Vec3::splat(0.2)),
        ImpactEffect {
            lifetime: 0.1,
            max_lifetime: 0.1,
        },
    ));
}

/// 生成子彈曳光
fn spawn_bullet_tracer(
    commands: &mut Commands,
    visuals: &CombatVisuals,
    start: Vec3,
    end: Vec3,
    weapon_type: WeaponType,
) {
    let tracer_style = weapon_type.tracer_style();
    let Some(tracer_config) = visuals.get_tracer(tracer_style) else {
        return;
    };

    let mid = (start + end) / 2.0;
    let length = start.distance(end);
    let direction = (end - start).normalize();

    commands.spawn((
        Mesh3d(tracer_config.mesh.clone()),
        MeshMaterial3d(tracer_config.material.clone()),
        Transform::from_translation(mid)
            .looking_to(direction, Vec3::Y)
            .with_scale(Vec3::new(
                tracer_config.thickness * 0.02,
                tracer_config.thickness * 0.02,
                length,
            )),
        BulletTracer {
            lifetime: tracer_config.lifetime,
            start_pos: start,
            end_pos: end,
        },
    ));
}
