//! 警用直升機系統
//!
//! 5 星通緝時出動，追蹤玩家並使用機槍射擊。

#![allow(dead_code)] // Phase 5+ 預留功能

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::combat::{
    DamageEvent, DamageSource, Health,
    CombatVisuals, TracerStyle, spawn_bullet_tracer, spawn_muzzle_flash,
};
use crate::player::Player;
use super::WantedLevel;

// ============================================================================
// 常數
// ============================================================================

/// 直升機生成所需通緝等級
pub const HELICOPTER_SPAWN_WANTED_LEVEL: u8 = 5;
/// 最大直升機數量
pub const HELICOPTER_MAX_COUNT: usize = 2;
/// 直升機生成冷卻（秒）
pub const HELICOPTER_SPAWN_COOLDOWN: f32 = 45.0;

/// 直升機懸停高度
pub const HELICOPTER_HOVER_ALTITUDE: f32 = 40.0;
/// 直升機最大高度
pub const HELICOPTER_MAX_ALTITUDE: f32 = 80.0;
/// 直升機最小高度
pub const HELICOPTER_MIN_ALTITUDE: f32 = 25.0;
/// 直升機飛行速度
pub const HELICOPTER_SPEED: f32 = 35.0;
/// 直升機轉向速率
pub const HELICOPTER_TURN_RATE: f32 = 1.2;
/// 直升機垂直移動速度
pub const HELICOPTER_VERTICAL_SPEED: f32 = 10.0;

/// 直升機攻擊範圍
pub const HELICOPTER_ATTACK_RANGE: f32 = 50.0;
/// 直升機射擊頻率（每秒發射數）
pub const HELICOPTER_FIRE_RATE: f32 = 8.0;
/// 直升機子彈傷害
pub const HELICOPTER_BULLET_DAMAGE: f32 = 8.0;
/// 直升機生命值
pub const HELICOPTER_HEALTH: f32 = 500.0;

/// 探照燈範圍
pub const SPOTLIGHT_RANGE: f32 = 60.0;
/// 探照燈錐角（度）
pub const SPOTLIGHT_CONE_ANGLE: f32 = 25.0;

/// 主旋翼旋轉速度（度/秒）
pub const MAIN_ROTOR_SPEED: f32 = 720.0;
/// 尾旋翼旋轉速度（度/秒）
pub const TAIL_ROTOR_SPEED: f32 = 1200.0;

/// 規避時間（秒）
pub const EVADE_DURATION: f32 = 3.0;
/// 墜毀旋轉速度（度/秒）
pub const CRASH_ROTATION_SPEED: f32 = 180.0;

// ============================================================================
// 組件
// ============================================================================

/// 直升機狀態機
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HelicopterState {
    #[default]
    Approaching,  // 飛向玩家
    Hovering,     // 懸停觀察
    Pursuing,     // 追蹤移動中的玩家
    Attacking,    // 射擊玩家
    Evading,      // 規避傷害
    Crashing,     // 被擊落墜毀
}

/// 警用直升機組件
#[derive(Component)]
pub struct PoliceHelicopter {
    /// 當前狀態
    pub state: HelicopterState,
    /// 生命值
    pub health: f32,
    /// 目標高度
    pub target_altitude: f32,
    /// 射擊冷卻
    pub fire_cooldown: f32,
    /// 規避計時器
    pub evade_timer: f32,
    /// 目標位置
    pub target_position: Option<Vec3>,
    /// 懸停計時器
    pub hover_timer: f32,
    /// 搜索計時器
    pub search_timer: f32,
    /// 上次受傷時間
    pub last_hit_time: f32,
    /// 墜落速度
    pub crash_velocity: Vec3,
}

impl Default for PoliceHelicopter {
    fn default() -> Self {
        Self {
            state: HelicopterState::Approaching,
            health: HELICOPTER_HEALTH,
            target_altitude: HELICOPTER_HOVER_ALTITUDE,
            fire_cooldown: 0.0,
            evade_timer: 0.0,
            target_position: None,
            hover_timer: 0.0,
            search_timer: 0.0,
            last_hit_time: 0.0,
            crash_velocity: Vec3::ZERO,
        }
    }
}

/// 旋翼組件
#[derive(Component)]
pub struct HelicopterRotor {
    /// 旋轉速度（度/秒）
    pub rotation_speed: f32,
    /// 是否為主旋翼
    pub is_main_rotor: bool,
}

impl HelicopterRotor {
    pub fn main() -> Self {
        Self {
            rotation_speed: MAIN_ROTOR_SPEED,
            is_main_rotor: true,
        }
    }

    pub fn tail() -> Self {
        Self {
            rotation_speed: TAIL_ROTOR_SPEED,
            is_main_rotor: false,
        }
    }
}

/// 探照燈組件
#[derive(Component)]
pub struct HelicopterSpotlight {
    /// 追蹤目標
    pub target: Option<Entity>,
    /// 光線強度
    pub intensity: f32,
}

impl Default for HelicopterSpotlight {
    fn default() -> Self {
        Self {
            target: None,
            intensity: 100.0,
        }
    }
}

/// 直升機父實體標記（用於查找子組件）
#[derive(Component)]
pub struct HelicopterParent(pub Entity);

// ============================================================================
// 資源
// ============================================================================

/// 直升機生成狀態
#[derive(Resource, Default)]
pub struct HelicopterSpawnState {
    /// 當前直升機數量
    pub count: usize,
    /// 生成冷卻計時器
    pub cooldown: f32,
}

/// 直升機視覺資源
#[derive(Resource)]
pub struct HelicopterVisuals {
    /// 機身材質
    pub body_material: Handle<StandardMaterial>,
    /// 旋翼材質
    pub rotor_material: Handle<StandardMaterial>,
    /// 探照燈材質
    pub spotlight_material: Handle<StandardMaterial>,
    /// 機身 mesh
    pub body_mesh: Handle<Mesh>,
    /// 主旋翼 mesh
    pub main_rotor_mesh: Handle<Mesh>,
    /// 尾旋翼 mesh
    pub tail_rotor_mesh: Handle<Mesh>,
}

// ============================================================================
// 設置系統
// ============================================================================

/// 初始化直升機視覺資源
pub fn setup_helicopter_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 機身材質（深藍色警用塗裝）
    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.15, 0.3),
        metallic: 0.6,
        perceptual_roughness: 0.4,
        ..default()
    });

    // 旋翼材質（灰色金屬）
    let rotor_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.4, 0.45),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    // 探照燈材質（發光白色）
    let spotlight_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 0.9),
        emissive: LinearRgba::rgb(10.0, 10.0, 9.0),
        ..default()
    });

    // 機身 mesh（簡化橢圓體）
    let body_mesh = meshes.add(Capsule3d::new(1.5, 4.0));

    // 主旋翼 mesh（扁平圓柱代表旋轉中的旋翼）
    let main_rotor_mesh = meshes.add(Cylinder::new(4.5, 0.1));

    // 尾旋翼 mesh（較小圓柱）
    let tail_rotor_mesh = meshes.add(Cylinder::new(1.0, 0.05));

    commands.insert_resource(HelicopterVisuals {
        body_material,
        rotor_material,
        spotlight_material,
        body_mesh,
        main_rotor_mesh,
        tail_rotor_mesh,
    });

    commands.init_resource::<HelicopterSpawnState>();
}

// ============================================================================
// 生成系統
// ============================================================================

/// 直升機生成系統
pub fn spawn_helicopter_system(
    mut commands: Commands,
    time: Res<Time>,
    wanted: Res<WantedLevel>,
    mut spawn_state: ResMut<HelicopterSpawnState>,
    visuals: Res<HelicopterVisuals>,
    player_query: Query<&Transform, With<Player>>,
    helicopter_query: Query<Entity, With<PoliceHelicopter>>,
) {
    // 更新冷卻
    spawn_state.cooldown -= time.delta_secs();

    // 更新當前數量
    spawn_state.count = helicopter_query.iter().count();

    // 檢查是否需要生成
    if wanted.stars < HELICOPTER_SPAWN_WANTED_LEVEL {
        return;
    }

    if spawn_state.count >= HELICOPTER_MAX_COUNT {
        return;
    }

    if spawn_state.cooldown > 0.0 {
        return;
    }

    // 取得玩家位置
    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;

    // 在玩家後方遠處生成
    let spawn_angle = rand::random::<f32>() * std::f32::consts::TAU;
    let spawn_distance = 100.0 + rand::random::<f32>() * 50.0;
    let spawn_pos = Vec3::new(
        player_pos.x + spawn_angle.cos() * spawn_distance,
        HELICOPTER_HOVER_ALTITUDE + 20.0,  // 高空進場
        player_pos.z + spawn_angle.sin() * spawn_distance,
    );

    // 生成直升機實體
    let _helicopter_id = spawn_helicopter(&mut commands, &visuals, spawn_pos);

    info!("警用直升機出動！位置: {:?}", spawn_pos);

    // 重置冷卻（count 由下一幀的 query 自動更新）
    spawn_state.cooldown = HELICOPTER_SPAWN_COOLDOWN;
}

/// 生成單個直升機
fn spawn_helicopter(
    commands: &mut Commands,
    visuals: &HelicopterVisuals,
    position: Vec3,
) -> Entity {
    // 機身
    let helicopter_id = commands.spawn((
        Mesh3d(visuals.body_mesh.clone()),
        MeshMaterial3d(visuals.body_material.clone()),
        Transform::from_translation(position),
        PoliceHelicopter::default(),
        Health::new(HELICOPTER_HEALTH),
        Name::new("PoliceHelicopter"),
    )).id();

    // 主旋翼（在機身上方）
    let main_rotor_id = commands.spawn((
        Mesh3d(visuals.main_rotor_mesh.clone()),
        MeshMaterial3d(visuals.rotor_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
        HelicopterRotor::main(),
        HelicopterParent(helicopter_id),
        Name::new("MainRotor"),
    )).id();

    // 尾旋翼（在機尾側面）
    let tail_rotor_id = commands.spawn((
        Mesh3d(visuals.tail_rotor_mesh.clone()),
        MeshMaterial3d(visuals.rotor_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.5, -4.0))
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        HelicopterRotor::tail(),
        HelicopterParent(helicopter_id),
        Name::new("TailRotor"),
    )).id();

    // 探照燈（在機身下方）
    let spotlight_id = commands.spawn((
        SpotLight {
            color: Color::srgb(1.0, 1.0, 0.9),
            intensity: 500000.0,
            range: SPOTLIGHT_RANGE,
            outer_angle: SPOTLIGHT_CONE_ANGLE.to_radians(),
            inner_angle: (SPOTLIGHT_CONE_ANGLE * 0.6).to_radians(),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, -1.5, 1.0))
            .looking_at(Vec3::new(0.0, -10.0, 5.0), Vec3::Y),
        HelicopterSpotlight::default(),
        HelicopterParent(helicopter_id),
        Name::new("Spotlight"),
    )).id();

    // 設置父子關係
    commands.entity(helicopter_id).add_children(&[main_rotor_id, tail_rotor_id, spotlight_id]);

    helicopter_id
}

// ============================================================================
// AI 系統
// ============================================================================

/// 計算水平距離
fn calc_horizontal_distance(pos1: Vec3, pos2: Vec3) -> f32 {
    Vec2::new(pos1.x - pos2.x, pos1.z - pos2.z).length()
}

/// 處理接近狀態
fn handle_approaching_state(helicopter: &mut PoliceHelicopter, horizontal_distance: f32) {
    if horizontal_distance < HELICOPTER_ATTACK_RANGE * 0.8 {
        helicopter.state = HelicopterState::Hovering;
        helicopter.hover_timer = 0.0;
    }
}

/// 處理懸停狀態
fn handle_hovering_state(
    helicopter: &mut PoliceHelicopter,
    horizontal_distance: f32,
    player_visible: bool,
    dt: f32,
) {
    helicopter.hover_timer += dt;

    if helicopter.hover_timer > 2.0 && player_visible {
        helicopter.state = HelicopterState::Attacking;
    } else if horizontal_distance > HELICOPTER_ATTACK_RANGE * 1.2 {
        helicopter.state = HelicopterState::Pursuing;
    }
}

/// 處理追擊狀態
fn handle_pursuing_state(helicopter: &mut PoliceHelicopter, horizontal_distance: f32) {
    if horizontal_distance < HELICOPTER_ATTACK_RANGE * 0.8 {
        helicopter.state = HelicopterState::Hovering;
        helicopter.hover_timer = 0.0;
    }
}

/// 處理攻擊狀態
fn handle_attacking_state(
    helicopter: &mut PoliceHelicopter,
    horizontal_distance: f32,
    player_visible: bool,
    dt: f32,
) {
    if horizontal_distance > HELICOPTER_ATTACK_RANGE {
        helicopter.state = HelicopterState::Pursuing;
        return;
    }

    if !player_visible {
        helicopter.search_timer += dt;
        if helicopter.search_timer > 5.0 {
            helicopter.state = HelicopterState::Hovering;
            helicopter.search_timer = 0.0;
        }
    } else {
        helicopter.search_timer = 0.0;
    }
}

/// 處理規避狀態
fn handle_evading_state(helicopter: &mut PoliceHelicopter, dt: f32) {
    helicopter.evade_timer -= dt;
    if helicopter.evade_timer <= 0.0 {
        helicopter.state = HelicopterState::Pursuing;
    }
}

/// 檢查是否需要觸發規避
fn should_trigger_evade(helicopter: &PoliceHelicopter, current_time: f32) -> bool {
    current_time - helicopter.last_hit_time < 0.5
        && helicopter.state != HelicopterState::Crashing
        && helicopter.state != HelicopterState::Evading
}

/// 直升機 AI 系統
pub fn helicopter_ai_system(
    time: Res<Time>,
    wanted: Res<WantedLevel>,
    player_query: Query<&Transform, With<Player>>,
    mut helicopter_query: Query<(&mut PoliceHelicopter, &Transform)>,
) {
    let dt = time.delta_secs();
    let current_time = time.elapsed_secs();

    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;

    for (mut helicopter, transform) in helicopter_query.iter_mut() {
        if helicopter.state == HelicopterState::Crashing {
            continue;
        }

        let horizontal_distance = calc_horizontal_distance(transform.translation, player_pos);
        helicopter.target_position = Some(player_pos);

        match helicopter.state {
            HelicopterState::Approaching => {
                handle_approaching_state(&mut helicopter, horizontal_distance);
            }
            HelicopterState::Hovering => {
                handle_hovering_state(&mut helicopter, horizontal_distance, wanted.player_visible, dt);
            }
            HelicopterState::Pursuing => {
                handle_pursuing_state(&mut helicopter, horizontal_distance);
            }
            HelicopterState::Attacking => {
                handle_attacking_state(&mut helicopter, horizontal_distance, wanted.player_visible, dt);
            }
            HelicopterState::Evading => {
                handle_evading_state(&mut helicopter, dt);
            }
            HelicopterState::Crashing => {}
        }

        if should_trigger_evade(&helicopter, current_time) {
            helicopter.state = HelicopterState::Evading;
            helicopter.evade_timer = EVADE_DURATION;
        }
    }
}

// ============================================================================
// 移動系統
// ============================================================================

/// 處理墜毀移動
fn handle_crash_movement(transform: &mut Transform, crash_velocity: Vec3, dt: f32) {
    transform.translation += crash_velocity * dt;
    transform.rotate_y(CRASH_ROTATION_SPEED.to_radians() * dt);
    transform.rotate_x(30.0_f32.to_radians() * dt);

    if transform.translation.y < 0.0 {
        transform.translation.y = 0.0;
    }
}

/// 取得狀態對應的速度倍率
fn get_speed_multiplier(state: HelicopterState) -> f32 {
    match state {
        HelicopterState::Approaching => 1.0,
        HelicopterState::Pursuing => 1.2,
        HelicopterState::Evading => 1.5,
        HelicopterState::Hovering | HelicopterState::Attacking => 0.2,
        HelicopterState::Crashing => 0.0,
    }
}

/// 計算規避時的移動方向
fn calc_evade_direction(horizontal_dir: Vec3, elapsed_secs: f32) -> Vec3 {
    let evade_angle = (elapsed_secs * 2.0).sin() * 0.5;
    Quat::from_rotation_y(evade_angle) * horizontal_dir
}

/// 處理正常飛行移動
fn handle_normal_flight(
    transform: &mut Transform,
    helicopter: &PoliceHelicopter,
    elapsed_secs: f32,
    dt: f32,
) {
    let Some(target) = helicopter.target_position else { return };

    let to_target = target - transform.translation;
    let horizontal_dir = Vec3::new(to_target.x, 0.0, to_target.z).normalize_or_zero();
    let speed_mult = get_speed_multiplier(helicopter.state);

    let move_dir = if helicopter.state == HelicopterState::Evading {
        calc_evade_direction(horizontal_dir, elapsed_secs)
    } else {
        horizontal_dir
    };

    // 水平移動
    let horizontal_distance = Vec2::new(to_target.x, to_target.z).length();
    if horizontal_distance > 10.0 || helicopter.state == HelicopterState::Evading {
        transform.translation += move_dir * HELICOPTER_SPEED * speed_mult * dt;
    }

    // 垂直移動
    let altitude_diff = helicopter.target_altitude - transform.translation.y;
    if altitude_diff.abs() > 1.0 {
        transform.translation.y += altitude_diff.signum() * HELICOPTER_VERTICAL_SPEED * dt;
    }

    // 高度限制
    transform.translation.y = transform.translation.y.clamp(
        HELICOPTER_MIN_ALTITUDE,
        HELICOPTER_MAX_ALTITUDE,
    );

    // 面向目標
    if horizontal_dir != Vec3::ZERO {
        let target_rotation = Quat::from_rotation_y(
            (-horizontal_dir.z).atan2(horizontal_dir.x) - std::f32::consts::FRAC_PI_2
        );
        transform.rotation = transform.rotation.slerp(target_rotation, HELICOPTER_TURN_RATE * dt);
    }
}

/// 直升機移動系統
pub fn helicopter_movement_system(
    time: Res<Time>,
    mut helicopter_query: Query<(&mut Transform, &PoliceHelicopter)>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for (mut transform, helicopter) in helicopter_query.iter_mut() {
        if helicopter.state == HelicopterState::Crashing {
            handle_crash_movement(&mut transform, helicopter.crash_velocity, dt);
        } else {
            handle_normal_flight(&mut transform, helicopter, elapsed, dt);
        }
    }
}

// ============================================================================
// 戰鬥系統
// ============================================================================

/// 檢查直升機是否可以射擊
fn can_helicopter_fire(helicopter: &PoliceHelicopter, distance: f32) -> bool {
    helicopter.state == HelicopterState::Attacking
        && helicopter.fire_cooldown <= 0.0
        && distance <= HELICOPTER_ATTACK_RANGE
}

/// 計算槍口位置
fn calc_muzzle_position(heli_pos: Vec3, forward: Dir3) -> Vec3 {
    heli_pos + forward * 2.0 + Vec3::new(0.0, -1.0, 0.0)
}

/// 直升機射擊系統
pub fn helicopter_combat_system(
    mut commands: Commands,
    time: Res<Time>,
    visuals: Res<CombatVisuals>,
    mut damage_events: MessageWriter<DamageEvent>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    mut helicopter_query: Query<(Entity, &mut PoliceHelicopter, &Transform)>,
    rapier_context: ReadRapierContext,
) {
    let dt = time.delta_secs();

    let Ok((player_entity, player_transform)) = player_query.single() else { return; };
    let player_pos = player_transform.translation;
    let Ok(rapier) = rapier_context.single() else { return; };

    for (heli_entity, mut helicopter, transform) in helicopter_query.iter_mut() {
        helicopter.fire_cooldown = (helicopter.fire_cooldown - dt).max(0.0);

        let heli_pos = transform.translation;
        let to_player = player_pos - heli_pos;
        let distance = to_player.length();

        if !can_helicopter_fire(&helicopter, distance) {
            continue;
        }

        let direction = to_player.normalize();
        let muzzle_pos = calc_muzzle_position(heli_pos, transform.forward());

        spawn_muzzle_flash(&mut commands, &visuals, muzzle_pos);

        // 計算子彈終點
        let tracer_end = rapier
            .cast_ray(muzzle_pos, direction, HELICOPTER_ATTACK_RANGE, true, QueryFilter::default())
            .map(|(_, toi)| muzzle_pos + direction * toi)
            .unwrap_or_else(|| muzzle_pos + direction * HELICOPTER_ATTACK_RANGE);

        spawn_bullet_tracer(&mut commands, &visuals, muzzle_pos, tracer_end, TracerStyle::SMG);

        // 傷害判定
        if let Some((hit_entity, _)) = rapier.cast_ray(
            muzzle_pos, direction, distance, true, QueryFilter::default()
        ) {
            if hit_entity == player_entity {
                damage_events.write(DamageEvent {
                    target: player_entity,
                    amount: HELICOPTER_BULLET_DAMAGE,
                    source: DamageSource::Bullet,
                    attacker: Some(heli_entity),
                    hit_position: Some(player_pos),
                    is_headshot: false,
                });
            }
        }

        helicopter.fire_cooldown = 1.0 / HELICOPTER_FIRE_RATE;
    }
}

// ============================================================================
// 旋翼動畫系統
// ============================================================================

/// 旋翼旋轉動畫系統
pub fn rotor_animation_system(
    time: Res<Time>,
    mut rotor_query: Query<(&mut Transform, &HelicopterRotor, &HelicopterParent)>,
    helicopter_query: Query<&PoliceHelicopter>,
) {
    let dt = time.delta_secs();

    for (mut transform, rotor, parent) in rotor_query.iter_mut() {
        // 檢查父直升機是否墜毀
        let is_crashing = helicopter_query
            .get(parent.0)
            .map(|h| h.state == HelicopterState::Crashing)
            .unwrap_or(false);

        // 墜毀時旋翼逐漸減速
        let speed_mult = if is_crashing { 0.3 } else { 1.0 };

        // 旋轉
        let rotation_amount = rotor.rotation_speed * speed_mult * dt;
        if rotor.is_main_rotor {
            transform.rotate_y(rotation_amount.to_radians());
        } else {
            transform.rotate_x(rotation_amount.to_radians());
        }
    }
}

// ============================================================================
// 探照燈系統
// ============================================================================

/// 探照燈追蹤系統
pub fn spotlight_tracking_system(
    player_query: Query<&Transform, With<Player>>,
    helicopter_query: Query<(&Transform, &PoliceHelicopter), Without<Player>>,
    mut spotlight_query: Query<(&mut Transform, &HelicopterSpotlight, &HelicopterParent), (Without<PoliceHelicopter>, Without<Player>)>,
) {
    let Ok(player_transform) = player_query.single() else { return; };
    let player_pos = player_transform.translation;

    for (mut spotlight_transform, _spotlight, parent) in spotlight_query.iter_mut() {
        // 取得父直升機位置
        let Ok((heli_transform, helicopter)) = helicopter_query.get(parent.0) else { continue };

        // 墜毀時不追蹤
        if helicopter.state == HelicopterState::Crashing {
            continue;
        }

        // 計算從直升機到玩家的方向
        let heli_pos = heli_transform.translation;
        let to_player = player_pos - heli_pos;

        // 探照燈朝向玩家（在本地座標系）
        let local_target = heli_transform.rotation.inverse() * to_player;
        if local_target.length() > 0.1 {
            spotlight_transform.look_at(local_target.normalize() * 10.0, Vec3::Y);
        }
    }
}

// ============================================================================
// 傷害系統
// ============================================================================

/// 直升機受傷系統
pub fn helicopter_damage_system(
    time: Res<Time>,
    mut commands: Commands,
    mut helicopter_query: Query<(Entity, &mut PoliceHelicopter, &Health, &Transform)>,
    mut spawn_state: ResMut<HelicopterSpawnState>,
) {
    let current_time = time.elapsed_secs();

    for (entity, mut helicopter, health, transform) in helicopter_query.iter_mut() {
        // 同步生命值
        if health.current < helicopter.health {
            let damage_taken = helicopter.health - health.current;
            helicopter.health = health.current;
            helicopter.last_hit_time = current_time;

            info!("直升機受傷: -{:.0} HP, 剩餘: {:.0}", damage_taken, helicopter.health);
        }

        // 檢查是否墜毀
        if helicopter.health <= 0.0 && helicopter.state != HelicopterState::Crashing {
            helicopter.state = HelicopterState::Crashing;
            helicopter.crash_velocity = Vec3::new(
                (rand::random::<f32>() - 0.5) * 10.0,
                -15.0,
                (rand::random::<f32>() - 0.5) * 10.0,
            );

            warn!("警用直升機被擊落！");
        }

        // 墜毀到地面後移除
        if helicopter.state == HelicopterState::Crashing && transform.translation.y <= 0.5 {
            commands.entity(entity).despawn();
            spawn_state.count = spawn_state.count.saturating_sub(1);
            info!("直升機墜毀！");
        }
    }
}

// ============================================================================
// 清理系統
// ============================================================================

/// 直升機清理系統（脫離通緝時）
pub fn despawn_helicopter_system(
    mut commands: Commands,
    wanted: Res<WantedLevel>,
    helicopter_query: Query<Entity, With<PoliceHelicopter>>,
    mut spawn_state: ResMut<HelicopterSpawnState>,
) {
    // 通緝等級低於閾值時移除所有直升機
    if wanted.stars >= HELICOPTER_SPAWN_WANTED_LEVEL {
        return;
    }

    for entity in helicopter_query.iter() {
        commands.entity(entity).despawn();
    }

    spawn_state.count = 0;
    spawn_state.cooldown = 0.0;
}
