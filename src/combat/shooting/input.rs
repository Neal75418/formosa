//! 射擊輸入與換彈系統
//!
//! 處理玩家射擊輸入、武器切換、冷卻、換彈邏輯。

use bevy::prelude::*;

use crate::combat::components::*;
use crate::combat::weapon::*;
use crate::combat::RespawnState;
use crate::audio::{
    play_reload_sound, play_weapon_switch_sound, AudioManager, WeaponSounds,
};
use crate::core::GameState;
use crate::player::{Player, PlayerSkills};
use crate::ui::NotificationQueue;

/// 射擊輸入收集系統
pub fn shooting_input_system(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut input: ResMut<ShootingInput>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    player_query: Query<&WeaponInventory, With<Player>>,
) {
    // 死亡時或在車上時不處理射擊輸入
    if respawn_state.is_dead || game_state.player_in_vehicle {
        input.is_fire_pressed = false;
        input.is_fire_held = false;
        input.is_aim_pressed = false;
        input.is_block_pressed = false;
        input.is_reload_pressed = false;
        input.weapon_switch = None;
        input.mouse_wheel = 0.0;
        return;
    }

    // 檢查當前武器是否為近戰武器
    let is_melee = player_query
        .single()
        .ok()
        .and_then(|inv| inv.current_weapon())
        .map(|w| w.stats.weapon_type.is_melee())
        .unwrap_or(false);

    // 射擊：R 鍵（與 UI 提示一致）
    input.is_fire_pressed = keyboard.just_pressed(KeyCode::KeyR);
    input.is_fire_held = keyboard.pressed(KeyCode::KeyR);
    // 近戰武器狀態下不啟用瞄準模式（沒有準星）
    input.is_aim_pressed = !is_melee && mouse.pressed(MouseButton::Right);
    // 近戰武器右鍵 = 格擋
    input.is_block_pressed = is_melee && mouse.pressed(MouseButton::Right);
    // 換彈：T 鍵
    input.is_reload_pressed = keyboard.just_pressed(KeyCode::KeyT);

    // 武器切換 (1-4 數字鍵)
    input.weapon_switch = None;
    if keyboard.just_pressed(KeyCode::Digit1) {
        input.weapon_switch = Some(1);
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        input.weapon_switch = Some(2);
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        input.weapon_switch = Some(3);
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        input.weapon_switch = Some(4);
    }

    // 滑鼠滾輪切換武器
    input.mouse_wheel = 0.0;
    for event in mouse_wheel.read() {
        input.mouse_wheel += event.y;
    }
}

/// 武器冷卻計時系統
pub fn weapon_cooldown_system(
    time: Res<Time>,
    mut player_query: Query<&mut WeaponInventory, With<Player>>,
) {
    let dt = time.delta_secs();

    for mut inventory in player_query.iter_mut() {
        if let Some(weapon) = inventory.current_weapon_mut() {
            if weapon.fire_cooldown > 0.0 {
                weapon.fire_cooldown = (weapon.fire_cooldown - dt).max(0.0);
            }
        }
    }
}

/// 連擊窗口超時重置系統
///
/// 當距離上次近戰命中超過 COMBO_WINDOW 秒時，重置連擊鏈。
/// 切換到非近戰武器時也會重置。
pub fn melee_combo_timeout_system(
    time: Res<Time>,
    mut combo: ResMut<MeleeComboState>,
    player_query: Query<&WeaponInventory, With<Player>>,
) {
    if !combo.active {
        return;
    }

    let current_time = time.elapsed_secs();

    // 超時重置
    if (current_time - combo.last_hit_time) > COMBO_WINDOW {
        combo.reset();
        return;
    }

    // 切換到非近戰武器時重置
    for inventory in player_query.iter() {
        if let Some(weapon) = inventory.current_weapon() {
            if !weapon.stats.weapon_type.is_melee() {
                combo.reset();
            }
        }
    }
}

/// 顯示武器切換通知
fn notify_weapon_switch(notifications: &mut NotificationQueue, inventory: &WeaponInventory) {
    if let Some(weapon) = inventory.current_weapon() {
        notifications.info(format!(
            "{} {}",
            weapon.stats.weapon_type.icon(),
            weapon.stats.weapon_type.name()
        ));
    }
}

/// 處理武器切換邏輯
fn handle_weapon_switch(
    input: &ShootingInput,
    inventory: &mut WeaponInventory,
    notifications: &mut NotificationQueue,
) -> bool {
    // 數字鍵切換（1-4）
    if let Some(slot) = input.weapon_switch {
        inventory.select_weapon(slot);
        notify_weapon_switch(notifications, inventory);
        return true;
    }
    false
}

/// 檢查是否應該切換武器
#[inline]
fn should_switch_weapon(input: &ShootingInput) -> bool {
    input.weapon_switch.is_some()
}

/// 切換武器時取消換彈
fn cancel_reload_on_switch(weapon: &mut Weapon, notifications: &mut NotificationQueue) {
    if weapon.is_reloading {
        weapon.cancel_reload();
        notifications.warning("換彈取消");
    }
}

/// 更新換彈進度，返回是否完成換彈
fn update_reload_progress(weapon: &mut Weapon, dt: f32) -> bool {
    if !weapon.is_reloading {
        return false;
    }
    weapon.reload_timer -= dt;
    weapon.reload_timer <= 0.0
}

/// 嘗試開始換彈，返回是否成功開始
fn try_start_reload(
    weapon: &mut Weapon,
    is_reload_pressed: bool,
    reload_speed_multiplier: f32,
    notifications: &mut NotificationQueue,
) -> bool {
    if is_reload_pressed && weapon.start_reload(reload_speed_multiplier) {
        notifications.info("換彈中...");
        return true;
    }
    if weapon.needs_reload() && weapon.start_reload(reload_speed_multiplier) {
        notifications.warning("彈匣空了！換彈中...");
        return true;
    }
    false
}

/// 播放換彈音效（如果可用）
#[inline]
fn play_reload_sound_if_available(
    commands: &mut Commands,
    weapon_sounds: &Option<Res<WeaponSounds>>,
    audio_manager: &AudioManager,
    is_complete: bool,
) {
    if let Some(ref sounds) = weapon_sounds {
        play_reload_sound(commands, sounds, audio_manager, is_complete);
    }
}

/// 播放武器切換音效（如果可用）
#[inline]
fn play_switch_sound_if_available(
    commands: &mut Commands,
    weapon_sounds: &Option<Res<WeaponSounds>>,
    audio_manager: &AudioManager,
) {
    if let Some(ref sounds) = weapon_sounds {
        play_weapon_switch_sound(commands, sounds, audio_manager);
    }
}

/// 換彈系統
#[allow(clippy::too_many_arguments)]
pub fn reload_system(
    time: Res<Time>,
    input: Res<ShootingInput>,
    mut commands: Commands,
    weapon_sounds: Option<Res<WeaponSounds>>,
    audio_manager: Res<AudioManager>,
    skills: Res<PlayerSkills>,
    mut player_query: Query<&mut WeaponInventory, With<Player>>,
    mut notifications: ResMut<NotificationQueue>,
) {
    let dt = time.delta_secs();

    for mut inventory in player_query.iter_mut() {
        // 切換武器前取消換彈
        if should_switch_weapon(&input) {
            if let Some(weapon) = inventory.current_weapon_mut() {
                cancel_reload_on_switch(weapon, &mut notifications);
            }
            handle_weapon_switch(&input, &mut inventory, &mut notifications);
            play_switch_sound_if_available(&mut commands, &weapon_sounds, &audio_manager);
            continue;
        }

        let Some(weapon) = inventory.current_weapon_mut() else {
            continue;
        };

        // 更新換彈進度
        if update_reload_progress(weapon, dt) {
            weapon.finish_reload();
            notifications.success("換彈完成");
            play_reload_sound_if_available(&mut commands, &weapon_sounds, &audio_manager, true);
            continue;
        }

        // 跳過正在換彈的武器
        if weapon.is_reloading {
            continue;
        }

        // 嘗試開始換彈（應用射擊技能加成）
        if try_start_reload(weapon, input.is_reload_pressed, skills.reload_speed_multiplier(), &mut notifications) {
            play_reload_sound_if_available(&mut commands, &weapon_sounds, &audio_manager, false);
        }
    }
}

/// 格擋狀態更新系統
///
/// 根據 `ShootingInput.is_block_pressed` 更新 `BlockState`，
/// 並處理反擊加成超時失效。
pub fn block_update_system(
    time: Res<Time>,
    input: Res<ShootingInput>,
    mut block_state: ResMut<BlockState>,
) {
    let current_time = time.elapsed_secs();

    if input.is_block_pressed {
        block_state.start_block(current_time);
    } else {
        block_state.stop_block();
    }

    block_state.update_counter_timeout(current_time);
}
