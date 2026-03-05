//! GTA5 風格群體恐慌傳播系統

#![allow(
    clippy::needless_pass_by_value,
    clippy::similar_names,
    clippy::items_after_statements
)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::KinematicCharacterController;
use rand::Rng;

use crate::core::PedestrianSpatialHash;
use crate::pedestrian::components::{
    GunshotTracker, PedState, Pedestrian, PedestrianConfig, PedestrianState, WalkingAnimation,
};
use crate::pedestrian::panic::{PanicState, PanicWave, PanicWaveManager};

// ============================================================================
// GTA 5 風格群體恐慌傳播系統
// ============================================================================

/// 恐慌系統常數
mod panic_constants {
    /// 恐慌消退速率（每秒減少的 `panic_level`）
    pub const PANIC_CALM_DOWN_RATE: f32 = 0.05;
    /// 恐慌逃跑時的速度加成
    pub const PANIC_FLEE_SPEED_MULTIPLIER: f32 = 1.5;
    /// 恐慌狀態下的隨機方向偏移（弧度）
    pub const PANIC_DIRECTION_JITTER: f32 = 0.3;
    /// 逃跑計時器基礎時間（秒）
    pub const FLEE_TIMER_BASE: f32 = 8.0;
    /// 逃跑計時器恐慌加成係數
    pub const FLEE_TIMER_PANIC_MULTIPLIER: f32 = 5.0;
    /// 旋轉插值速度
    pub const ROTATION_SLERP_SPEED: f32 = 8.0;
}

// ============================================================================
// 恐慌傳播輔助函數
// ============================================================================
/// 收集被恐慌波影響的行人
fn collect_panic_triggers(
    waves: &[PanicWave],
    ped_hash: &PedestrianSpatialHash,
    wave_front_width: f32,
) -> Vec<(Entity, f32, Vec3)> {
    let mut panic_triggers: Vec<(Entity, f32, Vec3)> = Vec::new();

    for wave in waves {
        if wave.current_radius < 0.1 {
            continue;
        }

        for (entity, _, dist_sq) in ped_hash.query_radius(wave.origin, wave.current_radius) {
            let dist = dist_sq.sqrt();
            // 檢查是否在波前緣
            if dist <= wave.current_radius && dist > wave.current_radius - wave_front_width {
                update_panic_trigger(&mut panic_triggers, entity, wave.intensity, wave.origin);
            }
        }
    }

    panic_triggers
}

/// 更新或添加恐慌觸發記錄
fn update_panic_trigger(
    triggers: &mut Vec<(Entity, f32, Vec3)>,
    entity: Entity,
    intensity: f32,
    source: Vec3,
) {
    if let Some(existing) = triggers.iter_mut().find(|(e, _, _)| *e == entity) {
        if intensity > existing.1 {
            existing.1 = intensity;
            existing.2 = source;
        }
    } else {
        triggers.push((entity, intensity, source));
    }
}

/// 對單個行人應用恐慌觸發
fn apply_panic_to_pedestrian(
    ped_state: &mut PedestrianState,
    panic_state: &mut PanicState,
    intensity: f32,
    source: Vec3,
    flee_timer_base: f32,
    flee_timer_panic_mul: f32,
) {
    panic_state.trigger_panic(intensity, source);

    if panic_state.is_panicked() && ped_state.state != PedState::Fleeing {
        // 保存當前狀態以便恐慌結束後恢復
        if panic_state.previous_state.is_none() {
            panic_state.previous_state = Some(ped_state.state);
        }
        ped_state.state = PedState::Fleeing;
        ped_state.fear_level = panic_state.panic_level;
        ped_state.flee_timer = flee_timer_base + panic_state.panic_level * flee_timer_panic_mul;
        ped_state.last_threat_pos = Some(source);
    }
}

/// 處理恐慌消退
fn handle_panic_fade(
    ped_state: &mut PedestrianState,
    panic_state: &mut PanicState,
    still_in_wave: bool,
    calm_down_rate: f32,
    dt: f32,
) {
    if still_in_wave {
        return;
    }

    panic_state.calm_down(calm_down_rate, dt);

    if !panic_state.is_panicked()
        && ped_state.state == PedState::Fleeing
        && ped_state.flee_timer <= 0.0
    {
        // 恢復恐慌前的狀態，若無則預設為 Walking
        ped_state.state = panic_state
            .previous_state
            .take()
            .unwrap_or(PedState::Walking);
        ped_state.fear_level = 0.0;
    }
}

/// 恐慌波傳播系統（空間哈希優化版）
///
/// 使用 `PedestrianSpatialHash` 將 O(行人×波數) 降為 O(波數×附近行人)。
/// 每個恐慌波只檢查其半徑內的行人，而非所有行人。
pub fn panic_wave_propagation_system(
    time: Res<Time>,
    mut panic_manager: ResMut<PanicWaveManager>,
    ped_hash: Res<PedestrianSpatialHash>,
    mut ped_query: Query<(&Transform, &mut PedestrianState, &mut PanicState), With<Pedestrian>>,
) {
    use panic_constants::{FLEE_TIMER_BASE, FLEE_TIMER_PANIC_MULTIPLIER, PANIC_CALM_DOWN_RATE};
    let dt = time.delta_secs();
    const WAVE_FRONT_WIDTH: f32 = 2.0;

    // 更新所有恐慌波（擴展半徑、清理過期）
    panic_manager.update(dt);

    // 階段 1：使用空間哈希找出被恐慌波影響的行人
    let panic_triggers = collect_panic_triggers(
        panic_manager.active_waves.make_contiguous(),
        &ped_hash,
        WAVE_FRONT_WIDTH,
    );

    // 階段 2：處理所有行人（更新計時器）
    for (_, _, mut panic_state) in &mut ped_query {
        panic_state.update(dt);
    }

    // 階段 3：應用恐慌觸發
    for (entity, intensity, source) in panic_triggers {
        let Ok((_, mut ped_state, mut panic_state)) = ped_query.get_mut(entity) else {
            continue;
        };
        apply_panic_to_pedestrian(
            &mut ped_state,
            &mut panic_state,
            intensity,
            source,
            FLEE_TIMER_BASE,
            FLEE_TIMER_PANIC_MULTIPLIER,
        );
    }

    // 階段 4：恐慌消退（僅處理正在恐慌的行人）
    for (ped_transform, mut ped_state, mut panic_state) in &mut ped_query {
        if panic_state.panic_level <= 0.0 {
            continue;
        }

        let still_in_wave = panic_manager
            .check_panic_at(ped_transform.translation)
            .is_some();
        handle_panic_fade(
            &mut ped_state,
            &mut panic_state,
            still_in_wave,
            PANIC_CALM_DOWN_RATE,
            dt,
        );
    }
}

/// 行人尖叫傳播恐慌系統
/// 高度恐慌的行人會尖叫，產生新的恐慌波
pub fn pedestrian_scream_system(
    time: Res<Time>,
    mut panic_manager: ResMut<PanicWaveManager>,
    mut ped_query: Query<(&Transform, &mut PanicState), With<Pedestrian>>,
) {
    let current_time = time.elapsed_secs();

    for (ped_transform, mut panic_state) in &mut ped_query {
        // 檢查是否可以尖叫傳播恐慌
        if panic_state.can_scream() {
            let ped_pos = ped_transform.translation;

            // 產生新的恐慌波
            panic_manager.create_from_scream(ped_pos, panic_state.panic_level, current_time);

            // 標記已尖叫（設置冷卻）
            panic_state.do_scream();
        }
    }
}

/// 槍聲觸發恐慌波系統
/// 當玩家開槍時，在槍聲位置創建恐慌波
pub fn gunshot_panic_trigger_system(
    time: Res<Time>,
    mut panic_manager: ResMut<PanicWaveManager>,
    gunshot_tracker: Res<GunshotTracker>,
    mut last_processed_count: Local<usize>,
) {
    let current_time = time.elapsed_secs();
    let current_count = gunshot_tracker.recent_shots.len();

    // 只處理新增的槍擊事件
    if current_count > *last_processed_count {
        for shot in gunshot_tracker
            .recent_shots
            .iter()
            .skip(*last_processed_count)
        {
            let (shot_pos, _shot_time) = *shot;
            panic_manager.create_from_gunshot(shot_pos, current_time);
        }
        *last_processed_count = current_count;
    }

    // 重置計數器（當 tracker 清理過期事件時）
    if current_count < *last_processed_count {
        *last_processed_count = 0;
    }
}

/// 恐慌逃跑方向系統
/// 讓恐慌的行人朝著遠離恐慌源的方向逃跑，並加入一些隨機偏移
pub fn panic_flee_direction_system(
    time: Res<Time>,
    config: Res<PedestrianConfig>,
    mut rng: Local<Option<rand::rngs::StdRng>>,
    mut ped_query: Query<
        (
            &mut Transform,
            &PedestrianState,
            &PanicState,
            &mut WalkingAnimation,
            &mut KinematicCharacterController,
        ),
        With<Pedestrian>,
    >,
) {
    use panic_constants::{
        PANIC_DIRECTION_JITTER, PANIC_FLEE_SPEED_MULTIPLIER, ROTATION_SLERP_SPEED,
    };
    use rand::SeedableRng;

    let dt = time.delta_secs();

    // 初始化持久化 RNG（只在第一次調用時創建）
    let rng = rng.get_or_insert_with(|| rand::rngs::StdRng::from_rng(&mut rand::rng()));

    for (mut transform, ped_state, panic_state, mut anim, mut controller) in &mut ped_query {
        // 只處理因恐慌而逃跑的行人
        if ped_state.state != PedState::Fleeing || !panic_state.is_panicked() {
            continue;
        }

        // 計算逃跑方向
        if let Some(flee_dir) = panic_state.flee_direction(transform.translation) {
            // 加入隨機方向偏移（模擬恐慌中的混亂）
            let jitter_angle = rng.random_range(-PANIC_DIRECTION_JITTER..PANIC_DIRECTION_JITTER);
            let jitter_rotation = Quat::from_rotation_y(jitter_angle);
            let jittered_dir = jitter_rotation * flee_dir;

            // 計算移動速度（恐慌程度越高越快）
            let speed = config.flee_speed * PANIC_FLEE_SPEED_MULTIPLIER * panic_state.panic_level;

            // 透過角色控制器移動（加入重力保持地面接觸）
            let movement = jittered_dir * speed * dt;
            controller.translation = Some(movement + Vec3::new(0.0, -9.8 * dt, 0.0));

            // 更新朝向（使用標準 Bevy 坐標系統慣例）
            if jittered_dir.length_squared() > 0.01 {
                let target_rotation =
                    Quat::from_rotation_y((-jittered_dir.x).atan2(-jittered_dir.z));
                transform.rotation = transform
                    .rotation
                    .slerp(target_rotation, dt * ROTATION_SLERP_SPEED);
            }

            // 更新動畫速度（恐慌時動畫更快）
            anim.speed = speed / config.walk_speed;
        }
    }
}
