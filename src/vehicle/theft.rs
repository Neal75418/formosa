//! 偷車系統
//!
//! GTA 風格的偷車機制，包含：
//! - 車輛所有權檢測
//! - 破窗動畫
//! - 熱線啟動小遊戲
//! - 車主反應 AI
//! - 警報系統

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

use crate::player::Player;
use crate::wanted::CrimeEvent;
use crate::pedestrian::{Pedestrian, PedestrianState, PedState};
use crate::core::GameState;
use crate::combat::{RespawnState, Health};

use super::components::{Vehicle, VehicleType};

// ============================================================================
// 常數
// ============================================================================

/// 破窗動畫時長（秒）
const WINDOW_BREAK_DURATION: f32 = 0.8;
/// 熱線啟動時長（秒）- 根據車輛類型不同
const HOTWIRE_BASE_DURATION: f32 = 3.0;
/// 車主逃跑機率
const OWNER_FLEE_CHANCE: f32 = 0.7;
/// 車主反擊機率
const OWNER_FIGHT_CHANCE: f32 = 0.2;
/// 警報持續時間（秒）
const ALARM_DURATION: f32 = 30.0;
/// 車門互動距離
const DOOR_INTERACT_DISTANCE: f32 = 3.0;

// 視覺效果常數
/// 玻璃碎片旋轉速度 X
const SHARD_ROTATION_SPEED_X: f32 = 5.0;
/// 玻璃碎片旋轉速度 Z
const SHARD_ROTATION_SPEED_Z: f32 = 3.0;
/// 玻璃碎片基礎生命週期
const SHARD_BASE_LIFETIME: f32 = 2.0;
/// 火花閃爍速率
const SPARK_FLICKER_RATE: f32 = 10.0;
/// 重力加速度
const GRAVITY: f32 = 9.8;

// UI 常數
/// 進度條左側位置（百分比）
const THEFT_UI_LEFT: f32 = 40.0;
/// 進度條頂部位置（百分比）
const THEFT_UI_TOP: f32 = 60.0;
/// 進度條寬度（百分比）
const THEFT_UI_WIDTH: f32 = 20.0;
/// 進度條高度（像素）
const THEFT_UI_HEIGHT: f32 = 20.0;
/// 進度條邊框寬度（像素）
const THEFT_UI_BORDER: f32 = 2.0;

// ============================================================================
// 組件與資源
// ============================================================================

/// 車輛所有權組件
#[derive(Component)]
pub struct VehicleOwnership {
    /// 車主實體（None 表示無主車輛）
    pub owner: Option<Entity>,
    /// 車門是否上鎖
    pub is_locked: bool,
    /// 車窗是否完好
    pub window_intact: bool,
    /// 警報系統是否啟用
    pub has_alarm: bool,
    /// 警報是否響起中
    pub alarm_active: bool,
    /// 警報剩餘時間
    pub alarm_timer: f32,
}

impl Default for VehicleOwnership {
    fn default() -> Self {
        Self {
            owner: None,
            is_locked: true,
            window_intact: true,
            has_alarm: true,
            alarm_active: false,
            alarm_timer: 0.0,
        }
    }
}

impl VehicleOwnership {
    /// 玩家擁有的車輛（不上鎖）
    pub fn player_owned() -> Self {
        Self {
            owner: None, // 玩家擁有
            is_locked: false,
            window_intact: true,
            has_alarm: false,
            alarm_active: false,
            alarm_timer: 0.0,
        }
    }

    /// NPC 擁有的車輛
    pub fn npc_owned(owner: Entity, has_alarm: bool) -> Self {
        Self {
            owner: Some(owner),
            is_locked: true,
            window_intact: true,
            has_alarm,
            alarm_active: false,
            alarm_timer: 0.0,
        }
    }

    /// 路邊停放的無主車輛
    pub fn parked(has_alarm: bool) -> Self {
        Self {
            owner: None,
            is_locked: true,
            window_intact: true,
            has_alarm,
            alarm_active: false,
            alarm_timer: 0.0,
        }
    }
}

/// 偷車狀態
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TheftState {
    /// 無偷車行為
    #[default]
    None,
    /// 靠近車門
    Approaching,
    /// 打破車窗
    BreakingWindow,
    /// 熱線啟動
    Hotwiring,
    /// 成功進入
    Entered,
    /// 被打斷
    Interrupted,
}

/// 玩家偷車狀態組件
#[derive(Component, Default)]
pub struct PlayerTheftState {
    /// 當前狀態
    pub state: TheftState,
    /// 目標車輛
    pub target_vehicle: Option<Entity>,
    /// 當前動畫進度 (0.0 ~ 1.0)
    pub progress: f32,
    /// 當前階段所需時間
    pub required_time: f32,
    /// 累積時間
    pub elapsed_time: f32,
    /// 開始偷車時的血量（用於檢測受傷中斷）
    pub initial_health: f32,
}

impl PlayerTheftState {
    /// 開始偷車流程
    pub fn start_theft(&mut self, vehicle: Entity, required_time: f32) {
        self.state = TheftState::Approaching;
        self.target_vehicle = Some(vehicle);
        self.progress = 0.0;
        self.required_time = required_time;
        self.elapsed_time = 0.0;
    }

    /// 進入下一階段
    pub fn next_stage(&mut self, state: TheftState, required_time: f32) {
        self.state = state;
        self.progress = 0.0;
        self.required_time = required_time;
        self.elapsed_time = 0.0;
    }

    /// 重置狀態
    pub fn reset(&mut self) {
        self.state = TheftState::None;
        self.target_vehicle = None;
        self.progress = 0.0;
        self.required_time = 0.0;
        self.elapsed_time = 0.0;
    }

    /// 是否正在偷車
    pub fn is_stealing(&self) -> bool {
        !matches!(self.state, TheftState::None | TheftState::Entered)
    }
}

/// 車主反應組件
#[derive(Component)]
pub struct VehicleOwnerReaction {
    /// 反應類型
    pub reaction: OwnerReactionType,
    /// 反應計時器
    pub reaction_timer: f32,
    /// 是否已反應
    pub has_reacted: bool,
}

/// 車主反應類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnerReactionType {
    /// 逃跑
    Flee,
    /// 反擊
    Fight,
    /// 呼救
    CallForHelp,
}

/// 偷車視覺效果資源
#[derive(Resource)]
pub struct TheftVisuals {
    /// 玻璃碎片網格
    pub glass_shard_mesh: Handle<Mesh>,
    /// 玻璃材質
    pub glass_material: Handle<StandardMaterial>,
    /// 火花材質
    pub spark_material: Handle<StandardMaterial>,
}

impl TheftVisuals {
    /// 建立新實例
    pub fn new(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        Self {
            glass_shard_mesh: meshes.add(Mesh::from(Cuboid::new(0.05, 0.08, 0.01))),
            glass_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.6, 0.8, 0.9, 0.6),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            spark_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.7, 0.2),
                emissive: LinearRgba::new(5.0, 3.0, 0.5, 1.0),
                unlit: true,
                ..default()
            }),
        }
    }
}

/// 玻璃碎片組件
#[derive(Component)]
pub struct GlassShard {
    pub lifetime: f32,
    pub velocity: Vec3,
}

/// 熱線火花組件
#[derive(Component)]
pub struct HotwireSpark {
    pub lifetime: f32,
}

/// 偷車 UI 組件
#[derive(Component)]
pub struct TheftProgressUI;

/// 偷車事件
#[derive(Message, Clone)]
pub struct TheftEvent {
    /// 偷車者
    pub thief: Entity,
    /// 目標車輛
    pub vehicle: Entity,
    /// 事件類型
    pub event_type: TheftEventType,
}

/// 偷車事件類型
#[derive(Clone, Copy, Debug)]
pub enum TheftEventType {
    /// 開始偷車
    Started,
    /// 打破車窗
    WindowBroken,
    /// 熱線啟動中
    Hotwiring,
    /// 成功偷車
    Succeeded,
    /// 被打斷
    Interrupted,
    /// 警報觸發
    AlarmTriggered,
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 尋找最近的上鎖車輛
fn find_nearest_locked_vehicle(
    player_pos: Vec3,
    vehicle_query: &Query<(Entity, &Transform, &Vehicle, Option<&VehicleOwnership>), Without<Player>>,
) -> Option<(Entity, VehicleType)> {
    vehicle_query.iter()
        .filter_map(|(entity, transform, vehicle, ownership)| {
            // 跳過機車（通常不用破窗）
            if vehicle.vehicle_type == VehicleType::Scooter {
                return None;
            }

            let distance = (transform.translation - player_pos).length();
            if distance > DOOR_INTERACT_DISTANCE {
                return None;
            }

            // 檢查是否上鎖
            ownership.filter(|o| o.is_locked).map(|_| (entity, distance, vehicle.vehicle_type))
        })
        .min_by(|(_, a, _), (_, b, _)| a.total_cmp(b))
        .map(|(entity, _, vehicle_type)| (entity, vehicle_type))
}

/// 發送偷車中斷事件並重置狀態
fn send_interrupt_and_reset(
    theft_state: &mut PlayerTheftState,
    theft_events: &mut MessageWriter<TheftEvent>,
    player_entity: Entity,
    reason: &str,
) {
    if let Some(vehicle_entity) = theft_state.target_vehicle {
        theft_events.write(TheftEvent {
            thief: player_entity,
            vehicle: vehicle_entity,
            event_type: TheftEventType::Interrupted,
        });
    }
    theft_state.reset();
    info!("偷車中斷：{}", reason);
}

/// 決定車主反應類型
fn decide_owner_reaction() -> OwnerReactionType {
    if rand::random::<f32>() < OWNER_FLEE_CHANCE {
        OwnerReactionType::Flee
    } else if rand::random::<f32>() < OWNER_FIGHT_CHANCE / (1.0 - OWNER_FLEE_CHANCE) {
        OwnerReactionType::Fight
    } else {
        OwnerReactionType::CallForHelp
    }
}

/// 生成隨機旋轉四元數
pub fn random_rotation() -> Quat {
    Quat::from_euler(
        EulerRot::XYZ,
        rand::random::<f32>() * std::f32::consts::TAU,
        rand::random::<f32>() * std::f32::consts::TAU,
        rand::random::<f32>() * std::f32::consts::TAU,
    )
}

/// 設置行人恐慌狀態
fn set_pedestrian_panic(
    ped_state: &mut PedestrianState,
    threat_pos: Vec3,
    fear_level: f32,
) {
    ped_state.state = PedState::Fleeing;
    ped_state.fear_level = fear_level;
    ped_state.last_threat_pos = Some(threat_pos);
}

/// 檢查偷車是否被中斷（玩家受傷或按鍵放開）
fn check_theft_interruption(
    theft_state: &PlayerTheftState,
    player_health: Option<&Health>,
    keyboard: &ButtonInput<KeyCode>,
) -> Option<&'static str> {
    // 檢查玩家是否受傷
    if let Some(health) = player_health {
        if health.current < theft_state.initial_health - 0.1 {
            return Some("玩家被攻擊");
        }
    }

    // 檢查是否放開按鍵
    if !keyboard.pressed(KeyCode::KeyX) {
        return Some("放開按鍵");
    }

    None
}

/// 處理破窗階段完成
fn handle_window_break_complete(
    commands: &mut Commands,
    visuals: &TheftVisuals,
    theft_state: &mut PlayerTheftState,
    ownership: Option<&mut VehicleOwnership>,
    vehicle_transform: &Transform,
    player_entity: Entity,
    vehicle_entity: Entity,
    theft_events: &mut MessageWriter<TheftEvent>,
    crime_events: &mut MessageWriter<CrimeEvent>,
) {
    // 生成玻璃碎片
    spawn_glass_shards(commands, visuals, vehicle_transform.translation);

    // 觸發警報
    if let Some(ownership) = ownership {
        ownership.window_intact = false;
        if ownership.has_alarm {
            ownership.alarm_active = true;
            ownership.alarm_timer = ALARM_DURATION;
            info!("車輛警報觸發！");

            theft_events.write(TheftEvent {
                thief: player_entity,
                vehicle: vehicle_entity,
                event_type: TheftEventType::AlarmTriggered,
            });
        }
    }

    theft_events.write(TheftEvent {
        thief: player_entity,
        vehicle: vehicle_entity,
        event_type: TheftEventType::WindowBroken,
    });

    // 觸發犯罪事件
    crime_events.write(CrimeEvent::VehicleTheft {
        position: vehicle_transform.translation,
    });

    // 進入熱線啟動階段
    theft_state.next_stage(TheftState::Hotwiring, HOTWIRE_BASE_DURATION);
    info!("車窗打破，開始熱線啟動");
}

/// 處理熱線啟動階段完成
fn handle_hotwire_complete(
    theft_state: &mut PlayerTheftState,
    ownership: Option<&mut VehicleOwnership>,
    player_entity: Entity,
    vehicle_entity: Entity,
    theft_events: &mut MessageWriter<TheftEvent>,
) {
    // 解鎖車輛
    if let Some(ownership) = ownership {
        ownership.is_locked = false;
    }

    theft_events.write(TheftEvent {
        thief: player_entity,
        vehicle: vehicle_entity,
        event_type: TheftEventType::Succeeded,
    });

    theft_state.next_stage(TheftState::Entered, 0.0);
    info!("🚗 偷車成功: 車輛 {:?}", vehicle_entity);
}

/// 處理車主對偷車事件的反應
fn handle_owner_theft_reaction(
    commands: &mut Commands,
    owner_entity: Entity,
    vehicle_pos: Vec3,
    ped_state_query: &mut Query<&mut PedestrianState>,
) {
    let reaction = decide_owner_reaction();

    commands.entity(owner_entity).insert(VehicleOwnerReaction {
        reaction,
        reaction_timer: 5.0,
        has_reacted: false,
    });

    if let Ok(mut ped_state) = ped_state_query.get_mut(owner_entity) {
        set_pedestrian_panic(&mut ped_state, vehicle_pos, 1.0);
    }

    info!("車主反應: {:?}", reaction);
}

/// 警報附近的行人
fn alert_nearby_pedestrians(
    vehicle_pos: Vec3,
    pedestrian_query: &Query<(Entity, &Transform), With<Pedestrian>>,
    ped_state_query: &mut Query<&mut PedestrianState>,
) {
    for (entity, ped_transform) in pedestrian_query {
        let distance = (ped_transform.translation - vehicle_pos).length();
        if distance < 15.0 && rand::random::<f32>() < 0.3 {
            if let Ok(mut ped_state) = ped_state_query.get_mut(entity) {
                set_pedestrian_panic(&mut ped_state, vehicle_pos, 0.8);
            }
        }
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 初始化偷車視覺效果
pub fn setup_theft_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TheftVisuals::new(&mut meshes, &mut materials));
}

/// 偷車輸入系統
pub fn theft_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    respawn_state: Res<RespawnState>,
    mut player_query: Query<(&Transform, &mut PlayerTheftState, Option<&Health>), With<Player>>,
    vehicle_query: Query<(Entity, &Transform, &Vehicle, Option<&VehicleOwnership>), Without<Player>>,
) {
    // 如果已經在車上或玩家死亡，不處理偷車
    if game_state.player_in_vehicle || respawn_state.is_dead {
        return;
    }

    let Ok((player_transform, mut theft_state, player_health)) = player_query.single_mut() else {
        return;
    };

    let player_pos = player_transform.translation;

    // 如果正在偷車，按 Esc 可以取消
    if theft_state.is_stealing() {
        if keyboard.just_pressed(KeyCode::Escape) {
            theft_state.reset();
            info!("偷車取消");
        }
        return;
    }

    // X 鍵觸發偷車（如果車輛上鎖）
    if !keyboard.just_pressed(KeyCode::KeyX) {
        return;
    }

    // 尋找最近的上鎖車輛
    let Some((vehicle_entity, vehicle_type)) = find_nearest_locked_vehicle(player_pos, &vehicle_query) else {
        return;
    };

    // 根據車輛類型計算偷車時間
    let _hotwire_time = match vehicle_type {
        VehicleType::Car => HOTWIRE_BASE_DURATION,
        VehicleType::Taxi => HOTWIRE_BASE_DURATION * 0.8, // 計程車較容易
        VehicleType::Bus => HOTWIRE_BASE_DURATION * 1.5,  // 公車較難
        VehicleType::Scooter => HOTWIRE_BASE_DURATION * 0.5, // 機車最容易
    };

    theft_state.start_theft(vehicle_entity, WINDOW_BREAK_DURATION);
    theft_state.next_stage(TheftState::BreakingWindow, WINDOW_BREAK_DURATION);

    // 記錄開始偷車時的血量（用於檢測被攻擊中斷）
    theft_state.initial_health = player_health.map(|h| h.current).unwrap_or(100.0);

    info!("開始偷車");
}

/// 偷車進度更新系統
pub fn theft_progress_system(
    mut commands: Commands,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    respawn_state: Res<RespawnState>,
    mut player_query: Query<(Entity, &Transform, &mut PlayerTheftState, Option<&Health>), With<Player>>,
    mut vehicle_query: Query<(Entity, &Transform, &mut Vehicle, Option<&mut VehicleOwnership>)>,
    visuals: Option<Res<TheftVisuals>>,
    mut theft_events: MessageWriter<TheftEvent>,
    mut crime_events: MessageWriter<CrimeEvent>,
) {
    let dt = time.delta_secs();
    let Some(visuals) = visuals else { return; };

    let Ok((player_entity, player_transform, mut theft_state, player_health)) = player_query.single_mut() else {
        return;
    };

    // 玩家死亡時重置偷車狀態
    if respawn_state.is_dead && theft_state.is_stealing() {
        send_interrupt_and_reset(&mut theft_state, &mut theft_events, player_entity, "玩家死亡");
        return;
    }

    if !theft_state.is_stealing() {
        return;
    }

    let Some(vehicle_entity) = theft_state.target_vehicle else {
        theft_state.reset();
        return;
    };

    // 檢查中斷條件
    if let Some(reason) = check_theft_interruption(&theft_state, player_health, &keyboard) {
        send_interrupt_and_reset(&mut theft_state, &mut theft_events, player_entity, reason);
        return;
    }

    // 檢查距離
    let Ok((_, vehicle_transform, _, _)) = vehicle_query.get(vehicle_entity) else {
        theft_state.reset();
        return;
    };

    let distance = (vehicle_transform.translation - player_transform.translation).length();
    if distance > DOOR_INTERACT_DISTANCE + 1.0 {
        theft_state.reset();
        info!("偷車中斷：距離過遠");
        return;
    }

    // 更新進度
    theft_state.elapsed_time += dt;
    theft_state.progress = (theft_state.elapsed_time / theft_state.required_time).min(1.0);

    // 階段完成檢查
    if theft_state.progress < 1.0 {
        return;
    }

    match theft_state.state {
        TheftState::BreakingWindow => {
            let Ok((_, vehicle_transform, _, mut ownership)) = vehicle_query.get_mut(vehicle_entity) else {
                theft_state.reset();
                return;
            };
            handle_window_break_complete(
                &mut commands, &visuals, &mut theft_state, ownership.as_deref_mut(),
                vehicle_transform, player_entity, vehicle_entity,
                &mut theft_events, &mut crime_events,
            );
        }
        TheftState::Hotwiring => {
            let Ok((_, _, _, mut ownership)) = vehicle_query.get_mut(vehicle_entity) else {
                theft_state.reset();
                return;
            };
            handle_hotwire_complete(
                &mut theft_state, ownership.as_deref_mut(),
                player_entity, vehicle_entity, &mut theft_events,
            );
        }
        _ => {}
    }
}

/// 車輛警報更新系統
pub fn vehicle_alarm_system(
    mut vehicle_query: Query<(Entity, &mut VehicleOwnership)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (_, mut ownership) in &mut vehicle_query {
        if ownership.alarm_active {
            ownership.alarm_timer -= dt;

            if ownership.alarm_timer <= 0.0 {
                ownership.alarm_active = false;
                info!("車輛警報停止");
            }
        }
    }
}

/// 車主反應系統
pub fn owner_reaction_system(
    mut commands: Commands,
    mut theft_events: MessageReader<TheftEvent>,
    pedestrian_query: Query<(Entity, &Transform), With<Pedestrian>>,
    mut ped_state_query: Query<&mut PedestrianState>,
    vehicle_query: Query<(&Transform, &VehicleOwnership)>,
    player_query: Query<&Transform, With<Player>>,
) {
    for event in theft_events.read() {
        // 只處理破窗和警報觸發事件
        if !matches!(event.event_type, TheftEventType::WindowBroken | TheftEventType::AlarmTriggered) {
            continue;
        }

        let Ok((vehicle_transform, ownership)) = vehicle_query.get(event.vehicle) else {
            continue;
        };

        let vehicle_pos = vehicle_transform.translation;

        // 處理車主反應
        if let Some(owner_entity) = ownership.owner {
            if pedestrian_query.get(owner_entity).is_ok() {
                handle_owner_theft_reaction(&mut commands, owner_entity, vehicle_pos, &mut ped_state_query);
            }
        }

        // 附近行人也可能報警
        if player_query.single().is_ok() {
            alert_nearby_pedestrians(vehicle_pos, &pedestrian_query, &mut ped_state_query);
        }
    }
}

/// 玻璃碎片更新系統
pub fn glass_shard_update_system(
    mut commands: Commands,
    mut shard_query: Query<(Entity, &mut GlassShard, &mut Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut shard, mut transform) in &mut shard_query {
        shard.lifetime -= dt;

        if shard.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 移動
        transform.translation += shard.velocity * dt;

        // 重力
        shard.velocity.y -= GRAVITY * dt;

        // 旋轉
        transform.rotate_x(dt * SHARD_ROTATION_SPEED_X);
        transform.rotate_z(dt * SHARD_ROTATION_SPEED_Z);

        // 縮小
        let scale = (shard.lifetime / SHARD_BASE_LIFETIME).min(1.0);
        transform.scale = Vec3::splat(scale * 0.5 + 0.5);
    }
}

/// 熱線火花更新系統
pub fn hotwire_spark_update_system(
    mut commands: Commands,
    mut spark_query: Query<(Entity, &mut HotwireSpark, &mut Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (entity, mut spark, mut transform) in &mut spark_query {
        spark.lifetime -= dt;

        if spark.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 閃爍縮放
        let scale = (spark.lifetime * SPARK_FLICKER_RATE).sin().abs() * 0.5 + 0.5;
        transform.scale = Vec3::splat(scale * 0.1);
    }
}

/// 偷車進度 UI 更新系統
pub fn theft_ui_system(
    mut commands: Commands,
    player_query: Query<&PlayerTheftState, With<Player>>,
    mut ui_query: Query<(Entity, &mut Node, &mut BackgroundColor), With<TheftProgressUI>>,
) {
    let Ok(theft_state) = player_query.single() else {
        // 移除 UI
        for (entity, _, _) in &ui_query {
            commands.entity(entity).despawn();
        }
        return;
    };

    if theft_state.is_stealing() {
        // 創建或更新進度條
        if ui_query.is_empty() {
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(THEFT_UI_LEFT),
                    top: Val::Percent(THEFT_UI_TOP),
                    width: Val::Percent(THEFT_UI_WIDTH),
                    height: Val::Px(THEFT_UI_HEIGHT),
                    border: UiRect::all(Val::Px(THEFT_UI_BORDER)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                BorderColor::all(Color::WHITE),
                TheftProgressUI,
            )).with_children(|parent| {
                parent.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 0.5, 0.0)),
                    Name::new("ProgressBar"),
                ));
            });
        } else {
            // 更新進度條
            for (_, _, _) in &mut ui_query {
                // 這裡簡化處理，實際需要更新子節點的寬度
            }
        }
    } else {
        // 移除 UI
        for (entity, _, _) in &ui_query {
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// 輔助函數
// ============================================================================

/// 生成玻璃碎片
fn spawn_glass_shards(
    commands: &mut Commands,
    visuals: &TheftVisuals,
    position: Vec3,
) {
    for _ in 0..12 {
        let velocity = Vec3::new(
            (rand::random::<f32>() - 0.5) * 4.0,
            rand::random::<f32>() * 3.0 + 1.0,
            (rand::random::<f32>() - 0.5) * 4.0,
        );

        let shard_pos = position + Vec3::new(
            (rand::random::<f32>() - 0.5) * 0.3,
            1.0 + rand::random::<f32>() * 0.5,
            (rand::random::<f32>() - 0.5) * 0.3,
        );

        commands.spawn((
            Mesh3d(visuals.glass_shard_mesh.clone()),
            MeshMaterial3d(visuals.glass_material.clone()),
            Transform::from_translation(shard_pos)
                .with_rotation(random_rotation())
                .with_scale(Vec3::splat(0.5 + rand::random::<f32>() * 0.5)),
            GlassShard {
                lifetime: 2.0 + rand::random::<f32>() * 1.0,
                velocity,
            },
        ));
    }
}

/// 生成熱線火花
fn spawn_hotwire_sparks(
    commands: &mut Commands,
    visuals: &TheftVisuals,
    position: Vec3,
) {
    for _ in 0..5 {
        let spark_pos = position + Vec3::new(
            (rand::random::<f32>() - 0.5) * 0.2,
            0.8,
            (rand::random::<f32>() - 0.5) * 0.2,
        );

        commands.spawn((
            Mesh3d(visuals.glass_shard_mesh.clone()),
            MeshMaterial3d(visuals.spark_material.clone()),
            Transform::from_translation(spark_pos)
                .with_scale(Vec3::splat(0.1)),
            HotwireSpark {
                lifetime: 0.2 + rand::random::<f32>() * 0.2,
            },
        ));
    }
}

/// 檢查車輛是否可以直接進入（未上鎖）
pub fn can_enter_directly(ownership: Option<&VehicleOwnership>) -> bool {
    match ownership {
        None => true, // 無所有權組件，可直接進入
        Some(o) => !o.is_locked,
    }
}
