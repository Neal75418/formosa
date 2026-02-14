//! 行人日常行為系統（逛街、看手機、拍照、躲雨）

use bevy::prelude::*;
use rand::Rng;

use crate::core::{WeatherState, WeatherType};
use crate::pedestrian::behavior::{
    BehaviorType, DailyBehavior, PointOfInterestType, PointsOfInterest, ShelterSeeker,
};
use crate::pedestrian::components::{PedState, Pedestrian, PedestrianState};
use crate::pedestrian::pathfinding::AStarPath;

// ============================================================================
// 躲雨行為常數
// ============================================================================

/// 躲雨機率係數（雨量強度 * 此值 = 每幀躲雨機率）
const SHELTER_SEEK_PROBABILITY_FACTOR: f32 = 0.02;
/// 庇護點搜索半徑
const SHELTER_SEARCH_RADIUS: f32 = 80.0;
/// 商店櫥窗搜索半徑（備用庇護）
const SHOP_FALLBACK_SEARCH_RADIUS: f32 = 50.0;
/// 庇護點到達距離平方 (2.0²)
const SHELTER_ARRIVAL_SQ: f32 = 4.0;

// ============================================================================
// 日常行為系統
// ============================================================================

/// 日常行為初始化系統（為新生成的行人添加 DailyBehavior 和 AStarPath）
pub fn daily_behavior_init_system(
    mut commands: Commands,
    pois: Option<Res<PointsOfInterest>>,
    new_peds: Query<(Entity, &Transform), (With<Pedestrian>, Without<DailyBehavior>)>,
) {
    let mut rng = rand::rng();

    for (entity, transform) in new_peds.iter() {
        // 添加日常行為組件
        commands.entity(entity).insert(DailyBehavior::default());

        // 50% 機率使用 A* 尋路（更智能的行人）
        if rng.random_bool(0.5) {
            // 選擇隨機目標點
            let goal = if let Some(ref pois) = pois {
                // 嘗試找一個興趣點作為目標
                let roll: f32 = rng.random();
                if roll < 0.3 {
                    pois.find_nearest(transform.translation, PointOfInterestType::ShopWindow, 50.0)
                } else if roll < 0.5 {
                    pois.find_nearest(transform.translation, PointOfInterestType::Bench, 50.0)
                } else if roll < 0.7 {
                    pois.find_nearest(transform.translation, PointOfInterestType::PhotoSpot, 50.0)
                } else {
                    None
                }
            } else {
                None
            };

            // 如果找到興趣點，設為目標；否則使用隨機位置
            let target = goal.unwrap_or_else(|| {
                Vec3::new(
                    rng.random_range(-30.0..30.0),
                    0.25,
                    rng.random_range(-30.0..30.0),
                )
            });

            commands.entity(entity).insert(AStarPath {
                waypoints: Vec::new(),
                current_index: 0,
                goal: target,
                needs_recalc: true,
                recalc_cooldown: 0.0,
            });
        }
    }
}

// ============================================================================
// 日常行為輔助函數
// ============================================================================
/// 處理逃跑中的行人（釋放庇護點）
fn handle_fleeing_state(
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    pois: &mut PointsOfInterest,
) {
    behavior.behavior = BehaviorType::Walking;
    if shelter_seeker.is_sheltered {
        if let Some(target) = shelter_seeker.target_shelter {
            pois.release_shelter(target);
        }
        *shelter_seeker = ShelterSeeker::default();
    }
}

/// 檢查是否到達庇護點並處理
fn handle_shelter_arrival(
    pos: Vec3,
    shelter_seeker: &mut ShelterSeeker,
    astar_path: Option<&mut AStarPath>,
    pois: &mut PointsOfInterest,
    current_time: f32,
) {
    if shelter_seeker.is_sheltered {
        return;
    }

    let Some(target) = shelter_seeker.target_shelter else {
        return;
    };
    let dist_sq = pos.distance_squared(target);

    if dist_sq >= SHELTER_ARRIVAL_SQ {
        return;
    }

    if pois.occupy_shelter(target) {
        shelter_seeker.arrive_at_shelter(current_time);
    } else if let Some(new_shelter) = pois.find_nearest_shelter(pos, SHELTER_SEARCH_RADIUS) {
        shelter_seeker.target_shelter = Some(new_shelter);
        if let Some(path) = astar_path {
            path.goal = new_shelter;
            path.needs_recalc = true;
        }
    }
}

/// 處理雨停的情況
fn handle_rain_stopped(
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    pois: &mut PointsOfInterest,
    rng: &mut impl Rng,
) {
    if let Some(target) = shelter_seeker.target_shelter {
        if shelter_seeker.is_sheltered {
            pois.release_shelter(target);
        }
    }
    behavior.behavior = shelter_seeker.previous_behavior;
    behavior.timer = 0.0;
    behavior.duration = rng.random_range(5.0..15.0);
    *shelter_seeker = ShelterSeeker::default();
}

/// 嘗試開始尋找庇護點
fn try_start_shelter_seeking(
    pos: Vec3,
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    astar_path: Option<&mut AStarPath>,
    pois: &PointsOfInterest,
) -> bool {
    let shelter_target = pois
        .find_nearest_shelter(pos, SHELTER_SEARCH_RADIUS)
        .or_else(|| {
            pois.find_nearest(
                pos,
                PointOfInterestType::ShopWindow,
                SHOP_FALLBACK_SEARCH_RADIUS,
            )
        });

    let Some(target) = shelter_target else {
        return false;
    };

    shelter_seeker.start_seeking(target, behavior.behavior);
    behavior.behavior = BehaviorType::SeekingShelter;
    behavior.duration = 120.0;
    behavior.timer = 0.0;

    if let Some(path) = astar_path {
        path.goal = target;
        path.needs_recalc = true;
    }
    true
}

/// 選擇雨天行為
fn select_rainy_behavior(rng: &mut impl Rng) -> BehaviorType {
    let roll: f32 = rng.random();
    if roll < 0.6 {
        BehaviorType::SeekingShelter
    } else if roll < 0.8 {
        BehaviorType::PhoneWatching
    } else {
        BehaviorType::Resting
    }
}

/// 根據新行為更新 A* 路徑
fn update_path_for_new_behavior(
    pos: Vec3,
    new_behavior: BehaviorType,
    path: &mut AStarPath,
    pois: &PointsOfInterest,
) {
    let poi_target = match new_behavior {
        BehaviorType::WindowShopping => {
            pois.find_nearest(pos, PointOfInterestType::ShopWindow, 20.0)
        }
        BehaviorType::Resting => pois.find_nearest(pos, PointOfInterestType::Bench, 30.0),
        BehaviorType::TakingPhoto => pois.find_nearest(pos, PointOfInterestType::PhotoSpot, 40.0),
        BehaviorType::SeekingShelter => {
            pois.find_nearest(pos, PointOfInterestType::Shelter, SHELTER_SEARCH_RADIUS)
        }
        _ => None,
    };

    if let Some(target) = poi_target {
        path.goal = target;
        path.needs_recalc = true;
    }
}

/// 處理單一行人的躲雨行為
fn process_shelter_behavior(
    pos: Vec3,
    is_raining: bool,
    current_time: f32,
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    astar_path: Option<&mut AStarPath>,
    pois: &mut PointsOfInterest,
    rng: &mut impl Rng,
) -> bool {
    handle_shelter_arrival(pos, shelter_seeker, astar_path, pois, current_time);
    if !is_raining {
        handle_rain_stopped(behavior, shelter_seeker, pois, rng);
    }
    true // 表示已處理，主迴圈應 continue
}

/// 嘗試在雨中開始躲雨
fn try_rain_shelter(
    pos: Vec3,
    rain_intensity: f32,
    behavior: &mut DailyBehavior,
    shelter_seeker: &mut ShelterSeeker,
    astar_path: Option<&mut AStarPath>,
    pois: &PointsOfInterest,
    rng: &mut impl Rng,
) -> bool {
    let shelter_chance = rain_intensity * SHELTER_SEEK_PROBABILITY_FACTOR;
    if rng.random::<f32>() >= shelter_chance {
        return false;
    }
    try_start_shelter_seeking(pos, behavior, shelter_seeker, astar_path, pois)
}

/// 更新行為計時並檢查是否需要切換
fn update_behavior_timer(
    dt: f32,
    is_raining: bool,
    pos: Vec3,
    behavior: &mut DailyBehavior,
    astar_path: Option<&mut AStarPath>,
    pois: &PointsOfInterest,
    rng: &mut impl Rng,
) {
    behavior.timer += dt;
    if behavior.timer < behavior.duration {
        return;
    }

    let new_behavior = if is_raining {
        select_rainy_behavior(rng)
    } else {
        select_next_behavior(rng, pos, pois)
    };

    let (min_dur, max_dur) = new_behavior.duration_range();
    behavior.behavior = new_behavior;
    behavior.duration = rng.random_range(min_dur..max_dur);
    behavior.timer = 0.0;

    if let Some(path) = astar_path {
        update_path_for_new_behavior(pos, new_behavior, path, pois);
    }
}

/// 日常行為更新系統（包含天氣反應）
pub fn daily_behavior_update_system(
    time: Res<Time>,
    mut pois: Option<ResMut<PointsOfInterest>>,
    weather: Res<WeatherState>,
    mut ped_query: Query<
        (
            Entity,
            &Transform,
            &PedestrianState,
            &mut DailyBehavior,
            &mut ShelterSeeker,
            Option<&mut AStarPath>,
        ),
        With<Pedestrian>,
    >,
) {
    let dt = time.delta_secs();
    let current_time = time.elapsed_secs();
    let mut rng = rand::rng();

    let Some(ref mut pois) = pois else { return };

    let is_raining = weather.weather_type == WeatherType::Rainy;
    let rain_intensity = if is_raining { weather.intensity } else { 0.0 };

    for (_entity, transform, state, mut behavior, mut shelter_seeker, mut astar_path) in
        ped_query.iter_mut()
    {
        let pos = transform.translation;

        if state.state == PedState::Fleeing {
            handle_fleeing_state(&mut behavior, &mut shelter_seeker, pois);
            continue;
        }

        if behavior.behavior == BehaviorType::SeekingShelter {
            process_shelter_behavior(
                pos,
                is_raining,
                current_time,
                &mut behavior,
                &mut shelter_seeker,
                astar_path.as_deref_mut(),
                pois,
                &mut rng,
            );
            continue;
        }

        if is_raining
            && try_rain_shelter(
                pos,
                rain_intensity,
                &mut behavior,
                &mut shelter_seeker,
                astar_path.as_deref_mut(),
                pois,
                &mut rng,
            )
        {
            continue;
        }

        update_behavior_timer(
            dt,
            is_raining,
            pos,
            &mut behavior,
            astar_path.as_deref_mut(),
            pois,
            &mut rng,
        );
    }
}

/// 選擇下一個行為
fn select_next_behavior(rng: &mut impl Rng, pos: Vec3, pois: &PointsOfInterest) -> BehaviorType {
    // 根據附近興趣點調整機率
    let has_shop_nearby = pois
        .find_nearest(pos, PointOfInterestType::ShopWindow, 15.0)
        .is_some();
    let has_bench_nearby = pois
        .find_nearest(pos, PointOfInterestType::Bench, 20.0)
        .is_some();
    let has_photo_spot = pois
        .find_nearest(pos, PointOfInterestType::PhotoSpot, 30.0)
        .is_some();

    let in_busy_area = pos.x.abs() <= 30.0 && pos.z.abs() <= 30.0;
    if !in_busy_area && !has_shop_nearby && !has_bench_nearby && !has_photo_spot {
        return BehaviorType::Walking;
    }

    let roll: f32 = rng.random();

    // 行為機率分配
    if roll < 0.40 {
        BehaviorType::Walking
    } else if roll < 0.55 {
        BehaviorType::PhoneWatching
    } else if roll < 0.70 && has_shop_nearby {
        BehaviorType::WindowShopping
    } else if roll < 0.80 && has_bench_nearby {
        BehaviorType::Resting
    } else if roll < 0.90 && has_photo_spot {
        BehaviorType::TakingPhoto
    } else if roll < 0.95 {
        BehaviorType::Chatting
    } else {
        BehaviorType::Walking
    }
}
