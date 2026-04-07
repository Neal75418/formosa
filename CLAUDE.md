# CLAUDE.md

Claude Code 在此專案中的工作指引。專題知識按需載入自 `.claude/rules/`。

## AI 助理技能

撰寫程式碼前，參閱 `.agent/skills/` 中的專門指南：

| 技能       | 路徑                                           |
|----------|----------------------------------------------|
| Rust 專家  | `.agent/skills/rust-expert/SKILL.md`         |
| Bevy 架構師 | `.agent/skills/bevy-architect/SKILL.md`      |
| 遊戲數學與物理  | `.agent/skills/game-math-physicist/SKILL.md` |
| 資源管理員    | `.agent/skills/asset-manager/SKILL.md`       |

## 專案概述

**島嶼狂飆 (Island Rampage)** — 以台灣西門町為舞台的 GTA 風格 3D 開放世界動作遊戲。

| 技術               | 版本           | 用途       |
|------------------|--------------|----------|
| Rust             | 2021 Edition | 程式語言     |
| Bevy             | 0.17         | ECS 遊戲引擎 |
| bevy_rapier3d    | 0.32         | 3D 物理引擎  |
| serde/serde_json | 1.0          | 存檔系統     |

**規模**：252 個 .rs 檔案、86,729 行代碼、817 個單元測試、0 clippy warnings

## 常用指令

```bash
cargo dev                    # 開發模式（含 dev_tools，見 .cargo/config.toml）
cargo run                    # 開發模式（不含 dev_tools）
cargo run --release          # 發布模式（最佳效能，不含 dev_tools）
cargo check                  # 編譯檢查
cargo test                   # 執行 817 個單元測試
cargo test economy::tests    # 特定模組測試
cargo clippy                 # 靜態分析
cargo fmt                    # 格式化
```

## 代碼質量規範（摘要）

> 完整規範含範例見 `.claude/rules/code-quality.md`

- **檔案限制**：單檔不得超過 800 行，超過 500 行考慮分割成子模組
- **分層依賴**：高層可依賴低層（Presentation → Entities → Core），禁止向上依賴
- **跨模組通訊**：優先使用 Events 解耦，避免直接修改其他模組的 Resource
- **dead_code**：禁止無註解的 `#![allow(dead_code)]`，必須說明預留用途
- **Clone**：小型 Copy 類型可 Clone，大型 Vec/String 用引用或 `Arc<T>`
- **unwrap()**：僅限 `#[cfg(test)]` 和有 `// SAFETY:` 註解的初始化保證。遊戲運行時用 `let Ok(...) = ... else { return; }`
- **距離計算**：永遠用 `distance_squared` + 預計算常數（`const RANGE_SQ: f32 = 1600.0; // 40m`）
- **Query 衝突**：多 Query 存取相同組件時用 `Without<T>` 消除歧義
- **測試**：新功能必須有單元測試。核心系統 > 80%、UI > 60%、整體 > 75%
- **Commit**：`<type>(<scope>): <subject>` + `Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>`

## 關鍵檔案速查

| 系統      | 檔案                                                                 |
|---------|--------------------------------------------------------------------|
| 空間哈希    | `src/core/spatial_hash.rs`                                         |
| 戰鬥插件    | `src/combat/mod.rs`                                                |
| 傷害計算    | `src/combat/damage/` (calculation, death, effects, reactions)      |
| 射擊系統    | `src/combat/shooting/` (input, firing, effects)                    |
| 爆炸物     | `src/combat/explosives/` (systems, explosion, effects)             |
| 掩體      | `src/combat/cover.rs`                                              |
| 警用直升機   | `src/wanted/police_helicopter/` (components, spawning, ai, combat) |
| 偷車      | `src/vehicle/theft.rs`                                             |
| 車輛改裝    | `src/vehicle/modifications/` (performance, visuals, systems)       |
| 車輛效果    | `src/vehicle/effects.rs`                                           |
| 行人生命週期  | `src/pedestrian/systems/lifecycle.rs`                              |
| 恐慌系統    | `src/pedestrian/panic.rs`                                          |
| 目擊者系統   | `src/pedestrian/systems/witnesses.rs`                              |
| 世界生成    | `src/world/setup/`                                                 |
| 天氣效果    | `src/world/time_weather/weather_effects.rs`                        |
| 可破壞物件   | `src/environment/systems.rs`                                       |
| 股票市場    | `src/economy/stock_market.rs`                                      |
| 賭場      | `src/economy/casino.rs`                                            |
| 手機 UI   | `src/ui/phone.rs`, `phone_apps.rs`, `phone_apps_stock.rs`          |
| 改裝商店 UI | `src/ui/mod_shop.rs`                                               |
| 車內電台    | `src/audio/integration.rs`, `src/audio/components.rs`              |
| 玩家游泳    | `src/player/swimming.rs`                                           |
| 載具變形    | `src/vehicle/vehicle_damage/visuals.rs`                            |

## Bevy 0.17 注意事項

- `on_timer()` 不存在 — 使用自訂 `Timer` resource + `run_if(|t: Res<T>| t.timer.just_finished())`
- FPS 顯示 — `DiagnosticsStore` + `FrameTimeDiagnosticsPlugin::FPS`
- Query 單一結果 — `query.single()` 取代 `get_single()`（Bevy 0.16 → 0.17）

## 驗證

```bash
cargo check && cargo test && cargo clippy
```

817 個單元測試，修改後必跑（~0.01s）。建置時間：37-84 秒（動態連結）。

## 按需載入的規則（`.claude/rules/`）

| 規則檔 | 內容 | 載入條件（`paths:` frontmatter） |
|---|---|---|
| `architecture.md` | 3 個 Mermaid 架構圖（分層/執行順序/Plugin 列表） | `src/main.rs`、`src/*/mod.rs`、`src/lib.rs` |
| `patterns.md` | 6 個關鍵模式（Message、空間哈希、物件池、距離平方、Query 衝突、SystemParam） | `src/**/*.rs` |
| `code-quality.md` | 代碼質量完整規範含範例（檔案大小/dead_code/模組依賴/Clone/錯誤處理/提交格式） | `src/**/*.rs` |
| `dev-tools.md` | Debug 工具表、Gizmos 可視化、開發工具 code pattern | `src/**/dev_tools*`、`src/**/debug*`、`src/**/fps*`、`src/**/inspector*` |
| `controls.md` | 完整按鍵對應表（步行/駕駛/開發） | `src/player/**`、`src/vehicle/**`、`src/ui/**`、`src/camera/**` |
| `test-coverage.md` | 15 模組測試分佈表 + 覆蓋率目標 | `src/**/*test*`、`src/**/tests.rs` |
