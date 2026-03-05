//! 企業管理系統
//!
//! 玩家可購買並經營各類企業（夜市攤位、便利商店、酒吧等）。
//! 每間企業可僱用員工，員工數量和升級影響日收入。

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]
// 遊戲數學常用 f32/i32/u32 互轉，允許精度與截斷轉型
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
// 小型 Copy 結構上的 &self 方法保留 Rust 慣用風格

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::PlayerWallet;
use crate::core::WorldTime;

// ============================================================================
// 企業類型
// ============================================================================

/// 企業類型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnterpriseType {
    /// 夜市攤位（低成本、穩定收入）
    NightMarketStall,
    /// 便利商店（中等投資）
    ConvenienceStore,
    /// 泡沫紅茶店（台灣特色）
    BubbleTeaShop,
    /// 機車行（修車 + 販售）
    MotorcycleShop,
    /// 酒吧（高收入、需管理）
    Bar,
    /// 夜店（最高收入、最高風險）
    Nightclub,
}

impl EnterpriseType {
    /// 購買價格
    pub fn purchase_price(&self) -> i32 {
        match self {
            EnterpriseType::NightMarketStall => 15_000,
            EnterpriseType::ConvenienceStore => 50_000,
            EnterpriseType::BubbleTeaShop => 35_000,
            EnterpriseType::MotorcycleShop => 80_000,
            EnterpriseType::Bar => 120_000,
            EnterpriseType::Nightclub => 250_000,
        }
    }

    /// 基礎日收入（無員工時）
    pub fn base_daily_income(&self) -> i32 {
        match self {
            EnterpriseType::NightMarketStall => 200,
            EnterpriseType::ConvenienceStore => 500,
            EnterpriseType::BubbleTeaShop => 400,
            EnterpriseType::MotorcycleShop => 800,
            EnterpriseType::Bar => 1200,
            EnterpriseType::Nightclub => 2500,
        }
    }

    /// 最大員工數
    pub fn max_employees(&self) -> u32 {
        match self {
            EnterpriseType::NightMarketStall => 2,
            EnterpriseType::ConvenienceStore => 5,
            EnterpriseType::BubbleTeaShop => 3,
            EnterpriseType::MotorcycleShop => 4,
            EnterpriseType::Bar => 6,
            EnterpriseType::Nightclub => 10,
        }
    }

    /// 每位員工日薪
    pub fn employee_salary(&self) -> i32 {
        match self {
            EnterpriseType::NightMarketStall => 80,
            EnterpriseType::ConvenienceStore => 100,
            EnterpriseType::BubbleTeaShop => 90,
            EnterpriseType::MotorcycleShop => 120,
            EnterpriseType::Bar => 150,
            EnterpriseType::Nightclub => 200,
        }
    }

    /// 每位員工帶來的收入加成比例
    pub fn employee_income_bonus(&self) -> f32 {
        match self {
            EnterpriseType::NightMarketStall => 0.30, // +30%/人
            EnterpriseType::ConvenienceStore => 0.20,
            EnterpriseType::BubbleTeaShop | EnterpriseType::MotorcycleShop => 0.25,
            EnterpriseType::Bar => 0.15,
            EnterpriseType::Nightclub => 0.12,
        }
    }

    /// 企業中文名
    pub fn label(&self) -> &'static str {
        match self {
            EnterpriseType::NightMarketStall => "夜市攤位",
            EnterpriseType::ConvenienceStore => "便利商店",
            EnterpriseType::BubbleTeaShop => "泡沫紅茶店",
            EnterpriseType::MotorcycleShop => "機車行",
            EnterpriseType::Bar => "酒吧",
            EnterpriseType::Nightclub => "夜店",
        }
    }
}

// ============================================================================
// 企業實體
// ============================================================================

/// 企業升級等級
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpgradeLevel {
    #[default]
    None,
    /// 裝潢升級（+20% 收入）
    Renovation,
    /// 設備升級（+40% 收入）
    Equipment,
    /// 頂級升級（+70% 收入）
    Premium,
}

impl UpgradeLevel {
    /// 升級帶來的收入加成比例
    pub fn income_multiplier(&self) -> f32 {
        match self {
            UpgradeLevel::None => 1.0,
            UpgradeLevel::Renovation => 1.2,
            UpgradeLevel::Equipment => 1.4,
            UpgradeLevel::Premium => 1.7,
        }
    }

    /// 升級到下一級的費用（None 則無法再升級）
    pub fn upgrade_cost(&self, enterprise_type: &EnterpriseType) -> Option<i32> {
        let base = enterprise_type.purchase_price();
        match self {
            UpgradeLevel::None => Some(base / 4),       // 25% 購買價
            UpgradeLevel::Renovation => Some(base / 3), // 33%
            UpgradeLevel::Equipment => Some(base / 2),  // 50%
            UpgradeLevel::Premium => None,              // 已滿級
        }
    }

    /// 升級到下一級
    pub fn next(&self) -> Option<UpgradeLevel> {
        match self {
            UpgradeLevel::None => Some(UpgradeLevel::Renovation),
            UpgradeLevel::Renovation => Some(UpgradeLevel::Equipment),
            UpgradeLevel::Equipment => Some(UpgradeLevel::Premium),
            UpgradeLevel::Premium => None,
        }
    }
}

/// 單一企業狀態（附加到 Entity 上作為 Component）
#[derive(Component, Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct Enterprise {
    /// 企業類型
    pub enterprise_type: EnterpriseType,
    /// 是否已被玩家購買
    pub owned: bool,
    /// 員工人數
    pub employees: u32,
    /// 升級等級
    pub upgrade_level: UpgradeLevel,
    /// 上次收入結算的遊戲日
    pub last_income_day: u32,
    /// 累計收入（統計用）
    pub total_income: i32,
    /// 累計支出（薪水 + 升級）
    pub total_expenses: i32,
}

impl Enterprise {
    /// 建立一間待售企業
    pub fn for_sale(enterprise_type: EnterpriseType) -> Self {
        Self {
            enterprise_type,
            owned: false,
            employees: 0,
            upgrade_level: UpgradeLevel::None,
            last_income_day: 0,
            total_income: 0,
            total_expenses: 0,
        }
    }

    /// 購買企業
    pub fn purchase(&mut self) {
        self.owned = true;
    }

    /// 計算日收入（扣除薪水前）
    pub fn daily_gross_income(&self) -> i32 {
        if !self.owned {
            return 0;
        }

        let base = self.enterprise_type.base_daily_income() as f32;
        let employee_bonus =
            1.0 + self.employees as f32 * self.enterprise_type.employee_income_bonus();
        let upgrade_mult = self.upgrade_level.income_multiplier();

        (base * employee_bonus * upgrade_mult) as i32
    }

    /// 計算日薪水支出
    pub fn daily_salary(&self) -> i32 {
        self.employees as i32 * self.enterprise_type.employee_salary()
    }

    /// 計算日淨收入
    pub fn daily_net_income(&self) -> i32 {
        self.daily_gross_income() - self.daily_salary()
    }

    /// 僱用一位員工（回傳是否成功）
    pub fn hire_employee(&mut self) -> bool {
        if !self.owned {
            return false;
        }
        if self.employees >= self.enterprise_type.max_employees() {
            return false;
        }
        self.employees += 1;
        true
    }

    /// 解僱一位員工
    pub fn fire_employee(&mut self) -> bool {
        if self.employees == 0 {
            return false;
        }
        self.employees -= 1;
        true
    }

    /// 嘗試升級（回傳升級費用，或 None 表示無法升級）
    pub fn try_upgrade(&mut self) -> Option<i32> {
        if !self.owned {
            return None;
        }
        let cost = self.upgrade_level.upgrade_cost(&self.enterprise_type)?;
        let next = self.upgrade_level.next()?;
        self.upgrade_level = next;
        self.total_expenses += cost;
        Some(cost)
    }

    /// 結算今日收入（回傳收入金額，若已結算則回傳 0）
    pub fn collect_income(&mut self, current_day: u32) -> i32 {
        if !self.owned || current_day <= self.last_income_day {
            return 0;
        }

        let net = self.daily_net_income();
        self.last_income_day = current_day;
        self.total_income += net.max(0);
        self.total_expenses += self.daily_salary();
        net
    }

    /// 投資回本所需天數（目前日淨收入）
    pub fn days_to_roi(&self) -> Option<u32> {
        let net = self.daily_net_income();
        if net <= 0 {
            return None;
        }
        let total_investment = self.enterprise_type.purchase_price() + self.total_expenses;
        Some((total_investment as f32 / net as f32).ceil() as u32)
    }
}

// ============================================================================
// 企業管理資源
// ============================================================================

/// 企業管理全局狀態
#[derive(Resource)]
pub struct EnterpriseManager {
    /// 玩家擁有的企業數量
    pub owned_count: u32,
    /// 今日總收入（UI 顯示用）
    pub today_total_income: i32,
    /// 遊戲日計數器（用於收入結算）
    pub current_day: u32,
    /// 本日是否已結算（防止同一個 8AM 窗口內重複結算）
    collected_this_cycle: bool,
}

impl Default for EnterpriseManager {
    fn default() -> Self {
        Self {
            owned_count: 0,
            today_total_income: 0,
            current_day: 1, // 從 1 開始，因為 Enterprise.last_income_day 初始為 0
            collected_this_cycle: false,
        }
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 企業日收入結算系統
pub fn enterprise_income_system(
    world_time: Res<WorldTime>,
    mut wallet: ResMut<PlayerWallet>,
    mut manager: ResMut<EnterpriseManager>,
    mut enterprises: Query<&mut Enterprise>,
) {
    // 早上 8 點結算（與 rental_income_system 類似但不衝突）
    let hour = world_time.hour;
    if !(7.5..8.5).contains(&hour) {
        // 離開結算窗口後，推進日計數器並重置標記
        if manager.collected_this_cycle {
            manager.current_day += 1;
            manager.collected_this_cycle = false;
        }
        return;
    }

    // 本窗口已結算過，跳過
    if manager.collected_this_cycle {
        return;
    }

    manager.collected_this_cycle = true;
    let current_day = manager.current_day;

    let mut daily_total = 0;
    let mut owned = 0u32;

    for mut enterprise in &mut enterprises {
        if enterprise.owned {
            owned += 1;
            let income = enterprise.collect_income(current_day);
            if income != 0 {
                // add_cash 正確處理正/負值和統計追蹤
                wallet.add_cash(income);
                daily_total += income;
            }
        }
    }

    manager.owned_count = owned;
    manager.today_total_income = daily_total;
}

// ============================================================================
// 測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enterprise_for_sale() {
        let e = Enterprise::for_sale(EnterpriseType::NightMarketStall);
        assert!(!e.owned);
        assert_eq!(e.employees, 0);
        assert_eq!(e.daily_gross_income(), 0); // 未購買
    }

    #[test]
    fn enterprise_purchase_enables_income() {
        let mut e = Enterprise::for_sale(EnterpriseType::NightMarketStall);
        e.purchase();
        assert!(e.owned);
        assert_eq!(e.daily_gross_income(), 200); // 基礎收入
    }

    #[test]
    fn enterprise_hire_employees() {
        let mut e = Enterprise::for_sale(EnterpriseType::NightMarketStall);
        e.purchase();

        assert!(e.hire_employee()); // 1/2
        assert!(e.hire_employee()); // 2/2
        assert!(!e.hire_employee()); // 已滿

        // 2 人 × 30% 加成 = 1.6x
        // 200 * 1.6 = 320
        assert_eq!(e.daily_gross_income(), 320);
        // 薪水：2 × 80 = 160
        assert_eq!(e.daily_salary(), 160);
        // 淨收入：320 - 160 = 160
        assert_eq!(e.daily_net_income(), 160);
    }

    #[test]
    fn enterprise_fire_employee() {
        let mut e = Enterprise::for_sale(EnterpriseType::ConvenienceStore);
        e.purchase();
        e.hire_employee();
        e.hire_employee();

        assert!(e.fire_employee());
        assert_eq!(e.employees, 1);

        assert!(e.fire_employee());
        assert_eq!(e.employees, 0);

        assert!(!e.fire_employee()); // 已無員工
    }

    #[test]
    fn enterprise_upgrade_levels() {
        let mut e = Enterprise::for_sale(EnterpriseType::BubbleTeaShop);
        e.purchase();

        let base = e.daily_gross_income();

        // 升級 1：裝潢（1.2x）
        let cost1 = e.try_upgrade();
        assert!(cost1.is_some());
        assert_eq!(e.upgrade_level, UpgradeLevel::Renovation);
        assert_eq!(e.daily_gross_income(), (base as f32 * 1.2) as i32);

        // 升級 2：設備（1.4x）
        let cost2 = e.try_upgrade();
        assert!(cost2.is_some());
        assert_eq!(e.upgrade_level, UpgradeLevel::Equipment);

        // 升級 3：頂級（1.7x）
        let cost3 = e.try_upgrade();
        assert!(cost3.is_some());
        assert_eq!(e.upgrade_level, UpgradeLevel::Premium);

        // 無法再升級
        assert!(e.try_upgrade().is_none());
    }

    #[test]
    fn enterprise_collect_income() {
        let mut e = Enterprise::for_sale(EnterpriseType::Bar);
        e.purchase();

        let income = e.collect_income(1);
        assert_eq!(income, 1200); // 0 員工，無升級
        assert_eq!(e.last_income_day, 1);

        // 同一天再次結算 → 0
        let income2 = e.collect_income(1);
        assert_eq!(income2, 0);

        // 隔天結算
        let income3 = e.collect_income(2);
        assert_eq!(income3, 1200);
    }

    #[test]
    fn enterprise_not_owned_no_hire() {
        let mut e = Enterprise::for_sale(EnterpriseType::Nightclub);
        assert!(!e.hire_employee()); // 未購買不能僱人
    }

    #[test]
    fn enterprise_not_owned_no_upgrade() {
        let mut e = Enterprise::for_sale(EnterpriseType::Nightclub);
        assert!(e.try_upgrade().is_none());
    }

    #[test]
    fn enterprise_days_to_roi() {
        let mut e = Enterprise::for_sale(EnterpriseType::NightMarketStall);
        e.purchase();
        // 購買價 15000，日淨收入 200
        // 15000 / 200 = 75 天
        assert_eq!(e.days_to_roi(), Some(75));
    }

    #[test]
    fn enterprise_days_to_roi_with_employees() {
        let mut e = Enterprise::for_sale(EnterpriseType::ConvenienceStore);
        e.purchase();
        e.hire_employee();
        e.hire_employee();

        // 基礎 500，2 員工 × 20% = 1.4x → 700 毛收入
        // 薪水 2 × 100 = 200
        // 淨收入 500
        // 購買價 50000 / 500 = 100 天
        assert_eq!(e.daily_gross_income(), 700);
        assert_eq!(e.daily_net_income(), 500);
        assert_eq!(e.days_to_roi(), Some(100));
    }

    #[test]
    fn upgrade_costs_scale_with_price() {
        let stall_cost = UpgradeLevel::None.upgrade_cost(&EnterpriseType::NightMarketStall);
        let club_cost = UpgradeLevel::None.upgrade_cost(&EnterpriseType::Nightclub);

        // 夜市攤位 15000 × 25% = 3750
        assert_eq!(stall_cost, Some(3750));
        // 夜店 250000 × 25% = 62500
        assert_eq!(club_cost, Some(62500));
    }

    #[test]
    fn enterprise_type_labels() {
        assert_eq!(EnterpriseType::NightMarketStall.label(), "夜市攤位");
        assert_eq!(EnterpriseType::Nightclub.label(), "夜店");
    }
}
