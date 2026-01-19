//! 隨機事件系統
//!
//! GTA 風格的隨機事件，包含：
//! - 街頭搶劫
//! - 車禍
//! - 打架
//! - 乞丐求助
//! - 警察追捕逃犯

#![allow(dead_code)] // Phase 5+ 預留功能

use bevy::prelude::*;

use crate::player::Player;
use crate::economy::PlayerWallet;

// ============================================================================
// 常數
// ============================================================================

/// 隨機事件檢查間隔（秒）
const EVENT_CHECK_INTERVAL: f32 = 30.0;
/// 事件最小距離（距玩家）
const EVENT_MIN_DISTANCE: f32 = 20.0;
/// 事件最大距離（距玩家）
const EVENT_MAX_DISTANCE: f32 = 50.0;
/// 事件有效距離（玩家可以介入的距離）
const EVENT_ACTIVE_DISTANCE: f32 = 15.0;
/// 事件超時時間（秒）
const EVENT_TIMEOUT: f32 = 120.0;
/// 基礎獎勵金額
const BASE_REWARD: i32 = 200;

// ============================================================================
// 組件與資源
// ============================================================================

/// 隨機事件管理器
#[derive(Resource)]
pub struct RandomEventManager {
    /// 事件檢查計時器
    pub check_timer: f32,
    /// 當前活動的事件數量
    pub active_event_count: u32,
    /// 最大同時活動事件數量
    pub max_active_events: u32,
    /// 事件觸發機率（0.0-1.0）
    pub spawn_chance: f32,
    /// 是否啟用隨機事件
    pub enabled: bool,
    /// 總完成事件數
    pub total_completed: u32,
    /// 總獎勵獲得
    pub total_rewards: i32,
}

impl Default for RandomEventManager {
    fn default() -> Self {
        Self {
            check_timer: 0.0,
            active_event_count: 0,
            max_active_events: 3,
            spawn_chance: 0.3,
            enabled: true,
            total_completed: 0,
            total_rewards: 0,
        }
    }
}

/// 隨機事件類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomEventType {
    /// 街頭搶劫：有人搶奪行人財物，玩家可以追捕搶匪或幫助受害者
    StreetRobbery,
    /// 車禍：兩車相撞，可能需要幫助受傷者
    CarAccident,
    /// 打架：兩人或多人打架，玩家可以介入或報警
    StreetFight,
    /// 乞丐：乞丐請求金錢，給錢可能獲得情報或好感
    Beggar,
    /// 警察追捕：警察追捕逃犯，玩家可以幫助或阻礙
    PoliceChase,
    /// 失竊車輛：有人試圖偷車
    CarTheft,
    /// 路邊攤販：特價商品
    StreetVendor,
}

impl RandomEventType {
    /// 獲取事件基礎獎勵
    pub fn base_reward(&self) -> i32 {
        match self {
            RandomEventType::StreetRobbery => 300,
            RandomEventType::CarAccident => 100,
            RandomEventType::StreetFight => 150,
            RandomEventType::Beggar => -50, // 負數表示需要支付
            RandomEventType::PoliceChase => 500,
            RandomEventType::CarTheft => 250,
            RandomEventType::StreetVendor => 0,
        }
    }

    /// 獲取事件描述
    pub fn description(&self) -> &'static str {
        match self {
            RandomEventType::StreetRobbery => "有人正在搶劫！",
            RandomEventType::CarAccident => "發生車禍！",
            RandomEventType::StreetFight => "有人在打架！",
            RandomEventType::Beggar => "乞丐請求幫助",
            RandomEventType::PoliceChase => "警察正在追捕逃犯！",
            RandomEventType::CarTheft => "有人正在偷車！",
            RandomEventType::StreetVendor => "路邊攤特價！",
        }
    }

    /// 獲取事件持續時間
    pub fn duration(&self) -> f32 {
        match self {
            RandomEventType::StreetRobbery => 60.0,
            RandomEventType::CarAccident => 180.0,
            RandomEventType::StreetFight => 45.0,
            RandomEventType::Beggar => 90.0,
            RandomEventType::PoliceChase => 120.0,
            RandomEventType::CarTheft => 30.0,
            RandomEventType::StreetVendor => 300.0,
        }
    }
}

/// 隨機事件組件
#[derive(Component)]
pub struct RandomEvent {
    /// 事件類型
    pub event_type: RandomEventType,
    /// 事件狀態
    pub state: RandomEventState,
    /// 事件位置
    pub position: Vec3,
    /// 剩餘時間
    pub remaining_time: f32,
    /// 參與者實體
    pub participants: Vec<Entity>,
    /// 獎勵已領取
    pub reward_claimed: bool,
    /// 玩家已介入
    pub player_intervened: bool,
    /// 絕對超時計時器（防止卡住）
    pub absolute_timeout: f32,
}

impl RandomEvent {
    pub fn new(event_type: RandomEventType, position: Vec3) -> Self {
        Self {
            event_type,
            state: RandomEventState::Active,
            position,
            remaining_time: event_type.duration(),
            participants: Vec::new(),
            reward_claimed: false,
            player_intervened: false,
            absolute_timeout: EVENT_TIMEOUT,
        }
    }

    /// 檢查是否需要強制結束（超時或參與者無效）
    pub fn should_force_end(&self) -> bool {
        self.absolute_timeout <= 0.0
    }
}

/// 隨機事件狀態
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RandomEventState {
    /// 活動中
    #[default]
    Active,
    /// 玩家介入中
    PlayerIntervening,
    /// 已解決（成功）
    Resolved,
    /// 已失敗（超時或逃跑）
    Failed,
    /// 已完成（獎勵已領取）
    Completed,
}

/// 事件標記組件（用於 UI 顯示）
#[derive(Component)]
pub struct RandomEventMarker {
    /// 關聯的事件實體
    pub event_entity: Entity,
}

/// 事件 UI 組件
#[derive(Component)]
pub struct RandomEventUI;

/// 事件通知組件
#[derive(Component)]
pub struct EventNotification {
    /// 通知文字
    pub text: String,
    /// 剩餘顯示時間
    pub remaining_time: f32,
}

// ============================================================================
// 事件
// ============================================================================

/// 隨機事件觸發事件
#[derive(Message, Clone)]
pub struct RandomEventTriggered {
    pub event_entity: Entity,
    pub event_type: RandomEventType,
    pub position: Vec3,
}

/// 隨機事件完成事件
#[derive(Message, Clone)]
pub struct RandomEventCompleted {
    pub event_entity: Entity,
    pub event_type: RandomEventType,
    pub success: bool,
    pub reward: i32,
}

// ============================================================================
// 系統
// ============================================================================

/// 初始化隨機事件系統
pub fn setup_random_events(mut commands: Commands) {
    commands.init_resource::<RandomEventManager>();
}

/// 隨機事件生成系統
pub fn random_event_spawn_system(
    mut commands: Commands,
    mut manager: ResMut<RandomEventManager>,
    player_query: Query<&Transform, With<Player>>,
    event_query: Query<Entity, With<RandomEvent>>,
    time: Res<Time>,
    mut event_triggered: MessageWriter<RandomEventTriggered>,
) {
    if !manager.enabled {
        return;
    }

    manager.check_timer += time.delta_secs();

    if manager.check_timer < EVENT_CHECK_INTERVAL {
        return;
    }

    manager.check_timer = 0.0;

    // 檢查是否達到上限
    manager.active_event_count = event_query.iter().count() as u32;
    if manager.active_event_count >= manager.max_active_events {
        return;
    }

    // 機率檢查
    if rand::random::<f32>() > manager.spawn_chance {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation;

    // 隨機選擇事件類型
    let event_types = [
        RandomEventType::StreetRobbery,
        RandomEventType::CarAccident,
        RandomEventType::StreetFight,
        RandomEventType::Beggar,
        RandomEventType::CarTheft,
        RandomEventType::StreetVendor,
    ];

    let event_type = event_types[(rand::random::<u32>() as usize) % event_types.len()];

    // 計算事件位置（玩家周圍隨機位置）
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let distance = EVENT_MIN_DISTANCE + rand::random::<f32>() * (EVENT_MAX_DISTANCE - EVENT_MIN_DISTANCE);

    let event_pos = Vec3::new(
        player_pos.x + angle.cos() * distance,
        0.0,
        player_pos.z + angle.sin() * distance,
    );

    // 生成事件
    let event_entity = commands.spawn((
        Name::new(format!("RandomEvent_{:?}", event_type)),
        Transform::from_translation(event_pos),
        GlobalTransform::default(),
        RandomEvent::new(event_type, event_pos),
    )).id();

    // 發送觸發事件
    event_triggered.write(RandomEventTriggered {
        event_entity,
        event_type,
        position: event_pos,
    });

    info!("隨機事件生成: {:?} at ({:.1}, {:.1})", event_type, event_pos.x, event_pos.z);
}

/// 隨機事件更新系統
pub fn random_event_update_system(
    mut commands: Commands,
    mut event_query: Query<(Entity, &mut RandomEvent, &Transform)>,
    player_query: Query<&Transform, (With<Player>, Without<RandomEvent>)>,
    participant_query: Query<Entity>,
    mut event_completed: MessageWriter<RandomEventCompleted>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let dt = time.delta_secs();

    for (entity, mut event, transform) in &mut event_query {
        // 更新計時器
        event.remaining_time -= dt;
        event.absolute_timeout -= dt;

        // 檢查絕對超時（防止任何情況下卡住）
        if event.should_force_end() {
            if event.state != RandomEventState::Completed {
                warn!("事件 {:?} 強制結束（絕對超時）", event.event_type);
                event.state = RandomEventState::Failed;
            }
        }

        // 檢查參與者是否仍然存在（防止參與者被殺死後卡住）
        if !event.participants.is_empty() {
            let valid_participants: Vec<Entity> = event.participants
                .iter()
                .filter(|&&e| participant_query.get(e).is_ok())
                .copied()
                .collect();

            // 如果所有參與者都消失了，根據事件類型決定結果
            if valid_participants.is_empty() && event.state == RandomEventState::Active {
                info!("事件 {:?} 所有參與者消失，自動完成", event.event_type);
                // 搶劫/偷車事件：參與者消失視為玩家成功
                // 其他事件：視為失敗
                event.state = match event.event_type {
                    RandomEventType::StreetRobbery | RandomEventType::CarTheft => {
                        RandomEventState::Resolved
                    }
                    _ => RandomEventState::Failed,
                };
            }

            event.participants = valid_participants;
        }

        // 計算玩家距離
        let distance = (transform.translation - player_pos).length();

        match event.state {
            RandomEventState::Active => {
                // 檢查玩家是否靠近
                if distance < EVENT_ACTIVE_DISTANCE && !event.player_intervened {
                    event.state = RandomEventState::PlayerIntervening;
                    event.player_intervened = true;
                    info!("玩家介入事件: {:?}", event.event_type);
                }

                // 超時失敗
                if event.remaining_time <= 0.0 {
                    event.state = RandomEventState::Failed;
                }
            }

            RandomEventState::PlayerIntervening => {
                // 簡化：玩家靠近一段時間後自動解決
                // 實際遊戲中應該根據事件類型有不同的解決條件
                if event.remaining_time <= event.event_type.duration() - 10.0 {
                    event.state = RandomEventState::Resolved;
                }

                // 玩家離開太遠，事件失敗
                if distance > EVENT_MAX_DISTANCE {
                    event.state = RandomEventState::Failed;
                }
            }

            RandomEventState::Resolved => {
                if !event.reward_claimed {
                    let reward = event.event_type.base_reward();

                    event_completed.write(RandomEventCompleted {
                        event_entity: entity,
                        event_type: event.event_type,
                        success: true,
                        reward,
                    });

                    event.reward_claimed = true;
                    event.state = RandomEventState::Completed;
                }
            }

            RandomEventState::Failed => {
                if !event.reward_claimed {
                    event_completed.write(RandomEventCompleted {
                        event_entity: entity,
                        event_type: event.event_type,
                        success: false,
                        reward: 0,
                    });

                    event.reward_claimed = true;
                    event.state = RandomEventState::Completed;
                }
            }

            RandomEventState::Completed => {
                // 完成後移除事件
                commands.entity(entity).despawn();
            }
        }
    }
}

/// 處理事件完成
pub fn handle_event_completed_system(
    mut event_completed: MessageReader<RandomEventCompleted>,
    mut wallet: ResMut<PlayerWallet>,
    mut manager: ResMut<RandomEventManager>,
) {
    for event in event_completed.read() {
        if event.success && event.reward > 0 {
            wallet.add_cash(event.reward);
            manager.total_rewards += event.reward;
            info!("事件完成！獲得 ${}", event.reward);
        } else if event.reward < 0 {
            // 負獎勵表示需要支付（如幫助乞丐）
            // 使用 spend_up_to 確保統計正確
            wallet.spend_up_to(-event.reward);
        }

        if event.success {
            manager.total_completed += 1;
        }
    }
}

/// 事件通知 UI 系統
pub fn event_notification_system(
    mut commands: Commands,
    mut event_triggered: MessageReader<RandomEventTriggered>,
    mut notification_query: Query<(Entity, &mut EventNotification)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // 處理新事件通知
    for event in event_triggered.read() {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Px(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                margin: UiRect::left(Val::Px(-150.0)),
                width: Val::Px(300.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            EventNotification {
                text: event.event_type.description().to_string(),
                remaining_time: 5.0,
            },
        )).with_children(|parent| {
            parent.spawn((
                Text::new(event.event_type.description()),
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
            ));
        });
    }

    // 更新通知計時器
    for (entity, mut notification) in &mut notification_query {
        notification.remaining_time -= dt;

        if notification.remaining_time <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// 事件標記 UI 系統（小地圖或 HUD 上的標記）
pub fn event_marker_system(
    event_query: Query<(&RandomEvent, &Transform)>,
    player_query: Query<&Transform, (With<Player>, Without<RandomEvent>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 這裡可以更新小地圖上的事件標記
    // 由於沒有小地圖系統，暫時只做距離檢測
    for (event, transform) in &event_query {
        if event.state == RandomEventState::Active {
            let distance = (transform.translation - player_pos).length();
            if distance < EVENT_ACTIVE_DISTANCE * 2.0 {
                // 在這裡可以顯示提示
            }
        }
    }
}
