//! 玩家游泳系統
//!
//! 偵測玩家入水後切換到游泳模式：水面移動、潛水、快游、體力消耗、溺水。
//! 參考 `pedestrian/swimming.rs` 的 NPC 游泳邏輯。

use bevy::prelude::*;
use bevy_rapier3d::prelude::KinematicCharacterController;

use super::components::{Player, Stamina, VehicleTransitionState};
use crate::combat::{DamageEvent, DamageSource};
use crate::core::GameState;
use crate::vehicle::watercraft::WATER_LEVEL;

// ============================================================================
// 常數
// ============================================================================

/// 入水偵測門檻（玩家 Y < `WATER_LEVEL` + 此值 → 入水）
const WATER_ENTER_THRESHOLD: f32 = 0.5;
/// 出水偵測門檻（玩家 Y > `WATER_LEVEL` + 此值 → 出水）
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

/// 偵測玩家是否入水/出水，插入或移除 `PlayerSwimming` 組件
pub fn player_water_detection_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    transition: Res<VehicleTransitionState>,
    query: Query<(Entity, &Transform), With<Player>>,
    swimming_query: Query<&PlayerSwimming>,
) {
    // 上下車途中和在車上，玩家高度會被設成車的高度（停著的機車車心只有 0.4），不是在水裡
    if game_state.player_in_vehicle || transition.is_animating() {
        return;
    }
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
            let drain = if is_fast {
                FAST_SWIM_STAMINA_DRAIN
            } else {
                SWIM_STAMINA_DRAIN
            };

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
            let clamped_y = (transform.translation.y + sink.y).max(WATER_LEVEL - MAX_DIVE_DEPTH);
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
        damage_events.write(DamageEvent::new(
            entity,
            DROWN_DAMAGE_PER_SEC * dt,
            DamageSource::Environment,
        ));
    } else if swimming.breath_timer <= 0.0 && swimming.dive_depth > 0.2 {
        // 憋氣耗盡時在水下自傷（與溺水互斥，避免雙重傷害）
        damage_events.write(DamageEvent::new(
            entity,
            DROWN_DAMAGE_PER_SEC * 0.5 * dt,
            DamageSource::Environment,
        ));
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

    /// 跑遊戲裡的上下車系統（含跟車）、游泳偵測和死亡重生，時間每幀走 0.1 秒（第一幀 0）；
    /// 機車停在車心 0.4（比入水門檻低），玩家站在原點；回傳 (app, 玩家, 機車, 座位)
    fn scooter_app() -> (App, Entity, Entity, Vec3) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        crate::world::install_map(&mut app);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ))
        .add_message::<crate::wanted::CrimeEvent>()
        .init_resource::<crate::player::PlayerConfig>()
        .init_resource::<GameState>()
        .init_resource::<VehicleTransitionState>()
        .init_resource::<crate::combat::RespawnState>()
        .init_resource::<crate::ui::ScreenEffectState>()
        .init_resource::<crate::ui::NotificationQueue>()
        .add_systems(
            Update,
            (
                super::super::vehicle_transition::vehicle_transition_systems(),
                player_water_detection_system,
                crate::combat::player_respawn_system,
            ),
        );
        let seat = Vec3::new(2.5, 0.4, 0.0);
        let scooter = app
            .world_mut()
            .spawn((
                crate::vehicle::Vehicle::default(),
                Transform::from_translation(seat),
            ))
            .id();
        let player = app
            .world_mut()
            .spawn((
                Player::default(),
                Transform::from_xyz(0.0, 0.7, 0.0),
                Visibility::Visible,
                crate::combat::Health::new(100.0),
            ))
            .id();
        (app, player, scooter, seat)
    }

    /// 跑 frames 幀，每一幀都不能被當成在游泳
    fn assert_never_swims(app: &mut App, player: Entity, frames: usize, stage: &str) {
        for frame in 0..frames {
            app.update();
            assert!(
                !app.world().entity(player).contains::<PlayerSwimming>(),
                "{stage} 第 {frame} 幀被當成在游泳，玩家 y={}",
                app.world().get::<Transform>(player).unwrap().translation.y
            );
        }
    }

    #[test]
    fn riding_a_scooter_is_not_swimming() {
        // 上車、騎車、下車的每一幀都不能被當成在游泳；下車後偵測照常（低於門檻會入水、高過出水門檻會解除）
        let (mut app, player, scooter, seat) = scooter_app();
        app.world_mut()
            .resource_mut::<VehicleTransitionState>()
            .start_enter(
                Vec3::new(0.0, 0.7, 0.0),
                scooter,
                seat - Vec3::X * 1.2,
                false,
            );
        assert_never_swims(&mut app, player, 15, "上車");
        assert!(app.world().resource::<GameState>().player_in_vehicle);
        assert_never_swims(&mut app, player, 15, "騎車");
        app.world_mut()
            .resource_mut::<VehicleTransitionState>()
            .start_exit(seat, scooter, seat + Vec3::X * 2.5, true);
        assert_never_swims(&mut app, player, 15, "下車");
        assert!(!app.world().resource::<GameState>().player_in_vehicle);

        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation
            .y = 0.3;
        app.update();
        assert!(app.world().entity(player).contains::<PlayerSwimming>());
        app.world_mut()
            .get_mut::<Transform>(player)
            .unwrap()
            .translation
            .y = 1.2;
        app.update();
        assert!(!app.world().entity(player).contains::<PlayerSwimming>());
    }

    #[test]
    fn leaving_a_parked_scooter_abruptly_is_not_swimming() {
        // 進座位時被打死、進座位時機車不見、剛坐上停著的機車就被打死：
        // 中止或清掉在車上的狀態後，玩家不能停在機車的高度被當成入水（重生後也一樣）
        for case in ["進座位時被打死", "進座位時機車不見", "坐著被打死"] {
            let (mut app, player, scooter, seat) = scooter_app();
            if case == "坐著被打死" {
                app.world_mut()
                    .get_mut::<Transform>(player)
                    .unwrap()
                    .translation = seat;
                *app.world_mut().resource_mut::<GameState>() = GameState {
                    player_in_vehicle: true,
                    current_vehicle: Some(scooter),
                };
            } else {
                let mut transition = app.world_mut().resource_mut::<VehicleTransitionState>();
                transition.start_enter(
                    Vec3::new(0.0, 0.7, 0.0),
                    scooter,
                    seat - Vec3::X * 1.2,
                    false,
                );
                transition.phase = crate::player::VehicleTransitionPhase::EnteringVehicle;
                transition.progress = 0.6;
                app.world_mut()
                    .get_mut::<Transform>(player)
                    .unwrap()
                    .translation = seat;
            }
            if case == "進座位時機車不見" {
                app.world_mut().entity_mut(scooter).despawn();
            } else {
                let mut respawn = app
                    .world_mut()
                    .resource_mut::<crate::combat::RespawnState>();
                respawn.is_dead = true;
                respawn.respawn_timer = 0.5;
            }
            assert_never_swims(&mut app, player, 30, case);
        }
    }
}
