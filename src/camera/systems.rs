//! 攝影機系統

// Bevy 系統需要 Res<T> 按值傳遞
#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;
use crate::core::{GameState, CameraSettings, RecoilState, CameraShake};
use crate::player::Player;
use crate::vehicle::Vehicle;
use crate::combat::{CombatState, WeaponInventory, WeaponType};

/// 攝影機自動跟隨速度（越大越快跟上玩家）
const CAMERA_FOLLOW_SPEED: f32 = 3.0;

/// 遊戲攝影機標記
#[derive(Component)]
pub struct GameCamera;

// === 攝影機輸入輔助函數 ===

/// 處理鍵盤旋轉（目前未使用，Q/E 改為斜向移動）
#[inline]
fn handle_keyboard_rotation(_keyboard: &ButtonInput<KeyCode>, _camera_settings: &mut CameraSettings, _delta_secs: f32) {
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
        .map(|w| w.stats.weapon_type == WeaponType::Fist)
        .unwrap_or(false)
}

/// 處理滑鼠 Y 軸移動（pitch 或距離調整）
#[inline]
fn handle_mouse_y_axis(camera_settings: &mut CameraSettings, delta_y: f32, is_aiming: bool) {
    if is_aiming {
        // 瞄準時：上下移動 = pitch 俯仰角
        camera_settings.pitch += delta_y * camera_settings.sensitivity;
    } else {
        // 非瞄準時：上下移動 = 調整距離
        camera_settings.distance += delta_y * 0.1;
        camera_settings.distance = camera_settings.distance.clamp(5.0, 80.0);
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
    time: Res<Time>,
    player_query: Query<&WeaponInventory, With<Player>>,
) {
    // 鍵盤旋轉
    handle_keyboard_rotation(&keyboard, &mut camera_settings, time.delta_secs());

    // 檢查武器和瞄準狀態
    let is_fist = check_is_fist_weapon(&player_query);
    let is_aiming = !is_fist && mouse_button.pressed(MouseButton::Right);
    combat_state.is_aiming = is_aiming;

    // 滑鼠按鍵狀態
    let left_pressed = mouse_button.pressed(MouseButton::Left);
    let right_pressed = mouse_button.pressed(MouseButton::Right);

    if left_pressed || right_pressed {
        handle_mouse_motion(&mut mouse_motion, &mut camera_settings, is_aiming, left_pressed && right_pressed);
    } else {
        mouse_motion.clear();
    }

    // 限制俯仰角範圍
    camera_settings.pitch = camera_settings.pitch.clamp(-0.3, 1.2);

    // 滾輪縮放
    for event in scroll.read() {
        camera_settings.distance -= event.y * 0.4;
        camera_settings.distance = camera_settings.distance.clamp(5.0, 80.0);
    }
}

/// 攝影機跟隨（支援過肩瞄準模式、後座力、震動）
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn camera_follow(
    game_state: Res<GameState>,
    camera_settings: Res<CameraSettings>,
    combat_state: Res<CombatState>,
    recoil_state: Res<RecoilState>,
    camera_shake: Res<CameraShake>,
    player_query: Query<&Transform, (With<Player>, Without<GameCamera>, Without<Vehicle>)>,
    vehicle_query: Query<&Transform, (With<Vehicle>, Without<GameCamera>, Without<Player>)>,
    mut camera_query: Query<&mut Transform, With<GameCamera>>,
    time: Res<Time>,
) {
    let Ok(mut camera_transform) = camera_query.single_mut() else { return; };

    let target_pos = if game_state.player_in_vehicle {
        if let Some(vehicle_entity) = game_state.current_vehicle {
            vehicle_query.get(vehicle_entity).map(|t| t.translation).unwrap_or(Vec3::ZERO)
        } else {
            return;
        }
    } else {
        player_query.single().map(|t| t.translation).unwrap_or(Vec3::ZERO)
    };

    // 應用後座力偏移到 yaw 和 pitch
    let yaw = camera_settings.yaw + recoil_state.current_offset.x;
    let base_pitch = camera_settings.pitch;

    // 瞄準模式：拉近攝影機、過肩偏移
    // 瞄準時 pitch 由玩家滑鼠控制，非瞄準時使用預設值
    let is_aiming = combat_state.is_aiming && !game_state.player_in_vehicle;
    let (distance, pitch, shoulder_offset) = if is_aiming {
        (
            camera_settings.aim_distance,
            base_pitch + recoil_state.current_offset.y, // 後座力向上偏移
            camera_settings.aim_shoulder_offset,
        )
    } else {
        (camera_settings.distance, base_pitch + recoil_state.current_offset.y * 0.3, 0.0)
    };

    // 限制後座力影響後的 pitch 範圍
    let pitch = pitch.clamp(-0.3, 1.5);

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
    let shake_offset = camera_shake.get_offset(time.elapsed_secs());

    let desired_pos = target_pos + offset + shoulder + shake_offset;

    // 瞄準時使用更快的插值（更緊跟）
    let lerp_speed = if is_aiming { 15.0 } else { 8.0 };
    camera_transform.translation = camera_transform.translation.lerp(desired_pos, lerp_speed * time.delta_secs());

    // 瞄準時看向角色前方（考慮 pitch 俯仰角）
    let look_target = if is_aiming {
        // 計算瞄準方向（考慮 yaw 和 pitch）
        let aim_forward = Vec3::new(
            -yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        target_pos + Vec3::Y * 1.5 + aim_forward * 10.0
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
            .map(|w| w.stats.recoil_recovery)
            .unwrap_or(5.0)
    } else {
        5.0
    };

    // 更新後座力恢復
    recoil_state.update_recovery(recovery_rate, dt);
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
) {
    // 不跟隨的情況：
    // 1. 在車上（車輛有自己的攝影機邏輯）
    // 2. 瞄準模式（需要自由控制攝影機）
    // 3. 手動旋轉攝影機（Q/E 或滑鼠按住）
    if game_state.player_in_vehicle || combat_state.is_aiming {
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

    // 正規化到 -PI ~ PI 範圍
    while angle_diff > std::f32::consts::PI {
        angle_diff -= std::f32::consts::TAU;
    }
    while angle_diff < -std::f32::consts::PI {
        angle_diff += std::f32::consts::TAU;
    }

    // 平滑插值
    let dt = time.delta_secs();
    camera_settings.yaw += angle_diff * CAMERA_FOLLOW_SPEED * dt;
}
