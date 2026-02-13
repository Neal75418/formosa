//! 車輛視覺效果組件（GTA 5 風格）

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use crate::core::{lifetime_fade_alpha, lifetime_linear_alpha};

/// Vehicle visual root for applying roll/pitch/lean without affecting physics.
#[derive(Component)]
pub struct VehicleVisualRoot;

/// 輪胎痕跡組件
/// 漂移/急煞時在地面留下的胎痕
#[derive(Component)]
pub struct TireTrack {
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間（痕跡完全消失）
    pub max_lifetime: f32,
    /// 痕跡點列表 (位置, 寬度)
    pub points: Vec<(Vec3, f32)>,
}

impl Default for TireTrack {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 10.0,  // 10 秒後消失
            points: Vec::new(),
        }
    }
}

impl TireTrack {
    /// 建立新實例
    pub fn new(points: Vec<(Vec3, f32)>) -> Self {
        Self {
            points,
            ..Default::default()
        }
    }

    /// 計算當前透明度
    pub fn alpha(&self) -> f32 {
        lifetime_fade_alpha(self.lifetime, self.max_lifetime, 0.7)
    }
}

/// 漂移煙霧粒子組件
#[derive(Component)]
pub struct DriftSmoke {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 初始縮放
    pub initial_scale: f32,
}

impl DriftSmoke {
    /// 建立新實例
    pub fn new(velocity: Vec3, max_lifetime: f32) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime,
            initial_scale: 0.3,
        }
    }

    /// 計算當前透明度（煙霧會擴散變淡）
    pub fn alpha(&self) -> f32 {
        lifetime_linear_alpha(self.lifetime, self.max_lifetime)
    }

    /// 計算當前縮放（煙霧會擴散變大）
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.initial_scale * (1.0 + progress * 2.0)  // 最終是初始的 3 倍大
    }
}

/// 氮氣火焰粒子組件
#[derive(Component)]
pub struct NitroFlame {
    /// 粒子速度
    pub velocity: Vec3,
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間
    pub max_lifetime: f32,
    /// 初始縮放
    pub initial_scale: f32,
}

impl NitroFlame {
    /// 建立新實例
    pub fn new(velocity: Vec3) -> Self {
        Self {
            velocity,
            lifetime: 0.0,
            max_lifetime: 0.15,  // 火焰粒子生命較短
            initial_scale: 0.2,
        }
    }

    /// 計算當前顏色（從藍白漸變到橙紅）
    pub fn color(&self) -> Color {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if progress < 0.3 {
            // 藍白色（核心高溫）
            Color::srgba(0.8, 0.9, 1.0, 1.0 - progress)
        } else if progress < 0.6 {
            // 黃橙色（中間）
            Color::srgba(1.0, 0.8, 0.3, 1.0 - progress)
        } else {
            // 橙紅色（外焰）
            Color::srgba(1.0, 0.4, 0.1, (1.0 - progress) * 0.5)
        }
    }

    /// 計算當前縮放（火焰會逐漸縮小消散）
    pub fn scale(&self) -> f32 {
        let progress = if self.max_lifetime > 0.0 {
            (self.lifetime / self.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.initial_scale * (1.0 - progress * 0.5)
    }
}

/// 車輛視覺效果資源（預生成的 mesh 和 material）
#[derive(Resource)]
pub struct VehicleEffectVisuals {
    /// 煙霧粒子 mesh (球體)
    pub smoke_mesh: Handle<Mesh>,
    /// 煙霧粒子材質 (半透明灰白色)
    pub smoke_material: Handle<StandardMaterial>,
    /// 輪胎痕跡材質 (深色)
    pub tire_track_material: Handle<StandardMaterial>,
    /// 輪胎痕跡 mesh (薄平面)
    pub tire_track_mesh: Handle<Mesh>,
    /// 氮氣火焰 mesh (拉長的球體模擬火焰)
    pub nitro_flame_mesh: Handle<Mesh>,
    /// 氮氣火焰材質 (發光藍白色)
    pub nitro_flame_material: Handle<StandardMaterial>,
}

impl VehicleEffectVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            smoke_mesh: meshes.add(Sphere::new(0.5)),
            smoke_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.8, 0.8, 0.5),  // 灰白色半透明
                alpha_mode: AlphaMode::Blend,
                unlit: true,  // 不受光照影響
                ..default()
            }),
            tire_track_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.1, 0.1, 0.1, 0.8),  // 深色輪胎痕
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,  // 雙面可見
                ..default()
            }),
            tire_track_mesh: meshes.add(Cuboid::new(0.3, 0.01, 0.5)),  // 薄平面
            // 氮氣火焰：拉長的球體
            nitro_flame_mesh: meshes.add(Sphere::new(0.3)),
            nitro_flame_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.9, 1.0, 0.9),  // 藍白色
                emissive: LinearRgba::rgb(5.0, 6.0, 8.0),  // 強發光
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            }),
        }
    }
}

/// 車輛效果追蹤器
#[derive(Resource, Default)]
pub struct VehicleEffectTracker {
    /// 當前煙霧粒子數量
    pub smoke_count: usize,
    /// 最大煙霧粒子數量
    pub max_smoke_count: usize,
    /// 當前輪胎痕跡數量
    pub track_count: usize,
    /// 最大輪胎痕跡數量
    pub max_track_count: usize,
    /// 上次生成煙霧的時間
    pub last_smoke_spawn: f32,
    /// 煙霧生成間隔（秒）
    pub smoke_spawn_interval: f32,
    /// 上次生成輪胎痕跡的時間
    pub last_track_spawn: f32,
    /// 輪胎痕跡生成間隔（秒）
    pub track_spawn_interval: f32,
}

impl VehicleEffectTracker {
    /// 建立新實例
    pub fn new() -> Self {
        Self {
            smoke_count: 0,
            max_smoke_count: 50,
            track_count: 0,
            max_track_count: 30,
            last_smoke_spawn: 0.0,
            smoke_spawn_interval: 0.05,  // 每 0.05 秒生成一批煙霧
            last_track_spawn: 0.0,
            track_spawn_interval: 0.1,  // 每 0.1 秒生成一段痕跡
        }
    }
}

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- DriftSmoke ---

    #[test]
    fn drift_smoke_alpha_and_scale() {
        let mut s = DriftSmoke::new(Vec3::Y, 2.0);
        assert!((s.alpha() - 1.0).abs() < f32::EPSILON);
        assert!((s.scale() - 0.3).abs() < 0.01);
        s.lifetime = 1.0;
        assert!((s.alpha() - 0.5).abs() < f32::EPSILON);
        assert!((s.scale() - 0.3 * 2.0).abs() < 0.01);
    }

    // --- NitroFlame ---

    #[test]
    fn nitro_flame_scale_shrinks() {
        let mut f = NitroFlame::new(Vec3::Z);
        let s0 = f.scale();
        f.lifetime = 0.15;
        let s1 = f.scale();
        assert!(s0 > s1);
    }
}
