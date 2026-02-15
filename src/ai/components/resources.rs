//! AI 計時器與視覺資源

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use crate::combat::EnemyType;

/// AI 更新計時器（降低 CPU 負載）
#[derive(Resource)]
pub struct AiUpdateTimer {
    pub perception_timer: Timer,  // 感知更新
    pub decision_timer: Timer,    // 決策更新
}

impl Default for AiUpdateTimer {
    fn default() -> Self {
        Self {
            perception_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            decision_timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }
}

/// 敵人生成計時器
#[derive(Resource)]
pub struct EnemySpawnTimer {
    pub timer: Timer,
    pub max_enemies: usize,
    pub spawn_radius: f32,
}

impl Default for EnemySpawnTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(5.0, TimerMode::Repeating),
            max_enemies: 10,
            spawn_radius: 70.0,  // 增加到 70m，配合最小生成距離 45m
        }
    }
}

// ============================================================================
// 敵人共享視覺資源
// ============================================================================

/// 髮型類型
#[derive(Clone, Copy)]
pub enum HairStyle {
    ShortSpiky,  // 小混混：短刺頭
    Bald,        // 打手：光頭
    SlickedBack, // Boss：油頭後梳
}

/// 敵人類型外觀（每種敵人類型的材質 + 髮型配置）
pub struct EnemyTypeAppearance {
    pub skin: Handle<StandardMaterial>,
    pub shirt: Handle<StandardMaterial>,
    pub pants: Handle<StandardMaterial>,
    pub shoes: Handle<StandardMaterial>,
    pub hair: Handle<StandardMaterial>,
    pub hair_style: HairStyle,
    pub has_beard: bool,
}

/// 敵人共享視覺資源（Startup 預建，避免每次 spawn 分配 ~59 個 GPU 資源）
///
/// 設計仿照 `PedestrianVisuals`：所有 mesh/material 在啟動時建立一次，
/// spawn 時僅做 `Handle::clone()`（Arc clone，近乎零成本）。
#[derive(Resource)]
pub struct EnemyVisuals {
    // === Head ===
    pub head: Handle<Mesh>,
    pub eye_white: Handle<Mesh>,     // Sphere(0.018) — 也作為手腕
    pub pupil: Handle<Mesh>,
    pub brow: Handle<Mesh>,
    pub nose: Handle<Mesh>,
    pub mouth: Handle<Mesh>,
    pub ear: Handle<Mesh>,
    // === Hair ===
    pub spike_hair: Handle<Mesh>,
    pub bald_shadow: Handle<Mesh>,
    pub beard_mesh: Handle<Mesh>,
    pub slicked_hair: Handle<Mesh>,
    pub slicked_side: Handle<Mesh>,
    // === Body ===
    pub neck: Handle<Mesh>,
    pub chest: Handle<Mesh>,
    pub waist_mesh: Handle<Mesh>,
    pub hip_body: Handle<Mesh>,
    // === Arm ===
    pub joint_medium: Handle<Mesh>,  // Sphere(0.038) — 肩 & 膝共用
    pub upper_arm: Handle<Mesh>,
    pub elbow: Handle<Mesh>,
    pub forearm: Handle<Mesh>,
    pub hand: Handle<Mesh>,
    pub fingers: Handle<Mesh>,
    // === Leg ===
    pub hip_joint: Handle<Mesh>,
    pub thigh: Handle<Mesh>,
    pub shin: Handle<Mesh>,
    pub ankle_toe: Handle<Mesh>,     // Sphere(0.028) — 腳踝 & 鞋頭共用
    pub foot: Handle<Mesh>,
    // === Shared materials ===
    pub eye_white_mat: Handle<StandardMaterial>,
    pub eye_iris_mat: Handle<StandardMaterial>,
    pub lip_mat: Handle<StandardMaterial>,
    // === Per-type appearance ===
    gangster: EnemyTypeAppearance,
    thug: EnemyTypeAppearance,
    boss: EnemyTypeAppearance,
    military: EnemyTypeAppearance,
}

impl EnemyVisuals {
    pub fn appearance(&self, enemy_type: EnemyType) -> &EnemyTypeAppearance {
        match enemy_type {
            EnemyType::Gangster => &self.gangster,
            EnemyType::Thug => &self.thug,
            EnemyType::Boss => &self.boss,
            EnemyType::Military => &self.military,
        }
    }

    pub fn new(
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        // Head
        let head = meshes.add(Sphere::new(0.1));
        let eye_white = meshes.add(Sphere::new(0.018));
        let pupil = meshes.add(Sphere::new(0.008));
        let brow = meshes.add(Cuboid::new(0.03, 0.008, 0.01));
        let nose = meshes.add(Cuboid::new(0.02, 0.035, 0.025));
        let mouth = meshes.add(Cuboid::new(0.04, 0.012, 0.015));
        let ear = meshes.add(Sphere::new(0.025));
        // Hair
        let spike_hair = meshes.add(Capsule3d::new(0.015, 0.04));
        let bald_shadow = meshes.add(Sphere::new(0.101));
        let beard_mesh = meshes.add(Cuboid::new(0.06, 0.04, 0.02));
        let slicked_hair = meshes.add(Sphere::new(0.108));
        let slicked_side = meshes.add(Cuboid::new(0.21, 0.02, 0.08));
        // Body
        let neck = meshes.add(Cylinder::new(0.04, 0.08));
        let chest = meshes.add(Cuboid::new(0.28, 0.2, 0.14));
        let waist_mesh = meshes.add(Cuboid::new(0.22, 0.1, 0.12));
        let hip_body = meshes.add(Cuboid::new(0.26, 0.1, 0.14));
        // Arm
        let joint_medium = meshes.add(Sphere::new(0.038));
        let upper_arm = meshes.add(Capsule3d::new(0.030, 0.10));
        let elbow = meshes.add(Sphere::new(0.026));
        let forearm = meshes.add(Capsule3d::new(0.024, 0.08));
        let hand = meshes.add(Cuboid::new(0.038, 0.055, 0.018));
        let fingers = meshes.add(Cuboid::new(0.032, 0.035, 0.014));
        // Leg
        let hip_joint = meshes.add(Sphere::new(0.045));
        let thigh = meshes.add(Capsule3d::new(0.045, 0.11));
        let shin = meshes.add(Capsule3d::new(0.034, 0.10));
        let ankle_toe = meshes.add(Sphere::new(0.028));
        let foot = meshes.add(Cuboid::new(0.055, 0.035, 0.10));

        // Shared materials
        let eye_white_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.95, 0.95),
            ..default()
        });
        let eye_iris_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.15, 0.1),
            ..default()
        });
        let lip_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.45, 0.4),
            perceptual_roughness: 0.4,
            ..default()
        });

        // Per-type appearance
        let gangster = Self::create_type(
            materials,
            Color::srgb(0.87, 0.72, 0.62), Color::srgb(0.15, 0.15, 0.2),
            Color::srgb(0.2, 0.22, 0.3), Color::srgb(0.9, 0.9, 0.95),
            Color::srgb(0.15, 0.12, 0.08), HairStyle::ShortSpiky, false,
        );
        let thug = Self::create_type(
            materials,
            Color::srgb(0.75, 0.58, 0.45), Color::srgb(0.08, 0.08, 0.08),
            Color::srgb(0.25, 0.2, 0.15), Color::srgb(0.12, 0.12, 0.12),
            Color::srgb(0.1, 0.08, 0.06), HairStyle::Bald, true,
        );
        let boss = Self::create_type(
            materials,
            Color::srgb(0.82, 0.68, 0.58), Color::srgb(0.1, 0.1, 0.12),
            Color::srgb(0.08, 0.08, 0.1), Color::srgb(0.2, 0.12, 0.08),
            Color::srgb(0.05, 0.05, 0.05), HairStyle::SlickedBack, false,
        );
        let military = Self::create_type(
            materials,
            Color::srgb(0.80, 0.65, 0.50), Color::srgb(0.25, 0.30, 0.18),
            Color::srgb(0.22, 0.27, 0.15), Color::srgb(0.15, 0.12, 0.08),
            Color::srgb(0.12, 0.10, 0.06), HairStyle::Bald, false,
        );

        Self {
            head, eye_white, pupil, brow, nose, mouth, ear,
            spike_hair, bald_shadow, beard_mesh, slicked_hair, slicked_side,
            neck, chest, waist_mesh, hip_body,
            joint_medium, upper_arm, elbow, forearm, hand, fingers,
            hip_joint, thigh, shin, ankle_toe, foot,
            eye_white_mat, eye_iris_mat, lip_mat,
            gangster, thug, boss, military,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn create_type(
        materials: &mut Assets<StandardMaterial>,
        skin: Color, shirt: Color, pants: Color, shoes: Color, hair: Color,
        hair_style: HairStyle, has_beard: bool,
    ) -> EnemyTypeAppearance {
        EnemyTypeAppearance {
            skin: materials.add(StandardMaterial { base_color: skin, perceptual_roughness: 0.6, ..default() }),
            shirt: materials.add(StandardMaterial { base_color: shirt, perceptual_roughness: 0.8, ..default() }),
            pants: materials.add(StandardMaterial { base_color: pants, perceptual_roughness: 0.7, ..default() }),
            shoes: materials.add(StandardMaterial { base_color: shoes, perceptual_roughness: 0.5, ..default() }),
            hair: materials.add(StandardMaterial { base_color: hair, perceptual_roughness: 0.9, ..default() }),
            hair_style,
            has_beard,
        }
    }
}
