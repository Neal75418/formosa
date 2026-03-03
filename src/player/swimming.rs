//! 玩家游泳系統
//!
//! 偵測玩家入水後切換到游泳模式：水面移動、潛水、快游、體力消耗、溺水。
//! 參考 `pedestrian/swimming.rs` 的 NPC 游泳邏輯。

use bevy::prelude::*;
use bevy_rapier3d::prelude::KinematicCharacterController;

use super::components::{Player, Stamina};
use crate::combat::{DamageEvent, DamageSource};
use crate::vehicle::watercraft::WATER_LEVEL;

// ============================================================================
// 常數
// ============================================================================

/// 入水偵測門檻（玩家 Y < WATER_LEVEL + 此值 → 入水）
const WATER_ENTER_THRESHOLD: f32 = 0.5;
/// 出水偵測門檻（玩家 Y > WATER_LEVEL + 此值 → 出水）
const WATER_EXIT_THRESHOLD: f32 = 1.0;
/// 水面游泳高度（頭部露出水面）
const SWIM_SURFACE_HEIGHT: f32 = 0.3;
/// 游泳速度（m/s）
const SWIM_SPEED: f32 = 4.0;
/// 快游速度（m/s，Shift 加速）
const FAST_SWIM_SPEED: f32 = 7.0;
/// 游泳體力消耗（每秒）
const SWIM_STAMINA_DRAIN: f32 = 3.0;
/// 快游體力消耗（每秒）
const FAST_SWIM_STAMINA_DRAIN: f32 = 8.0;
/// 最大潛水深度
const MAX_DIVE_DEPTH: f32 = 5.0;
/// 最大憋氣時間（秒）
const MAX_BREATH: f32 = 15.0;
/// 潛水時每秒自傷（憋氣耗盡後）
const DROWN_DAMAGE_PER_SEC: f32 = 10.0;
/// 溺水下沉速度（m/s）
const DROWNING_SINK_SPEED: f32 = 0.5;
/// 上浮/下潛速度
const VERTICAL_SWIM_SPEED: f32 = 3.0;

// ============================================================================
// 組件
// ============================================================================

/// 玩家游泳狀態
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerSwimState {
    #[default]
    OnLand,
    Swimming,
    Drowning,
}

/// 玩家游泳組件（入水時插入，出水時移除）
#[derive(Component, Debug)]
pub struct PlayerSwimming {
    pub state: PlayerSwimState,
    /// 當前潛水深度（負值 = 水下）
    pub dive_depth: f32,
    /// 憋氣計時器（潛水時遞減）
    pub breath_timer: f32,
}

impl Default for PlayerSwimming {
    fn default() -> Self {
        Self {
            state: PlayerSwimState::Swimming,
            dive_depth: 0.0,
            breath_timer: MAX_BREATH,
        }
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 偵測玩家是否入水/出水，插入或移除 PlayerSwimming 組件
pub fn player_water_detection_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<Player>>,
    swimming_query: Query<&PlayerSwimming>,
) {
    let Ok((entity, transform)) = query.single() else {
        return;
    };
    let y = transform.translation.y;
    let has_swimming = swimming_query.get(entity).is_ok();

    if y < WATER_LEVEL + WATER_ENTER_THRESHOLD && !has_swimming {
        // 入水：插入游泳組件
        commands.entity(entity).insert(PlayerSwimming::default());
    } else if y > WATER_LEVEL + WATER_EXIT_THRESHOLD && has_swimming {
        // 出水：移除游泳組件
        commands.entity(entity).remove::<PlayerSwimming>();
    }
}

/// 玩家游泳移動系統
///
/// 水中 WASD 控制方向，Space 上浮，Ctrl 下潛，Shift 快游。
/// 體力耗盡時進入 Drowning 狀態並下沉。
pub fn player_swim_movement_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    camera_settings: Res<crate::core::CameraSettings>,
    mut query: Query<(
        &mut Transform,
        &mut Player,
        &mut PlayerSwimming,
        &mut Stamina,
        &mut KinematicCharacterController,
    )>,
) {
    let Ok((mut transform, mut player, mut swimming, mut stamina, mut controller)) =
        query.single_mut()
    else {
        return;
    };
    let dt = time.delta_secs();

    // 確保 grounded 為 false（水中不在地面）
    player.is_grounded = false;

    match swimming.state {
        PlayerSwimState::OnLand => {} // 不應進入此分支
        PlayerSwimState::Swimming => {
            // 輸入方向
            let mut input = Vec3::ZERO;
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
            let input = input.normalize_or_zero();

            // 快游
            let is_fast = keyboard.pressed(KeyCode::ShiftLeft);
            let speed = if is_fast { FAST_SWIM_SPEED } else { SWIM_SPEED };
            let drain = if is_fast { FAST_SWIM_STAMINA_DRAIN } else { SWIM_STAMINA_DRAIN };

            // 消耗體力
            if input != Vec3::ZERO || is_fast {
                stamina.current = (stamina.current - drain * dt).max(0.0);
                if stamina.current <= 0.0 {
                    stamina.exhausted = true;
                    swimming.state = PlayerSwimState::Drowning;
                }
            } else {
                // 漂浮時緩慢恢復
                stamina.regenerate(dt);
            }

            // 計算世界空間方向
            let yaw = camera_settings.yaw;
            let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
            let right = Vec3::new(forward.z, 0.0, -forward.x);
            let move_dir = (forward * input.z + right * input.x).normalize_or_zero();

            // 垂直移動
            let mut vertical = 0.0;
            if keyboard.pressed(KeyCode::Space) {
                vertical += VERTICAL_SWIM_SPEED;
            }
            if keyboard.pressed(KeyCode::ControlLeft) {
                vertical -= VERTICAL_SWIM_SPEED;
            }

            // Y 座標限制
            let target_y = transform.translation.y + vertical * dt;
            let clamped_y = target_y.clamp(
                WATER_LEVEL - MAX_DIVE_DEPTH,
                WATER_LEVEL + SWIM_SURFACE_HEIGHT,
            );
            let vertical_displacement = clamped_y - transform.translation.y;
            swimming.dive_depth = WATER_LEVEL - clamped_y;

            // 憋氣（水面下）
            if clamped_y < WATER_LEVEL - 0.2 {
                swimming.breath_timer = (swimming.breath_timer - dt).max(0.0);
            } else {
                // 水面上恢復憋氣
                swimming.breath_timer = (swimming.breath_timer + dt * 3.0).min(MAX_BREATH);
            }

            let movement = move_dir * speed * dt + Vec3::Y * vertical_displacement;
            controller.translation = Some(movement);

            // 面朝移動方向
            if move_dir.length_squared() > 0.01 {
                let target_rot = Quat::from_rotation_y(move_dir.x.atan2(move_dir.z));
                transform.rotation = transform.rotation.slerp(target_rot, 5.0 * dt);
            }
        }
        PlayerSwimState::Drowning => {
            // 溺水：持續下沉 + 造成傷害
            let sink = Vec3::Y * (-DROWNING_SINK_SPEED) * dt;
            let clamped_y = (transform.translation.y + sink.y)
                .max(WATER_LEVEL - MAX_DIVE_DEPTH);
            let actual_sink = Vec3::Y * (clamped_y - transform.translation.y);
            controller.translation = Some(actual_sink);

            // 體力恢復後可以恢復游泳
            stamina.regenerate(dt);
            if stamina.current > stamina.max * Stamina::RECOVERY_THRESHOLD {
                stamina.exhausted = false;
                swimming.state = PlayerSwimState::Swimming;
            }
        }
    }
}

/// 玩家游泳傷害系統（溺水自傷）
pub fn player_swim_damage_system(
    time: Res<Time>,
    query: Query<(Entity, &PlayerSwimming)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok((entity, swimming)) = query.single() else {
        return;
    };
    let dt = time.delta_secs();

    // 溺水時持續自傷（體力耗盡）
    if swimming.state == PlayerSwimState::Drowning {
        damage_events.write(
            DamageEvent::new(entity, DROWN_DAMAGE_PER_SEC * dt, DamageSource::Environment),
        );
    } else if swimming.breath_timer <= 0.0 && swimming.dive_depth > 0.2 {
        // 憋氣耗盡時在水下自傷（與溺水互斥，避免雙重傷害）
        damage_events.write(
            DamageEvent::new(entity, DROWN_DAMAGE_PER_SEC * 0.5 * dt, DamageSource::Environment),
        );
    }
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swim_state_default() {
        let swimming = PlayerSwimming::default();
        assert_eq!(swimming.state, PlayerSwimState::Swimming);
        assert_eq!(swimming.dive_depth, 0.0);
        assert_eq!(swimming.breath_timer, MAX_BREATH);
    }

    #[test]
    fn test_swim_constants() {
        const { assert!(SWIM_SPEED > 0.0) };
        const { assert!(FAST_SWIM_SPEED > SWIM_SPEED) };
        const { assert!(SWIM_STAMINA_DRAIN > 0.0) };
        const { assert!(FAST_SWIM_STAMINA_DRAIN > SWIM_STAMINA_DRAIN) };
        const { assert!(MAX_DIVE_DEPTH > 0.0) };
        const { assert!(MAX_BREATH > 0.0) };
    }

    #[test]
    fn test_water_thresholds() {
        const { assert!(WATER_ENTER_THRESHOLD < WATER_EXIT_THRESHOLD) };
        const { assert!(SWIM_SURFACE_HEIGHT > 0.0) };
    }

    #[test]
    fn test_drown_damage_positive() {
        const { assert!(DROWN_DAMAGE_PER_SEC > 0.0) };
        const { assert!(DROWNING_SINK_SPEED > 0.0) };
    }

    #[test]
    fn test_vertical_swim_speed() {
        const { assert!(VERTICAL_SWIM_SPEED > 0.0) };
    }
}
