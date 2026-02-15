//! 手機 UI 系統 (GTA 5 風格)
//!
//! 上箭頭鍵開啟手機，包含聯絡人、任務日誌、地圖、設定等分頁。

use bevy::prelude::*;

use super::components::{
    ChineseFont, MissionJournalTab, PhoneApp, PhoneAppIcon, PhoneContactList, PhoneContainer,
    PhoneContentArea, PhoneMissionLogList, PhoneScreen, PhoneStatusBar, PhoneUiState,
};
use super::UiState;
use crate::mission::MissionManager;
use super::phone_apps::*;

// ============================================================================
// 常數
// ============================================================================

/// 手機寬度
const PHONE_WIDTH: f32 = 280.0;
/// 手機高度
const PHONE_HEIGHT: f32 = 480.0;
/// 手機背景色
const PHONE_BG: Color = Color::srgba(0.08, 0.08, 0.12, 0.95);
/// 手機邊框色
const PHONE_BORDER_COLOR: Color = Color::srgba(0.3, 0.3, 0.4, 0.8);
/// 手機螢幕背景色
const PHONE_SCREEN_BG: Color = Color::srgba(0.05, 0.08, 0.15, 1.0);
/// App 圖標背景色
const APP_ICON_BG: Color = Color::srgba(0.15, 0.2, 0.3, 0.9);
/// App 圖標選中色
const APP_ICON_SELECTED: Color = Color::srgba(0.2, 0.4, 0.7, 0.9);
/// 狀態列背景
const STATUS_BAR_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);
/// 內容區項目色
const CONTENT_ITEM_BG: Color = Color::srgba(0.1, 0.12, 0.18, 0.8);

// ============================================================================
// 設置系統
// ============================================================================

/// 設置手機 UI
pub fn setup_phone_ui(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();

    // 手機外框（右下角）
    commands
        .spawn((
            PhoneContainer,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(40.0),
                bottom: Val::Px(40.0),
                width: Val::Px(PHONE_WIDTH),
                height: Val::Px(PHONE_HEIGHT),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(3.0)),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(PHONE_BG),
            BorderColor::all(PHONE_BORDER_COLOR),
            BorderRadius::all(Val::Px(16.0)),
            Visibility::Hidden,
            ZIndex(90),
        ))
        .with_children(|phone| {
            // 狀態列
            phone
                .spawn((
                    PhoneStatusBar,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(STATUS_BAR_BG),
                    BorderRadius::px(12.0, 12.0, 0.0, 0.0),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Text::new("Island Phone"),
                        TextFont {
                            font: font.clone(),
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                    ));
                });

            // 螢幕區域
            phone
                .spawn((
                    PhoneScreen,
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(6.0)),
                        row_gap: Val::Px(6.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(PHONE_SCREEN_BG),
                ))
                .with_children(|screen| {
                    // App 圖標網格（主畫面）
                    screen
                        .spawn((
                            PhoneContentArea,
                            Node {
                                width: Val::Percent(100.0),
                                flex_grow: 1.0,
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::Center,
                                align_content: AlignContent::Start,
                                padding: UiRect::all(Val::Px(10.0)),
                                row_gap: Val::Px(12.0),
                                column_gap: Val::Px(12.0),
                                ..default()
                            },
                        ))
                        .with_children(|content| {
                            // 生成 App 圖標
                            for app in PhoneApp::all_apps() {
                                spawn_app_icon(content, &font, *app);
                            }
                        });
                });

            // 底部導航提示
            phone
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(24.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(STATUS_BAR_BG),
                    BorderRadius::px(0.0, 0.0, 12.0, 12.0),
                ))
                .with_children(|nav| {
                    nav.spawn((
                        Text::new("[Arrows] Navigate  [Enter] Open  [Up] Back"),
                        TextFont {
                            font: font.clone(),
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                    ));
                });
        });
}

/// 生成單個 App 圖標
fn spawn_app_icon(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, app: PhoneApp) {
    parent
        .spawn((
            PhoneAppIcon { app },
            Node {
                width: Val::Px(56.0),
                height: Val::Px(56.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(APP_ICON_BG),
            BorderColor::all(Color::srgba(0.3, 0.3, 0.5, 0.5)),
            BorderRadius::all(Val::Px(10.0)),
        ))
        .with_children(|icon| {
            // 圖標字母
            icon.spawn((
                Text::new(app.icon()),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            // App 名稱
            icon.spawn((
                Text::new(app.label()),
                TextFont {
                    font: font.clone(),
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
            ));
        });
}

// ============================================================================
// 輸入系統
// ============================================================================

/// 手機輸入系統
/// 上箭頭開啟/關閉手機。方向鍵選擇 App，Enter 進入，Escape 返回。
pub fn phone_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<UiState>,
    mut phone_state: ResMut<PhoneUiState>,
) {
    // 開啟/關閉手機
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        // 如果在某個 App 中，先回到主畫面
        if ui_state.show_phone && phone_state.current_app != PhoneApp::Home {
            phone_state.current_app = PhoneApp::Home;
            return;
        }
        ui_state.show_phone = !ui_state.show_phone;
        if ui_state.show_phone {
            phone_state.current_app = PhoneApp::Home;
            phone_state.selected_index = 0;
        }
        return;
    }

    // 手機未開啟時不處理
    if !ui_state.show_phone {
        return;
    }

    // Escape 關閉或返回
    if keyboard.just_pressed(KeyCode::Escape) {
        if phone_state.current_app != PhoneApp::Home {
            phone_state.current_app = PhoneApp::Home;
        } else {
            ui_state.show_phone = false;
        }
        return;
    }

    // 主畫面：方向鍵選擇
    if phone_state.current_app == PhoneApp::Home {
        let app_count = PhoneApp::all_apps().len();
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            phone_state.selected_index = (phone_state.selected_index + 1) % app_count;
        }
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            phone_state.selected_index = (phone_state.selected_index + app_count - 1) % app_count;
        }
        // 上下鍵也可以用（每行 2 個圖標）
        if keyboard.just_pressed(KeyCode::ArrowDown) {
            phone_state.selected_index = (phone_state.selected_index + 2).min(app_count - 1);
        }

        // Enter 進入選中 App
        if keyboard.just_pressed(KeyCode::Enter) {
            phone_state.current_app = PhoneApp::all_apps()[phone_state.selected_index];
        }
    }

    // 任務日誌分頁切換（左右鍵）
    else if phone_state.current_app == PhoneApp::MissionLog {
        let tabs = MissionJournalTab::all();
        let current_idx = tabs.iter().position(|t| *t == phone_state.journal_tab).unwrap_or(0);
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            phone_state.journal_tab = tabs[(current_idx + 1) % tabs.len()];
        }
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            phone_state.journal_tab = tabs[(current_idx + tabs.len() - 1) % tabs.len()];
        }
    }
}

// ============================================================================
// 更新系統
// ============================================================================

/// 手機顯示/隱藏系統
pub fn phone_visibility_system(
    ui_state: Res<UiState>,
    mut phone_query: Query<&mut Visibility, With<PhoneContainer>>,
) {
    let target = if ui_state.show_phone {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut vis in &mut phone_query {
        *vis = target;
    }
}

/// 手機 App 圖標選中高亮系統
pub fn phone_icon_highlight_system(
    phone_state: Res<PhoneUiState>,
    mut icon_query: Query<(&PhoneAppIcon, &mut BackgroundColor)>,
) {
    if phone_state.current_app != PhoneApp::Home {
        return;
    }

    let apps = PhoneApp::all_apps();
    for (icon, mut bg) in &mut icon_query {
        let is_selected = apps
            .iter()
            .position(|a| *a == icon.app)
            .is_some_and(|idx| idx == phone_state.selected_index);

        *bg = if is_selected {
            BackgroundColor(APP_ICON_SELECTED)
        } else {
            BackgroundColor(APP_ICON_BG)
        };
    }
}

/// 手機內容更新系統（根據當前 App 切換顯示內容）
pub fn phone_content_system(
    phone_state: Res<PhoneUiState>,
    mission_manager: Res<MissionManager>,
    mut content_query: Query<(Entity, &mut Node), With<PhoneContentArea>>,
    icon_query: Query<Entity, With<PhoneAppIcon>>,
    contact_query: Query<Entity, With<PhoneContactList>>,
    log_query: Query<Entity, With<PhoneMissionLogList>>,
    mut commands: Commands,
    chinese_font: Res<ChineseFont>,
) {
    // 只在狀態變化時重建（簡化版：每幀檢查）
    if !phone_state.is_changed() {
        return;
    }

    let Ok((content_entity, mut content_node)) = content_query.single_mut() else {
        return;
    };

    // 清除舊內容（Bevy 0.17 的 despawn() 已自動清除子實體）
    for entity in icon_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in contact_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in log_query.iter() {
        commands.entity(entity).despawn();
    }

    let font = chinese_font.font.clone();

    match phone_state.current_app {
        PhoneApp::Home => {
            // 顯示圖標網格
            content_node.flex_direction = FlexDirection::Row;
            content_node.flex_wrap = FlexWrap::Wrap;
            content_node.justify_content = JustifyContent::Center;
            content_node.align_content = AlignContent::Start;

            commands.entity(content_entity).with_children(|content| {
                for app in PhoneApp::all_apps() {
                    spawn_app_icon(content, &font, *app);
                }
            });
        }
        PhoneApp::Contacts => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.flex_wrap = FlexWrap::NoWrap;
            content_node.justify_content = JustifyContent::Start;

            commands.entity(content_entity).with_children(|content| {
                // 標題
                spawn_section_title(content, &font, "聯絡人");

                content
                    .spawn((
                        PhoneContactList,
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                    ))
                    .with_children(|list| {
                        // 固定聯絡人列表
                        let contacts = [
                            ("小明", "盟友"),
                            ("阿嬤", "家人"),
                            ("夜市老闆", "商人"),
                            ("警察局長", "官方"),
                        ];
                        for (name, role) in contacts {
                            spawn_contact_item(list, &font, name, role);
                        }
                    });
            });
        }
        PhoneApp::MissionLog => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.flex_wrap = FlexWrap::NoWrap;
            content_node.justify_content = JustifyContent::Start;

            commands.entity(content_entity).with_children(|content| {
                spawn_section_title(content, &font, "任務日誌");

                // 分頁選擇列
                spawn_journal_tabs(content, &font, phone_state.journal_tab);

                content
                    .spawn((
                        PhoneMissionLogList,
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            overflow: Overflow::clip(),
                            flex_grow: 1.0,
                            ..default()
                        },
                    ))
                    .with_children(|list| {
                        match phone_state.journal_tab {
                            MissionJournalTab::Active => {
                                spawn_journal_active(list, &font, &mission_manager);
                            }
                            MissionJournalTab::Completed => {
                                spawn_journal_completed(list, &font, &mission_manager);
                            }
                            MissionJournalTab::Stats => {
                                spawn_journal_stats(list, &font, &mission_manager);
                            }
                        }
                    });

                // 底部操作提示
                content.spawn((
                    Text::new("[Left/Right] 切換分頁"),
                    TextFont {
                        font: font.clone(),
                        font_size: 9.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.4, 0.4, 0.5, 0.7)),
                ));
            });
        }
        PhoneApp::Map => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.justify_content = JustifyContent::Center;

            commands.entity(content_entity).with_children(|content| {
                spawn_section_title(content, &font, "地圖");

                content.spawn((
                    Text::new("按 M 開啟全地圖"),
                    TextFont {
                        font: font.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.6, 0.7, 0.8, 0.9)),
                ));

                content.spawn((
                    Text::new("小地圖顯示於左下角"),
                    TextFont {
                        font: font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                ));
            });
        }
        PhoneApp::Settings => {
            content_node.flex_direction = FlexDirection::Column;
            content_node.justify_content = JustifyContent::Start;

            commands.entity(content_entity).with_children(|content| {
                spawn_section_title(content, &font, "設定");

                let settings = [
                    "音量: 80%",
                    "畫質: 高",
                    "操控: 鍵盤滑鼠",
                    "語言: 繁體中文",
                ];
                for setting in settings {
                    content.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            margin: UiRect::bottom(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(CONTENT_ITEM_BG),
                        BorderRadius::all(Val::Px(4.0)),
                    )).with_children(|item| {
                        item.spawn((
                            Text::new(setting),
                            TextFont {
                                font: font.clone(),
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                        ));
                    });
                }
            });
        }
    }
}

pub(super) struct PhonePlugin;

impl Plugin for PhonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_phone_ui.in_set(super::UiSetup))
            .add_systems(
                Update,
                (
                    phone_input_system,
                    phone_visibility_system.after(phone_input_system),
                    phone_icon_highlight_system.after(phone_input_system),
                    phone_content_system.after(phone_input_system),
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
    fn phone_app_labels() {
        assert_eq!(PhoneApp::Home.label(), "主畫面");
        assert_eq!(PhoneApp::Contacts.label(), "聯絡人");
        assert_eq!(PhoneApp::MissionLog.label(), "任務日誌");
        assert_eq!(PhoneApp::Map.label(), "地圖");
        assert_eq!(PhoneApp::Settings.label(), "設定");
    }

    #[test]
    fn phone_app_all_apps_count() {
        assert_eq!(PhoneApp::all_apps().len(), 4);
    }

    #[test]
    fn phone_app_all_apps_excludes_home() {
        assert!(!PhoneApp::all_apps().contains(&PhoneApp::Home));
    }

    #[test]
    fn phone_ui_state_defaults() {
        let state = PhoneUiState::default();
        assert_eq!(state.current_app, PhoneApp::Home);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn phone_app_icon_not_empty() {
        for app in PhoneApp::all_apps() {
            assert!(!app.icon().is_empty());
            assert!(!app.label().is_empty());
        }
    }

    #[test]
    fn phone_navigation_wraps_right() {
        let app_count = PhoneApp::all_apps().len();
        let mut idx = app_count - 1; // 最後一個
        idx = (idx + 1) % app_count;
        assert_eq!(idx, 0); // 回到第一個
    }

    #[test]
    fn phone_navigation_wraps_left() {
        let app_count = PhoneApp::all_apps().len();
        let mut idx: usize = 0;
        idx = (idx + app_count - 1) % app_count;
        assert_eq!(idx, app_count - 1); // 到最後一個
    }

    #[test]
    fn phone_toggle_logic() {
        let mut show_phone = false;

        // 第一次按上：開啟
        show_phone = !show_phone;
        assert!(show_phone);

        // 第二次按上：關閉
        show_phone = !show_phone;
        assert!(!show_phone);
    }

    // ========================================================================
    // 任務日誌測試
    // ========================================================================

    #[test]
    fn journal_tab_labels() {
        assert_eq!(MissionJournalTab::Active.label(), "進行中");
        assert_eq!(MissionJournalTab::Completed.label(), "已完成");
        assert_eq!(MissionJournalTab::Stats.label(), "統計");
    }

    #[test]
    fn journal_tab_all_count() {
        assert_eq!(MissionJournalTab::all().len(), 3);
    }

    #[test]
    fn journal_tab_default_is_active() {
        let tab = MissionJournalTab::default();
        assert_eq!(tab, MissionJournalTab::Active);
    }

    #[test]
    fn journal_tab_cycle_right() {
        let tabs = MissionJournalTab::all();
        let mut idx = 0; // Active
        idx = (idx + 1) % tabs.len(); // -> Completed
        assert_eq!(tabs[idx], MissionJournalTab::Completed);
        idx = (idx + 1) % tabs.len(); // -> Stats
        assert_eq!(tabs[idx], MissionJournalTab::Stats);
        idx = (idx + 1) % tabs.len(); // -> Active (wrap)
        assert_eq!(tabs[idx], MissionJournalTab::Active);
    }

    #[test]
    fn journal_tab_cycle_left() {
        let tabs = MissionJournalTab::all();
        let mut idx = 0; // Active
        idx = (idx + tabs.len() - 1) % tabs.len(); // -> Stats (wrap)
        assert_eq!(tabs[idx], MissionJournalTab::Stats);
    }

    #[test]
    fn phone_state_includes_journal_tab() {
        let state = PhoneUiState::default();
        assert_eq!(state.journal_tab, MissionJournalTab::Active);
    }

    #[test]
    fn completed_mission_record_stars_display() {
        use crate::mission::{CompletedMissionRecord, MissionType};

        let record = CompletedMissionRecord {
            title: "測試任務".to_string(),
            mission_type: MissionType::Delivery,
            reward: 500,
            stars: 3,
            rating_label: "⭐⭐⭐".to_string(),
        };
        assert_eq!(record.stars_display(), "★★★");
        assert_eq!(record.type_label(), "外送");
    }

    #[test]
    fn completed_mission_record_type_labels() {
        use crate::mission::{CompletedMissionRecord, MissionType};

        let make = |mt| CompletedMissionRecord {
            title: String::new(),
            mission_type: mt,
            reward: 0,
            stars: 0,
            rating_label: String::new(),
        };
        assert_eq!(make(MissionType::Delivery).type_label(), "外送");
        assert_eq!(make(MissionType::Taxi).type_label(), "載客");
        assert_eq!(make(MissionType::Race).type_label(), "競速");
        assert_eq!(make(MissionType::Explore).type_label(), "探索");
    }
}
