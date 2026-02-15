//! 準星與武器 HUD 設置系統

use bevy::prelude::*;

use crate::ui::components::{
    AmmoBulletIcon, AmmoVisualGrid, ChineseFont, Crosshair, CrosshairDirection, CrosshairDot,
    CrosshairHitMarker, CrosshairLine, CrosshairOuterRing, CurrentAmmoShadow, CurrentAmmoText,
    HitMarkerLine, ReserveAmmoShadow, ReserveAmmoText, WeaponAreaContainer, WeaponDisplay,
    WeaponDisplayShadow, WeaponSlot,
};
use crate::ui::constants::*;

/// 大字陰影偏移量（用於彈藥數字等大字體）
const LARGE_TEXT_SHADOW_OFFSET: f32 = 2.0;

/// 建立帶陰影的文字 Node（GTA 風格陰影效果）
/// 回傳 (陰影 Node, 主文字 Node) 的 tuple，呼叫者可各自附加 marker component
fn shadow_text_node(shadow_offset: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(shadow_offset),
        top: Val::Px(shadow_offset),
        ..default()
    }
}

/// 生成準星的其中一條線（包含陰影與主線條）
fn spawn_crosshair_line(parent: &mut ChildSpawnerCommands, direction: CrosshairDirection) {
    let (shadow_w, shadow_h, main_w, main_h, top, bottom, left, right) = match direction {
        CrosshairDirection::Top => (
            4.0,
            12.0,
            2.0,
            10.0,
            Val::Px(3.0),
            Val::Auto,
            Val::Px(28.0),
            Val::Auto,
        ),
        CrosshairDirection::Bottom => (
            4.0,
            12.0,
            2.0,
            10.0,
            Val::Auto,
            Val::Px(3.0),
            Val::Px(28.0),
            Val::Auto,
        ),
        CrosshairDirection::Left => (
            12.0,
            4.0,
            10.0,
            2.0,
            Val::Px(28.0),
            Val::Auto,
            Val::Px(3.0),
            Val::Auto,
        ),
        CrosshairDirection::Right => (
            12.0,
            4.0,
            10.0,
            2.0,
            Val::Px(28.0),
            Val::Auto,
            Val::Auto,
            Val::Px(3.0),
        ),
    };

    // 陰影
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(shadow_w),
            height: Val::Px(shadow_h),
            top,
            bottom,
            left,
            right,
            ..default()
        },
        BackgroundColor(CROSSHAIR_SHADOW),
        BorderRadius::all(Val::Px(2.0)),
    ));

    // 主線條 (位置偏移 +1.0)
    let offset_val = |v: Val| {
        if let Val::Px(p) = v {
            Val::Px(p + 1.0)
        } else {
            v
        }
    };
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(main_w),
            height: Val::Px(main_h),
            top: offset_val(top),
            bottom: offset_val(bottom),
            left: offset_val(left),
            right: offset_val(right),
            ..default()
        },
        BackgroundColor(CROSSHAIR_MAIN),
        BorderRadius::all(Val::Px(1.0)),
        CrosshairLine { direction },
    ));
}

fn spawn_hit_marker_line(
    parent: &mut ChildSpawnerCommands,
    rotation_z: f32,
    top: Val,
    bottom: Val,
    left: Val,
    right: Val,
) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(3.0),
            height: Val::Px(10.0),
            top,
            bottom,
            left,
            right,
            ..default()
        },
        BackgroundColor(HIT_MARKER_COLOR),
        BorderRadius::all(Val::Px(1.0)),
        Transform::from_rotation(Quat::from_rotation_z(rotation_z)),
        HitMarkerLine,
    ));
}

/// 生成子彈圖示
pub(super) fn spawn_bullet_icon(parent: &mut ChildSpawnerCommands, index: usize, color: Color) {
    parent.spawn((
        Node {
            width: Val::Px(4.0),
            height: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(color),
        BorderRadius::top(Val::Px(2.0)),
        AmmoBulletIcon { index },
    ));
}

/// 生成武器槽位
fn spawn_weapon_slot(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    font: Handle<Font>,
    is_active: bool,
) {
    parent
        .spawn((
            icon_box_node(),
            BackgroundColor(if is_active {
                SLOT_ACTIVE
            } else {
                SLOT_INACTIVE
            }),
            BorderColor::all(Color::srgba(0.5, 0.5, 0.5, 0.5)),
            BorderRadius::all(Val::Px(4.0)),
            WeaponSlot { slot_index: index },
        ))
        .with_children(|slot| {
            slot.spawn((
                Text::new(format!("{}", index + 1)),
                TextFont {
                    font_size: 14.0,
                    font: font.clone(),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// 設置準星和彈藥 UI
pub fn setup_crosshair(mut commands: Commands, chinese_font: Res<ChineseFont>) {
    let font = chinese_font.font.clone();
    spawn_crosshair_center(&mut commands);
    spawn_weapon_hud(&mut commands, font);
}

/// 螢幕中央準星（點、線、外圈、命中標記）
fn spawn_crosshair_center(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalTransform::default(), // B0004: 後代有 Transform（命中標記旋轉）
            Crosshair,
        ))
        .with_children(|parent| {
            // 準星容器（增加尺寸以容納外圈）
            parent
                .spawn((
                    Node {
                        width: Val::Px(60.0),
                        height: Val::Px(60.0),
                        position_type: PositionType::Relative,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    GlobalTransform::default(), // B0004: 後代有 Transform（命中標記旋轉）
                ))
                .with_children(|crosshair| {
                    // 外圈（動態擴散時使用）
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(50.0),
                            height: Val::Px(50.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        BorderColor::all(CROSSHAIR_OUTER_RING),
                        BorderRadius::all(Val::Percent(50.0)),
                        CrosshairOuterRing,
                    ));

                    // 中心點陰影（輪廓效果）
                    crosshair.spawn((
                        Node {
                            width: Val::Px(6.0),
                            height: Val::Px(6.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_SHADOW),
                        BorderRadius::all(Val::Percent(50.0)),
                    ));

                    // 中心點
                    crosshair.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(4.0),
                            height: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(CROSSHAIR_MAIN),
                        BorderRadius::all(Val::Percent(50.0)),
                        CrosshairDot,
                    ));

                    // 四方向線條（帶陰影）
                    spawn_crosshair_line(crosshair, CrosshairDirection::Top);
                    spawn_crosshair_line(crosshair, CrosshairDirection::Bottom);
                    spawn_crosshair_line(crosshair, CrosshairDirection::Left);
                    spawn_crosshair_line(crosshair, CrosshairDirection::Right);

                    // 命中標記（X 形，初始隱藏）
                    crosshair
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Px(20.0),
                                height: Val::Px(20.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            Visibility::Hidden,
                            GlobalTransform::default(), // B0004: 子實體需要 GlobalTransform
                            CrosshairHitMarker,
                        ))
                        .with_children(|hit_marker| {
                            // X 的四條線（斜向）- 使用四個小方塊模擬
                            // X 的四條線（斜向）- 使用四個小方塊模擬
                            let f_pi_4 = std::f32::consts::FRAC_PI_4;
                            spawn_hit_marker_line(
                                hit_marker,
                                f_pi_4,
                                Val::Px(0.0),
                                Val::Auto,
                                Val::Px(3.0),
                                Val::Auto,
                            ); // 左上
                            spawn_hit_marker_line(
                                hit_marker,
                                -f_pi_4,
                                Val::Px(0.0),
                                Val::Auto,
                                Val::Auto,
                                Val::Px(3.0),
                            ); // 右上
                            spawn_hit_marker_line(
                                hit_marker,
                                -f_pi_4,
                                Val::Auto,
                                Val::Px(0.0),
                                Val::Px(3.0),
                                Val::Auto,
                            ); // 左下
                            spawn_hit_marker_line(
                                hit_marker,
                                f_pi_4,
                                Val::Auto,
                                Val::Px(0.0),
                                Val::Auto,
                                Val::Px(3.0),
                            ); // 右下
                        });
                });
        });
}

/// 右下角 GTA 風格武器 HUD（武器名、彈藥數、彈藥網格、武器槽位）
fn spawn_weapon_hud(commands: &mut Commands, font: Handle<Font>) {
    // 外發光層
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                right: Val::Px(16.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(HUD_GLOW_OUTER),
            BorderRadius::all(Val::Px(12.0)),
        ))
        .with_children(|glow| {
            // 主容器
            glow.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(HUD_BG),
                BorderColor::all(HUD_BORDER_HIGHLIGHT),
                BorderRadius::all(Val::Px(8.0)),
                WeaponAreaContainer,
            ))
            .with_children(|parent| {
                // 武器名稱區（圖示 + 名稱）
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                    .with_children(|row| {
                        // 武器圖示（金色子彈形狀）
                        row.spawn((
                            Node {
                                width: Val::Px(6.0),
                                height: Val::Px(16.0),
                                ..default()
                            },
                            BackgroundColor(AMMO_NORMAL),
                            BorderRadius::top(Val::Px(3.0)),
                        ));
                        // 武器名稱（帶陰影）
                        row.spawn((Node { ..default() },))
                            .with_children(|label_container| {
                                // 陰影層
                                label_container.spawn((
                                    Text::new("手槍"),
                                    TextFont {
                                        font_size: 22.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    shadow_text_node(TEXT_SHADOW_OFFSET),
                                    WeaponDisplayShadow,
                                ));
                                // 主文字
                                label_container.spawn((
                                    Text::new("手槍"),
                                    TextFont {
                                        font_size: 22.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    WeaponDisplay,
                                ));
                            });
                    });

                // 彈藥數量區
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Baseline,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },))
                    .with_children(|ammo_row| {
                        // 當前彈藥（大字，帶陰影）
                        ammo_row
                            .spawn((Node { ..default() },))
                            .with_children(|label_container| {
                                // 陰影層
                                label_container.spawn((
                                    Text::new("12"),
                                    TextFont {
                                        font_size: 36.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    shadow_text_node(LARGE_TEXT_SHADOW_OFFSET),
                                    CurrentAmmoShadow,
                                ));
                                // 主文字
                                label_container.spawn((
                                    Text::new("12"),
                                    TextFont {
                                        font_size: 36.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(AMMO_NORMAL),
                                    CurrentAmmoText,
                                ));
                            });
                        // 分隔線（帶陰影）
                        ammo_row
                            .spawn((Node { ..default() },))
                            .with_children(|sep_container| {
                                sep_container.spawn((
                                    Text::new("/"),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    shadow_text_node(TEXT_SHADOW_OFFSET),
                                ));
                                sep_container.spawn((
                                    Text::new("/"),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(AMMO_RESERVE),
                                ));
                            });
                        // 後備彈藥（小字，帶陰影）
                        ammo_row
                            .spawn((Node { ..default() },))
                            .with_children(|label_container| {
                                // 陰影層
                                label_container.spawn((
                                    Text::new("120"),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(TEXT_SHADOW_COLOR),
                                    shadow_text_node(TEXT_SHADOW_OFFSET),
                                    ReserveAmmoShadow,
                                ));
                                // 主文字
                                label_container.spawn((
                                    Text::new("120"),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.clone(),
                                        ..default()
                                    },
                                    TextColor(AMMO_RESERVE),
                                    ReserveAmmoText,
                                ));
                            });
                    });

                // 彈藥視覺化網格（子彈圖示）
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            justify_content: JustifyContent::FlexEnd,
                            column_gap: Val::Px(2.0),
                            row_gap: Val::Px(2.0),
                            max_width: Val::Px(140.0),
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },
                        AmmoVisualGrid,
                    ))
                    .with_children(|grid| {
                        // 初始生成 12 個子彈圖示（預設手槍彈匣）
                        for i in 0..12 {
                            spawn_bullet_icon(grid, i, BULLET_FILLED);
                        }
                    });

                // 武器槽位指示器 [1][2][3][4]
                parent
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        margin: UiRect::top(Val::Px(5.0)),
                        ..default()
                    },))
                    .with_children(|slots| {
                        for i in 0..4 {
                            let is_active = i == 0; // 預設第一格選中
                            spawn_weapon_slot(slots, i, font.clone(), is_active);
                        }
                    });
            });
        });
}
