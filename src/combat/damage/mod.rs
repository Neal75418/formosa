//! 傷害系統
//!
//! 處理傷害計算、死亡邏輯、受傷反應、視覺特效。
//!
//! 子模組：
//! - `calculation` — 傷害計算與事件處理
//! - `death` — 死亡、重生、布娃娃、生命回復
//! - `reactions` — 受傷反應（擊退、視覺傾斜）
//! - `effects` — 血液粒子、浮動傷害數字、護甲特效
#![allow(dead_code)]

mod calculation;
mod death;
mod effects;
mod reactions;

// Re-exports — 保持對外 API 不變
pub use calculation::damage_system;
pub use death::{
    death_system, player_respawn_system, ragdoll_update_system, ragdoll_visual_system,
};
pub use effects::{
    armor_break_effect_system, armor_shard_update_system, armor_spark_update_system,
    blood_particle_update_system, floating_damage_number_update_system,
};
pub use reactions::{
    enemy_hit_reaction_knockback_system, hit_reaction_knockback_system,
    hit_reaction_update_system, hit_reaction_visual_system,
    pedestrian_hit_reaction_knockback_system,
};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::components::*;
use super::killcam::KillCamState;
use super::ragdoll::BodyPart;
use super::visuals::*;
use crate::audio::{AudioManager, WeaponSounds};
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::ui::{ChineseFont, DamageIndicatorState, FloatingDamageTracker, NotificationQueue};
use crate::wanted::PoliceOfficer;
use crate::world::{PLAYER_RESPAWN_Y, PLAYER_SPAWN_X, PLAYER_SPAWN_Z};

/// 傷害系統資源參數包（解決 Bevy 16 參數限制）
#[derive(SystemParam)]
pub struct DamageSystemResources<'w> {
    pub weapon_sounds: Option<Res<'w, WeaponSounds>>,
    pub audio_manager: Res<'w, AudioManager>,
    pub time: Res<'w, Time>,
    pub notifications: ResMut<'w, NotificationQueue>,
    pub combat_state: ResMut<'w, CombatState>,
    pub damage_indicator: ResMut<'w, DamageIndicatorState>,
    pub damage_tracker: ResMut<'w, FloatingDamageTracker>,
    pub font: Option<Res<'w, ChineseFont>>,
}

/// 死亡處理系統資源參數包
#[derive(SystemParam)]
pub struct DeathSystemResources<'w> {
    pub notifications: ResMut<'w, NotificationQueue>,
    pub respawn_state: ResMut<'w, RespawnState>,
    pub ragdoll_tracker: ResMut<'w, RagdollTracker>,
    pub killcam: ResMut<'w, KillCamState>,
    pub blood_visuals: Option<Res<'w, BloodVisuals>>,
    pub time: Res<'w, Time>,
}

/// 死亡處理系統查詢參數包
#[derive(SystemParam)]
pub struct DeathSystemQueries<'w, 's> {
    pub player: Query<'w, 's, (Entity, &'static Transform), With<Player>>,
    pub enemies: Query<'w, 's, (Entity, &'static Transform), (With<Enemy>, Without<Ragdoll>)>,
    pub all_enemies: Query<'w, 's, Entity, (With<Enemy>, Without<Ragdoll>)>,
    pub pedestrians: Query<
        'w,
        's,
        (Entity, &'static Transform, &'static Children),
        (
            With<Pedestrian>,
            Without<Player>,
            Without<Enemy>,
        ),
    >,
    pub police: Query<
        'w,
        's,
        &'static Transform,
        (With<PoliceOfficer>, Without<Player>, Without<Enemy>),
    >,
    pub body_parts: Query<
        'w,
        's,
        (
            &'static BodyPart,
            &'static Transform,
            &'static Mesh3d,
            &'static MeshMaterial3d<StandardMaterial>,
        ),
    >,
}

/// 玩家重生狀態
#[derive(Resource, Default)]
pub struct RespawnState {
    pub is_dead: bool,
    pub respawn_timer: f32,
    pub death_position: Vec3,
}

/// 重生位置（西門町漢中街起點）
pub const RESPAWN_POSITION: Vec3 = Vec3::new(PLAYER_SPAWN_X, PLAYER_RESPAWN_Y, PLAYER_SPAWN_Z);
