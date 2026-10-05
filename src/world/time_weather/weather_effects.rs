//! 天氣效果系統（雨、積水、閃電；天空與霧在 `sky.rs`）

use bevy::prelude::*;

use crate::core::{WeatherState, WeatherType};

// ============================================================================
// 天氣系統
// ============================================================================
/// 雨滴組件
#[derive(Component)]
pub struct RainDrop {
    pub velocity: Vec3,
    pub lifetime: f32,
}

/// 天氣過渡更新
pub fn update_weather_transition(time: Res<Time>, mut weather: ResMut<WeatherState>) {
    weather.update_transition(time.delta_secs());
}

/// 天氣切換輸入（F1 鍵）
pub fn weather_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut weather: ResMut<WeatherState>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        let next_weather = weather.weather_type.next();
        weather.start_transition(next_weather);
        info!(
            "🌤️ 天氣切換中: {} → {}",
            weather.weather_type.name(),
            next_weather.name()
        );
    }
}

/// 雨滴標記組件（用於識別雨滴父容器）
#[derive(Component)]
pub struct RainSystem;

/// 生成雨滴系統
pub fn spawn_rain_drops(
    mut commands: Commands,
    weather: Res<WeatherState>,
    rain_query: Query<Entity, With<RainSystem>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let should_rain = weather.weather_type.has_rain()
        || (weather.is_transitioning && weather.target_weather.has_rain());

    // 清理邏輯移至 cleanup_rain 系統，避免重複 despawn
    if !should_rain {
        return;
    }

    // 如果已經有雨滴系統，不重複生成
    if !rain_query.is_empty() {
        return;
    }

    // 生成雨滴系統（暴風雨時更大的雨滴）
    let rain_intensity = if matches!(weather.weather_type, WeatherType::Stormy) {
        "⛈️ 暴風雨"
    } else {
        "🌧️"
    };
    info!("{} 開始下雨...", rain_intensity);

    // 雨滴材質（半透明藍白色）
    let rain_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.8, 0.95, 0.4),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    // 雨滴網格（細長圓柱）
    let rain_mesh = meshes.add(Cylinder::new(0.02, 0.5));

    // 生成 300 個雨滴
    commands
        .spawn((
            RainSystem,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            Name::new("RainSystem"),
        ))
        .with_children(|parent| {
            use rand::Rng;
            let mut rng = rand::rng();
            for _ in 0..300 {
                let x = rng.random::<f32>() * 200.0 - 100.0;
                let y = rng.random::<f32>() * 50.0 + 10.0;
                let z = rng.random::<f32>() * 200.0 - 100.0;

                parent.spawn((
                    Mesh3d(rain_mesh.clone()),
                    MeshMaterial3d(rain_mat.clone()),
                    Transform::from_xyz(x, y, z).with_rotation(Quat::from_rotation_x(-0.1)), // 稍微傾斜
                    GlobalTransform::default(),
                    RainDrop {
                        velocity: Vec3::new(
                            rng.random::<f32>() * 2.0 - 1.0,    // 風的影響
                            -20.0 - rng.random::<f32>() * 10.0, // 下落速度
                            rng.random::<f32>() * 2.0 - 1.0,
                        ),
                        lifetime: rng.random::<f32>() * 3.0,
                    },
                ));
            }
        });
}

/// 更新雨滴位置
pub fn update_rain_drops(
    time: Res<Time>,
    weather: Res<WeatherState>,
    mut rain_query: Query<(&mut Transform, &mut RainDrop)>,
) {
    use rand::Rng;

    if !weather.weather_type.has_rain() {
        return;
    }

    let dt = time.delta_secs();
    let mut rng = rand::rng();

    for (mut transform, mut drop) in &mut rain_query {
        // 更新位置
        transform.translation += drop.velocity * dt;

        // 重置超出範圍的雨滴
        if transform.translation.y < 0.0 {
            transform.translation.x = rng.random::<f32>() * 200.0 - 100.0;
            transform.translation.y = 50.0 + rng.random::<f32>() * 10.0;
            transform.translation.z = rng.random::<f32>() * 200.0 - 100.0;
            drop.lifetime = rng.random::<f32>() * 3.0;
        }
    }
}

/// 清理雨滴系統
pub fn cleanup_rain(
    mut commands: Commands,
    weather: Res<WeatherState>,
    rain_system_query: Query<Entity, With<RainSystem>>,
) {
    // 只有在天氣不是雨天且不在過渡到雨天時才清理
    let should_rain = weather.weather_type.has_rain()
        || (weather.is_transitioning && weather.target_weather.has_rain());

    if !should_rain {
        for entity in rain_system_query.iter() {
            // Bevy 0.17: get_entity 返回 Option，安全處理已不存在的實體
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
                info!("🌤️ 雨停了");
            }
        }
    }
}

// ============================================================================
// GTA 5 風格天氣增強：雨水積水 + 雷暴閃電
// ============================================================================

/// 雨水積水組件
#[derive(Component)]
pub struct RainPuddle {
    /// 當前生命時間
    pub lifetime: f32,
    /// 最大生命時間（雨停後多久消失）
    pub max_lifetime: f32,
    /// 水坑大小
    pub size: f32,
}

impl Default for RainPuddle {
    fn default() -> Self {
        Self {
            lifetime: 0.0,
            max_lifetime: 30.0, // 雨停後 30 秒消失
            size: 2.0,
        }
    }
}

/// 積水系統標記
#[derive(Component)]
pub struct PuddleSystem;

/// 閃電狀態資源
#[derive(Resource)]
pub struct LightningState {
    /// 下次閃電時間
    pub next_flash_time: f32,
    /// 閃電持續時間
    pub flash_duration: f32,
    /// 當前閃電進度 (0.0 = 無閃電, 1.0 = 最亮)
    pub flash_intensity: f32,
    /// 是否正在閃電中
    pub is_flashing: bool,
    /// 閃電間隔範圍（秒）
    pub min_interval: f32,
    pub max_interval: f32,
}

impl Default for LightningState {
    fn default() -> Self {
        Self {
            next_flash_time: 5.0, // 5 秒後第一次閃電
            flash_duration: 0.15, // 閃電持續 0.15 秒
            flash_intensity: 0.0,
            is_flashing: false,
            min_interval: 8.0,  // 最少 8 秒一次
            max_interval: 20.0, // 最多 20 秒一次
        }
    }
}

/// 生成雨水積水系統
pub fn spawn_rain_puddles(
    mut commands: Commands,
    weather: Res<WeatherState>,
    puddle_query: Query<Entity, With<PuddleSystem>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let should_have_puddles = weather.weather_type.has_rain()
        || (weather.is_transitioning && weather.target_weather.has_rain());

    // 如果不是雨天，不生成水坑
    if !should_have_puddles {
        return;
    }

    // 如果已經有水坑系統，不重複生成
    if !puddle_query.is_empty() {
        return;
    }

    info!("💧 生成雨水積水...");

    // 水坑材質（半透明反射）
    let puddle_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.25, 0.35, 0.6), // 深藍灰色
        alpha_mode: AlphaMode::Blend,
        metallic: 0.9,             // 高金屬度模擬反射
        perceptual_roughness: 0.1, // 光滑表面
        reflectance: 0.8,          // 高反射率
        ..default()
    });

    // 水坑網格（扁平圓柱）
    let puddle_mesh = meshes.add(Cylinder::new(1.0, 0.02));

    // 生成水坑系統
    commands
        .spawn((
            PuddleSystem,
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            Name::new("PuddleSystem"),
        ))
        .with_children(|parent| {
            use rand::Rng;
            let mut rng = rand::rng();

            // 在徒步區和道路上生成水坑
            // 徒步區: X ∈ [-30, 30], Z ∈ [-50, 20]
            // 道路: 中華路 X=75, 西寧南路 X=-50
            let puddle_positions = [
                // 徒步區水坑
                Vec3::new(5.0, 0.01, -10.0),
                Vec3::new(-8.0, 0.01, 5.0),
                Vec3::new(15.0, 0.01, -25.0),
                Vec3::new(-12.0, 0.01, 10.0),
                Vec3::new(0.0, 0.01, -35.0),
                Vec3::new(20.0, 0.01, 0.0),
                // 道路水坑
                Vec3::new(70.0, 0.01, 20.0),
                Vec3::new(80.0, 0.01, -30.0),
                Vec3::new(-48.0, 0.01, 15.0),
                Vec3::new(-52.0, 0.01, -20.0),
            ];

            for base_pos in puddle_positions {
                // 隨機偏移
                let offset = Vec3::new(
                    rng.random::<f32>() * 4.0 - 2.0,
                    0.0,
                    rng.random::<f32>() * 4.0 - 2.0,
                );
                let pos = base_pos + offset;

                // 隨機大小
                let size = 1.5 + rng.random::<f32>() * 2.0;
                let scale = Vec3::new(size, 1.0, size * (0.7 + rng.random::<f32>() * 0.6));

                parent.spawn((
                    Mesh3d(puddle_mesh.clone()),
                    MeshMaterial3d(puddle_mat.clone()),
                    Transform::from_translation(pos).with_scale(scale),
                    GlobalTransform::default(),
                    RainPuddle {
                        size,
                        ..Default::default()
                    },
                ));
            }
        });
}

/// 處理水坑乾涸邏輯，回傳是否應該移除
fn handle_puddle_drying(puddle: &mut RainPuddle, transform: &mut Transform, dt: f32) -> bool {
    puddle.lifetime += dt;

    // 根據生命時間縮小水坑
    let half_lifetime = puddle.max_lifetime * 0.5;
    if puddle.lifetime > half_lifetime {
        let shrink_progress = (puddle.lifetime - half_lifetime) / half_lifetime;
        let scale_factor = (1.0 - shrink_progress).max(0.1);
        transform.scale.x = puddle.size * scale_factor;
        transform.scale.z = puddle.size * scale_factor * 0.8;
    }

    // 回傳是否應該消失
    puddle.lifetime >= puddle.max_lifetime
}

/// 清理水坑父實體
fn cleanup_puddle_system(
    commands: &mut Commands,
    puddle_system_query: &Query<Entity, With<PuddleSystem>>,
) {
    for entity in puddle_system_query.iter() {
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn();
            info!("💧 水坑已乾涸");
        }
    }
}

/// 更新雨水積水（雨停後漸漸消失）
pub fn update_rain_puddles(
    mut commands: Commands,
    time: Res<Time>,
    weather: Res<WeatherState>,
    mut puddle_query: Query<(Entity, &mut RainPuddle, &mut Transform)>,
    puddle_system_query: Query<Entity, With<PuddleSystem>>,
) {
    let is_raining = weather.weather_type.has_rain();
    let dt = time.delta_secs();

    for (entity, mut puddle, mut transform) in &mut puddle_query {
        if is_raining {
            puddle.lifetime = 0.0;
        } else if handle_puddle_drying(&mut puddle, &mut transform, dt) {
            if let Ok(mut entity_commands) = commands.get_entity(entity) {
                entity_commands.despawn();
            }
        }
    }

    // 如果所有水坑都消失了，清理父實體
    if puddle_query.is_empty() && !is_raining {
        cleanup_puddle_system(&mut commands, &puddle_system_query);
    }
}

/// 閃電更新系統
pub fn update_lightning(
    time: Res<Time>,
    weather: Res<WeatherState>,
    mut lightning: ResMut<LightningState>,
    mut ambient: ResMut<AmbientLight>,
) {
    use rand::Rng;

    // 只在雨天/暴風雨有閃電
    if !weather.weather_type.has_rain() {
        lightning.is_flashing = false;
        lightning.flash_intensity = 0.0;
        return;
    }

    let current_time = time.elapsed_secs();
    let dt = time.delta_secs();

    // 檢查是否該閃電了
    if !lightning.is_flashing && current_time >= lightning.next_flash_time {
        // 開始閃電
        lightning.is_flashing = true;
        lightning.flash_intensity = 1.0;

        // 計算下次閃電時間
        let mut rng = rand::rng();
        let interval = lightning.min_interval
            + rng.random::<f32>() * (lightning.max_interval - lightning.min_interval);
        lightning.next_flash_time = current_time + interval;

        info!("⚡ 閃電！");
    }

    // 更新閃電強度
    if lightning.is_flashing {
        // 快速衰減
        lightning.flash_intensity -= dt / lightning.flash_duration;

        if lightning.flash_intensity <= 0.0 {
            lightning.flash_intensity = 0.0;
            lightning.is_flashing = false;
        }

        // 應用到環境光（閃電時瞬間變亮）
        let flash_boost = lightning.flash_intensity * 2.0;
        ambient.brightness += flash_boost * 500.0;
    }
}

/// 閃電視覺效果系統
/// 在閃電時瞬間增亮整個場景
pub fn lightning_visual_effect(
    lightning: Res<LightningState>,
    mut sun_query: Query<&mut DirectionalLight>,
) {
    if !lightning.is_flashing {
        return;
    }

    // 閃電時太陽光瞬間增強
    for mut sun in &mut sun_query {
        sun.illuminance += lightning.flash_intensity * 30000.0;
    }
}
