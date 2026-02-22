//\! 手機 App 頁面渲染輔助函數
//\!
//\! 從 phone.rs 拆分，降低單檔行數。

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

use super::components::MissionJournalTab;
use crate::mission::MissionManager;

/// 內容項目背景色
const CONTENT_ITEM_BG: Color = Color::srgba(0.1, 0.12, 0.18, 0.8);

// ============================================================================
// 輔助函數
// ============================================================================

/// 生成內容列容器（寬 100%、橫向、兩端對齊、padding 8px、圓角 4px）
fn spawn_content_row<'a>(
    parent: &'a mut ChildSpawnerCommands,
    bg_color: Color,
) -> EntityCommands<'a> {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(bg_color),
        BorderRadius::all(Val::Px(4.0)),
    ))
}

pub(super) fn spawn_section_title(parent: &mut ChildSpawnerCommands, font: &Handle<Font>, title: &str) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(6.0), Val::Px(10.0)),
            ..default()
        },
    )).with_children(|row| {
        row.spawn((
            Text::new(title),
            TextFont {
                font: font.clone(),
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

pub(super) fn spawn_contact_item(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    name: &str,
    role: &str,
) {
    spawn_content_row(parent, CONTENT_ITEM_BG).with_children(|row| {
            row.spawn((
                Text::new(name),
                TextFont {
                    font: font.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            row.spawn((
                Text::new(role),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.6, 0.7, 0.8)),
            ));
        });
}

/// 生成任務日誌分頁選擇列
pub(super) fn spawn_journal_tabs(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    current_tab: MissionJournalTab,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceEvenly,
                padding: UiRect::vertical(Val::Px(4.0)),
                margin: UiRect::bottom(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.1, 0.8)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|row| {
            for tab in MissionJournalTab::all() {
                let is_selected = *tab == current_tab;
                let bg = if is_selected {
                    Color::srgba(0.2, 0.4, 0.7, 0.9)
                } else {
                    Color::NONE
                };
                row.spawn((
                    Node {
                        padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(3.0), Val::Px(3.0)),
                        ..default()
                    },
                    BackgroundColor(bg),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|tab_btn| {
                    tab_btn.spawn((
                        Text::new(tab.label()),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(if is_selected {
                            Color::WHITE
                        } else {
                            Color::srgba(0.5, 0.5, 0.6, 0.8)
                        }),
                    ));
                });
            }
        });
}

/// 生成「進行中」分頁內容
pub(super) fn spawn_journal_active(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    mission_manager: &MissionManager,
) {
    if let Some(current) = &mission_manager.active_mission {
        // 任務標題
        spawn_mission_item(parent, font, &current.data.title, "進行中", true);

        // 任務描述
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::new(Val::Px(12.0), Val::Px(8.0), Val::Px(2.0), Val::Px(6.0)),
                ..default()
            },
        )).with_children(|desc| {
            desc.spawn((
                Text::new(&current.data.description),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
            ));
        });

        // 任務類型和獎勵
        let type_label = match current.data.mission_type {
            crate::mission::MissionType::Delivery => "外送",
            crate::mission::MissionType::Taxi => "載客",
            crate::mission::MissionType::Race => "競速",
            crate::mission::MissionType::Explore => "探索",
            crate::mission::MissionType::Assassination => "暗殺",
            crate::mission::MissionType::Escort => "護送",
            crate::mission::MissionType::ChaseDown => "飛車追逐",
            crate::mission::MissionType::Photography => "拍照",
        };
        spawn_mission_detail_row(parent, font, "類型", type_label);
        spawn_mission_detail_row(parent, font, "獎勵", &format!("${}", current.data.reward));

        if let Some(time_limit) = current.data.time_limit {
            spawn_mission_detail_row(
                parent,
                font,
                "時限",
                &format!("{:.0} 秒", time_limit),
            );
        }
    } else {
        spawn_mission_item(parent, font, "目前沒有進行中的任務", "", false);
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
        )).with_children(|hint| {
            hint.spawn((
                Text::new("前往任務標記點接取任務"),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.4, 0.5, 0.6, 0.7)),
            ));
        });
    }
}

/// 生成「已完成」分頁內容
pub(super) fn spawn_journal_completed(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    mission_manager: &MissionManager,
) {
    if mission_manager.completed_missions.is_empty() {
        spawn_mission_item(parent, font, "尚無已完成任務", "", false);
        return;
    }

    // 顯示最近完成的任務（倒序，最多顯示 8 個）
    let missions = &mission_manager.completed_missions;
    let show_count = missions.len().min(8);
    for record in missions.iter().rev().take(show_count) {
        parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(CONTENT_ITEM_BG),
                BorderRadius::all(Val::Px(4.0)),
            ))
            .with_children(|card| {
                // 第一行：標題 + 類型
                card.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                )).with_children(|row| {
                    row.spawn((
                        Text::new(&record.title),
                        TextFont {
                            font: font.clone(),
                            font_size: 11.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
                        Text::new(record.type_label()),
                        TextFont {
                            font: font.clone(),
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.5, 0.6, 0.7, 0.8)),
                    ));
                });

                // 第二行：星級 + 獎勵
                card.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        margin: UiRect::top(Val::Px(2.0)),
                        ..default()
                    },
                )).with_children(|row| {
                    row.spawn((
                        Text::new(record.stars_display()),
                        TextFont {
                            font: font.clone(),
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 0.85, 0.0, 0.9)),
                    ));
                    row.spawn((
                        Text::new(format!("${}", record.reward)),
                        TextFont {
                            font: font.clone(),
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.3, 0.8, 0.4, 0.9)),
                    ));
                });
            });
    }

    if missions.len() > show_count {
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(4.0)),
                justify_content: JustifyContent::Center,
                ..default()
            },
        )).with_children(|more| {
            more.spawn((
                Text::new(format!("...還有 {} 個任務", missions.len() - show_count)),
                TextFont {
                    font: font.clone(),
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgba(0.4, 0.4, 0.5, 0.7)),
            ));
        });
    }
}

/// 生成「統計」分頁內容
pub(super) fn spawn_journal_stats(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    mission_manager: &MissionManager,
) {
    spawn_stat_row(parent, font, "完成任務", &mission_manager.completed_count.to_string());
    spawn_stat_row(parent, font, "總收入", &format!("${}", mission_manager.total_earnings));
    spawn_stat_row(parent, font, "外送次數", &mission_manager.total_deliveries.to_string());

    if mission_manager.total_deliveries > 0 {
        spawn_stat_row(
            parent,
            font,
            "平均評價",
            &format!("{:.1}", mission_manager.average_rating),
        );
    }

    spawn_stat_row(parent, font, "目前連擊", &mission_manager.delivery_streak.to_string());

    // 各類型統計
    let completed = &mission_manager.completed_missions;
    let delivery_count = completed.iter().filter(|r| r.mission_type == crate::mission::MissionType::Delivery).count();
    let taxi_count = completed.iter().filter(|r| r.mission_type == crate::mission::MissionType::Taxi).count();
    let race_count = completed.iter().filter(|r| r.mission_type == crate::mission::MissionType::Race).count();

    if !completed.is_empty() {
        // 分隔線
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.5)),
        ));

        spawn_stat_row(parent, font, "外送任務", &delivery_count.to_string());
        spawn_stat_row(parent, font, "載客任務", &taxi_count.to_string());
        spawn_stat_row(parent, font, "競速任務", &race_count.to_string());

        // 平均星級
        let total_stars: u32 = completed.iter().map(|r| r.stars as u32).sum();
        let avg_stars = total_stars as f32 / completed.len() as f32;
        spawn_stat_row(parent, font, "平均星級", &format!("{:.1} ★", avg_stars));
    }
}

/// 生成統計行
pub(super) fn spawn_stat_row(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    value: &str,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(5.0), Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(CONTENT_ITEM_BG),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgba(0.6, 0.6, 0.7, 0.9)),
            ));
            row.spawn((
                Text::new(value),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// 生成任務詳情行
pub(super) fn spawn_mission_detail_row(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    value: &str,
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::new(Val::Px(12.0), Val::Px(8.0), Val::Px(2.0), Val::Px(2.0)),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
            ));
            row.spawn((
                Text::new(value),
                TextFont {
                    font: font.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgba(0.8, 0.8, 0.9, 0.9)),
            ));
        });
}

pub(super) fn spawn_mission_item(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    name: &str,
    status: &str,
    is_active: bool,
) {
    let bg_color = if is_active {
        Color::srgba(0.1, 0.15, 0.25, 0.9)
    } else {
        CONTENT_ITEM_BG
    };

    spawn_content_row(parent, bg_color).with_children(|row| {
            row.spawn((
                Text::new(name),
                TextFont {
                    font: font.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            if !status.is_empty() {
                let status_color = if is_active {
                    Color::srgba(0.3, 0.8, 0.4, 0.9)
                } else {
                    Color::srgba(0.5, 0.5, 0.6, 0.8)
                };
                row.spawn((
                    Text::new(status),
                    TextFont {
                        font: font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(status_color),
                ));
            }
        });
}

