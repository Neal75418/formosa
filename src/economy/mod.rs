//! 經濟系統模組
//!
//! 統一管理金錢、商店、ATM 互動

mod components;
mod systems;

#[cfg(test)]
mod tests;

pub use components::*;
pub use systems::*;

use bevy::prelude::*;

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
            // 事件
            .add_message::<MoneyChangedEvent>()
            .add_message::<PurchaseEvent>()
            .add_message::<TransactionEvent>()
            // 系統
            .add_systems(Update, (
                sync_money_display,
                handle_shop_interaction,
                handle_atm_interaction,
                process_transactions,
                update_money_ui,
                update_cash_pickups,
            ).chain());
    }
}
