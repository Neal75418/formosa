//! AI 感知系統（視覺、聽覺、天氣影響）

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};

use super::{AiBehavior, AiConfig, AiPerception};
use crate::combat::Enemy;
use crate::core::{
    WeatherState, WeatherType, COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC,
    COLLISION_GROUP_VEHICLE,
};
use crate::player::Player;

/// 感知系統本地計時器（避免資源競爭）
#[derive(Default)]
pub struct PerceptionTimer(Option<Timer>);

/// AI 感知系統：檢測玩家位置
/// GTA 5 風格：60° FOV + 視線遮擋檢測 + 天氣影響
pub fn ai_perception_system(
    time: Res<Time>,
    mut local_timer: Local<PerceptionTimer>,
    config: Res<AiConfig>,
    weather: Res<WeatherState>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    mut enemy_query: Query<(Entity, &Transform, &mut AiPerception, &mut AiBehavior), With<Enemy>>,
    rapier_context: ReadRapierContext,
) {
    // 初始化本地計時器（只執行一次）
    let timer = local_timer.0.get_or_insert_with(|| {
        Timer::from_seconds(0.1, TimerMode::Repeating)
    });
    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }

    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;
    let current_time = time.elapsed_secs();

    // 取得物理世界
    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    // === GTA5 風格：天氣影響 AI 感知 ===
    let mut weather_sight_multiplier = match weather.weather_type {
        WeatherType::Clear => config.weather_clear_sight,
        WeatherType::Cloudy => config.weather_cloudy_sight,
        WeatherType::Rainy => {
            config.weather_rainy_sight_base - weather.intensity * config.weather_rainy_sight_decay
        }
        WeatherType::Foggy => {
            config.weather_foggy_sight_base - weather.intensity * config.weather_foggy_sight_decay
        }
        WeatherType::Stormy => 0.5 - weather.intensity * 0.15, // 暴風雨：視線極差
        WeatherType::Sandstorm => 0.3 - weather.intensity * 0.1, // 沙塵暴：幾乎看不見
    };
    if weather_sight_multiplier < 0.0 {
        weather_sight_multiplier = 0.0;
    }

    for (enemy_entity, transform, mut perception, mut behavior) in &mut enemy_query {
        let my_pos = transform.translation;
        let my_forward = transform.forward().as_vec3();

        // 重置感知狀態
        perception.can_see_target = false;

        // 1. 檢查距離（根據天氣調整感知範圍）- 使用 distance_squared 避免 sqrt
        let effective_sight_range = perception.sight_range * weather_sight_multiplier;
        let effective_sight_range_sq = effective_sight_range * effective_sight_range;
        if my_pos.distance_squared(player_pos) > effective_sight_range_sq {
            continue;
        }

        // 2. 檢查 FOV（60° 視野錐）
        if !perception.is_in_fov(my_pos, my_forward, player_pos) {
            // 不在視野內 - 玩家可以從背後偷襲
            continue;
        }

        // 3. 檢查視線遮擋（Raycast）
        let ray_origin = my_pos + Vec3::Y * config.eye_height;
        let ray_target = player_pos + Vec3::Y * config.player_body_height;
        let ray_dir = (ray_target - ray_origin).normalize_or_zero();
        let max_distance = ray_origin.distance(ray_target);

        // 設定碰撞過濾：排除自己，檢測靜態物體、車輛和角色
        let filter = QueryFilter::default()
            .exclude_rigid_body(enemy_entity)
            .groups(CollisionGroups::new(
                Group::ALL,
                COLLISION_GROUP_STATIC | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_CHARACTER,
            ));

        // 執行射線檢測
        let has_line_of_sight = if let Some((hit_entity, toi)) =
            rapier.cast_ray(ray_origin, ray_dir, max_distance as RapierReal, true, filter)
        {
            // 射線命中玩家自身不算遮擋
            hit_entity == player_entity
                || toi >= (max_distance * config.line_of_sight_tolerance) as RapierReal
        } else {
            // 沒有打到任何東西，視線通暢
            true
        };

        if has_line_of_sight {
            perception.can_see_target = true;
            behavior.see_target(player_entity, player_pos, current_time);
        }
    }
}
