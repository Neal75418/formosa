//! 存檔槽選擇 UI
//!
//! 10 格存檔槽選擇介面，支援存檔/讀檔模式切換。
//! F7 開啟存檔模式、F8 開啟讀檔模式、ESC 關閉。

use bevy::prelude::*;
use bevy::time::Real;

use super::components::{
    ButtonScaleState, SaveSlotCard, SaveSlotContainer, SaveSlotDetail, SaveSlotMode,
    SaveSlotModeTab, SaveSlotUiState, UiState,
};
#[allow(clippy::wildcard_imports)]
use super::constants::*;
use super::systems::{spawn_full_screen_overlay, spawn_key_hint};
use crate::save::{LoadGameEvent, LoadType, SaveData, SaveGameEvent, SaveManager, SaveType};

/// 存檔槽卡片寬度
const SAVE_CARD_WIDTH: f32 = 300.0;
/// 存檔槽卡片高度
const SAVE_CARD_HEIGHT: f32 = 70.0;
/// 存檔面板寬度
const SAVE_PANEL_WIDTH: f32 = 700.0;

// ============================================================================
// 設置系統
// ============================================================================

/// 設置存檔槽 UI 面板（由 `setup_ui` 呼叫，初始隱藏）
pub(super) fn setup_save_slot_ui_panel(commands: &mut Commands, font: &Handle<Font>) {
    let f = font.clone();

    spawn_full_screen_overlay(
        commands,
        PAUSE_BG_OUTER,
        SaveSlotContainer,
        FlexDirection::Row,
        Val::Px(0.0),
    )
    .with_children(|parent| {
        // 面板外發光層
        parent
            .spawn((
                Node {
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(PAUSE_PANEL_GLOW),
                BorderRadius::all(Val::Px(14.0)),
            ))
            .with_children(|glow| {
                // 面板主邊框層
                glow.spawn((
                    Node {
                        padding: UiRect::all(Val::Px(2.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(PAUSE_PANEL_BORDER),
                    BorderColor::all(Color::srgba(0.5, 0.55, 0.6, 0.6)),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|border| {
                    // 面板內容區
                    border
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::new(
                                    Val::Px(30.0),
                                    Val::Px(30.0),
                                    Val::Px(25.0),
                                    Val::Px(20.0),
                                ),
                                row_gap: Val::Px(12.0),
                                align_items: AlignItems::Center,
                                width: Val::Px(SAVE_PANEL_WIDTH),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(PAUSE_PANEL_BG),
                            BorderColor::all(PAUSE_PANEL_INNER_BORDER),
                            BorderRadius::all(Val::Px(8.0)),
                        ))
                        .with_children(|panel| {
                            spawn_title(panel, &f);
                            spawn_mode_tabs(panel, &f);
                            spawn_separator(panel);
                            spawn_slot_grid(panel, &f);
                            spawn_separator(panel);
                            spawn_bottom_hints(panel, &f);
                        });
                });
            });
    });
}

fn spawn_title(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent.spawn((
        Text::new("存檔管理"),
        TextFont {
            font_size: 28.0,
            font: font.clone(),
            ..default()
        },
        TextColor(PAUSE_TITLE_COLOR),
    ));
}

fn spawn_mode_tabs(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            spawn_tab_button(row, "存檔", SaveSlotMode::Save, font, true);
            spawn_tab_button(row, "讀檔", SaveSlotMode::Load, font, false);
        });
}

fn spawn_tab_button(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    mode: SaveSlotMode,
    font: &Handle<Font>,
    active: bool,
) {
    let bg = if active {
        SAVE_SLOT_ACTIVE_TAB
    } else {
        SAVE_SLOT_INACTIVE_TAB
    };

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(100.0),
                height: Val::Px(36.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(bg),
            BorderRadius::all(Val::Px(6.0)),
            SaveSlotModeTab { mode },
            ButtonScaleState::default(),
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(text),
                TextFont {
                    font_size: 18.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_separator(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            width: Val::Percent(90.0),
            height: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.3, 0.3, 0.35, 0.4)),
    ));
}

fn spawn_slot_grid(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|grid| {
            for row in 0..5 {
                grid.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    ..default()
                })
                .with_children(|row_node| {
                    spawn_slot_card(row_node, row * 2, font);
                    spawn_slot_card(row_node, row * 2 + 1, font);
                });
            }
        });
}

fn spawn_slot_card(parent: &mut ChildSpawnerCommands, slot: usize, font: &Handle<Font>) {
    // 卡片邊框
    parent
        .spawn((
            Node {
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(SAVE_SLOT_CARD_BORDER),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|border| {
            // 卡片按鈕
            border
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(SAVE_CARD_WIDTH),
                        height: Val::Px(SAVE_CARD_HEIGHT),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::new(
                            Val::Px(12.0),
                            Val::Px(12.0),
                            Val::Px(8.0),
                            Val::Px(8.0),
                        ),
                        row_gap: Val::Px(4.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(SAVE_SLOT_CARD_BG),
                    BorderRadius::all(Val::Px(4.0)),
                    SaveSlotCard { slot },
                    ButtonScaleState::default(),
                ))
                .with_children(|card| {
                    // 第一行：槽位名稱
                    card.spawn((
                        Text::new(format!("存檔 #{:02}", slot + 1)),
                        TextFont {
                            font_size: 16.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(SAVE_SLOT_TITLE_COLOR),
                    ));

                    // 第二行：詳細資訊（初始顯示「空白」）
                    card.spawn((
                        Text::new("-- 空白 --"),
                        TextFont {
                            font_size: 13.0,
                            font: font.clone(),
                            ..default()
                        },
                        TextColor(SAVE_SLOT_EMPTY_TEXT),
                        SaveSlotDetail { slot },
                    ));
                });
        });
}

fn spawn_bottom_hints(parent: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(20.0),
            ..default()
        })
        .with_children(|row| {
            spawn_key_hint(row, "ESC", "關閉", 6.0, font);
            spawn_key_hint(row, "F7", "存檔", 6.0, font);
            spawn_key_hint(row, "F8", "讀檔", 6.0, font);
        });
}

// ============================================================================
// 輸入系統
// ============================================================================

/// 切換存檔槽模式：若已開啟同模式則關閉，否則切換至目標模式
fn toggle_save_slot_mode(
    is_visible: bool,
    target_mode: SaveSlotMode,
    save_slot_state: &mut SaveSlotUiState,
    ui_state: &mut UiState,
    visibility: &mut Visibility,
) {
    if is_visible && save_slot_state.mode == target_mode {
        *visibility = Visibility::Hidden;
        ui_state.show_save_slots = false;
    } else {
        save_slot_state.mode = target_mode;
        save_slot_state.needs_refresh = true;
        *visibility = Visibility::Visible;
        ui_state.show_save_slots = true;
    }
}

/// 存檔槽 UI 開關（F7 存檔、F8 讀檔、ESC 關閉）
pub fn save_slot_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut save_slot_state: ResMut<SaveSlotUiState>,
    mut ui_state: ResMut<UiState>,
    mut container_query: Query<&mut Visibility, With<SaveSlotContainer>>,
) {
    let Ok(mut visibility) = container_query.single_mut() else {
        return;
    };

    let is_visible = *visibility == Visibility::Visible;

    // ESC 關閉
    if is_visible && keyboard.just_pressed(KeyCode::Escape) {
        *visibility = Visibility::Hidden;
        ui_state.show_save_slots = false;
        return;
    }

    // F7 = 存檔模式切換
    if keyboard.just_pressed(KeyCode::F7) {
        toggle_save_slot_mode(
            is_visible,
            SaveSlotMode::Save,
            &mut save_slot_state,
            &mut ui_state,
            &mut visibility,
        );
        return;
    }

    // F8 = 讀檔模式切換
    if keyboard.just_pressed(KeyCode::F8) {
        toggle_save_slot_mode(
            is_visible,
            SaveSlotMode::Load,
            &mut save_slot_state,
            &mut ui_state,
            &mut visibility,
        );
    }
}

// ============================================================================
// 更新系統
// ============================================================================

/// 模式分頁切換 + 分頁顏色更新
pub fn save_slot_tab_system(
    mut save_slot_state: ResMut<SaveSlotUiState>,
    tab_interaction_query: Query<(&Interaction, &SaveSlotModeTab), Changed<Interaction>>,
    mut tab_bg_query: Query<(&SaveSlotModeTab, &mut BackgroundColor)>,
) {
    for (interaction, tab) in &tab_interaction_query {
        if *interaction == Interaction::Pressed && tab.mode != save_slot_state.mode {
            save_slot_state.mode = tab.mode;
            save_slot_state.needs_refresh = true;
        }
    }

    for (tab, mut bg) in &mut tab_bg_query {
        *bg = if tab.mode == save_slot_state.mode {
            BackgroundColor(SAVE_SLOT_ACTIVE_TAB)
        } else {
            BackgroundColor(SAVE_SLOT_INACTIVE_TAB)
        };
    }
}

/// 掃描存檔檔案並刷新 UI 文字
pub fn save_slot_refresh_system(
    mut save_slot_state: ResMut<SaveSlotUiState>,
    save_manager: Res<SaveManager>,
    container_query: Query<&Visibility, With<SaveSlotContainer>>,
    mut detail_query: Query<(&SaveSlotDetail, &mut Text, &mut TextColor)>,
) {
    let Ok(visibility) = container_query.single() else {
        return;
    };

    if *visibility != Visibility::Visible || !save_slot_state.needs_refresh {
        return;
    }

    save_slot_state.needs_refresh = false;

    // 掃描 10 個存檔槽
    for slot in 0..10 {
        let path = save_manager.get_save_path(slot);
        save_slot_state.slot_cache[slot] = scan_save_file(&path);
    }

    // 更新 UI 文字
    for (detail, mut text, mut color) in &mut detail_query {
        if detail.slot >= 10 {
            continue;
        }
        if let Some(entry) = &save_slot_state.slot_cache[detail.slot] {
            let date = format_timestamp(entry.timestamp);
            let play = format_play_time(entry.play_time_secs);
            *text = Text::new(format!("{} | {} | ${}", date, play, entry.cash));
            *color = TextColor(TEXT_LIGHT_GRAY);
        } else {
            *text = Text::new("-- 空白 --");
            *color = TextColor(SAVE_SLOT_EMPTY_TEXT);
        }
    }
}

/// 存檔槽卡片點擊 → 觸發存檔/讀檔
pub fn save_slot_click_system(
    save_slot_state: Res<SaveSlotUiState>,
    card_query: Query<(&Interaction, &SaveSlotCard), Changed<Interaction>>,
    mut save_events: MessageWriter<SaveGameEvent>,
    mut load_events: MessageWriter<LoadGameEvent>,
    mut container_query: Query<&mut Visibility, With<SaveSlotContainer>>,
    mut ui_state: ResMut<UiState>,
    save_manager: Res<SaveManager>,
) {
    for (interaction, card) in &card_query {
        if *interaction != Interaction::Pressed || card.slot >= 10 {
            continue;
        }
        if save_manager.is_busy {
            warn!("存檔系統忙碌中，請稍候");
            continue;
        }

        match save_slot_state.mode {
            SaveSlotMode::Save => {
                save_events.write(SaveGameEvent {
                    save_type: SaveType::Slot,
                    slot: Some(card.slot),
                });
                info!("存檔至槽 #{:02}", card.slot + 1);
            }
            SaveSlotMode::Load => {
                if save_slot_state
                    .slot_cache
                    .get(card.slot)
                    .is_some_and(Option::is_some)
                {
                    load_events.write(LoadGameEvent {
                        load_type: LoadType::Slot,
                        slot: Some(card.slot),
                    });
                    info!("讀取槽 #{:02}", card.slot + 1);
                } else {
                    info!("存檔槽 #{:02} 為空白，無法讀取", card.slot + 1);
                    continue;
                }
            }
        }

        // 操作後關閉 UI
        if let Ok(mut vis) = container_query.single_mut() {
            *vis = Visibility::Hidden;
        }
        ui_state.show_save_slots = false;
    }
}

/// 存檔槽卡片懸停效果
pub fn save_slot_hover_system(
    time: Res<Time<Real>>,
    mut card_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut Node,
            &mut ButtonScaleState,
        ),
        With<SaveSlotCard>,
    >,
) {
    let lerp_speed = (time.delta_secs() * BUTTON_SCALE_SPEED).min(1.0);

    for (interaction, mut bg, mut node, mut scale_state) in &mut card_query {
        let (target, color) = match *interaction {
            Interaction::Pressed => (BUTTON_PRESSED_SCALE, SAVE_SLOT_CARD_PRESSED),
            Interaction::Hovered => (BUTTON_HOVER_SCALE, SAVE_SLOT_CARD_HOVER),
            Interaction::None => (1.0, SAVE_SLOT_CARD_BG),
        };

        *bg = BackgroundColor(color);
        scale_state.target = target;

        let diff = scale_state.target - scale_state.current;
        if diff.abs() > 0.001 {
            scale_state.current += diff * lerp_speed;
            node.width = Val::Px(SAVE_CARD_WIDTH * scale_state.current);
            node.height = Val::Px(SAVE_CARD_HEIGHT * scale_state.current);
        }
    }
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 掃描存檔檔案，提取槽位概要資訊
fn scan_save_file(path: &std::path::Path) -> Option<super::components::SlotCacheEntry> {
    let json = std::fs::read_to_string(path).ok()?;
    let data: SaveData = serde_json::from_str(&json).ok()?;
    Some(super::components::SlotCacheEntry {
        timestamp: data.timestamp,
        play_time_secs: data.play_time_secs,
        cash: data.player.cash,
        chapter: data.missions.current_chapter,
    })
}

/// 格式化遊玩時間
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn format_play_time(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    format!("{h}h {m:02}m")
}

/// 格式化 Unix 時間戳為 YYYY-MM-DD HH:MM
fn format_timestamp(timestamp: u64) -> String {
    if timestamp == 0 {
        return "未知日期".to_string();
    }
    let days = timestamp / 86400;
    let day_secs = timestamp % 86400;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}")
}

/// 將 Unix epoch 起算天數轉為 (年, 月, 日)
/// 基於 Howard Hinnant 的 `civil_from_days` 演算法
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

pub(super) struct SaveSlotPlugin;

impl Plugin for SaveSlotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                save_slot_input_system,
                save_slot_tab_system,
                save_slot_refresh_system,
                save_slot_click_system,
                save_slot_hover_system,
            )
                .in_set(super::UiActive),
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
    fn format_play_time_zero() {
        assert_eq!(format_play_time(0.0), "0h 00m");
    }

    #[test]
    fn format_play_time_hours_and_minutes() {
        // 2 hours 15 minutes = 8100 seconds
        assert_eq!(format_play_time(8100.0), "2h 15m");
    }

    #[test]
    fn format_play_time_large() {
        // 100 hours
        assert_eq!(format_play_time(360_000.0), "100h 00m");
    }

    #[test]
    fn format_timestamp_zero() {
        assert_eq!(format_timestamp(0), "未知日期");
    }

    #[test]
    fn format_timestamp_known_date() {
        // 2024-01-01 00:00 UTC = 1704067200
        let result = format_timestamp(1_704_067_200);
        assert_eq!(result, "2024-01-01 00:00");
    }

    #[test]
    fn format_timestamp_with_time() {
        // 1718458200 = 19889 days + 48600 secs = 13h 30m
        let result = format_timestamp(1_718_458_200);
        assert_eq!(result, "2024-06-15 13:30");
    }

    #[test]
    fn civil_from_days_epoch() {
        // Day 0 = 1970-01-01
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn civil_from_days_2024() {
        // 2024-01-01 = day 19723
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }

    #[test]
    fn save_slot_mode_equality() {
        assert_eq!(SaveSlotMode::Save, SaveSlotMode::Save);
        assert_ne!(SaveSlotMode::Save, SaveSlotMode::Load);
    }

    #[test]
    fn save_slot_ui_state_default() {
        let state = SaveSlotUiState::default();
        assert_eq!(state.mode, SaveSlotMode::Save);
        assert!(state.needs_refresh);
        assert_eq!(state.slot_cache.len(), 10);
        assert!(state.slot_cache.iter().all(std::option::Option::is_none));
    }
}
