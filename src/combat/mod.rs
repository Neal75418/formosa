//! 戰鬥系統
//!
//! 處理武器、射擊、傷害、死亡等戰鬥相關邏輯。

mod components;
mod cover;
mod damage;
mod explosives;
mod health;
mod killcam;
mod ragdoll;
mod shooting;
mod vehicle_shooting;
mod visuals;
mod weapon;

#[cfg(test)]
mod tests;

pub use components::*;
pub use cover::*;
pub use damage::*;
pub use explosives::*;
pub use health::*;
pub use killcam::*;
pub use ragdoll::*;
pub use shooting::*;
pub use vehicle_shooting::*;
pub use visuals::*;
pub use weapon::*;

use crate::core::AppState;
use bevy::prelude::*;

/// 戰鬥系統插件
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app
            // 事件 (Bevy 0.17: Message 類型)
            .add_message::<DamageEvent>()
            .add_message::<DeathEvent>()
            .add_message::<ArmorBreakEvent>()
            .add_message::<PlayerCoverEvent>()
            .add_message::<ExplosionEvent>()
            .add_message::<ThrowExplosiveEvent>()
            // 資源
            .init_resource::<CombatState>()
            .init_resource::<ShootingInput>()
            .init_resource::<RespawnState>()
            .init_resource::<RagdollTracker>()
            .init_resource::<KillCamState>()
            // 設置系統
            .add_systems(Startup, setup_combat_visuals)
            .add_systems(Startup, setup_explosive_visuals)
            // 武器模型在 PostStartup 生成，確保玩家實體已完全創建
            .add_systems(PostStartup, spawn_player_weapons)
            // 更新系統 - 第一組（武器和射擊）
            .add_systems(
                Update,
                (
                    spawn_player_weapons,
                    weapon_visibility_system,
                    weapon_visibility_init_system,
                    shooting_input_system,
                    holding_pose_system,
                    weapon_cooldown_system,
                    reload_system,
                    punch_animation_trigger_system,
                    fire_weapon_system,
                    punch_animation_update_system,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            // 更新系統 - 傷害處理（參數過多，無法使用 run_if，改在系統內部檢查暫停）
            .add_systems(Update, damage_system)
            .add_systems(Update, death_system)
            // 更新系統 - 第二組B（受傷反應）
            .add_systems(
                Update,
                (
                    hit_reaction_update_system,          // 受傷反應狀態更新
                    hit_reaction_knockback_system,       // 玩家擊退
                    enemy_hit_reaction_knockback_system, // 敵人擊退
                    hit_reaction_visual_system,          // 受傷視覺效果
                )
                    .run_if(in_state(AppState::InGame)),
            )
            // 更新系統 - 第二組B（死亡與視覺效果）
            .add_systems(
                Update,
                (
                    player_respawn_system,
                    ragdoll_update_system,
                    ragdoll_visual_system,
                    skeletal_ragdoll_update_system, // 骨骼布娃娃生命週期
                    skeletal_ragdoll_visual_system, // 骨骼布娃娃視覺淡出
                    skeletal_ragdoll_ground_clamp_system, // 防止穿透地面
                    blood_particle_update_system,
                    bleed_damage_system,                  // 流血持續傷害
                    floating_damage_number_update_system, // GTA 5 風格浮動傷害數字
                    muzzle_flash_system,
                    bullet_tracer_system,
                    impact_effect_system,
                )
                    .run_if(in_state(AppState::InGame)),
            )
            // 更新系統 - 第三組（行人受傷與護甲特效）
            .add_systems(
                Update,
                (
                    pedestrian_hit_reaction_knockback_system, // 行人受傷擊退
                    armor_break_effect_system,                // 護甲破碎特效生成
                    armor_spark_update_system,                // 護甲火花更新
                    armor_shard_update_system,                // 護甲碎片更新
                )
                    .run_if(in_state(AppState::InGame)),
            )
            // Kill Cam 系統（獨立組，需要控制時間縮放）
            .add_systems(
                Update,
                (killcam_update_system, killcam_visual_system).chain(),
            )
            // 玩家掩體系統
            .add_systems(
                Update,
                (
                    player_cover_input_system,
                    player_cover_event_system,
                    player_cover_update_system,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            // 車上射擊系統
            .add_systems(
                Update,
                (vehicle_shooting_input_system, vehicle_shooting_fire_system)
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            // 爆炸物系統
            .add_systems(
                Update,
                (
                    explosive_input_system,
                    handle_throw_event_system,
                    explosive_update_system,
                    detonate_sticky_bomb_system,
                    handle_explosion_event_system,
                    explosion_effect_update_system,
                    shockwave_effect_update_system,
                    fire_zone_update_system,
                    smoke_particle_update_system, // 煙霧粒子更新
                    fire_particle_update_system,  // 火焰粒子更新
                    smoke_emitter_update_system,  // 煙霧發射器
                    throw_preview_render_system,
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

/// 初始化戰鬥視覺效果資源
fn setup_combat_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(CombatVisuals::new(&mut meshes, &mut materials));
    commands.insert_resource(WeaponVisuals::new(&mut meshes, &mut materials));
    commands.insert_resource(BloodVisuals::new(&mut meshes, &mut materials));
    commands.insert_resource(ArmorEffectVisuals::new(&mut meshes, &mut materials));
}
