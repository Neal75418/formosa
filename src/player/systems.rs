//! 玩家系統

use super::character_switch_animation::CharacterSwitchAnimation;
use super::PlayerConfig;
use super::{
    ClimbState, DodgeState, DoubleTapTracker, NoiseLevel, Player, PlayerSprintState, Stamina,
    StealthState, VehicleTransitionState,
};
use crate::combat::{CombatState, PlayerCoverState, RespawnState};
use crate::core::{rapier_real_to_f32, GameState, InteractionState};
use crate::vehicle::Vehicle;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *}; // 引入 Rapier 物理引擎類型 (RapierContext, QueryFilter 等)

// ============================================================================
// 常數
// ============================================================================

/// 速度低於此值視為停止（避免無限趨近 0）
const MIN_MOVEMENT_SPEED: f32 = 0.1;
/// 下車方向偵測射線距離
const EXIT_RAY_DISTANCE: RapierReal = 2.0;
/// 下車位置與車輛的偏移距離
const EXIT_OFFSET_DISTANCE: f32 = 2.5;
/// 車門偏移乘數（用於計算門的側向位置）
const DOOR_OFFSET_MULTIPLIER: f32 = 1.2;
/// 射線碰撞容差（判定是否碰到目標車輛後方）
const RAY_HIT_TOLERANCE: f32 = 0.5;

/// 玩家移動系統資源參數包
#[derive(SystemParam)]
pub struct PlayerMovementState<'w> {
    pub time: Res<'w, Time>,
    pub game_state: Res<'w, GameState>,
    pub respawn_state: Res<'w, RespawnState>,
    pub combat_state: Res<'w, CombatState>,
    pub camera_settings: Res<'w, crate::core::CameraSettings>,
    pub config: Res<'w, PlayerConfig>,
    pub switch_anim: Res<'w, CharacterSwitchAnimation>,
}

/// 玩家輸入處理（按住 Shift 衝刺）
pub fn player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    switch_anim: Res<CharacterSwitchAnimation>,
    mut query: Query<&mut Player>,
) {
    // 角色切換動畫期間禁止玩家輸入
    if switch_anim.is_active() {
        return;
    }
    // 死亡時不處理輸入
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    if let Ok(mut player) = query.single_mut() {
        // Ctrl 切換蹲伏
        if keyboard.just_pressed(KeyCode::ControlLeft) || keyboard.just_pressed(KeyCode::ControlRight)
        {
            player.is_crouching = !player.is_crouching;
        }
        // 按住 Shift = 衝刺（蹲伏時衝刺自動解除蹲伏）
        let wants_sprint =
            keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
        if wants_sprint && player.is_crouching {
            player.is_crouching = false;
        }
        player.is_sprinting = wants_sprint;
    }
}

/// 體力系統（衝刺消耗、步行恢復、耗盡強制步行）
pub fn stamina_system(
    time: Res<Time>,
    mut query: Query<(&mut Stamina, &mut Player)>,
) {
    let dt = time.delta_secs();
    for (mut stamina, mut player) in &mut query {
        if player.is_sprinting && player.current_speed > player.speed * 0.5 {
            // 衝刺中消耗體力
            if !stamina.drain(dt) {
                // 體力耗盡，強制步行
                player.is_sprinting = false;
            }
        } else {
            // 非衝刺時恢復體力
            stamina.regenerate(dt);
        }

        // 體力耗盡狀態下禁止衝刺
        if stamina.exhausted {
            player.is_sprinting = false;
        }
    }
}

/// 計算玩家移動輸入（WASD + 滑鼠雙鍵）
fn calculate_movement_input(
    keyboard: &ButtonInput<KeyCode>,
    mouse_button: &ButtonInput<MouseButton>,
    in_cover: bool,
) -> Vec3 {
    let mut input = Vec3::ZERO;

    // 滑鼠左右鍵同時按 = 直走
    if mouse_button.pressed(MouseButton::Left) && mouse_button.pressed(MouseButton::Right) {
        input.z -= 1.0;
    }

    // WASD 基本移動
    if keyboard.pressed(KeyCode::KeyW) {
        input.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        input.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        input.x += 1.0;
    }

    // Q/E 斜向前進（左前方/右前方）
    // 掩體狀態下不處理（掩體系統用 Q/E 切換掩體位置）
    if !in_cover {
        if keyboard.pressed(KeyCode::KeyQ) {
            input.z -= 1.0; // 前
            input.x -= 1.0; // 左
        }
        if keyboard.pressed(KeyCode::KeyE) {
            input.z -= 1.0; // 前
            input.x += 1.0; // 右
        }
    }

    input
}

/// 計算世界空間移動方向
fn calculate_world_direction(input: Vec3, yaw: f32) -> Vec3 {
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    (forward * (-input.z) + right * input.x).normalize_or_zero()
}

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
    rotation_speed: f32,
) {
    // 動作遊戲風格：更快的旋轉響應
    let rotation_factor = (rotation_speed * dt).min(1.0);

    if is_aiming || !is_forward_movement {
        // 瞄準時或後退/平移時：面向攝影機方向
        let target_rotation = Quat::from_rotation_y(yaw + std::f32::consts::PI);
        transform.rotation = transform.rotation.slerp(target_rotation, rotation_factor);
    } else {
        // 向前走時：面向移動方向（模型前方是 +Z，需加 PI）
        let target_rotation =
            Quat::from_rotation_y((-direction.x).atan2(-direction.z) + std::f32::consts::PI);
        transform.rotation = transform.rotation.slerp(target_rotation, rotation_factor);
    }
}

/// 玩家移動（新控制方式）
/// - WASD = 前後左右
/// - Q/E = 左前方/右前方斜向移動
/// - 滑鼠左右鍵同時按 = 直走（跟隨視角方向）
/// - 瞄準時角色面向攝影機方向
/// - 閃避期間不處理普通移動（由閃避系統接管）
pub fn player_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    state: PlayerMovementState,
    mut query: Query<(
        &mut Transform,
        &mut Player,
        &DodgeState,
        &PlayerCoverState,
        &ClimbState,
        &mut KinematicCharacterController,
        &mut PlayerSprintState,
    )>,
) {
    if state.switch_anim.is_active() || state.respawn_state.is_dead || state.game_state.player_in_vehicle {
        return;
    }

    let Ok((mut transform, mut player, dodge, cover_state, climb_state, mut controller, mut sprint_state)) =
        query.single_mut()
    else {
        return;
    };

    // 閃避或攀爬期間不處理普通移動
    if dodge.is_dodging || climb_state.is_climbing() {
        return;
    }

    let input = calculate_movement_input(&keyboard, &mouse_button, cover_state.is_in_cover);
    let yaw = state.camera_settings.yaw;
    let dt = state.time.delta_secs();

    // 判斷是否為「向前移動」（W 鍵或滑鼠雙鍵直走，且沒有後退）
    let is_forward_movement = input.z < 0.0 && input.z.abs() >= input.x.abs();

    // 瞄準時始終更新朝向
    if state.combat_state.is_aiming && input == Vec3::ZERO {
        transform.rotation = Quat::from_rotation_y(yaw + std::f32::consts::PI);
    }

    // === 加速度系統：平滑過渡速度 ===
    let has_input = input != Vec3::ZERO;
    let target_speed = if has_input {
        if player.is_sprinting {
            player.sprint_speed
        } else if player.is_crouching {
            player.crouch_speed
        } else {
            player.speed
        }
    } else {
        0.0 // 沒有輸入時減速到 0
    };

    // 指數衰減插值（比線性更自然）
    // acceleration_time/deceleration_time 已在 Player::default() 中驗證 > 0
    let accel_rate = if target_speed > player.current_speed {
        1.0 / player.acceleration_time
    } else {
        1.0 / player.deceleration_time
    };
    // 公式：current = lerp(current, target, 1 - exp(-rate * dt))
    let blend = 1.0 - (-accel_rate * dt).exp();
    player.current_speed = player.current_speed + (target_speed - player.current_speed) * blend;

    // 速度過低時視為停止（避免無限趨近 0）
    if player.current_speed < MIN_MOVEMENT_SPEED && !has_input {
        player.current_speed = 0.0;
        controller.translation = Some(Vec3::ZERO);
        return;
    }

    // 計算移動方向
    let direction = if has_input {
        let new_dir = calculate_world_direction(input.normalize(), yaw);
        // 儲存當前移動方向供慣性滑行使用
        player.last_movement_direction = new_dir;
        new_dir
    } else {
        // 沒有輸入時使用上次的移動方向（慣性滑行）
        player.last_movement_direction
    };

    controller.translation = Some(direction * player.current_speed * dt);

    // === 動態轉向速度：高速時轉向較慢（更真實） ===
    let speed_ratio = (player.current_speed / player.sprint_speed).clamp(0.0, 1.0);
    let dynamic_rotation_speed = state.config.movement.turn_speed_walk
        + (state.config.movement.turn_speed_sprint - state.config.movement.turn_speed_walk) * speed_ratio;

    update_character_rotation(
        &mut transform,
        direction,
        yaw,
        state.combat_state.is_aiming,
        is_forward_movement,
        dt,
        dynamic_rotation_speed,
    );

    // === 更新衝刺狀態機（用於動畫/音效系統） ===
    sprint_state.state.update(player.current_speed, player.speed, player.sprint_speed, dt);
}

/// 玩家跳躍
pub fn player_jump(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    config: Res<PlayerConfig>,
    mut query: Query<(
        &mut Player,
        &ClimbState,
        &mut KinematicCharacterController,
        Option<&KinematicCharacterControllerOutput>,
    )>,
) {
    // 死亡時或在車上時不處理跳躍
    if respawn_state.is_dead || game_state.player_in_vehicle {
        return;
    }

    let Ok((mut player, climb_state, mut controller, controller_output)) = query.single_mut()
    else {
        return;
    };

    if climb_state.is_climbing() {
        return;
    }

    let dt = time.delta_secs();

    let output_grounded = controller_output
        .map(|output| output.grounded)
        .unwrap_or(player.is_grounded);

    if output_grounded && player.vertical_velocity <= 0.0 {
        player.is_grounded = true;
        player.vertical_velocity = 0.0;
    } else if !output_grounded {
        player.is_grounded = false;
    }

    if keyboard.just_pressed(KeyCode::Space) && player.is_grounded {
        player.vertical_velocity = player.jump_force;
        player.is_grounded = false;
    }

    if !player.is_grounded {
        player.vertical_velocity -= config.movement.gravity * dt;
        // 終端速度限制，避免高速穿透
        player.vertical_velocity = player
            .vertical_velocity
            .max(-config.movement.max_fall_speed);

        let mut translation = controller.translation.unwrap_or(Vec3::ZERO);
        translation.y += player.vertical_velocity * dt;
        controller.translation = Some(translation);
    }
}

/// 上下車（F 鍵）- GTA 5 風格動畫版
pub fn enter_exit_vehicle(
    mut interaction: ResMut<InteractionState>,
    game_state: ResMut<GameState>,
    mut transition: ResMut<VehicleTransitionState>,
    player_query: Query<&mut Transform, (With<Player>, Without<Vehicle>)>,
    vehicle_query: Query<(Entity, &mut Transform, &mut Vehicle), Without<Player>>,
    _visibility_query: Query<&mut Visibility, With<Player>>,
    rapier_context: ReadRapierContext,
    config: Res<PlayerConfig>,
) {
    if transition.is_animating() || !interaction.can_interact() {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    if game_state.player_in_vehicle {
        try_exit_vehicle(&game_state, &mut transition, &mut interaction, &vehicle_query, &rapier_context, &config);
    } else {
        try_enter_vehicle(&mut transition, &mut interaction, player_transform, &vehicle_query, &rapier_context, &config);
    }
}

fn try_exit_vehicle(
    game_state: &GameState,
    transition: &mut VehicleTransitionState,
    interaction: &mut InteractionState,
    vehicle_query: &Query<(Entity, &mut Transform, &mut Vehicle), Without<Player>>,
    rapier_context: &ReadRapierContext,
    config: &PlayerConfig,
) {
    let Ok(rapier_ctx) = rapier_context.single() else {
        return;
    };
    let Some(vehicle_entity) = game_state.current_vehicle else {
        return;
    };
    let Ok((_, vehicle_transform, _)) = vehicle_query.get(vehicle_entity) else {
        return;
    };

    let right = vehicle_transform.right();
    let left = -right;
    let origin = vehicle_transform.translation
        + Vec3::new(0.0, config.interaction.ray_origin_height, 0.0);
    let filter = QueryFilter::new().exclude_rigid_body(vehicle_entity);

    let (exit_dir, from_right) = if rapier_ctx
        .cast_ray(origin, *right, EXIT_RAY_DISTANCE, true, filter)
        .is_none()
    {
        (*right, true)
    } else if rapier_ctx
        .cast_ray(origin, *left, EXIT_RAY_DISTANCE, true, filter)
        .is_none()
    {
        (*left, false)
    } else {
        (*right, true)
    };

    let exit_pos = vehicle_transform.translation + exit_dir * EXIT_OFFSET_DISTANCE;
    let seat_pos = vehicle_transform.translation;

    transition.start_exit(seat_pos, vehicle_entity, exit_pos, from_right);
    interaction.consume();
}

fn try_enter_vehicle(
    transition: &mut VehicleTransitionState,
    interaction: &mut InteractionState,
    player_transform: &Transform,
    vehicle_query: &Query<(Entity, &mut Transform, &mut Vehicle), Without<Player>>,
    rapier_context: &ReadRapierContext,
    config: &PlayerConfig,
) {
    let Ok(rapier_ctx) = rapier_context.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let ray_origin = player_pos + Vec3::new(0.0, config.interaction.ray_origin_height, 0.0);

    let nearest_vehicle = vehicle_query
        .iter()
        .filter_map(|(entity, transform, vehicle)| {
            let distance = can_enter_vehicle(
                entity,
                vehicle,
                transform.translation,
                player_pos,
                ray_origin,
                &rapier_ctx,
                config,
            )?;
            Some((entity, transform.translation, distance))
        })
        .min_by(|(_, _, a), (_, _, b)| a.total_cmp(b));

    let Some((vehicle_entity, vehicle_pos, _)) = nearest_vehicle else {
        return;
    };
    let Ok((_, vehicle_transform, _)) = vehicle_query.get(vehicle_entity) else {
        return;
    };

    let to_vehicle_delta = vehicle_pos - player_pos;
    let to_vehicle = if to_vehicle_delta.length_squared() > 1e-6 {
        to_vehicle_delta.normalize()
    } else {
        player_transform.forward().as_vec3()
    };
    let vehicle_right = vehicle_transform.right();
    let from_right = to_vehicle.dot(*vehicle_right) > 0.0;

    let door_offset = if from_right {
        *vehicle_right
    } else {
        -*vehicle_right
    } * DOOR_OFFSET_MULTIPLIER;
    let door_pos = vehicle_pos + door_offset;

    transition.start_enter(player_pos, vehicle_entity, door_pos, from_right);
    interaction.consume();
}

// /// 上車距離常數 (Moved to Config)
// const VEHICLE_ENTRY_DISTANCE: f32 = 4.0;

/// 檢查到車輛的路徑是否暢通（射線檢測）
fn is_path_clear_to_vehicle(
    ray_origin: Vec3,
    vehicle_entity: Entity,
    vehicle_pos: Vec3,
    distance: f32,
    rapier_context: &RapierContext,
) -> bool {
    if distance <= f32::EPSILON {
        return true;
    }
    let direction_delta = vehicle_pos - ray_origin;
    if direction_delta.length_squared() <= 1e-6 {
        return true;
    }
    let direction = direction_delta.normalize();
    let filter = QueryFilter::new();

    match rapier_context.cast_ray(ray_origin, direction, distance as RapierReal, true, filter) {
        Some((hit_entity, toi)) => {
            // 如果碰到的是目標車輛，或碰撞點在車輛之後，則路徑暢通
            hit_entity == vehicle_entity || rapier_real_to_f32(toi) >= distance - RAY_HIT_TOLERANCE
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
    config: &PlayerConfig,
) -> Option<f32> {
    if vehicle.is_occupied {
        return None;
    }

    let distance = player_pos.distance(vehicle_pos);
    if distance >= config.interaction.vehicle_entry_distance {
        return None;
    }

    if !is_path_clear_to_vehicle(ray_origin, entity, vehicle_pos, distance, rapier_context) {
        return None;
    }

    Some(distance)
}

// ============================================================================
// 閃避系統
// ============================================================================
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

    let Ok(mut dodge) = query.single_mut() else {
        return;
    };

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
pub fn dodge_state_update_system(time: Res<Time>, mut query: Query<&mut DodgeState, With<Player>>) {
    let Ok(mut dodge) = query.single_mut() else {
        return;
    };
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

    let Ok((dodge, mut controller)) = query.single_mut() else {
        return;
    };

    if dodge.is_dodging {
        let velocity = dodge.get_dodge_velocity();
        controller.translation = Some(velocity * time.delta_secs());
    }
}

// ============================================================================
// 潛行噪音系統
// ============================================================================

/// 更新玩家噪音等級
///
/// 根據玩家移動狀態和射擊狀態計算噪音值，
/// 射擊後噪音在 `NOISE_DECAY_TIME` 秒內衰減。
pub fn stealth_noise_system(
    time: Res<Time>,
    combat_state: Res<CombatState>,
    player_query: Query<&Player>,
    mut stealth: ResMut<StealthState>,
) {
    let dt = time.delta_secs();
    let current_time = time.elapsed_secs();

    // 射擊產生最大噪音
    let recently_fired =
        (current_time - combat_state.last_shot_time) < super::NOISE_DECAY_TIME;

    if recently_fired {
        stealth.noise_level = NoiseLevel::Max;
        stealth.noise_decay_timer = super::NOISE_DECAY_TIME
            - (current_time - combat_state.last_shot_time);
        return;
    }

    // 衰減計時器
    if stealth.noise_decay_timer > 0.0 {
        stealth.noise_decay_timer -= dt;
        if stealth.noise_decay_timer > 0.0 {
            stealth.noise_level = NoiseLevel::Loud;
            return;
        }
    }

    // 根據移動狀態決定噪音
    let Ok(player) = player_query.single() else {
        return;
    };

    stealth.noise_level = if player.is_crouching {
        NoiseLevel::Silent
    } else if player.is_sprinting && player.current_speed > player.speed * 0.5 {
        NoiseLevel::Loud
    } else if player.current_speed > 0.5 {
        NoiseLevel::Low
    } else {
        NoiseLevel::Silent
    };
}

