//! 角色切換衛星動畫系統 (GTA 5 風格)
//!
//! 按數字鍵 5/6/7 切換角色時，攝影機會先拉高至衛星視角，
//! 然後俯瞰滑行至新角色位置，最後拉近回到正常視角。

use bevy::prelude::*;

use crate::camera::GameCamera;
use crate::combat::{Health, RespawnState};
use crate::core::{ease_in_quad, ease_out_quad, AppState, CameraSettings};
use crate::economy::PlayerWallet;

use super::{CharacterId, CharacterManager, Player, PlayerSkills, VehicleTransitionState};

// ============================================================================
// 常數
// ============================================================================

/// ZoomOut 階段持續時間（秒）
const ZOOM_OUT_DURATION: f32 = 1.0;
/// Hold 階段持續時間（秒）
const HOLD_DURATION: f32 = 0.5;
/// ZoomIn 階段持續時間（秒）
const ZOOM_IN_DURATION: f32 = 1.0;
/// ZoomOut + Hold 結束時間
const ZOOM_IN_START: f32 = ZOOM_OUT_DURATION + HOLD_DURATION;
/// 動畫總時長
const TOTAL_DURATION: f32 = ZOOM_OUT_DURATION + HOLD_DURATION + ZOOM_IN_DURATION;

/// 衛星視角攝影機距離
const SATELLITE_DISTANCE: f32 = 500.0;
/// 衛星視角俯仰角（~80°，近乎正上方俯瞰）
const SATELLITE_PITCH: f32 = 1.4;

// ============================================================================
// 類型定義
// ============================================================================

/// 動畫階段
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum SwitchPhase {
    #[default]
    Inactive,
    /// 拉遠至衛星視角
    ZoomOut,
    /// 衛星視角停留（滑行至目標位置）
    Hold,
    /// 拉近至新角色
    ZoomIn,
}

/// 角色切換衛星動畫狀態
#[derive(Resource, Default)]
pub struct CharacterSwitchAnimation {
    /// 當前階段
    phase: SwitchPhase,
    /// 已經過時間（秒）
    elapsed: f32,
    /// 目標角色 ID
    target_id: Option<CharacterId>,
    /// 目標角色位置
    target_position: Vec3,
    /// 動畫開始時的攝影機距離
    original_distance: f32,
    /// 動畫開始時的攝影機俯仰角
    original_pitch: f32,
    /// 動畫開始時的攝影機注視位置（玩家位置）
    origin_position: Vec3,
    /// 是否已執行角色傳送
    teleported: bool,
}

impl CharacterSwitchAnimation {
    /// 是否正在播放動畫
    pub fn is_active(&self) -> bool {
        self.phase != SwitchPhase::Inactive
    }

    fn start(
        &mut self,
        target: CharacterId,
        target_pos: Vec3,
        player_pos: Vec3,
        camera_settings: &CameraSettings,
    ) {
        self.phase = SwitchPhase::ZoomOut;
        self.elapsed = 0.0;
        self.target_id = Some(target);
        self.target_position = target_pos;
        self.origin_position = player_pos;
        self.original_distance = camera_settings.distance;
        self.original_pitch = camera_settings.pitch;
        self.teleported = false;
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 角色切換輸入系統（數字鍵 5/6/7）
pub fn character_switch_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut anim: ResMut<CharacterSwitchAnimation>,
    manager: Res<CharacterManager>,
    camera_settings: Res<CameraSettings>,
    player_query: Query<&Transform, With<Player>>,
    respawn_state: Res<RespawnState>,
    screen_effect: Res<crate::ui::ScreenEffectState>,
) {
    // 動畫進行中、死亡中、WASTED/BUSTED 期間不接受新輸入
    if anim.is_active() || respawn_state.is_dead || screen_effect.is_active() {
        return;
    }

    // 偵測按鍵 → 角色 ID 對應
    let target = if keyboard.just_pressed(KeyCode::Digit5) {
        Some(CharacterId::ALong)
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        Some(CharacterId::XiaoMei)
    } else if keyboard.just_pressed(KeyCode::Digit7) {
        Some(CharacterId::ACai)
    } else {
        None
    };

    let Some(target_id) = target else { return };

    // 驗證是否可切換（冷卻、已解鎖、非自身）
    if !manager.can_switch_to(target_id) {
        return;
    }

    // 取得目標角色位置
    let Some(target_snapshot) = manager.get(target_id) else {
        return;
    };
    let target_pos = target_snapshot.position;

    // 取得玩家當前位置
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    info!(
        "🛰️ 角色切換動畫開始: {} → {}",
        manager.active.name(),
        target_id.name()
    );
    anim.start(target_id, target_pos, player_pos, &camera_settings);
}

/// 角色切換攝影機動畫系統
///
/// 動畫期間直接控制攝影機 Transform，覆蓋正常的 camera_follow。
/// 在 Hold→ZoomIn 過渡時執行角色資料保存與傳送。
#[allow(clippy::too_many_arguments)]
pub fn character_switch_camera_system(
    time: Res<Time>,
    mut anim: ResMut<CharacterSwitchAnimation>,
    mut manager: ResMut<CharacterManager>,
    mut camera_settings: ResMut<CameraSettings>,
    mut camera_query: Query<&mut Transform, With<GameCamera>>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<GameCamera>)>,
    mut health_query: Query<&mut Health, With<Player>>,
    mut skills: ResMut<PlayerSkills>,
    mut wallet: ResMut<PlayerWallet>,
    respawn_state: Res<RespawnState>,
    vehicle_transition: Res<VehicleTransitionState>,
) {
    if !anim.is_active() {
        return;
    }

    // 緊急中斷：玩家死亡或載具動畫衝突
    if respawn_state.is_dead || vehicle_transition.is_animating() {
        warn!("🛰️ 角色切換動畫被中斷");
        camera_settings.distance = anim.original_distance;
        camera_settings.pitch = anim.original_pitch;
        anim.reset();
        return;
    }

    let dt = time.delta_secs();
    anim.elapsed += dt;

    // 階段轉換
    match anim.phase {
        SwitchPhase::ZoomOut => {
            if anim.elapsed >= ZOOM_OUT_DURATION {
                anim.phase = SwitchPhase::Hold;
            }
        }
        SwitchPhase::Hold => {
            if anim.elapsed >= ZOOM_IN_START {
                anim.phase = SwitchPhase::ZoomIn;
            }
        }
        SwitchPhase::ZoomIn => {
            // 在 ZoomIn 開始時執行傳送（僅一次）
            if !anim.teleported {
                anim.teleported = true;
                teleport_to_target(
                    &mut anim,
                    &mut manager,
                    &mut player_query,
                    &mut health_query,
                    &mut skills,
                    &mut wallet,
                );
            }

            if anim.elapsed >= TOTAL_DURATION {
                // 動畫結束，恢復攝影機參數
                camera_settings.distance = anim.original_distance;
                camera_settings.pitch = anim.original_pitch;
                info!("🛰️ 角色切換動畫結束");
                anim.reset();
                return;
            }
        }
        SwitchPhase::Inactive => return,
    }

    // 計算當前插值參數
    let (current_distance, current_pitch, look_target) = match anim.phase {
        SwitchPhase::ZoomOut => {
            let t = (anim.elapsed / ZOOM_OUT_DURATION).min(1.0);
            let eased = ease_out_quad(t);
            let dist = anim.original_distance
                + (SATELLITE_DISTANCE - anim.original_distance) * eased;
            let pitch = anim.original_pitch
                + (SATELLITE_PITCH - anim.original_pitch) * eased;
            (dist, pitch, anim.origin_position)
        }
        SwitchPhase::Hold => {
            // 衛星視角，注視點從原位滑行至目標位置
            let hold_elapsed = anim.elapsed - ZOOM_OUT_DURATION;
            let t = (hold_elapsed / HOLD_DURATION).min(1.0);
            let look = anim.origin_position.lerp(anim.target_position, t);
            (SATELLITE_DISTANCE, SATELLITE_PITCH, look)
        }
        SwitchPhase::ZoomIn => {
            let zoom_elapsed = anim.elapsed - ZOOM_IN_START;
            let t = (zoom_elapsed / ZOOM_IN_DURATION).min(1.0);
            let eased = ease_in_quad(t);
            let dist = SATELLITE_DISTANCE
                + (anim.original_distance - SATELLITE_DISTANCE) * eased;
            let pitch = SATELLITE_PITCH
                + (anim.original_pitch - SATELLITE_PITCH) * eased;
            (dist, pitch, anim.target_position)
        }
        SwitchPhase::Inactive => unreachable!(),
    };

    // 更新攝影機位置
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let yaw = camera_settings.yaw;
    let offset = Vec3::new(
        current_distance * current_pitch.cos() * yaw.sin(),
        current_distance * current_pitch.sin(),
        current_distance * current_pitch.cos() * yaw.cos(),
    );
    camera_transform.translation = look_target + offset;
    camera_transform.look_at(look_target, Vec3::Y);
}

/// 執行角色傳送：保存當前角色狀態 → 切換至目標角色 → 移動玩家實體並恢復狀態
fn teleport_to_target(
    anim: &mut CharacterSwitchAnimation,
    manager: &mut CharacterManager,
    player_query: &mut Query<&mut Transform, (With<Player>, Without<GameCamera>)>,
    health_query: &mut Query<&mut Health, With<Player>>,
    skills: &mut PlayerSkills,
    wallet: &mut PlayerWallet,
) {
    let Some(target_id) = anim.target_id else {
        return;
    };

    // 讀取當前玩家狀態
    let Ok(mut player_transform) = player_query.single_mut() else {
        return;
    };
    let current_pos = player_transform.translation;
    let (_, rotation_y, _) = player_transform.rotation.to_euler(EulerRot::YXZ);
    let current_hp = health_query
        .single()
        .map(|h| h.current)
        .unwrap_or(100.0);

    // 保存當前角色狀態並切換
    let target_snapshot = manager.switch_to(
        target_id,
        current_pos,
        rotation_y,
        current_hp,
        wallet.cash,
        wallet.bank,
        skills,
    );

    if let Some(snapshot) = target_snapshot {
        // 傳送玩家實體至目標位置
        player_transform.translation = snapshot.position;
        player_transform.rotation = Quat::from_rotation_y(snapshot.rotation_y);

        // 恢復目標角色的完整狀態
        if let Ok(mut health) = health_query.single_mut() {
            health.current = snapshot.hp;
        }
        wallet.cash = snapshot.cash;
        wallet.bank = snapshot.bank;
        *skills = snapshot.skills.clone();

        info!(
            "🛰️ 傳送至 {} — HP: {:.0}, 現金: ${}",
            target_id.name(),
            snapshot.hp,
            snapshot.cash,
        );
    }
}

// ============================================================================
// Plugin
// ============================================================================

pub(super) struct CharacterSwitchAnimationPlugin;

impl Plugin for CharacterSwitchAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterSwitchAnimation>()
            .add_systems(
                Update,
                (
                    character_switch_input_system
                        .before(crate::camera::camera_input),
                    character_switch_camera_system
                        .after(character_switch_input_system)
                        .after(crate::camera::camera_auto_follow),
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
