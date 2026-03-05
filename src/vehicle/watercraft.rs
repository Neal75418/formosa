//! 水上載具系統
//!
//! 快艇（Speedboat）水面物理模擬，包含浮力、波浪、港口生成。

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 水面定義
// ============================================================================

/// 水面高度（全局常數）
pub const WATER_LEVEL: f32 = 0.0;

/// 水上載具類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WatercraftType {
    /// 快艇（高速、低耐久）
    Speedboat,
    /// 漁船（中速、高耐久）
    FishingBoat,
    /// 水上摩托車（極速、極低耐久）
    JetSki,
}

impl WatercraftType {
    /// 最大速度
    pub fn max_speed(&self) -> f32 {
        match self {
            WatercraftType::Speedboat => 25.0,
            WatercraftType::FishingBoat => 12.0,
            WatercraftType::JetSki => 35.0,
        }
    }

    /// 加速度
    pub fn acceleration(&self) -> f32 {
        match self {
            WatercraftType::Speedboat => 8.0,
            WatercraftType::FishingBoat => 3.0,
            WatercraftType::JetSki => 15.0,
        }
    }

    /// 轉向速度（度/秒）
    pub fn turn_speed(&self) -> f32 {
        match self {
            WatercraftType::Speedboat => 60.0,
            WatercraftType::FishingBoat => 35.0,
            WatercraftType::JetSki => 90.0,
        }
    }

    /// 最大耐久
    pub fn max_hp(&self) -> f32 {
        match self {
            WatercraftType::Speedboat => 150.0,
            WatercraftType::FishingBoat => 300.0,
            WatercraftType::JetSki => 80.0,
        }
    }

    /// 質量（影響浮力和碰撞）
    pub fn mass(&self) -> f32 {
        match self {
            WatercraftType::Speedboat => 800.0,
            WatercraftType::FishingBoat => 2000.0,
            WatercraftType::JetSki => 300.0,
        }
    }

    /// 中文名
    pub fn label(&self) -> &'static str {
        match self {
            WatercraftType::Speedboat => "快艇",
            WatercraftType::FishingBoat => "漁船",
            WatercraftType::JetSki => "水上摩托車",
        }
    }
}

// ============================================================================
// 水上載具組件
// ============================================================================

/// 水上載具組件
#[derive(Component, Clone, Debug)]
pub struct Watercraft {
    /// 載具類型
    pub craft_type: WatercraftType,
    /// 當前速度
    pub speed: f32,
    /// 當前 HP
    pub hp: f32,
    /// 是否有玩家駕駛
    pub is_occupied: bool,
    /// 油門輸入（-1.0 ~ 1.0）
    pub throttle: f32,
    /// 轉向輸入（-1.0 ~ 1.0）
    pub steering: f32,
}

impl Watercraft {
    pub fn new(craft_type: WatercraftType) -> Self {
        Self {
            craft_type,
            speed: 0.0,
            hp: craft_type.max_hp(),
            is_occupied: false,
            throttle: 0.0,
            steering: 0.0,
        }
    }

    /// 是否已損壞
    pub fn is_destroyed(&self) -> bool {
        self.hp <= 0.0
    }

    /// 受傷
    pub fn damage(&mut self, amount: f32) {
        self.hp = (self.hp - amount).max(0.0);
    }

    /// HP 百分比
    pub fn hp_ratio(&self) -> f32 {
        self.hp / self.craft_type.max_hp()
    }
}

// ============================================================================
// 浮力 & 波浪物理
// ============================================================================

/// 波浪參數
#[derive(Resource)]
pub struct WaveParams {
    /// 波浪振幅（米）
    pub amplitude: f32,
    /// 波浪頻率（Hz）
    pub frequency: f32,
    /// 波浪傳播速度
    pub wave_speed: f32,
}

impl Default for WaveParams {
    fn default() -> Self {
        Self {
            amplitude: 0.3,
            frequency: 0.5,
            wave_speed: 5.0,
        }
    }
}

impl WaveParams {
    /// 計算某位置某時刻的水面高度
    pub fn water_height(&self, position: Vec3, time: f32) -> f32 {
        let wave1 = (position.x * 0.1 + time * self.frequency).sin() * self.amplitude;
        let wave2 = (position.z * 0.08 + time * self.frequency * 0.7).sin() * self.amplitude * 0.6;
        let wave3 = ((position.x + position.z) * 0.05 + time * self.frequency * 1.3).sin()
            * self.amplitude
            * 0.3;

        WATER_LEVEL + wave1 + wave2 + wave3
    }

    /// 計算某位置的水面法線（用於船身傾斜）
    pub fn water_normal(&self, position: Vec3, time: f32) -> Vec3 {
        let dx = 0.5;
        let dz = 0.5;

        let h_center = self.water_height(position, time);
        let h_right = self.water_height(position + Vec3::new(dx, 0.0, 0.0), time);
        let h_forward = self.water_height(position + Vec3::new(0.0, 0.0, dz), time);

        let tangent_x = Vec3::new(dx, h_right - h_center, 0.0);
        let tangent_z = Vec3::new(0.0, h_forward - h_center, dz);

        tangent_z.cross(tangent_x).normalize_or_zero()
    }
}

/// 浮力數據（附加到水上載具實體）
#[allow(clippy::struct_field_names)]
#[derive(Component, Clone, Debug)]
pub struct Buoyancy {
    /// 浮力係數（越大浮力越強）
    pub buoyancy_force: f32,
    /// 水阻力係數
    pub drag: f32,
    /// 當前沉沒深度（正=在水面下）
    pub submersion_depth: f32,
}

impl Default for Buoyancy {
    fn default() -> Self {
        Self {
            buoyancy_force: 15.0,
            drag: 2.0,
            submersion_depth: 0.0,
        }
    }
}

// ============================================================================
// 港口
// ============================================================================

/// 港口/碼頭標記（水上載具生成點）
#[derive(Component, Clone, Debug)]
pub struct Harbor {
    /// 港口名稱
    pub name: String,
    /// 位置
    pub position: Vec3,
    /// 可用船隻類型
    pub available_crafts: Vec<WatercraftType>,
}

/// 預定義港口列表
pub fn create_harbors() -> Vec<Harbor> {
    vec![
        Harbor {
            name: "西門碼頭".to_string(),
            position: Vec3::new(-80.0, WATER_LEVEL, 0.0),
            available_crafts: vec![WatercraftType::Speedboat, WatercraftType::JetSki],
        },
        Harbor {
            name: "漁人碼頭".to_string(),
            position: Vec3::new(90.0, WATER_LEVEL, -50.0),
            available_crafts: vec![WatercraftType::FishingBoat, WatercraftType::Speedboat],
        },
        Harbor {
            name: "河濱碼頭".to_string(),
            position: Vec3::new(0.0, WATER_LEVEL, 90.0),
            available_crafts: vec![WatercraftType::JetSki],
        },
    ]
}

// ============================================================================
// 系統
// ============================================================================

/// 水上載具浮力系統
pub fn watercraft_buoyancy_system(
    time: Res<Time>,
    wave_params: Res<WaveParams>,
    mut query: Query<(&mut Transform, &mut Buoyancy, &Watercraft)>,
) {
    let t = time.elapsed_secs();
    let dt = time.delta_secs();

    for (mut transform, mut buoyancy, craft) in &mut query {
        let water_h = wave_params.water_height(transform.translation, t);
        buoyancy.submersion_depth = water_h - transform.translation.y;

        // 浮力：當船低於水面時向上推
        if buoyancy.submersion_depth > 0.0 {
            let buoyancy_accel =
                buoyancy.buoyancy_force * buoyancy.submersion_depth - buoyancy.drag * dt;
            transform.translation.y += buoyancy_accel * dt;
        } else {
            // 重力：船在水面上方時下墜
            transform.translation.y -= 9.81 * dt * 0.5;
        }

        // 保持在水面附近（防止跳太高或沉太深）
        let max_height = water_h + 1.0;
        let min_height = water_h - 0.5;
        transform.translation.y = transform.translation.y.clamp(min_height, max_height);

        // 船身跟隨水面法線傾斜
        if craft.is_occupied {
            let normal = wave_params.water_normal(transform.translation, t);
            let target_rotation = Quat::from_rotation_arc(Vec3::Y, normal);
            transform.rotation = transform.rotation.slerp(target_rotation, dt * 3.0);
        }
    }
}

/// 水上載具移動系統
pub fn watercraft_movement_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Watercraft)>,
) {
    let dt = time.delta_secs();

    for (mut transform, mut craft) in &mut query {
        if !craft.is_occupied || craft.is_destroyed() {
            // 未駕駛或已損壞：自然減速
            craft.speed *= (1.0 - 2.0 * dt).max(0.0);
            continue;
        }

        let max_speed = craft.craft_type.max_speed();
        let accel = craft.craft_type.acceleration();
        let turn_speed = craft.craft_type.turn_speed();

        // 加速/減速
        let target_speed = craft.throttle * max_speed;
        if craft.throttle.abs() > 0.01 {
            craft.speed += (target_speed - craft.speed).signum() * accel * dt;
            craft.speed = craft.speed.clamp(-max_speed * 0.3, max_speed);
        } else {
            // 自然減速
            craft.speed *= (1.0 - 1.5 * dt).max(0.0);
        }

        // 轉向（只在有速度時生效）
        if craft.speed.abs() > 0.5 {
            let turn_amount = craft.steering * turn_speed * dt;
            transform.rotate_y(-turn_amount.to_radians());
        }

        // 移動
        let forward = transform.forward();
        transform.translation += *forward * craft.speed * dt;
    }
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watercraft_types() {
        assert!(WatercraftType::Speedboat.max_speed() > WatercraftType::FishingBoat.max_speed());
        assert!(WatercraftType::JetSki.max_speed() > WatercraftType::Speedboat.max_speed());
    }

    #[test]
    fn watercraft_damage() {
        let mut craft = Watercraft::new(WatercraftType::Speedboat);
        assert!(!craft.is_destroyed());
        assert!((craft.hp_ratio() - 1.0).abs() < f32::EPSILON);

        craft.damage(100.0);
        assert!(!craft.is_destroyed());
        assert!((craft.hp - 50.0).abs() < f32::EPSILON);

        craft.damage(100.0); // 超出剩餘 HP
        assert!(craft.is_destroyed());
        assert!(craft.hp.abs() < f32::EPSILON);
    }

    #[test]
    fn wave_height_varies_with_time() {
        let waves = WaveParams::default();
        let pos = Vec3::new(10.0, 0.0, 10.0);

        let h1 = waves.water_height(pos, 0.0);
        let h2 = waves.water_height(pos, 1.0);
        let h3 = waves.water_height(pos, 2.0);

        // 不同時間點的水面高度應該不同
        assert!((h1 - h2).abs() > 0.001 || (h2 - h3).abs() > 0.001);
    }

    #[test]
    fn wave_height_varies_with_position() {
        let waves = WaveParams::default();
        let t = 0.0;

        let h1 = waves.water_height(Vec3::ZERO, t);
        let h2 = waves.water_height(Vec3::new(50.0, 0.0, 0.0), t);

        // 不同位置的水面高度應該不同
        assert!((h1 - h2).abs() > 0.001);
    }

    #[test]
    fn wave_normal_roughly_up() {
        let waves = WaveParams::default();
        let normal = waves.water_normal(Vec3::ZERO, 0.0);

        // 法線應大致朝上
        assert!(
            normal.y > 0.5,
            "Normal Y should be positive, got {}",
            normal.y
        );
    }

    #[test]
    fn buoyancy_default_values() {
        let b = Buoyancy::default();
        assert!(b.buoyancy_force > 0.0);
        assert!(b.drag > 0.0);
        assert!(b.submersion_depth.abs() < f32::EPSILON);
    }

    #[test]
    fn harbors_created() {
        let harbors = create_harbors();
        assert!(harbors.len() >= 3);
        for harbor in &harbors {
            assert!(!harbor.name.is_empty());
            assert!(!harbor.available_crafts.is_empty());
        }
    }

    #[test]
    fn watercraft_new_full_hp() {
        let craft = Watercraft::new(WatercraftType::JetSki);
        assert!((craft.hp - 80.0).abs() < f32::EPSILON);
        assert!(!craft.is_occupied);
        assert!(craft.speed.abs() < f32::EPSILON);
    }

    #[test]
    fn watercraft_type_labels() {
        assert_eq!(WatercraftType::Speedboat.label(), "快艇");
        assert_eq!(WatercraftType::FishingBoat.label(), "漁船");
        assert_eq!(WatercraftType::JetSki.label(), "水上摩托車");
    }

    #[test]
    fn watercraft_type_mass() {
        assert!(WatercraftType::FishingBoat.mass() > WatercraftType::Speedboat.mass());
        assert!(WatercraftType::Speedboat.mass() > WatercraftType::JetSki.mass());
    }
}
