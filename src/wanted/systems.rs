//! 通緝系統函數實作

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::player::Player;
use crate::ai::AiMovement;
use crate::combat::{Health, DamageEvent, DamageSource, HitReaction, CombatVisuals, MuzzleFlash, spawn_bullet_tracer, TracerStyle};

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

        // 設置搜索區域
        wanted.search_center = Some(event.position());
        wanted.search_radius = 30.0;

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

        // 更新搜索區域到報警位置
        wanted.search_center = Some(report.position);

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

/// 通緝等級消退系統
pub fn wanted_cooldown_system(
    mut wanted: ResMut<WantedLevel>,
    mut level_changed: MessageWriter<WantedLevelChanged>,
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    police_query: Query<(&Transform, &PoliceOfficer)>,
    rapier_context: ReadRapierContext,
) {
    // 沒有通緝就不處理
    if wanted.stars == 0 {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 檢查是否被警察看到
    let mut player_visible = false;

    if let Ok(rapier) = rapier_context.single() {
        for (police_transform, officer) in &police_query {
            if officer.state == PoliceState::Patrolling {
                continue;
            }

            let police_pos = police_transform.translation;
            let to_player = player_pos - police_pos;
            let distance = to_player.length();

            // 檢查距離
            if distance > 40.0 {
                continue;
            }

            // 檢查視線
            let direction = to_player.normalize();
            let ray_origin = police_pos + Vec3::Y * 1.5;

            if let Some((_, toi)) = rapier.cast_ray(
                ray_origin,
                direction,
                distance,
                true,
                QueryFilter::default(),
            ) {
                // 射線到達玩家附近
                if toi >= distance - 1.0 {
                    player_visible = true;
                    wanted.player_last_seen_pos = Some(player_pos);
                    break;
                }
            }
        }
    }

    wanted.player_visible = player_visible;

    // 如果玩家不可見，開始消退
    if !player_visible {
        wanted.cooldown_timer += time.delta_secs();

        let cooldown_duration = wanted.cooldown_duration();

        if wanted.cooldown_timer >= cooldown_duration {
            let old_stars = wanted.stars;

            // 每次消退減少 20 熱度（1 星）
            wanted.reduce_heat(20.0);
            wanted.cooldown_timer = 0.0;

            if wanted.stars != old_stars {
                level_changed.write(WantedLevelChanged::new(old_stars, wanted.stars));
                info!(
                    "通緝等級消退: {} -> {} (熱度: {:.1})",
                    old_stars, wanted.stars, wanted.heat
                );
            }

            // 完全消退時清除搜索區域
            if wanted.stars == 0 {
                wanted.search_center = None;
                wanted.player_last_seen_pos = None;
            }
        }
    } else {
        // 被看到時重置消退計時器
        wanted.cooldown_timer = 0.0;
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
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    for (mut transform, mut officer, movement, mut controller) in &mut police_query {
        let police_pos = transform.translation;
        let to_player = player_pos - police_pos;
        let distance = to_player.length();
        let direction = if distance > 0.1 {
            to_player.normalize()
        } else {
            Vec3::ZERO
        };

        // 更新警察狀態
        match officer.state {
            PoliceState::Patrolling => {
                // 如果有通緝，切換到警覺狀態
                if wanted.stars > 0 {
                    officer.state = PoliceState::Alerted;
                    officer.target_player = true;
                }
            }

            PoliceState::Alerted => {
                // 優先使用無線電通報的位置，否則使用搜索區域
                let target_pos = officer.radio_alert_position
                    .or(wanted.search_center);

                if let Some(search_center) = target_pos {
                    let to_search = search_center - police_pos;
                    let search_dist = to_search.length();

                    if search_dist > 2.0 {
                        let move_dir = to_search.normalize();
                        // 如果收到無線電通報，跑步前進
                        let speed = if officer.radio_alerted {
                            movement.run_speed
                        } else {
                            movement.walk_speed * 1.5
                        };
                        let dt = time.delta_secs();
                        let velocity = Vec3::new(move_dir.x * speed * dt, -9.81 * dt, move_dir.z * speed * dt);
                        controller.translation = Some(velocity);

                        // 面向移動方向
                        let target_rotation = Quat::from_rotation_y((-move_dir.x).atan2(-move_dir.z));
                        transform.rotation = transform.rotation.slerp(target_rotation, 5.0 * time.delta_secs());
                    } else {
                        // 到達目標位置，清除無線電警報
                        officer.radio_alerted = false;
                        officer.radio_alert_position = None;
                    }
                }

                // 如果看到玩家，切換到追捕
                if distance < config.vision_range && wanted.player_visible {
                    officer.state = PoliceState::Pursuing;
                    officer.radio_alerted = false;
                    officer.radio_alert_position = None;
                }
            }

            PoliceState::Pursuing => {
                // 追捕玩家
                if distance > config.attack_range {
                    // 追趕
                    let speed = movement.run_speed;
                    let dt = time.delta_secs();
                    let velocity = Vec3::new(direction.x * speed * dt, -9.81 * dt, direction.z * speed * dt);
                    controller.translation = Some(velocity);

                    // 面向玩家
                    let target_rotation = Quat::from_rotation_y((-direction.x).atan2(-direction.z));
                    transform.rotation = transform.rotation.slerp(target_rotation, 8.0 * time.delta_secs());
                } else {
                    // 進入戰鬥狀態
                    officer.state = PoliceState::Engaging;
                }

                // 如果失去視線，開始搜索
                if !wanted.player_visible {
                    officer.state = PoliceState::Searching;
                    officer.search_timer = 0.0;
                }
            }

            PoliceState::Searching => {
                // 搜索玩家
                officer.search_timer += time.delta_secs();

                // 朝最後看到的位置移動
                if let Some(last_pos) = wanted.player_last_seen_pos {
                    let to_last = last_pos - police_pos;
                    let last_dist = to_last.length();

                    if last_dist > 2.0 {
                        let move_dir = to_last.normalize();
                        let speed = movement.walk_speed;
                        let dt = time.delta_secs();
                        let velocity = Vec3::new(move_dir.x * speed * dt, -9.81 * dt, move_dir.z * speed * dt);
                        controller.translation = Some(velocity);
                    }
                }

                // 如果看到玩家，恢復追捕
                if wanted.player_visible {
                    officer.state = PoliceState::Pursuing;
                }

                // 搜索超時，返回巡邏（如果通緝消退）
                if officer.search_timer > 30.0 && wanted.stars == 0 {
                    officer.state = PoliceState::Returning;
                }
            }

            PoliceState::Engaging => {
                // 戰鬥狀態 - 保持距離並射擊
                // 面向玩家
                let dt = time.delta_secs();
                let target_rotation = Quat::from_rotation_y((-direction.x).atan2(-direction.z));
                transform.rotation = transform.rotation.slerp(target_rotation, 10.0 * dt);

                // 如果玩家跑遠了，恢復追捕
                if distance > config.attack_range * 1.5 {
                    officer.state = PoliceState::Pursuing;
                }

                // 側移躲避
                let strafe_dir = Vec3::new(-direction.z, 0.0, direction.x);
                let strafe_amount = (time.elapsed_secs() * 2.0).sin() * 0.3;
                let strafe_speed = strafe_amount * movement.walk_speed * dt;
                controller.translation = Some(Vec3::new(
                    strafe_dir.x * strafe_speed,
                    -9.81 * dt,
                    strafe_dir.z * strafe_speed,
                ));
            }

            PoliceState::Returning => {
                // 通緝消退後返回巡邏狀態
                if wanted.stars > 0 {
                    officer.state = PoliceState::Alerted;
                } else {
                    officer.state = PoliceState::Patrolling;
                }
            }
        }
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
        let ray_origin = police_pos + Vec3::Y * 1.5; // 從頭部高度發射
        let ray_direction = to_player.normalize();
        let filter = QueryFilter::default()
            .exclude_rigid_body(police_entity);

        let has_line_of_sight = match rapier.cast_ray(
            ray_origin,
            ray_direction,
            distance,
            true,
            filter,
        ) {
            Some((hit_entity, toi)) => {
                // 射線擊中玩家或接近玩家位置
                hit_entity == player_entity || toi >= distance - 1.0
            }
            None => true, // 沒有擊中任何東西，視線暢通
        };

        if !has_line_of_sight {
            // 有障礙物阻擋，無法射擊
            continue;
        }

        // 發射傷害事件（假設警察使用手槍）
        // 距離越遠，命中率越低
        let distance_penalty = (distance / config.attack_range) * config.distance_hit_penalty;
        let hit_chance = (config.base_hit_chance - distance_penalty).max(0.1);

        // 計算槍口位置（警察手部位置）
        let muzzle_pos = police_pos + Vec3::Y * 1.2 + ray_direction * 0.5;

        // 計算彈道終點
        let is_hit = rand::random::<f32>() < hit_chance;
        let tracer_end = if is_hit {
            // 命中：直指玩家身體中心
            player_pos + Vec3::Y * 1.0
        } else {
            // 未命中：彈道偏移
            let miss_offset = Vec3::new(
                rand::random::<f32>() * 2.0 - 1.0,
                rand::random::<f32>() * 1.5 - 0.5,
                rand::random::<f32>() * 2.0 - 1.0,
            );
            player_pos + Vec3::Y * 1.0 + miss_offset
        };

        // 生成視覺效果
        if let Some(ref visuals) = combat_visuals {
            // 槍口閃光
            commands.spawn((
                Mesh3d(visuals.muzzle_mesh.clone()),
                MeshMaterial3d(visuals.muzzle_material.clone()),
                Transform::from_translation(muzzle_pos),
                MuzzleFlash { lifetime: 0.05 },
            ));

            // 子彈拖尾（警察使用手槍風格）
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

/// 無線電呼叫範圍（公尺）
const RADIO_CALL_RANGE: f32 = 100.0;
/// 無線電呼叫冷卻時間（秒）
const RADIO_CALL_COOLDOWN: f32 = 5.0;

/// 警察無線電呼叫系統
///
/// 當一名警察發現玩家時，會透過無線電通知附近的其他警察，
/// 讓他們也進入警覺狀態並前往玩家位置。
///
/// 優化版本：僅兩次迭代（原本三次）
pub fn police_radio_call_system(
    mut police_query: Query<(Entity, &Transform, &mut PoliceOfficer)>,
    player_query: Query<&Transform, (With<Player>, Without<PoliceOfficer>)>,
    wanted: Res<WantedLevel>,
    time: Res<Time>,
) {
    // 沒有通緝就不處理
    if wanted.stars == 0 {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let dt = time.delta_secs();

    // 第一階段：收集發送者並同時重置冷卻
    // 合併原本的第一次和第三次迭代
    let mut radio_alerts: Vec<Vec3> = Vec::new(); // 發送者位置

    for (_entity, transform, mut officer) in &mut police_query {
        // 更新所有警察的無線電冷卻
        if officer.radio_cooldown > 0.0 {
            officer.radio_cooldown -= dt;
        }

        // 只有看到玩家且處於追捕/交戰狀態的警察才會發送無線電
        let can_send = (officer.state == PoliceState::Pursuing || officer.state == PoliceState::Engaging)
            && officer.radio_cooldown <= 0.0
            && wanted.player_visible;

        if can_send {
            radio_alerts.push(transform.translation);
            officer.radio_cooldown = RADIO_CALL_COOLDOWN;
            debug!(
                "🔊 警察在 ({:.1}, {:.1}) 發送無線電呼叫",
                transform.translation.x, transform.translation.z
            );
        }
    }

    // 如果沒有發送者，提前返回
    if radio_alerts.is_empty() {
        return;
    }

    // 第二階段：處理無線電接收
    for (_entity, transform, mut officer) in &mut police_query {
        let police_pos = transform.translation;

        // 只處理可以接收無線電的警察
        let can_receive = officer.state == PoliceState::Patrolling
            || officer.state == PoliceState::Alerted
            || officer.state == PoliceState::Searching;

        if !can_receive {
            continue;
        }

        for sender_pos in &radio_alerts {
            // 不要通知自己（發送者）
            if police_pos.distance(*sender_pos) < 1.0 {
                continue;
            }

            // 檢查是否在無線電範圍內
            if police_pos.distance(*sender_pos) <= RADIO_CALL_RANGE {
                // 收到無線電通知
                officer.radio_alerted = true;
                officer.radio_alert_position = Some(player_pos);

                // 如果正在巡邏，切換到警覺狀態
                if officer.state == PoliceState::Patrolling {
                    officer.state = PoliceState::Alerted;
                    officer.target_player = true;
                }
                break; // 收到一次就夠了
            }
        }
    }
}

/// 更新通緝等級 HUD
pub fn update_wanted_hud(
    mut commands: Commands,
    wanted: Res<WantedLevel>,
    hud_query: Query<Entity, With<WantedHud>>,
    star_query: Query<(Entity, &WantedStar)>,
) {
    // 如果 HUD 不存在，創建它
    if hud_query.is_empty() && wanted.stars > 0 {
        // 創建 HUD 容器
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

        // 創建 5 個星星
        for i in 0..5 {
            let star_entity = commands
                .spawn((
                    Node {
                        width: Val::Px(24.0),
                        height: Val::Px(24.0),
                        ..default()
                    },
                    BackgroundColor(if i < wanted.stars as usize {
                        Color::srgb(1.0, 0.8, 0.0) // 金色（啟用）
                    } else {
                        Color::srgba(0.3, 0.3, 0.3, 0.5) // 灰色（未啟用）
                    }),
                    WantedStar { index: i as u8 },
                ))
                .id();

            commands.entity(hud_entity).add_child(star_entity);
        }
    }

    // 更新星星顏色
    for (entity, star) in &star_query {
        let color = if star.index < wanted.stars {
            // 啟用的星星 - 金色閃爍
            let pulse = (wanted.cooldown_timer * 3.0).sin() * 0.2 + 0.8;
            if wanted.player_visible {
                // 被追捕時紅色
                Color::srgb(1.0, pulse * 0.3, 0.0)
            } else {
                // 消退中黃色
                Color::srgb(1.0, pulse, 0.0)
            }
        } else {
            Color::srgba(0.3, 0.3, 0.3, 0.5)
        };

        commands.entity(entity).insert(BackgroundColor(color));
    }

    // 如果通緝消退，移除 HUD
    if wanted.stars == 0 {
        for entity in &hud_query {
            // Bevy 0.17: despawn() 會自動移除子實體（星星）
            commands.entity(entity).despawn();
        }
    }
}
