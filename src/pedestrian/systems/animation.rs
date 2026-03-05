//! 行人動畫系統（行走擺動、行為動作）

use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use std::f32::consts::PI;

use crate::pedestrian::behavior::{BehaviorType, DailyBehavior};
use crate::pedestrian::components::{
    PedState, Pedestrian, PedestrianArm, PedestrianLeg, PedestrianState, WalkingAnimation,
};

// ============================================================================
// 行走動畫系統
// ============================================================================

// ============================================================================
// 動畫輔助函數
// ============================================================================
/// 計算行人動畫目標速度
fn get_animation_target_speed(state: PedState) -> f32 {
    match state {
        PedState::Fleeing => 12.0,
        PedState::Walking => 6.0,
        PedState::Idle | PedState::CallingPolice => 0.0,
    }
}

/// 更新腿部動畫
fn update_leg_transform(transform: &mut Transform, anim: &WalkingAnimation, is_left: bool) {
    let base_x = if is_left { -0.08 } else { 0.08 };
    let base_y = -0.25 - 0.225; // torso_height/2 + leg_height/2

    if anim.speed > 0.1 {
        let phase_offset = if is_left { 0.0 } else { PI };
        let swing = (anim.phase + phase_offset).sin() * 0.4;

        transform.translation = Vec3::new(base_x, base_y, swing * 0.15);
        transform.rotation = Quat::from_rotation_x(swing);
    } else {
        transform.translation = Vec3::new(base_x, base_y, 0.0);
        transform.rotation = Quat::IDENTITY;
    }
}

/// 更新手臂動畫
fn update_arm_transform(transform: &mut Transform, anim: &WalkingAnimation, is_left: bool) {
    let base_x = if is_left { -0.22 } else { 0.22 };
    let base_z_rot = if is_left { 0.15 } else { -0.15 };

    if anim.speed > 0.1 {
        let phase_offset = if is_left { PI } else { 0.0 };
        let swing = (anim.phase + phase_offset).sin() * 0.3;

        transform.translation = Vec3::new(base_x, 0.125, swing * 0.1);
        transform.rotation = Quat::from_rotation_z(base_z_rot) * Quat::from_rotation_x(swing * 0.5);
    } else {
        transform.translation = Vec3::new(base_x, 0.125, 0.0);
        transform.rotation = Quat::from_rotation_z(base_z_rot);
    }
}

/// 行走動畫更新系統
pub fn pedestrian_walking_animation_system(
    time: Res<Time>,
    mut ped_query: Query<(&PedestrianState, &mut WalkingAnimation), With<Pedestrian>>,
    mut leg_query: Query<(&ChildOf, &PedestrianLeg, &mut Transform)>,
    mut arm_query: Query<(&ChildOf, &PedestrianArm, &mut Transform), Without<PedestrianLeg>>,
) {
    let dt = time.delta_secs();

    // 更新每個行人的動畫相位
    for (state, mut anim) in &mut ped_query {
        let target_speed = get_animation_target_speed(state.state);

        // 平滑過渡動畫速度
        anim.speed = anim.speed + (target_speed - anim.speed) * dt * 5.0;
        anim.phase += anim.speed * dt;

        // 保持相位在合理範圍
        if anim.phase > PI * 2.0 {
            anim.phase -= PI * 2.0;
        }
    }

    // 更新腿部擺動
    for (parent, leg, mut transform) in &mut leg_query {
        let Ok((_, anim)) = ped_query.get(parent.get()) else {
            continue;
        };
        update_leg_transform(&mut transform, anim, leg.is_left);
    }

    // 更新手臂擺動（與腿相反）
    for (parent, arm, mut transform) in &mut arm_query {
        let Ok((_, anim)) = ped_query.get(parent.get()) else {
            continue;
        };
        update_arm_transform(&mut transform, anim, arm.is_left);
    }
}

/// 行為動畫效果系統
pub fn behavior_animation_system(
    time: Res<Time>,
    mut ped_query: Query<(&DailyBehavior, &mut Transform, &mut WalkingAnimation), With<Pedestrian>>,
) {
    let elapsed = time.elapsed_secs();

    for (behavior, mut transform, mut anim) in &mut ped_query {
        match behavior.behavior {
            BehaviorType::PhoneWatching => {
                // 看手機：微微低頭，偶爾抬頭
                let head_bob = (elapsed * 0.3).sin() * 0.05;
                // 透過動畫速度控制腿部停止
                anim.speed = 0.0;
                // 身體微微前傾
                transform.rotation = transform.rotation.slerp(
                    Quat::from_rotation_x(0.1 + head_bob),
                    time.delta_secs() * 2.0,
                );
            }
            BehaviorType::WindowShopping => {
                // 逛櫥窗：緩慢左右轉動看櫥窗
                let look_angle = (elapsed * 0.5).sin() * 0.3;
                let base_rotation = transform.rotation;
                let look_rotation = Quat::from_rotation_y(look_angle);
                transform.rotation =
                    base_rotation.slerp(base_rotation * look_rotation, time.delta_secs() * 1.0);
            }
            BehaviorType::TakingPhoto => {
                // 拍照：舉起手（透過手臂旋轉模擬，這裡只做身體穩定）
                anim.speed = 0.0;
            }
            BehaviorType::Chatting => {
                // 聊天：身體微微搖擺
                let sway = (elapsed * 2.0).sin() * 0.02;
                transform.rotation = transform
                    .rotation
                    .slerp(Quat::from_rotation_z(sway), time.delta_secs() * 2.0);
                anim.speed = 0.0;
            }
            BehaviorType::Resting => {
                // 休息：完全靜止
                anim.speed = 0.0;
            }
            BehaviorType::Walking => {
                // 正常行走：恢復動畫
                // 動畫速度在 walking_animation_system 中處理
            }
            BehaviorType::SeekingShelter => {
                // 躲雨：快速奔跑（類似 Walking 但更快）
                // 動畫速度在 walking_animation_system 中處理
                anim.speed = 2.0; // 加快動畫速度表現匆忙感
            }
        }
    }
}
