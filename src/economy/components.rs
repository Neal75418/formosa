//! 經濟系統組件
//!
//! 包含錢包、商店、ATM、商品等定義

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 玩家錢包 (單一金錢來源)
// ============================================================================

/// 玩家錢包 - 統一的金錢管理資源
#[derive(Resource, Serialize, Deserialize)]
pub struct PlayerWallet {
    /// 現金（身上）
    pub cash: i32,
    /// 銀行存款
    pub bank: i32,
    /// 累計收入（統計用）
    pub total_earned: i32,
    /// 累計支出（統計用）
    pub total_spent: i32,
}

impl Default for PlayerWallet {
    fn default() -> Self {
        Self {
            cash: 5000,
            bank: 10000,
            total_earned: 0,
            total_spent: 0,
        }
    }
}

impl PlayerWallet {
    /// 獲取總資產
    pub fn total(&self) -> i32 {
        self.cash.saturating_add(self.bank)
    }

    /// 增加現金
    pub fn add_cash(&mut self, amount: i32) -> i32 {
        self.cash = self.cash.saturating_add(amount);
        if amount > 0 {
            self.total_earned = self.total_earned.saturating_add(amount);
        }
        self.cash
    }

    /// 花費現金（如果足夠且金額為正）
    pub fn spend_cash(&mut self, amount: i32) -> bool {
        if amount >= 0 && self.cash >= amount {
            self.cash = self.cash.saturating_sub(amount);
            self.total_spent = self.total_spent.saturating_add(amount);
            true
        } else {
            false
        }
    }

    /// 盡可能花費現金（用於罰款等情況，不足部分豁免）
    /// 返回實際支付的金額
    pub fn spend_up_to(&mut self, amount: i32) -> i32 {
        let actual = amount.min(self.cash).max(0);
        if actual > 0 {
            self.cash = self.cash.saturating_sub(actual);
            self.total_spent = self.total_spent.saturating_add(actual);
        }
        actual
    }

    /// 存款到銀行
    pub fn deposit(&mut self, amount: i32) -> bool {
        if self.cash >= amount && amount > 0 {
            self.cash = self.cash.saturating_sub(amount);
            self.bank = self.bank.saturating_add(amount);
            true
        } else {
            false
        }
    }

    /// 從銀行提款
    pub fn withdraw(&mut self, amount: i32) -> bool {
        if self.bank >= amount && amount > 0 {
            self.bank = self.bank.saturating_sub(amount);
            self.cash = self.cash.saturating_add(amount);
            true
        } else {
            false
        }
    }
}

// ============================================================================
// 商店系統
// ============================================================================

/// 商店類型
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ShopType {
    /// 便利商店（補給品）
    ConvenienceStore,
    /// 武器店
    WeaponShop,
    /// 服裝店
    ClothingStore,
    /// 車輛經銷商
    VehicleDealer,
    /// 改裝店
    ModShop,
}

impl ShopType {
    /// 取得商店名稱
    pub fn name(&self) -> &'static str {
        match self {
            ShopType::ConvenienceStore => "便利商店",
            ShopType::WeaponShop => "槍械行",
            ShopType::ClothingStore => "服飾店",
            ShopType::VehicleDealer => "車行",
            ShopType::ModShop => "改裝廠",
        }
    }
}

/// 商店組件
#[derive(Component)]
pub struct Shop {
    /// 商店類型
    pub shop_type: ShopType,
    /// 商店名稱
    pub name: String,
    /// 是否營業中
    pub is_open: bool,
    /// 營業開始時間 (0-24)
    pub open_hour: f32,
    /// 營業結束時間 (0-24)
    pub close_hour: f32,
}

impl Shop {
    /// 建立新實例
    pub fn new(shop_type: ShopType, name: impl Into<String>) -> Self {
        Self {
            shop_type,
            name: name.into(),
            is_open: true,
            open_hour: 6.0,
            close_hour: 24.0,
        }
    }

    /// 24 小時營業
    pub fn always_open(mut self) -> Self {
        self.open_hour = 0.0;
        self.close_hour = 24.0;
        self
    }

    /// 檢查是否在營業時間
    pub fn is_open_at(&self, hour: f32) -> bool {
        if self.open_hour <= self.close_hour {
            hour >= self.open_hour && hour < self.close_hour
        } else {
            // 跨日營業（如 22:00 - 06:00）
            hour >= self.open_hour || hour < self.close_hour
        }
    }
}

/// 商品類別
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ItemCategory {
    /// 食物（回血）
    Food,
    /// 飲料（回體力）
    Drink,
    /// 彈藥
    Ammo,
    /// 護甲
    Armor,
    /// 武器
    Weapon,
    /// 服裝
    Clothing,
    /// 車輛配件
    VehiclePart,
}

/// 商品定義
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShopItem {
    /// 商品 ID
    pub id: String,
    /// 商品名稱
    pub name: String,
    /// 商品描述
    pub description: String,
    /// 類別
    pub category: ItemCategory,
    /// 價格
    pub price: i32,
    /// 庫存數量（-1 = 無限）
    pub stock: i32,
    /// 效果值（如回復量、傷害等）
    pub effect_value: f32,
}

impl ShopItem {
    /// 建立新實例
    pub fn new(id: impl Into<String>, name: impl Into<String>, category: ItemCategory, price: i32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            category,
            price,
            stock: -1,
            effect_value: 0.0,
        }
    }

    /// 設定描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 設定效果
    pub fn with_effect(mut self, value: f32) -> Self {
        self.effect_value = value;
        self
    }

    /// 設定庫存
    pub fn with_stock(mut self, stock: i32) -> Self {
        self.stock = stock;
        self
    }
}

/// 商店庫存資源
#[derive(Resource, Default)]
pub struct ShopInventory {
    /// 便利商店商品
    pub convenience_items: Vec<ShopItem>,
    /// 武器店商品
    pub weapon_items: Vec<ShopItem>,
}

impl ShopInventory {
    /// 建立新實例
    pub fn new() -> Self {
        let mut inventory = Self::default();
        inventory.init_convenience_store();
        inventory.init_weapon_shop();
        inventory
    }

    fn init_convenience_store(&mut self) {
        self.convenience_items = vec![
            ShopItem::new("food_bento", "便當", ItemCategory::Food, 80)
                .with_description("回復 50 HP")
                .with_effect(50.0),
            ShopItem::new("food_onigiri", "飯糰", ItemCategory::Food, 30)
                .with_description("回復 20 HP")
                .with_effect(20.0),
            ShopItem::new("drink_tea", "茶裏王", ItemCategory::Drink, 25)
                .with_description("回復 10 HP")
                .with_effect(10.0),
            ShopItem::new("drink_energy", "蠻牛", ItemCategory::Drink, 40)
                .with_description("回復 25 HP，短暫加速")
                .with_effect(25.0),
            ShopItem::new("armor_vest", "防彈背心", ItemCategory::Armor, 500)
                .with_description("護甲 +50")
                .with_effect(50.0),
        ];
    }

    fn init_weapon_shop(&mut self) {
        self.weapon_items = vec![
            ShopItem::new("ammo_pistol", "手槍彈匣", ItemCategory::Ammo, 50)
                .with_description("手槍彈藥 x30")
                .with_effect(30.0),
            ShopItem::new("ammo_smg", "衝鋒槍彈匣", ItemCategory::Ammo, 100)
                .with_description("衝鋒槍彈藥 x60")
                .with_effect(60.0),
            ShopItem::new("ammo_shotgun", "霰彈", ItemCategory::Ammo, 80)
                .with_description("霰彈 x12")
                .with_effect(12.0),
            ShopItem::new("ammo_rifle", "步槍彈匣", ItemCategory::Ammo, 150)
                .with_description("步槍彈藥 x30")
                .with_effect(30.0),
            ShopItem::new("weapon_pistol", "9mm 手槍", ItemCategory::Weapon, 1500)
                .with_description("基礎手槍")
                .with_stock(1),
            ShopItem::new("weapon_smg", "衝鋒槍", ItemCategory::Weapon, 5000)
                .with_description("高射速")
                .with_stock(1),
            ShopItem::new("weapon_shotgun", "霰彈槍", ItemCategory::Weapon, 8000)
                .with_description("近距離高傷害")
                .with_stock(1),
            ShopItem::new("weapon_rifle", "突擊步槍", ItemCategory::Weapon, 15000)
                .with_description("中遠距離")
                .with_stock(1),
            ShopItem::new("armor_heavy", "重型防彈衣", ItemCategory::Armor, 2000)
                .with_description("護甲 +100")
                .with_effect(100.0),
        ];
    }

    /// 取得指定商店類型的商品列表
    pub fn get_items(&self, shop_type: ShopType) -> &[ShopItem] {
        match shop_type {
            ShopType::ConvenienceStore => &self.convenience_items,
            ShopType::WeaponShop => &self.weapon_items,
            _ => &[],
        }
    }
}

// ============================================================================
// ATM 系統
// ============================================================================

/// ATM 組件
#[derive(Component)]
pub struct Atm {
    /// ATM 名稱/位置
    pub name: String,
    /// 是否可用
    pub is_functional: bool,
    /// 單次最大提款金額
    pub max_withdrawal: i32,
}

impl Default for Atm {
    fn default() -> Self {
        Self {
            name: "ATM".to_string(),
            is_functional: true,
            max_withdrawal: 10000,
        }
    }
}

// ============================================================================
// 事件
// ============================================================================

/// 金錢變動事件
#[derive(Message)]
pub struct MoneyChangedEvent {
    /// 變動金額（正=獲得，負=支出）
    pub amount: i32,
    /// 變動原因
    pub reason: MoneyChangeReason,
    /// 變動後餘額
    pub new_balance: i32,
}

/// 金錢變動原因
#[derive(Clone, Copy, Debug)]
pub enum MoneyChangeReason {
    /// 任務獎勵
    MissionReward,
    /// 撿取掉落
    Pickup,
    /// 購物
    Purchase,
    /// ATM 提款
    AtmWithdraw,
    /// ATM 存款
    AtmDeposit,
    /// 罰款
    Fine,
    /// 其他
    Other,
}

/// 購買事件
#[derive(Message)]
pub struct PurchaseEvent {
    /// 商品 ID
    pub item_id: String,
    /// 商店實體
    pub shop_entity: Entity,
    /// 購買數量
    pub quantity: i32,
}

/// 交易事件（用於處理各種交易）
#[derive(Message)]
pub struct TransactionEvent {
    /// 交易類型
    pub transaction_type: TransactionType,
    /// 金額
    pub amount: i32,
}

/// 交易類型
#[derive(Clone, Copy, Debug)]
pub enum TransactionType {
    /// 銀行存款
    BankDeposit,
    /// 銀行提款
    BankWithdraw,
    /// 購買商品
    BuyItem,
    /// 支付罰款
    PayFine,
}

// ============================================================================
// 可互動標記
// ============================================================================

/// 可互動物件標記（用於商店、ATM 等）
#[derive(Component)]
pub struct Interactable {
    /// 互動提示文字
    pub prompt: String,
    /// 互動距離
    pub range: f32,
    /// 互動類型
    pub interaction_type: InteractionType,
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            prompt: "按 F 互動".to_string(),
            range: 3.0,
            interaction_type: InteractionType::Generic,
        }
    }
}

/// 互動類型
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InteractionType {
    /// 商店
    Shop,
    /// ATM
    Atm,
    /// 門
    Door,
    /// 撿取物品
    Pickup,
    /// 通用
    Generic,
}

/// 金錢掉落物
#[derive(Component)]
pub struct CashPickup {
    /// 金額
    pub amount: i32,
    /// 存在時間
    pub lifetime: f32,
    /// 最大存在時間
    pub max_lifetime: f32,
}

impl CashPickup {
    /// 建立新實例
    pub fn new(amount: i32) -> Self {
        Self {
            amount,
            lifetime: 0.0,
            max_lifetime: 60.0, // 60 秒後消失
        }
    }
}

// ============================================================================
// UI 狀態
// ============================================================================

/// 商店選單狀態
#[derive(Resource, Default)]
pub struct ShopMenuState {
    /// 是否開啟
    pub is_open: bool,
    /// 當前商店實體
    pub current_shop: Option<Entity>,
    /// 選中的商品索引
    pub selected_index: usize,
    /// 當前商店類型
    pub shop_type: Option<ShopType>,
}

/// ATM 選單狀態
#[derive(Resource, Default)]
pub struct AtmMenuState {
    /// 是否開啟
    pub is_open: bool,
    /// 當前 ATM 實體
    pub current_atm: Option<Entity>,
    /// 輸入金額
    pub input_amount: i32,
    /// 當前操作模式
    pub mode: AtmMode,
}

/// ATM 操作模式
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum AtmMode {
    #[default]
    Main,      // 主選單
    Deposit,   // 存款
    Withdraw,  // 提款
    Balance,   // 查詢餘額
}
