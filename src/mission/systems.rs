//! 任務系統


use bevy::prelude::*;
use super::{
    MissionManager, MissionMarker, MissionStatus, ActiveMission, DeliveryRating,
    MissionType, MissionData, RaceMedal, TaxiRating,
};
use crate::player::Player;
use crate::core::InteractionState;
use crate::economy::PlayerWallet;
use crate::ui::NotificationQueue;
use crate::vehicle::Vehicle;

// ============================================================================
// 任務標記顏色常數
// ============================================================================
const MARKER_COLOR_START: Color = Color::srgba(0.9, 0.8, 0.2, 0.7);  // 黃色 - 起點/取餐點
const MARKER_COLOR_END: Color = Color::srgba(0.2, 0.8, 0.2, 0.7);    // 綠色 - 終點/送達點

// ============================================================================
// 任務互動距離常數 (使用平方距離優化)
// ============================================================================
const DELIVERY_INTERACT_DIST_SQ: f32 = 64.0;  // 8.0 * 8.0
const MISSION_INTERACT_DIST_SQ: f32 = 25.0;   // 5.0 * 5.0
const VEHICLE_INTERACT_DIST_SQ: f32 = 9.0;    // 3.0 * 3.0
const CHECKPOINT_INTERACT_DIST_SQ: f32 = 64.0; // 8.0 * 8.0
const TAXI_INTERACT_DIST_SQ: f32 = 36.0;      // 6.0 * 6.0

/// 生成任務標記的輔助函數
fn spawn_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    mission_id: u32,
    is_start: bool,
) {
    let color = if is_start { MARKER_COLOR_START } else { MARKER_COLOR_END };
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(2.0, 0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_translation(position + Vec3::new(0.0, 0.3, 0.0)),
        GlobalTransform::default(),
        MissionMarker { mission_id, is_start },
    ));
}

/// 生成任務標記（Startup 系統）
pub fn spawn_mission_markers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mission_manager: Res<MissionManager>,
) {
    for mission in &mission_manager.available_missions {
        spawn_marker(
            &mut commands,
            &mut meshes,
            &mut materials,
            mission.start_pos,
            mission.id,
            true,
        );
    }
}

/// 清除指定任務的標記
fn cleanup_mission_markers(
    commands: &mut Commands,
    marker_query: &Query<(Entity, &MissionMarker)>,
    mission_id: u32,
) {
    for (entity, marker) in marker_query {
        if marker.mission_id == mission_id {
            commands.entity(entity).despawn();
        }
    }
}

/// 任務結果
enum MissionResult {
    None,
    Completed(DeliveryRating),
    RaceCompleted(RaceMedal, f32),  // (獎章, 完成時間)
    TaxiCompleted(TaxiRating),
    Failed,
}

/// 更新進行中的任務狀態
#[allow(clippy::too_many_arguments)]
fn update_active_mission(
    active: &mut ActiveMission,
    player_pos: Vec3,
    player_in_vehicle: bool,
    interaction: &mut InteractionState,
    time_delta: f32,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    notifications: &mut NotificationQueue,
) -> MissionResult {
    active.time_elapsed += time_delta;

    // 檢查超時（競速任務例外，用獎章判定）
    if let Some(limit) = active.data.time_limit {
        if active.time_elapsed > limit + 10.0 && active.data.mission_type != MissionType::Race {
            notifications.error("❌ 任務失敗：超時太久！");
            return MissionResult::Failed;
        }
    }

    // 根據任務類型分派處理
    match active.data.mission_type {
        MissionType::Delivery => {
            update_delivery_mission(active, player_pos, interaction, commands, meshes, materials, notifications)
        }
        MissionType::Race => {
            update_race_mission(active, player_pos, player_in_vehicle, commands, meshes, materials, notifications)
        }
        MissionType::Taxi => {
            update_taxi_mission(active, player_pos, player_in_vehicle, interaction, time_delta, commands, meshes, materials, notifications)
        }
        MissionType::Explore => {
            // 探索任務：直接檢查終點 (使用 distance_squared 優化)
            let distance_sq = player_pos.distance_squared(active.data.end_pos);
            if distance_sq < MISSION_INTERACT_DIST_SQ {
                MissionResult::Completed(DeliveryRating::ThreeStars)
            } else {
                MissionResult::None
            }
        }
    }
}

/// 更新外送任務狀態
fn update_delivery_mission(
    active: &mut ActiveMission,
    player_pos: Vec3,
    interaction: &mut InteractionState,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    notifications: &mut NotificationQueue,
) -> MissionResult {
    if !active.picked_up {
        // 檢查是否到達取餐點 (使用 distance_squared 優化)
        let distance_sq = player_pos.distance_squared(active.data.start_pos);
        if distance_sq < MISSION_INTERACT_DIST_SQ && interaction.can_interact() {
            active.picked_up = true;
            notifications.success("📦 已取餐！請送往目的地");

            // 生成終點標記
            spawn_marker(commands, meshes, materials, active.data.end_pos, active.data.id, false);
            interaction.consume();
        }
        return MissionResult::None;
    }

    // 已取餐，檢查是否到達送餐點 (使用 distance_squared 優化)
    let distance_sq = player_pos.distance_squared(active.data.end_pos);
    if distance_sq < MISSION_INTERACT_DIST_SQ {
        let rating = calculate_delivery_rating(active);
        return MissionResult::Completed(rating);
    }

    MissionResult::None
}

/// 計算外送評價
fn calculate_delivery_rating(active: &ActiveMission) -> DeliveryRating {
    if let Some(limit) = active.data.time_limit {
        // 確保比率不會變成負數（超時情況）
        let remaining_ratio = ((limit - active.time_elapsed) / limit).max(0.0);
        DeliveryRating::from_time_ratio(remaining_ratio)
    } else {
        DeliveryRating::ThreeStars // 無時限任務給 3 星
    }
}

// ============================================================================
// 競速任務邏輯
// ============================================================================

/// 更新競速任務狀態
#[allow(clippy::too_many_arguments)]
fn update_race_mission(
    active: &mut ActiveMission,
    player_pos: Vec3,
    player_in_vehicle: bool,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    notifications: &mut NotificationQueue,
) -> MissionResult {
    // 競速任務需要在車上
    if !player_in_vehicle {
        return MissionResult::None;
    }

    let Some(ref mut race_data) = active.data.race_data else {
        return MissionResult::None;
    };

    // 取得當前檢查點
    let Some(checkpoint) = race_data.current_checkpoint_pos() else {
        // 已完成所有檢查點
        let medal = race_data.medal_for_time(active.time_elapsed);
        return MissionResult::RaceCompleted(medal, active.time_elapsed);
    };

    // 檢查是否到達檢查點 (使用 distance_squared 優化)
    let distance_sq = player_pos.distance_squared(checkpoint);
    if distance_sq < CHECKPOINT_INTERACT_DIST_SQ {
        // 通過檢查點
        let checkpoint_num = race_data.current_checkpoint + 1;
        let total = race_data.checkpoints.len();

        if race_data.advance_checkpoint() {
            // 還有下一個檢查點
            notifications.info(format!("🏁 檢查點 {}/{}", checkpoint_num, total));

            // 生成下一個檢查點標記
            if let Some(next_cp) = race_data.current_checkpoint_pos() {
                spawn_checkpoint_marker(commands, meshes, materials, next_cp, active.data.id, race_data.current_checkpoint);
            }
        } else {
            // 完成比賽！
            let medal = race_data.medal_for_time(active.time_elapsed);
            return MissionResult::RaceCompleted(medal, active.time_elapsed);
        }
    }

    MissionResult::None
}

/// 生成檢查點標記
fn spawn_checkpoint_marker(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    mission_id: u32,
    checkpoint_index: usize,
) {
    // 使用橙色圓柱標記檢查點
    let color = Color::srgba(1.0, 0.6, 0.2, 0.8);
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(3.0, 0.3))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::new(1.0, 0.4, 0.0, 1.0),
            ..default()
        })),
        Transform::from_translation(position + Vec3::new(0.0, 0.2, 0.0)),
        GlobalTransform::default(),
        MissionMarker { mission_id, is_start: false },
        CheckpointMarker { index: checkpoint_index },
    ));
}

/// 檢查點標記組件
#[derive(Component)]
pub struct CheckpointMarker {
    pub index: usize,
}

// ============================================================================
// 計程車任務邏輯
// ============================================================================

/// 更新計程車任務狀態
#[allow(clippy::too_many_arguments)]
fn update_taxi_mission(
    active: &mut ActiveMission,
    player_pos: Vec3,
    player_in_vehicle: bool,
    interaction: &mut InteractionState,
    time_delta: f32,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    notifications: &mut NotificationQueue,
) -> MissionResult {
    let Some(ref mut taxi_data) = active.data.taxi_data else {
        return MissionResult::None;
    };

    if !taxi_data.passenger_picked_up {
        // 等待接客
        // 需要在車上才能接客
        if !player_in_vehicle {
            // 減少乘客耐心
            taxi_data.patience -= time_delta * 0.02;
            if taxi_data.patience <= 0.0 {
                notifications.error("❌ 乘客等太久，已離開！");
                return MissionResult::Failed;
            }
            return MissionResult::None;
        }

        // 檢查是否到達接客點 (使用 distance_squared 優化)
        let distance_sq = player_pos.distance_squared(active.data.start_pos);
        if distance_sq < TAXI_INTERACT_DIST_SQ && interaction.can_interact() {
            taxi_data.passenger_picked_up = true;
            notifications.success(format!("🚕 {} 已上車！前往 {}",
                taxi_data.passenger_name, taxi_data.destination_name));

            // 生成終點標記
            spawn_marker(commands, meshes, materials, active.data.end_pos, active.data.id, false);
            interaction.consume();
        }
        return MissionResult::None;
    }

    // 乘客已上車
    // 如果玩家下車，乘客不滿意
    if !player_in_vehicle {
        taxi_data.update_satisfaction(-0.3);
        notifications.warning("😒 乘客：「怎麼停下來了？」");
    }

    // 檢查是否到達目的地 (使用 distance_squared 優化)
    let distance_sq = player_pos.distance_squared(active.data.end_pos);
    if distance_sq < TAXI_INTERACT_DIST_SQ {
        let rating = taxi_data.rating();
        return MissionResult::TaxiCompleted(rating);
    }

    MissionResult::None
}

/// 重置任務標記（清除舊標記並重新生成起始點）
fn reset_mission_markers(
    mission_id: Option<u32>,
    mission_manager: &MissionManager,
    commands: &mut Commands,
    marker_query: &Query<(Entity, &MissionMarker)>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let Some(id) = mission_id else { return };

    cleanup_mission_markers(commands, marker_query, id);

    if let Some(mission_data) = mission_manager.available_missions.iter().find(|m| m.id == id) {
        info!("🔄 任務重置：{}", mission_data.title);
        spawn_marker(commands, meshes, materials, mission_data.start_pos, id, true);
    }
}

/// 處理任務完成
#[allow(clippy::too_many_arguments)]
fn handle_mission_completion(
    rating: DeliveryRating,
    mission_manager: &mut MissionManager,
    wallet: &mut PlayerWallet,
    notifications: &mut NotificationQueue,
    commands: &mut Commands,
    marker_query: &Query<(Entity, &MissionMarker)>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mission_id = mission_manager.active_mission.as_ref().map(|a| a.data.id);

    // 計算獎勵
    let final_reward = mission_manager.complete_delivery(rating);
    wallet.add_cash(final_reward as i32);

    // 顯示結果
    let streak_msg = if mission_manager.delivery_streak > 1 {
        format!(" 🔥{}連擊！", mission_manager.delivery_streak)
    } else {
        String::new()
    };
    notifications.success(format!(
        "✅ 外送完成！{} 獲得 ${}{}",
        rating.stars(), final_reward, streak_msg
    ));

    reset_mission_markers(mission_id, mission_manager, commands, marker_query, meshes, materials);
    mission_manager.active_mission = None;
}

/// 處理任務失敗
fn handle_mission_failure(
    mission_manager: &mut MissionManager,
    notifications: &mut NotificationQueue,
    commands: &mut Commands,
    marker_query: &Query<(Entity, &MissionMarker)>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mission_id = mission_manager.active_mission.as_ref().map(|a| a.data.id);

    mission_manager.fail_delivery();
    notifications.warning("💔 任務失敗！");

    reset_mission_markers(mission_id, mission_manager, commands, marker_query, meshes, materials);
    mission_manager.active_mission = None;
}

/// 處理競速任務完成
fn handle_race_completion(
    medal: RaceMedal,
    finish_time: f32,
    mission_manager: &mut MissionManager,
    wallet: &mut PlayerWallet,
    notifications: &mut NotificationQueue,
    commands: &mut Commands,
    marker_query: &Query<(Entity, &MissionMarker)>,
) {
    let mission_id = mission_manager.active_mission.as_ref().map(|a| a.data.id);

    // 計算獎勵
    let base_reward = mission_manager.active_mission.as_ref()
        .map(|m| m.data.reward)
        .unwrap_or(0);
    let final_reward = (base_reward as f32 * medal.bonus_multiplier()) as u32;

    wallet.add_cash(final_reward as i32);
    mission_manager.completed_count += 1;
    mission_manager.total_earnings += final_reward;

    // 顯示結果
    let medal_text = if medal != RaceMedal::None {
        format!("{} ", medal.emoji())
    } else {
        String::new()
    };

    notifications.success(format!(
        "🏁 競速完成！{}時間 {:.2}秒 | 獲得 ${}",
        medal_text, finish_time, final_reward
    ));

    // 清除標記
    if let Some(id) = mission_id {
        cleanup_mission_markers(commands, marker_query, id);
    }
    mission_manager.active_mission = None;
}

/// 處理計程車任務完成
fn handle_taxi_completion(
    rating: TaxiRating,
    mission_manager: &mut MissionManager,
    wallet: &mut PlayerWallet,
    notifications: &mut NotificationQueue,
    commands: &mut Commands,
    marker_query: &Query<(Entity, &MissionMarker)>,
) {
    let mission_id = mission_manager.active_mission.as_ref().map(|a| a.data.id);

    // 計算車資和小費
    let base_reward = mission_manager.active_mission.as_ref()
        .map(|m| m.data.reward)
        .unwrap_or(0);
    let tip = (base_reward as f32 * rating.tip_multiplier() * 0.3) as u32;
    let final_reward = base_reward + tip;

    wallet.add_cash(final_reward as i32);
    mission_manager.completed_count += 1;
    mission_manager.total_earnings += final_reward;

    // 顯示結果
    let tip_msg = if tip > 0 {
        format!(" (+${} 小費)", tip)
    } else {
        String::new()
    };

    notifications.success(format!(
        "🚕 載客完成！{} 獲得 ${}{}",
        rating.emoji(), final_reward, tip_msg
    ));

    // 清除標記
    if let Some(id) = mission_id {
        cleanup_mission_markers(commands, marker_query, id);
    }
    mission_manager.active_mission = None;
}

/// 尋找附近可接取的任務
/// 使用 distance_squared 優化，避免不必要的 sqrt 運算
fn find_nearby_mission(
    mission_manager: &MissionManager,
    player_pos: Vec3,
) -> Option<MissionData> {
    // 先檢查外送訂單（較大範圍）
    for order in &mission_manager.delivery_orders {
        if player_pos.distance_squared(order.start_pos) < DELIVERY_INTERACT_DIST_SQ {
            return Some(order.clone());
        }
    }

    // 再檢查傳統任務
    for mission in &mission_manager.available_missions {
        if player_pos.distance_squared(mission.start_pos) < MISSION_INTERACT_DIST_SQ {
            return Some(mission.clone());
        }
    }

    None
}

/// 接取新任務
fn accept_mission(
    mission: MissionData,
    mission_manager: &mut MissionManager,
    notifications: &mut NotificationQueue,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // 根據任務類型顯示不同資訊
    match mission.mission_type {
        MissionType::Delivery => {
            let time_msg = mission.time_limit.map_or(String::new(), |limit| format!(" ⏱️{:.0}秒", limit));
            notifications.info(format!(
                "📋 {} | ${}{} 📍先取餐",
                mission.title, mission.reward, time_msg
            ));
        }
        MissionType::Race => {
            if let Some(ref race_data) = mission.race_data {
                notifications.info(format!(
                    "🏁 {} | 金牌 {:.1}秒 / 銀牌 {:.1}秒 / 銅牌 {:.1}秒",
                    mission.title, race_data.gold_time, race_data.silver_time, race_data.bronze_time
                ));
                // 生成第一個檢查點標記
                if let Some(first_cp) = race_data.checkpoints.get(1) {
                    spawn_checkpoint_marker(commands, meshes, materials, *first_cp, mission.id, 1);
                }
            }
        }
        MissionType::Taxi => {
            if let Some(ref taxi_data) = mission.taxi_data {
                notifications.info(format!(
                    "🚕 {} | 前往 {} | ${}",
                    mission.title, taxi_data.destination_name, mission.reward
                ));
            }
        }
        MissionType::Explore => {
            let time_msg = mission.time_limit.map_or(String::new(), |limit| format!(" ⏱️{:.0}秒", limit));
            notifications.info(format!(
                "🔍 {} | ${}{}",
                mission.title, mission.reward, time_msg
            ));
            spawn_marker(commands, meshes, materials, mission.end_pos, mission.id, false);
        }
    }

    // 判斷是否需要先取物
    let needs_pickup = matches!(mission.mission_type, MissionType::Delivery | MissionType::Taxi);

    mission_manager.active_mission = Some(ActiveMission {
        data: mission,
        status: MissionStatus::Active,
        time_elapsed: 0.0,
        picked_up: !needs_pickup, // 外送和計程車需要先取物/接客
        last_rating: DeliveryRating::None,
    });
}

/// 任務邏輯（含外送評價系統）
#[allow(clippy::too_many_arguments)]
pub fn mission_system(
    mut interaction: ResMut<InteractionState>,
    mut mission_manager: ResMut<MissionManager>,
    mut wallet: ResMut<PlayerWallet>,
    mut notifications: ResMut<NotificationQueue>,
    player_query: Query<&Transform, With<Player>>,
    vehicle_query: Query<&Transform, With<Vehicle>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    marker_query: Query<(Entity, &MissionMarker)>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;

    // 檢查玩家是否在車上（簡化判斷：與任何車輛距離 < 3m）
    // 使用 distance_squared 優化避免 sqrt 運算
    let player_in_vehicle = vehicle_query.iter().any(|vt| {
        vt.translation.distance_squared(player_pos) < VEHICLE_INTERACT_DIST_SQ
    });

    // 更新進行中的任務
    if let Some(ref mut active) = mission_manager.active_mission {
        let result = update_active_mission(
            active,
            player_pos,
            player_in_vehicle,
            &mut interaction,
            time.delta_secs(),
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut notifications,
        );

        match result {
            MissionResult::Completed(rating) => {
                handle_mission_completion(
                    rating,
                    &mut mission_manager,
                    &mut wallet,
                    &mut notifications,
                    &mut commands,
                    &marker_query,
                    &mut meshes,
                    &mut materials,
                );
                return;
            }
            MissionResult::RaceCompleted(medal, finish_time) => {
                handle_race_completion(
                    medal,
                    finish_time,
                    &mut mission_manager,
                    &mut wallet,
                    &mut notifications,
                    &mut commands,
                    &marker_query,
                );
                return;
            }
            MissionResult::TaxiCompleted(rating) => {
                handle_taxi_completion(
                    rating,
                    &mut mission_manager,
                    &mut wallet,
                    &mut notifications,
                    &mut commands,
                    &marker_query,
                );
                return;
            }
            MissionResult::Failed => {
                handle_mission_failure(
                    &mut mission_manager,
                    &mut notifications,
                    &mut commands,
                    &marker_query,
                    &mut meshes,
                    &mut materials,
                );
                return;
            }
            MissionResult::None => {}
        }
        return; // 有進行中任務時不接新任務
    }

    // 接取新任務
    if interaction.can_interact() {
        if let Some(mission) = find_nearby_mission(&mission_manager, player_pos) {
            accept_mission(
                mission,
                &mut mission_manager,
                &mut notifications,
                &mut commands,
                &mut meshes,
                &mut materials,
            );
            interaction.consume();
        }
    }
}

/// 任務標記動畫
pub fn mission_marker_animation(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<MissionMarker>>,
) {
    for mut transform in query.iter_mut() {
        let t = time.elapsed_secs();
        transform.translation.y = 0.3 + (t * 2.0).sin() * 0.3;
        transform.rotate_y(time.delta_secs() * 1.5);
    }
}
