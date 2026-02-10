//! 爆炸物視覺效果系統
//!
//! 爆炸效果、衝擊波、煙霧粒子、火焰粒子、煙霧發射器、投擲預覽。

use bevy::prelude::*;

use super::*;

// ============================================================================
// 粒子生成輔助函數
// ============================================================================

/// 生成煙霧粒子（輔助函數）
/// 每個粒子使用獨立材質避免共享修改問題
pub(super) fn spawn_smoke_particles(
    commands: &mut Commands,
    visuals: &ExplosiveVisuals,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    radius: f32,
    count: usize,
) {
    use rand::Rng;
    let mut rng = rand::rng();

    // 取得基礎材質用於複製
    let base_material = materials.get(&visuals.smoke_material).cloned();

    for _ in 0..count {
        // 隨機位置（在爆炸半徑內）
        let offset = Vec3::new(
            rng.random::<f32>() * radius - radius * 0.5,
            rng.random::<f32>() * radius * 0.5, // 偏上方
            rng.random::<f32>() * radius - radius * 0.5,
        );

        // 隨機向上速度（帶一點水平擴散）
        let velocity = Vec3::new(
            rng.random::<f32>() * 2.0 - 1.0,
            3.0 + rng.random::<f32>() * 2.0, // 主要向上
            rng.random::<f32>() * 2.0 - 1.0,
        );

        let lifetime = 2.0 + rng.random::<f32>() * 1.5;

        // 每個粒子創建獨立材質
        let particle_material = base_material
            .clone()
            .map(|m| materials.add(m))
            .unwrap_or_else(|| visuals.smoke_material.clone());

        commands.spawn((
            Mesh3d(visuals.smoke_mesh.clone()),
            MeshMaterial3d(particle_material),
            Transform::from_translation(position + offset)
                .with_scale(Vec3::splat(0.5 + rng.random::<f32>() * 0.3)),
            SmokeParticle::new(velocity, lifetime),
        ));
    }
}

/// 生成火焰粒子（輔助函數）
/// 每個粒子使用獨立材質避免共享修改問題
pub(super) fn spawn_fire_particles(
    commands: &mut Commands,
    visuals: &ExplosiveVisuals,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    radius: f32,
    count: usize,
) {
    use rand::Rng;
    let mut rng = rand::rng();

    // 取得基礎材質用於複製
    let base_material = materials.get(&visuals.fire_particle_material).cloned();

    for _ in 0..count {
        // 隨機位置（在火焰區域內）
        let offset = Vec3::new(
            rng.random::<f32>() * radius - radius * 0.5,
            rng.random::<f32>() * 0.5, // 貼近地面
            rng.random::<f32>() * radius - radius * 0.5,
        );

        let lifetime = 0.5 + rng.random::<f32>() * 0.5; // 火焰粒子短命

        // 每個粒子創建獨立材質
        let particle_material = base_material
            .clone()
            .map(|m| materials.add(m))
            .unwrap_or_else(|| visuals.fire_particle_material.clone());

        commands.spawn((
            Mesh3d(visuals.fire_particle_mesh.clone()),
            MeshMaterial3d(particle_material),
            Transform::from_translation(position + offset)
                .with_scale(Vec3::splat(0.3 + rng.random::<f32>() * 0.2)),
            FireParticle::new(lifetime),
        ));
    }
}

// ============================================================================
// 視覺效果更新系統
// ============================================================================

/// 爆炸效果更新系統
pub fn explosion_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut effect_query: Query<(Entity, &mut Transform, &mut ExplosionEffect)>,
) {
    for (entity, mut transform, mut effect) in &mut effect_query {
        effect.lifetime += time.delta_secs();

        // 爆炸擴散然後縮小（防止除零）
        let progress = if effect.max_lifetime > 0.0 {
            (effect.lifetime / effect.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let scale = if progress < 0.3 {
            // 快速擴張
            effect.radius * (progress / 0.3)
        } else {
            // 緩慢消失
            effect.radius * (1.0 - (progress - 0.3) / 0.7)
        };
        transform.scale = Vec3::splat(scale.max(0.01));

        if effect.lifetime >= effect.max_lifetime {
            commands.entity(entity).despawn();
        }
    }
}

/// 衝擊波效果更新系統（GTA5 風格擴散環）
pub fn shockwave_effect_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_query: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
        &mut ShockwaveEffect,
    )>,
) {
    for (entity, mut transform, material_handle, mut effect) in &mut effect_query {
        effect.lifetime += time.delta_secs();

        // 防止除零
        let progress = if effect.max_lifetime > 0.0 {
            (effect.lifetime / effect.max_lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        };

        // 線性擴張
        let scale = effect.max_radius * progress;
        // 保持環的厚度不變，只擴大半徑
        transform.scale = Vec3::new(scale.max(0.1), scale.max(0.1), 0.15);

        // 更新透明度（快速淡出）
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let alpha = effect.initial_alpha * (1.0 - progress * progress); // 二次方淡出
            material.base_color = Color::srgba(1.0, 0.95, 0.9, alpha);
            // 減弱發光
            let emissive_strength = 8.0 * (1.0 - progress);
            material.emissive = LinearRgba::new(
                emissive_strength,
                emissive_strength * 0.75,
                emissive_strength * 0.5,
                1.0,
            );
        }

        if effect.lifetime >= effect.max_lifetime {
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// 粒子更新系統
// ============================================================================

/// 煙霧粒子更新系統（GTA5 風格上升漸散煙霧）
pub fn smoke_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut smoke_query: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
        &mut SmokeParticle,
    )>,
) {
    let dt = time.delta_secs();

    for (entity, mut transform, material_handle, mut smoke) in &mut smoke_query {
        smoke.lifetime -= dt;

        if smoke.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 更新位置（向上飄動）
        transform.translation += smoke.velocity * dt;

        // 隨時間減慢速度（空氣阻力）
        // 使用 powf 確保幀率無關：0.98^60 ≈ 0.3 每秒
        smoke.velocity *= 0.98_f32.powf(dt * 60.0);

        // 計算進度（0 = 剛生成，1 = 即將消失）
        let progress = 1.0 - smoke.lifetime / smoke.max_lifetime;

        // 膨脹效果：煙霧隨時間變大
        let scale = smoke.initial_scale + (smoke.final_scale - smoke.initial_scale) * progress;
        transform.scale = Vec3::splat(scale);

        // 更新透明度（漸漸消失）
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let alpha = 0.6 * (1.0 - progress * progress); // 二次方淡出
            let gray = 0.2 + 0.1 * progress; // 顏色漸淺
            material.base_color = Color::srgba(gray, gray, gray, alpha);
        }
    }
}

/// 火焰粒子更新系統（GTA5 風格閃爍火焰）
pub fn fire_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut fire_query: Query<(
        Entity,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
        &mut FireParticle,
    )>,
) {
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    for (entity, mut transform, material_handle, mut fire) in &mut fire_query {
        fire.lifetime -= dt;

        if fire.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // 計算進度
        let progress = 1.0 - fire.lifetime / fire.max_lifetime;

        // 閃爍效果
        let flicker = (t * fire.flicker_speed + fire.flicker_phase).sin() * 0.5 + 0.5;
        let scale_factor = 0.8 + flicker * 0.4; // 0.8 ~ 1.2

        // 火焰向上飄動並縮小
        transform.translation.y += dt * (1.0 + flicker);
        transform.translation += fire.base_offset * dt * 0.5; // 輕微水平晃動

        let scale = (1.0 - progress * 0.5) * scale_factor;
        transform.scale = Vec3::splat(scale.max(0.1));

        // 更新發光強度（閃爍）
        if let Some(material) = materials.get_mut(&material_handle.0) {
            let intensity = 20.0 * flicker * (1.0 - progress);
            material.emissive = LinearRgba::new(intensity, intensity * 0.4, intensity * 0.1, 1.0);

            // 顏色從橙黃變紅（燃燒後期）
            let r = 1.0;
            let g = 0.5 - progress * 0.3;
            let b = 0.1 - progress * 0.05;
            let alpha = 0.9 * (1.0 - progress);
            material.base_color = Color::srgba(r, g.max(0.1), b.max(0.05), alpha);
        }
    }
}

/// 煙霧發射器更新系統（持續產生煙霧）
pub fn smoke_emitter_update_system(
    mut commands: Commands,
    time: Res<Time>,
    visuals: Option<Res<ExplosiveVisuals>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut emitter_query: Query<(&Transform, &mut SmokeEmitter)>,
) {
    let Some(visuals) = visuals else {
        return;
    };
    let dt = time.delta_secs();

    for (transform, mut emitter) in &mut emitter_query {
        // 更新剩餘時間
        if emitter.remaining_time > 0.0 {
            emitter.remaining_time -= dt;
            if emitter.remaining_time <= 0.0 {
                continue; // 發射器已過期
            }
        }

        // 累積發射計時
        emitter.spawn_accumulator += dt * emitter.particles_per_second;

        // 發射新粒子
        while emitter.spawn_accumulator >= 1.0 {
            emitter.spawn_accumulator -= 1.0;
            spawn_smoke_particles(
                &mut commands,
                &visuals,
                &mut materials,
                transform.translation,
                emitter.radius,
                1,
            );
        }
    }
}

// ============================================================================
// 投擲預覽渲染
// ============================================================================

/// 投擲預覽渲染系統
pub fn throw_preview_render_system(
    mut commands: Commands,
    throw_state: Res<ThrowPreviewState>,
    visuals: Option<Res<ExplosiveVisuals>>,
    preview_query: Query<Entity, With<TrajectoryPreviewPoint>>,
) {
    // 清除舊的預覽點
    for entity in &preview_query {
        commands.entity(entity).despawn();
    }

    if !throw_state.is_previewing {
        return;
    }

    let Some(visuals) = visuals else {
        return;
    };

    // 生成新的預覽點
    for (i, &point) in throw_state.trajectory_points.iter().enumerate() {
        // 每隔幾個點顯示一個
        if i % 2 == 0 {
            commands.spawn((
                Mesh3d(visuals.trajectory_mesh.clone()),
                MeshMaterial3d(visuals.trajectory_material.clone()),
                Transform::from_translation(point),
                TrajectoryPreviewPoint,
            ));
        }
    }
}
