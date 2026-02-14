//! 近戰攻擊系統
//!
//! 從 firing.rs 拆分，處理近戰攻擊邏輯：拳頭、刀、棍棒。

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::combat::health::{BleedEffect, DamageEvent, DamageSource, Damageable, BLEED_CHANCE};
use crate::combat::weapon::*;
use crate::core::rapier_real_to_f32;

/// 棍棒掃擊檢測步數
const STAFF_SWEEP_STEPS: usize = 5;

/// 近戰攻擊（回傳是否命中，用於推進連擊）
#[allow(clippy::too_many_arguments)]
pub(super) fn fire_melee(
    commands: &mut Commands,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    damageable_query: &Query<Entity, (With<Damageable>, With<Transform>)>,
    combo_multiplier: f32,
    is_finisher: bool,
) -> bool {
    let filter = QueryFilter::default().exclude_collider(attacker);
    let damage = weapon.stats.damage * combo_multiplier;

    let hit = match weapon.stats.weapon_type {
        WeaponType::Staff => {
            // 棍棒：弧形掃擊，可命中多個目標
            fire_staff_sweep(
                commands,
                attacker,
                origin,
                direction,
                weapon,
                rapier,
                damage_events,
                filter,
                combo_multiplier,
                is_finisher,
            )
        }
        WeaponType::Knife => {
            // 刀：單目標，有機率觸發流血
            fire_knife_attack(
                commands,
                attacker,
                origin,
                direction,
                weapon,
                rapier,
                damage_events,
                filter,
                combo_multiplier,
                is_finisher,
            )
        }
        _ => {
            // 拳頭或其他近戰：單目標直線攻擊
            if let Some((hit_entity, toi)) = rapier.cast_ray(
                origin,
                direction,
                weapon.stats.range as bevy_rapier3d::prelude::Real,
                true,
                filter,
            ) {
                let hit_pos = origin + direction * rapier_real_to_f32(toi);
                let mut event = DamageEvent::new(hit_entity, damage, DamageSource::Melee)
                    .with_attacker(attacker)
                    .with_position(hit_pos);
                if is_finisher {
                    event.force_knockback = true;
                }
                damage_events.write(event);
                true
            } else {
                false
            }
        }
    };

    let _ = damageable_query; // 保留參數以供未來使用
    hit
}

/// 棍棒弧形掃擊攻擊（回傳是否命中）
fn fire_staff_sweep(
    _commands: &mut Commands,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    filter: QueryFilter,
    combo_multiplier: f32,
    is_finisher: bool,
) -> bool {
    let sweep_angle = weapon.stats.spread.to_radians(); // 使用 spread 作為掃擊角度
    let mut hit_entities: Vec<Entity> = Vec::new();
    let damage = weapon.stats.damage * combo_multiplier;

    // 在弧形範圍內進行多次射線檢測
    for i in 0..STAFF_SWEEP_STEPS {
        let t = i as f32 / (STAFF_SWEEP_STEPS - 1) as f32;
        let angle = -sweep_angle / 2.0 + t * sweep_angle;

        // 繞 Y 軸旋轉方向向量
        let rotated_dir = Quat::from_rotation_y(angle) * direction;

        if let Some((hit_entity, toi)) = rapier.cast_ray(
            origin,
            rotated_dir,
            weapon.stats.range as bevy_rapier3d::prelude::Real,
            true,
            filter,
        ) {
            // 避免對同一目標重複造成傷害
            if !hit_entities.contains(&hit_entity) {
                hit_entities.push(hit_entity);

                let hit_pos = origin + rotated_dir * rapier_real_to_f32(toi);
                let mut event = DamageEvent::new(hit_entity, damage, DamageSource::Melee)
                    .with_attacker(attacker)
                    .with_position(hit_pos);
                if is_finisher {
                    event.force_knockback = true;
                }
                damage_events.write(event);
            }
        }
    }
    !hit_entities.is_empty()
}

/// 刀攻擊（有流血效果，回傳是否命中）
fn fire_knife_attack(
    commands: &mut Commands,
    attacker: Entity,
    origin: Vec3,
    direction: Vec3,
    weapon: &Weapon,
    rapier: &RapierContext,
    damage_events: &mut MessageWriter<DamageEvent>,
    filter: QueryFilter,
    combo_multiplier: f32,
    is_finisher: bool,
) -> bool {
    if let Some((hit_entity, toi)) = rapier.cast_ray(
        origin,
        direction,
        weapon.stats.range as bevy_rapier3d::prelude::Real,
        true,
        filter,
    ) {
        let hit_pos = origin + direction * rapier_real_to_f32(toi);
        let damage = weapon.stats.damage * combo_multiplier;

        // 發送傷害事件
        let mut event = DamageEvent::new(hit_entity, damage, DamageSource::Melee)
            .with_attacker(attacker)
            .with_position(hit_pos);
        if is_finisher {
            event.force_knockback = true;
        }
        damage_events.write(event);

        // 機率觸發流血效果
        if rand::random::<f32>() < BLEED_CHANCE {
            commands
                .entity(hit_entity)
                .insert(BleedEffect::new(attacker));
        }
        true
    } else {
        false
    }
}
