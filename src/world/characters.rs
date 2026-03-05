//! 角色生成系統
//!
//! 玩家角色和 AI 掩體點的生成

use crate::ai::CoverPoint;
use crate::combat::{
    Armor, Damageable, Health, HitReaction, PlayerArm, PlayerHand, Weapon, WeaponInventory,
    WeaponStats,
};
use crate::core::{COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE};
use crate::player::{DodgeState, Player};
use crate::world::constants::{
    X_HAN, X_KANGDING, X_XINING, X_ZHONGHUA, Z_CHENGDU, Z_EMEI, Z_WUCHANG,
};
use crate::world::PlayerInteriorState;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// 生成程序化人形角色（台灣年輕人風格）
/// 完整關節系統：肩關節、肘關節、髖關節、膝關節、腳踝
/// 身高約 1.7 公尺（遊戲單位）
#[allow(clippy::too_many_lines)]
pub fn spawn_player_character(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    player: Player,
) -> Entity {
    // 碰撞體參數
    const COLLIDER_HALF_HEIGHT: f32 = 0.45;
    const COLLIDER_RADIUS: f32 = 0.25;

    // 身體比例常數（相對於碰撞體中心）
    const HEAD_Y: f32 = 0.58;
    const NECK_Y: f32 = 0.42;
    const CHEST_Y: f32 = 0.18;
    const WAIST_Y: f32 = -0.02;
    const HIP_Y: f32 = -0.10;

    // === 材質定義 ===
    let skin_color = Color::srgb(0.96, 0.80, 0.69); // 亞洲膚色
    let hair_color = Color::srgb(0.1, 0.08, 0.05); // 深黑髮
    let shirt_color = Color::srgb(0.2, 0.5, 0.9); // 藍色 T 恤
    let pants_color = Color::srgb(0.2, 0.2, 0.25); // 深色牛仔褲
    let shoe_color = Color::srgb(0.95, 0.95, 0.95); // 白色球鞋

    let skin_mat = materials.add(StandardMaterial {
        base_color: skin_color,
        perceptual_roughness: 0.6,
        ..default()
    });
    let hair_mat = materials.add(StandardMaterial {
        base_color: hair_color,
        perceptual_roughness: 0.9,
        ..default()
    });
    let shirt_mat = materials.add(StandardMaterial {
        base_color: shirt_color,
        perceptual_roughness: 0.8,
        ..default()
    });
    let pants_mat = materials.add(StandardMaterial {
        base_color: pants_color,
        perceptual_roughness: 0.7,
        ..default()
    });
    let shoe_mat = materials.add(StandardMaterial {
        base_color: shoe_color,
        perceptual_roughness: 0.5,
        ..default()
    });
    let eye_white_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95),
        ..default()
    });
    let eye_iris_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.1, 0.05),
        ..default()
    });
    let lip_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.55, 0.55),
        perceptual_roughness: 0.4,
        ..default()
    });

    // 生成主體
    let collider_center_y = COLLIDER_HALF_HEIGHT + COLLIDER_RADIUS;
    let spawn_pos = position + Vec3::new(0.0, collider_center_y, 0.0);

    // 初始化玩家武器庫存
    let mut weapon_inventory = WeaponInventory::default();
    weapon_inventory.add_weapon(Weapon::new(WeaponStats::pistol()));
    weapon_inventory.add_weapon(Weapon::new(WeaponStats::smg()));
    weapon_inventory.add_weapon(Weapon::new(WeaponStats::shotgun()));
    weapon_inventory.add_weapon(Weapon::new(WeaponStats::rifle()));

    let player_entity = commands
        .spawn((
            Transform::from_translation(spawn_pos),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            RigidBody::KinematicPositionBased,
            Collider::capsule_y(COLLIDER_HALF_HEIGHT, COLLIDER_RADIUS),
            KinematicCharacterController {
                slide: true,
                ..default()
            },
            player,
            crate::core::ThirdPersonCameraTarget,
            Name::new("Player"),
            weapon_inventory,
            Health::new(100.0),
            Armor::default(),
            Damageable,
        ))
        .insert(DodgeState::default())
        .insert(crate::player::ClimbState::default())
        .insert(HitReaction::default())
        .insert(PlayerInteriorState::default())
        .insert(crate::combat::PlayerCoverState::default())
        .insert(crate::combat::ExplosiveInventory {
            grenades: 3,
            molotovs: 2,
            sticky_bombs: 1,
            selected: Some(crate::combat::ExplosiveType::Grenade),
            throw_cooldown: 0.0,
        })
        .insert(crate::wanted::PlayerSurrenderState::default())
        .insert(crate::vehicle::PlayerTheftState::default())
        .insert(crate::player::PlayerSprintState::default())
        .insert(crate::player::Stamina::default())
        .insert(CollisionGroups::new(
            COLLISION_GROUP_CHARACTER,
            COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
        ))
        .with_children(|parent| {
            // === 頭部 ===
            let head_radius = 0.12;
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(head_radius))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y, 0.0).with_scale(Vec3::new(0.95, 1.0, 0.9)),
            ));

            // 眼白
            let eye_y = HEAD_Y + 0.015;
            let eye_z = head_radius * 0.85;
            let eye_spacing = 0.038;
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.02))),
                MeshMaterial3d(eye_white_mat.clone()),
                Transform::from_xyz(eye_spacing, eye_y, eye_z).with_scale(Vec3::new(1.2, 0.8, 0.5)),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.02))),
                MeshMaterial3d(eye_white_mat.clone()),
                Transform::from_xyz(-eye_spacing, eye_y, eye_z)
                    .with_scale(Vec3::new(1.2, 0.8, 0.5)),
            ));

            // 瞳孔
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.009))),
                MeshMaterial3d(eye_iris_mat.clone()),
                Transform::from_xyz(eye_spacing, eye_y, eye_z + 0.01),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.009))),
                MeshMaterial3d(eye_iris_mat.clone()),
                Transform::from_xyz(-eye_spacing, eye_y, eye_z + 0.01),
            ));

            // 眉毛
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.035, 0.008, 0.012))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(eye_spacing, eye_y + 0.028, eye_z - 0.005),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.035, 0.008, 0.012))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(-eye_spacing, eye_y + 0.028, eye_z - 0.005),
            ));

            // 鼻子
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.022, 0.04, 0.028))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y - 0.01, eye_z + 0.012),
            ));

            // 嘴巴
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.045, 0.014, 0.016))),
                MeshMaterial3d(lip_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y - 0.048, eye_z - 0.01),
            ));

            // 耳朵
            let ear_x = head_radius * 0.92;
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.028))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(ear_x, HEAD_Y, 0.0).with_scale(Vec3::new(0.4, 1.0, 0.7)),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(0.028))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(-ear_x, HEAD_Y, 0.0).with_scale(Vec3::new(0.4, 1.0, 0.7)),
            ));

            // 頭髮
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(head_radius * 1.12))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y + 0.05, -0.02)
                    .with_scale(Vec3::new(1.05, 0.5, 1.15)),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(head_radius * 0.9))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y + 0.02, -0.08).with_scale(Vec3::new(0.9, 0.8, 0.6)),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.2, 0.035, 0.05))),
                MeshMaterial3d(hair_mat.clone()),
                Transform::from_xyz(0.0, HEAD_Y + head_radius * 0.85, 0.08),
            ));

            // === 脖子 ===
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.045, 0.1))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, NECK_Y, 0.0),
            ));

            // === 軀幹 ===
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.3, 0.22, 0.15))),
                MeshMaterial3d(shirt_mat.clone()),
                Transform::from_xyz(0.0, CHEST_Y, 0.0),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.24, 0.1, 0.13))),
                MeshMaterial3d(shirt_mat.clone()),
                Transform::from_xyz(0.0, WAIST_Y, 0.0),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.28, 0.1, 0.15))),
                MeshMaterial3d(pants_mat.clone()),
                Transform::from_xyz(0.0, HIP_Y, 0.0),
            ));

            // === 左手臂 ===
            spawn_arm(parent, meshes, &skin_mat, &shirt_mat, CHEST_Y, true);

            // === 右手臂 ===
            spawn_arm(parent, meshes, &skin_mat, &shirt_mat, CHEST_Y, false);

            // === 左腿 ===
            spawn_leg(parent, meshes, &pants_mat, &shoe_mat, HIP_Y, true);

            // === 右腿 ===
            spawn_leg(parent, meshes, &pants_mat, &shoe_mat, HIP_Y, false);

            // === 外送背包 ===
            let backpack_color = Color::srgb(0.1, 0.7, 0.4);
            let backpack_mat = materials.add(StandardMaterial {
                base_color: backpack_color,
                perceptual_roughness: 0.6,
                ..default()
            });
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.28, 0.32, 0.18))),
                MeshMaterial3d(backpack_mat.clone()),
                Transform::from_xyz(0.0, CHEST_Y - 0.02, -0.22),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.26, 0.05, 0.16))),
                MeshMaterial3d(backpack_mat),
                Transform::from_xyz(0.0, CHEST_Y + 0.15, -0.22),
            ));
        })
        .id();

    player_entity
}

/// 生成手臂（簡化版，完整版保留在原始 setup.rs 中）
fn spawn_arm(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut ResMut<Assets<Mesh>>,
    skin_mat: &Handle<StandardMaterial>,
    shirt_mat: &Handle<StandardMaterial>,
    chest_y: f32,
    is_left: bool,
) {
    let side = if is_left { 1.0 } else { -1.0 };
    let shoulder_pos = Vec3::new(side * 0.18, chest_y + 0.06, 0.0);
    let arm_offset = Vec3::new(side * 0.03, -0.08, 0.0);
    let arm_pos = shoulder_pos + arm_offset;
    let arm_rot = Quat::from_rotation_z(-side * 0.15);

    // 肩關節
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.045))),
        MeshMaterial3d(shirt_mat.clone()),
        Transform::from_translation(shoulder_pos),
    ));

    // 上臂
    parent
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(0.035, 0.10))),
            MeshMaterial3d(shirt_mat.clone()),
            Transform::from_translation(arm_pos).with_rotation(arm_rot),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            if is_left {
                PlayerArm::left(arm_pos, arm_rot)
            } else {
                PlayerArm::right(arm_pos, arm_rot)
            },
            Name::new(if is_left { "LeftArm" } else { "RightArm" }),
        ))
        .with_children(|arm| {
            // 肘關節
            arm.spawn((
                Mesh3d(meshes.add(Sphere::new(0.03))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, -0.12, 0.0),
                GlobalTransform::default(),
            ));
            // 前臂
            arm.spawn((
                Mesh3d(meshes.add(Capsule3d::new(0.028, 0.08))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, -0.22, 0.0),
                GlobalTransform::default(),
            ));
            // 手腕
            arm.spawn((
                Mesh3d(meshes.add(Sphere::new(0.022))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, -0.32, 0.0),
                GlobalTransform::default(),
            ));
            // 手掌
            arm.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.045, 0.055, 0.022))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, -0.36, 0.0),
                GlobalTransform::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                PlayerHand { is_right: !is_left },
                Name::new(if is_left { "LeftHand" } else { "RightHand" }),
            ));
            // 手指
            arm.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.04, 0.035, 0.018))),
                MeshMaterial3d(skin_mat.clone()),
                Transform::from_xyz(0.0, -0.40, 0.0),
                GlobalTransform::default(),
            ));
        });
}

/// 生成腿部
fn spawn_leg(
    parent: &mut ChildSpawnerCommands,
    meshes: &mut ResMut<Assets<Mesh>>,
    pants_mat: &Handle<StandardMaterial>,
    shoe_mat: &Handle<StandardMaterial>,
    hip_y: f32,
    is_left: bool,
) {
    let hip_x = if is_left { 0.08 } else { -0.08 };

    // 髖關節
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.050))),
        MeshMaterial3d(pants_mat.clone()),
        Transform::from_xyz(hip_x, hip_y - 0.04, 0.0),
    ));
    // 大腿
    parent.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.050, 0.12))),
        MeshMaterial3d(pants_mat.clone()),
        Transform::from_xyz(hip_x, hip_y - 0.16, 0.0),
    ));
    // 膝關節
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.042))),
        MeshMaterial3d(pants_mat.clone()),
        Transform::from_xyz(hip_x, hip_y - 0.30, 0.0),
    ));
    // 小腿
    parent.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.038, 0.11))),
        MeshMaterial3d(pants_mat.clone()),
        Transform::from_xyz(hip_x, hip_y - 0.42, 0.0),
    ));
    // 腳踝
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.030))),
        MeshMaterial3d(shoe_mat.clone()),
        Transform::from_xyz(hip_x, hip_y - 0.54, 0.0),
    ));
    // 腳掌
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.06, 0.04, 0.12))),
        MeshMaterial3d(shoe_mat.clone()),
        Transform::from_xyz(hip_x, hip_y - 0.57, 0.025),
    ));
    // 鞋頭
    parent.spawn((
        Mesh3d(meshes.add(Sphere::new(0.032))),
        MeshMaterial3d(shoe_mat.clone()),
        Transform::from_xyz(hip_x, hip_y - 0.57, 0.075).with_scale(Vec3::new(1.0, 0.7, 1.2)),
    ));
}

// ============================================================================
// AI 掩體點生成系統
// ============================================================================
/// 掩體點 Y 座標常數
const COVER_POINT_Y: f32 = 0.5;

/// 掩體類型枚舉
#[derive(Clone, Copy)]
enum CoverKind {
    Low,
    High,
    Full,
}

/// 批量生成掩體點
fn spawn_cover_batch(
    commands: &mut Commands,
    positions: &[(f32, f32, Vec3)],
    kind: CoverKind,
    label: &'static str,
) -> u32 {
    let mut count = 0;
    for (i, &(x, z, direction)) in positions.iter().enumerate() {
        let pos = Vec3::new(x, COVER_POINT_Y, z);
        let cover = match kind {
            CoverKind::Low => CoverPoint::low(direction),
            CoverKind::High => CoverPoint::high(direction),
            CoverKind::Full => CoverPoint::full(direction),
        };
        commands.spawn((
            Transform::from_translation(pos),
            GlobalTransform::default(),
            cover,
            Name::new(format!("{label}_{i}")),
        ));
        count += 1;
    }
    count
}

/// 在世界中策略位置生成 `CoverPoint` 實體
#[allow(clippy::similar_names)]
pub fn spawn_cover_points(commands: &mut Commands) {
    let mut cover_count = 0;

    // === 建築角落掩體點 (High Cover) ===
    let building_corners = [
        (-35.0, 8.0, Vec3::NEG_Z),
        (-35.0, -8.0, Vec3::Z),
        (-28.0, 8.0, Vec3::NEG_Z),
        (-28.0, -8.0, Vec3::Z),
        (-60.0, -15.0, Vec3::X),
        (-55.0, -15.0, Vec3::NEG_X),
        (-45.0, -20.0, Vec3::Z),
        (-45.0, -5.0, Vec3::NEG_Z),
        (8.0, -12.0, Vec3::NEG_X),
        (-8.0, -12.0, Vec3::X),
        (8.0, 15.0, Vec3::NEG_X),
        (-8.0, 15.0, Vec3::X),
        (30.0, -55.0, Vec3::Z),
        (45.0, -55.0, Vec3::Z),
        (55.0, -60.0, Vec3::X),
        (35.0, -40.0, Vec3::NEG_Z),
        (-25.0, 38.0, Vec3::NEG_Z),
        (15.0, 38.0, Vec3::NEG_Z),
        (28.0, 38.0, Vec3::NEG_Z),
        (X_KANGDING + 15.0, -60.0, Vec3::Z),
        (X_KANGDING + 15.0, -20.0, Vec3::Z),
        (X_KANGDING + 15.0, 20.0, Vec3::NEG_Z),
        (X_KANGDING + 15.0, 45.0, Vec3::NEG_Z),
        (30.0, -5.0, Vec3::Z),
    ];
    cover_count += spawn_cover_batch(
        commands,
        &building_corners,
        CoverKind::High,
        "Cover_Building",
    );

    // === 販賣機旁掩體點 (Low Cover) ===
    let vending_covers = [
        (13.0, -15.0, Vec3::NEG_X),
        (-69.0, -15.0, Vec3::X),
        (-31.0, -22.0, Vec3::X),
        (43.0, 36.0, Vec3::NEG_X),
        (-74.0, 12.0, Vec3::NEG_X),
    ];
    cover_count += spawn_cover_batch(commands, &vending_covers, CoverKind::Low, "Cover_Vending");

    // === 垃圾桶旁掩體點 (Low Cover) ===
    let trash_covers = [
        (9.0, -10.0, Vec3::NEG_X),
        (-9.0, -10.0, Vec3::X),
        (9.0, -55.0, Vec3::NEG_X),
        (-9.0, -55.0, Vec3::X),
        (-29.0, 12.0, Vec3::X),
        (31.0, 12.0, Vec3::NEG_X),
    ];
    cover_count += spawn_cover_batch(commands, &trash_covers, CoverKind::Low, "Cover_Trash");

    // === 停放車輛旁掩體點 (Low Cover) ===
    let vehicle_covers = [
        (13.0, -8.0, Vec3::NEG_X),
        (13.0, -5.0, Vec3::NEG_X),
        (X_ZHONGHUA - 29.0, Z_CHENGDU + 12.0, Vec3::Z),
        (X_ZHONGHUA - 33.0, Z_CHENGDU + 12.0, Vec3::Z),
        (X_XINING + 3.0, Z_EMEI + 8.0, Vec3::NEG_Z),
        (X_XINING + 5.0, Z_EMEI + 8.0, Vec3::NEG_Z),
        (X_KANGDING + 20.0, Z_EMEI + 30.0, Vec3::Z),
        (X_KANGDING + 30.0, Z_EMEI + 30.0, Vec3::Z),
    ];
    cover_count += spawn_cover_batch(commands, &vehicle_covers, CoverKind::Low, "Cover_Vehicle");

    // === 街道轉角掩體點 (Full Cover) ===
    let diag_ne = Vec3::new(1.0, 0.0, -1.0).normalize();
    let diag_nw = Vec3::new(-1.0, 0.0, -1.0).normalize();
    let diag_se = Vec3::new(1.0, 0.0, 1.0).normalize();
    let diag_sw = Vec3::new(-1.0, 0.0, 1.0).normalize();

    let corner_full_covers = [
        (X_HAN + 10.0, Z_EMEI - 10.0, diag_sw),
        (X_HAN - 10.0, Z_EMEI - 10.0, diag_se),
        (X_HAN + 10.0, Z_EMEI + 10.0, diag_nw),
        (X_HAN - 10.0, Z_EMEI + 10.0, diag_ne),
        (X_HAN + 10.0, Z_WUCHANG - 10.0, diag_sw),
        (X_HAN - 10.0, Z_WUCHANG - 10.0, diag_se),
        (X_HAN + 10.0, Z_WUCHANG + 10.0, diag_nw),
        (X_HAN - 10.0, Z_WUCHANG + 10.0, diag_ne),
    ];
    cover_count += spawn_cover_batch(
        commands,
        &corner_full_covers,
        CoverKind::Full,
        "Cover_Corner",
    );

    info!("🛡️ 已生成 {} 個 AI 掩體點", cover_count);
}
