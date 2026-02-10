//! 準星動態更新系統 — 命中標記、彈藥顯示、武器切換動畫、準星動態效果

use bevy::prelude::*;

use crate::combat::{CombatState, WeaponInventory};
use crate::core::GameState;
use crate::player::Player;
use crate::ui::components::{
    AmmoBulletIcon, AmmoVisualGrid, Crosshair, CrosshairDirection, CrosshairDot,
    CrosshairDynamics, CrosshairHitMarker, CrosshairLine, CrosshairOuterRing, CurrentAmmoShadow,
    CurrentAmmoText, HitMarkerLine, ReserveAmmoShadow, ReserveAmmoText, WeaponDisplay,
    WeaponDisplayShadow, WeaponSlot, WeaponSwitchAnimation,
};
use crate::ui::constants::*;

use super::setup::spawn_bullet_icon;

// ============================================================================
// 準星動態常數
// ============================================================================
/// 準星動態：射擊後展開量
#[allow(dead_code)]
const CROSSHAIR_FIRE_EXPAND: f32 = 1.8;
/// 準星動態：瞄準時收縮量
const CROSSHAIR_AIM_SHRINK: f32 = 0.6;
/// 準星動態：恢復速度
const CROSSHAIR_RECOVERY_SPEED: f32 = 5.0;
/// 準星動態：命中反彈縮放
#[allow(dead_code)]
const CROSSHAIR_HIT_BOUNCE_SCALE: f32 = 1.3;
/// 準星動態：命中反彈恢復速度
const CROSSHAIR_HIT_BOUNCE_RECOVERY: f32 = 8.0;

// ============================================================================
// 準星輔助函數
// ============================================================================
/// 判斷是否應該顯示準星（不在車上、正在瞄準、持有遠程武器）
fn should_show_crosshair(
    game_state: &GameState,
    combat_state: &CombatState,
    player_query: &Query<&WeaponInventory, With<Player>>,
) -> bool {
    if game_state.player_in_vehicle || !combat_state.is_aiming {
        return false;
    }
    player_query
        .single()
        .ok()
        .and_then(|inv| inv.current_weapon())
        .map(|w| w.stats.magazine_size > 0)
        .unwrap_or(false)
}

/// 更新準星擴散值（逐漸恢復）
fn update_crosshair_bloom(combat_state: &mut CombatState, dt: f32) {
    if combat_state.crosshair_bloom > 0.0 {
        let recovery_rate = if combat_state.is_aiming { 5.0 } else { 2.0 };
        combat_state.crosshair_bloom = (combat_state.crosshair_bloom - dt * recovery_rate).max(0.0);
    }
}

/// 計算準星偏移量
fn calculate_crosshair_offset(bloom: f32, is_aiming: bool) -> f32 {
    let bloom_offset = bloom.min(1.0) * 12.0;
    let aim_shrink = if is_aiming { 3.0 } else { 0.0 };
    (bloom_offset - aim_shrink).max(-3.0)
}

/// 計算外圈大小
fn calculate_outer_ring_size(bloom: f32, is_aiming: bool) -> f32 {
    let base_size = if is_aiming { 40.0 } else { 50.0 };
    let bloom_expand = bloom.min(1.0) * 20.0;
    base_size + bloom_expand
}

/// 應用準星線條偏移（新版 GTA 風格，基於 4.0 的基礎位置）
fn apply_crosshair_line_offset(node: &mut Node, direction: CrosshairDirection, offset: f32) {
    // 基礎位置 4.0 + 偏移
    let base = 4.0;
    let val = Val::Px(base - offset);
    match direction {
        CrosshairDirection::Top => node.top = val,
        CrosshairDirection::Bottom => node.bottom = val,
        CrosshairDirection::Left => node.left = val,
        CrosshairDirection::Right => node.right = val,
    }
}

// ============================================================================
// 準星更新系統
// ============================================================================
/// 更新準星 UI（根據射擊狀態調整準星大小）- GTA 風格
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_crosshair(
    time: Res<Time>,
    mut combat_state: ResMut<CombatState>,
    game_state: Res<GameState>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut crosshair_query: Query<&mut Visibility, With<Crosshair>>,
    mut line_query: Query<(&mut Node, &CrosshairLine), Without<CrosshairOuterRing>>,
    mut outer_ring_query: Query<&mut Node, (With<CrosshairOuterRing>, Without<CrosshairLine>)>,
    mut dot_query: Query<&mut BackgroundColor, With<CrosshairDot>>,
) {
    // 更新可見性
    let should_show = should_show_crosshair(&game_state, &combat_state, &player_query);
    for mut visibility in crosshair_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 更新擴散
    update_crosshair_bloom(&mut combat_state, time.delta_secs());

    let bloom = combat_state.crosshair_bloom;
    let is_aiming = combat_state.is_aiming;

    // 更新線條位置
    let offset = calculate_crosshair_offset(bloom, is_aiming);
    for (mut node, line) in line_query.iter_mut() {
        apply_crosshair_line_offset(&mut node, line.direction, offset);
    }

    // 更新外圈大小
    let ring_size = calculate_outer_ring_size(bloom, is_aiming);
    for mut node in outer_ring_query.iter_mut() {
        node.width = Val::Px(ring_size);
        node.height = Val::Px(ring_size);
    }

    // 瞄準時中心點變亮
    let dot_color = if is_aiming {
        CROSSHAIR_AIM
    } else {
        CROSSHAIR_MAIN
    };
    for mut bg in dot_query.iter_mut() {
        *bg = BackgroundColor(dot_color);
    }
}

/// 更新命中標記（X 形回饋）
#[allow(clippy::type_complexity)]
pub fn update_hit_marker(
    time: Res<Time>,
    mut combat_state: ResMut<CombatState>,
    mut hit_marker_query: Query<&mut Visibility, With<CrosshairHitMarker>>,
    mut hit_marker_line_query: Query<&mut BackgroundColor, With<HitMarkerLine>>,
) {
    // 更新計時器
    if combat_state.hit_marker_timer > 0.0 {
        combat_state.hit_marker_timer -= time.delta_secs();
    }

    // 更新命中標記可見性
    let should_show = combat_state.hit_marker_timer > 0.0;
    for mut visibility in hit_marker_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 更新命中標記顏色（爆頭為金色，普通為紅色）
    if should_show {
        let color = if combat_state.hit_marker_headshot {
            HEADSHOT_MARKER_COLOR
        } else {
            HIT_MARKER_COLOR
        };
        // 根據剩餘時間計算透明度（淡出效果）
        let alpha = (combat_state.hit_marker_timer / HIT_MARKER_DURATION).min(1.0);
        let faded_color = Color::srgba(
            color.to_srgba().red,
            color.to_srgba().green,
            color.to_srgba().blue,
            color.to_srgba().alpha * alpha,
        );
        for mut bg in hit_marker_line_query.iter_mut() {
            *bg = BackgroundColor(faded_color);
        }
    }
}

// ============================================================================
// 彈藥顯示輔助函數
// ============================================================================
/// 格式化當前彈藥文字
fn format_current_ammo_text(magazine_size: u32, current_ammo: u32, is_reloading: bool) -> String {
    if magazine_size == 0 {
        "∞".to_string()
    } else if is_reloading {
        "...".to_string()
    } else {
        format!("{}", current_ammo)
    }
}

/// 取得當前彈藥顏色
fn get_current_ammo_color(magazine_size: u32, is_reloading: bool, is_low_ammo: bool) -> Color {
    if magazine_size == 0 {
        AMMO_NORMAL
    } else if is_reloading {
        AMMO_RESERVE
    } else if is_low_ammo {
        AMMO_LOW
    } else {
        AMMO_NORMAL
    }
}

/// 格式化後備彈藥文字
fn format_reserve_ammo_text(magazine_size: u32, reserve_ammo: u32, is_reloading: bool) -> String {
    if magazine_size == 0 {
        String::new()
    } else if is_reloading {
        "換彈中".to_string()
    } else {
        format!("{}", reserve_ammo)
    }
}

// ============================================================================
// 彈藥顯示系統
// ============================================================================
/// 更新彈藥顯示（GTA 風格，支援低彈藥變色）
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ammo_display(
    player_query: Query<&WeaponInventory, With<Player>>,
    mut current_ammo_query: Query<
        (&mut Text, &mut TextColor),
        (
            With<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<WeaponDisplay>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut current_ammo_shadow_query: Query<
        &mut Text,
        (
            With<CurrentAmmoShadow>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<ReserveAmmoShadow>,
            Without<WeaponDisplay>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut reserve_ammo_query: Query<
        &mut Text,
        (
            With<ReserveAmmoText>,
            Without<CurrentAmmoText>,
            Without<WeaponDisplay>,
            Without<ReserveAmmoShadow>,
            Without<CurrentAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut reserve_ammo_shadow_query: Query<
        &mut Text,
        (
            With<ReserveAmmoShadow>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<WeaponDisplay>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut weapon_query: Query<
        &mut Text,
        (
            With<WeaponDisplay>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<WeaponDisplayShadow>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
        ),
    >,
    mut weapon_shadow_query: Query<
        &mut Text,
        (
            With<WeaponDisplayShadow>,
            Without<WeaponDisplay>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
        ),
    >,
    mut slot_query: Query<(&mut BackgroundColor, &WeaponSlot)>,
) {
    let Ok(inventory) = player_query.single() else {
        return;
    };
    let Some(weapon) = inventory.current_weapon() else {
        return;
    };

    let magazine_size = weapon.stats.magazine_size;
    let is_low_ammo = magazine_size > 0
        && weapon.current_ammo < (magazine_size as f32 * 0.25) as u32
        && !weapon.is_reloading;

    // 當前彈藥顯示
    let current_text =
        format_current_ammo_text(magazine_size, weapon.current_ammo, weapon.is_reloading);
    let current_color = get_current_ammo_color(magazine_size, weapon.is_reloading, is_low_ammo);

    for (mut text, mut color) in current_ammo_query.iter_mut() {
        **text = current_text.clone();
        color.0 = current_color;
    }
    for mut text in current_ammo_shadow_query.iter_mut() {
        **text = current_text.clone();
    }

    // 後備彈藥顯示
    let reserve_text =
        format_reserve_ammo_text(magazine_size, weapon.reserve_ammo, weapon.is_reloading);
    for mut text in reserve_ammo_query.iter_mut() {
        **text = reserve_text.clone();
    }
    for mut text in reserve_ammo_shadow_query.iter_mut() {
        **text = reserve_text.clone();
    }

    // 武器名稱
    let weapon_name = weapon.stats.weapon_type.name().to_string();
    for mut text in weapon_query.iter_mut() {
        **text = weapon_name.clone();
    }
    for mut text in weapon_shadow_query.iter_mut() {
        **text = weapon_name.clone();
    }

    // 武器槽位高亮
    let current_slot = inventory.current_index;
    for (mut bg, slot) in slot_query.iter_mut() {
        *bg = BackgroundColor(if slot.slot_index == current_slot {
            SLOT_ACTIVE
        } else {
            SLOT_INACTIVE
        });
    }
}

// ============================================================================
// 彈藥視覺化輔助函數
// ============================================================================
/// 計算低彈藥時的閃爍 alpha
fn calculate_low_ammo_blink_alpha(elapsed_secs: f32, is_low_ammo: bool) -> f32 {
    if is_low_ammo {
        let phase = elapsed_secs * 8.0;
        0.5 + 0.5 * phase.sin()
    } else {
        1.0
    }
}

/// 取得子彈圖示顏色
fn get_bullet_icon_color(is_filled: bool, is_low_ammo: bool, blink_alpha: f32) -> Color {
    if !is_filled {
        return BULLET_EMPTY;
    }
    if !is_low_ammo {
        return BULLET_FILLED;
    }
    let base = BULLET_LOW_WARN.to_srgba();
    Color::srgba(base.red, base.green, base.blue, blink_alpha)
}

/// 檢查是否為低彈藥狀態
fn is_low_ammo_state(current: usize, max: usize, is_reloading: bool) -> bool {
    max > 0 && current < (max as f32 * 0.25).ceil() as usize && !is_reloading
}

/// 更新彈藥視覺化網格（子彈圖示）
#[allow(clippy::type_complexity)]
pub fn update_ammo_visual_grid(
    time: Res<Time>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut bullet_query: Query<(&mut BackgroundColor, &AmmoBulletIcon)>,
    grid_query: Query<Entity, With<AmmoVisualGrid>>,
    children_query: Query<&Children>,
    mut commands: Commands,
) {
    let Ok(inventory) = player_query.single() else {
        return;
    };
    let Some(weapon) = inventory.current_weapon() else {
        return;
    };

    let current_ammo = weapon.current_ammo as usize;
    let magazine_size = weapon.stats.magazine_size as usize;
    let is_low_ammo = is_low_ammo_state(current_ammo, magazine_size, weapon.is_reloading);
    let blink_alpha = calculate_low_ammo_blink_alpha(time.elapsed_secs(), is_low_ammo);

    // 獲取網格中現有的子彈圖示數量
    let Ok(grid_entity) = grid_query.single() else {
        return;
    };
    let Ok(children) = children_query.get(grid_entity) else {
        return;
    };
    let existing_count = children.len();

    // 如果彈匣大小改變（切換武器），需要重建子彈圖示
    if existing_count != magazine_size && magazine_size > 0 {
        // 刪除所有現有子彈圖示
        for child in children.iter() {
            commands.entity(child).despawn();
        }

        // 生成新的子彈圖示
        commands.entity(grid_entity).with_children(|grid| {
            for i in 0..magazine_size {
                let is_filled = i < current_ammo;
                let color = if is_filled {
                    BULLET_FILLED
                } else {
                    BULLET_EMPTY
                };
                spawn_bullet_icon(grid, i, color);
            }
        });
        return;
    }

    // 更新現有子彈圖示顏色
    for (mut bg, bullet) in bullet_query.iter_mut() {
        let is_filled = bullet.index < current_ammo;
        *bg = BackgroundColor(get_bullet_icon_color(is_filled, is_low_ammo, blink_alpha));
    }
}

// ============================================================================
// 武器切換動畫輔助函數
// ============================================================================
/// 計算武器切換動畫的兩階段淡入淡出透明度
fn calculate_switch_opacity(progress: f32) -> f32 {
    if progress < 0.5 {
        1.0 - (progress * 2.0)
    } else {
        (progress - 0.5) * 2.0
    }
}

/// 計算武器槽位的亮度脈衝
fn calculate_slot_brightness(ease_out: f32) -> f32 {
    0.7 + (ease_out * std::f32::consts::PI).sin() * 0.3
}

/// 應用亮度到顏色
fn apply_brightness_to_color(color: Color, brightness: f32) -> Color {
    let base = color.to_srgba();
    Color::srgba(
        (base.red * brightness).min(1.0),
        (base.green * brightness).min(1.0),
        (base.blue * brightness).min(1.0),
        base.alpha,
    )
}

/// 更新切換動畫進度，回傳 (透明度, ease_out)
fn update_switch_animation_progress(
    anim: &mut WeaponSwitchAnimation,
    dt: f32,
) -> Option<(f32, f32)> {
    anim.progress += dt / anim.duration;
    if anim.progress >= 1.0 {
        anim.is_switching = false;
        anim.progress = 1.0;
        return None;
    }
    let p = anim.progress;
    Some((calculate_switch_opacity(p), 1.0 - (1.0 - p).powi(2)))
}

/// 更新武器切換動畫（簡化版：只做透明度和縮放，不做位置變化）
/// 注意：此系統需要在 update_ammo_display 之後執行，避免 Query 衝突導致 SIGSEGV
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_weapon_switch_animation(
    time: Res<Time>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut switch_anim: ResMut<WeaponSwitchAnimation>,
    mut weapon_display_query: Query<
        &mut TextColor,
        (
            With<WeaponDisplay>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<WeaponDisplayShadow>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
        ),
    >,
    mut weapon_shadow_query: Query<
        &mut TextColor,
        (
            With<WeaponDisplayShadow>,
            Without<WeaponDisplay>,
            Without<CurrentAmmoText>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
        ),
    >,
    mut current_ammo_query: Query<
        &mut TextColor,
        (
            With<CurrentAmmoText>,
            Without<WeaponDisplay>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<ReserveAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut current_shadow_query: Query<
        &mut TextColor,
        (
            With<CurrentAmmoShadow>,
            Without<CurrentAmmoText>,
            Without<WeaponDisplay>,
            Without<ReserveAmmoText>,
            Without<ReserveAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut reserve_ammo_query: Query<
        &mut TextColor,
        (
            With<ReserveAmmoText>,
            Without<CurrentAmmoText>,
            Without<WeaponDisplay>,
            Without<ReserveAmmoShadow>,
            Without<CurrentAmmoShadow>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut reserve_shadow_query: Query<
        &mut TextColor,
        (
            With<ReserveAmmoShadow>,
            Without<ReserveAmmoText>,
            Without<CurrentAmmoText>,
            Without<CurrentAmmoShadow>,
            Without<WeaponDisplay>,
            Without<WeaponDisplayShadow>,
        ),
    >,
    mut slot_query: Query<
        &mut BackgroundColor,
        (
            With<WeaponSlot>,
            Without<WeaponDisplay>,
            Without<WeaponDisplayShadow>,
        ),
    >,
) {
    let Ok(inventory) = player_query.single() else {
        return;
    };

    // 檢測武器切換
    if inventory.current_index != switch_anim.last_weapon_index {
        switch_anim.is_switching = true;
        switch_anim.progress = 0.0;
        switch_anim.last_weapon_index = inventory.current_index;
    }

    if !switch_anim.is_switching {
        return;
    }

    let Some((opacity, ease_out)) =
        update_switch_animation_progress(&mut switch_anim, time.delta_secs())
    else {
        return;
    };

    // 應用透明度到文字和陰影
    let shadow_alpha = opacity * 0.65;
    for mut c in weapon_display_query.iter_mut() {
        c.0 = c.0.with_alpha(opacity);
    }
    for mut c in weapon_shadow_query.iter_mut() {
        c.0 = c.0.with_alpha(shadow_alpha);
    }
    for mut c in current_ammo_query.iter_mut() {
        c.0 = c.0.with_alpha(opacity);
    }
    for mut c in current_shadow_query.iter_mut() {
        c.0 = c.0.with_alpha(shadow_alpha);
    }
    for mut c in reserve_ammo_query.iter_mut() {
        c.0 = c.0.with_alpha(opacity);
    }
    for mut c in reserve_shadow_query.iter_mut() {
        c.0 = c.0.with_alpha(shadow_alpha);
    }

    // 武器槽位亮度脈衝
    let brightness = calculate_slot_brightness(ease_out);
    for mut bg in slot_query.iter_mut() {
        bg.0 = apply_brightness_to_color(bg.0, brightness);
    }
}

// ============================================================================
// 準星動態效果系統
// ============================================================================
/// 更新準星動態效果（射擊展開、瞄準收縮、命中反彈）
pub fn update_crosshair_dynamics(
    time: Res<Time>,
    combat_state: Res<CombatState>,
    mut dynamics: ResMut<CrosshairDynamics>,
) {
    let dt = time.delta_secs();

    // 根據瞄準狀態設置目標散佈值
    dynamics.target_spread = if combat_state.is_aiming {
        CROSSHAIR_AIM_SHRINK
    } else {
        1.0
    };

    // 平滑過渡到目標散佈值
    let spread_diff = dynamics.target_spread - dynamics.current_spread;
    dynamics.current_spread += spread_diff * CROSSHAIR_RECOVERY_SPEED * dt;

    // 命中反彈恢復
    if dynamics.hit_bounce_scale > 1.0 {
        dynamics.hit_bounce_scale -=
            (dynamics.hit_bounce_scale - 1.0) * CROSSHAIR_HIT_BOUNCE_RECOVERY * dt;
        if dynamics.hit_bounce_scale < 1.01 {
            dynamics.hit_bounce_scale = 1.0;
        }
    }
}

/// 觸發準星射擊展開效果
#[allow(dead_code)]
pub fn trigger_crosshair_fire_expand(dynamics: &mut CrosshairDynamics) {
    dynamics.current_spread = (dynamics.current_spread * CROSSHAIR_FIRE_EXPAND).min(2.5);
}

/// 觸發準星命中反彈效果
#[allow(dead_code)]
pub fn trigger_crosshair_hit_bounce(dynamics: &mut CrosshairDynamics) {
    dynamics.hit_bounce_scale = CROSSHAIR_HIT_BOUNCE_SCALE;
}
