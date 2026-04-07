---
paths:
  - "src/**/*.rs"
---

# 代碼質量規範（詳細版）

> 基於 2026-02 code review 建立的規範。主檔 CLAUDE.md 有規則摘要，本檔提供完整範例。

## 檔案大小限制

**硬性限制**：單一檔案不得超過 **800 行**（含註解、空行）。超過 500 行應考慮分割。

**分割原則**：
1. **按職責分割**：每個檔案應專注於單一概念或功能
2. **使用子模組**：創建 `module_name/` 目錄並用 `mod.rs` 重新匯出
3. **保持測試同步**：分割時確保單元測試一併移動

**分割範例**（已完成的重構）：
```
# Before (❌ God Object)
src/vehicle/vehicle_damage.rs  (1,171 行)

# After (✅ 單一職責)
src/vehicle/vehicle_damage/
  ├── mod.rs              (重新匯出)
  ├── health.rs           (血量、輪胎狀態)
  ├── fire.rs             (引擎起火系統)
  ├── explosion.rs        (爆炸條件與效果)
  └── collision.rs        (碰撞傷害計算)
```

## `#![allow(dead_code)]` 使用規則

- ✅ 有註解說明未來用途：`#![allow(dead_code)]  // 天氣效果系統預留，將於 v0.5 整合`
- ❌ 無註解的全模組 dead_code suppress
- ❌ 用來隱藏真正應刪除的死代碼

## 模組依賴規則

**分層依賴**（參考架構圖）：
1. **向下依賴**：高層可依賴低層（Presentation → Entities → Core）
2. **禁止向上依賴**：Core 不得依賴 Entities 或 Systems
3. **橫向依賴最小化**：同層模組間盡量避免直接依賴

**跨模組通訊**：
- ✅ **優先使用 Events**：解耦模組間通訊
- ✅ **Resource 作為共享狀態**：明確的資料來源
- ❌ **避免直接修改其他模組的 Resource**：應透過 Events 通知

**範例**：
```rust
// ✅ 好的設計：使用 Event
#[derive(Message)]
pub struct MissionCompletedEvent {
    pub reward: u32,
}

// economy 模組監聽 Event
fn handle_mission_reward(
    mut events: MessageReader<MissionCompletedEvent>,
    mut wallet: ResMut<PlayerWallet>,
) {
    for event in events.read() {
        wallet.add_cash(event.reward as i32);
    }
}

// ❌ 壞的設計：mission 直接操作 economy 的 Resource
fn mission_system(mut wallet: ResMut<PlayerWallet>) {
    wallet.add_cash(100);
}
```

## 性能：Clone 使用指引

```rust
// ❌ 壞：每次都克隆
for item in items.iter() { process(item.clone()); }

// ✅ 好：傳引用
for item in items.iter() { process(item); }

// ✅ 大型數據使用 Arc 共享
pub struct MissionData {
    pub dialogue: Arc<Vec<DialogueLine>>,
    pub cutscenes: Arc<Vec<Cutscene>>,
}
```

**何時用 Clone**：✅ 小型 Copy 類型（u32/f32/Vec3）、需要修改副本、Bevy 組件。❌ 大型 Vec/String（改用引用或 Arc）。

## 錯誤處理

```rust
// ❌ 壞：可能 panic
let player = player_query.single().unwrap();

// ✅ 好：優雅處理
let Ok(player) = player_query.single() else {
    warn!("Player not found, skipping update");
    return;
};
```

`unwrap()` 僅限：✅ `#[cfg(test)]`、✅ 初始化保證存在（加 `// SAFETY:` 註解）。❌ 遊戲運行時邏輯。

## 測試要求

**新功能必須包含測試**：
- **核心邏輯**：所有計算、狀態轉換必須有單元測試
- **公開 API**：所有 `pub fn` 應有測試覆蓋
- **邊界條件**：測試極值、空值、錯誤情況

## 測試組織

```rust
// 方案 A：小型模組（<200 行）— 測試寫在同檔案
#[cfg(test)]
mod tests { use super::*; ... }

// 方案 B：大型模組 — 獨立 tests.rs
src/ui/
  ├── mod.rs
  ├── notification.rs
  └── tests.rs  // #[cfg(test)] mod tests;
```

## 提交規範

```
<type>(<scope>): <subject>

<body>

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `perf`, `style`, `chore`
