//! 改裝系統：購買處理、氮氣加速

use bevy::prelude::*;

#[allow(clippy::wildcard_imports)]
use super::performance::*;
#[allow(clippy::wildcard_imports)]
use super::visuals::*;

// ============================================================================
// 事件
// ============================================================================

/// 購買改裝事件
#[derive(Message)]
pub struct PurchaseModificationEvent {
    /// 車輛實體
    pub vehicle: Entity,
    /// 改裝類別
    pub category: ModCategory,
}

/// 購買氮氣事件
#[derive(Message)]
pub struct PurchaseNitroEvent {
    /// 車輛實體
    pub vehicle: Entity,
}

/// 改裝完成事件
#[derive(Message)]
pub struct ModificationCompleteEvent {
    /// 車輛實體
    pub vehicle: Entity,
    /// 改裝類別
    pub category: ModCategory,
    /// 新等級
    pub new_level: ModLevel,
}

// ============================================================================
// 系統
// ============================================================================

/// 處理改裝購買事件
pub fn purchase_modification_system(
    mut events: MessageReader<PurchaseModificationEvent>,
    mut complete_events: MessageWriter<ModificationCompleteEvent>,
    mut vehicle_query: Query<(
        &mut VehicleModifications,
        Option<&mut super::super::VehicleHealth>,
    )>,
    mut wallet: ResMut<crate::economy::PlayerWallet>,
) {
    for event in events.read() {
        let Ok((mut mods, health)) = vehicle_query.get_mut(event.vehicle) else {
            warn!("找不到車輛 {:?}，無法套用改裝", event.vehicle);
            continue;
        };

        let current_level = mods.get_level(event.category);
        let Some(next_level) = current_level.next() else {
            info!("已達最高等級: {:?}", event.category);
            continue;
        };

        let price = next_level.price();

        // 扣款並升級（spend_cash 會檢查餘額並追蹤 total_spent）
        if !wallet.spend_cash(price) {
            info!("餘額不足: 需要 ${}, 現有 ${}", price, wallet.cash);
            continue;
        }
        mods.upgrade(event.category);

        // 裝甲改裝：增加車輛最大血量
        if event.category == ModCategory::Armor {
            if let Some(mut vehicle_health) = health {
                // 計算增量倍率（新等級 / 舊等級）
                let incremental_multiplier = next_level.multiplier() / current_level.multiplier();
                vehicle_health.apply_armor_upgrade(incremental_multiplier);
                info!(
                    "裝甲升級: 血量 {} -> {} ({}x)",
                    vehicle_health.max / incremental_multiplier,
                    vehicle_health.max,
                    incremental_multiplier
                );
            }
        }

        info!(
            "購買改裝: {:?} -> {} (${price})",
            event.category,
            next_level.name()
        );

        complete_events.write(ModificationCompleteEvent {
            vehicle: event.vehicle,
            category: event.category,
            new_level: next_level,
        });
    }
}

/// 處理氮氣購買事件
pub fn purchase_nitro_system(
    mut events: MessageReader<PurchaseNitroEvent>,
    mut vehicle_query: Query<(&mut VehicleModifications, Option<&mut NitroBoost>)>,
    mut commands: Commands,
    mut wallet: ResMut<crate::economy::PlayerWallet>,
) {
    for event in events.read() {
        let Ok((mut mods, nitro)) = vehicle_query.get_mut(event.vehicle) else {
            warn!("找不到車輛 {:?}，無法啟用氮氣", event.vehicle);
            continue;
        };

        if mods.has_nitro {
            info!("已安裝氮氣加速");
            continue;
        }

        // 扣款並安裝（spend_cash 會檢查餘額並追蹤 total_spent）
        if !wallet.spend_cash(NITRO_PRICE) {
            info!("餘額不足: 需要 ${}, 現有 ${}", NITRO_PRICE, wallet.cash);
            continue;
        }
        mods.has_nitro = true;
        mods.nitro_charge = 1.0;

        // 添加 NitroBoost 組件
        if nitro.is_none() {
            commands.entity(event.vehicle).insert(NitroBoost::new());
        }

        info!("購買氮氣加速 (${NITRO_PRICE})");
    }
}

/// 氮氣加速系統（僅作用於玩家當前車輛）
pub fn nitro_boost_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut VehicleModifications, &mut NitroBoost)>,
    game_state: Res<crate::core::GameState>,
) {
    if !game_state.player_in_vehicle {
        return;
    }

    let Some(current_vehicle) = game_state.current_vehicle else {
        return;
    };

    let Ok((mut mods, mut nitro)) = query.get_mut(current_vehicle) else {
        return;
    };

    if !mods.has_nitro {
        return;
    }

    let dt = time.delta_secs();

    // Shift 鍵（左或右）啟動氮氣
    let wants_boost = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    if wants_boost && mods.nitro_charge > 0.0 {
        nitro.is_active = true;
        mods.nitro_charge = (mods.nitro_charge - NITRO_DRAIN_RATE * dt).max(0.0);
    } else {
        nitro.is_active = false;
        // 不使用時緩慢回充
        mods.nitro_charge = (mods.nitro_charge + NITRO_RECHARGE_RATE * dt).min(1.0);
    }
}

/// 處理視覺改裝購買事件
pub fn purchase_visual_mod_system(
    mut events: MessageReader<PurchaseVisualModEvent>,
    mut vehicle_query: Query<&mut VehicleVisualMods>,
    mut wallet: ResMut<crate::economy::PlayerWallet>,
) {
    for event in events.read() {
        let Ok(mut visuals) = vehicle_query.get_mut(event.vehicle) else {
            warn!("找不到車輛 {:?}，無法套用視覺改裝", event.vehicle);
            continue;
        };

        let price = event.modification.price();
        if !wallet.spend_cash(price) {
            info!("餘額不足: 需要 ${}, 現有 ${}", price, wallet.cash);
            continue;
        }

        match &event.modification {
            VisualModPurchase::Paint(color) => visuals.paint = *color,
            VisualModPurchase::Tint(tint) => visuals.tint = *tint,
            VisualModPurchase::Spoiler(spoiler) => visuals.spoiler = *spoiler,
            VisualModPurchase::Rims(rims) => visuals.rims = *rims,
        }

        info!("視覺改裝完成: {} (${price})", event.modification.name());
    }
}
