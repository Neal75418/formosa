//! AI 實體生命週期（生成、死亡、模型建構）

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

use super::{
    AiBehavior, AiCombat, AiConfig, AiMovement, AiPerception, CoverSeeker, EnemySpawnTimer,
    EnemyTypeAppearance, EnemyVisuals, HairStyle, SquadMember, SquadRole,
};
use crate::combat::{
    Damageable, DeathEvent, Enemy, EnemyArm, EnemyType, Health, HitReaction, Ragdoll, Weapon,
};
use crate::core::{COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE};
use crate::player::Player;

// ============================================================================
// Startup：預建敵人共享視覺資源
// ============================================================================

/// 預建敵人共享 Mesh / Material（避免每次 spawn 分配 ~59 個 GPU 資源）
pub fn setup_enemy_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(EnemyVisuals::new(&mut meshes, &mut materials));
}

// ============================================================================
// 敵人生成系統
// ============================================================================

/// 敵人生成系統
pub fn enemy_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<AiConfig>,
    mut timer: ResMut<EnemySpawnTimer>,
    visuals: Res<EnemyVisuals>,
    player_query: Query<&Transform, With<Player>>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }

    // 檢查敵人數量上限
    let current_count = enemy_query.iter().count();
    if current_count >= timer.max_enemies {
        return;
    }

    // 取得玩家位置
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    // 隨機敵人類型（先決定類型，再計算高度）
    let mut rng = rand::rng();
    let enemy_type = match rng.random_range(0..10) {
        0..=6 => EnemyType::Gangster, // 70%
        7..=8 => EnemyType::Thug,     // 20%
        _ => EnemyType::Boss,         // 10%
    };

    // 隨機生成位置（在玩家周圍，但在攻擊範圍外）
    // 最小距離 45m，確保敵人生成在攻擊範圍（30m）之外
    // 玩家需要先看到敵人，敵人才會靠近攻擊
    let angle: f32 = rng.random::<f32>() * std::f32::consts::TAU;
    let min_spawn_distance: f32 = config.min_spawn_distance;
    let distance: f32 = min_spawn_distance
        + rng.random::<f32>() * (timer.spawn_radius - min_spawn_distance).max(5.0);

    // 計算正確的生成高度（碰撞體中心高度 = half_height + radius）
    // 新的碰撞體參數：Gangster (0.45, 0.25), Thug (0.50, 0.28), Boss (0.55, 0.30)
    let spawn_height = match enemy_type {
        EnemyType::Gangster => 0.45 + 0.25,                   // 0.70
        EnemyType::Thug | EnemyType::Military => 0.50 + 0.28, // 0.78（Military 與 Thug 相同體型）
        EnemyType::Boss => 0.55 + 0.30,                       // 0.85
    };

    let spawn_pos = Vec3::new(
        player_pos.x + angle.cos() * distance,
        spawn_height,
        player_pos.z + angle.sin() * distance,
    );

    // 生成敵人（使用共享 mesh/material handles，近乎零成本）
    spawn_enemy(
        &config,
        &mut commands,
        &visuals,
        spawn_pos,
        enemy_type,
        &mut rng,
    );

    debug!(
        "生成敵人: {:?} 於 ({:.1}, {:.1})，目前數量: {}",
        enemy_type,
        spawn_pos.x,
        spawn_pos.z,
        current_count + 1
    );
}

/// 生成單個敵人（人形模型 - 有關節的完整人體）
fn spawn_enemy(
    config: &AiConfig,
    commands: &mut Commands,
    visuals: &EnemyVisuals,
    position: Vec3,
    enemy_type: EnemyType,
    rng: &mut rand::prelude::ThreadRng,
) {
    // === 根據敵人類型取得共享外觀（Handle::clone = Arc clone，近乎零成本）===
    let app = visuals.appearance(enemy_type);

    // 敵人尺寸（碰撞體）
    let (collider_half_height, collider_radius) = match enemy_type {
        EnemyType::Gangster => (0.45, 0.25),
        EnemyType::Thug | EnemyType::Military => (0.50, 0.28),
        EnemyType::Boss => (0.55, 0.30),
    };

    // 身體比例縮放因子
    let scale = match enemy_type {
        EnemyType::Gangster => 1.0,
        EnemyType::Thug | EnemyType::Military => 1.1, // 打手/軍人更壯
        EnemyType::Boss => 1.05,                      // Boss 略高
    };

    // 分批插入組件以避免 tuple 大小限制
    let entity = commands
        .spawn((
            Name::new(format!("Enemy_{enemy_type:?}")),
            Enemy,
            Damageable,
            Health::new(enemy_type.health()),
            Weapon::new(enemy_type.weapon()),
            HitReaction::default(), // 受傷反應
        ))
        .id();

    // 隨機分配小隊角色（根據敵人類型調整權重）
    let squad_role = {
        let role_roll: f32 = rng.random();
        match enemy_type {
            EnemyType::Gangster => {
                // 小混混：50% 突擊, 40% 側翼, 10% 壓制
                if role_roll < config.gangster_rusher_threshold {
                    SquadRole::Rusher
                } else if role_roll < config.gangster_flanker_threshold {
                    SquadRole::Flanker
                } else {
                    SquadRole::Suppressor
                }
            }
            EnemyType::Thug => {
                // 打手：70% 突擊, 20% 側翼, 10% 壓制
                if role_roll < config.thug_rusher_threshold {
                    SquadRole::Rusher
                } else if role_roll < config.thug_flanker_threshold {
                    SquadRole::Flanker
                } else {
                    SquadRole::Suppressor
                }
            }
            EnemyType::Boss => {
                // Boss：30% 隊長, 30% 壓制, 40% 側翼
                if role_roll < 0.3 {
                    SquadRole::Leader
                } else if role_roll < 0.6 {
                    SquadRole::Suppressor
                } else {
                    SquadRole::Flanker
                }
            }
            EnemyType::Military => {
                // 軍人：40% 壓制, 30% 突擊, 30% 側翼
                if role_roll < 0.4 {
                    SquadRole::Suppressor
                } else if role_roll < 0.7 {
                    SquadRole::Rusher
                } else {
                    SquadRole::Flanker
                }
            }
        }
    };

    // AI 組件
    commands.entity(entity).insert((
        AiBehavior::default(),
        AiPerception::default().with_range(30.0, 50.0),
        AiMovement {
            walk_speed: 3.0,
            run_speed: 6.0,
            ..default()
        },
        AiCombat {
            attack_range: enemy_type.weapon().range * 0.6,
            accuracy: match enemy_type {
                EnemyType::Gangster => 0.4,
                EnemyType::Thug => 0.55,
                EnemyType::Boss => 0.7,
                EnemyType::Military => 0.65, // 軍人精準度高
            },
            ..default()
        },
        CoverSeeker::default(),             // 掩體尋找
        SquadMember::with_role(squad_role), // 小隊角色
    ));

    // 物理和視覺
    commands.entity(entity).insert((
        Collider::capsule_y(collider_half_height, collider_radius),
        RigidBody::KinematicPositionBased, // 敵人使用運動學剛體
        KinematicCharacterController::default(),
        CollisionGroups::new(
            COLLISION_GROUP_CHARACTER,
            COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
        ), // 敵人與角色、載具、靜態物碰撞
        Transform::from_translation(position),
        GlobalTransform::default(), // 必須有此組件，否則子實體會觸發 B0004 警告
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    // 添加子實體（完整人形視覺網格 — 使用共享 Handle::clone，近乎零 GPU 成本）
    commands.entity(entity).with_children(|parent| {
        spawn_humanoid_mesh(parent, visuals, app, scale, collider_half_height);
    });
}

// ============================================================================
// 人形網格建構（使用共享 Handle::clone，近乎零 GPU 成本）
// ============================================================================

/// 生成完整人形網格（有關節）
///
/// 所有 mesh/material 來自 `EnemyVisuals`（Startup 預建），
/// spawn 時僅做 `Handle::clone()`（Arc clone），不再呼叫 `meshes.add()`。
fn spawn_humanoid_mesh(
    parent: &mut ChildSpawnerCommands,
    v: &EnemyVisuals,
    app: &EnemyTypeAppearance,
    scale: f32,
    half_height: f32,
) {
    // === 身體比例常數（以碰撞體中心為原點）===
    let head_y = half_height + 0.12 * scale;
    let neck_y = half_height - 0.02 * scale;
    let chest_y = 0.15 * scale;
    let waist_y = -0.05 * scale;
    let hip_y = -0.18 * scale;

    // === 頭部 ===
    spawn_head(parent, v, app, head_y, scale);

    // === 脖子 ===
    parent.spawn((
        Mesh3d(v.neck.clone()),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(0.0, neck_y, 0.0).with_scale(Vec3::splat(scale)),
    ));

    // === 軀幹（胸部 + 腰部 + 臀部）===
    // 胸部
    parent.spawn((
        Mesh3d(v.chest.clone()),
        MeshMaterial3d(app.shirt.clone()),
        Transform::from_xyz(0.0, chest_y, 0.0).with_scale(Vec3::splat(scale)),
    ));
    // 腰部（較窄）
    parent.spawn((
        Mesh3d(v.waist_mesh.clone()),
        MeshMaterial3d(app.shirt.clone()),
        Transform::from_xyz(0.0, waist_y, 0.0).with_scale(Vec3::splat(scale)),
    ));
    // 臀部/髖部
    parent.spawn((
        Mesh3d(v.hip_body.clone()),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(0.0, hip_y, 0.0).with_scale(Vec3::splat(scale)),
    ));

    // === 手臂（上臂 + 肘關節 + 前臂 + 手）===
    spawn_arm(parent, v, app, scale, chest_y, true); // 左臂
    spawn_arm(parent, v, app, scale, chest_y, false); // 右臂

    // === 腿部（大腿 + 膝關節 + 小腿 + 腳踝 + 腳掌）===
    spawn_leg(parent, v, app, scale, hip_y, true); // 左腿
    spawn_leg(parent, v, app, scale, hip_y, false); // 右腿
}

/// 生成頭部（含臉部細節和髮型）
#[allow(clippy::too_many_lines)]
fn spawn_head(
    parent: &mut ChildSpawnerCommands,
    v: &EnemyVisuals,
    app: &EnemyTypeAppearance,
    head_y: f32,
    scale: f32,
) {
    let head_radius = 0.1 * scale;

    // 頭部主體（略扁的球體）
    parent.spawn((
        Mesh3d(v.head.clone()),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(0.0, head_y, 0.0).with_scale(Vec3::new(
            0.95 * scale,
            1.0 * scale,
            0.9 * scale,
        )),
    ));

    // === 臉部細節 ===
    // 眼睛（左）
    let eye_y = head_y + 0.015 * scale;
    let eye_z = head_radius * 0.85;
    let eye_spacing = 0.035 * scale;

    // 眼白
    parent.spawn((
        Mesh3d(v.eye_white.clone()),
        MeshMaterial3d(v.eye_white_mat.clone()),
        Transform::from_xyz(eye_spacing, eye_y, eye_z).with_scale(Vec3::new(
            1.2 * scale,
            0.8 * scale,
            0.5 * scale,
        )),
    ));
    parent.spawn((
        Mesh3d(v.eye_white.clone()),
        MeshMaterial3d(v.eye_white_mat.clone()),
        Transform::from_xyz(-eye_spacing, eye_y, eye_z).with_scale(Vec3::new(
            1.2 * scale,
            0.8 * scale,
            0.5 * scale,
        )),
    ));

    // 瞳孔
    parent.spawn((
        Mesh3d(v.pupil.clone()),
        MeshMaterial3d(v.eye_iris_mat.clone()),
        Transform::from_xyz(eye_spacing, eye_y, eye_z + 0.008).with_scale(Vec3::splat(scale)),
    ));
    parent.spawn((
        Mesh3d(v.pupil.clone()),
        MeshMaterial3d(v.eye_iris_mat.clone()),
        Transform::from_xyz(-eye_spacing, eye_y, eye_z + 0.008).with_scale(Vec3::splat(scale)),
    ));

    // 眉毛
    parent.spawn((
        Mesh3d(v.brow.clone()),
        MeshMaterial3d(app.hair.clone()),
        Transform::from_xyz(eye_spacing, eye_y + 0.025 * scale, eye_z - 0.005)
            .with_scale(Vec3::splat(scale)),
    ));
    parent.spawn((
        Mesh3d(v.brow.clone()),
        MeshMaterial3d(app.hair.clone()),
        Transform::from_xyz(-eye_spacing, eye_y + 0.025 * scale, eye_z - 0.005)
            .with_scale(Vec3::splat(scale)),
    ));

    // 鼻子
    parent.spawn((
        Mesh3d(v.nose.clone()),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(0.0, head_y - 0.01 * scale, eye_z + 0.01)
            .with_scale(Vec3::splat(scale)),
    ));

    // 嘴巴
    parent.spawn((
        Mesh3d(v.mouth.clone()),
        MeshMaterial3d(v.lip_mat.clone()),
        Transform::from_xyz(0.0, head_y - 0.045 * scale, eye_z - 0.01)
            .with_scale(Vec3::splat(scale)),
    ));

    // 耳朵
    let ear_y = head_y;
    let ear_x = head_radius * 0.9;
    parent.spawn((
        Mesh3d(v.ear.clone()),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(ear_x, ear_y, 0.0).with_scale(Vec3::new(
            0.4 * scale,
            1.0 * scale,
            0.7 * scale,
        )),
    ));
    parent.spawn((
        Mesh3d(v.ear.clone()),
        MeshMaterial3d(app.skin.clone()),
        Transform::from_xyz(-ear_x, ear_y, 0.0).with_scale(Vec3::new(
            0.4 * scale,
            1.0 * scale,
            0.7 * scale,
        )),
    ));

    // === 髮型（根據類型變化）===
    match app.hair_style {
        HairStyle::ShortSpiky => {
            // 短刺頭：多個小尖刺
            #[allow(clippy::cast_precision_loss)]
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::TAU / 8.0;
                let spike_x = angle.cos() * head_radius * 0.6;
                let spike_z = angle.sin() * head_radius * 0.6 - 0.02;
                parent.spawn((
                    Mesh3d(v.spike_hair.clone()),
                    MeshMaterial3d(app.hair.clone()),
                    Transform::from_xyz(spike_x, head_y + head_radius * 0.7, spike_z)
                        .with_rotation(Quat::from_rotation_x(-0.3 + angle.sin() * 0.2))
                        .with_scale(Vec3::splat(scale)),
                ));
            }
        }
        HairStyle::Bald => {
            // 光頭：只有一點點陰影/刺青
            parent.spawn((
                Mesh3d(v.bald_shadow.clone()),
                MeshMaterial3d(app.hair.clone()),
                Transform::from_xyz(0.0, head_y + head_radius * 0.3, -head_radius * 0.3)
                    .with_scale(Vec3::new(0.5 * scale, 0.2 * scale, 0.5 * scale)),
            ));
            // 鬍子
            if app.has_beard {
                parent.spawn((
                    Mesh3d(v.beard_mesh.clone()),
                    MeshMaterial3d(app.hair.clone()),
                    Transform::from_xyz(0.0, head_y - 0.06 * scale, eye_z - 0.02)
                        .with_scale(Vec3::splat(scale)),
                ));
            }
        }
        HairStyle::SlickedBack => {
            // 油頭後梳：光滑的髮型
            parent.spawn((
                Mesh3d(v.slicked_hair.clone()),
                MeshMaterial3d(app.hair.clone()),
                Transform::from_xyz(0.0, head_y + head_radius * 0.15, -head_radius * 0.2)
                    .with_scale(Vec3::new(1.0 * scale, 0.5 * scale, 1.2 * scale)),
            ));
            // 側面髮際線
            parent.spawn((
                Mesh3d(v.slicked_side.clone()),
                MeshMaterial3d(app.hair.clone()),
                Transform::from_xyz(0.0, head_y + head_radius * 0.6, -head_radius * 0.3)
                    .with_scale(Vec3::splat(scale)),
            ));
        }
    }
}

/// 生成手臂（有關節，帶有 `EnemyArm` 組件以支援揮拳動畫）
/// 比例：手指到大腿中段
fn spawn_arm(
    parent: &mut ChildSpawnerCommands,
    v: &EnemyVisuals,
    app: &EnemyTypeAppearance,
    scale: f32,
    chest_y: f32,
    is_left: bool,
) {
    let side = if is_left { 1.0 } else { -1.0 };
    let shoulder_x = 0.15 * scale * side;
    let arm_tilt = 0.12 * side; // 手臂自然下垂角度

    // 肩關節位置
    let shoulder_y = chest_y + 0.06 * scale;

    // 計算手臂整體的靜止位置和旋轉
    let rest_position = Vec3::new(shoulder_x, shoulder_y, 0.0);
    let rest_rotation = Quat::from_rotation_z(arm_tilt);

    // 創建手臂根實體（帶有 EnemyArm 組件）
    parent
        .spawn((
            Transform::from_translation(rest_position).with_rotation(rest_rotation),
            GlobalTransform::default(), // 必須有此組件，否則 mesh 子實體會觸發 B0004 警告
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            if is_left {
                EnemyArm::left(rest_position, rest_rotation)
            } else {
                EnemyArm::right(rest_position, rest_rotation)
            },
            Name::new(if is_left { "LeftArm" } else { "RightArm" }),
        ))
        .with_children(|arm| {
            // 肩關節（球形）- 相對於手臂根
            arm.spawn((
                Mesh3d(v.joint_medium.clone()),
                MeshMaterial3d(app.shirt.clone()),
                Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(scale)),
            ));

            // 上臂（縮短：手指到大腿中段）
            let upper_arm_len = 0.10 * scale;
            arm.spawn((
                Mesh3d(v.upper_arm.clone()),
                MeshMaterial3d(app.shirt.clone()),
                Transform::from_xyz(0.0, -upper_arm_len, 0.0).with_scale(Vec3::splat(scale)),
            ));

            // 肘關節（球形）
            let elbow_y = -upper_arm_len * 2.0 - 0.015 * scale;
            arm.spawn((
                Mesh3d(v.elbow.clone()),
                MeshMaterial3d(app.skin.clone()),
                Transform::from_xyz(0.0, elbow_y, 0.0).with_scale(Vec3::splat(scale)),
            ));

            // 前臂（縮短）
            let forearm_len = 0.08 * scale;
            let forearm_y = elbow_y - forearm_len;
            arm.spawn((
                Mesh3d(v.forearm.clone()),
                MeshMaterial3d(app.skin.clone()),
                Transform::from_xyz(0.0, forearm_y, 0.0).with_scale(Vec3::splat(scale)),
            ));

            // 手腕（Sphere(0.018) — 與 eye_white 共用 mesh）
            let wrist_y = forearm_y - forearm_len;
            arm.spawn((
                Mesh3d(v.eye_white.clone()),
                MeshMaterial3d(app.skin.clone()),
                Transform::from_xyz(0.0, wrist_y, 0.0).with_scale(Vec3::splat(scale)),
            ));

            // 手掌
            arm.spawn((
                Mesh3d(v.hand.clone()),
                MeshMaterial3d(app.skin.clone()),
                Transform::from_xyz(0.0, wrist_y - 0.038 * scale, 0.0)
                    .with_scale(Vec3::splat(scale)),
            ));

            // 手指（簡化為一組）
            arm.spawn((
                Mesh3d(v.fingers.clone()),
                MeshMaterial3d(app.skin.clone()),
                Transform::from_xyz(0.0, wrist_y - 0.08 * scale, 0.0)
                    .with_scale(Vec3::splat(scale)),
            ));
        });
}

/// 生成腿部（有關節）
/// 比例修正：腿部總長度應在碰撞體範圍內（約 0.52 從臀部到腳底）
fn spawn_leg(
    parent: &mut ChildSpawnerCommands,
    v: &EnemyVisuals,
    app: &EnemyTypeAppearance,
    scale: f32,
    hip_y: f32,
    is_left: bool,
) {
    let side = if is_left { 1.0 } else { -1.0 };
    let hip_x = 0.07 * scale * side;

    // 髖關節（球形）
    let joint_y = hip_y - 0.03 * scale;
    parent.spawn((
        Mesh3d(v.hip_joint.clone()),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(hip_x, joint_y, 0.0).with_scale(Vec3::splat(scale)),
    ));

    // 大腿（縮短）
    let thigh_len = 0.11 * scale;
    let thigh_y = joint_y - thigh_len;
    parent.spawn((
        Mesh3d(v.thigh.clone()),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(hip_x, thigh_y, 0.0).with_scale(Vec3::splat(scale)),
    ));

    // 膝關節（Sphere(0.038) — 與肩關節共用 mesh）
    let knee_y = thigh_y - thigh_len - 0.015 * scale;
    parent.spawn((
        Mesh3d(v.joint_medium.clone()),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(hip_x, knee_y, 0.0).with_scale(Vec3::splat(scale)),
    ));

    // 小腿（縮短）
    let shin_len = 0.10 * scale;
    let shin_y = knee_y - shin_len;
    parent.spawn((
        Mesh3d(v.shin.clone()),
        MeshMaterial3d(app.pants.clone()),
        Transform::from_xyz(hip_x, shin_y, 0.0).with_scale(Vec3::splat(scale)),
    ));

    // 腳踝（Sphere(0.028) — 與鞋頭共用 mesh）
    let ankle_y = shin_y - shin_len - 0.015 * scale;
    parent.spawn((
        Mesh3d(v.ankle_toe.clone()),
        MeshMaterial3d(app.shoes.clone()),
        Transform::from_xyz(hip_x, ankle_y, 0.0).with_scale(Vec3::splat(scale)),
    ));

    // 腳掌（鞋子）
    let foot_y = ankle_y - 0.02 * scale;
    parent.spawn((
        Mesh3d(v.foot.clone()),
        MeshMaterial3d(app.shoes.clone()),
        Transform::from_xyz(hip_x, foot_y, 0.02 * scale).with_scale(Vec3::splat(scale)),
    ));

    // 鞋頭（腳趾部分，Sphere(0.028) — 與腳踝共用 mesh）
    parent.spawn((
        Mesh3d(v.ankle_toe.clone()),
        MeshMaterial3d(app.shoes.clone()),
        Transform::from_xyz(hip_x, foot_y, 0.065 * scale).with_scale(Vec3::new(
            1.0 * scale,
            0.7 * scale,
            1.2 * scale,
        )),
    ));
}

// ============================================================================
// 敵人死亡系統
// ============================================================================

/// 敵人死亡處理系統（已棄用）
///
/// 注意：此系統的功能已整合至 `combat/damage.rs::death_system`：
/// - 布娃娃效果
/// - 血液粒子
/// - 金錢掉落
///
/// 保留此函數以維持 API 相容性，但實際上不做任何處理。
/// 敵人死亡的完整流程現在由 death_system 統一處理，避免競爭條件。
#[allow(unused_variables)]
pub fn enemy_death_system(
    commands: Commands,
    death_events: MessageReader<DeathEvent>,
    enemy_query: Query<(&Transform, Entity), (With<Enemy>, Without<Ragdoll>)>,
) {
    // 此系統已棄用，所有敵人死亡處理已移至 death_system
    // 保留空實現以維持系統註冊相容性
}
