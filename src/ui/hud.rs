//! HUD 系統 (GTA 風格)
//!
//! 包含：時間顯示、金錢顯示、血量/護甲條、任務狀態、HUD 動畫

use bevy::prelude::*;

use super::components::{
    ArmorBarFill, ArmorLabel, ArmorLabelShadow, ArmorSection, ControlSpeedDisplay, ControlStatusTag,
    HealthBarFill, HealthBarGlow, HealthBarHighlight, HealthLabel, HealthLabelShadow,
    HudAnimationState, MinimapPlayerGlow, MinimapScanLine, MissionInfo, MoneyDisplay, TimeDisplay,
    UiText,
};
use crate::combat::{Armor, Health};
use crate::core::{GameState, WorldTime};
use crate::mission::MissionManager;
use crate::player::Player;
use crate::vehicle::{Vehicle, VehicleType};

// === HUD 動畫常數 ===

/// 低血量脈衝閾值（血量百分比）
const LOW_HEALTH_THRESHOLD: f32 = 0.3;
/// 低血量脈衝速度（每秒弧度）
const LOW_HEALTH_PULSE_SPEED: f32 = 4.0;
/// 低血量脈衝發光最大強度
const LOW_HEALTH_GLOW_MAX: f32 = 0.6;
/// 低血量脈衝發光最小強度
const LOW_HEALTH_GLOW_MIN: f32 = 0.15;

/// 小地圖掃描線速度（每秒完成一次掃描）
const MINIMAP_SCAN_SPEED: f32 = 0.5;

/// 玩家標記脈衝速度
const PLAYER_MARKER_PULSE_SPEED: f32 = 3.0;

// === 載具名稱輔助函數 ===

/// 取得載具類型中文名稱
fn get_vehicle_type_name(vehicle_type: VehicleType) -> &'static str {
    match vehicle_type {
        VehicleType::Scooter => "機車",
        VehicleType::Car => "汽車",
        VehicleType::Taxi => "計程車",
        VehicleType::Bus => "公車",
    }
}

/// 取得控制提示文字
fn get_control_hint_text(
    game_state: &GameState,
    vehicle_query: &Query<&Vehicle>,
    mission_manager: &MissionManager,
) -> String {
    if !game_state.player_in_vehicle {
        return if mission_manager.active_mission.is_some() {
            "[步行] WASD移動 | Q/E旋轉 | R射擊 T換彈 | 1-4武器 | Tab上車".to_string()
        } else {
            "[步行] WASD移動 | Q/E旋轉 | R射擊 T換彈 | 1-4武器 | F接任務".to_string()
        };
    }

    let Some(vehicle_entity) = game_state.current_vehicle else {
        return String::new();
    };

    let Ok(vehicle) = vehicle_query.get(vehicle_entity) else {
        return String::new();
    };

    let speed_kmh = (vehicle.current_speed * 3.6).abs() as i32;
    format!(
        "[{}] {} km/h | WASD駕駛 | Space煞車 | Tab下車",
        get_vehicle_type_name(vehicle.vehicle_type),
        speed_kmh
    )
}

/// 格式化世界時間
fn format_world_time(world_time: &WorldTime) -> String {
    let hour = world_time.hour as u32;
    let minute = ((world_time.hour - hour as f32) * 60.0) as u32;
    let day_night = if (6..18).contains(&hour) { "D" } else { "N" };
    format!("[{}] {:02}:{:02}", day_night, hour, minute)
}

// === UI 更新系統 ===

/// 更新 UI（時間、速度、狀態標籤）
#[allow(clippy::type_complexity)]
pub fn update_ui(
    game_state: Res<GameState>,
    world_time: Res<WorldTime>,
    vehicle_query: Query<&Vehicle>,
    mission_manager: Res<MissionManager>,
    mut text_query: Query<
        &mut Text,
        (
            With<UiText>,
            Without<TimeDisplay>,
            Without<ControlStatusTag>,
            Without<ControlSpeedDisplay>,
        ),
    >,
    mut time_query: Query<&mut Text, With<TimeDisplay>>,
    mut status_tag_query: Query<
        &mut Text,
        (
            With<ControlStatusTag>,
            Without<TimeDisplay>,
            Without<UiText>,
            Without<ControlSpeedDisplay>,
        ),
    >,
    mut speed_display_query: Query<
        (&mut Text, &mut Visibility),
        (
            With<ControlSpeedDisplay>,
            Without<TimeDisplay>,
            Without<UiText>,
            Without<ControlStatusTag>,
        ),
    >,
) {
    // 更新舊版控制提示文字（保留兼容）
    if let Ok(mut text) = text_query.single_mut() {
        **text = get_control_hint_text(&game_state, &vehicle_query, &mission_manager);
    }

    // 更新時間顯示
    if let Ok(mut text) = time_query.single_mut() {
        **text = format_world_time(&world_time);
    }

    // 更新 GTA 風格狀態標籤
    if let Ok(mut status_text) = status_tag_query.single_mut() {
        let name = if game_state.player_in_vehicle {
            vehicle_query
                .iter()
                .next()
                .map(|v| get_vehicle_type_name(v.vehicle_type))
                .unwrap_or("駕駛")
        } else {
            "步行"
        };
        **status_text = name.to_string();
    }

    // 更新速度顯示
    let Ok((mut speed_text, mut visibility)) = speed_display_query.single_mut() else {
        return;
    };
    if !game_state.player_in_vehicle {
        *visibility = Visibility::Hidden;
        return;
    }
    if let Some(vehicle) = vehicle_query.iter().next() {
        let speed_kmh = (vehicle.current_speed * 3.6).abs() as i32;
        **speed_text = format!("{} km/h", speed_kmh);
        *visibility = Visibility::Visible;
    }
}

/// 任務 UI 更新
pub fn update_mission_ui(
    mission_manager: Res<MissionManager>,
    player_query: Query<&Transform, With<Player>>,
    mut mission_info_query: Query<&mut Text, With<MissionInfo>>,
) {
    if let Ok(mut text) = mission_info_query.single_mut() {
        if let Some(ref active) = mission_manager.active_mission {
            if let Ok(player_transform) = player_query.single() {
                let distance = player_transform.translation.distance(active.data.end_pos);

                if let Some(limit) = active.data.time_limit {
                    let remaining = (limit - active.time_elapsed).max(0.0);
                    **text = format!(
                        "[任務] {} | {:.0}m | {:.0}s",
                        active.data.title, distance, remaining
                    );
                } else {
                    **text = format!("[任務] {} | {:.0}m", active.data.title, distance);
                }
            }
        } else {
            **text = "".to_string();
        }
    }
}

// === 護甲區更新輔助函數 ===

/// 更新護甲區顯示
#[allow(clippy::type_complexity)]
fn update_armor_section(
    armor_opt: Option<&Armor>,
    armor_section_query: &mut Query<&mut Visibility, With<ArmorSection>>,
    armor_fill_query: &mut Query<
        &mut Node,
        (
            With<ArmorBarFill>,
            Without<HealthBarFill>,
            Without<HealthBarHighlight>,
        ),
    >,
    armor_label_query: &mut Query<
        &mut Text,
        (
            With<ArmorLabel>,
            Without<HealthLabel>,
            Without<MoneyDisplay>,
            Without<ArmorLabelShadow>,
        ),
    >,
    armor_shadow_query: &mut Query<
        &mut Text,
        (
            With<ArmorLabelShadow>,
            Without<ArmorLabel>,
            Without<HealthLabel>,
            Without<HealthLabelShadow>,
        ),
    >,
) {
    let should_show = armor_opt.map(|a| a.current > 0.0).unwrap_or(false);

    if let Ok(mut visibility) = armor_section_query.single_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Some(armor) = armor_opt.filter(|a| a.current > 0.0) {
        let armor_percent = (armor.current / armor.max * 100.0).clamp(0.0, 100.0);

        if let Ok(mut node) = armor_fill_query.single_mut() {
            node.width = Val::Percent(armor_percent);
        }

        let armor_text = format!("{:.0}/{:.0}", armor.current, armor.max);
        if let Ok(mut text) = armor_label_query.single_mut() {
            **text = armor_text.clone();
        }
        if let Ok(mut text) = armor_shadow_query.single_mut() {
            **text = armor_text;
        }
    }
}

// === 玩家狀態 HUD 更新 ===

/// 更新 HUD（血量條、護甲條、金錢）- GTA 風格
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_hud(
    player_query: Query<(&Health, Option<&Armor>, &Player)>,
    mut health_fill_query: Query<
        &mut Node,
        (
            With<HealthBarFill>,
            Without<HealthBarHighlight>,
            Without<ArmorBarFill>,
        ),
    >,
    mut health_highlight_query: Query<
        &mut Node,
        (
            With<HealthBarHighlight>,
            Without<HealthBarFill>,
            Without<ArmorBarFill>,
        ),
    >,
    mut health_label_query: Query<
        &mut Text,
        (
            With<HealthLabel>,
            Without<MoneyDisplay>,
            Without<ArmorLabel>,
            Without<HealthLabelShadow>,
        ),
    >,
    mut health_shadow_query: Query<
        &mut Text,
        (
            With<HealthLabelShadow>,
            Without<HealthLabel>,
            Without<ArmorLabel>,
            Without<ArmorLabelShadow>,
        ),
    >,
    mut armor_section_query: Query<&mut Visibility, With<ArmorSection>>,
    mut armor_fill_query: Query<
        &mut Node,
        (
            With<ArmorBarFill>,
            Without<HealthBarFill>,
            Without<HealthBarHighlight>,
        ),
    >,
    mut armor_label_query: Query<
        &mut Text,
        (
            With<ArmorLabel>,
            Without<HealthLabel>,
            Without<MoneyDisplay>,
            Without<ArmorLabelShadow>,
        ),
    >,
    mut armor_shadow_query: Query<
        &mut Text,
        (
            With<ArmorLabelShadow>,
            Without<ArmorLabel>,
            Without<HealthLabel>,
            Without<HealthLabelShadow>,
        ),
    >,
    mut money_query: Query<
        &mut Text,
        (
            With<MoneyDisplay>,
            Without<HealthLabel>,
            Without<ArmorLabel>,
            Without<HealthLabelShadow>,
            Without<ArmorLabelShadow>,
        ),
    >,
) {
    let Ok((health, armor_opt, player)) = player_query.single() else {
        return;
    };

    let health_percent = health.percentage() * 100.0;

    // 更新血量條填充寬度
    if let Ok(mut node) = health_fill_query.single_mut() {
        node.width = Val::Percent(health_percent);
    }

    // 更新血量條高光寬度（跟隨填充）
    if let Ok(mut node) = health_highlight_query.single_mut() {
        node.width = Val::Percent(health_percent);
    }

    // 更新血量數值標籤和陰影
    let health_text = format!("{:.0}/{:.0}", health.current, health.max);
    if let Ok(mut text) = health_label_query.single_mut() {
        **text = health_text.clone();
    }
    if let Ok(mut text) = health_shadow_query.single_mut() {
        **text = health_text;
    }

    // 更新護甲區
    update_armor_section(
        armor_opt,
        &mut armor_section_query,
        &mut armor_fill_query,
        &mut armor_label_query,
        &mut armor_shadow_query,
    );

    // 更新金錢顯示
    if let Ok(mut text) = money_query.single_mut() {
        **text = format!("$ {}", player.money);
    }
}

// === HUD 動畫系統 ===

/// 將動畫相位環繞到 0..TAU 範圍
fn wrap_animation_phase(phase: &mut f32) {
    if *phase > std::f32::consts::TAU {
        *phase -= std::f32::consts::TAU;
    }
}

/// 計算低血量發光強度
fn calculate_low_health_glow_intensity(pulse_phase: f32, health_percent: f32) -> f32 {
    let pulse = (pulse_phase.sin() + 1.0) * 0.5;
    let glow_intensity = LOW_HEALTH_GLOW_MIN + (LOW_HEALTH_GLOW_MAX - LOW_HEALTH_GLOW_MIN) * pulse;
    let health_factor = 1.0 - (health_percent / LOW_HEALTH_THRESHOLD);
    glow_intensity * health_factor
}

/// 更新 HUD 動畫狀態（低血量脈衝、小地圖掃描）
#[allow(clippy::type_complexity)]
pub fn update_hud_animations(
    time: Res<Time>,
    mut anim_state: ResMut<HudAnimationState>,
    player_query: Query<&Health, With<Player>>,
    mut health_glow_query: Query<&mut BackgroundColor, With<HealthBarGlow>>,
    mut minimap_scan_query: Query<&mut Node, With<MinimapScanLine>>,
    mut player_glow_query: Query<
        &mut BackgroundColor,
        (With<MinimapPlayerGlow>, Without<HealthBarGlow>),
    >,
) {
    let dt = time.delta_secs();

    // === 低血量脈衝動畫 ===
    if let Ok(health) = player_query.single() {
        let health_percent = health.percentage();
        let is_low_health = health_percent < LOW_HEALTH_THRESHOLD && !health.is_dead();

        if is_low_health {
            anim_state.low_health_pulse_phase += LOW_HEALTH_PULSE_SPEED * dt;
            wrap_animation_phase(&mut anim_state.low_health_pulse_phase);
            let final_intensity = calculate_low_health_glow_intensity(
                anim_state.low_health_pulse_phase,
                health_percent,
            );
            for mut bg in health_glow_query.iter_mut() {
                *bg = BackgroundColor(Color::srgba(0.8, 0.15, 0.1, final_intensity));
            }
        } else {
            anim_state.low_health_pulse_phase = 0.0;
            for mut bg in health_glow_query.iter_mut() {
                *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0));
            }
        }
    }

    // === 小地圖掃描線動畫 ===
    anim_state.minimap_scan_position += MINIMAP_SCAN_SPEED * dt;
    if anim_state.minimap_scan_position > 1.0 {
        anim_state.minimap_scan_position -= 1.0;
    }
    for mut node in minimap_scan_query.iter_mut() {
        node.top = Val::Percent(anim_state.minimap_scan_position * 100.0);
    }

    // === 玩家標記脈衝動畫 ===
    anim_state.player_marker_pulse_phase += PLAYER_MARKER_PULSE_SPEED * dt;
    wrap_animation_phase(&mut anim_state.player_marker_pulse_phase);
    let marker_pulse = (anim_state.player_marker_pulse_phase.sin() + 1.0) * 0.5;
    let marker_glow_alpha = 0.15 + marker_pulse * 0.2;
    for mut bg in player_glow_query.iter_mut() {
        *bg = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, marker_glow_alpha));
    }
}
