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
