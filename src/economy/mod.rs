//! 經濟系統模組
//!
//! 統一管理金錢、商店、ATM 互動

pub mod casino;
mod components;
pub mod enterprise;
pub mod stock_market;
mod systems;

#[cfg(test)]
mod tests;

pub use casino::*;
pub use components::*;
pub use enterprise::*;
pub use stock_market::*;
pub use systems::*;

use bevy::prelude::*;
use crate::core::InteractionSet;

/// 經濟系統插件
pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app
            // 資源
            .init_resource::<PlayerWallet>()
            .insert_resource(ShopInventory::new())
            .init_resource::<ShopMenuState>()
            .init_resource::<AtmMenuState>()
            .init_resource::<StockMarket>()
            .init_resource::<CasinoMenuState>()
            .init_resource::<BlackjackGame>()
            .init_resource::<SlotMachine>()
            .init_resource::<EnterpriseManager>()
            // 事件
            .add_message::<MoneyChangedEvent>()
            .add_message::<PurchaseEvent>()
            .add_message::<TransactionEvent>()
            // 系統
            .add_systems(Update, (
                sync_money_display,
                handle_shop_interaction.in_set(InteractionSet::Economy),
                handle_atm_interaction.in_set(InteractionSet::Economy),
                property_purchase_system.in_set(InteractionSet::Economy),
                store_robbery_system.in_set(InteractionSet::Economy),
                process_transactions,
                update_money_ui,
                update_cash_pickups,
                rental_income_system,
                robbery_cooldown_system,
                stock_price_update_system,
                blackjack_bet_system,
                blackjack_play_system,
                slot_machine_system,
                enterprise_income_system,
            ).chain());
    }
}
