//! 玩家系統

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *}; // 引入 Rapier 物理引擎類型 (RapierContext, QueryFilter 等)
use super::{Player, DodgeState, DoubleTapTracker, VehicleTransitionState, VehicleTransitionPhase};
use crate::core::GameState;
use crate::vehicle::Vehicle;
use crate::combat::{RespawnState, CombatState};
use crate::wanted::CrimeEvent;
use crate::pedestrian::Pedestrian;

/// 將 Rapier 的 Real 類型轉換為 f32
/// 注意：bevy_rapier3d 0.32 的 Real 就是 f32，但因與 bevy::prelude::Real 衝突需明確轉換
#[inline]
fn rapier_real_to_f32(r: bevy_rapier3d::prelude::Real) -> f32 {
    r
}

/// 玩家輸入處理（按住 Shift 衝刺）
pub fn player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    mut query: Query<&mut Player>,
) {
    // 死亡時不處理輸入
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    if let Ok(mut player) = query.single_mut() {
        // 按住 Shift = 衝刺（更直覺的控制）
        player.is_sprinting = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    }
}

/// 計算玩家移動輸入（WASD + 滑鼠雙鍵）
fn calculate_movement_input(
    keyboard: &ButtonInput<KeyCode>,
    mouse_button: &ButtonInput<MouseButton>,
) -> Vec3 {
    let mut input = Vec3::ZERO;

    // 滑鼠左右鍵同時按 = 直走
    if mouse_button.pressed(MouseButton::Left) && mouse_button.pressed(MouseButton::Right) {
        input.z -= 1.0;
    }

    // WASD 基本移動
    if keyboard.pressed(KeyCode::KeyW) { input.z -= 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { input.z += 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { input.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { input.x += 1.0; }

    input
}

/// 計算世界空間移動方向
fn calculate_world_direction(input: Vec3, yaw: f32) -> Vec3 {
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    (forward * (-input.z) + right * input.x).normalize()
}

/// 角色旋轉速度（越大越快，動作遊戲風格使用較高值）
const ROTATION_SPEED: f32 = 25.0;

/// 更新角色旋轉
/// - 向前走時：面向移動方向
/// - 後退/平移時：保持面向攝影機方向（像瞄準一樣）
fn update_character_rotation(
    transform: &mut Transform,
    direction: Vec3,
    yaw: f32,
    is_aiming: bool,
    is_forward_movement: bool,
    dt: f32,
) {
    // 動作遊戲風格：更快的旋轉響應
    let rotation_factor = (ROTATION_SPEED * dt).min(1.0);

    if is_aiming || !is_forward_movement {
        // 瞄準時或後退/平移時：面向攝影機方向
        let target_rotation = Quat::from_rotation_y(yaw + std::f32::consts::PI);
        transform.rotation = transform.rotation.slerp(target_rotation, rotation_factor);
    } else {
        // 向前走時：面向移動方向（模型前方是 +Z，需加 PI）
        let target_rotation = Quat::from_rotation_y((-direction.x).atan2(-direction.z) + std::f32::consts::PI);
        transform.rotation = transform.rotation.slerp(target_rotation, rotation_factor);
    }
}

/// 玩家移動（新控制方式）
/// - WASD = 前後左右
/// - 滑鼠左右鍵同時按 = 直走（跟隨視角方向）
/// - 瞄準時角色面向攝影機方向
/// - 閃避期間不處理普通移動（由閃避系統接管）
#[allow(clippy::too_many_arguments)]
pub fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    combat_state: Res<CombatState>,
    camera_settings: Res<crate::core::CameraSettings>,
    mut query: Query<(&mut Transform, &Player, &DodgeState, &mut KinematicCharacterController)>,
) {
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    let Ok((mut transform, player, dodge, mut controller)) = query.single_mut() else {
        return;
    };

    // 閃避期間不處理普通移動
    if dodge.is_dodging {
        return;
    }

    let input = calculate_movement_input(&keyboard, &mouse_button);
    let yaw = camera_settings.yaw;
    let dt = time.delta_secs();

    // 判斷是否為「向前移動」（W 鍵或滑鼠雙鍵直走，且沒有後退）
    let is_forward_movement = input.z < 0.0 && input.z.abs() >= input.x.abs();

    // 瞄準時始終更新朝向
    if combat_state.is_aiming && input == Vec3::ZERO {
        transform.rotation = Quat::from_rotation_y(yaw + std::f32::consts::PI);
    }

    if input == Vec3::ZERO {
        controller.translation = Some(Vec3::ZERO);
        return;
    }

    let direction = calculate_world_direction(input.normalize(), yaw);
    let speed = if player.is_sprinting { player.sprint_speed } else { player.speed };

    controller.translation = Some(direction * speed * dt);
    update_character_rotation(&mut transform, direction, yaw, combat_state.is_aiming, is_forward_movement, dt);
}

/// 玩家跳躍
pub fn player_jump(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    mut query: Query<(&mut Transform, &mut Player)>,
) {
    // 死亡時或在車上時不處理跳躍
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    let Ok((mut transform, mut player)) = query.single_mut() else {
        return;
    };

    const GRAVITY: f32 = 30.0;
    // 修正：碰撞體中心 Y = COLLIDER_HALF_HEIGHT + COLLIDER_RADIUS = 0.45 + 0.25 = 0.7
    // 當腳踩在地面（Y=0）時，Transform 中心應在 Y=0.7
    const GROUND_CENTER_Y: f32 = 0.7;

    if keyboard.just_pressed(KeyCode::Space) && player.is_grounded {
        player.vertical_velocity = player.jump_force;
        player.is_grounded = false;
    }

    if !player.is_grounded {
        player.vertical_velocity -= GRAVITY * time.delta_secs();
        // 終端速度限制，防止穿透地面
        player.vertical_velocity = player.vertical_velocity.max(-50.0);
        transform.translation.y += player.vertical_velocity * time.delta_secs();

        if transform.translation.y <= GROUND_CENTER_Y {
            transform.translation.y = GROUND_CENTER_Y;
            player.vertical_velocity = 0.0;
            player.is_grounded = true;
        }
    }
}

/// 上下車（Tab 鍵）- GTA 5 風格動畫版
pub fn enter_exit_vehicle(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut transition: ResMut<VehicleTransitionState>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<Vehicle>)>,
    mut vehicle_query: Query<(Entity, &mut Transform, &mut Vehicle), Without<Player>>,
    mut visibility_query: Query<&mut Visibility, With<Player>>,
    rapier_context: ReadRapierContext,
) {
    // 如果正在動畫中，不處理輸入
    if transition.is_animating() {
        return;
    }

    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }

    let Ok(player_transform) = player_query.single() else { return; };

    // 如果玩家在車上 -> 開始下車動畫
    if game_state.player_in_vehicle {
        let Ok(rapier_ctx) = rapier_context.single() else { return; };
        let Some(vehicle_entity) = game_state.current_vehicle else { return; };
        let Ok((_, vehicle_transform, _)) = vehicle_query.get(vehicle_entity) else { return; };

        // 計算下車位置
        let right = vehicle_transform.right();
        let left = -right;
        let origin = vehicle_transform.translation + Vec3::new(0.0, 0.5, 0.0);
        let filter = QueryFilter::new().exclude_rigid_body(vehicle_entity);

        // 決定下車方向
        let (exit_dir, from_right) = if rapier_ctx.cast_ray(origin, *right, 2.0 as RapierReal, true, filter).is_none() {
            (*right, true)
        } else if rapier_ctx.cast_ray(origin, *left, 2.0 as RapierReal, true, filter).is_none() {
            (*left, false)
        } else {
            (*right, true) // 預設右側
        };

        let exit_pos = vehicle_transform.translation + exit_dir * 2.5;
        let seat_pos = vehicle_transform.translation;

        // 開始下車動畫
        transition.start_exit(seat_pos, vehicle_entity, exit_pos, from_right);
    }
    // 如果玩家在車外 -> 開始上車動畫
    else {
        let Ok(rapier_ctx) = rapier_context.single() else { return; };
        let player_pos = player_transform.translation;
        let ray_origin = player_pos + Vec3::new(0.0, 0.5, 0.0);

        // 尋找最近且可到達的車輛
        let nearest_vehicle = vehicle_query.iter()
            .filter_map(|(entity, transform, vehicle)| {
                let distance = can_enter_vehicle(
                    entity, vehicle, transform.translation, player_pos, ray_origin, &rapier_ctx
                )?;
                Some((entity, transform.translation, distance))
            })
            .min_by(|(_, _, a), (_, _, b)| a.total_cmp(b));

        let Some((vehicle_entity, vehicle_pos, _)) = nearest_vehicle else { return; };
        let Ok((_, vehicle_transform, _)) = vehicle_query.get(vehicle_entity) else { return; };

        // 計算上車側（玩家面向車輛的哪一側）
        let to_vehicle = (vehicle_pos - player_pos).normalize();
        let vehicle_right = vehicle_transform.right();
        let from_right = to_vehicle.dot(*vehicle_right) > 0.0;

        // 門的位置（車輛側邊）
        let door_offset = if from_right { *vehicle_right } else { -*vehicle_right } * 1.2;
        let door_pos = vehicle_pos + door_offset;

        // 開始上車動畫
        transition.start_enter(player_pos, vehicle_entity, door_pos, from_right);
    }
}


/// 處理下車邏輯 (Helper)
fn handle_vehicle_exit(
    game_state: &mut ResMut<GameState>,
    player_transform: &mut Transform,
    vehicle_query: &mut Query<(Entity, &mut Transform, &mut Vehicle), Without<Player>>,
    visibility_query: &mut Query<&mut Visibility, With<Player>>,
    rapier_context: &RapierContext,
) {
    let Some(vehicle_entity) = game_state.current_vehicle else { return; };
    let Ok((_, vehicle_transform, mut vehicle)) = vehicle_query.get_mut(vehicle_entity) else { return; };

    // 智慧下車檢測參數
    // let check_dist: f32 = 2.0; // Inlined below to avoid type issues
    let exit_dist_normal = 2.5;
    let exit_dist_blocked = 3.5;

    let right = vehicle_transform.right();
    let left = -right;
    let up = vehicle_transform.up();
    let origin = vehicle_transform.translation + Vec3::new(0.0, 0.5, 0.0);
    
    let filter = QueryFilter::new().exclude_rigid_body(vehicle_entity);

    // 決定下車方向
    // 使用 as RapierReal 強制轉型，解決 f32/f64 與 Real 的類型不匹配問題
    let (exit_dir, valid_exit) = if rapier_context.cast_ray(origin, *right, 2.0 as RapierReal, true, filter).is_none() {
        (*right, true)
    } else if rapier_context.cast_ray(origin, *left, 2.0 as RapierReal, true, filter).is_none() {
        (*left, true)
    } else {
        (*up, false)
    };
    
    let exit_dist = if valid_exit { exit_dist_normal } else { exit_dist_blocked };
    let exit_offset = exit_dir * exit_dist;
    
    player_transform.translation = vehicle_transform.translation + exit_offset;
    // 修正：使用正確的碰撞體中心高度 (0.45 + 0.25 = 0.7)
    player_transform.translation.y = 0.7; 
    
    vehicle.is_occupied = false;
    vehicle.current_speed = 0.0;
    
    game_state.player_in_vehicle = false;
    game_state.current_vehicle = None;
    
    if let Ok(mut visibility) = visibility_query.single_mut() {
        *visibility = Visibility::Visible;
    }
}

/// 上車距離常數
const VEHICLE_ENTRY_DISTANCE: f32 = 4.0;

/// 檢查到車輛的路徑是否暢通（射線檢測）
fn is_path_clear_to_vehicle(
    ray_origin: Vec3,
    vehicle_entity: Entity,
    vehicle_pos: Vec3,
    distance: f32,
    rapier_context: &RapierContext,
) -> bool {
    let direction = (vehicle_pos - ray_origin).normalize();
    let filter = QueryFilter::new();

    match rapier_context.cast_ray(ray_origin, direction, distance as RapierReal, true, filter) {
        Some((hit_entity, toi)) => {
            // 如果碰到的是目標車輛，或碰撞點在車輛之後，則路徑暢通
            hit_entity == vehicle_entity || rapier_real_to_f32(toi) >= distance - 0.5
        }
        None => true, // 沒有碰到任何東西，路徑暢通
    }
}

/// 檢查車輛是否可以上車
fn can_enter_vehicle(
    entity: Entity,
    vehicle: &Vehicle,
    vehicle_pos: Vec3,
    player_pos: Vec3,
    ray_origin: Vec3,
    rapier_context: &RapierContext,
) -> Option<f32> {
    if vehicle.is_occupied {
        return None;
    }

    let distance = player_pos.distance(vehicle_pos);
    if distance >= VEHICLE_ENTRY_DISTANCE {
        return None;
    }

    if !is_path_clear_to_vehicle(ray_origin, entity, vehicle_pos, distance, rapier_context) {
        return None;
    }

    Some(distance)
}

/// 處理上車邏輯 (Helper)
/// 包含射線檢測，防止穿牆上車
fn handle_vehicle_entry(
    game_state: &mut ResMut<GameState>,
    player_transform: &Transform,
    vehicle_query: &mut Query<(Entity, &mut Transform, &mut Vehicle), Without<Player>>,
    visibility_query: &mut Query<&mut Visibility, With<Player>>,
    rapier_context: &RapierContext,
) {
    let player_pos = player_transform.translation;
    let ray_origin = player_pos + Vec3::new(0.0, 0.5, 0.0);

    // 尋找最近且可到達的車輛
    let nearest_vehicle = vehicle_query.iter()
        .filter_map(|(entity, transform, vehicle)| {
            let distance = can_enter_vehicle(
                entity, vehicle, transform.translation, player_pos, ray_origin, rapier_context
            )?;
            Some((entity, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b));

    // 上車
    let Some((vehicle_entity, _)) = nearest_vehicle else { return; };
    let Ok((_, _, mut vehicle)) = vehicle_query.get_mut(vehicle_entity) else { return; };

    vehicle.is_occupied = true;
    game_state.player_in_vehicle = true;
    game_state.current_vehicle = Some(vehicle_entity);

    if let Ok(mut visibility) = visibility_query.single_mut() {
        *visibility = Visibility::Hidden;
    }
}

// === 閃避系統 ===

/// 閃避偵測系統（雙擊方向鍵觸發閃避）
pub fn dodge_detection_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    camera_settings: Res<crate::core::CameraSettings>,
    mut tracker: ResMut<DoubleTapTracker>,
    mut query: Query<&mut DodgeState, With<Player>>,
) {
    // 死亡或在車上時不處理閃避
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    let Ok(mut dodge) = query.single_mut() else { return; };

    // 正在閃避中則不檢測新的閃避
    if dodge.is_dodging {
        return;
    }

    // 更新累計時間
    tracker.time += time.delta_secs();
    let current_time = tracker.time;
    let yaw = camera_settings.yaw;

    // 檢測 WASD 雙擊
    let keys = [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD];
    for key in keys {
        if keyboard.just_pressed(key) && tracker.check_double_tap(key, current_time) {
            // 計算閃避方向（根據按鍵和攝影機朝向）
            let direction = match key {
                KeyCode::KeyW => calculate_world_direction(Vec3::new(0.0, 0.0, -1.0), yaw),
                KeyCode::KeyS => calculate_world_direction(Vec3::new(0.0, 0.0, 1.0), yaw),
                KeyCode::KeyA => calculate_world_direction(Vec3::new(-1.0, 0.0, 0.0), yaw),
                KeyCode::KeyD => calculate_world_direction(Vec3::new(1.0, 0.0, 0.0), yaw),
                _ => Vec3::ZERO,
            };
            if direction != Vec3::ZERO {
                dodge.start_dodge(direction);
            }
            break;
        }
    }
}

/// 閃避狀態更新系統
pub fn dodge_state_update_system(
    time: Res<Time>,
    mut query: Query<&mut DodgeState, With<Player>>,
) {
    let Ok(mut dodge) = query.single_mut() else { return; };
    dodge.update(time.delta_secs());
}

/// 閃避移動系統（應用閃避位移）
pub fn dodge_movement_system(
    time: Res<Time>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    mut query: Query<(&DodgeState, &mut KinematicCharacterController), With<Player>>,
) {
    // 死亡或在車上時不處理
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    let Ok((dodge, mut controller)) = query.single_mut() else { return; };

    if dodge.is_dodging {
        let velocity = dodge.get_dodge_velocity();
        controller.translation = Some(velocity * time.delta_secs());
    }
}

// ============================================================================
// 車輛進出動畫系統 (GTA 5 風格)
// ============================================================================

/// 車輛進出動畫更新系統
/// 處理上下車動畫的位置插值和狀態轉換
pub fn vehicle_transition_animation_system(
    time: Res<Time>,
    mut transition: ResMut<VehicleTransitionState>,
    mut game_state: ResMut<GameState>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<Vehicle>)>,
    mut vehicle_query: Query<(&Transform, &mut Vehicle), Without<Player>>,
    mut visibility_query: Query<&mut Visibility, With<Player>>,
    mut crime_events: MessageWriter<CrimeEvent>,
    pedestrian_query: Query<&Transform, (With<Pedestrian>, Without<Player>, Without<Vehicle>)>,
) {
    if !transition.is_animating() {
        return;
    }

    let dt = time.delta_secs();
    let should_advance = transition.update(dt);

    let Ok(mut player_transform) = player_query.single_mut() else { return; };
    let Some(vehicle_entity) = transition.target_vehicle else {
        transition.reset();
        return;
    };

    // 取得車輛資訊
    let vehicle_info = vehicle_query.get(vehicle_entity).ok().map(|(t, _)| t.translation);
    let Some(vehicle_pos) = vehicle_info else {
        transition.reset();
        return;
    };

    // 根據當前階段處理動畫
    let progress = ease_in_out_cubic(transition.progress.clamp(0.0, 1.0));

    match transition.phase {
        VehicleTransitionPhase::WalkingToVehicle => {
            // 玩家走向車門
            let new_pos = transition.start_position.lerp(transition.target_position, progress);
            player_transform.translation = new_pos;
            player_transform.translation.y = 0.7; // 保持正確高度

            // 面向車輛
            let to_vehicle = (vehicle_pos - player_transform.translation).normalize();
            if to_vehicle.length_squared() > 0.01 {
                let target_rotation = Quat::from_rotation_y((-to_vehicle.x).atan2(-to_vehicle.z));
                player_transform.rotation = player_transform.rotation.slerp(target_rotation, 0.2);
            }
        }
        VehicleTransitionPhase::OpeningDoor => {
            // 玩家停在門旁，門正在打開（視覺效果在其他系統處理）
        }
        VehicleTransitionPhase::EnteringVehicle => {
            // 玩家從門旁移動到座位
            let seat_pos = vehicle_pos;
            let new_pos = transition.target_position.lerp(seat_pos, progress);
            player_transform.translation = new_pos;

            // 逐漸隱藏玩家
            if progress > 0.5 {
                if let Ok(mut vis) = visibility_query.single_mut() {
                    *vis = Visibility::Hidden;
                }
            }
        }
        VehicleTransitionPhase::ClosingDoor => {
            // 門正在關閉，玩家已經在車內
            // 確保玩家隱藏
            if let Ok(mut vis) = visibility_query.single_mut() {
                *vis = Visibility::Hidden;
            }
        }
        VehicleTransitionPhase::OpeningDoorExit => {
            // 下車：門正在打開
            // 玩家即將可見
            if progress > 0.3 {
                if let Ok(mut vis) = visibility_query.single_mut() {
                    *vis = Visibility::Visible;
                }
                // 設置玩家初始位置在座位
                player_transform.translation = vehicle_pos;
                player_transform.translation.y = 0.7;
            }
        }
        VehicleTransitionPhase::ExitingVehicle => {
            // 玩家從座位移動到門外
            let new_pos = transition.start_position.lerp(transition.target_position, progress);
            player_transform.translation = new_pos;
            player_transform.translation.y = 0.7;

            // 確保玩家可見
            if let Ok(mut vis) = visibility_query.single_mut() {
                *vis = Visibility::Visible;
            }
        }
        VehicleTransitionPhase::ClosingDoorExit => {
            // 門正在關閉
            player_transform.translation = transition.target_position;
            player_transform.translation.y = 0.7;
        }
        VehicleTransitionPhase::WalkingAway => {
            // 玩家走離車輛（小距離移動）
            let away_dir = (transition.target_position - vehicle_pos).normalize();
            let final_pos = transition.target_position + away_dir * 0.5;
            let new_pos = transition.target_position.lerp(final_pos, progress);
            player_transform.translation = new_pos;
            player_transform.translation.y = 0.7;
        }
        VehicleTransitionPhase::None => {}
    }

    // 切換到下一階段
    if should_advance {
        let current_phase = transition.phase;
        transition.advance_phase();

        // 處理狀態變更
        match current_phase {
            VehicleTransitionPhase::ClosingDoor => {
                // 上車動畫完成 - 更新遊戲狀態
                if let Ok((vehicle_transform, mut vehicle)) = vehicle_query.get_mut(vehicle_entity) {
                    let vehicle_pos = vehicle_transform.translation;

                    // GTA 風格：只有在有目擊者時才觸發搶車犯罪
                    // 檢查 20 單位內是否有行人目擊
                    const WITNESS_RANGE: f32 = 20.0;
                    let has_witness = pedestrian_query.iter().any(|ped_transform| {
                        ped_transform.translation.distance(vehicle_pos) < WITNESS_RANGE
                    });

                    if has_witness {
                        crime_events.write(CrimeEvent::VehicleTheft {
                            position: vehicle_pos,
                        });
                    }
                    vehicle.is_occupied = true;
                }
                game_state.player_in_vehicle = true;
                game_state.current_vehicle = Some(vehicle_entity);
            }
            VehicleTransitionPhase::WalkingAway => {
                // 下車動畫完成 - 更新遊戲狀態
                if let Ok((_, mut vehicle)) = vehicle_query.get_mut(vehicle_entity) {
                    vehicle.is_occupied = false;
                    vehicle.current_speed = 0.0;
                }
                game_state.player_in_vehicle = false;
                game_state.current_vehicle = None;
            }
            _ => {}
        }
    }
}

/// 緩入緩出曲線（用於平滑位置插值）
fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}
