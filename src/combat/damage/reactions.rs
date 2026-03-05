//! 受傷反應系統
//!
//! 處理受傷後的視覺回饋、擊退效果。

use bevy::math::EulerRot;
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::combat::components::{Enemy, HitReaction, HitReactionPhase};
use crate::combat::visuals::Ragdoll;
use crate::pedestrian::Pedestrian;

/// 受傷反應更新系統
/// 每幀更新所有 HitReaction 組件的狀態
pub fn hit_reaction_update_system(time: Res<Time>, mut query: Query<&mut HitReaction>) {
    let delta = time.delta_secs();

    for mut reaction in &mut query {
        reaction.update(delta);
    }
}

/// 受傷反應視覺效果系統
/// 將 HitReaction 的視覺旋轉應用到實體的子視覺組件上
pub fn hit_reaction_visual_system(
    reaction_query: Query<(&HitReaction, &Children), Changed<HitReaction>>,
    mut transform_query: Query<&mut Transform, Without<HitReaction>>,
) {
    for (reaction, children) in &reaction_query {
        if reaction.phase == HitReactionPhase::None {
            continue;
        }

        // 將視覺旋轉應用到第一個子實體（通常是模型）
        for child in children {
            if let Ok(mut transform) = transform_query.get_mut(*child) {
                // 只修改 X 軸旋轉（後仰效果），保持其他旋轉
                let current_euler = transform.rotation.to_euler(EulerRot::XYZ);
                let target_euler = reaction.visual_rotation.to_euler(EulerRot::XYZ);
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    target_euler.0,  // 使用反應的 X 旋轉
                    current_euler.1, // 保持 Y 旋轉
                    current_euler.2, // 保持 Z 旋轉
                );
                break; // 只處理第一個子實體
            }
        }
    }
}

/// 受傷反應擊退系統
/// 將擊退速度應用到角色控制器
pub fn hit_reaction_knockback_system(
    time: Res<Time>,
    mut query: Query<(&HitReaction, &mut KinematicCharacterController)>,
) {
    let delta = time.delta_secs();

    for (reaction, mut controller) in &mut query {
        let knockback = reaction.get_knockback_velocity();
        if knockback.length_squared() > 0.001 {
            // 將擊退速度加到控制器的位移上
            let current_translation = controller.translation.unwrap_or(Vec3::ZERO);
            controller.translation = Some(current_translation + knockback * delta);
        }
    }
}

/// 應用擊退效果到 Transform（共用邏輯）
#[inline]
fn apply_knockback_to_transform(reaction: &HitReaction, transform: &mut Transform, delta: f32) {
    let knockback = reaction.get_knockback_velocity();
    if knockback.length_squared() <= 0.001 {
        return;
    }

    transform.translation += knockback * delta;
    // 確保不會掉到地面以下
    if transform.translation.y < 0.0 {
        transform.translation.y = 0.0;
    }
}

/// 敵人受傷反應擊退系統
pub fn enemy_hit_reaction_knockback_system(
    time: Res<Time>,
    mut query: Query<(&HitReaction, &mut Transform), (With<Enemy>, Without<Ragdoll>)>,
) {
    let delta = time.delta_secs();
    for (reaction, mut transform) in &mut query {
        apply_knockback_to_transform(reaction, &mut transform, delta);
    }
}

/// 行人受傷反應擊退系統
pub fn pedestrian_hit_reaction_knockback_system(
    time: Res<Time>,
    mut query: Query<(&HitReaction, &mut Transform), (With<Pedestrian>, Without<Ragdoll>)>,
) {
    let delta = time.delta_secs();
    for (reaction, mut transform) in &mut query {
        apply_knockback_to_transform(reaction, &mut transform, delta);
    }
}
