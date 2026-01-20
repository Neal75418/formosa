//! 通緝系統函數實作

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::player::Player;
use crate::ai::AiMovement;
use crate::combat::{Health, DamageEvent, DamageSource, HitReaction, CombatVisuals, spawn_bullet_tracer, spawn_muzzle_flash, TracerStyle};
use crate::core::PoliceSpatialHash;

use super::components::*;
use super::events::*;

/// 設置警察視覺資源
pub fn setup_police_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 創建警察模型的基礎 Mesh
    let body_mesh = meshes.add(Cuboid::new(0.4, 0.6, 0.25));
    let head_mesh = meshes.add(Sphere::new(0.15));
    let arm_mesh = meshes.add(Cuboid::new(0.1, 0.4, 0.1));
    let leg_mesh = meshes.add(Cuboid::new(0.12, 0.45, 0.12));

    // 警察制服材質（深藍色）
    let uniform_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.15, 0.3),
        ..default()
    });

    // 皮膚材質
    let skin_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.87, 0.72, 0.53),
        ..default()
    });

    // 警徽材質（金色）
    let badge_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.65, 0.13),
        metallic: 0.9,
        ..default()
    });

    commands.insert_resource(PoliceVisuals {
        body_mesh,
        head_mesh,
        arm_mesh,
        leg_mesh,
        uniform_material,
        skin_material,
        badge_material,
    });
}

// ============================================================================
// 空間哈希系統
// ============================================================================

/// 更新警察空間哈希（每幀執行，在視野檢測前）
///
/// 將場景中所有警察位置插入空間哈希網格，
/// 供玩家視野檢測和無線電通訊系統使用，將 O(n²) 降為 O(n)。
pub fn update_police_spatial_hash_system(
    mut police_hash: ResMut<PoliceSpatialHash>,
    police_query: Query<(Entity, &Transform), With<PoliceOfficer>>,
) {
    // 清空舊資料
    police_hash.clear();

    // 插入所有警察（批量插入效能更好）
    police_hash.insert_batch(
        police_query.iter().map(|(entity, transform)| {
            (entity, transform.translation)
        })
    );
}

/// 處理犯罪事件，更新通緝等級
pub fn process_crime_events(
    mut crime_events: MessageReader<CrimeEvent>,
    mut wanted: ResMut<WantedLevel>,
    mut level_changed: MessageWriter<WantedLevelChanged>,
    time: Res<Time>,
) {
    for event in crime_events.read() {
        let old_stars = wanted.stars;
        let heat_increase = event.heat_value();

        wanted.add_heat(heat_increase);
        wanted.last_crime_time = time.elapsed_secs();
        wanted.cooldown_timer = 0.0; // 重置消退計時器

        // 設置搜索區域（含超時）
        wanted.search_center = Some(event.position());
        wanted.search_radius = 30.0;
        wanted.search_timer = 45.0;  // 45 秒後若未找到玩家則清除搜索區域

        // 如果星級變化，發送事件
        if wanted.stars != old_stars {
            level_changed.write(WantedLevelChanged::new(old_stars, wanted.stars));
            info!(
                "通緝等級變化: {} -> {} (熱度: {:.1})",
                old_stars, wanted.stars, wanted.heat
            );
        }
    }
}

/// 處理目擊者報警事件
/// 當行人完成報警電話時增加熱度（但不重複計算原始犯罪）
pub fn process_witness_reports(
    mut witness_events: MessageReader<WitnessReport>,
    mut wanted: ResMut<WantedLevel>,
    mut level_changed: MessageWriter<WantedLevelChanged>,
) {
    for report in witness_events.read() {
        let old_stars = wanted.stars;

        // 報警完成增加固定熱度
        wanted.add_heat(WitnessReport::HEAT_VALUE);

        // 更新搜索區域到報警位置（重置超時）
        wanted.search_center = Some(report.position);
        wanted.search_timer = 45.0;

        // 如果星級變化，發送事件
        if wanted.stars != old_stars {
            level_changed.write(WantedLevelChanged::new(old_stars, wanted.stars));
            info!(
                "目擊者報警: {} - 通緝等級 {} -> {} (熱度: {:.1})",
                report.crime_description, old_stars, wanted.stars, wanted.heat
            );
        }
    }
}

// ============================================================================
// 通緝消退輔助函數
// ============================================================================

/// 視野檢測距離
const VISION_RANGE: f32 = 40.0;

/// 檢查警察是否能看到玩家
fn check_police_vision(
    player_pos: Vec3,
    police_hash: &PoliceSpatialHash,
    police_query: &Query<(&Transform, &PoliceOfficer)>,
    rapier: &RapierContext,
) -> bool {
    for (police_entity, police_pos, _) in police_hash.query_radius(player_pos, VISION_RANGE) {
        let Ok((police_transform, officer)) = police_query.get(police_entity) else { continue; };
        if officer.state == PoliceState::Patrolling { continue; }

        let to_player = player_pos - police_transform.translation;
        let distance = to_player.length();
        let ray_origin = police_pos + Vec3::Y * 1.5;

        if let Some((_, toi)) = rapier.cast_ray(
            ray_origin,
            to_player.normalize(),
            distance,
            true,
            QueryFilter::default(),
        ) {
            if toi >= distance - 1.0 {
                return true;
            }
        }
    }
    false
}

/// 處理通緝消退邏輯
fn process_cooldown(
    wanted: &mut WantedLevel,
    level_changed: &mut MessageWriter<WantedLevelChanged>,
    dt: f32,
) {
    wanted.cooldown_timer += dt;

    if wanted.cooldown_timer >= wanted.cooldown_duration() {
        let old_stars = wanted.stars;
        wanted.reduce_heat(20.0);
        wanted.cooldown_timer = 0.0;

        if wanted.stars != old_stars {
            level_changed.write(WantedLevelChanged::new(old_stars, wanted.stars));
            info!("通緝等級消退: {} -> {} (熱度: {:.1})", old_stars, wanted.stars, wanted.heat);
        }

        if wanted.stars == 0 {
            wanted.search_center = None;
            wanted.player_last_seen_pos = None;
        }
    }
}

/// 通緝等級消退系統（空間哈希優化版）
pub fn wanted_cooldown_system(
    mut wanted: ResMut<WantedLevel>,
    mut level_changed: MessageWriter<WantedLevelChanged>,
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    police_hash: Res<PoliceSpatialHash>,
    police_query: Query<(&Transform, &PoliceOfficer)>,
    rapier_context: ReadRapierContext,
) {
    if wanted.stars == 0 { return; }

    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;

    let player_visible = rapier_context
        .single()
        .map(|rapier| check_police_vision(player_pos, &police_hash, &police_query, &rapier))
        .unwrap_or(false);

    wanted.player_visible = player_visible;

    if player_visible {
        wanted.player_last_seen_pos = Some(player_pos);
        wanted.cooldown_timer = 0.0;
        // 玩家被看到時更新搜索區域
        wanted.search_center = Some(player_pos);
        wanted.search_timer = 45.0;
    } else {
        process_cooldown(&mut wanted, &mut level_changed, time.delta_secs());

        // 搜索區域超時邏輯：長時間找不到玩家則清除搜索區域
        if wanted.search_center.is_some() {
            wanted.search_timer -= time.delta_secs();
            if wanted.search_timer <= 0.0 {
                wanted.search_center = None;
                wanted.search_timer = 0.0;
                info!("搜索區域超時，警察失去玩家蹤跡");
            }
        }
    }
}

/// 警察生成系統
pub fn spawn_police_system(
    mut commands: Commands,
    wanted: Res<WantedLevel>,
    mut config: ResMut<PoliceConfig>,
    police_query: Query<Entity, With<PoliceOfficer>>,
    player_query: Query<&Transform, With<Player>>,
    visuals: Option<Res<PoliceVisuals>>,
    time: Res<Time>,
) {
    // 沒有通緝不生成警察
    if wanted.stars == 0 {
        return;
    }

    let Some(visuals) = visuals else {
        return;
    };

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let current_count = police_query.iter().count() as u32;
    let target_count = wanted.target_police_count();

    // 檢查是否需要生成更多警察
    if current_count >= target_count {
        return;
    }

    // 檢查生成間隔
    let elapsed = time.elapsed_secs();
    if elapsed - config.last_spawn_time < config.spawn_interval {
        return;
    }

    config.last_spawn_time = elapsed;

    // 計算生成位置（玩家周圍隨機位置）
    let player_pos = player_transform.translation;
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let distance = config.spawn_distance_min
        + rand::random::<f32>() * (config.spawn_distance_max - config.spawn_distance_min);

    let spawn_pos = Vec3::new(
        player_pos.x + angle.cos() * distance,
        0.0,
        player_pos.z + angle.sin() * distance,
    );

    // 生成警察
    spawn_police_officer(&mut commands, spawn_pos, &visuals, wanted.stars);

    info!(
        "生成警察 at ({:.1}, {:.1}) - 當前: {}/{}",
        spawn_pos.x,
        spawn_pos.z,
        current_count + 1,
        target_count
    );
}

/// 生成單個警察 NPC
fn spawn_police_officer(
    commands: &mut Commands,
    position: Vec3,
    visuals: &PoliceVisuals,
    wanted_stars: u8,
) {
    // 決定警察類型
    let officer_type = if wanted_stars >= 3 {
        PoliceType::Swat
    } else {
        PoliceType::Patrol
    };

    let initial_state = if wanted_stars > 0 {
        PoliceState::Alerted
    } else {
        PoliceState::Patrolling
    };

    // 創建警察實體
    let police_entity = commands
        .spawn((
            Name::new("PoliceOfficer"),
            Transform::from_translation(position + Vec3::Y * 0.9),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            PoliceOfficer {
                state: initial_state,
                officer_type,
                target_player: wanted_stars > 0,
                ..default()
            },
            Health {
                current: 100.0,
                max: 100.0,
                ..default()
            },
            HitReaction::default(),  // 受傷反應
            AiMovement {
                walk_speed: 3.0,
                run_speed: if officer_type == PoliceType::Swat { 7.0 } else { 5.5 },
                ..default()
            },
            // 物理組件
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(0.4, 0.25),
            KinematicCharacterController {
                offset: CharacterLength::Absolute(0.1),
                ..default()
            },
        ))
        .id();

    // 添加視覺模型（身體各部位）
    // 使用結構化資料定義身體部位，減少重複代碼
    let body_parts: &[(&Handle<Mesh>, &Handle<StandardMaterial>, Vec3)] = &[
        (&visuals.body_mesh, &visuals.uniform_material, Vec3::new(0.0, 0.0, 0.0)),   // 軀幹
        (&visuals.head_mesh, &visuals.skin_material, Vec3::new(0.0, 0.45, 0.0)),     // 頭部
        (&visuals.leg_mesh, &visuals.uniform_material, Vec3::new(-0.1, -0.5, 0.0)),  // 左腿
        (&visuals.leg_mesh, &visuals.uniform_material, Vec3::new(0.1, -0.5, 0.0)),   // 右腿
        (&visuals.arm_mesh, &visuals.uniform_material, Vec3::new(-0.3, 0.1, 0.0)),   // 左臂
        (&visuals.arm_mesh, &visuals.uniform_material, Vec3::new(0.3, 0.1, 0.0)),    // 右臂
    ];

    commands.entity(police_entity).with_children(|parent| {
        for (mesh, material, offset) in body_parts {
            parent.spawn((
                Mesh3d((*mesh).clone()),
                MeshMaterial3d((*material).clone()),
                Transform::from_translation(*offset),
            ));
        }
    });
}

// ============================================================================
// 警察 AI 輔助函數
// ============================================================================

/// 計算移動速度向量
fn calc_movement_velocity(direction: Vec3, speed: f32, dt: f32) -> Vec3 {
    Vec3::new(direction.x * speed * dt, -9.81 * dt, direction.z * speed * dt)
}

/// 計算面向目標的旋轉
fn calc_facing_rotation(direction: Vec3) -> Quat {
    Quat::from_rotation_y((-direction.x).atan2(-direction.z))
}

/// 處理巡邏狀態
fn handle_patrolling_state(officer: &mut PoliceOfficer, wanted_stars: u8) {
    if wanted_stars > 0 {
        officer.state = PoliceState::Alerted;
        officer.target_player = true;
    }
}

/// 處理警覺狀態
fn handle_alerted_state(
    officer: &mut PoliceOfficer,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    police_pos: Vec3,
    distance: f32,
    movement: &AiMovement,
    wanted: &WantedLevel,
    config: &PoliceConfig,
    dt: f32,
) {
    let target_pos = officer.radio_alert_position.or(wanted.search_center);

    if let Some(search_center) = target_pos {
        let to_search = search_center - police_pos;
        let search_dist = to_search.length();

        if search_dist > 2.0 {
            let move_dir = to_search.normalize();
            let speed = if officer.radio_alerted { movement.run_speed } else { movement.walk_speed * 1.5 };
            controller.translation = Some(calc_movement_velocity(move_dir, speed, dt));
            transform.rotation = transform.rotation.slerp(calc_facing_rotation(move_dir), 5.0 * dt);
        } else {
            officer.radio_alerted = false;
            officer.radio_alert_position = None;
        }
    }

    if distance < config.vision_range && wanted.player_visible {
        officer.state = PoliceState::Pursuing;
        officer.radio_alerted = false;
        officer.radio_alert_position = None;
    }
}

/// 處理追捕狀態
fn handle_pursuing_state(
    officer: &mut PoliceOfficer,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    direction: Vec3,
    distance: f32,
    movement: &AiMovement,
    player_visible: bool,
    attack_range: f32,
    dt: f32,
) {
    if distance > attack_range {
        controller.translation = Some(calc_movement_velocity(direction, movement.run_speed, dt));
        transform.rotation = transform.rotation.slerp(calc_facing_rotation(direction), 8.0 * dt);
    } else {
        officer.state = PoliceState::Engaging;
    }

    if !player_visible {
        officer.state = PoliceState::Searching;
        officer.search_timer = 0.0;
    }
}

/// 處理搜索狀態
fn handle_searching_state(
    officer: &mut PoliceOfficer,
    controller: &mut KinematicCharacterController,
    police_pos: Vec3,
    movement: &AiMovement,
    wanted: &WantedLevel,
    dt: f32,
) {
    officer.search_timer += dt;

    if let Some(last_pos) = wanted.player_last_seen_pos {
        let to_last = last_pos - police_pos;
        if to_last.length() > 2.0 {
            controller.translation = Some(calc_movement_velocity(to_last.normalize(), movement.walk_speed, dt));
        }
    }

    if wanted.player_visible {
        officer.state = PoliceState::Pursuing;
    }

    if officer.search_timer > 30.0 && wanted.stars == 0 {
        officer.state = PoliceState::Returning;
    }
}

/// 處理交戰狀態
fn handle_engaging_state(
    officer: &mut PoliceOfficer,
    transform: &mut Transform,
    controller: &mut KinematicCharacterController,
    direction: Vec3,
    distance: f32,
    movement: &AiMovement,
    attack_range: f32,
    elapsed_secs: f32,
    dt: f32,
) {
    transform.rotation = transform.rotation.slerp(calc_facing_rotation(direction), 10.0 * dt);

    if distance > attack_range * 1.5 {
        officer.state = PoliceState::Pursuing;
    }

    let strafe_dir = Vec3::new(-direction.z, 0.0, direction.x);
    let strafe_speed = (elapsed_secs * 2.0).sin() * 0.3 * movement.walk_speed * dt;
    controller.translation = Some(Vec3::new(strafe_dir.x * strafe_speed, -9.81 * dt, strafe_dir.z * strafe_speed));
}

/// 處理返回狀態
fn handle_returning_state(officer: &mut PoliceOfficer, wanted_stars: u8) {
    officer.state = if wanted_stars > 0 { PoliceState::Alerted } else { PoliceState::Patrolling };
}

/// 警察 AI 系統
pub fn police_ai_system(
    mut police_query: Query<(
        &mut Transform,
        &mut PoliceOfficer,
        &AiMovement,
        &mut KinematicCharacterController,
    )>,
    player_query: Query<&Transform, (With<Player>, Without<PoliceOfficer>)>,
    wanted: Res<WantedLevel>,
    time: Res<Time>,
    config: Res<PoliceConfig>,
) {
    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for (mut transform, mut officer, movement, mut controller) in &mut police_query {
        let police_pos = transform.translation;
        let to_player = player_pos - police_pos;
        let distance = to_player.length();
        let direction = if distance > 0.1 { to_player.normalize() } else { Vec3::ZERO };

        match officer.state {
            PoliceState::Patrolling => {
                handle_patrolling_state(&mut officer, wanted.stars);
            }
            PoliceState::Alerted => {
                handle_alerted_state(&mut officer, &mut transform, &mut controller, police_pos, distance, movement, &wanted, &config, dt);
            }
            PoliceState::Pursuing => {
                handle_pursuing_state(&mut officer, &mut transform, &mut controller, direction, distance, movement, wanted.player_visible, config.attack_range, dt);
            }
            PoliceState::Searching => {
                handle_searching_state(&mut officer, &mut controller, police_pos, movement, &wanted, dt);
            }
            PoliceState::Engaging => {
                handle_engaging_state(&mut officer, &mut transform, &mut controller, direction, distance, movement, config.attack_range, elapsed, dt);
            }
            PoliceState::Returning => {
                handle_returning_state(&mut officer, wanted.stars);
            }
        }
    }
}

// ============================================================================
// 警察戰鬥輔助函數
// ============================================================================

/// 檢查射線視線是否暢通
fn check_line_of_sight(
    rapier: &RapierContext,
    ray_origin: Vec3,
    ray_direction: Vec3,
    distance: f32,
    exclude_entity: Entity,
    player_entity: Entity,
) -> bool {
    let filter = QueryFilter::default().exclude_rigid_body(exclude_entity);

    match rapier.cast_ray(ray_origin, ray_direction, distance, true, filter) {
        Some((hit_entity, toi)) => hit_entity == player_entity || toi >= distance - 1.0,
        None => true,
    }
}

/// 計算命中率（距離越遠越低）
fn calc_hit_chance(distance: f32, config: &PoliceConfig) -> f32 {
    let distance_penalty = (distance / config.attack_range) * config.distance_hit_penalty;
    (config.base_hit_chance - distance_penalty).max(0.1)
}

/// 計算彈道終點位置
fn calc_tracer_end(player_pos: Vec3, is_hit: bool) -> Vec3 {
    let target_height = player_pos + Vec3::Y * 1.0;

    if is_hit {
        target_height
    } else {
        let miss_offset = Vec3::new(
            rand::random::<f32>() * 2.0 - 1.0,
            rand::random::<f32>() * 1.5 - 0.5,
            rand::random::<f32>() * 2.0 - 1.0,
        );
        target_height + miss_offset
    }
}

/// 警察戰鬥系統
pub fn police_combat_system(
    mut commands: Commands,
    mut police_query: Query<(Entity, &Transform, &mut PoliceOfficer)>,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<PoliceOfficer>)>,
    mut damage_events: MessageWriter<DamageEvent>,
    time: Res<Time>,
    config: Res<PoliceConfig>,
    rapier_context: ReadRapierContext,
    combat_visuals: Option<Res<CombatVisuals>>,
) {
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    for (police_entity, transform, mut officer) in &mut police_query {
        // 只有在戰鬥狀態才攻擊
        if officer.state != PoliceState::Engaging {
            continue;
        }

        // 更新攻擊冷卻
        officer.attack_cooldown -= time.delta_secs();
        if officer.attack_cooldown > 0.0 {
            continue;
        }

        let police_pos = transform.translation;
        let to_player = player_pos - police_pos;
        let distance = to_player.length();

        // 檢查是否在攻擊範圍內
        if distance > config.attack_range {
            continue;
        }

        // 射線檢測：確保警察和玩家之間沒有障礙物
        let ray_origin = police_pos + Vec3::Y * 1.5;
        let ray_direction = to_player.normalize();

        if !check_line_of_sight(&rapier, ray_origin, ray_direction, distance, police_entity, player_entity) {
            continue;
        }

        // 計算命中率和彈道
        let hit_chance = calc_hit_chance(distance, &config);
        let is_hit = rand::random::<f32>() < hit_chance;
        let muzzle_pos = police_pos + Vec3::Y * 1.2 + ray_direction * 0.5;
        let tracer_end = calc_tracer_end(player_pos, is_hit);

        // 生成視覺效果
        if let Some(ref visuals) = combat_visuals {
            spawn_muzzle_flash(&mut commands, visuals, muzzle_pos);
            spawn_bullet_tracer(&mut commands, visuals, muzzle_pos, tracer_end, TracerStyle::Pistol);
        }

        if is_hit {
            damage_events.write(DamageEvent {
                target: player_entity,
                amount: config.damage,
                source: DamageSource::Bullet,
                attacker: Some(police_entity),
                hit_position: Some(player_pos),
                is_headshot: false,
            });
        }

        // 設置攻擊冷卻
        officer.attack_cooldown = config.attack_cooldown;
    }
}

/// 警察消失系統
pub fn despawn_police_system(
    mut commands: Commands,
    police_query: Query<(Entity, &Transform), With<PoliceOfficer>>,
    player_query: Query<&Transform, (With<Player>, Without<PoliceOfficer>)>,
    wanted: Res<WantedLevel>,
    config: Res<PoliceConfig>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (entity, transform) in &police_query {
        let distance = (transform.translation - player_pos).length();

        // 如果通緝消退且距離太遠，消失
        let should_despawn = (wanted.stars == 0 && distance > config.despawn_distance)
            || distance > config.despawn_distance * 2.0;

        if should_despawn {
            // Bevy 0.17: despawn() 會自動移除子實體（視覺模型）
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// 無線電呼叫系統
// ============================================================================

/// 無線電呼叫範圍（公尺）- 降低以避免玩家被立即包圍
const RADIO_CALL_RANGE: f32 = 45.0;
/// 無線電呼叫冷卻時間（秒）
const RADIO_CALL_COOLDOWN: f32 = 5.0;

/// 檢查警察是否可以發送無線電
fn can_send_radio(officer: &PoliceOfficer, player_visible: bool) -> bool {
    (officer.state == PoliceState::Pursuing || officer.state == PoliceState::Engaging)
        && officer.radio_cooldown <= 0.0
        && player_visible
}

/// 檢查警察是否可以接收無線電
fn can_receive_radio(state: PoliceState) -> bool {
    matches!(state, PoliceState::Patrolling | PoliceState::Alerted | PoliceState::Searching)
}

/// 收集無線電發送者
fn collect_radio_senders(
    police_query: &mut Query<(Entity, &Transform, &mut PoliceOfficer)>,
    player_visible: bool,
    dt: f32,
) -> Vec<(Entity, Vec3)> {
    let mut senders = Vec::new();

    for (entity, transform, mut officer) in police_query.iter_mut() {
        if officer.radio_cooldown > 0.0 {
            officer.radio_cooldown -= dt;
        }

        if can_send_radio(&officer, player_visible) {
            senders.push((entity, transform.translation));
            officer.radio_cooldown = RADIO_CALL_COOLDOWN;
            debug!("🔊 警察在 ({:.1}, {:.1}) 發送無線電呼叫", transform.translation.x, transform.translation.z);
        }
    }
    senders
}

/// 收集附近的接收者
fn collect_receivers(
    senders: &[(Entity, Vec3)],
    police_hash: &PoliceSpatialHash,
) -> Vec<Entity> {
    let mut receivers = Vec::new();

    for (sender_entity, sender_pos) in senders {
        for (receiver_entity, _, _) in police_hash.query_radius(*sender_pos, RADIO_CALL_RANGE) {
            if receiver_entity != *sender_entity && !receivers.contains(&receiver_entity) {
                receivers.push(receiver_entity);
            }
        }
    }
    receivers
}

/// 通知接收者
fn notify_receivers(
    police_query: &mut Query<(Entity, &Transform, &mut PoliceOfficer)>,
    receivers: Vec<Entity>,
    player_pos: Vec3,
) {
    for receiver_entity in receivers {
        if let Ok((_, _, mut officer)) = police_query.get_mut(receiver_entity) {
            if can_receive_radio(officer.state) {
                officer.radio_alerted = true;
                officer.radio_alert_position = Some(player_pos);

                if officer.state == PoliceState::Patrolling {
                    officer.state = PoliceState::Alerted;
                    officer.target_player = true;
                }
            }
        }
    }
}

/// 警察無線電呼叫系統（空間哈希優化版）
pub fn police_radio_call_system(
    mut police_query: Query<(Entity, &Transform, &mut PoliceOfficer)>,
    police_hash: Res<PoliceSpatialHash>,
    player_query: Query<&Transform, (With<Player>, Without<PoliceOfficer>)>,
    wanted: Res<WantedLevel>,
    time: Res<Time>,
) {
    if wanted.stars == 0 { return; }

    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;

    let senders = collect_radio_senders(&mut police_query, wanted.player_visible, time.delta_secs());
    if senders.is_empty() { return; }

    let receivers = collect_receivers(&senders, &police_hash);
    notify_receivers(&mut police_query, receivers, player_pos);
}

// ============================================================================
// 通緝 HUD 輔助函數
// ============================================================================

/// 創建通緝 HUD 及星星
fn create_wanted_hud(commands: &mut Commands, wanted_stars: u8) {
    let hud_entity = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(5.0),
                ..default()
            },
            WantedHud,
        ))
        .id();

    for i in 0..5u8 {
        let initial_color = if i < wanted_stars {
            Color::srgb(1.0, 0.8, 0.0)
        } else {
            Color::srgba(0.3, 0.3, 0.3, 0.5)
        };

        let star_entity = commands
            .spawn((
                Node {
                    width: Val::Px(24.0),
                    height: Val::Px(24.0),
                    ..default()
                },
                BackgroundColor(initial_color),
                WantedStar { index: i },
            ))
            .id();

        commands.entity(hud_entity).add_child(star_entity);
    }
}

/// 計算星星顯示顏色
fn calc_star_color(star_index: u8, wanted: &WantedLevel) -> Color {
    if star_index < wanted.stars {
        let pulse = (wanted.cooldown_timer * 3.0).sin() * 0.2 + 0.8;
        if wanted.player_visible {
            Color::srgb(1.0, pulse * 0.3, 0.0) // 被追捕時紅色
        } else {
            Color::srgb(1.0, pulse, 0.0) // 消退中黃色
        }
    } else {
        Color::srgba(0.3, 0.3, 0.3, 0.5) // 未啟用灰色
    }
}

/// 更新通緝等級 HUD
pub fn update_wanted_hud(
    mut commands: Commands,
    wanted: Res<WantedLevel>,
    hud_query: Query<Entity, With<WantedHud>>,
    star_query: Query<(Entity, &WantedStar)>,
) {
    // 創建 HUD（如果不存在）
    if hud_query.is_empty() && wanted.stars > 0 {
        create_wanted_hud(&mut commands, wanted.stars);
    }

    // 更新星星顏色
    for (entity, star) in &star_query {
        let color = calc_star_color(star.index, &wanted);
        commands.entity(entity).insert(BackgroundColor(color));
    }

    // 移除 HUD（如果通緝消退）
    if wanted.stars == 0 {
        for entity in &hud_query {
            commands.entity(entity).despawn();
        }
    }
}
