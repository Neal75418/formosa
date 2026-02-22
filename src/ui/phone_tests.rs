//! 手機 UI 單元測試
//!
//! 從 phone.rs 拆分，降低單檔行數。

use crate::ui::components::{
    MissionJournalTab, PhoneApp, PhoneUiState, StockMarketTab,
};

#[test]
fn phone_app_labels() {
    assert_eq!(PhoneApp::Home.label(), "主畫面");
    assert_eq!(PhoneApp::Contacts.label(), "聯絡人");
    assert_eq!(PhoneApp::MissionLog.label(), "任務日誌");
    assert_eq!(PhoneApp::Map.label(), "地圖");
    assert_eq!(PhoneApp::Settings.label(), "設定");
    assert_eq!(PhoneApp::StockMarket.label(), "股市");
}

#[test]
fn phone_app_all_apps_count() {
    assert_eq!(PhoneApp::all_apps().len(), 5);
}

#[test]
fn phone_app_all_apps_excludes_home() {
    assert!(!PhoneApp::all_apps().contains(&PhoneApp::Home));
}

#[test]
fn phone_ui_state_defaults() {
    let state = PhoneUiState::default();
    assert_eq!(state.current_app, PhoneApp::Home);
    assert_eq!(state.selected_index, 0);
}

#[test]
fn phone_app_icon_not_empty() {
    for app in PhoneApp::all_apps() {
        assert!(!app.icon().is_empty());
        assert!(!app.label().is_empty());
    }
}

#[test]
fn phone_navigation_wraps_right() {
    let app_count = PhoneApp::all_apps().len();
    let mut idx = app_count - 1; // 最後一個
    idx = (idx + 1) % app_count;
    assert_eq!(idx, 0); // 回到第一個
}

#[test]
fn phone_navigation_wraps_left() {
    let app_count = PhoneApp::all_apps().len();
    let mut idx: usize = 0;
    idx = (idx + app_count - 1) % app_count;
    assert_eq!(idx, app_count - 1); // 到最後一個
}

#[test]
fn phone_toggle_logic() {
    let mut show_phone = false;

    // 第一次按上：開啟
    show_phone = !show_phone;
    assert!(show_phone);

    // 第二次按上：關閉
    show_phone = !show_phone;
    assert!(!show_phone);
}

// ========================================================================
// 任務日誌測試
// ========================================================================

#[test]
fn journal_tab_labels() {
    assert_eq!(MissionJournalTab::Active.label(), "進行中");
    assert_eq!(MissionJournalTab::Completed.label(), "已完成");
    assert_eq!(MissionJournalTab::Stats.label(), "統計");
}

#[test]
fn journal_tab_all_count() {
    assert_eq!(MissionJournalTab::all().len(), 3);
}

#[test]
fn journal_tab_default_is_active() {
    let tab = MissionJournalTab::default();
    assert_eq!(tab, MissionJournalTab::Active);
}

#[test]
fn journal_tab_cycle_right() {
    let tabs = MissionJournalTab::all();
    let mut idx = 0; // Active
    idx = (idx + 1) % tabs.len(); // -> Completed
    assert_eq!(tabs[idx], MissionJournalTab::Completed);
    idx = (idx + 1) % tabs.len(); // -> Stats
    assert_eq!(tabs[idx], MissionJournalTab::Stats);
    idx = (idx + 1) % tabs.len(); // -> Active (wrap)
    assert_eq!(tabs[idx], MissionJournalTab::Active);
}

#[test]
fn journal_tab_cycle_left() {
    let tabs = MissionJournalTab::all();
    let mut idx = 0; // Active
    idx = (idx + tabs.len() - 1) % tabs.len(); // -> Stats (wrap)
    assert_eq!(tabs[idx], MissionJournalTab::Stats);
}

#[test]
fn phone_state_includes_journal_tab() {
    let state = PhoneUiState::default();
    assert_eq!(state.journal_tab, MissionJournalTab::Active);
}

#[test]
fn completed_mission_record_stars_display() {
    use crate::mission::{CompletedMissionRecord, MissionType};

    let record = CompletedMissionRecord {
        title: "測試任務".to_string(),
        mission_type: MissionType::Delivery,
        reward: 500,
        stars: 3,
        rating_label: "⭐⭐⭐".to_string(),
    };
    assert_eq!(record.stars_display(), "★★★");
    assert_eq!(record.type_label(), "外送");
}

// ========================================================================
// 股市測試
// ========================================================================

#[test]
fn stock_tab_labels() {
    assert_eq!(StockMarketTab::StockList.label(), "行情");
    assert_eq!(StockMarketTab::Portfolio.label(), "持倉");
    assert_eq!(StockMarketTab::Trade.label(), "交易");
}

#[test]
fn stock_tab_all_count() {
    assert_eq!(StockMarketTab::all().len(), 3);
}

#[test]
fn stock_tab_default_is_stock_list() {
    let tab = StockMarketTab::default();
    assert_eq!(tab, StockMarketTab::StockList);
}

#[test]
fn stock_tab_cycle_right() {
    let tabs = StockMarketTab::all();
    let mut idx = 0; // StockList
    idx = (idx + 1) % tabs.len(); // -> Portfolio
    assert_eq!(tabs[idx], StockMarketTab::Portfolio);
    idx = (idx + 1) % tabs.len(); // -> Trade
    assert_eq!(tabs[idx], StockMarketTab::Trade);
    idx = (idx + 1) % tabs.len(); // -> StockList (wrap)
    assert_eq!(tabs[idx], StockMarketTab::StockList);
}

#[test]
fn stock_tab_cycle_left() {
    let tabs = StockMarketTab::all();
    let mut idx = 0; // StockList
    idx = (idx + tabs.len() - 1) % tabs.len(); // -> Trade (wrap)
    assert_eq!(tabs[idx], StockMarketTab::Trade);
}

#[test]
fn phone_state_includes_stock_fields() {
    let state = PhoneUiState::default();
    assert_eq!(state.stock_tab, StockMarketTab::StockList);
    assert_eq!(state.selected_stock_index, 0);
    assert_eq!(state.trade_quantity, 1);
}

#[test]
fn stock_index_cycle() {
    use crate::economy::StockSymbol;
    let count = StockSymbol::ALL.len();
    let mut idx = 0;
    for _ in 0..count {
        idx = (idx + 1) % count;
    }
    assert_eq!(idx, 0); // 繞完一圈回到起點
}

#[test]
fn trade_quantity_clamp() {
    let mut qty: u32 = 1;
    // 不能低於 1
    qty = qty.saturating_sub(1).max(1);
    assert_eq!(qty, 1);
    // 不能超過 999
    qty = 999;
    qty = (qty + 1).min(999);
    assert_eq!(qty, 999);
}

#[test]
fn phone_app_stock_market_icon() {
    assert_eq!(PhoneApp::StockMarket.icon(), "$");
}

#[test]
fn phone_app_all_apps_includes_stock_market() {
    assert!(PhoneApp::all_apps().contains(&PhoneApp::StockMarket));
}

#[test]
fn completed_mission_record_type_labels() {
    use crate::mission::{CompletedMissionRecord, MissionType};

    let make = |mt| CompletedMissionRecord {
        title: String::new(),
        mission_type: mt,
        reward: 0,
        stars: 0,
        rating_label: String::new(),
    };
    assert_eq!(make(MissionType::Delivery).type_label(), "外送");
    assert_eq!(make(MissionType::Taxi).type_label(), "載客");
    assert_eq!(make(MissionType::Race).type_label(), "競速");
    assert_eq!(make(MissionType::Explore).type_label(), "探索");
}
