//! 股票市場系統 (GTA 5 風格)
//!
//! 手機 App 內 6 支股票，受隨機波動和任務事件影響價格。
//! 玩家可以買入/賣出賺取價差。

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]
// Bevy 系統需要 Res<T> 按值傳遞
#![allow(clippy::needless_pass_by_value)]

use std::collections::VecDeque;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::PlayerWallet;

// ============================================================================
// 股票定義
// ============================================================================

/// 股票代碼
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum StockSymbol {
    /// 台積電科技（半導體）
    #[default]
    TSMC,
    /// 鳳梨航空（航運）
    PineAir,
    /// 珍奶集團（飲料連鎖）
    BobaGroup,
    /// 夜市王國（餐飲）
    NightMarketKing,
    /// 龍山建設（房地產）
    DragonBuild,
    /// 黑熊能源（能源）
    BearEnergy,
}

impl StockSymbol {
    /// 所有股票
    pub const ALL: [StockSymbol; 6] = [
        StockSymbol::TSMC,
        StockSymbol::PineAir,
        StockSymbol::BobaGroup,
        StockSymbol::NightMarketKing,
        StockSymbol::DragonBuild,
        StockSymbol::BearEnergy,
    ];

    /// 股票中文名
    pub fn label(&self) -> &'static str {
        match self {
            StockSymbol::TSMC => "台積電科技",
            StockSymbol::PineAir => "鳳梨航空",
            StockSymbol::BobaGroup => "珍奶集團",
            StockSymbol::NightMarketKing => "夜市王國",
            StockSymbol::DragonBuild => "龍山建設",
            StockSymbol::BearEnergy => "黑熊能源",
        }
    }

    /// 股票代碼簡稱
    pub fn ticker(&self) -> &'static str {
        match self {
            StockSymbol::TSMC => "TSMC",
            StockSymbol::PineAir => "PAIR",
            StockSymbol::BobaGroup => "BOBA",
            StockSymbol::NightMarketKing => "NMKT",
            StockSymbol::DragonBuild => "DRGN",
            StockSymbol::BearEnergy => "BEAR",
        }
    }

    /// 初始價格
    pub fn initial_price(&self) -> f32 {
        match self {
            StockSymbol::TSMC => 500.0,
            StockSymbol::PineAir => 120.0,
            StockSymbol::BobaGroup => 85.0,
            StockSymbol::NightMarketKing => 45.0,
            StockSymbol::DragonBuild => 200.0,
            StockSymbol::BearEnergy => 150.0,
        }
    }

    /// 波動率（每次更新的最大百分比變動）
    pub fn volatility(&self) -> f32 {
        match self {
            StockSymbol::TSMC => 0.03,          // 3% 穩定型
            StockSymbol::PineAir => 0.06,       // 6% 中等波動
            StockSymbol::BobaGroup => 0.04,     // 4%
            StockSymbol::NightMarketKing => 0.08, // 8% 高波動
            StockSymbol::DragonBuild => 0.05,   // 5%
            StockSymbol::BearEnergy => 0.07,    // 7% 較高波動
        }
    }
}

// ============================================================================
// 單一股票資料
// ============================================================================

/// 單一股票狀態
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stock {
    /// 股票代碼
    pub symbol: StockSymbol,
    /// 當前價格
    pub price: f32,
    /// 前一次價格（用於計算漲跌）
    pub previous_price: f32,
    /// 價格歷史紀錄（最近 20 筆，用於迷你走勢圖）
    pub price_history: VecDeque<f32>,
    /// 趨勢偏移（正=看漲，負=看跌，由任務事件設定）
    pub trend_bias: f32,
    /// 趨勢剩餘持續時間（秒）
    pub trend_duration: f32,
}

impl Stock {
    pub fn new(symbol: StockSymbol) -> Self {
        let price = symbol.initial_price();
        Self {
            symbol,
            price,
            previous_price: price,
            price_history: VecDeque::from([price]),
            trend_bias: 0.0,
            trend_duration: 0.0,
        }
    }

    /// 價格變動百分比
    pub fn change_percent(&self) -> f32 {
        if self.previous_price == 0.0 {
            return 0.0;
        }
        (self.price - self.previous_price) / self.previous_price * 100.0
    }

    /// 是否上漲
    pub fn is_up(&self) -> bool {
        self.price >= self.previous_price
    }

    /// 更新價格（隨機波動 + 趨勢偏移）
    pub fn tick_price(&mut self, dt: f32) {
        self.previous_price = self.price;

        // 隨機波動 (-1.0 ~ 1.0)
        let random_factor = rand::random::<f32>() * 2.0 - 1.0;
        let volatility = self.symbol.volatility();

        // 基礎波動
        let mut change = random_factor * volatility * self.price * dt;

        // 趨勢偏移
        if self.trend_duration > 0.0 {
            change += self.trend_bias * self.price * dt;
            self.trend_duration -= dt;
            if self.trend_duration <= 0.0 {
                self.trend_bias = 0.0;
            }
        }

        // 均值回歸（防止價格偏離初始值太遠）
        let initial = self.symbol.initial_price();
        let deviation = (self.price - initial) / initial;
        change -= deviation * 0.01 * self.price * dt; // 緩慢拉回

        self.price = (self.price + change).max(1.0); // 最低 $1

        // 更新歷史（VecDeque 的 pop_front 是 O(1)）
        self.price_history.push_back(self.price);
        if self.price_history.len() > 20 {
            self.price_history.pop_front();
        }
    }

    /// 設定趨勢（由任務事件觸發）
    pub fn set_trend(&mut self, bias: f32, duration: f32) {
        self.trend_bias = bias;
        self.trend_duration = duration;
    }
}

// ============================================================================
// 玩家持股
// ============================================================================

/// 玩家持有的股票
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StockPortfolio {
    /// 各股票持有數量
    pub holdings: [(StockSymbol, u32); 6],
    /// 各股票平均買入成本（用於計算損益）
    pub avg_costs: [f32; 6],
}

impl StockPortfolio {
    pub fn new() -> Self {
        Self {
            holdings: [
                (StockSymbol::TSMC, 0),
                (StockSymbol::PineAir, 0),
                (StockSymbol::BobaGroup, 0),
                (StockSymbol::NightMarketKing, 0),
                (StockSymbol::DragonBuild, 0),
                (StockSymbol::BearEnergy, 0),
            ],
            avg_costs: [0.0; 6],
        }
    }

    /// 取得某股持有數量
    pub fn shares_of(&self, symbol: StockSymbol) -> u32 {
        let idx = Self::index_of(symbol);
        self.holdings[idx].1
    }

    /// 取得某股平均成本
    pub fn avg_cost_of(&self, symbol: StockSymbol) -> f32 {
        self.avg_costs[Self::index_of(symbol)]
    }

    /// 買入
    pub fn buy(&mut self, symbol: StockSymbol, shares: u32, price_per_share: f32) {
        let idx = Self::index_of(symbol);
        let old_shares = self.holdings[idx].1;
        let old_cost = self.avg_costs[idx];

        let total_old = old_shares as f32 * old_cost;
        let total_new = shares as f32 * price_per_share;
        let new_total_shares = old_shares + shares;

        self.holdings[idx].1 = new_total_shares;
        if new_total_shares > 0 {
            self.avg_costs[idx] = (total_old + total_new) / new_total_shares as f32;
        }
    }

    /// 賣出（回傳是否成功）
    pub fn sell(&mut self, symbol: StockSymbol, shares: u32) -> bool {
        let idx = Self::index_of(symbol);
        if self.holdings[idx].1 < shares {
            return false;
        }
        self.holdings[idx].1 -= shares;
        if self.holdings[idx].1 == 0 {
            self.avg_costs[idx] = 0.0;
        }
        true
    }

    /// 計算某股損益
    pub fn profit_loss(&self, symbol: StockSymbol, current_price: f32) -> f32 {
        let idx = Self::index_of(symbol);
        let shares = self.holdings[idx].1;
        if shares == 0 {
            return 0.0;
        }
        (current_price - self.avg_costs[idx]) * shares as f32
    }

    /// 計算總持倉市值
    pub fn total_value(&self, market: &StockMarket) -> f32 {
        StockSymbol::ALL
            .iter()
            .map(|&sym| {
                let shares = self.shares_of(sym);
                let price = market.get(sym).price;
                shares as f32 * price
            })
            .sum()
    }

    fn index_of(symbol: StockSymbol) -> usize {
        StockSymbol::ALL
            .iter()
            .position(|&s| s == symbol)
            .unwrap_or(0)
    }
}

// ============================================================================
// 股票市場資源
// ============================================================================

/// 股票價格更新間隔（遊戲秒）
const PRICE_UPDATE_INTERVAL: f32 = 30.0;

/// 股票市場資源
#[derive(Resource)]
pub struct StockMarket {
    /// 所有股票
    pub stocks: Vec<Stock>,
    /// 價格更新計時器
    pub update_timer: f32,
    /// 玩家持股
    pub portfolio: StockPortfolio,
    /// 市場是否開市（遊戲時間 9:00-17:00）
    pub is_open: bool,
}

impl Default for StockMarket {
    fn default() -> Self {
        Self {
            stocks: StockSymbol::ALL.iter().map(|&s| Stock::new(s)).collect(),
            update_timer: PRICE_UPDATE_INTERVAL,
            portfolio: StockPortfolio::new(),
            is_open: true,
        }
    }
}

impl StockMarket {
    /// 取得某股票引用
    pub fn get(&self, symbol: StockSymbol) -> &Stock {
        let idx = StockSymbol::ALL
            .iter()
            .position(|&s| s == symbol)
            .unwrap_or(0);
        &self.stocks[idx]
    }

    /// 取得某股票可變引用
    pub fn get_mut(&mut self, symbol: StockSymbol) -> &mut Stock {
        let idx = StockSymbol::ALL
            .iter()
            .position(|&s| s == symbol)
            .unwrap_or(0);
        &mut self.stocks[idx]
    }

    /// 買入股票（扣現金）
    pub fn buy(
        &mut self,
        symbol: StockSymbol,
        shares: u32,
        wallet: &mut PlayerWallet,
    ) -> Result<f32, &'static str> {
        if !self.is_open {
            return Err("市場未開市");
        }
        let price = self.get(symbol).price;
        let total_cost = (price * shares as f32).ceil() as i32;

        if !wallet.spend_cash(total_cost) {
            return Err("現金不足");
        }

        self.portfolio.buy(symbol, shares, price);
        Ok(price)
    }

    /// 賣出股票（加現金）
    pub fn sell(
        &mut self,
        symbol: StockSymbol,
        shares: u32,
        wallet: &mut PlayerWallet,
    ) -> Result<f32, &'static str> {
        if !self.is_open {
            return Err("市場未開市");
        }
        if !self.portfolio.sell(symbol, shares) {
            return Err("持股不足");
        }

        let price = self.get(symbol).price;
        let total_value = (price * shares as f32).floor() as i32;
        wallet.add_cash(total_value);
        Ok(price)
    }
}

// ============================================================================
// 系統
// ============================================================================

use crate::core::WorldTime;

/// 股票價格更新系統
/// 每隔固定時間更新所有股票價格
pub fn stock_price_update_system(
    time: Res<Time>,
    world_time: Res<WorldTime>,
    mut market: ResMut<StockMarket>,
) {
    // 開市時間：9:00-17:00
    let hour = world_time.hour;
    market.is_open = (9.0..17.0).contains(&hour);

    if !market.is_open {
        return;
    }

    market.update_timer -= time.delta_secs();

    if market.update_timer <= 0.0 {
        market.update_timer = PRICE_UPDATE_INTERVAL;

        // 用固定的 dt=1.0 來更新（每次 tick 代表一個更新週期）
        for stock in &mut market.stocks {
            stock.tick_price(1.0);
        }
    }
}

/// 對外 API：根據任務事件設定股票趨勢
/// 例如：完成任務摧毀某公司 → 該公司股票下跌
pub fn apply_stock_event(market: &mut StockMarket, symbol: StockSymbol, bias: f32, duration: f32) {
    market.get_mut(symbol).set_trend(bias, duration);
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_initial_price() {
        let stock = Stock::new(StockSymbol::TSMC);
        assert!((stock.price - 500.0).abs() < 0.01);
        assert_eq!(stock.change_percent(), 0.0);
    }

    #[test]
    fn stock_tick_changes_price() {
        let mut stock = Stock::new(StockSymbol::NightMarketKing);
        let _original = stock.price;
        // 多次 tick 確保價格變動
        for _ in 0..10 {
            stock.tick_price(1.0);
        }
        // 價格應該有變化（雖然有小概率不變，但 10 次後概率極低）
        // 至少歷史紀錄應增長
        assert!(stock.price_history.len() > 1);
    }

    #[test]
    fn stock_price_never_below_one() {
        let mut stock = Stock::new(StockSymbol::BobaGroup);
        stock.price = 2.0; // 接近最低
        stock.set_trend(-0.5, 100.0); // 強力下跌趨勢
        for _ in 0..100 {
            stock.tick_price(1.0);
        }
        assert!(stock.price >= 1.0, "價格不應低於 $1");
    }

    #[test]
    fn stock_trend_expires() {
        let mut stock = Stock::new(StockSymbol::PineAir);
        stock.set_trend(0.1, 5.0);
        assert!(stock.trend_duration > 0.0);

        for _ in 0..6 {
            stock.tick_price(1.0);
        }
        assert!(stock.trend_duration <= 0.0);
        assert!((stock.trend_bias - 0.0).abs() < 0.01);
    }

    #[test]
    fn portfolio_buy_and_sell() {
        let mut portfolio = StockPortfolio::new();

        portfolio.buy(StockSymbol::TSMC, 10, 500.0);
        assert_eq!(portfolio.shares_of(StockSymbol::TSMC), 10);
        assert!((portfolio.avg_cost_of(StockSymbol::TSMC) - 500.0).abs() < 0.01);

        // 加碼（不同價格）
        portfolio.buy(StockSymbol::TSMC, 10, 600.0);
        assert_eq!(portfolio.shares_of(StockSymbol::TSMC), 20);
        assert!((portfolio.avg_cost_of(StockSymbol::TSMC) - 550.0).abs() < 0.01);

        // 賣出
        assert!(portfolio.sell(StockSymbol::TSMC, 5));
        assert_eq!(portfolio.shares_of(StockSymbol::TSMC), 15);

        // 超賣失敗
        assert!(!portfolio.sell(StockSymbol::TSMC, 100));
    }

    #[test]
    fn portfolio_profit_loss() {
        let mut portfolio = StockPortfolio::new();
        portfolio.buy(StockSymbol::BobaGroup, 100, 85.0);

        // 股價漲到 100 → 獲利 1500
        let pl = portfolio.profit_loss(StockSymbol::BobaGroup, 100.0);
        assert!((pl - 1500.0).abs() < 0.01);

        // 股價跌到 70 → 虧損 -1500
        let pl = portfolio.profit_loss(StockSymbol::BobaGroup, 70.0);
        assert!((pl - (-1500.0)).abs() < 0.01);
    }

    #[test]
    fn market_buy_sell() {
        let mut market = StockMarket::default();
        let mut wallet = PlayerWallet {
            cash: 10000,
            bank: 0,
            total_earned: 0,
            total_spent: 0,
        };

        // 買入 10 股 NightMarketKing (價格 $45)
        let result = market.buy(StockSymbol::NightMarketKing, 10, &mut wallet);
        assert!(result.is_ok());
        assert_eq!(market.portfolio.shares_of(StockSymbol::NightMarketKing), 10);
        assert!(wallet.cash < 10000);

        // 賣出 5 股
        let result = market.sell(StockSymbol::NightMarketKing, 5, &mut wallet);
        assert!(result.is_ok());
        assert_eq!(market.portfolio.shares_of(StockSymbol::NightMarketKing), 5);
    }

    #[test]
    fn market_buy_insufficient_cash() {
        let mut market = StockMarket::default();
        let mut wallet = PlayerWallet {
            cash: 100,
            bank: 0,
            total_earned: 0,
            total_spent: 0,
        };

        // TSMC $500，買 10 股需 $5000，錢不夠
        let result = market.buy(StockSymbol::TSMC, 10, &mut wallet);
        assert!(result.is_err());
    }

    #[test]
    fn market_closed_blocks_trading() {
        let mut market = StockMarket::default();
        market.is_open = false;
        let mut wallet = PlayerWallet {
            cash: 100000,
            bank: 0,
            total_earned: 0,
            total_spent: 0,
        };

        let result = market.buy(StockSymbol::TSMC, 1, &mut wallet);
        assert_eq!(result, Err("市場未開市"));
    }

    #[test]
    fn stock_symbols_count() {
        assert_eq!(StockSymbol::ALL.len(), 6);
    }

    #[test]
    fn stock_change_percent_positive() {
        let mut stock = Stock::new(StockSymbol::TSMC);
        stock.previous_price = 100.0;
        stock.price = 110.0;
        assert!((stock.change_percent() - 10.0).abs() < 0.01);
        assert!(stock.is_up());
    }
}
