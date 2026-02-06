//! 劇情任務 HUD 系統 (GTA 5 風格)
//!
//! 顯示當前劇情任務的標題、階段、目標和計時器

use bevy::prelude::*;

use super::components::{
    ChineseFont, StoryMissionHud, StoryMissionObjectiveCheck, StoryMissionObjectiveItem,
    StoryMissionObjectiveList, StoryMissionObjectiveText, StoryMissionPhaseText, StoryMissionTimer,
    StoryMissionTitle,
};
use crate::mission::{get_current_mission_info, StoryMissionDatabase, StoryMissionManager};

// ============================================================================
// 劇情任務 HUD 顏色常數
// ============================================================================
const STORY_HUD_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const STORY_HUD_TITLE_COLOR: Color = Color::srgb(1.0, 0.85, 0.0); // 金黃色標題
const STORY_HUD_PHASE_COLOR: Color = Color::srgb(0.9, 0.9, 0.9); // 白色描述
const STORY_HUD_OBJECTIVE_COLOR: Color = Color::srgb(0.7, 0.7, 0.7); // 灰白色目標
const STORY_HUD_OBJECTIVE_DONE: Color = Color::srgb(0.3, 0.8, 0.3); // 綠色完成
const STORY_HUD_TIMER_COLOR: Color = Color::srgb(1.0, 0.5, 0.3); // 橙色計時
const STORY_HUD_CHECK_EMPTY: &str = "○";
const STORY_HUD_CHECK_DONE: &str = "●";

/// 設置劇情任務 HUD
pub fn setup_story_mission_hud(mut commands: Commands, font: Option<Res<ChineseFont>>) {
    let Some(font) = font else { return };

    // 主容器（右上角，小地圖下方）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(220.0),
                right: Val::Px(10.0),
                width: Val::Px(280.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(STORY_HUD_BG),
            BorderRadius::all(Val::Px(8.0)),
            Visibility::Hidden,
            StoryMissionHud,
            Name::new("StoryMissionHud"),
        ))
        .with_children(|parent| {
            // 任務標題
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(STORY_HUD_TITLE_COLOR),
                StoryMissionTitle,
            ));

            // 階段描述
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(STORY_HUD_PHASE_COLOR),
                StoryMissionPhaseText,
            ));

            // 計時器（如果有時限）
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(STORY_HUD_TIMER_COLOR),
                StoryMissionTimer,
            ));

            // 目標列表容器
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                    StoryMissionObjectiveList,
                ))
                .with_children(|list| {
                    // 預先創建 5 個目標槽位（動態顯示/隱藏）
                    for i in 0..5 {
                        list.spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            Visibility::Hidden,
                            StoryMissionObjectiveItem { index: i },
                        ))
                        .with_children(|item| {
                            // 勾選框
                            item.spawn((
                                Text::new(STORY_HUD_CHECK_EMPTY),
                                TextFont {
                                    font: font.font.clone(),
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(STORY_HUD_OBJECTIVE_COLOR),
                                StoryMissionObjectiveCheck { index: i },
                            ));

                            // 目標文字
                            item.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.font.clone(),
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(STORY_HUD_OBJECTIVE_COLOR),
                                StoryMissionObjectiveText { index: i },
                            ));
                        });
                    }
                });
        });
}

/// 格式化任務計時器文字
fn format_mission_timer(time_remaining: Option<f32>) -> String {
    match time_remaining {
        Some(remaining) => {
            let mins = (remaining / 60.0).floor() as u32;
            let secs = (remaining % 60.0).floor() as u32;
            format!("⏱ {:02}:{:02}", mins, secs)
        }
        None => String::new(),
    }
}

/// 取得目標勾選框狀態
fn get_objective_check_state(is_completed: bool) -> (&'static str, Color) {
    if is_completed {
        (STORY_HUD_CHECK_DONE, STORY_HUD_OBJECTIVE_DONE)
    } else {
        (STORY_HUD_CHECK_EMPTY, STORY_HUD_OBJECTIVE_COLOR)
    }
}

/// 格式化目標文字
fn format_objective_text(description: &str, current_count: u32, target_count: u32) -> String {
    if target_count > 1 {
        format!("{} ({}/{})", description, current_count, target_count)
    } else {
        description.to_string()
    }
}

/// 取得目標文字顏色
fn get_objective_text_color(is_completed: bool) -> Color {
    if is_completed {
        STORY_HUD_OBJECTIVE_DONE
    } else {
        STORY_HUD_OBJECTIVE_COLOR
    }
}

/// 根據條件取得可見度
fn visibility_from_bool(visible: bool) -> Visibility {
    if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

/// 更新劇情任務 HUD
#[allow(clippy::type_complexity)]
pub fn update_story_mission_hud(
    story_manager: Res<StoryMissionManager>,
    story_database: Res<StoryMissionDatabase>,
    mut hud_query: Query<&mut Visibility, With<StoryMissionHud>>,
    mut title_query: Query<
        &mut Text,
        (
            With<StoryMissionTitle>,
            Without<StoryMissionPhaseText>,
            Without<StoryMissionTimer>,
            Without<StoryMissionObjectiveCheck>,
            Without<StoryMissionObjectiveText>,
        ),
    >,
    mut phase_query: Query<
        &mut Text,
        (
            With<StoryMissionPhaseText>,
            Without<StoryMissionTitle>,
            Without<StoryMissionTimer>,
            Without<StoryMissionObjectiveCheck>,
            Without<StoryMissionObjectiveText>,
        ),
    >,
    mut timer_query: Query<
        &mut Text,
        (
            With<StoryMissionTimer>,
            Without<StoryMissionTitle>,
            Without<StoryMissionPhaseText>,
            Without<StoryMissionObjectiveCheck>,
            Without<StoryMissionObjectiveText>,
        ),
    >,
    mut item_query: Query<(&mut Visibility, &StoryMissionObjectiveItem), Without<StoryMissionHud>>,
    mut check_query: Query<
        (&mut Text, &mut TextColor, &StoryMissionObjectiveCheck),
        (
            Without<StoryMissionTitle>,
            Without<StoryMissionPhaseText>,
            Without<StoryMissionTimer>,
            Without<StoryMissionObjectiveText>,
        ),
    >,
    mut text_query: Query<
        (&mut Text, &mut TextColor, &StoryMissionObjectiveText),
        (
            Without<StoryMissionTitle>,
            Without<StoryMissionPhaseText>,
            Without<StoryMissionTimer>,
            Without<StoryMissionObjectiveCheck>,
        ),
    >,
) {
    let mission_info = get_current_mission_info(&story_manager, &story_database);
    let hud_visible = visibility_from_bool(mission_info.is_some());
    for mut visibility in &mut hud_query {
        *visibility = hud_visible;
    }

    let Some(info) = mission_info else { return };

    // 更新標題、階段、計時器
    if let Ok(mut t) = title_query.single_mut() {
        **t = info.title;
    }
    if let Ok(mut t) = phase_query.single_mut() {
        **t = info.phase_description;
    }
    if let Ok(mut t) = timer_query.single_mut() {
        **t = format_mission_timer(info.time_remaining);
    }

    // 更新目標列表可見度
    let obj_count = info.objectives.len();
    for (mut vis, item) in &mut item_query {
        *vis = visibility_from_bool(item.index < obj_count);
    }

    // 更新勾選框
    for (mut check_text, mut check_color, check) in &mut check_query {
        if let Some(obj) = info.objectives.get(check.index) {
            let (text, color) = get_objective_check_state(obj.is_completed);
            **check_text = text.to_string();
            check_color.0 = color;
        }
    }

    // 更新目標文字
    for (mut obj_text, mut obj_color, text_comp) in &mut text_query {
        if let Some(obj) = info.objectives.get(text_comp.index) {
            **obj_text =
                format_objective_text(&obj.description, obj.current_count, obj.target_count);
            obj_color.0 = get_objective_text_color(obj.is_completed);
        }
    }
}
