//! 車輛損壞視覺效果（煙霧、火焰、粒子）

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use rand::Rng;
use super::super::{Vehicle, VehicleType, VehicleChassisMesh, VehicleCabinMesh, VehicleOriginalColor, VehicleVisualRoot};
use super::health::{
    BodyPartDamage, BodyPartState, VehicleDamageState, VehicleHealth,
    BODY_HOOD, BODY_FRONT_BUMPER, BODY_REAR_BUMPER, BODY_LEFT_PANEL, BODY_RIGHT_PANEL, BODY_ROOF,
};
use crate::core::lifetime_linear_alpha;

/// 車輛損壞視覺效果資源
#[derive(Resource)]
pub struct VehicleDamageVisuals {
    /// 冒煙粒子 mesh
    pub smoke_mesh: Handle<Mesh>,
    /// 輕微冒煙材質（白煙）
    pub light_smoke_material: Handle<StandardMaterial>,
    /// 嚴重冒煙材質（黑煙）
    pub heavy_smoke_material: Handle<StandardMaterial>,
    /// 火焰粒子 mesh
    pub fire_mesh: Handle<Mesh>,
    /// 火焰材質
    pub fire_material: Handle<StandardMaterial>,
    /// 爆炸粒子 mesh
    pub explosion_mesh: Handle<Mesh>,
    /// 爆炸材質
    pub explosion_material: Handle<StandardMaterial>,
}

impl VehicleDamageVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            smoke_mesh: meshes.add(Sphere::new(0.3)),
            light_smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.8, 0.8, 0.4),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            heavy_smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.2, 0.2, 0.6),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
            fire_mesh: meshes.add(Sphere::new(0.2)),
            fire_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0),
                emissive: LinearRgba::new(15.0, 8.0, 0.0, 1.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            explosion_mesh: meshes.add(Sphere::new(2.0)),
            explosion_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.2),
                emissive: LinearRgba::new(50.0, 30.0, 5.0, 1.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        }
    }
}

/// 車輛損壞煙霧粒子
#[derive(Component)]
pub struct VehicleDamageSmoke {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 是否為黑煙（嚴重損壞）
    pub is_heavy: bool,
}

impl VehicleDamageSmoke {
    /// 建立新實例
    pub fn new(velocity: Vec3, is_heavy: bool) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime: if is_heavy { 2.0 } else { 1.5 },
            is_heavy,
        }
    }

    /// 計算透明度
    pub fn alpha(&self) -> f32 {
        lifetime_linear_alpha(self.lifetime, self.max_lifetime)
    }
}

/// 車輛火焰粒子
#[derive(Component)]
pub struct VehicleFire {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl VehicleFire {
    /// 建立新實例
    pub fn new(velocity: Vec3) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime: 0.5,
        }
    }

    /// 計算縮放
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        (1.0 - progress * 0.5).max(0.3)
    }
}

// ============================================================================
// 車輛損壞視覺效果常數
// ============================================================================

/// 中度損壞煙霧生成率（每秒）
const MODERATE_SMOKE_RATE: f32 = 1.2;
/// 嚴重損壞煙霧生成率（每秒）
const HEAVY_SMOKE_RATE: f32 = 6.0;
/// 瀕臨爆炸煙霧生成率（每秒）
const CRITICAL_SMOKE_RATE: f32 = 9.0;
/// 瀕臨爆炸火焰生成率（每秒）
const CRITICAL_FIRE_RATE: f32 = 6.0;

/// 車輛損壞視覺效果系統
/// 根據損壞狀態生成煙霧和火焰粒子
/// 使用時間基準的生成率，確保效果與幀率無關
pub fn vehicle_damage_effect_system(
    mut commands: Commands,
    time: Res<Time>,
    damage_visuals: Option<Res<VehicleDamageVisuals>>,
    vehicle_query: Query<(&Transform, &Vehicle, &VehicleHealth)>,
) {
    let Some(visuals) = damage_visuals else {
        return;
    };
    let dt = time.delta_secs();
    let mut rng = rand::rng();

    for (transform, vehicle, health) in vehicle_query.iter() {
        // 計算引擎蓋位置（車頭）
        let hood_offset = match vehicle.vehicle_type {
            VehicleType::Scooter => Vec3::new(0.0, 0.3, -0.6),
            VehicleType::Car | VehicleType::Taxi => Vec3::new(0.0, 0.5, -1.5),
            VehicleType::Bus => Vec3::new(0.0, 1.0, -4.0),
        };
        let hood_pos = transform.translation + transform.rotation * hood_offset;

        // 根據損壞狀態生成效果
        // 使用時間基準的機率：rate * dt 使生成與幀率無關
        match health.damage_state {
            VehicleDamageState::Moderate => {
                // 中度損壞：偶爾冒白煙
                if rng.random::<f32>() < MODERATE_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, false, &mut rng);
                }
            }
            VehicleDamageState::Heavy => {
                // 嚴重損壞：持續冒黑煙
                if rng.random::<f32>() < HEAVY_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, true, &mut rng);
                }
            }
            VehicleDamageState::Critical => {
                // 瀕臨爆炸：冒黑煙 + 火焰
                if rng.random::<f32>() < CRITICAL_SMOKE_RATE * dt {
                    spawn_damage_smoke(&mut commands, &visuals, hood_pos, true, &mut rng);
                }
                if rng.random::<f32>() < CRITICAL_FIRE_RATE * dt {
                    spawn_vehicle_fire(&mut commands, &visuals, hood_pos, &mut rng);
                }
            }
            _ => {}
        }
    }
}

/// 生成損壞煙霧粒子
fn spawn_damage_smoke(
    commands: &mut Commands,
    visuals: &VehicleDamageVisuals,
    position: Vec3,
    is_heavy: bool,
    rng: &mut rand::prelude::ThreadRng,
) {
    let spread = Vec3::new(
        rng.random_range(-0.3..0.3),
        rng.random_range(0.5..1.5),
        rng.random_range(-0.3..0.3),
    );

    let material = if is_heavy {
        visuals.heavy_smoke_material.clone()
    } else {
        visuals.light_smoke_material.clone()
    };

    commands.spawn((
        Mesh3d(visuals.smoke_mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(position).with_scale(Vec3::splat(0.2)),
        VehicleDamageSmoke::new(spread, is_heavy),
    ));
}

/// 生成車輛火焰粒子
fn spawn_vehicle_fire(
    commands: &mut Commands,
    visuals: &VehicleDamageVisuals,
    position: Vec3,
    rng: &mut rand::prelude::ThreadRng,
) {
    let spread = Vec3::new(
        rng.random_range(-0.2..0.2),
        rng.random_range(0.8..1.5),
        rng.random_range(-0.2..0.2),
    );

    commands.spawn((
        Mesh3d(visuals.fire_mesh.clone()),
        MeshMaterial3d(visuals.fire_material.clone()),
        Transform::from_translation(position + Vec3::Y * 0.1)
            .with_scale(Vec3::splat(rng.random_range(0.3..0.6))),
        VehicleFire::new(spread),
    ));
}

/// 車輛損壞粒子更新系統
/// 處理煙霧和火焰粒子的移動和刪除
pub fn vehicle_damage_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut smoke_query: Query<(Entity, &mut VehicleDamageSmoke, &mut Transform)>,
    mut fire_query: Query<(Entity, &mut VehicleFire, &mut Transform), Without<VehicleDamageSmoke>>,
) {
    let dt = time.delta_secs();

    // 更新煙霧粒子
    for (entity, mut smoke, mut transform) in smoke_query.iter_mut() {
        smoke.lifetime += dt;

        if smoke.lifetime >= smoke.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 煙霧上飄並減速
        smoke.velocity *= 1.0 - dt * 1.5;
        transform.translation += smoke.velocity * dt;

        // 擴散變大
        let progress = smoke.lifetime / smoke.max_lifetime;
        let scale = 0.2 + progress * 0.6;
        transform.scale = Vec3::splat(scale);
    }

    // 更新火焰粒子
    for (entity, mut fire, mut transform) in fire_query.iter_mut() {
        fire.lifetime += dt;

        if fire.lifetime >= fire.max_lifetime {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
            continue;
        }

        // 火焰快速上飄
        transform.translation += fire.velocity * dt;

        // 閃爍效果
        let flicker = (fire.lifetime * 20.0).sin() * 0.1 + 1.0;
        transform.scale = Vec3::splat(fire.scale() * flicker);
    }
}

// ============================================================================
// 車體部位損壞材質變色系統
// ============================================================================

/// 車體損壞材質暗化基礎色
const DAMAGE_DARKEN_COLOR: Vec3 = Vec3::new(0.08, 0.06, 0.04);

/// 車體部位損壞視覺系統
///
/// 根據 `BodyPartDamage` 的平均損壞程度調整車輛材質顏色：
/// - Intact: 原色
/// - Scratched: 輕微暗化（10%）
/// - Dented: 明顯暗化（30%）
/// - Crushed: 嚴重暗化（55%），偏向焦黑色
///
/// 每台車輛已有獨立材質（`create_body_material` 產生），可安全修改。
pub fn body_part_visual_damage_system(
    vehicle_query: Query<(Entity, &BodyPartDamage), Changed<BodyPartDamage>>,
    children_query: Query<&Children>,
    visual_root_query: Query<&Children, With<VehicleVisualRoot>>,
    chassis_query: Query<(&MeshMaterial3d<StandardMaterial>, &VehicleOriginalColor), With<VehicleChassisMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, body_parts) in &vehicle_query {
        let darken = body_parts.average_darken_factor();
        if darken <= 0.0 {
            continue;
        }

        // 遍歷 Root → VehicleVisualRoot → 子 mesh
        let Ok(root_children) = children_query.get(entity) else {
            continue;
        };
        for child in root_children.iter() {
            let Ok(visual_children) = visual_root_query.get(child) else {
                continue;
            };
            for mesh_child in visual_children.iter() {
                let Ok((mat_handle, original_color)) = chassis_query.get(mesh_child) else {
                    continue;
                };
                let Some(material) = materials.get_mut(mat_handle) else {
                    continue;
                };
                // lerp(original_color, DAMAGE_DARKEN_COLOR, darken)
                let orig = original_color.0.to_srgba();
                let r = orig.red * (1.0 - darken) + DAMAGE_DARKEN_COLOR.x * darken;
                let g = orig.green * (1.0 - darken) + DAMAGE_DARKEN_COLOR.y * darken;
                let b = orig.blue * (1.0 - darken) + DAMAGE_DARKEN_COLOR.z * darken;
                material.base_color = Color::srgb(r, g, b);
            }
        }
    }
}

// ============================================================================
// 車體部位損壞變形系統
// ============================================================================

/// 根據 `BodyPartState` 計算底盤變形（縮放 + 偏移）
fn compute_chassis_deformation(body_parts: &BodyPartDamage) -> (Vec3, Vec3) {
    let mut scale = Vec3::ONE;
    let mut offset = Vec3::ZERO;

    // HOOD — Y 軸壓縮（引擎蓋下陷）
    match body_parts.states[BODY_HOOD] {
        BodyPartState::Scratched => { scale.y *= 0.95; }
        BodyPartState::Dented => { scale.y *= 0.88; offset.y -= 0.03; }
        BodyPartState::Crushed => { scale.y *= 0.75; offset.y -= 0.08; }
        _ => {}
    }

    // FRONT_BUMPER — Z 軸向後壓縮（前部撞凹）
    match body_parts.states[BODY_FRONT_BUMPER] {
        BodyPartState::Scratched => { offset.z -= 0.02; }
        BodyPartState::Dented => { offset.z -= 0.05; scale.z *= 0.92; }
        BodyPartState::Crushed => { offset.z -= 0.1; scale.z *= 0.8; }
        _ => {}
    }

    // REAR_BUMPER — Z 軸向前壓縮（後部撞凹）
    match body_parts.states[BODY_REAR_BUMPER] {
        BodyPartState::Scratched => { offset.z += 0.02; }
        BodyPartState::Dented => { offset.z += 0.05; scale.z *= 0.92; }
        BodyPartState::Crushed => { offset.z += 0.1; scale.z *= 0.8; }
        _ => {}
    }

    // LEFT_PANEL — X 軸向內壓縮
    match body_parts.states[BODY_LEFT_PANEL] {
        BodyPartState::Scratched => { offset.x += 0.02; }
        BodyPartState::Dented => { offset.x += 0.05; scale.x *= 0.95; }
        BodyPartState::Crushed => { offset.x += 0.08; scale.x *= 0.88; }
        _ => {}
    }

    // RIGHT_PANEL — X 軸向內壓縮
    match body_parts.states[BODY_RIGHT_PANEL] {
        BodyPartState::Scratched => { offset.x -= 0.02; }
        BodyPartState::Dented => { offset.x -= 0.05; scale.x *= 0.95; }
        BodyPartState::Crushed => { offset.x -= 0.08; scale.x *= 0.88; }
        _ => {}
    }

    (scale, offset)
}

/// 車體部位損壞變形系統
///
/// 根據各部位的 `BodyPartState` 修改底盤和車艙 mesh 的 Transform：
/// - 底盤：引擎蓋、保險桿、側板的壓縮和位移
/// - 車艙：車頂壓塌
pub fn vehicle_deformation_system(
    vehicle_query: Query<(Entity, &BodyPartDamage), Changed<BodyPartDamage>>,
    children_query: Query<&Children>,
    visual_root_query: Query<&Children, With<VehicleVisualRoot>>,
    mut chassis_query: Query<&mut Transform, (With<VehicleChassisMesh>, Without<VehicleCabinMesh>)>,
    mut cabin_query: Query<(&mut Transform, &VehicleCabinMesh), Without<VehicleChassisMesh>>,
) {
    for (entity, body_parts) in &vehicle_query {
        let Ok(root_children) = children_query.get(entity) else {
            continue;
        };
        for child in root_children.iter() {
            let Ok(visual_children) = visual_root_query.get(child) else {
                continue;
            };
            for mesh_child in visual_children.iter() {
                // 底盤變形
                if let Ok(mut transform) = chassis_query.get_mut(mesh_child) {
                    let (scale, offset) = compute_chassis_deformation(body_parts);
                    transform.scale = scale;
                    transform.translation = offset;
                }

                // 車艙變形（車頂壓塌）
                if let Ok((mut transform, cabin)) = cabin_query.get_mut(mesh_child) {
                    let mut scale = Vec3::ONE;
                    let mut y_offset = 0.0;

                    match body_parts.states[BODY_ROOF] {
                        BodyPartState::Dented => { scale.y = 0.92; }
                        BodyPartState::Crushed => { scale.y = 0.8; y_offset = -0.05; }
                        _ => {}
                    }

                    transform.scale = scale;
                    transform.translation = Vec3::new(0.0, cabin.base_y + y_offset, cabin.base_z);
                }
            }
        }
    }
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod deformation_tests {
    use super::*;

    #[test]
    fn test_chassis_no_damage_is_identity() {
        let bp = BodyPartDamage::default();
        let (scale, offset) = compute_chassis_deformation(&bp);
        assert_eq!(scale, Vec3::ONE);
        assert_eq!(offset, Vec3::ZERO);
    }

    #[test]
    fn test_chassis_hood_crushed() {
        let mut bp = BodyPartDamage::default();
        bp.states[BODY_HOOD] = BodyPartState::Crushed;
        let (scale, offset) = compute_chassis_deformation(&bp);
        assert!((scale.y - 0.75).abs() < 0.001);
        assert!(offset.y < 0.0);
        // X 和 Z 不受影響
        assert!((scale.x - 1.0).abs() < 0.001);
        assert!((scale.z - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_chassis_front_bumper_dented() {
        let mut bp = BodyPartDamage::default();
        bp.states[BODY_FRONT_BUMPER] = BodyPartState::Dented;
        let (scale, offset) = compute_chassis_deformation(&bp);
        assert!((scale.z - 0.92).abs() < 0.001);
        assert!(offset.z < 0.0); // 向後壓縮
    }

    #[test]
    fn test_chassis_both_panels_crushed() {
        let mut bp = BodyPartDamage::default();
        bp.states[BODY_LEFT_PANEL] = BodyPartState::Crushed;
        bp.states[BODY_RIGHT_PANEL] = BodyPartState::Crushed;
        let (scale, offset) = compute_chassis_deformation(&bp);
        // 左右面板壓縮同時作用：0.88 * 0.88
        assert!((scale.x - 0.88 * 0.88).abs() < 0.001);
        // 左右偏移互相抵消
        assert!(offset.x.abs() < 0.001);
    }

    #[test]
    fn test_chassis_combined_damage() {
        let mut bp = BodyPartDamage::default();
        bp.states[BODY_HOOD] = BodyPartState::Scratched;
        bp.states[BODY_FRONT_BUMPER] = BodyPartState::Crushed;
        bp.states[BODY_LEFT_PANEL] = BodyPartState::Dented;
        let (scale, offset) = compute_chassis_deformation(&bp);
        assert!((scale.y - 0.95).abs() < 0.001);
        assert!((scale.z - 0.8).abs() < 0.001);
        assert!((scale.x - 0.95).abs() < 0.001);
        assert!(offset.z < 0.0); // 前方撞凹
        assert!(offset.x > 0.0); // 左側面板向內
    }

    #[test]
    fn test_darken_color_constants() {
        assert!(DAMAGE_DARKEN_COLOR.x >= 0.0 && DAMAGE_DARKEN_COLOR.x <= 1.0);
        assert!(DAMAGE_DARKEN_COLOR.y >= 0.0 && DAMAGE_DARKEN_COLOR.y <= 1.0);
        assert!(DAMAGE_DARKEN_COLOR.z >= 0.0 && DAMAGE_DARKEN_COLOR.z <= 1.0);
    }

    #[test]
    fn test_rear_bumper_offset_positive_z() {
        let mut bp = BodyPartDamage::default();
        bp.states[BODY_REAR_BUMPER] = BodyPartState::Scratched;
        let (_scale, offset) = compute_chassis_deformation(&bp);
        assert!(offset.z > 0.0); // 後保險桿向前位移
    }

    #[test]
    fn test_deformation_severity_ordering() {
        // 更嚴重的損壞應該產生更大的偏移
        let mut bp1 = BodyPartDamage::default();
        bp1.states[BODY_HOOD] = BodyPartState::Scratched;
        let (scale1, _) = compute_chassis_deformation(&bp1);

        let mut bp2 = BodyPartDamage::default();
        bp2.states[BODY_HOOD] = BodyPartState::Crushed;
        let (scale2, _) = compute_chassis_deformation(&bp2);

        assert!(scale1.y > scale2.y); // Crushed 比 Scratched 壓縮更多
    }
}
