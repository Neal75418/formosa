//! 載具視覺效果（漂移煙霧、氮氣火焰、輪胎痕跡）

#[allow(clippy::wildcard_imports)]
use super::*;
use bevy::prelude::*;
use rand::Rng;

// ============================================================================
// 車輛視覺效果系統（GTA 5 風格）
// ============================================================================

/// 判斷車輛是否應該生成漂移煙霧
fn should_spawn_drift_smoke(vehicle: &Vehicle, drift: &VehicleDrift, input: &VehicleInput) -> bool {
    (drift.is_drifting && drift.drift_angle.abs() > 0.2)
        || (drift.is_handbraking && vehicle.current_speed > 10.0)
        || (input.wheel_spin > 0.5) // 輪胎打滑時也有煙
}

/// 取得車輛類型對應的後輪偏移量
fn get_rear_wheel_offset(vehicle_type: VehicleType) -> Vec3 {
    match vehicle_type {
        VehicleType::Scooter => Vec3::new(0.0, 0.0, 0.8),
        VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.0, 1.5),
        VehicleType::Bus => Vec3::new(0.0, 0.0, 3.0),
    }
}

/// 取得車輛類型對應的輪子側向偏移量
fn get_wheel_lateral_offset(vehicle_type: VehicleType, side: f32) -> f32 {
    match vehicle_type {
        VehicleType::Scooter => 0.0, // 機車只有中間
        _ => 0.8 * side,
    }
}

/// 漂移煙霧生成系統
/// 當車輛漂移或急煞時，在後輪位置生成煙霧粒子
pub fn drift_smoke_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    effect_visuals: Option<Res<VehicleEffectVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle, &VehicleDrift, &VehicleInput), Without<NpcVehicle>>,
) {
    let Some(visuals) = effect_visuals else {
        return;
    };
    let current_time = time.elapsed_secs();

    // 檢查生成間隔
    if current_time - effect_tracker.last_smoke_spawn < effect_tracker.smoke_spawn_interval {
        return;
    }

    for (transform, vehicle, drift, input) in vehicle_query.iter() {
        if !should_spawn_drift_smoke(vehicle, drift, input)
            || effect_tracker.smoke_count >= effect_tracker.max_smoke_count
        {
            continue;
        }

        let rear_offset = get_rear_wheel_offset(vehicle.vehicle_type);
        let world_pos = transform.translation + transform.rotation * rear_offset;
        let wheel_height = 0.2;

        let mut rng = rand::rng();
        for side in [-1.0, 1.0] {
            let wheel_offset = get_wheel_lateral_offset(vehicle.vehicle_type, side);
            let spawn_pos =
                world_pos + transform.rotation * Vec3::new(wheel_offset, wheel_height, 0.0);

            let spread = Vec3::new(
                rng.random_range(-0.5..0.5),
                rng.random_range(0.3..0.8),
                rng.random_range(-0.5..0.5),
            );
            let base_velocity =
                -transform.forward().as_vec3() * (vehicle.current_speed * 0.1).max(1.0);

            commands.spawn((
                Mesh3d(visuals.smoke_mesh.clone()),
                MeshMaterial3d(visuals.smoke_material.clone()),
                Transform::from_translation(spawn_pos).with_scale(Vec3::splat(0.3)),
                DriftSmoke::new(base_velocity + spread, rng.random_range(0.5..1.0)),
            ));

            effect_tracker.smoke_count += 1;

            // 機車只生成一個煙霧
            if vehicle.vehicle_type == VehicleType::Scooter {
                break;
            }
        }

        effect_tracker.last_smoke_spawn = current_time;
    }
}

/// 漂移煙霧更新系統
/// 處理煙霧粒子的移動、縮放、淡出和刪除
pub fn drift_smoke_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    mut smoke_query: Query<(Entity, &mut DriftSmoke, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (entity, mut smoke, mut transform) in &mut smoke_query {
        // 更新生命時間
        smoke.lifetime += dt;

        // 檢查是否過期
        if smoke.lifetime >= smoke.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                effect_tracker.smoke_count = effect_tracker.smoke_count.saturating_sub(1);
            }
            continue;
        }

        // 減速（空氣阻力）
        smoke.velocity *= 1.0 - dt * 2.0;

        // 輕微上飄（熱氣）
        smoke.velocity.y += dt * 0.5;

        // 更新位置
        transform.translation += smoke.velocity * dt;

        // 更新縮放（擴散變大）
        let scale = smoke.scale();
        transform.scale = Vec3::splat(scale);
    }
}

// ============================================================================
// 氮氣火焰效果系統
// ============================================================================

/// 氮氣火焰生成系統
/// 當車輛使用氮氣加速時，在排氣管後方產生火焰效果
pub fn nitro_flame_spawn_system(
    mut commands: Commands,
    effect_visuals: Option<Res<VehicleEffectVisuals>>,
    vehicle_query: Query<(&Transform, &VehicleModifications, &NitroBoost), Without<NpcVehicle>>,
) {
    let Some(visuals) = effect_visuals else {
        return;
    };

    for (transform, mods, nitro) in vehicle_query.iter() {
        // 只有在使用氮氣且有充能時生成火焰
        if !nitro.is_active || mods.nitro_charge <= 0.0 {
            continue;
        }

        // 排氣管位置（車尾）
        let exhaust_offset = transform.back() * 2.5 + Vec3::new(0.0, 0.3, 0.0);
        let exhaust_pos = transform.translation + exhaust_offset;

        // 生成多個火焰粒子
        let mut rng = rand::rng();
        for _ in 0..3 {
            // 隨機偏移
            let offset = Vec3::new(
                (rng.random::<f32>() - 0.5) * 0.3,
                (rng.random::<f32>() - 0.5) * 0.2,
                0.0,
            );

            // 火焰往後噴射
            let velocity = transform.back() * (3.0 + rng.random::<f32>() * 2.0)
                + Vec3::new(
                    (rng.random::<f32>() - 0.5) * 0.5,
                    rng.random::<f32>() * 0.3,
                    (rng.random::<f32>() - 0.5) * 0.5,
                );

            commands.spawn((
                Mesh3d(visuals.nitro_flame_mesh.clone()),
                MeshMaterial3d(visuals.nitro_flame_material.clone()),
                Transform::from_translation(exhaust_pos + offset)
                    .with_scale(Vec3::new(0.2, 0.2, 0.4)), // 拉長形狀
                NitroFlame::new(velocity),
            ));
        }
    }
}

/// 氮氣火焰更新系統
/// 處理火焰粒子的移動、縮放和顏色變化
pub fn nitro_flame_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut flame_query: Query<(Entity, &mut NitroFlame, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (entity, mut flame, mut transform) in &mut flame_query {
        // 更新生命時間
        flame.lifetime += dt;

        // 檢查是否過期
        if flame.lifetime >= flame.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // 更新位置
        transform.translation += flame.velocity * dt;

        // 更新縮放（逐漸消散）
        let scale = flame.scale();
        transform.scale = Vec3::new(scale, scale, scale * 2.0); // 保持拉長形狀
    }
}

/// 判斷車輛是否應該生成輪胎痕跡
fn should_spawn_tire_track(vehicle: &Vehicle, drift: &VehicleDrift) -> bool {
    (drift.is_drifting && drift.drift_angle.abs() > 0.15)
        || (drift.is_handbraking && vehicle.current_speed > 8.0)
}

/// 取得車輛類型對應的輪胎痕跡後輪偏移量
fn get_track_rear_offset(vehicle_type: VehicleType) -> Vec3 {
    match vehicle_type {
        VehicleType::Scooter => Vec3::new(0.0, 0.0, 0.7),
        VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.0, 1.2),
        VehicleType::Bus => Vec3::new(0.0, 0.0, 2.5),
    }
}

/// 輪胎痕跡生成系統
/// 當車輛漂移或急煞時，在地面留下輪胎痕跡
pub fn tire_track_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    effect_visuals: Option<Res<VehicleEffectVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle, &VehicleDrift), Without<NpcVehicle>>,
) {
    let Some(visuals) = effect_visuals else {
        return;
    };
    let current_time = time.elapsed_secs();

    if current_time - effect_tracker.last_track_spawn < effect_tracker.track_spawn_interval {
        return;
    }

    for (transform, vehicle, drift) in vehicle_query.iter() {
        if !should_spawn_tire_track(vehicle, drift)
            || effect_tracker.track_count >= effect_tracker.max_track_count
        {
            continue;
        }

        let rear_offset = get_track_rear_offset(vehicle.vehicle_type);
        let track_width = 0.2 + drift.drift_angle.abs() * 0.3;

        for side in [-1.0, 1.0] {
            let wheel_offset = get_wheel_lateral_offset(vehicle.vehicle_type, side);
            let track_pos = transform.translation
                + transform.rotation * (rear_offset + Vec3::new(wheel_offset, 0.0, 0.0));
            let ground_pos = Vec3::new(track_pos.x, 0.02, track_pos.z);

            commands.spawn((
                Mesh3d(visuals.tire_track_mesh.clone()),
                MeshMaterial3d(visuals.tire_track_material.clone()),
                Transform::from_translation(ground_pos)
                    .with_rotation(transform.rotation)
                    .with_scale(Vec3::new(track_width, 1.0, 0.8)),
                TireTrack::default(),
            ));

            effect_tracker.track_count += 1;

            if vehicle.vehicle_type == VehicleType::Scooter {
                break;
            }
        }

        effect_tracker.last_track_spawn = current_time;
    }
}

/// 輪胎痕跡更新系統
/// 處理痕跡的淡出和刪除
pub fn tire_track_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_tracker: ResMut<VehicleEffectTracker>,
    mut track_query: Query<(Entity, &mut TireTrack)>,
) {
    let dt = time.delta_secs();

    for (entity, mut track) in &mut track_query {
        // 更新生命時間
        track.lifetime += dt;

        // 檢查是否過期
        if track.lifetime >= track.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                effect_tracker.track_count = effect_tracker.track_count.saturating_sub(1);
            }
        }
    }
}

/// 初始化車輛視覺效果資源
pub fn setup_vehicle_effects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(VehicleEffectVisuals::new(&mut meshes, &mut materials));
    commands.insert_resource(VehicleEffectTracker::new());
    info!("🚗 車輛視覺效果系統已初始化");
}
