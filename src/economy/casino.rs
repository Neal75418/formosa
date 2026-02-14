//! 賭場小遊戲系統
//!
//! 提供 21 點（Blackjack）和老虎機（Slot Machine）兩種賭場遊戲。
//! 玩家在賭場區域內可開啟遊戲 UI，用現金下注。

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]
// Bevy 系統需要 Res<T> 按值傳遞
#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::PlayerWallet;

// ============================================================================
// 共用定義
// ============================================================================

/// 賭場遊戲類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CasinoGameType {
    Blackjack,
    SlotMachine,
}

/// 賭場互動區域標記
#[derive(Component)]
pub struct CasinoZone {
    pub game_type: CasinoGameType,
}

/// 賭場 UI 狀態
#[derive(Resource, Default)]
pub struct CasinoMenuState {
    /// 是否開啟中
    pub is_open: bool,
    /// 當前遊戲類型
    pub active_game: Option<CasinoGameType>,
}

// ============================================================================
// 撲克牌定義（21 點用）
// ============================================================================

/// 花色
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
}

/// 牌面
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rank {
    Ace,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    pub const ALL: [Rank; 13] = [
        Rank::Ace, Rank::Two, Rank::Three, Rank::Four, Rank::Five,
        Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten,
        Rank::Jack, Rank::Queen, Rank::King,
    ];

    /// 牌面點數（Ace 先算 11，之後視情況降為 1）
    pub fn value(&self) -> u32 {
        match self {
            Rank::Ace => 11, Rank::Two => 2, Rank::Three => 3,
            Rank::Four => 4, Rank::Five => 5, Rank::Six => 6,
            Rank::Seven => 7, Rank::Eight => 8, Rank::Nine => 9,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King => 10,
        }
    }
}

/// 一張牌
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }

    pub fn value(&self) -> u32 {
        self.rank.value()
    }
}

/// 計算手牌總點數（自動處理 Ace 1/11 轉換）
pub fn hand_value(cards: &[Card]) -> u32 {
    let mut total: u32 = cards.iter().map(|c| c.value()).sum();
    let mut aces = cards.iter().filter(|c| c.rank == Rank::Ace).count();

    // Ace 從 11 降為 1 直到不爆牌
    while total > 21 && aces > 0 {
        total -= 10;
        aces -= 1;
    }
    total
}

/// 是否爆牌
pub fn is_bust(cards: &[Card]) -> bool {
    hand_value(cards) > 21
}

/// 是否為 Blackjack（兩張牌正好 21 點）
pub fn is_blackjack(cards: &[Card]) -> bool {
    cards.len() == 2 && hand_value(cards) == 21
}

// ============================================================================
// 牌組
// ============================================================================

/// 一副牌（洗牌後使用）
#[derive(Clone, Debug)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    /// 建立一副完整的 52 張牌
    pub fn new() -> Self {
        let mut cards = Vec::with_capacity(52);
        for &suit in &Suit::ALL {
            for &rank in &Rank::ALL {
                cards.push(Card::new(suit, rank));
            }
        }
        Self { cards }
    }

    /// Fisher-Yates 洗牌
    pub fn shuffle(&mut self) {
        let len = self.cards.len();
        for i in (1..len).rev() {
            let j = (rand::random::<f32>() * (i + 1) as f32) as usize;
            let j = j.min(i); // 安全邊界
            self.cards.swap(i, j);
        }
    }

    /// 抽一張牌
    pub fn draw(&mut self) -> Option<Card> {
        self.cards.pop()
    }
}

// ============================================================================
// 21 點遊戲狀態
// ============================================================================

/// 21 點遊戲階段
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlackjackPhase {
    /// 等待下注
    #[default]
    Betting,
    /// 玩家回合
    PlayerTurn,
    /// 莊家回合
    DealerTurn,
    /// 結算
    Result,
}

/// 21 點遊戲結果
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlackjackResult {
    /// 玩家 Blackjack（1.5 倍賠率）
    PlayerBlackjack,
    /// 玩家勝
    PlayerWin,
    /// 莊家勝
    DealerWin,
    /// 平手（退還賭注）
    Push,
}

/// 21 點遊戲資源
#[derive(Resource)]
pub struct BlackjackGame {
    pub deck: Deck,
    pub player_hand: Vec<Card>,
    pub dealer_hand: Vec<Card>,
    pub bet: i32,
    pub phase: BlackjackPhase,
    pub result: Option<BlackjackResult>,
}

impl Default for BlackjackGame {
    fn default() -> Self {
        let mut deck = Deck::new();
        deck.shuffle();
        Self {
            deck,
            player_hand: Vec::new(),
            dealer_hand: Vec::new(),
            bet: 0,
            phase: BlackjackPhase::Betting,
            result: None,
        }
    }
}

impl BlackjackGame {
    /// 開始新一局（下注並發牌）
    pub fn start_round(&mut self, bet: i32) {
        // 如果牌不夠，重新洗牌
        if self.deck.cards.len() < 10 {
            self.deck = Deck::new();
            self.deck.shuffle();
        }

        self.player_hand.clear();
        self.dealer_hand.clear();
        self.bet = bet;
        self.result = None;

        // 發兩張牌給玩家和莊家（交替發牌）
        self.player_hand.push(self.deck.draw().unwrap());
        self.dealer_hand.push(self.deck.draw().unwrap());
        self.player_hand.push(self.deck.draw().unwrap());
        self.dealer_hand.push(self.deck.draw().unwrap());

        // 檢查 Blackjack
        if is_blackjack(&self.player_hand) {
            self.phase = BlackjackPhase::Result;
            if is_blackjack(&self.dealer_hand) {
                self.result = Some(BlackjackResult::Push);
            } else {
                self.result = Some(BlackjackResult::PlayerBlackjack);
            }
        } else {
            self.phase = BlackjackPhase::PlayerTurn;
        }
    }

    /// 玩家要牌（Hit）
    pub fn hit(&mut self) {
        if self.phase != BlackjackPhase::PlayerTurn {
            return;
        }

        if let Some(card) = self.deck.draw() {
            self.player_hand.push(card);
        }

        if is_bust(&self.player_hand) {
            self.phase = BlackjackPhase::Result;
            self.result = Some(BlackjackResult::DealerWin);
        }
    }

    /// 玩家停牌（Stand）→ 莊家回合
    pub fn stand(&mut self) {
        if self.phase != BlackjackPhase::PlayerTurn {
            return;
        }
        self.phase = BlackjackPhase::DealerTurn;
        self.play_dealer();
    }

    /// 莊家按規則補牌（17 以下必須補）
    fn play_dealer(&mut self) {
        while hand_value(&self.dealer_hand) < 17 {
            if let Some(card) = self.deck.draw() {
                self.dealer_hand.push(card);
            } else {
                break;
            }
        }

        self.phase = BlackjackPhase::Result;

        let player_val = hand_value(&self.player_hand);
        let dealer_val = hand_value(&self.dealer_hand);

        if is_bust(&self.dealer_hand) || player_val > dealer_val {
            self.result = Some(BlackjackResult::PlayerWin);
        } else if dealer_val > player_val {
            self.result = Some(BlackjackResult::DealerWin);
        } else {
            self.result = Some(BlackjackResult::Push);
        }
    }

    /// 根據結果計算淨收益（正=贏、負=輸、0=平）
    pub fn payout(&self) -> i32 {
        match self.result {
            Some(BlackjackResult::PlayerBlackjack) => (self.bet as f32 * 1.5) as i32,
            Some(BlackjackResult::PlayerWin) => self.bet,
            Some(BlackjackResult::DealerWin) => -self.bet,
            Some(BlackjackResult::Push) | None => 0,
        }
    }

    /// 玩家點數
    pub fn player_value(&self) -> u32 {
        hand_value(&self.player_hand)
    }

    /// 莊家點數
    pub fn dealer_value(&self) -> u32 {
        hand_value(&self.dealer_hand)
    }
}

// ============================================================================
// 老虎機
// ============================================================================

/// 老虎機圖案
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotSymbol {
    Cherry,
    Lemon,
    Orange,
    Bell,
    Bar,
    Seven,
}

impl SlotSymbol {
    pub const ALL: [SlotSymbol; 6] = [
        SlotSymbol::Cherry,
        SlotSymbol::Lemon,
        SlotSymbol::Orange,
        SlotSymbol::Bell,
        SlotSymbol::Bar,
        SlotSymbol::Seven,
    ];

    /// 圖案顯示文字
    pub fn label(&self) -> &'static str {
        match self {
            SlotSymbol::Cherry => "🍒",
            SlotSymbol::Lemon => "🍋",
            SlotSymbol::Orange => "🍊",
            SlotSymbol::Bell => "🔔",
            SlotSymbol::Bar => "BAR",
            SlotSymbol::Seven => "777",
        }
    }

    /// 隨機產生一個圖案（加權：Cherry/Lemon 較常出現）
    pub fn random() -> Self {
        let roll = rand::random::<f32>();
        if roll < 0.25 {
            SlotSymbol::Cherry
        } else if roll < 0.45 {
            SlotSymbol::Lemon
        } else if roll < 0.60 {
            SlotSymbol::Orange
        } else if roll < 0.75 {
            SlotSymbol::Bell
        } else if roll < 0.90 {
            SlotSymbol::Bar
        } else {
            SlotSymbol::Seven // 10% 機率
        }
    }
}

/// 老虎機遊戲資源
#[derive(Resource, Default)]
pub struct SlotMachine {
    /// 三個滾輪結果
    pub reels: [Option<SlotSymbol>; 3],
    /// 當前下注
    pub bet: i32,
    /// 上次贏得的金額（UI 顯示用）
    pub last_win: i32,
    /// 是否正在轉動（動畫用）
    pub spinning: bool,
}

impl SlotMachine {
    /// 拉桿！
    pub fn spin(&mut self, bet: i32) {
        self.bet = bet;
        self.reels = [
            Some(SlotSymbol::random()),
            Some(SlotSymbol::random()),
            Some(SlotSymbol::random()),
        ];
        self.spinning = false;
        self.last_win = self.calculate_payout();
    }

    /// 計算賠率
    pub fn calculate_payout(&self) -> i32 {
        let r = self.reels;
        let (Some(a), Some(b), Some(c)) = (r[0], r[1], r[2]) else {
            return 0;
        };

        // 三個一樣
        if a == b && b == c {
            let multiplier = match a {
                SlotSymbol::Seven => 50,   // 777 大獎
                SlotSymbol::Bar => 20,     // BAR BAR BAR
                SlotSymbol::Bell => 15,    // 三鈴鐺
                SlotSymbol::Orange => 10,  // 三橘子
                SlotSymbol::Lemon => 5,    // 三檸檬
                SlotSymbol::Cherry => 3,   // 三櫻桃
            };
            return self.bet * multiplier;
        }

        // 兩個櫻桃
        let cherry_count = [a, b, c]
            .iter()
            .filter(|&&s| s == SlotSymbol::Cherry)
            .count();
        if cherry_count == 2 {
            return self.bet * 2;
        }

        // 一個櫻桃退回賭注
        if cherry_count == 1 {
            return self.bet;
        }

        // 沒中
        0
    }

    /// 淨收益（扣除賭注）
    pub fn net_payout(&self) -> i32 {
        self.last_win - self.bet
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 21 點下注系統（UI 觸發）
pub fn blackjack_bet_system(
    mut game: ResMut<BlackjackGame>,
    mut wallet: ResMut<PlayerWallet>,
    menu: Res<CasinoMenuState>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if !menu.is_open || menu.active_game != Some(CasinoGameType::Blackjack) {
        return;
    }

    if game.phase != BlackjackPhase::Betting {
        return;
    }

    // 數字鍵 1-4 對應不同賭注金額
    let bet = if input.just_pressed(KeyCode::Digit1) {
        100
    } else if input.just_pressed(KeyCode::Digit2) {
        500
    } else if input.just_pressed(KeyCode::Digit3) {
        1000
    } else if input.just_pressed(KeyCode::Digit4) {
        5000
    } else {
        return;
    };

    if wallet.spend_cash(bet) {
        game.start_round(bet);
    }
}

/// 21 點玩家操作系統
pub fn blackjack_play_system(
    mut game: ResMut<BlackjackGame>,
    mut wallet: ResMut<PlayerWallet>,
    menu: Res<CasinoMenuState>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if !menu.is_open || menu.active_game != Some(CasinoGameType::Blackjack) {
        return;
    }

    match game.phase {
        BlackjackPhase::PlayerTurn => {
            if input.just_pressed(KeyCode::KeyH) {
                // Hit
                game.hit();
            } else if input.just_pressed(KeyCode::KeyS) {
                // Stand
                game.stand();
            }
        }
        BlackjackPhase::Result => {
            // 結算後按 Enter 領取獎金並重置
            if input.just_pressed(KeyCode::Enter) {
                let payout = game.payout();
                if payout > 0 {
                    // 贏：退回原始賭注 + 淨贏利
                    wallet.add_cash(game.bet + payout);
                } else if payout == 0 {
                    // Push：退還賭注
                    wallet.add_cash(game.bet);
                }
                // DealerWin (payout < 0)：賭注已扣，無需退還
                game.phase = BlackjackPhase::Betting;
                game.result = None;
            }
        }
        _ => {}
    }
}

/// 老虎機系統
pub fn slot_machine_system(
    mut slot: ResMut<SlotMachine>,
    mut wallet: ResMut<PlayerWallet>,
    menu: Res<CasinoMenuState>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if !menu.is_open || menu.active_game != Some(CasinoGameType::SlotMachine) {
        return;
    }

    // 數字鍵選擇賭注，Space 拉桿
    let bet = if input.just_pressed(KeyCode::Digit1) {
        Some(50)
    } else if input.just_pressed(KeyCode::Digit2) {
        Some(100)
    } else if input.just_pressed(KeyCode::Digit3) {
        Some(500)
    } else {
        None
    };

    if let Some(b) = bet {
        slot.bet = b;
    }

    let current_bet = slot.bet;
    if input.just_pressed(KeyCode::Space) && current_bet > 0 && wallet.spend_cash(current_bet) {
        slot.spin(current_bet);

        // 發放獎金
        if slot.last_win > 0 {
            wallet.add_cash(slot.last_win);
        }
    }
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_values() {
        assert_eq!(Card::new(Suit::Spades, Rank::Ace).value(), 11);
        assert_eq!(Card::new(Suit::Hearts, Rank::King).value(), 10);
        assert_eq!(Card::new(Suit::Diamonds, Rank::Five).value(), 5);
    }

    #[test]
    fn hand_value_simple() {
        let hand = vec![
            Card::new(Suit::Spades, Rank::Ten),
            Card::new(Suit::Hearts, Rank::Seven),
        ];
        assert_eq!(hand_value(&hand), 17);
    }

    #[test]
    fn hand_value_ace_adjustment() {
        // Ace + 10 + 5 = 26 → Ace 變 1 → 16
        let hand = vec![
            Card::new(Suit::Spades, Rank::Ace),
            Card::new(Suit::Hearts, Rank::Ten),
            Card::new(Suit::Diamonds, Rank::Five),
        ];
        assert_eq!(hand_value(&hand), 16);
    }

    #[test]
    fn hand_value_double_ace() {
        // Ace + Ace = 22 → 一個 Ace 變 1 → 12
        let hand = vec![
            Card::new(Suit::Spades, Rank::Ace),
            Card::new(Suit::Hearts, Rank::Ace),
        ];
        assert_eq!(hand_value(&hand), 12);
    }

    #[test]
    fn blackjack_detection() {
        let hand = vec![
            Card::new(Suit::Spades, Rank::Ace),
            Card::new(Suit::Hearts, Rank::King),
        ];
        assert!(is_blackjack(&hand));
        assert!(!is_bust(&hand));

        let not_bj = vec![
            Card::new(Suit::Spades, Rank::Ten),
            Card::new(Suit::Hearts, Rank::Five),
            Card::new(Suit::Diamonds, Rank::Six),
        ];
        assert!(!is_blackjack(&not_bj)); // 21 但 3 張牌
    }

    #[test]
    fn bust_detection() {
        let hand = vec![
            Card::new(Suit::Spades, Rank::Ten),
            Card::new(Suit::Hearts, Rank::Ten),
            Card::new(Suit::Diamonds, Rank::Five),
        ];
        assert!(is_bust(&hand)); // 25
    }

    #[test]
    fn blackjack_game_start_round() {
        let mut game = BlackjackGame::default();
        game.start_round(100);

        assert_eq!(game.bet, 100);
        assert_eq!(game.player_hand.len(), 2);
        assert_eq!(game.dealer_hand.len(), 2);
        // phase 應為 PlayerTurn 或 Result（若 Blackjack）
        assert!(
            game.phase == BlackjackPhase::PlayerTurn || game.phase == BlackjackPhase::Result
        );
    }

    #[test]
    fn blackjack_hit_and_bust() {
        let mut game = BlackjackGame::default();
        // 手動設定手牌讓玩家接近爆牌
        game.phase = BlackjackPhase::PlayerTurn;
        game.player_hand = vec![
            Card::new(Suit::Spades, Rank::Ten),
            Card::new(Suit::Hearts, Rank::Ten),
        ];
        // 20 點再 hit 很可能爆牌，但結果取決於抽到的牌
        game.hit();
        // 如果抽到 2-Ace，不爆牌繼續；否則爆牌
        if is_bust(&game.player_hand) {
            assert_eq!(game.phase, BlackjackPhase::Result);
            assert_eq!(game.result, Some(BlackjackResult::DealerWin));
        } else {
            assert_eq!(game.phase, BlackjackPhase::PlayerTurn);
        }
    }

    #[test]
    fn blackjack_stand_triggers_dealer() {
        let mut game = BlackjackGame::default();
        game.start_round(100);

        // 如果玩家有 Blackjack，跳過 stand 測試
        if game.phase == BlackjackPhase::PlayerTurn {
            game.stand();
            assert_eq!(game.phase, BlackjackPhase::Result);
            assert!(game.result.is_some());
        }
    }

    #[test]
    fn blackjack_payout_values() {
        let mut game = BlackjackGame::default();
        game.bet = 1000;

        game.result = Some(BlackjackResult::PlayerBlackjack);
        assert_eq!(game.payout(), 1500); // 1.5x

        game.result = Some(BlackjackResult::PlayerWin);
        assert_eq!(game.payout(), 1000); // 1x

        game.result = Some(BlackjackResult::DealerWin);
        assert_eq!(game.payout(), -1000); // -1x

        game.result = Some(BlackjackResult::Push);
        assert_eq!(game.payout(), 0); // 退還
    }

    #[test]
    fn deck_has_52_cards() {
        let deck = Deck::new();
        assert_eq!(deck.cards.len(), 52);
    }

    #[test]
    fn deck_draw_reduces_count() {
        let mut deck = Deck::new();
        deck.shuffle();
        deck.draw();
        assert_eq!(deck.cards.len(), 51);
    }

    #[test]
    fn slot_machine_three_sevens() {
        let mut slot = SlotMachine::default();
        slot.bet = 100;
        slot.reels = [
            Some(SlotSymbol::Seven),
            Some(SlotSymbol::Seven),
            Some(SlotSymbol::Seven),
        ];
        slot.last_win = slot.calculate_payout();
        assert_eq!(slot.last_win, 5000); // 50x
    }

    #[test]
    fn slot_machine_three_bars() {
        let mut slot = SlotMachine::default();
        slot.bet = 100;
        slot.reels = [
            Some(SlotSymbol::Bar),
            Some(SlotSymbol::Bar),
            Some(SlotSymbol::Bar),
        ];
        slot.last_win = slot.calculate_payout();
        assert_eq!(slot.last_win, 2000); // 20x
    }

    #[test]
    fn slot_machine_two_cherries() {
        let mut slot = SlotMachine::default();
        slot.bet = 100;
        slot.reels = [
            Some(SlotSymbol::Cherry),
            Some(SlotSymbol::Cherry),
            Some(SlotSymbol::Bell),
        ];
        slot.last_win = slot.calculate_payout();
        assert_eq!(slot.last_win, 200); // 2x
    }

    #[test]
    fn slot_machine_no_match() {
        let mut slot = SlotMachine::default();
        slot.bet = 100;
        slot.reels = [
            Some(SlotSymbol::Bell),
            Some(SlotSymbol::Lemon),
            Some(SlotSymbol::Orange),
        ];
        slot.last_win = slot.calculate_payout();
        assert_eq!(slot.last_win, 0);
    }

    #[test]
    fn slot_machine_net_payout() {
        let mut slot = SlotMachine::default();
        slot.bet = 100;
        slot.last_win = 200;
        assert_eq!(slot.net_payout(), 100); // 200 - 100

        slot.last_win = 0;
        assert_eq!(slot.net_payout(), -100); // 0 - 100
    }

    #[test]
    fn slot_symbol_random_coverage() {
        // 產生 100 個隨機圖案，確認範圍正確
        for _ in 0..100 {
            let sym = SlotSymbol::random();
            assert!(SlotSymbol::ALL.contains(&sym));
        }
    }
}
