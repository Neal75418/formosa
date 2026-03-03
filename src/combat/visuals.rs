//! 戰鬥視覺效果（子彈拖尾、布娃娃）

use super::weapon::TracerStyle;
use super::weapon::WeaponType;
use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

// ============================================================================
// 射擊視覺效果
// ============================================================================

/// 武器模型標記（附加在武器視覺實體上）
#[derive(Component, Debug)]
pub struct WeaponModel {
    pub weapon_type: WeaponType,
}

/// 子彈拖尾效果標記
#[derive(Component)]
pub struct BulletTracer {
    #[allow(dead_code)]
    pub start_pos: Vec3,
    #[allow(dead_code)]
    pub end_pos: Vec3,
    pub lifetime: f32,
}

/// 槍口閃光標記
#[derive(Component)]
pub struct MuzzleFlash {
    pub lifetime: f32,
}

/// 擊中特效標記
#[derive(Component)]
pub struct ImpactEffect {
    pub lifetime: f32,
    pub max_lifetime: f32,
}

// ============================================================================
// 血液粒子系統
// ============================================================================

/// 血液粒子組件
#[derive(Component)]
pub struct BloodParticle {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl BloodParticle {
    /// 建立新實例
    pub fn new(velocity: Vec3, max_lifetime: f32) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime,
        }
    }
}

/// 血液視覺效果資源（預生成的 mesh 和 material）
#[derive(Resource)]
pub struct BloodVisuals {
    /// 血液粒子 mesh
    pub particle_mesh: Handle<Mesh>,
    /// 血液粒子材質
    pub particle_material: Handle<StandardMaterial>,
}

impl BloodVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            particle_mesh: meshes.add(Sphere::new(0.04)),
            particle_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.0, 0.0), // 深紅色
                emissive: LinearRgba::new(0.3, 0.0, 0.0, 1.0),
                perceptual_roughness: 0.8,
                metallic: 0.0,
                ..default()
            }),
        }
    }
}

// ============================================================================
// 布娃娃系統 (簡易版)
// ============================================================================

///布娃娃狀態組件
/// 當敵人死亡時，添加此組件來啟用物理布娃娃效果
#[derive(Component)]
pub struct Ragdoll {
    /// 布娃娃持續時間計時器
    pub lifetime: f32,
    /// 最大持續時間（秒）
    pub max_lifetime: f32,
    #[allow(dead_code)]
    pub physics_applied: bool,
    #[allow(dead_code)]
    pub impulse_direction: Vec3,
    #[allow(dead_code)]
    pub impulse_strength: f32,
}

impl Default for Ragdoll {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 5.0, // 5 秒後消失
            physics_applied: false,
            impulse_direction: Vec3::NEG_Z,
            impulse_strength: 300.0,
        }
    }
}

impl Ragdoll {
    /// 創建帶方向的布娃娃
    pub fn with_impulse(direction: Vec3, strength: f32) -> Self {
        Self {
            impulse_direction: direction.normalize_or_zero(),
            impulse_strength: strength,
            ..Default::default()
        }
    }
}

/// 布娃娃追蹤器（限制屍體數量）
#[derive(Resource)]
pub struct RagdollTracker {
    /// 追蹤的布娃娃實體和生成時間（使用 VecDeque 以 O(1) 移除舊記錄）
    pub ragdolls: VecDeque<(Entity, f32)>,
    /// 最大屍體數量
    pub max_count: usize,
}

impl Default for RagdollTracker {
    fn default() -> Self {
        Self {
            ragdolls: VecDeque::new(),
            max_count: 10,
        }
    }
}

// ============================================================================
// 護甲特效
// ============================================================================

/// 護甲碎片粒子組件
#[derive(Component)]
pub struct ArmorShardParticle {
    /// 速度
    pub velocity: Vec3,
    /// 角速度
    pub angular_velocity: Vec3,
    /// 生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl ArmorShardParticle {
    /// 建立新實例
    pub fn new(velocity: Vec3, angular_velocity: Vec3, max_lifetime: f32) -> Self {
        Self {
            velocity,
            angular_velocity,
            lifetime: 0.0,
            max_lifetime,
        }
    }
}

/// 護甲火花粒子組件（受擊時的火花）
#[derive(Component)]
pub struct ArmorSparkParticle {
    /// 速度
    pub velocity: Vec3,
    /// 生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
}

impl ArmorSparkParticle {
    /// 建立新實例
    pub fn new(velocity: Vec3, max_lifetime: f32) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime,
        }
    }
}

/// 護甲特效視覺資源
#[derive(Resource)]
pub struct ArmorEffectVisuals {
    /// 碎片 Mesh
    pub shard_mesh: Handle<Mesh>,
    /// 碎片材質（金屬質感）
    pub shard_material: Handle<StandardMaterial>,
    /// 火花 Mesh
    pub spark_mesh: Handle<Mesh>,
    /// 火花材質（發光）
    pub spark_material: Handle<StandardMaterial>,
}

impl ArmorEffectVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        // 碎片 Mesh（小三角形）
        let shard_mesh = meshes.add(Cuboid::new(0.03, 0.015, 0.02));

        // 碎片材質（藍灰色金屬）
        let shard_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.5, 0.6),
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        });

        // 火花 Mesh（小球）
        let spark_mesh = meshes.add(Sphere::new(0.015));

        // 火花材質（明亮的黃/橙色發光）
        let spark_material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.2),
            emissive: LinearRgba::new(10.0, 6.0, 1.0, 1.0),
            ..default()
        });

        Self {
            shard_mesh,
            shard_material,
            spark_mesh,
            spark_material,
        }
    }
}

// ============================================================================
// 戰鬥視覺特效資源
// ============================================================================

/// 單一彈道風格配置
#[derive(Clone)]
pub struct TracerConfig {
    pub material: Handle<StandardMaterial>,
    pub mesh: Handle<Mesh>,
    pub lifetime: f32,  // 拖尾存活時間
    pub thickness: f32, // 用於 scale 調整
}

/// 戰鬥視覺效果共用資源（避免每次射擊創建新 Mesh/Material）
#[derive(Resource)]
pub struct CombatVisuals {
    /// 各武器類型的彈道配置
    pub tracers: HashMap<TracerStyle, TracerConfig>,
    /// 槍口閃光
    pub muzzle_material: Handle<StandardMaterial>,
    pub muzzle_mesh: Handle<Mesh>,
    /// 擊中特效（火花/塵土）
    pub impact_material: Handle<StandardMaterial>,
    pub impact_mesh: Handle<Mesh>,
}

impl CombatVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        let mut tracers = HashMap::new();

        // 手槍：淡黃色短軌跡，較淡
        tracers.insert(
            TracerStyle::Pistol,
            TracerConfig {
                material: materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.9, 0.6, 0.5), // 淡黃，半透明
                    emissive: LinearRgba::new(4.0, 3.5, 1.5, 1.0),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                }),
                mesh: meshes.add(Capsule3d::new(0.015, 0.5)), // 很細
                lifetime: 0.08,
                thickness: 1.0,
            },
        );

        // 衝鋒槍：橙黃色細軌跡
        tracers.insert(
            TracerStyle::SMG,
            TracerConfig {
                material: materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.7, 0.3, 0.7),
                    emissive: LinearRgba::new(8.0, 5.0, 1.0, 1.0),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                }),
                mesh: meshes.add(Capsule3d::new(0.02, 0.5)),
                lifetime: 0.1,
                thickness: 1.0,
            },
        );

        // 霰彈槍：白色/灰色彈丸軌跡
        tracers.insert(
            TracerStyle::Shotgun,
            TracerConfig {
                material: materials.add(StandardMaterial {
                    base_color: Color::srgba(0.9, 0.9, 0.95, 0.6),
                    emissive: LinearRgba::new(3.0, 3.0, 3.5, 1.0),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                }),
                mesh: meshes.add(Capsule3d::new(0.012, 0.3)), // 更短更細（彈丸）
                lifetime: 0.06,
                thickness: 1.0,
            },
        );

        // 步槍：紅/橙色長曳光彈（軍用曳光彈風格）
        tracers.insert(
            TracerStyle::Rifle,
            TracerConfig {
                material: materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.4, 0.2, 0.9),
                    emissive: LinearRgba::new(15.0, 6.0, 2.0, 1.0), // 明亮的紅橙色
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                }),
                mesh: meshes.add(Capsule3d::new(0.025, 0.5)),
                lifetime: 0.18,
                thickness: 1.2,
            },
        );

        Self {
            tracers,
            // 槍口閃光：明亮的橙黃色火光
            muzzle_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.7, 0.3, 0.9),
                emissive: LinearRgba::new(20.0, 12.0, 3.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            muzzle_mesh: meshes.add(Sphere::new(0.15)),
            // 擊中特效：橙黃色火花（比槍口閃光小）
            impact_material: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.8, 0.4, 0.9),
                emissive: LinearRgba::new(12.0, 8.0, 2.0, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            impact_mesh: meshes.add(Sphere::new(0.08)),
        }
    }

    /// 取得指定風格的彈道配置
    pub fn get_tracer(&self, style: TracerStyle) -> Option<&TracerConfig> {
        self.tracers.get(&style)
    }
}

// ============================================================================
// 武器模型視覺
// ============================================================================

/// 武器模型視覺資源（預生成的 mesh 和 material）
#[derive(Resource)]
pub struct WeaponVisuals {
    pub staff: WeaponModelData,
    pub knife: WeaponModelData,
    pub pistol: WeaponModelData,
    pub smg: WeaponModelData,
    pub shotgun: WeaponModelData,
    pub rifle: WeaponModelData,
}

/// 單一武器模型數據（用於多部件組合）
#[derive(Clone)]
pub struct WeaponModelData {
    /// 武器各部件（mesh, material, local_transform）
    pub parts: Vec<WeaponPart>,
    /// 槍口相對於武器根的偏移（本地座標）
    pub muzzle_offset: Vec3,
    /// 武器根相對於手的偏移和旋轉
    pub hand_offset: Vec3,
    pub hand_rotation: Quat,
}

/// 武器部件
#[derive(Clone)]
pub struct WeaponPart {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub transform: Transform,
}

impl WeaponVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        // === 材質定義 ===
        // 金屬槍身（深灰/黑色）
        let gun_metal = materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.15, 0.18),
            metallic: 0.9,
            perceptual_roughness: 0.3,
            ..default()
        });
        // 槍管（更深的黑色）
        let barrel_metal = materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.08, 0.10),
            metallic: 0.95,
            perceptual_roughness: 0.2,
            ..default()
        });
        // 握把（黑色塑膠/橡膠）
        let grip_plastic = materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 0.1),
            metallic: 0.0,
            perceptual_roughness: 0.8,
            ..default()
        });
        // 木質槍托
        let wood = materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.2, 0.1),
            metallic: 0.0,
            perceptual_roughness: 0.6,
            ..default()
        });
        // 彈匣（深色金屬）
        let mag_metal = materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.12, 0.14),
            metallic: 0.7,
            perceptual_roughness: 0.5,
            ..default()
        });

        Self {
            // ========================================
            // 棍棒（木製棍棒）
            // ========================================
            staff: WeaponModelData {
                parts: vec![
                    // 棍身（主體）
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.025, 0.9)),
                        material: wood.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.0)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                    // 握把纏繞
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.028, 0.15)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.35)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.0, 0.45),
                hand_offset: Vec3::new(0.0, 0.0, -0.35),
                hand_rotation: Quat::from_rotation_x(-0.3),
            },

            // ========================================
            // 刀（戰術刀）
            // ========================================
            knife: WeaponModelData {
                parts: vec![
                    // 刀刃
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.005, 0.025, 0.18)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.12),
                    },
                    // 刀背（較厚）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.008, 0.015, 0.16)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.012, 0.11),
                    },
                    // 護手
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.04, 0.012, 0.015)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.02),
                    },
                    // 握把
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.022, 0.028, 0.10)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.04),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.0, 0.22),
                hand_offset: Vec3::new(0.0, 0.02, -0.04),
                hand_rotation: Quat::from_rotation_x(-0.1),
            },

            // ========================================
            // 手槍（Glock 風格）
            // ========================================
            pistol: WeaponModelData {
                parts: vec![
                    // 滑套（上部）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.028, 0.032, 0.16)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.016, 0.02),
                    },
                    // 槍身/握把框架
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.026, 0.08, 0.10)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.04, -0.01),
                    },
                    // 扳機護弓
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.022, 0.015, 0.04)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.008, 0.03),
                    },
                    // 槍口
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.006, 0.02)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.016, 0.09)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.016, 0.10),
                hand_offset: Vec3::new(0.0, 0.04, 0.0),
                hand_rotation: Quat::from_rotation_x(-0.2),
            },

            // ========================================
            // 衝鋒槍（UZI/MP5 風格）
            // ========================================
            smg: WeaponModelData {
                parts: vec![
                    // 機匣（主體）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.045, 0.06, 0.22)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.05),
                    },
                    // 槍管
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.012, 0.15)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.01, 0.20)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                    // 握把
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.032, 0.09, 0.04)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.06, 0.0)
                            .with_rotation(Quat::from_rotation_x(0.2)),
                    },
                    // 彈匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.025, 0.12, 0.03)),
                        material: mag_metal.clone(),
                        transform: Transform::from_xyz(0.0, -0.08, 0.06),
                    },
                    // 摺疊槍托（簡化）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.02, 0.04, 0.12)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.10),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.01, 0.28),
                hand_offset: Vec3::new(0.0, 0.06, -0.02),
                hand_rotation: Quat::from_rotation_x(-0.15),
            },

            // ========================================
            // 霰彈槍（Remington 870 風格）
            // ========================================
            shotgun: WeaponModelData {
                parts: vec![
                    // 機匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.045, 0.055, 0.18)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    },
                    // 槍管（粗）
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.018, 0.45)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.01, 0.28)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                    // 泵動護木
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.038, 0.045, 0.12)),
                        material: wood.clone(),
                        transform: Transform::from_xyz(0.0, -0.015, 0.18),
                    },
                    // 握把
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.035, 0.08, 0.045)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.055, -0.02)
                            .with_rotation(Quat::from_rotation_x(0.25)),
                    },
                    // 槍托
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.04, 0.06, 0.22)),
                        material: wood.clone(),
                        transform: Transform::from_xyz(0.0, -0.01, -0.18),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.01, 0.52),
                hand_offset: Vec3::new(0.0, 0.055, -0.05),
                hand_rotation: Quat::from_rotation_x(-0.1),
            },

            // ========================================
            // 步槍（M4/AR-15 風格）
            // ========================================
            rifle: WeaponModelData {
                parts: vec![
                    // 上機匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.04, 0.05, 0.25)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.01, 0.05),
                    },
                    // 下機匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.038, 0.045, 0.15)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, -0.02, 0.0),
                    },
                    // 槍管
                    WeaponPart {
                        mesh: meshes.add(Cylinder::new(0.01, 0.35)),
                        material: barrel_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.015, 0.32)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    },
                    // 護木
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.035, 0.04, 0.18)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.22),
                    },
                    // 握把
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.03, 0.08, 0.04)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, -0.06, -0.02)
                            .with_rotation(Quat::from_rotation_x(0.3)),
                    },
                    // 彈匣
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.022, 0.10, 0.035)),
                        material: mag_metal.clone(),
                        transform: Transform::from_xyz(0.0, -0.07, 0.04),
                    },
                    // 槍托（伸縮）
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.035, 0.05, 0.16)),
                        material: grip_plastic.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.15),
                    },
                    // 提把/瞄準鏡座
                    WeaponPart {
                        mesh: meshes.add(Cuboid::new(0.025, 0.025, 0.08)),
                        material: gun_metal.clone(),
                        transform: Transform::from_xyz(0.0, 0.045, 0.08),
                    },
                ],
                muzzle_offset: Vec3::new(0.0, 0.015, 0.50),
                hand_offset: Vec3::new(0.0, 0.06, -0.08),
                hand_rotation: Quat::from_rotation_x(-0.08),
            },
        }
    }

    /// 根據武器類型取得模型數據
    pub fn get(&self, weapon_type: WeaponType) -> Option<&WeaponModelData> {
        match weapon_type {
            WeaponType::Fist => None, // 拳頭無模型
            WeaponType::Staff => Some(&self.staff),
            WeaponType::Knife => Some(&self.knife),
            WeaponType::Pistol => Some(&self.pistol),
            WeaponType::SMG => Some(&self.smg),
            WeaponType::Shotgun => Some(&self.shotgun),
            WeaponType::Rifle => Some(&self.rifle),
            WeaponType::SniperRifle => Some(&self.rifle), // 暫用步槍模型
            WeaponType::RPG => Some(&self.rifle),         // 暫用步槍模型
        }
    }
}
