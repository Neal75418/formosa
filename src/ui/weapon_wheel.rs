//! 武器輪盤系統 (GTA 5 風格)
//!
//! 按 Tab 打開武器輪盤進行武器選擇

use bevy::prelude::*;

use super::components::{
    ChineseFont, UiState, WeaponWheel, WeaponWheelAmmo, WeaponWheelBackground,
    WeaponWheelCenterInfo, WeaponWheelIcon, WeaponWheelName, WeaponWheelSelector,
    WeaponWheelSlot, WeaponWheelState,
};
use super::constants::BUTTON_BORDER_GRAY_60;
use crate::combat::WeaponInventory;
use crate::player::Player;

// === 武器輪盤顏色常數 ===
const WEAPON_WHEEL_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const WEAPON_WHEEL_SLOT_NORMAL: Color = Color::srgba(0.2, 0.2, 0.25, 0.8);
const WEAPON_WHEEL_SLOT_SELECTED: Color = Color::srgba(0.85, 0.75, 0.3, 0.9);
#[allow(dead_code)]
const WEAPON_WHEEL_SLOT_EMPTY: Color = Color::srgba(0.15, 0.15, 0.18, 0.5);
const WEAPON_WHEEL_TEXT: Color = Color::srgb(0.95, 0.95, 0.95);
const WEAPON_WHEEL_AMMO: Color = Color::srgb(0.85, 0.85, 0.3);

/// 設置武器輪盤 UI
pub fn setup_weapon_wheel(mut commands: Commands, font: Option<Res<ChineseFont>>) {
    let Some(font) = font else { return };

    // 主容器（隱藏狀態開始）
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(WEAPON_WHEEL_BG),
            Visibility::Hidden,
            WeaponWheel,
            Name::new("WeaponWheel"),
        ))
        .with_children(|parent| {
            // 武器輪盤容器
            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(400.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    WeaponWheelBackground,
                ))
                .with_children(|wheel| {
                    // 6 個武器槽位
                    for i in 0..6 {
                        let angle = WeaponWheelState::slot_angle(i);
                        let radius = 140.0;
                        let x = angle.cos() * radius;
                        let y = angle.sin() * radius;

                        wheel
                            .spawn((
                                Node {
                                    width: Val::Px(70.0),
                                    height: Val::Px(70.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(200.0 + x - 35.0),
                                    top: Val::Px(200.0 + y - 35.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(WEAPON_WHEEL_SLOT_NORMAL),
                                BorderColor::all(BUTTON_BORDER_GRAY_60),
                                BorderRadius::all(Val::Px(35.0)), // 圓形
                                WeaponWheelSlot {
                                    index: i,
                                    angle,
                                    is_selected: false,
                                },
                            ))
                            .with_children(|slot| {
                                // 武器圖示
                                slot.spawn((
                                    Text::new(weapon_slot_icon(i)),
                                    TextFont {
                                        font: font.font.clone(),
                                        font_size: 28.0,
                                        ..default()
                                    },
                                    TextColor(WEAPON_WHEEL_TEXT),
                                    WeaponWheelIcon { slot_index: i },
                                ));
                            });
                    }

                    // 中央資訊區域
                    wheel
                        .spawn((
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(80.0),
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            WeaponWheelCenterInfo,
                        ))
                        .with_children(|center| {
                            // 武器名稱
                            center.spawn((
                                Text::new("拳頭"),
                                TextFont {
                                    font: font.font.clone(),
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(WEAPON_WHEEL_TEXT),
                                WeaponWheelName,
                            ));
                            // 彈藥資訊
                            center.spawn((
                                Text::new("∞"),
                                TextFont {
                                    font: font.font.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(WEAPON_WHEEL_AMMO),
                                WeaponWheelAmmo,
                            ));
                        });

                    // 選擇指示器
                    wheel.spawn((
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(80.0),
                            position_type: PositionType::Absolute,
                            left: Val::Px(160.0),
                            top: Val::Px(160.0),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(WEAPON_WHEEL_SLOT_SELECTED),
                        BorderRadius::all(Val::Px(40.0)),
                        WeaponWheelSelector,
                    ));
                });
        });
}

/// 取得武器槽位圖示
fn weapon_slot_icon(slot: usize) -> &'static str {
    match slot {
        0 => "👊", // 拳頭
        1 => "🔫", // 手槍
        2 => "🔫", // 衝鋒槍
        3 => "🎯", // 霰彈槍
        4 => "🎯", // 步槍
        5 => "💣", // 空槽位
        _ => "❓",
    }
}

/// 取得武器輪盤顯示資訊（名稱、彈藥）
fn get_wheel_weapon_info(inventory: &WeaponInventory, slot_index: usize) -> (String, String) {
    if slot_index < inventory.weapons.len() {
        let weapon = &inventory.weapons[slot_index];
        let name = weapon.stats.weapon_type.name().to_string();
        let ammo = if weapon.stats.magazine_size == 0 {
            "∞".to_string()
        } else {
            format!("{} / {}", weapon.current_ammo, weapon.reserve_ammo)
        };
        (name, ammo)
    } else {
        ("空槽位".to_string(), "-".to_string())
    }
}

/// 確認武器選擇並切換
fn confirm_weapon_selection(inventory: &mut WeaponInventory, selected_index: usize) {
    if selected_index < inventory.weapons.len() {
        inventory.current_index = selected_index;
    }
}

/// 武器輪盤輸入系統
pub fn weapon_wheel_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut wheel_state: ResMut<WeaponWheelState>,
    mut wheel_query: Query<&mut Visibility, With<WeaponWheel>>,
    windows: Query<&Window>,
    mut player_query: Query<&mut WeaponInventory, With<Player>>,
) {
    // Tab 鍵打開武器輪盤
    if keyboard.just_pressed(KeyCode::Tab) {
        ui_state.show_weapon_wheel = true;
        wheel_state.is_animating = true;
        wheel_state.open_progress = 0.0;
        for mut vis in wheel_query.iter_mut() {
            *vis = Visibility::Visible;
        }
    }

    // Tab 鍵釋放關閉並確認選擇
    if keyboard.just_released(KeyCode::Tab) {
        if ui_state.show_weapon_wheel {
            if let Ok(mut inventory) = player_query.single_mut() {
                confirm_weapon_selection(&mut inventory, wheel_state.selected_index);
            }
        }
        ui_state.show_weapon_wheel = false;
        for mut vis in wheel_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }

    // 更新滑鼠位置選擇
    if !ui_state.show_weapon_wheel {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let offset = cursor_pos - center;
    wheel_state.update_selection(Vec2::new(offset.x, -offset.y));
}

/// 武器輪盤更新系統
#[allow(clippy::type_complexity)]
pub fn weapon_wheel_update_system(
    time: Res<Time>,
    ui_state: Res<UiState>,
    mut wheel_state: ResMut<WeaponWheelState>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut slot_query: Query<(&mut WeaponWheelSlot, &mut BackgroundColor, &mut BorderColor)>,
    mut selector_query: Query<&mut Node, With<WeaponWheelSelector>>,
    mut name_query: Query<&mut Text, (With<WeaponWheelName>, Without<WeaponWheelAmmo>)>,
    mut ammo_query: Query<&mut Text, (With<WeaponWheelAmmo>, Without<WeaponWheelName>)>,
) {
    if !ui_state.show_weapon_wheel {
        return;
    }

    let dt = time.delta_secs();

    // 更新打開動畫
    if wheel_state.is_animating {
        wheel_state.open_progress = (wheel_state.open_progress + dt * 5.0).min(1.0);
        if wheel_state.open_progress >= 1.0 {
            wheel_state.is_animating = false;
        }
    }

    let selected = wheel_state.selected_index;

    // 更新槽位高亮
    for (mut slot, mut bg, mut border) in slot_query.iter_mut() {
        slot.is_selected = slot.index == selected;
        if slot.is_selected {
            *bg = BackgroundColor(WEAPON_WHEEL_SLOT_SELECTED);
            *border = BorderColor::all(Color::srgba(1.0, 0.9, 0.5, 0.9));
        } else {
            *bg = BackgroundColor(WEAPON_WHEEL_SLOT_NORMAL);
            *border = BorderColor::all(BUTTON_BORDER_GRAY_60);
        }
    }

    // 更新選擇指示器位置
    let angle = WeaponWheelState::slot_angle(selected);
    let radius = 140.0;
    let x = angle.cos() * radius;
    let y = angle.sin() * radius;
    for mut node in selector_query.iter_mut() {
        node.left = Val::Px(200.0 + x - 40.0);
        node.top = Val::Px(200.0 + y - 40.0);
    }

    // 更新中央資訊
    if let Ok(inventory) = player_query.single() {
        let (weapon_name, ammo_text) = get_wheel_weapon_info(inventory, selected);
        if let Ok(mut text) = name_query.single_mut() {
            **text = weapon_name;
        }
        if let Ok(mut text) = ammo_query.single_mut() {
            **text = ammo_text;
        }
    }
}

/// 武器輪盤圖示更新系統
pub fn weapon_wheel_icon_update_system(
    ui_state: Res<UiState>,
    player_query: Query<&WeaponInventory, With<Player>>,
    mut icon_query: Query<(&WeaponWheelIcon, &mut Text)>,
) {
    if !ui_state.show_weapon_wheel {
        return;
    }

    if let Ok(inventory) = player_query.single() {
        for (icon, mut text) in icon_query.iter_mut() {
            let slot = icon.slot_index;
            if slot < inventory.weapons.len() {
                let weapon_type = inventory.weapons[slot].stats.weapon_type;
                **text = weapon_type.icon().to_string();
            } else {
                **text = "—".to_string(); // 空槽位
            }
        }
    }
}
