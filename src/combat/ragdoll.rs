//! 骨骼布娃娃系統
//!
//! GTA5 風格的多關節布娃娃物理，每個身體部位獨立模擬。


use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use std::collections::HashMap;

// ============================================================================
// 身體部位標記組件
// ============================================================================

/// 身體部位類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BodyPartType {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
    LeftFoot,
    RightFoot,
}

/// 身體部位標記組件
/// 在生成行人時添加到每個身體部位，用於布娃娃轉換
#[derive(Component, Clone)]
pub struct BodyPart {
    pub part_type: BodyPartType,
    /// 相對於軀幹的連接點（本地座標）
    pub attachment_point: Vec3,
    /// 碰撞器半徑
    pub collider_radius: f32,
    /// 碰撞器高度（膠囊）
    pub collider_height: f32,
    /// 質量
    pub mass: f32,
}

impl BodyPart {
    /// 頭部部件
    pub fn head() -> Self {
        Self {
            part_type: BodyPartType::Head,
            attachment_point: Vec3::new(0.0, 0.35, 0.0), // 脖子位置
            collider_radius: 0.12,
            collider_height: 0.0, // 球形
            mass: 4.5,            // 頭部約 4.5 kg
        }
    }

    /// 軀幹部件
    pub fn torso() -> Self {
        Self {
            part_type: BodyPartType::Torso,
            attachment_point: Vec3::ZERO,
            collider_radius: 0.18,
            collider_height: 0.5,
            mass: 35.0, // 軀幹約 35 kg
        }
    }

    /// 左臂部件
    pub fn left_arm() -> Self {
        Self {
            part_type: BodyPartType::LeftArm,
            attachment_point: Vec3::new(-0.22, 0.2, 0.0), // 左肩
            collider_radius: 0.05,
            collider_height: 0.35,
            mass: 3.5,
        }
    }

    /// 右臂部件
    pub fn right_arm() -> Self {
        Self {
            part_type: BodyPartType::RightArm,
            attachment_point: Vec3::new(0.22, 0.2, 0.0), // 右肩
            collider_radius: 0.05,
            collider_height: 0.35,
            mass: 3.5,
        }
    }

    /// 左腿部件
    pub fn left_leg() -> Self {
        Self {
            part_type: BodyPartType::LeftLeg,
            attachment_point: Vec3::new(-0.08, -0.25, 0.0), // 左髖
            collider_radius: 0.07,
            collider_height: 0.45,
            mass: 10.0,
        }
    }

    /// 右腿部件
    pub fn right_leg() -> Self {
        Self {
            part_type: BodyPartType::RightLeg,
            attachment_point: Vec3::new(0.08, -0.25, 0.0), // 右髖
            collider_radius: 0.07,
            collider_height: 0.45,
            mass: 10.0,
        }
    }

    /// 左腳部件
    pub fn left_foot() -> Self {
        Self {
            part_type: BodyPartType::LeftFoot,
            attachment_point: Vec3::new(0.0, -0.45, 0.0), // 腳踝
            collider_radius: 0.04,
            collider_height: 0.1,
            mass: 1.0,
        }
    }

    /// 右腳部件
    pub fn right_foot() -> Self {
        Self {
            part_type: BodyPartType::RightFoot,
            attachment_point: Vec3::new(0.0, -0.45, 0.0), // 腳踝
            collider_radius: 0.04,
            collider_height: 0.1,
            mass: 1.0,
        }
    }
}

// ============================================================================
// 骨骼布娃娃組件
// ============================================================================

/// 骨骼布娃娃標記組件
/// 標記整個布娃娃系統的根實體
#[derive(Component)]
pub struct SkeletalRagdoll {
    /// 生命週期計時器
    pub lifetime: f32,
    /// 最大生命週期
    pub max_lifetime: f32,
    /// 所有身體部位實體
    pub body_parts: Vec<Entity>,
    /// 衝擊力方向
    pub impulse_direction: Vec3,
    /// 衝擊力強度
    pub impulse_strength: f32,
}

impl Default for SkeletalRagdoll {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 6.0, // 比單體布娃娃稍長
            body_parts: Vec::new(),
            impulse_direction: Vec3::NEG_Z,
            impulse_strength: 300.0,
        }
    }
}

/// 布娃娃身體部位組件（轉換後）
/// 標記已轉換為物理實體的身體部位
#[derive(Component)]
pub struct RagdollPart {
    pub part_type: BodyPartType,
    /// 父布娃娃實體
    pub ragdoll_entity: Entity,
}

// ============================================================================
// 關節參數常數
// ============================================================================

/// 脖子關節角度限制（弧度）
const NECK_SWING_LIMIT: f32 = 0.6; // ~34 度
/// 肩膀關節角度限制
const SHOULDER_SWING_LIMIT: f32 = 1.5; // ~86 度
/// 髖關節角度限制
const HIP_SWING_LIMIT: f32 = 1.2; // ~69 度
/// 關節阻尼
const JOINT_DAMPING: f32 = 5.0;
/// 關節剛度
const JOINT_STIFFNESS: f32 = 0.0; // 無彈簧效果

// ============================================================================
// 布娃娃轉換函數
// ============================================================================

/// 將行人實體轉換為骨骼布娃娃
///
/// # Arguments
/// * `commands` - 命令緩衝
/// * `parent_entity` - 行人根實體
/// * `parent_transform` - 行人世界變換
/// * `children` - 行人子實體
/// * `body_parts_query` - 身體部位查詢
/// * `impulse_dir` - 衝擊方向
/// * `impulse_strength` - 衝擊強度
///
/// # Returns
/// 新創建的布娃娃根實體（供 RagdollTracker 追蹤），若無身體部位則返回 None
pub fn convert_to_skeletal_ragdoll(
    commands: &mut Commands,
    parent_entity: Entity,
    parent_transform: &Transform,
    children: &Children,
    body_parts_query: &Query<(
        &BodyPart,
        &Transform,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    impulse_dir: Vec3,
    impulse_strength: f32,
) -> Option<Entity> {
    let parent_pos = parent_transform.translation;
    let parent_rot = parent_transform.rotation;

    // 收集所有身體部位資訊
    let mut parts_info: Vec<(
        Entity,
        BodyPart,
        Transform,
        Mesh3d,
        MeshMaterial3d<StandardMaterial>,
    )> = Vec::new();

    for child in children.iter() {
        if let Ok((body_part, local_transform, mesh, material)) = body_parts_query.get(child) {
            // 計算世界座標
            let world_pos = parent_pos + parent_rot * local_transform.translation;
            let world_rot = parent_rot * local_transform.rotation;
            let world_transform = Transform::from_translation(world_pos)
                .with_rotation(world_rot)
                .with_scale(local_transform.scale);

            parts_info.push((
                child,
                body_part.clone(),
                world_transform,
                mesh.clone(),
                material.clone(),
            ));
        }
    }

    // 如果沒有標記身體部位，返回 None（調用方可使用傳統單體布娃娃）
    if parts_info.is_empty() {
        return None;
    }

    // 創建骨骼布娃娃根實體（body_parts 稍後填充）
    let ragdoll_root = commands
        .spawn((
            Transform::from_translation(parent_pos),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // 創建物理身體部位
    let mut created_parts: Vec<(BodyPartType, Entity)> = Vec::new();

    for (old_entity, body_part, world_transform, mesh, material) in &parts_info {
        // 創建碰撞器
        let collider = if body_part.collider_height > 0.01 {
            Collider::capsule_y(body_part.collider_height / 2.0, body_part.collider_radius)
        } else {
            Collider::ball(body_part.collider_radius)
        };

        // 計算衝量（頭部和軀幹接收主要衝量）
        let part_impulse = match body_part.part_type {
            BodyPartType::Torso => impulse_strength,
            BodyPartType::Head => impulse_strength * 0.8,
            _ => impulse_strength * 0.3,
        };

        // 計算旋轉衝量（讓布娃娃有翻滾效果）
        let torque_axis = Vec3::new(impulse_dir.z, 0.0, -impulse_dir.x).normalize_or_zero();
        let torque_strength = match body_part.part_type {
            BodyPartType::Torso => part_impulse * 0.3,
            BodyPartType::Head => part_impulse * 0.1,
            _ => part_impulse * 0.05,
        };

        // 生成物理身體部位
        let part_entity = commands
            .spawn((
                mesh.clone(),
                material.clone(),
                *world_transform,
                GlobalTransform::default(),
                Visibility::default(),
                RagdollPart {
                    part_type: body_part.part_type,
                    ragdoll_entity: ragdoll_root,
                },
            ))
            .insert((
                RigidBody::Dynamic,
                collider,
                ColliderMassProperties::Mass(body_part.mass),
                Velocity::default(),
                ExternalImpulse {
                    impulse: Vec3::new(
                        impulse_dir.x * part_impulse,
                        part_impulse * 0.3, // 向上推力
                        impulse_dir.z * part_impulse,
                    ),
                    torque_impulse: torque_axis * torque_strength,
                },
                GravityScale(1.5),
                Damping {
                    linear_damping: 0.5,
                    angular_damping: 1.0,
                },
                CollisionGroups::new(Group::GROUP_10, Group::GROUP_1 | Group::GROUP_10),
                Friction::coefficient(0.7),
                Restitution::coefficient(0.1),
            ))
            .id();

        created_parts.push((body_part.part_type, part_entity));

        // 移除原始子實體
        commands.entity(*old_entity).despawn();
    }

    // 創建關節約束
    create_skeletal_joints(commands, &created_parts);

    // 更新布娃娃根實體的部位列表
    let part_entities: Vec<Entity> = created_parts.iter().map(|(_, e)| *e).collect();
    commands.entity(ragdoll_root).insert(SkeletalRagdoll {
        body_parts: part_entities,
        impulse_direction: impulse_dir,
        impulse_strength,
        ..default()
    });

    // 移除原始行人實體（子實體已被單獨處理或 despawn）
    commands.entity(parent_entity).despawn();

    Some(ragdoll_root)
}

/// 創建身體部位間的關節約束
fn create_skeletal_joints(commands: &mut Commands, parts: &[(BodyPartType, Entity)]) {
    // 使用 HashMap 預先索引，避免重複遍歷
    let part_map: HashMap<BodyPartType, Entity> = parts.iter().cloned().collect();

    // 找到軀幹實體（中心）
    let Some(&torso_entity) = part_map.get(&BodyPartType::Torso) else {
        return;
    };

    // 創建關節
    for (part_type, part_entity) in parts {
        if *part_type == BodyPartType::Torso {
            continue;
        }

        // 關節錨點配置：(父部位類型, 父端錨點, 子端錨點)
        let (parent_type, anchor1, anchor2) = match part_type {
            // 頭部連接到軀幹（脖子位置）
            BodyPartType::Head => (
                BodyPartType::Torso,
                Vec3::new(0.0, 0.3, 0.0),
                Vec3::new(0.0, -0.1, 0.0),
            ),
            // 手臂連接到軀幹（肩膀位置）
            BodyPartType::LeftArm => (
                BodyPartType::Torso,
                Vec3::new(-0.2, 0.2, 0.0),
                Vec3::new(0.0, 0.15, 0.0),
            ),
            BodyPartType::RightArm => (
                BodyPartType::Torso,
                Vec3::new(0.2, 0.2, 0.0),
                Vec3::new(0.0, 0.15, 0.0),
            ),
            // 腿部連接到軀幹（髖部位置）
            BodyPartType::LeftLeg => (
                BodyPartType::Torso,
                Vec3::new(-0.08, -0.25, 0.0),
                Vec3::new(0.0, 0.2, 0.0),
            ),
            BodyPartType::RightLeg => (
                BodyPartType::Torso,
                Vec3::new(0.08, -0.25, 0.0),
                Vec3::new(0.0, 0.2, 0.0),
            ),
            // 腳連接到腿（腳踝位置）
            BodyPartType::LeftFoot => (
                BodyPartType::LeftLeg,
                Vec3::new(0.0, -0.2, 0.0),
                Vec3::new(0.0, 0.05, 0.0),
            ),
            BodyPartType::RightFoot => (
                BodyPartType::RightLeg,
                Vec3::new(0.0, -0.2, 0.0),
                Vec3::new(0.0, 0.05, 0.0),
            ),
            BodyPartType::Torso => continue,
        };

        // 從 HashMap 取得父實體，找不到時回退到軀幹
        let parent_entity = part_map.get(&parent_type).copied().unwrap_or(torso_entity);

        // 創建球窩關節（允許三軸旋轉）
        let joint = SphericalJointBuilder::new()
            .local_anchor1(anchor1)
            .local_anchor2(anchor2)
            .motor_position(JointAxis::AngX, 0.0, JOINT_STIFFNESS, JOINT_DAMPING)
            .motor_position(JointAxis::AngY, 0.0, JOINT_STIFFNESS, JOINT_DAMPING)
            .motor_position(JointAxis::AngZ, 0.0, JOINT_STIFFNESS, JOINT_DAMPING);

        // 根據部位類型設置角度限制
        let swing_limit = match part_type {
            BodyPartType::Head => NECK_SWING_LIMIT,
            BodyPartType::LeftArm | BodyPartType::RightArm => SHOULDER_SWING_LIMIT,
            BodyPartType::LeftLeg | BodyPartType::RightLeg => HIP_SWING_LIMIT,
            _ => 0.5, // 腳踝限制較小
        };

        let joint = joint
            .limits(JointAxis::AngX, [-swing_limit, swing_limit])
            .limits(JointAxis::AngY, [-swing_limit * 0.5, swing_limit * 0.5])
            .limits(JointAxis::AngZ, [-swing_limit, swing_limit]);

        commands
            .entity(*part_entity)
            .insert(ImpulseJoint::new(parent_entity, joint));
    }
}

// ============================================================================
// 骨骼布娃娃更新系統
// ============================================================================

/// 骨骼布娃娃生命週期更新系統
pub fn skeletal_ragdoll_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut ragdoll_query: Query<(Entity, &mut SkeletalRagdoll)>,
    part_velocity_query: Query<&Velocity, With<RagdollPart>>,
) {
    let dt = time.delta_secs();

    for (entity, mut ragdoll) in &mut ragdoll_query {
        ragdoll.lifetime += dt;

        // 檢查所有部位是否靜止
        let (all_settled, valid_parts) = check_ragdoll_settled(&ragdoll, &part_velocity_query);

        // 如果所有部位靜止超過 1 秒，加速生命週期
        if all_settled && ragdoll.lifetime > 1.0 && valid_parts > 0 {
            ragdoll.lifetime += dt * 2.0;
        }

        // 超時移除
        if ragdoll.lifetime >= ragdoll.max_lifetime {
            despawn_ragdoll(&mut commands, entity, &ragdoll);
        }
    }
}

fn check_ragdoll_settled(
    ragdoll: &SkeletalRagdoll,
    part_velocity_query: &Query<&Velocity, With<RagdollPart>>,
) -> (bool, u32) {
    let mut all_settled = true;
    let mut valid_parts = 0;

    for &part_entity in &ragdoll.body_parts {
        if let Ok(velocity) = part_velocity_query.get(part_entity) {
            valid_parts += 1;
            if velocity.linvel.length() > 0.3 {
                all_settled = false;
            }
        }
    }
    (all_settled, valid_parts)
}

fn despawn_ragdoll(commands: &mut Commands, entity: Entity, ragdoll: &SkeletalRagdoll) {
    // 移除所有身體部位
    for &part_entity in &ragdoll.body_parts {
        if let Ok(mut entity_commands) = commands.get_entity(part_entity) {
            entity_commands.despawn();
        }
    }
    commands.entity(entity).despawn();
}

/// 骨骼布娃娃視覺淡出系統
pub fn skeletal_ragdoll_visual_system(
    ragdoll_query: Query<&SkeletalRagdoll>,
    mut part_query: Query<(&RagdollPart, &mut Visibility)>,
) {
    for (part, mut visibility) in &mut part_query {
        if let Ok(ragdoll) = ragdoll_query.get(part.ragdoll_entity) {
            let fade_start = ragdoll.max_lifetime - 1.5;

            if ragdoll.lifetime > fade_start {
                let fade_progress = (ragdoll.lifetime - fade_start) / 1.5;
                let blink_rate = 4.0 + fade_progress * 12.0;
                let visible = (ragdoll.lifetime * blink_rate).sin() > 0.0;

                *visibility = if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

/// 防止身體部位穿透地面（使用物理兼容方式）
pub fn skeletal_ragdoll_ground_clamp_system(
    mut part_query: Query<(&Transform, &mut Velocity), With<RagdollPart>>,
) {
    for (transform, mut velocity) in &mut part_query {
        // 如果接近地面且仍在下降，停止下降並施加輕微反彈
        if transform.translation.y < 0.1 && velocity.linvel.y < 0.0 {
            velocity.linvel.y = velocity.linvel.y.abs() * 0.2; // 輕微反彈
            velocity.linvel.x *= 0.9; // 地面摩擦
            velocity.linvel.z *= 0.9;
        }
    }
}
