//! 投降/逮捕系統
//!
//! GTA 風格的投降與逮捕機制，包含：
//! - 玩家投降（舉手）
//! - 警察逮捕流程
//! - 敵人投降（低血量或被包圍）
//! - 監獄/警局釋放點

use bevy::prelude::*;

use crate::player::Player;
use crate::core::GameState;
use crate::combat::{Health, WeaponInventory};
use crate::economy::PlayerWallet;

use super::components::*;

// ============================================================================
// 常數
// ============================================================================

/// 投降所需時間（秒）
const SURRENDER_HOLD_TIME: f32 = 2.0;
/// 逮捕所需時間（秒）
const ARREST_TIME: f32 = 3.0;
/// 罰款基礎值（每星 $500）
const FINE_PER_STAR: f32 = 500.0;
/// 武器沒收比例（被逮捕時損失的武器比例）
const WEAPON_CONFISCATION_RATE: f32 = 0.5;
/// 警察逮捕距離
const ARREST_DISTANCE: f32 = 2.0;
/// 投降後免疫時間（秒）
const POST_ARREST_IMMUNITY: f32 = 10.0;
/// 敵人投降血量閾值（百分比）
const ENEMY_SURRENDER_HEALTH_THRESHOLD: f32 = 0.2;
/// 警局釋放點位置
const POLICE_STATION_POSITION: Vec3 = Vec3::new(100.0, 0.0, 100.0);
/// 載具內投降警告顯示時間（秒）
const VEHICLE_SURRENDER_WARNING_DURATION: f32 = 2.0;

// ============================================================================
// 組件
// ============================================================================

/// 玩家投降狀態
#[derive(Component, Default)]
pub struct PlayerSurrenderState {
    /// 是否正在投降
    pub is_surrendering: bool,
    /// 投降按住計時器
    pub surrender_hold_timer: f32,
    /// 是否已完全投降（手舉起）
    pub has_surrendered: bool,
    /// 是否正在被逮捕
    pub being_arrested: bool,
    /// 逮捕進度（0.0 - 1.0）
    pub arrest_progress: f32,
    /// 逮捕中的警察實體
    pub arresting_officer: Option<Entity>,
    /// 逮捕後免疫計時器
    pub post_arrest_immunity: f32,
    /// 載具內投降提示計時器（顯示「先下車」訊息）
    pub vehicle_warning_timer: f32,
}

/// 敵人投降狀態
#[derive(Component, Default)]
pub struct EnemySurrenderState {
    /// 是否已投降
    pub has_surrendered: bool,
    /// 投降計時器
    pub surrender_timer: f32,
    /// 是否可以投降（某些敵人永不投降）
    pub can_surrender: bool,
    /// 投降原因
    pub surrender_reason: SurrenderReason,
}

/// 投降原因
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SurrenderReason {
    #[default]
    None,
    /// 血量過低
    LowHealth,
    /// 被多名警察包圍
    Surrounded,
    /// 無武器
    Unarmed,
    /// 隊友全滅
    AlliesDefeated,
}

/// 逮捕事件
#[derive(Message, Clone)]
pub struct ArrestEvent {
    /// 被逮捕的實體
    pub target: Entity,
    /// 執行逮捕的警察
    pub officer: Entity,
    /// 逮捕類型
    pub arrest_type: ArrestType,
}

/// 逮捕類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrestType {
    /// 玩家投降被逮捕
    PlayerSurrender,
    /// 敵人投降被逮捕
    EnemySurrender,
}

/// 逮捕結果事件
#[derive(Message, Clone)]
pub struct ArrestComplete {
    /// 被逮捕的實體
    pub target: Entity,
    /// 罰款金額
    pub fine: f32,
    /// 沒收的武器數量
    pub weapons_confiscated: u32,
}

/// 逮捕配置資源
#[derive(Resource)]
pub struct ArrestConfig {
    /// 罰款倍率
    pub fine_multiplier: f32,
    /// 是否啟用投降系統
    pub surrender_enabled: bool,
    /// 是否啟用敵人投降
    pub enemy_surrender_enabled: bool,
}

impl Default for ArrestConfig {
    fn default() -> Self {
        Self {
            fine_multiplier: 1.0,
            surrender_enabled: true,
            enemy_surrender_enabled: true,
        }
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 初始化逮捕系統
pub fn setup_arrest_system(mut commands: Commands) {
    commands.init_resource::<ArrestConfig>();
}

/// 檢查玩家是否可以進行投降操作
fn can_process_surrender(
    config: &ArrestConfig,
    wanted: &WantedLevel,
    surrender_state: &PlayerSurrenderState,
) -> bool {
    config.surrender_enabled
        && wanted.stars > 0
        && !surrender_state.being_arrested
        && surrender_state.post_arrest_immunity <= 0.0
}

/// 玩家投降輸入系統
pub fn player_surrender_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    wanted: Res<WantedLevel>,
    config: Res<ArrestConfig>,
    mut player_query: Query<&mut PlayerSurrenderState, With<Player>>,
    time: Res<Time>,
) {
    let Ok(mut surrender_state) = player_query.single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    // 更新計時器
    if surrender_state.vehicle_warning_timer > 0.0 {
        surrender_state.vehicle_warning_timer -= dt;
    }
    if surrender_state.post_arrest_immunity > 0.0 {
        surrender_state.post_arrest_immunity -= dt;
    }

    // 在車上按 Y 顯示警告
    if game_state.player_in_vehicle {
        if keyboard.just_pressed(KeyCode::KeyY) && wanted.stars > 0 {
            surrender_state.vehicle_warning_timer = VEHICLE_SURRENDER_WARNING_DURATION;
            info!("無法投降：請先下車");
        }
        return;
    }

    // 檢查是否可以投降
    if !can_process_surrender(&config, &wanted, &surrender_state) {
        return;
    }

    // 處理投降輸入
    if keyboard.pressed(KeyCode::KeyY) {
        surrender_state.is_surrendering = true;
        surrender_state.surrender_hold_timer += dt;

        if surrender_state.surrender_hold_timer >= SURRENDER_HOLD_TIME && !surrender_state.has_surrendered {
            surrender_state.has_surrendered = true;
            info!("🏳️ 玩家投降！通緝等級: {} 星", wanted.stars);
        }
    } else if !surrender_state.has_surrendered {
        // 放開按鍵且尚未投降，取消投降
        surrender_state.is_surrendering = false;
        surrender_state.surrender_hold_timer = 0.0;
    }
}

/// 警察逮捕系統
pub fn police_arrest_system(
    mut player_query: Query<(Entity, &Transform, &mut PlayerSurrenderState), With<Player>>,
    police_query: Query<(Entity, &Transform, &PoliceOfficer), Without<Player>>,
    mut arrest_events: MessageWriter<ArrestEvent>,
    time: Res<Time>,
) {
    let Ok((player_entity, player_transform, mut surrender_state)) = player_query.single_mut() else {
        return;
    };

    // 玩家沒有投降，不處理
    if !surrender_state.has_surrendered {
        return;
    }

    let player_pos = player_transform.translation;
    let dt = time.delta_secs();

    // 如果已經在被逮捕
    if surrender_state.being_arrested {
        surrender_state.arrest_progress += dt / ARREST_TIME;

        if surrender_state.arrest_progress >= 1.0 {
            // 逮捕完成
            if let Some(officer) = surrender_state.arresting_officer {
                arrest_events.write(ArrestEvent {
                    target: player_entity,
                    officer,
                    arrest_type: ArrestType::PlayerSurrender,
                });
            }

            // 重置狀態
            surrender_state.has_surrendered = false;
            surrender_state.is_surrendering = false;
            surrender_state.being_arrested = false;
            surrender_state.arrest_progress = 0.0;
            surrender_state.arresting_officer = None;
            surrender_state.surrender_hold_timer = 0.0;
        }

        return;
    }

    // 尋找可以執行逮捕的警察
    for (police_entity, police_transform, officer) in &police_query {
        // 警覺、追逐、交戰、搜索狀態的警察皆可逮捕
        if !matches!(
            officer.state,
            PoliceState::Engaging | PoliceState::Pursuing | PoliceState::Alerted | PoliceState::Searching
        ) {
            continue;
        }

        let distance_sq = (police_transform.translation - player_pos).length_squared();

        if distance_sq <= ARREST_DISTANCE * ARREST_DISTANCE {
            // 開始逮捕
            surrender_state.being_arrested = true;
            surrender_state.arresting_officer = Some(police_entity);
            surrender_state.arrest_progress = 0.0;
            info!("🚔 警察開始逮捕玩家！");
            break;
        }
    }
}

/// 處理逮捕完成事件
pub fn handle_arrest_event_system(
    mut arrest_events: MessageReader<ArrestEvent>,
    mut arrest_complete: MessageWriter<ArrestComplete>,
    mut player_query: Query<(&mut Transform, &mut PlayerSurrenderState, &mut WeaponInventory), With<Player>>,
    mut wallet: ResMut<PlayerWallet>,
    mut wanted: ResMut<WantedLevel>,
) {
    for event in arrest_events.read() {
        match event.arrest_type {
            ArrestType::PlayerSurrender => {
                let Ok((mut transform, mut surrender_state, mut inventory)) = player_query.single_mut() else {
                    continue;
                };

                // 計算罰款
                let fine = wanted.stars as f32 * FINE_PER_STAR;
                let fine_amount = fine as i32;

                // 扣除罰款（盡可能支付，不足部分豁免）
                let _actual_paid = wallet.spend_up_to(fine_amount);

                // 沒收部分武器
                let weapons_count = inventory.weapons.len();
                let confiscate_count = (weapons_count as f32 * WEAPON_CONFISCATION_RATE) as usize;
                let mut confiscated = 0u32;

                // 從後往前移除武器（保留拳頭和手槍）
                for _ in 0..confiscate_count {
                    if inventory.weapons.len() > 2 {
                        inventory.weapons.pop();
                        confiscated += 1;
                    }
                }

                // 重置彈藥
                for weapon in &mut inventory.weapons {
                    weapon.current_ammo = weapon.stats.magazine_size / 2;
                    weapon.reserve_ammo = weapon.stats.max_ammo / 2;
                }

                // 傳送到警局
                transform.translation = POLICE_STATION_POSITION + Vec3::Y * 1.0;

                // 重置通緝等級
                wanted.stars = 0;
                wanted.heat = 0.0;
                wanted.search_center = None;
                wanted.player_last_seen_pos = None;

                // 設置免疫時間
                surrender_state.post_arrest_immunity = POST_ARREST_IMMUNITY;

                // 發送完成事件
                arrest_complete.write(ArrestComplete {
                    target: event.target,
                    fine,
                    weapons_confiscated: confiscated,
                });

                info!(
                    "逮捕完成！罰款: ${:.0}, 沒收武器: {}",
                    fine, confiscated
                );
            }

            ArrestType::EnemySurrender => {
                // 敵人投降處理（未來擴展）
                info!("敵人被逮捕: {:?}", event.target);
            }
        }
    }
}

/// 敵人投降檢測系統
pub fn enemy_surrender_check_system(
    config: Res<ArrestConfig>,
    mut enemy_query: Query<(&Health, &mut EnemySurrenderState)>,
    _police_query: Query<&Transform, With<PoliceOfficer>>,
) {
    if !config.enemy_surrender_enabled {
        return;
    }

    for (health, mut surrender_state) in &mut enemy_query {
        if !surrender_state.can_surrender || surrender_state.has_surrendered {
            continue;
        }

        // 檢查血量
        let health_percent = health.current / health.max;
        if health_percent <= ENEMY_SURRENDER_HEALTH_THRESHOLD {
            surrender_state.has_surrendered = true;
            surrender_state.surrender_reason = SurrenderReason::LowHealth;
            info!("敵人因血量過低投降！");
        }
    }
}

/// 投降視覺效果系統（舉手動畫）
pub fn surrender_visual_system(
    player_query: Query<(&PlayerSurrenderState, &Transform), With<Player>>,
    // 這裡可以添加動畫相關的查詢
) {
    let Ok((surrender_state, _transform)) = player_query.single() else {
        return;
    };

    if surrender_state.is_surrendering || surrender_state.has_surrendered {
        // 在這裡處理舉手動畫
        // 由於沒有骨骼動畫系統，這裡只是佔位
    }
}

/// 生成進度條 UI 的輔助函數
fn spawn_progress_bar_ui(
    commands: &mut Commands,
    bar_color: Color,
    text: &'static str,
) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(70.0),
            width: Val::Px(200.0),
            height: Val::Px(30.0),
            margin: UiRect::left(Val::Px(-100.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        SurrenderUI,
    )).with_children(|parent| {
        // 進度條
        parent.spawn((
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(bar_color),
            SurrenderProgressBar,
        ));
    }).with_children(|parent| {
        // 文字
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Text::new(text),
            TextColor(Color::WHITE),
            TextFont {
                font_size: 16.0,
                ..default()
            },
        ));
    });
}

/// 投降 UI 系統
pub fn surrender_ui_system(
    mut commands: Commands,
    player_query: Query<&PlayerSurrenderState, With<Player>>,
    ui_query: Query<Entity, With<SurrenderUI>>,
) {
    let Ok(surrender_state) = player_query.single() else {
        return;
    };

    let has_ui = !ui_query.is_empty();

    // 顯示載具內警告訊息
    if surrender_state.vehicle_warning_timer > 0.0 {
        if !has_ui {
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(70.0),
                    width: Val::Px(200.0),
                    height: Val::Px(30.0),
                    margin: UiRect::left(Val::Px(-100.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.8, 0.2, 0.2, 0.8)),
                SurrenderUI,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("請先下車再投降"),
                    TextColor(Color::WHITE),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                ));
            });
        }
        return;
    }

    // 判斷需要顯示的 UI 類型
    let needs_surrender_ui = surrender_state.is_surrendering && !surrender_state.has_surrendered;
    let needs_arrest_ui = surrender_state.being_arrested;

    if needs_surrender_ui && !has_ui {
        spawn_progress_bar_ui(&mut commands, Color::srgb(1.0, 1.0, 0.0), "投降中... 按住 Y");
    } else if needs_arrest_ui && !has_ui {
        spawn_progress_bar_ui(&mut commands, Color::srgb(1.0, 0.0, 0.0), "被逮捕中...");
    } else if !needs_surrender_ui && !needs_arrest_ui {
        // 移除 UI
        for entity in &ui_query {
            commands.entity(entity).despawn();
        }
    }
}

/// 更新投降進度條
pub fn update_surrender_progress_bar(
    player_query: Query<&PlayerSurrenderState, With<Player>>,
    mut progress_bar_query: Query<&mut Node, With<SurrenderProgressBar>>,
) {
    let Ok(surrender_state) = player_query.single() else {
        return;
    };

    let Ok(mut node) = progress_bar_query.single_mut() else {
        return;
    };

    let progress = if surrender_state.being_arrested {
        surrender_state.arrest_progress
    } else {
        surrender_state.surrender_hold_timer / SURRENDER_HOLD_TIME
    };

    node.width = Val::Percent(progress.min(1.0) * 100.0);
}

/// 投降 UI 標記
#[derive(Component)]
pub struct SurrenderUI;

/// 投降進度條標記
#[derive(Component)]
pub struct SurrenderProgressBar;
