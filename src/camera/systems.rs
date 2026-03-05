//! 攝影機系統

use crate::combat::{CombatState, Enemy, LockOnState, WeaponInventory, WeaponType};
use crate::core::{
    CameraSettings, CameraShake, CameraViewMode, CinematicState, GameState, LetterboxBottom,
    LetterboxTop, RecoilState, FOV_LERP_SPEED, LETTERBOX_ANIM_SPEED, LETTERBOX_HEIGHT_RATIO,
};
use crate::player::{CharacterSwitchAnimation, Player, PlayerSprintState};
use crate::vehicle::{Vehicle, VehicleType};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::constants::*;

/// 遊戲攝影機標記
#[derive(Component)]
pub struct GameCamera;

/// 攝影機跟隨系統所需的資源參數包
#[derive(SystemParam)]
pub struct CameraFollowResources<'w> {
    pub game_state: Res<'w, GameState>,
    pub camera_settings: Res<'w, CameraSettings>,
    pub combat_state: Res<'w, CombatState>,
    pub lock_on: Res<'w, LockOnState>,
    pub recoil_state: Res<'w, RecoilState>,
    pub camera_shake: Res<'w, CameraShake>,
    pub switch_anim: Res<'w, CharacterSwitchAnimation>,
    pub time: Res<'w, Time>,
}

// ============================================================================
// 攝影機輸入輔助函數
// ============================================================================
/// 處理鍵盤旋轉（目前未使用，Q/E 改為斜向移動）
#[inline]
fn handle_keyboard_rotation(
    _keyboard: &ButtonInput<KeyCode>,
    _camera_settings: &mut CameraSettings,
    _delta_secs: f32,
) {
    // Q/E 已改為玩家斜向移動，不再旋轉攝影機
    // 保留函數以維持 API 相容性
}

/// 檢查當前武器是否為拳頭（拳頭不能瞄準）
#[inline]
fn check_is_fist_weapon(player_query: &Query<&WeaponInventory, With<Player>>) -> bool {
    player_query
        .single()
        .ok()
        .and_then(|inv| inv.current_weapon())
        .is_some_and(|w| w.stats.weapon_type == WeaponType::Fist)
}

/// 處理滑鼠 Y 軸移動（pitch 或距離調整）
#[inline]
fn handle_mouse_y_axis(camera_settings: &mut CameraSettings, delta_y: f32, is_aiming: bool) {
    if is_aiming {
        // 瞄準時：上下移動 = pitch 俯仰角
        camera_settings.pitch += delta_y * camera_settings.sensitivity;
    } else {
        // 非瞄準時：上下移動 = 調整距離
        camera_settings.distance += delta_y * DISTANCE_MOUSE_FACTOR;
        camera_settings.distance = camera_settings.distance.clamp(DISTANCE_MIN, DISTANCE_MAX);
    }
}

/// 處理滑鼠移動事件
#[inline]
fn handle_mouse_motion(
    mouse_motion: &mut MessageReader<bevy::input::mouse::MouseMotion>,
    camera_settings: &mut CameraSettings,
    is_aiming: bool,
    both_mouse_buttons: bool,
) {
    for event in mouse_motion.read() {
        // 左右移動 = yaw 旋轉（始終有效）
        camera_settings.yaw -= event.delta.x * camera_settings.sensitivity;

        // 雙鍵模式（直走）時，不處理 Y 軸
        if !both_mouse_buttons {
            handle_mouse_y_axis(camera_settings, event.delta.y, is_aiming);
        }
    }
}

/// 攝影機輸入
pub fn camera_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
    mut camera_settings: ResMut<CameraSettings>,
    mut combat_state: ResMut<CombatState>,
    game_state: Res<GameState>,
    time: Res<Time>,
    player_query: Query<&WeaponInventory, With<Player>>,
    switch_anim: Res<CharacterSwitchAnimation>,
) {
    // 角色切換動畫期間禁止攝影機輸入
    if switch_anim.is_active() {
        mouse_motion.clear();
        scroll.clear();
        return;
    }

    // 電影模式下跳過一般輸入（由 cinematic_camera_system 處理）
    if camera_settings.view_mode == CameraViewMode::Cinematic {
        mouse_motion.clear();
        scroll.clear();
        return;
    }

    // 下車時自動切回 TPS（防止步行時卡在車內視角）
    if !game_state.player_in_vehicle && camera_settings.view_mode == CameraViewMode::VehicleInterior
    {
        camera_settings.view_mode = CameraViewMode::ThirdPerson;
    }

    // V 鍵切換視角
    // 車上：ThirdPerson ↔ VehicleInterior
    // 步行：ThirdPerson ↔ FirstPerson
    if keyboard.just_pressed(KeyCode::KeyV) {
        if game_state.player_in_vehicle {
            if camera_settings.view_mode == CameraViewMode::VehicleInterior {
                camera_settings.view_mode = CameraViewMode::ThirdPerson;
            } else {
                // 進入車內視角時重置相對 yaw/pitch
                camera_settings.vehicle_interior_yaw = 0.0;
                camera_settings.vehicle_interior_pitch = 0.0;
                camera_settings.view_mode = CameraViewMode::VehicleInterior;
            }
        } else {
            camera_settings.view_mode = match camera_settings.view_mode {
                CameraViewMode::FirstPerson => CameraViewMode::ThirdPerson,
                _ => CameraViewMode::FirstPerson,
            };
        }
    }

    // 鍵盤旋轉
    handle_keyboard_rotation(&keyboard, &mut camera_settings, time.delta_secs());

    // 檢查武器和瞄準狀態
    let is_fist = check_is_fist_weapon(&player_query);
    let is_aiming = !is_fist && mouse_button.pressed(MouseButton::Right);
    combat_state.is_aiming = is_aiming;

    // 滑鼠按鍵狀態
    let left_pressed = mouse_button.pressed(MouseButton::Left);
    let right_pressed = mouse_button.pressed(MouseButton::Right);

    let view_mode = camera_settings.view_mode;
    let is_fps = view_mode == CameraViewMode::FirstPerson;
    let is_interior = view_mode == CameraViewMode::VehicleInterior;

    if is_interior {
        // 車內視角：滑鼠直接控制相對 yaw/pitch（不需按住按鍵）
        let sensitivity = camera_settings.sensitivity;
        let yaw_limit = camera_settings.vehicle_interior_yaw_limit;
        for event in mouse_motion.read() {
            camera_settings.vehicle_interior_yaw -= event.delta.x * sensitivity;
            camera_settings.vehicle_interior_pitch += event.delta.y * sensitivity;
        }
        // 限制車內視角範圍
        camera_settings.vehicle_interior_yaw = camera_settings
            .vehicle_interior_yaw
            .clamp(-yaw_limit, yaw_limit);
        camera_settings.vehicle_interior_pitch = camera_settings
            .vehicle_interior_pitch
            .clamp(PITCH_MIN, VEHICLE_INTERIOR_PITCH_MAX);
        scroll.clear();
    } else if is_fps {
        // FPS 模式：滑鼠直接控制 yaw + pitch
        handle_mouse_motion(&mut mouse_motion, &mut camera_settings, true, false);
        camera_settings.pitch = camera_settings
            .pitch
            .clamp(PITCH_MIN, PITCH_MAX_WITH_RECOIL);
        scroll.clear();
    } else {
        // TPS 模式
        if left_pressed || right_pressed {
            handle_mouse_motion(
                &mut mouse_motion,
                &mut camera_settings,
                is_aiming,
                left_pressed && right_pressed,
            );
        } else {
            mouse_motion.clear();
        }
        camera_settings.pitch = camera_settings.pitch.clamp(PITCH_MIN, PITCH_MAX_INPUT);
        for event in scroll.read() {
            camera_settings.distance -= event.y * DISTANCE_SCROLL_STEP;
            camera_settings.distance = camera_settings.distance.clamp(DISTANCE_MIN, DISTANCE_MAX);
        }
    }
}

/// 根據車種回傳駕駛座眼睛偏移（車輛本地座標）
pub fn driver_eye_offset(vehicle_type: VehicleType) -> Vec3 {
    match vehicle_type {
        // 機車：騎士坐姿較高、略微偏後
        VehicleType::Scooter => Vec3::new(0.0, 1.4, 0.1),
        // 汽車/計程車：左駕座、略低
        VehicleType::Car | VehicleType::Taxi => Vec3::new(-0.5, 1.2, 0.4),
        // 公車：駕駛座高、偏左、偏前
        VehicleType::Bus => Vec3::new(-0.8, 2.8, 1.8),
    }
}

/// 攝影機跟隨（支援過肩瞄準模式、後座力、震動、鎖定追蹤、車內視角）
#[allow(clippy::type_complexity, clippy::too_many_lines)]
pub fn camera_follow(
    res: CameraFollowResources,
    player_query: Query<&Transform, (With<Player>, Without<GameCamera>, Without<Vehicle>)>,
    vehicle_query: Query<(&Transform, &Vehicle), (Without<GameCamera>, Without<Player>)>,
    mut camera_query: Query<&mut Transform, With<GameCamera>>,
    enemy_query: Query<
        &Transform,
        (
            With<Enemy>,
            Without<GameCamera>,
            Without<Player>,
            Without<Vehicle>,
        ),
    >,
) {
    // 角色切換動畫期間由動畫系統控制攝影機
    if res.switch_anim.is_active() {
        return;
    }

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let target_pos = if res.game_state.player_in_vehicle {
        if let Some(vehicle_entity) = res.game_state.current_vehicle {
            vehicle_query
                .get(vehicle_entity)
                .map(|(t, _)| t.translation)
                .unwrap_or(Vec3::ZERO)
        } else {
            return;
        }
    } else {
        player_query
            .single()
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO)
    };

    // ===== 車內視角 =====
    if res.camera_settings.view_mode == CameraViewMode::VehicleInterior {
        if let Some(vehicle_entity) = res.game_state.current_vehicle {
            if let Ok((vehicle_transform, vehicle)) = vehicle_query.get(vehicle_entity) {
                let eye_offset = driver_eye_offset(vehicle.vehicle_type);

                // 將本地偏移轉換為世界座標（跟隨車輛旋轉）
                let world_offset = vehicle_transform.rotation * eye_offset;
                let eye_pos = vehicle_transform.translation + world_offset;

                let shake_offset = res.camera_shake.get_offset(res.time.elapsed_secs());
                camera_transform.translation = eye_pos + shake_offset;

                // 注視方向：車輛前方 + 玩家相對 yaw/pitch 偏移
                let interior_yaw = res.camera_settings.vehicle_interior_yaw;
                let interior_pitch = res.camera_settings.vehicle_interior_pitch;

                // 以車輛前方為基準旋轉
                let local_look = Vec3::new(
                    -interior_yaw.sin() * interior_pitch.cos(),
                    -interior_pitch.sin(),
                    -interior_yaw.cos() * interior_pitch.cos(),
                );
                let world_look = vehicle_transform.rotation * local_look;
                let look_at = camera_transform.translation + world_look * 10.0;
                camera_transform.look_at(look_at, Vec3::Y);
            }
        }
        return;
    }

    // ===== FPS 模式 =====
    if res.camera_settings.view_mode == CameraViewMode::FirstPerson {
        let yaw = res.camera_settings.yaw + res.recoil_state.current_offset.x;
        let pitch = (res.camera_settings.pitch + res.recoil_state.current_offset.y)
            .clamp(PITCH_MIN, PITCH_MAX_WITH_RECOIL);

        // 攝影機位於角色眼睛位置
        let eye_pos = target_pos + Vec3::Y * res.camera_settings.fps_eye_height;
        let shake_offset = res.camera_shake.get_offset(res.time.elapsed_secs());

        camera_transform.translation = eye_pos + shake_offset;

        // 計算注視方向（yaw + pitch）
        let look_dir = Vec3::new(
            -yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        let look_at = camera_transform.translation + look_dir * 10.0;
        camera_transform.look_at(look_at, Vec3::Y);
        return;
    }

    // ===== TPS 模式（以下為原有邏輯）=====

    // 應用後座力偏移到 yaw 和 pitch
    let yaw = res.camera_settings.yaw + res.recoil_state.current_offset.x;
    let base_pitch = res.camera_settings.pitch;

    // 瞄準模式：拉近攝影機、過肩偏移
    // 瞄準時 pitch 由玩家滑鼠控制，非瞄準時使用預設值
    let is_aiming = res.combat_state.is_aiming && !res.game_state.player_in_vehicle;
    let (distance, pitch, shoulder_offset) = if is_aiming {
        (
            res.camera_settings.aim_distance,
            base_pitch + res.recoil_state.current_offset.y, // 後座力向上偏移
            res.camera_settings.aim_shoulder_offset,
        )
    } else {
        (
            res.camera_settings.distance,
            base_pitch + res.recoil_state.current_offset.y * TPS_RECOIL_FACTOR,
            0.0,
        )
    };

    // 限制後座力影響後的 pitch 範圍
    let pitch = pitch.clamp(PITCH_MIN, PITCH_MAX_WITH_RECOIL);

    // 計算攝影機偏移（後方 + 上方）
    let offset = Vec3::new(
        distance * pitch.cos() * yaw.sin(),
        distance * pitch.sin(),
        distance * pitch.cos() * yaw.cos(),
    );

    // 過肩偏移（向右肩移動）
    let right = Vec3::new(-yaw.cos(), 0.0, yaw.sin()); // 攝影機右方向量
    let shoulder = right * shoulder_offset;

    // 攝影機震動偏移
    let shake_offset = res.camera_shake.get_offset(res.time.elapsed_secs());

    let desired_pos = target_pos + offset + shoulder + shake_offset;

    // 瞄準時使用更快的插值（更緊跟）
    let lerp_speed = if is_aiming {
        AIM_FOLLOW_LERP_SPEED
    } else {
        NORMAL_FOLLOW_LERP_SPEED
    };
    camera_transform.translation = camera_transform
        .translation
        .lerp(desired_pos, lerp_speed * res.time.delta_secs());

    // 瞄準時看向角色前方（考慮 pitch 俯仰角）
    let look_target = if is_aiming {
        // 計算瞄準方向（考慮 yaw 和 pitch）
        let aim_forward = Vec3::new(
            -yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        let mut look = target_pos + Vec3::Y * AIM_LOOK_TARGET_Y_OFFSET + aim_forward * 10.0;

        // 鎖定目標時微調攝影機看向目標（混合追蹤感）
        if let Some(locked_entity) = res.lock_on.locked_target {
            if let Ok(locked_transform) = enemy_query.get(locked_entity) {
                let locked_center = locked_transform.translation + Vec3::Y * LOCK_ON_Y_OFFSET;
                look = look.lerp(locked_center, LOCK_ON_LOOK_BLEND);
            }
        }
        look
    } else {
        target_pos
    };
    camera_transform.look_at(look_target, Vec3::Y);
}

/// 後座力和攝影機震動更新系統
pub fn recoil_and_shake_update_system(
    time: Res<Time>,
    mut recoil_state: ResMut<RecoilState>,
    mut camera_shake: ResMut<CameraShake>,
    player_query: Query<&WeaponInventory, With<Player>>,
) {
    let dt = time.delta_secs();

    // 更新攝影機震動
    camera_shake.update(dt);

    // 取得當前武器的後座力恢復速度
    let recovery_rate = if let Ok(inventory) = player_query.single() {
        inventory
            .current_weapon()
            .map_or(5.0, |w| w.stats.recoil_recovery)
    } else {
        5.0
    };

    // 更新後座力恢復
    recoil_state.update_recovery(recovery_rate, dt);
}

/// 動態 FOV 系統（衝刺 → 擴大 FOV，瞄準 → 縮小 FOV）
pub fn dynamic_fov_system(
    mut camera_settings: ResMut<CameraSettings>,
    combat_state: Res<CombatState>,
    cinematic: Res<CinematicState>,
    sprint_query: Query<&PlayerSprintState, With<Player>>,
    weapon_query: Query<&WeaponInventory, With<Player>>,
    mut camera_query: Query<&mut Projection, With<GameCamera>>,
    time: Res<Time>,
) {
    // 檢查當前武器是否為狙擊槍（瞄準時使用 scope FOV）
    let is_scoped = combat_state.is_aiming
        && weapon_query
            .single()
            .ok()
            .and_then(|inv| inv.current_weapon())
            .is_some_and(|w| w.stats.weapon_type == WeaponType::SniperRifle);

    // 決定目標 FOV：電影 > 車內 > FPS > 狙擊鏡 > 瞄準 > 衝刺 > 預設
    let target_fov = if camera_settings.view_mode == CameraViewMode::Cinematic {
        cinematic.fov
    } else if camera_settings.view_mode == CameraViewMode::VehicleInterior {
        camera_settings.vehicle_interior_fov
    } else if camera_settings.view_mode == CameraViewMode::FirstPerson {
        camera_settings.fps_fov
    } else if is_scoped {
        camera_settings.scope_fov
    } else if combat_state.is_aiming {
        camera_settings.aim_fov
    } else if sprint_query
        .single()
        .ok()
        .is_some_and(|s| s.state.is_sprint_related())
    {
        // 根據衝刺進度混合 FOV（加速中漸增、減速中漸減）
        let sprint_progress = sprint_query
            .single()
            .ok()
            .map_or(0.0, |s| s.state.animation_blend());
        camera_settings.base_fov
            + (camera_settings.sprint_fov - camera_settings.base_fov) * sprint_progress
    } else {
        camera_settings.base_fov
    };

    // 平滑插值
    let dt = time.delta_secs();
    let lerp_t = (FOV_LERP_SPEED * dt).min(1.0);
    camera_settings.current_fov += (target_fov - camera_settings.current_fov) * lerp_t;

    // 套用到攝影機投影
    let fov_radians = camera_settings.current_fov.to_radians();
    for mut projection in &mut camera_query {
        if let Projection::Perspective(ref mut persp) = *projection {
            persp.fov = fov_radians;
        }
    }
}

/// 攝影機自動跟隨玩家方向（GTA 風格）
/// 當玩家移動時，攝影機會自動旋轉到玩家背後
#[allow(clippy::too_many_arguments)]
pub fn camera_auto_follow(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    game_state: Res<GameState>,
    combat_state: Res<CombatState>,
    mut camera_settings: ResMut<CameraSettings>,
    player_query: Query<&Transform, With<Player>>,
    time: Res<Time>,
    switch_anim: Res<CharacterSwitchAnimation>,
) {
    // 角色切換動畫期間不自動跟隨
    if switch_anim.is_active() {
        return;
    }

    // 不跟隨的情況：
    // 1. 在車上（車輛有自己的攝影機邏輯）
    // 2. 瞄準模式（需要自由控制攝影機）
    // 3. FPS 模式（玩家完全控制視角）
    // 4. 手動旋轉攝影機（Q/E 或滑鼠按住）
    if game_state.player_in_vehicle
        || combat_state.is_aiming
        || camera_settings.view_mode == CameraViewMode::FirstPerson
    {
        return;
    }

    // 如果正在手動旋轉，不自動跟隨
    let manual_rotation = keyboard.pressed(KeyCode::KeyQ)
        || keyboard.pressed(KeyCode::KeyE)
        || mouse_button.pressed(MouseButton::Left)
        || mouse_button.pressed(MouseButton::Right);

    if manual_rotation {
        return;
    }

    // 檢查移動按鍵
    let moving_forward = keyboard.pressed(KeyCode::KeyW);
    let moving_backward = keyboard.pressed(KeyCode::KeyS);
    let moving_left = keyboard.pressed(KeyCode::KeyA);
    let moving_right = keyboard.pressed(KeyCode::KeyD);

    // 只有「轉向前進」時才自動跟隨（W+A 或 W+D）
    // 純粹前進（只按 W）不跟隨，避免反饋循環
    let is_turning = moving_forward && (moving_left || moving_right) && !moving_backward;

    if !is_turning {
        return;
    }

    // 取得玩家當前朝向
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    // 從玩家的 rotation 計算目標 yaw（攝影機應該在玩家背後）
    // EulerRot::YXZ 返回 (yaw, pitch, roll)，取第一個元素
    // 玩家旋轉 = 移動方向 yaw + PI，所以攝影機 yaw = player_yaw - PI
    let (player_yaw, _, _) = player_transform.rotation.to_euler(EulerRot::YXZ);
    let target_yaw = player_yaw - std::f32::consts::PI;

    // 計算角度差（考慮角度繞圈）
    let mut angle_diff = target_yaw - camera_settings.yaw;

    // 正規化到 -PI ~ PI 範圍（O(1)，避免極端值的多次迭代）
    angle_diff = (angle_diff + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;

    // 平滑插值
    let dt = time.delta_secs();
    camera_settings.yaw += angle_diff * CAMERA_FOLLOW_SPEED * dt;
}

// ============================================================================
// 電影模式系統
// ============================================================================

/// 電影模式輸入與自由攝影
/// C 鍵切換進入/退出電影模式。模式中 WASD 飛行、滑鼠旋轉、滾輪調速。
pub fn cinematic_camera_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
    mut camera_settings: ResMut<CameraSettings>,
    mut cinematic: ResMut<CinematicState>,
    mut camera_query: Query<&mut Transform, With<GameCamera>>,
    time: Res<Time>,
) {
    // C 鍵切換電影模式
    if keyboard.just_pressed(KeyCode::KeyC) {
        if camera_settings.view_mode == CameraViewMode::Cinematic {
            camera_settings.view_mode = CameraViewMode::ThirdPerson;
        } else {
            camera_settings.view_mode = CameraViewMode::Cinematic;
        }
    }

    // 非電影模式時不處理
    if camera_settings.view_mode != CameraViewMode::Cinematic {
        mouse_motion.clear();
        scroll.clear();
        return;
    }

    let Ok(mut cam_tf) = camera_query.single_mut() else {
        mouse_motion.clear();
        scroll.clear();
        return;
    };

    let dt = time.delta_secs();
    let sensitivity = camera_settings.sensitivity;

    // 滑鼠旋轉
    for event in mouse_motion.read() {
        camera_settings.yaw -= event.delta.x * sensitivity;
        camera_settings.pitch += event.delta.y * sensitivity;
    }
    camera_settings.pitch = camera_settings
        .pitch
        .clamp(-CINEMATIC_PITCH_LIMIT, CINEMATIC_PITCH_LIMIT);

    // 滾輪調整飛行速度
    for event in scroll.read() {
        cinematic.fly_speed = (cinematic.fly_speed + event.y * 2.0).clamp(1.0, 100.0);
    }

    // WASD + QE 飛行
    let yaw = camera_settings.yaw;
    let pitch = camera_settings.pitch;

    // 攝影機前方向量（考慮 pitch，可以向上/下飛行）
    let forward = Vec3::new(
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        -yaw.cos() * pitch.cos(),
    );
    // 水平右方向量
    let right = Vec3::new(-yaw.cos(), 0.0, yaw.sin());
    let up = Vec3::Y;

    let mut move_dir = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        move_dir += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        move_dir -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        move_dir -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        move_dir += right;
    }
    if keyboard.pressed(KeyCode::KeyE) {
        move_dir += up;
    }
    if keyboard.pressed(KeyCode::KeyQ) {
        move_dir -= up;
    }

    // Shift 加速
    let speed_mult = if keyboard.pressed(KeyCode::ShiftLeft) {
        3.0
    } else {
        1.0
    };

    if move_dir.length_squared() > 0.0 {
        move_dir = move_dir.normalize();
        cam_tf.translation += move_dir * cinematic.fly_speed * speed_mult * dt;
    }

    // 更新攝影機旋轉
    let look_at = cam_tf.translation + forward * 10.0;
    cam_tf.look_at(look_at, Vec3::Y);
}

/// Letterbox 動畫系統
/// 電影模式啟動時展開黑邊，退出時收起。
pub fn cinematic_letterbox_system(
    camera_settings: Res<CameraSettings>,
    mut cinematic: ResMut<CinematicState>,
    mut top_query: Query<&mut Node, (With<LetterboxTop>, Without<LetterboxBottom>)>,
    mut bottom_query: Query<&mut Node, (With<LetterboxBottom>, Without<LetterboxTop>)>,
    time: Res<Time>,
) {
    let is_cinematic = camera_settings.view_mode == CameraViewMode::Cinematic;
    let dt = time.delta_secs();

    // 動畫進度趨向目標
    let target = if is_cinematic { 1.0 } else { 0.0 };
    let speed = LETTERBOX_ANIM_SPEED * dt;
    if cinematic.letterbox_progress < target {
        cinematic.letterbox_progress = (cinematic.letterbox_progress + speed).min(1.0);
    } else if cinematic.letterbox_progress > target {
        cinematic.letterbox_progress = (cinematic.letterbox_progress - speed).max(0.0);
    }

    // 更新黑邊高度
    let bar_height = Val::Percent(LETTERBOX_HEIGHT_RATIO * 100.0 * cinematic.letterbox_progress);
    for mut node in &mut top_query {
        node.height = bar_height;
    }
    for mut node in &mut bottom_query {
        node.height = bar_height;
    }
}

/// 設置 Letterbox UI 節點（在 setup 階段呼叫一次）
pub fn setup_cinematic_letterbox(mut commands: Commands) {
    // 上方黑邊
    commands.spawn((
        LetterboxTop,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        ZIndex(100), // 確保在最上層
    ));

    // 下方黑邊
    commands.spawn((
        LetterboxBottom,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        ZIndex(100),
    ));
}

/// 電影模式 HUD 隱藏系統
/// 在電影模式中隱藏所有 HUD 元素（letterbox 進度 > 0.5 時開始隱藏）
pub fn cinematic_hud_toggle_system(
    cinematic: Res<CinematicState>,
    mut hud_query: Query<&mut Visibility, With<crate::ui::PlayerStatusContainer>>,
) {
    let should_hide = cinematic.letterbox_progress > 0.5;
    let target = if should_hide {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut vis in &mut hud_query {
        *vis = target;
    }
}
