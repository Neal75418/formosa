---
paths:
  - "src/**/dev_tools*"
  - "src/**/debug*"
  - "src/**/fps*"
  - "src/**/inspector*"
---

# 開發工具

條件編譯：`#[cfg(all(debug_assertions, feature = "dev_tools"))]`

## Debug 工具

| 工具              | 按鍵 | 位置        | 說明                     |
|-----------------|----|-----------|------------------------|
| World Inspector | -  | 全螢幕       | 即時編輯實體/組件（dev 模式常駐）    |
| FPS Counter     | -  | 左上角       | 綠(>60)/黃(30-60)/紅(<30) |
| AI Debug        | F3 | -         | AI 視野/聽覺範圍             |
| Debug Viz       | F4 | -         | 警察視野/路徑/恐慌範圍           |
| Rapier Debug    | -  | 場景中       | 綠色碰撞箱線框                |
| Entity Names    | -  | Inspector | 每秒自動命名（英文）             |

## Gizmos 可視化

- 警察 FOV 錐 - 綠色扇形（半徑：`PoliceConfig.vision_range`）
- 視線狀態 - 紅（看見）/灰（未看見）
- A* 路徑 - 藍色折線 + 黃色球（waypoints）
- 恐慌範圍 - 黃色圓圈（半徑 10m）

## 開發工具模式

```rust
// Timer-based system（非關鍵 debug 功能）
#[derive(Resource)]
pub struct MyDebugTimer { timer: Timer }

app.init_resource::<MyDebugTimer>()
   .add_systems(Update, (
       update_timer,
       debug_system.run_if(|t: Res<MyDebugTimer>| t.timer.just_finished()),
   ).chain());

// Toggle-based system（F3 類按鍵切換）
#[derive(Resource, Default)]
pub struct DebugState { pub enabled: bool }

app.init_resource::<DebugState>()
   .add_systems(Update, debug_viz.run_if(|s: Res<DebugState>| s.enabled));
```

## UI 位置慣例

- **左上角**：Debug 資訊（FPS、座標等）
- **右上角**：小地圖（避免放置其他 UI）
- **中下**：通緝等級、武器、血量

## 整合注意事項

- **FlyCam 衝突**：與自訂 camera_follow 系統衝突，已移除
- **Inspector 中文亂碼**：預設字體不支援中文，實體命名使用英文
- **Gizmos 性能**：預設每幀繪製，大量物件時用 `run_if` 條件執行或 F3 切換
- **條件編譯**：所有 dev tools 模組需加 `#[cfg(all(debug_assertions, feature = "dev_tools"))]`

## 遙控測試（BRP，`cargo brp`）

條件編譯：`#[cfg(all(debug_assertions, feature = "brp"))]`；HTTP 只綁 `127.0.0.1:15702`，遊戲在背景、沒有焦點也收得到輸入。

- ⚠️ 沒有驗證：本機任何程式、甚至瀏覽器裡的網頁（視瀏覽器而定）都可能對這個 port 送請求（`brp_extras/screenshot` 可在任意路徑建立或覆寫圖片檔，並建出中間目錄）→ 測完立刻 `brp_extras/shutdown`，不要長駐背景
- 啟動時 port 已被占用（舊實例還在跑）會印錯誤並直接結束，避免請求打到舊 build；換 port 用環境變數 `BRP_EXTRAS_PORT`

- 等載入：log 出現 `📦 載入完成，轉場至 InGame` 才送指令
- 呼叫：`curl -s -X POST http://127.0.0.1:15702 -H 'Content-Type: application/json' -d '<JSON-RPC>'`
- 按鍵：`"method":"brp_extras/send_keys","params":{"keys":["KeyW"],"duration_ms":2000}`（鍵名 PascalCase，上限 60000 ms）
- 截圖：`brp_extras/screenshot`，`params: {"path": "<絕對路徑>.png"}`；非同步寫檔，回傳後要等檔案出現
- 玩家座標：`world.query` 取 `bevy_ecs::name::Name` + `bevy_transform::components::transform::Transform`，找 Name 為 `"Player"`（`Player` 沒有 Reflect，不能當 filter）
- 結束：`brp_extras/shutdown`
- 0.17 版只有 send_keys／screenshot／shutdown／set_window_title，沒有滑鼠方法（README 列的 click_mouse 等是新版才有）
- 存檔寫到 `$TMPDIR/IslandRampage-brp/saves`；一般模式每 300 秒自動存到 `dirs::data_dir()/IslandRampage/saves/autosave.json`
- 比對前後畫面：玩家每次都從 (5, 0.7, -5) 出生，但 `world/buildings/generic.rs` 的建築顏色每次啟動都隨機，要比外觀先固定隨機種子
