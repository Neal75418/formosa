//! 對話系統 UI 組件
//!
//! 顯示對話框、文字、選項等 UI 元素

use bevy::prelude::*;

use super::dialogue::{DialogueDatabase, DialogueEvent, DialogueSpeaker, DialogueState};
use super::dialogue_systems::{
    get_displayed_text, get_valid_choices, process_dialogue_consequences,
};
use super::economy::RespectManager;
use super::relationship::RelationshipManager;
use super::unlocks::UnlockManager;
use crate::economy::PlayerWallet;

// ============================================================================
// 顏色常數
// ============================================================================

/// 選項按鈕：預設背景色
const CHOICE_BUTTON_COLOR_NORMAL: Color = Color::srgba(0.15, 0.15, 0.15, 0.9);
/// 選項按鈕：滑鼠懸停背景色
const CHOICE_BUTTON_COLOR_HOVERED: Color = Color::srgba(0.25, 0.25, 0.25, 0.9);
/// 選項按鈕：點擊時背景色
const CHOICE_BUTTON_COLOR_PRESSED: Color = Color::srgba(0.3, 0.3, 0.3, 0.9);

/// 對話 UI Plugin
pub struct DialogueUIPlugin;

impl Plugin for DialogueUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_dialogue_ui).add_systems(
            Update,
            (
                update_dialogue_ui_visibility,
                update_dialogue_text,
                update_dialogue_speaker,
                update_dialogue_choices,
                choice_button_interaction, // 滑鼠點擊選項處理
            )
                .chain(),
        );
    }
}

// ============================================================================
// UI 組件標記
// ============================================================================

/// 對話 UI 根容器
#[derive(Component)]
pub struct DialogueUIRoot;

/// 對話框背景
#[derive(Component)]
pub struct DialogueBox;

/// 說話者名稱標籤
#[derive(Component)]
pub struct SpeakerNameLabel;

/// 對話文字區域
#[derive(Component)]
pub struct DialogueTextArea;

/// 選項容器
#[derive(Component)]
pub struct ChoicesContainer;

/// 單個選項按鈕
#[derive(Component)]
pub struct ChoiceButton {
    pub index: usize,
}

/// 選項文字
#[derive(Component)]
pub struct ChoiceText {
    pub index: usize,
}

/// 繼續提示（按空白鍵繼續）
#[derive(Component)]
pub struct ContinuePrompt;

/// 頭像容器
#[derive(Component)]
pub struct PortraitContainer;

// ============================================================================
// UI 設置
// ============================================================================

/// 設置對話 UI
#[allow(clippy::too_many_lines)]
fn setup_dialogue_ui(mut commands: Commands) {
    // 根容器（全螢幕覆蓋）
    commands
        .spawn((
            DialogueUIRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::End,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            // 對話框
            parent
                .spawn((
                    DialogueBox,
                    Node {
                        width: Val::Percent(80.0),
                        max_width: Val::Px(900.0),
                        min_height: Val::Px(150.0),
                        flex_direction: FlexDirection::Row,
                        padding: UiRect::all(Val::Px(15.0)),
                        margin: UiRect::bottom(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|parent| {
                    // 頭像區域（左側）
                    parent.spawn((
                        PortraitContainer,
                        Node {
                            width: Val::Px(100.0),
                            height: Val::Px(100.0),
                            margin: UiRect::right(Val::Px(15.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.5)),
                        BorderRadius::all(Val::Px(5.0)),
                    ));

                    // 文字區域（右側）
                    parent
                        .spawn((Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            ..default()
                        },))
                        .with_children(|parent| {
                            // 說話者名稱
                            parent.spawn((
                                SpeakerNameLabel,
                                Text::new(""),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.9, 0.3)),
                                Node {
                                    margin: UiRect::bottom(Val::Px(8.0)),
                                    ..default()
                                },
                            ));

                            // 對話文字
                            parent.spawn((
                                DialogueTextArea,
                                Text::new(""),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                Node {
                                    flex_grow: 1.0,
                                    ..default()
                                },
                            ));

                            // 繼續提示
                            parent.spawn((
                                ContinuePrompt,
                                Text::new("按 空白鍵 或 點擊 繼續..."),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.8)),
                                Node {
                                    margin: UiRect::top(Val::Px(10.0)),
                                    ..default()
                                },
                                Visibility::Hidden,
                            ));
                        });
                });

            // 選項容器
            parent
                .spawn((
                    ChoicesContainer,
                    Node {
                        width: Val::Percent(80.0),
                        max_width: Val::Px(900.0),
                        flex_direction: FlexDirection::Column,
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..default()
                    },
                    Visibility::Hidden,
                ))
                .with_children(|parent| {
                    // 預先創建 4 個選項按鈕
                    for i in 0..4 {
                        parent
                            .spawn((
                                ChoiceButton { index: i },
                                Button, // 啟用滑鼠互動
                                Node {
                                    width: Val::Percent(100.0),
                                    padding: UiRect::new(
                                        Val::Px(15.0),
                                        Val::Px(15.0),
                                        Val::Px(10.0),
                                        Val::Px(10.0),
                                    ),
                                    margin: UiRect::bottom(Val::Px(5.0)),
                                    ..default()
                                },
                                BackgroundColor(CHOICE_BUTTON_COLOR_NORMAL),
                                BorderRadius::all(Val::Px(5.0)),
                                Visibility::Hidden,
                            ))
                            .with_children(|parent| {
                                // 數字標籤
                                parent.spawn((
                                    Text::new(format!("{}. ", i + 1)),
                                    TextFont {
                                        font_size: 16.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(1.0, 0.9, 0.3)),
                                    Node {
                                        margin: UiRect::right(Val::Px(5.0)),
                                        ..default()
                                    },
                                ));

                                // 選項文字
                                parent.spawn((
                                    ChoiceText { index: i },
                                    Text::new(""),
                                    TextFont {
                                        font_size: 16.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                    }
                });
        });
}

// ============================================================================
// UI 更新系統
// ============================================================================

/// 更新對話 UI 可見性
fn update_dialogue_ui_visibility(
    dialogue_state: Res<DialogueState>,
    mut root_query: Query<&mut Visibility, With<DialogueUIRoot>>,
) {
    let Ok(mut visibility) = root_query.single_mut() else {
        return;
    };

    *visibility = if dialogue_state.active_dialogue.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

/// 更新對話文字
fn update_dialogue_text(
    dialogue_state: Res<DialogueState>,
    database: Res<DialogueDatabase>,
    mut text_query: Query<&mut Text, With<DialogueTextArea>>,
    mut continue_query: Query<&mut Visibility, With<ContinuePrompt>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let Some(active) = &dialogue_state.active_dialogue else {
        text.0 = String::new();
        return;
    };

    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 根據打字進度顯示文字
    let displayed = get_displayed_text(&node.text, active.typing_progress, &active.participants);
    text.0 = displayed;

    // 更新繼續提示可見性
    if let Ok(mut continue_vis) = continue_query.single_mut() {
        *continue_vis = if active.typing_complete && node.choices.is_empty() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 更新說話者名稱
fn update_dialogue_speaker(
    dialogue_state: Res<DialogueState>,
    database: Res<DialogueDatabase>,
    mut speaker_query: Query<&mut Text, With<SpeakerNameLabel>>,
) {
    let Ok(mut speaker_text) = speaker_query.single_mut() else {
        return;
    };

    let Some(active) = &dialogue_state.active_dialogue else {
        speaker_text.0 = String::new();
        return;
    };

    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 取得說話者名稱
    let speaker_name = if let Some(name) = &node.speaker_name {
        name.clone()
    } else {
        match &node.speaker {
            DialogueSpeaker::Player => "玩家".to_string(),
            DialogueSpeaker::Npc(npc_id) => {
                if let Some(npc) = database.get_npc(*npc_id) {
                    npc.name.clone()
                } else {
                    format!("NPC #{npc_id}")
                }
            }
            DialogueSpeaker::Narrator => String::new(), // 旁白不顯示名字
            DialogueSpeaker::Radio => "📻 無線電".to_string(),
            DialogueSpeaker::System => "系統".to_string(),
        }
    };

    speaker_text.0 = speaker_name;
}

/// 更新選項顯示
fn update_dialogue_choices(
    dialogue_state: Res<DialogueState>,
    database: Res<DialogueDatabase>,
    story_manager: Res<super::story_manager::StoryMissionManager>,
    wallet: Res<PlayerWallet>,
    _respect: Res<RespectManager>,
    unlocks: Res<UnlockManager>,
    relationship: Res<RelationshipManager>,
    mut choices_container_query: Query<
        &mut Visibility,
        (With<ChoicesContainer>, Without<ChoiceButton>),
    >,
    mut choice_button_query: Query<(&ChoiceButton, &mut Visibility), Without<ChoicesContainer>>,
    mut choice_text_query: Query<(&ChoiceText, &mut Text)>,
) {
    let Ok(mut container_vis) = choices_container_query.single_mut() else {
        return;
    };

    let Some(active) = &dialogue_state.active_dialogue else {
        *container_vis = Visibility::Hidden;
        return;
    };

    if !active.typing_complete {
        *container_vis = Visibility::Hidden;
        return;
    }

    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    // 取得有效選項
    let valid_choices = get_valid_choices(node, &story_manager, &wallet, &unlocks, &relationship);

    if valid_choices.is_empty() {
        *container_vis = Visibility::Hidden;
        return;
    }

    *container_vis = Visibility::Visible;

    // 更新每個選項按鈕
    for (button, mut vis) in &mut choice_button_query {
        if button.index < valid_choices.len() {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // 更新選項文字
    for (choice_text, mut text) in &mut choice_text_query {
        if choice_text.index < valid_choices.len() {
            text.0.clone_from(&valid_choices[choice_text.index].text);
        } else {
            text.0 = String::new();
        }
    }
}

// ============================================================================
// 滑鼠互動系統
// ============================================================================

/// 處理選項按鈕滑鼠互動
pub fn choice_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &ChoiceButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    dialogue_state: Res<DialogueState>,
    database: Res<DialogueDatabase>,
    mut story_manager: ResMut<super::story_manager::StoryMissionManager>,
    mut wallet: ResMut<PlayerWallet>,
    _respect: ResMut<RespectManager>,
    mut unlocks: ResMut<UnlockManager>,
    mut relationship: ResMut<RelationshipManager>,
    mut events: MessageWriter<DialogueEvent>,
) {
    // 早期返回：沒有按鈕互動變化
    if interaction_query.is_empty() {
        return;
    }

    let Some(active) = &dialogue_state.active_dialogue else {
        return;
    };

    // 打字未完成時不處理點擊
    if !active.typing_complete {
        return;
    }

    let Some(tree) = database.get_dialogue(active.dialogue_id) else {
        return;
    };

    let Some(node) = tree.get_node(active.current_node) else {
        return;
    };

    let valid_choices = get_valid_choices(node, &story_manager, &wallet, &unlocks, &relationship);

    for (interaction, button, mut bg_color) in &mut interaction_query {
        if button.index >= valid_choices.len() {
            continue;
        }

        match *interaction {
            Interaction::Pressed => {
                // 選擇此選項
                let choice = valid_choices[button.index];

                // 處理後果
                process_dialogue_consequences(
                    &choice.consequences,
                    &mut story_manager,
                    &mut wallet,
                    &mut unlocks,
                    &mut relationship,
                );

                // 跳轉或結束
                if choice.ends_dialogue {
                    process_dialogue_consequences(
                        &tree.on_complete,
                        &mut story_manager,
                        &mut wallet,
                        &mut unlocks,
                        &mut relationship,
                    );
                    events.write(DialogueEvent::Completed(active.dialogue_id));
                    events.write(DialogueEvent::End);
                } else if let Some(next_node) = choice.next_node {
                    events.write(DialogueEvent::GoToNode(next_node));
                }

                *bg_color = BackgroundColor(CHOICE_BUTTON_COLOR_PRESSED);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(CHOICE_BUTTON_COLOR_HOVERED);
            }
            Interaction::None => {
                *bg_color = BackgroundColor(CHOICE_BUTTON_COLOR_NORMAL);
            }
        }
    }
}
