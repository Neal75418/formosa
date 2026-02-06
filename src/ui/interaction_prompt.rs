//! 互動提示 UI 系統
//!
//! 顯示可互動物件的提示（如「按 F 互動」）

use bevy::prelude::*;

use super::components::{
    ChineseFont, InteractionPromptContainer, InteractionPromptKey, InteractionPromptState,
    InteractionPromptText,
};
use crate::mission::{Trigger as MissionTrigger, TriggerType};
use crate::player::Player;

// ============================================================================
// 互動提示顏色常數
// ============================================================================
/// 互動提示背景色
const INTERACTION_PROMPT_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
/// 互動提示按鍵背景色
const INTERACTION_KEY_BG: Color = Color::srgb(0.95, 0.85, 0.2);
/// 互動提示文字色
const INTERACTION_TEXT_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);

/// 設置互動提示 UI
pub fn setup_interaction_prompt(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    // 外層容器（全寬，用於居中）
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(180.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
            InteractionPromptContainer,
            Name::new("InteractionPromptWrapper"),
        ))
        .with_children(|wrapper| {
            // 內層提示框
            wrapper
                .spawn((
                    Node {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                    BackgroundColor(INTERACTION_PROMPT_BG),
                    BorderRadius::all(Val::Px(6.0)),
                ))
                .with_children(|parent| {
                    // 按鍵框
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(36.0),
                                height: Val::Px(36.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(INTERACTION_KEY_BG),
                            BorderRadius::all(Val::Px(4.0)),
                            InteractionPromptKey,
                        ))
                        .with_children(|key_parent| {
                            key_parent.spawn((
                                Text::new("F"),
                                TextFont {
                                    font: chinese_font.font.clone(),
                                    font_size: 22.0,
                                    ..default()
                                },
                                TextColor(Color::BLACK),
                            ));
                        });

                    // 提示文字
                    parent.spawn((
                        Text::new("按 F 互動"),
                        TextFont {
                            font: chinese_font.font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(INTERACTION_TEXT_COLOR),
                        InteractionPromptText,
                    ));
                });
        });

    // 初始化狀態資源
    commands.insert_resource(InteractionPromptState::default());
}

/// 更新互動提示狀態（檢測玩家是否靠近觸發點）
pub fn update_interaction_prompt_state(
    player_query: Query<&Transform, With<Player>>,
    trigger_query: Query<(&Transform, &MissionTrigger)>,
    mut prompt_state: ResMut<InteractionPromptState>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.single() else {
        prompt_state.hide();
        prompt_state.update(time.delta_secs());
        return;
    };

    let player_pos = player_transform.translation;

    // 尋找最近的可互動觸發點 (OnInteract)
    let mut closest_trigger: Option<(&MissionTrigger, f32)> = None;

    for (trigger_transform, trigger) in &trigger_query {
        if !trigger.enabled {
            continue;
        }

        // 只處理需要互動的觸發點
        if trigger.trigger_type != TriggerType::OnInteract {
            continue;
        }

        // 檢查玩家是否在觸發範圍內
        let in_range = trigger
            .shape
            .contains(trigger_transform.translation, player_pos);
        if !in_range {
            continue;
        }

        // 計算距離
        let distance = player_pos.distance(trigger_transform.translation);

        // 保留最近的觸發點
        let is_closer = closest_trigger.is_none_or(|(_, d)| distance < d);
        if is_closer {
            closest_trigger = Some((trigger, distance));
        }
    }

    // 更新提示狀態
    if let Some((trigger, _)) = closest_trigger {
        let prompt_text = trigger
            .prompt_text
            .clone()
            .unwrap_or_else(|| "按 F 互動".to_string());
        prompt_state.show(prompt_text, "F");
    } else {
        prompt_state.hide();
    }

    prompt_state.update(time.delta_secs());
}

/// 更新互動提示 UI 顯示
pub fn update_interaction_prompt_ui(
    prompt_state: Res<InteractionPromptState>,
    mut container_query: Query<&mut Visibility, With<InteractionPromptContainer>>,
    mut text_query: Query<&mut Text, With<InteractionPromptText>>,
) {
    // 更新容器可見性
    for mut visibility in container_query.iter_mut() {
        let should_show = prompt_state.fade_progress > 0.01;
        let new_visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        if *visibility != new_visibility {
            *visibility = new_visibility;
        }
    }

    // 只在狀態變更時更新文字內容
    if prompt_state.is_changed() {
        for mut text in text_query.iter_mut() {
            if **text != prompt_state.text {
                **text = prompt_state.text.clone();
            }
        }
    }
}
