# Changelog

All notable changes to this project will be documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/) · Commits: [Conventional Commits](https://www.conventionalcommits.org/)

---

## [Unreleased]

### Added

- **車內廣播電台系統**：8 個台灣主題頻道、Q/E 快捷切換、音量淡入淡出、手機開啟時自動靜音
- **股票市場手機 App**：行情/持倉/交易三分頁、6 支台灣主題股票即時行情、買賣交易 UI
- **車輛改裝商店手機 App**：6 項改裝類別 UI（引擎/變速箱/懸吊/煞車/輪胎/裝甲）、等級/價格/效果顯示、購買互動系統
- **PhoneContentCleanupQueries SystemParam**：重構手機 UI 清理查詢，減少系統參數複雜度（14→10）
- **測試覆蓋擴展**：從 386 增加到 804 個單元測試

### Changed

- ModShop icon 統一為 ASCII 風格（W），與其他 Phone App 一致
- handle_mod_shop_buttons 簡化為純事件發送，信任系統層驗證
- Unified markdown style with Mermaid architecture diagrams
- Updated README.md, CLAUDE.md, LICENSE, .gitignore
- Added CHANGELOG.md and PR template

### Fixed

- ModShop UI 競態條件（改為事件驅動 ModificationCompleteEvent 通知）
- Runtime unwrap() 使用（改用 next.price() 安全取值）
- ModShop 購買後 UI 未自動刷新（新增 wallet.is_changed() 檢測）
- ModShop UI 重複驗證邏輯（簡化 handle_mod_shop_buttons，-22 行）
- Clippy `field_reassign_with_default` 錯誤（story_manager.rs）
- Clippy `assertions_on_constants` 錯誤（audio/components.rs）
- 手機開啟時數字鍵同時觸發電台切換的輸入衝突

---

## [0.1.0] — 2026-01-19

### Added

- **Phase 1** — Core systems: player control, economy, save/load
- **Phase 2** — Combat: shooting, cover, explosives, damage, ragdoll
- **Phase 3** — Wanted system: 5-star levels, police AI, helicopter, roadblocks
- **Phase 4** — Open world: random events, destructible environment, car theft
- **Phase 5** — Advanced features: helicopter, melee, vehicle mods, performance
- **Phase 6** — Code quality: module splitting, complexity optimization
- **Phase 7** — Architecture refactor: God Module splitting, component decomposition
- **Phase 8** — Test coverage: 329 unit tests across all core modules

### Fixed

- CI sccache install permission issue (switched to `~/.cargo/bin`)

### Technical

- Rust 2021 Edition + Bevy 0.17 + bevy_rapier3d 0.32
- 140 `.rs` files, ~62,800 lines of code
- Spatial hash grid for O(1) proximity queries
- Async save/load with JSON serialization
- GitHub Actions CI with sccache
