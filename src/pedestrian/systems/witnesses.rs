//! GTA5 風格行人報警系統

use bevy::prelude::*;
use rand::Rng;

use crate::combat::{WeaponInventory, WeaponType};
use crate::pedestrian::components::{
    PedState, Pedestrian, PedestrianState, WitnessState, WitnessedCrime,
};
use crate::player::Player;
use crate::wanted::{CrimeEvent, WitnessReport};

/// 目擊者 UI 顯示距離平方 (30.0²)
#[allow(dead_code)]
const WITNESS_UI_DISTANCE_SQ: f32 = 900.0;

// ============================================================================
// GTA 5 風格行人報警系統
// ============================================================================

/// 報警系統常數
mod witness_constants {
    /// 目擊視野角度（度）- 行人只能看到前方的犯罪
    pub const WITNESS_FOV_DEGREES: f32 = 120.0;
    /// 報警時的逃跑機率（部分行人會選擇逃跑而不是報警）
    pub const FLEE_INSTEAD_OF_CALL_CHANCE: f32 = 0.4;
    /// 報警基礎時間（秒）
    pub const BASE_CALL_DURATION: f32 = 3.0;
    /// 玩家靠近時報警中斷距離
    pub const INTIMIDATION_DISTANCE: f32 = 5.0;
    /// 玩家持槍時的恐嚇距離（更遠）
    pub const ARMED_INTIMIDATION_DISTANCE: f32 = 10.0;
}

// ============================================================================
// 目擊系統輔助函數
// ============================================================================
/// 將犯罪事件轉換為目擊類型
fn crime_event_to_witnessed_crime(crime: &CrimeEvent) -> WitnessedCrime {
    match crime {
        CrimeEvent::Shooting { .. } => WitnessedCrime::Gunshot,
        CrimeEvent::Assault { .. } => WitnessedCrime::Assault,
        CrimeEvent::Murder { .. } => WitnessedCrime::Murder,
        CrimeEvent::VehicleTheft { .. } => WitnessedCrime::VehicleTheft,
        CrimeEvent::VehicleHit { .. } => WitnessedCrime::VehicleHit,
        CrimeEvent::PoliceKilled { .. } => WitnessedCrime::Murder,
    }
}

/// 檢查行人是否能目擊犯罪
fn can_witness_crime(
    ped_transform: &Transform,
    crime_pos: Vec3,
    witness_range_sq: f32,
    fov_cos: f32,
    witnessed_crime: WitnessedCrime,
) -> bool {
    let ped_pos = ped_transform.translation;
    let distance_sq = ped_pos.distance_squared(crime_pos);

    if distance_sq > witness_range_sq {
        return false;
    }

    // 槍聲是聽覺，不需要視野檢查
    if witnessed_crime == WitnessedCrime::Gunshot {
        return true;
    }

    let to_crime = (crime_pos - ped_pos).normalize_or_zero();
    let forward = ped_transform.forward().as_vec3();
    forward.dot(to_crime) >= fov_cos
}

/// 處理行人對犯罪的反應
fn apply_witness_reaction(
    state: &mut PedestrianState,
    witness: &mut WitnessState,
    witnessed_crime: WitnessedCrime,
    crime_pos: Vec3,
    player_pos: Vec3,
    flee_chance: f32,
    base_call_duration: f32,
) {
    let mut rng = rand::rng();

    if rng.random::<f32>() < flee_chance {
        state.state = PedState::Fleeing;
        state.flee_timer = 10.0;
        state.fear_level = 1.0;
        state.last_threat_pos = Some(player_pos);
    } else {
        witness.witness_crime(witnessed_crime, crime_pos);
        state.state = PedState::CallingPolice;
        state.fear_level = 0.8;
        state.last_threat_pos = Some(player_pos);
        witness.call_duration = base_call_duration / witnessed_crime.severity();
    }
}

/// 行人目擊犯罪偵測系統
/// 當玩家犯罪時，通知範圍內的行人
pub fn witness_crime_detection_system(
    _time: Res<Time>,
    mut crime_events: MessageReader<CrimeEvent>,
    player_query: Query<&Transform, With<Player>>,
    mut ped_query: Query<(&Transform, &mut PedestrianState, &mut WitnessState), With<Pedestrian>>,
) {
    use witness_constants::*;

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let fov_cos = (WITNESS_FOV_DEGREES / 2.0).to_radians().cos();

    for crime in crime_events.read() {
        let crime_pos = crime.position();
        let witnessed_crime = crime_event_to_witnessed_crime(crime);
        let witness_range_sq = witnessed_crime.witness_range().powi(2);
        let flee_chance = FLEE_INSTEAD_OF_CALL_CHANCE * witnessed_crime.severity();

        for (ped_transform, mut state, mut witness) in ped_query.iter_mut() {
            // 跳過已經在逃跑或報警的行人
            if state.state == PedState::Fleeing || state.state == PedState::CallingPolice {
                continue;
            }

            if !can_witness_crime(
                ped_transform,
                crime_pos,
                witness_range_sq,
                fov_cos,
                witnessed_crime,
            ) {
                continue;
            }

            apply_witness_reaction(
                &mut state,
                &mut witness,
                witnessed_crime,
                crime_pos,
                player_pos,
                flee_chance,
                BASE_CALL_DURATION,
            );
        }
    }
}

/// 檢查玩家是否持武器
fn is_player_armed(weapon_inventory: Option<&WeaponInventory>) -> bool {
    weapon_inventory
        .and_then(|inv| inv.current_weapon())
        .map(|w| w.stats.weapon_type != WeaponType::Fist)
        .unwrap_or(false)
}

/// 獲取目擊犯罪的描述
fn get_witnessed_crime_description(crime_type: WitnessedCrime) -> &'static str {
    match crime_type {
        WitnessedCrime::Gunshot => "槍擊",
        WitnessedCrime::Assault => "攻擊",
        WitnessedCrime::Murder => "謀殺",
        WitnessedCrime::VehicleTheft => "搶車",
        WitnessedCrime::VehicleHit => "撞人",
    }
}

/// 處理被恐嚇的情況（重置報警並逃跑）
fn handle_witness_intimidation(state: &mut PedestrianState, witness: &mut WitnessState) {
    witness.reset();
    state.state = PedState::Fleeing;
    state.flee_timer = 8.0;
    state.fear_level = 1.0;
}

/// 處理報警完成
fn handle_call_completion(
    witness: &WitnessState,
    state: &mut PedestrianState,
    witness_reports: &mut MessageWriter<WitnessReport>,
) {
    if let (Some(crime_type), Some(crime_pos)) = (witness.crime_type, witness.crime_position) {
        let description = get_witnessed_crime_description(crime_type);
        witness_reports.write(WitnessReport::new(crime_pos, description));
    }
    state.state = PedState::Walking;
    state.fear_level = 0.3;
}

/// 行人報警進度系統
/// 處理報警中的行人，更新進度並在完成時發送報警事件
pub fn witness_phone_call_system(
    time: Res<Time>,
    player_query: Query<(&Transform, Option<&WeaponInventory>), With<Player>>,
    mut ped_query: Query<
        (Entity, &Transform, &mut PedestrianState, &mut WitnessState),
        With<Pedestrian>,
    >,
    mut witness_reports: MessageWriter<WitnessReport>,
) {
    use witness_constants::*;

    let dt = time.delta_secs();

    let Ok((player_transform, weapon_inventory)) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    let intimidation_dist = if is_player_armed(weapon_inventory) {
        ARMED_INTIMIDATION_DISTANCE
    } else {
        INTIMIDATION_DISTANCE
    };
    let intimidation_dist_sq = intimidation_dist * intimidation_dist;

    for (_entity, ped_transform, mut state, mut witness) in ped_query.iter_mut() {
        // 只處理正在報警的行人
        if state.state != PedState::CallingPolice {
            witness.tick(dt);
            continue;
        }

        let dist_to_player_sq = ped_transform.translation.distance_squared(player_pos);

        // 玩家靠近時被恐嚇，中斷報警並逃跑
        if dist_to_player_sq < intimidation_dist_sq {
            handle_witness_intimidation(&mut state, &mut witness);
            continue;
        }

        // 更新報警進度
        if witness.tick(dt) {
            handle_call_completion(&witness, &mut state, &mut witness_reports);
        }
    }
}

/// 報警 UI 標記組件
#[derive(Component)]
pub struct WitnessPhoneIcon {
    pub owner: Entity,
}

/// 行人報警視覺效果系統
/// 在報警中的行人頭上顯示手機圖標和進度條
pub fn witness_visual_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    ped_query: Query<(Entity, &Transform, &PedestrianState, &WitnessState), With<Pedestrian>>,
    existing_icons: Query<(Entity, &WitnessPhoneIcon)>,
) {
    // 移除不再需要的圖標
    for (icon_entity, icon) in existing_icons.iter() {
        let should_remove = ped_query
            .get(icon.owner)
            .map(|(_, _, state, _)| state.state != PedState::CallingPolice)
            .unwrap_or(true);

        if should_remove {
            commands.entity(icon_entity).despawn();
        }
    }

    // 為報警中的行人添加圖標
    for (ped_entity, transform, state, _witness) in ped_query.iter() {
        if state.state != PedState::CallingPolice {
            continue;
        }

        // 檢查是否已有圖標
        let has_icon = existing_icons
            .iter()
            .any(|(_, icon)| icon.owner == ped_entity);
        if has_icon {
            continue;
        }

        // 在行人頭上生成手機圖標（使用簡單的方塊表示）
        let icon_pos = transform.translation + Vec3::new(0.0, 2.2, 0.0);

        // 手機圖標（藍色小方塊）
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.15, 0.25, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.5, 1.0),
                emissive: LinearRgba::rgb(0.0, 0.3, 1.0),
                ..default()
            })),
            Transform::from_translation(icon_pos),
            WitnessPhoneIcon { owner: ped_entity },
        ));
    }
}

/// 報警圖標跟隨系統
/// 讓圖標跟隨行人移動並顯示進度
pub fn witness_icon_follow_system(
    time: Res<Time>,
    ped_query: Query<(&Transform, &WitnessState), With<Pedestrian>>,
    mut icon_query: Query<(&WitnessPhoneIcon, &mut Transform), Without<Pedestrian>>,
) {
    let elapsed = time.elapsed_secs();

    for (icon, mut icon_transform) in icon_query.iter_mut() {
        if let Ok((ped_transform, witness)) = ped_query.get(icon.owner) {
            // 跟隨行人
            let target_pos = ped_transform.translation + Vec3::new(0.0, 2.2, 0.0);
            icon_transform.translation = target_pos;

            // 旋轉動畫（模擬打電話）
            let wobble = (elapsed * 8.0).sin() * 0.1;
            icon_transform.rotation = Quat::from_rotation_z(wobble);

            // 根據報警進度縮放（越接近完成越大）
            let scale = 1.0 + witness.call_progress * 0.5;
            icon_transform.scale = Vec3::splat(scale);
        }
    }
}

/// 報警進度條系統
/// 在 UI 上顯示附近報警中行人的進度
#[allow(dead_code)]
pub fn witness_progress_ui_system(
    player_query: Query<&Transform, With<Player>>,
    ped_query: Query<(&Transform, &WitnessState), (With<Pedestrian>, Changed<WitnessState>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 找出最近的報警中行人
    let mut _nearest_witness: Option<(&Transform, &WitnessState, f32)> = None;

    for (ped_transform, witness) in ped_query.iter() {
        if !witness.witnessed_crime {
            continue;
        }

        // 使用 distance_squared 避免 sqrt
        let dist_sq = ped_transform.translation.distance_squared(player_pos);
        if dist_sq < WITNESS_UI_DISTANCE_SQ {
            let is_closer = _nearest_witness.is_none_or(|(_, _, d)| dist_sq < d);
            if is_closer {
                _nearest_witness = Some((ped_transform, witness, dist_sq));
            }
        }
    }

    // UI 渲染在 ui 模組中處理，這裡只做資料準備
    // 實際的 UI 會在 ui/systems.rs 中讀取 WitnessState 並渲染進度條
}
