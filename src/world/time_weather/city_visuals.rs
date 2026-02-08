//! 城市視覺效果（霓虹燈招牌、建築窗戶）

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::core::WorldTime;
use crate::world::{BuildingWindow, NeonSign};

/// 窗戶更新計時器（效能優化：避免每幀更新）
#[derive(Resource)]
pub struct WindowUpdateTimer(pub Timer);

impl Default for WindowUpdateTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}

// ============================================================================
// 霓虹燈輔助函數
// ============================================================================
/// 計算夜晚亮度加成
#[inline]
fn get_night_boost(hour: f32) -> f32 {
    if (6.0..=18.0).contains(&hour) {
        0.8
    } else {
        1.5
    }
}

/// 計算波形閃爍值
#[inline]
fn calculate_wave(t: f32, speed: f32, phase: f32) -> f32 {
    ((t * speed + phase).sin() + 1.0) * 0.5
}

/// 計算故障燈隨機閃爍
fn calculate_broken_flicker(rng: &mut impl rand::Rng, t: f32, neon: &NeonSign) -> f32 {
    let random_flicker = if rng.random::<f32>() < 0.02 {
        rng.random::<f32>() * 0.5 // 偶爾完全熄滅
    } else {
        1.0
    };
    let wave = calculate_wave(t, neon.flicker_speed, neon.phase_offset);
    random_flicker * (1.0 - neon.flicker_amount + neon.flicker_amount * wave)
}

/// 計算霓虹燈閃爍強度
fn calculate_neon_flicker(rng: &mut impl rand::Rng, t: f32, neon: &NeonSign) -> f32 {
    if neon.is_broken {
        calculate_broken_flicker(rng, t, neon)
    } else if neon.flicker_speed > 0.0 {
        let wave = calculate_wave(t, neon.flicker_speed, neon.phase_offset);
        1.0 - neon.flicker_amount + neon.flicker_amount * wave
    } else {
        1.0
    }
}

/// 更新霓虹燈招牌閃爍效果
/// 每幀更新所有霓虹燈的發光強度
pub fn update_neon_signs(
    time: Res<Time>,
    world_time: Res<WorldTime>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    neon_query: Query<(&NeonSign, &MeshMaterial3d<StandardMaterial>)>,
) {
    let t = time.elapsed_secs();
    let night_boost = get_night_boost(world_time.hour);
    let mut rng = rand::rng();

    for (neon, material_handle) in &neon_query {
        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        let flicker = calculate_neon_flicker(&mut rng, t, neon);
        let intensity = neon.base_intensity * flicker * night_boost;
        let color = neon.color.to_linear();

        material.emissive = LinearRgba::new(
            color.red * intensity,
            color.green * intensity,
            color.blue * intensity,
            1.0,
        );
    }
}

// ============================================================================
// 建築窗戶輔助函數
// ============================================================================
/// 計算各時段窗戶點亮的基礎機率
#[inline]
fn calculate_base_lit_chance(hour: f32) -> f32 {
    match () {
        _ if (6.0..18.0).contains(&hour) => 0.1,  // 日間：10%
        _ if (18.0..20.0).contains(&hour) => 0.6, // 傍晚：60%
        _ if (0.0..2.0).contains(&hour) || (22.0..24.0).contains(&hour) => 0.2, // 深夜：20%
        _ => 0.4,                                 // 一般夜晚：40%
    }
}

/// 判斷窗戶是否應該點亮
#[inline]
fn should_window_be_lit(
    window: &BuildingWindow,
    base_chance: f32,
    shop_closed: bool,
    rng: &mut impl rand::Rng,
) -> bool {
    if window.is_shop && shop_closed {
        return false;
    }
    let effective_chance = base_chance * window.light_probability;
    rng.random::<f32>() < effective_chance
}

/// 設置窗戶發光材質
#[inline]
fn set_window_emissive(material: &mut StandardMaterial, window: &BuildingWindow, lit: bool) {
    material.emissive = if lit {
        let color = window.base_color.to_linear();
        LinearRgba::new(
            color.red * window.lit_emissive,
            color.green * window.lit_emissive,
            color.blue * window.lit_emissive,
            1.0,
        )
    } else {
        LinearRgba::new(0.0, 0.0, 0.0, 1.0)
    };
}

/// 更新建築窗戶燈光（根據時間變化）
/// 日間窗戶暗淡，夜間隨機點亮
/// 效能優化：每 5 秒更新一次（而非每幀）
pub fn update_building_windows(
    time: Res<Time>,
    mut timer: ResMut<WindowUpdateTimer>,
    world_time: Res<WorldTime>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut window_query: Query<(&mut BuildingWindow, &MeshMaterial3d<StandardMaterial>)>,
) {
    // 效能優化：僅在計時器觸發時更新
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let hour = world_time.hour;
    let base_lit_chance = calculate_base_lit_chance(hour);
    let shop_closed = (0.0..6.0).contains(&hour);
    let mut rng = rand::rng();

    for (mut window, material_handle) in window_query.iter_mut() {
        let should_be_lit = should_window_be_lit(&window, base_lit_chance, shop_closed, &mut rng);

        // 只在狀態改變時更新材質
        if window.is_lit != should_be_lit {
            window.is_lit = should_be_lit;
            if let Some(material) = materials.get_mut(&material_handle.0) {
                set_window_emissive(material, &window, should_be_lit);
            }
        }
    }
}

/// 生成霓虹燈招牌
/// 在指定位置生成一個發光的招牌
pub fn spawn_neon_sign(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    size: Vec3, // (width, height, depth)
    text: &str, // 招牌文字（目前僅用於識別）
    neon_config: NeonSign,
) {
    let color = neon_config.color;
    let intensity = neon_config.base_intensity;

    // 霓虹燈材質（發光）
    let neon_mat = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * intensity,
        ..default()
    });

    // 招牌底板（深色背景）
    let back_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.08),
        perceptual_roughness: 0.9,
        ..default()
    });

    // 生成招牌實體
    commands
        .spawn((
            Transform::from_translation(position),
            GlobalTransform::default(),
            Visibility::default(),
            Name::new(format!("NeonSign_{}", text)),
            // 招牌碰撞體
            Collider::cuboid(size.x / 2.0 + 0.1, size.y / 2.0 + 0.05, size.z / 2.0 + 0.05),
            RigidBody::Fixed,
        ))
        .with_children(|parent| {
            // 底板（放在後面）
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x + 0.2, size.y + 0.1, size.z))),
                MeshMaterial3d(back_mat),
                Transform::from_xyz(0.0, 0.0, -0.05),
                GlobalTransform::default(),
            ));

            // 發光文字區（放在前面，與底板保持足夠距離）
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(size.x, size.y, 0.05))),
                MeshMaterial3d(neon_mat),
                Transform::from_xyz(0.0, 0.0, size.z / 2.0 + 0.05),
                GlobalTransform::default(),
                neon_config,
            ));

            // 招牌光源（照亮周圍）
            parent.spawn((
                PointLight {
                    color,
                    intensity: 50000.0 * intensity / 8.0,
                    range: 15.0,
                    radius: 1.0,
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, -1.0),
                GlobalTransform::default(),
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_boost_higher_at_night() {
        let day = get_night_boost(12.0);
        let night = get_night_boost(22.0);
        assert!(night > day);
        assert!((day - 0.8).abs() < f32::EPSILON);
        assert!((night - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn calculate_wave_bounded() {
        for t in [0.0, 1.0, 2.5, 10.0] {
            let v = calculate_wave(t, 3.0, 0.5);
            assert!((0.0..=1.0).contains(&v), "wave({t}) = {v} out of [0,1]");
        }
    }

    #[test]
    fn base_lit_chance_daytime_low() {
        let day = calculate_base_lit_chance(12.0);
        let evening = calculate_base_lit_chance(19.0);
        let deep_night = calculate_base_lit_chance(1.0);
        assert!((day - 0.1).abs() < f32::EPSILON);
        assert!((evening - 0.6).abs() < f32::EPSILON);
        assert!((deep_night - 0.2).abs() < f32::EPSILON);
    }
}
