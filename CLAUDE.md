# Island Rampage - Claude Code 開發指南

## 🤖 AI Assistant Skills

> **IMPORTANT**: This project utilizes specialized AI skills.
> Before writing any code, you MUST consult the guidelines in `.agent/skills/`.
>
> - **Rust Expert**: Read `.agent/skills/rust-expert/SKILL.md`
> - **Bevy Architect**: Read `.agent/skills/bevy-architect/SKILL.md`
> - **Game Math & Physics**: Read `.agent/skills/game-math-physicist/SKILL.md`
> - **Asset Manager**: Read `.agent/skills/asset-manager/SKILL.md`

## 專案概述

**島嶼狂飆 (Island Rampage)** 是一款以台灣為舞台的 3D 開放世界動作冒險遊戲，使用 Rust 和 Bevy 引擎開發。遊戲讓玩家體驗台灣街頭生活，從西門町開始探索，逐步擴展至環島公路和各大城市。

- **版本**: 0.1.0
- **作者**: Neal Chen
- **授權**: MIT License

---

## 產品願景

### 核心賣點

> **「第一款以台灣為舞台的開放世界遊戲」**

市面上幾乎沒有以台灣城市為背景的 3D 開放世界遊戲。島嶼狂飆填補這個空白，讓玩家體驗最道地的台灣街頭生活。

### 遊戲定位

| 元素     | 說明                                 |
| -------- | ------------------------------------ |
| 類型     | 開放世界生活模擬 + 動作冒險          |
| 風格     | 寫實台灣街景 + 輕鬆幽默敘事          |
| 目標玩家 | 台灣玩家、對台灣文化有興趣的海外玩家 |
| 核心體驗 | 騎機車穿梭街頭、體驗台灣日常生活     |

### 台灣特色元素

#### 交通文化

- **機車王國**：鑽車陣、待轉區、機車瀑布（橋下停車場）
- **計程車**：小黃載客、聽司機講故事
- **捷運/公車**：快速移動系統，體驗通勤生活

#### 街頭風景

- **夜市系統**：可互動攤販、小遊戲（夾娃娃、射氣球、套圈圈）
- **便利商店**：24 小時營業，買東西補血、繳費、領包裹
- **霓虹招牌**：西門町夜景的視覺震撼

#### 飲食文化

- **小吃攤**：雞排、珍奶、滷味、臭豆腐
- **餐廳**：鼎泰豐、火鍋店、牛肉麵
- **飲料店**：手搖飲、咖啡廳

### 任務系統設計

#### 主要任務類型

| 類型       | 說明                               | 獎勵              |
| ---------- | ---------------------------------- | ----------------- |
| 外送員     | Uber Eats / Foodpanda 風格限時送餐 | 金錢 + 評價       |
| 計程車司機 | 載客到目的地，聽乘客講故事         | 金錢 + 人脈       |
| 網紅打卡   | 到各景點拍照收集                   | 粉絲數 + 解鎖區域 |
| 街頭表演   | 節奏小遊戲賺錢                     | 金錢 + 名聲       |
| 都市傳說   | 探索西門町的神秘事件               | 劇情解鎖          |
| 跑腿雜務   | 幫 NPC 辦事                        | 關係值 + 小費     |

#### 主線故事：「小人物的台北夢」

```
第一章：落腳台北
├── 剛到台北的年輕人，身無分文
├── 從外送員開始打拼
└── 目標：賺到第一個月房租

第二章：站穩腳步
├── 認識各種 NPC（夜市老闆、便利商店店員、計程車司機）
├── 租到自己的小套房
└── 目標：存錢買一台二手機車

第三章：擴展人脈
├── 發現西門町背後的故事
├── 幫助 NPC 解決問題，建立關係網
└── 目標：成為西門町的熟面孔

第四章：追逐夢想
├── 選擇自己的道路
│   ├── 開一間自己的店？
│   ├── 成為網紅 YouTuber？
│   ├── 當計程車車行老闆？
│   └── 或是揭開都市傳說的真相？
└── 多結局系統
```

### 成長系統

#### 居住升級

```
網咖過夜 → 頂樓加蓋 → 雅房 → 套房 → 一房一廳 → 公寓
   $0      $3,000    $5,000  $8,000  $15,000   $30,000/月
```

#### 載具收集

```
徒步 → 腳踏車 → 二手機車 → 新機車 → 汽車 → 重機
         $500    $15,000   $50,000  $300,000 $800,000
```

#### 社交關係

- **友好度系統**：和 NPC 互動提升關係
- **解鎖支線任務**：關係夠好才能接特殊任務
- **獲得幫助**：危急時刻 NPC 會來幫忙

### 遊戲 UI 設計

#### HUD 佈局 (1080p)

```
┌────────────────────────────────────────────────────────────┐
│ [血量條]  $5,000                              [小地圖 300px]│
│ [體力條]  ⭐ Lv.3                              N            │
│                                               ┌───┐        │
│                                               │ ● │ 你     │
│                                               └───┘        │
│                                                            │
│                        [遊戲畫面]                           │
│                                                            │
│                                                            │
│ ┌──────────────────┐                                       │
│ │ 📋 送貨到紅樓    │                     [任務提示]        │
│ │ ⏱️ 45秒 | 📍 120m│                                       │
│ └──────────────────┘                                       │
│                                                            │
│ [WASD 移動] [Tab 上下車] [M 地圖] [ESC 選單]               │
└────────────────────────────────────────────────────────────┘
```

#### 選單系統 (Tab/ESC)

| 快捷鍵 | 功能                             |
| ------ | -------------------------------- |
| M      | 大地圖（標記任務、興趣點、導航） |
| Tab    | 任務列表（進行中/可接取）        |
| I      | 背包/物品                        |
| J      | 任務日誌                         |
| K      | 關係圖（NPC 友好度）             |
| ESC    | 暫停選單                         |

#### 互動提示

- 靠近 NPC/物件時顯示互動鍵 `[F] 對話` `[E] 購買`
- 任務目標在畫面上方顯示距離和方向箭頭

---

## 開發計劃

### Phase 1：核心體驗（目前進行中）

> **目標：讓「騎機車逛西門町」變得很爽**
>
> 完成標準：玩家可以流暢地騎機車在西門町閒晃，夜景漂亮，有沉浸感

#### 1.1 機車系統 ⭐ 最高優先

| 任務         | 說明                                 | 涉及檔案             |
| ------------ | ------------------------------------ | -------------------- |
| 實現機車物理 | 傾斜過彎、加速感、煞車滑行           | `vehicle/systems.rs` |
| 機車生成     | 在路邊/停車格生成可騎乘的機車        | `world/setup.rs`     |
| 騎乘動畫     | 玩家騎車時的姿勢（簡化版：隱藏玩家） | `player/systems.rs`  |
| 機車音效     | 引擎聲、喇叭聲                       | 新增 `audio/` 模組   |

#### 1.2 視覺效果

| 任務       | 說明                     | 涉及檔案                |
| ---------- | ------------------------ | ----------------------- |
| 霓虹燈招牌 | 建築物發光招牌、閃爍效果 | `world/setup.rs`        |
| 夜景氛圍   | 路燈光暈、車燈照明       | `world/time_weather.rs` |
| 人行道燈光 | 便利商店門口的光線       | `world/setup.rs`        |

#### 1.3 環境音效

| 任務     | 說明              | 涉及檔案           |
| -------- | ----------------- | ------------------ |
| 背景音樂 | 白天/夜晚不同 BGM | 新增 `audio/` 模組 |
| 環境音   | 人聲、車聲、廣播  | `audio/`           |
| 3D 音效  | 靠近聲源變大聲    | `audio/`           |

---

### Phase 2：生活系統

> **目標：讓玩家有事做**
>
> 完成標準：可以接外送單、進便利商店買東西、和 NPC 簡單對話

#### 2.1 外送系統 ⭐ 核心玩法

| 任務         | 說明                   | 涉及檔案             |
| ------------ | ---------------------- | -------------------- |
| 外送任務生成 | 隨機生成取餐點和送達點 | `mission/data.rs`    |
| 計時系統     | 倒數計時、超時懲罰     | `mission/systems.rs` |
| 評價系統     | 根據速度給星評         | `mission/data.rs`    |
| 外送 App UI  | 顯示可接單列表         | `ui/systems.rs`      |

#### 2.2 商店系統

| 任務         | 說明              | 涉及檔案            |
| ------------ | ----------------- | ------------------- |
| 便利商店互動 | 按 F 進入購買介面 | 新增 `shop/` 模組   |
| 商品系統     | 飲料、食物、道具  | `shop/items.rs`     |
| 購買 UI      | 商品列表、結帳    | `ui/shop_ui.rs`     |
| 補血/補體力  | 吃東西恢復狀態    | `player/systems.rs` |

#### 2.3 對話系統

| 任務      | 說明                 | 涉及檔案         |
| --------- | -------------------- | ---------------- |
| NPC 生成  | 路人、店員、固定 NPC | 新增 `npc/` 模組 |
| 對話框 UI | 顯示對話文字         | `ui/dialogue.rs` |
| 對話腳本  | 簡單的對話樹         | `npc/dialogues/` |

---

### Phase 3：成長動力

> **目標：讓玩家有目標**
>
> 完成標準：玩家想要賺錢升級住處、買更好的車

#### 3.1 居住系統

| 任務     | 說明                   |
| -------- | ---------------------- |
| 租屋地點 | 網咖、雅房、套房等地點 |
| 租金系統 | 每日扣款、欠租警告     |
| 存檔點   | 回家睡覺 = 存檔        |

#### 3.2 載具商店

| 任務     | 說明                   |
| -------- | ---------------------- |
| 車行 NPC | 購買/出售載具          |
| 載具清單 | 腳踏車、各種機車、汽車 |
| 載具屬性 | 速度、加速、油耗       |

#### 3.3 NPC 關係

| 任務       | 說明                  |
| ---------- | --------------------- |
| 友好度數值 | 每個 NPC 有好感度     |
| 關係獎勵   | 好感度高解鎖任務/折扣 |
| 關係 UI    | 查看 NPC 關係圖       |

---

### Phase 4-5：內容擴展（未來規劃）

- 夜市區域和小遊戲
- 信義區地圖擴展
- 主線故事章節
- 多結局系統
- 成就和圖鑑

---

## 當前開發任務

### ✅ Phase 1 核心體驗 - 已完成

- [x] Phase 1.1 機車系統 - 傾斜過彎、機車生成
- [x] Phase 1.2 視覺效果 - 霓虹燈招牌系統
- [x] Phase 1.3 環境音效 - 背景音樂、引擎聲、空間音效

### 🎯 下一步：Phase 2.1 外送系統

```
優先順序：
1. mission/data.rs     - 外送任務生成邏輯
2. mission/systems.rs  - 計時系統、評價系統
3. ui/systems.rs       - 外送 App UI
4. 測試完整外送流程
```

### 預估工作量

| Phase     | 核心功能數 | 預估複雜度 |
| --------- | ---------- | ---------- |
| Phase 1   | 3          | 中         |
| Phase 2   | 3          | 高         |
| Phase 3   | 3          | 中         |
| Phase 4-5 | 5+         | 高         |

---

## 技術棧

| 技術             | 版本         | 用途                |
| ---------------- | ------------ | ------------------- |
| Rust             | 2021 Edition | 主要語言            |
| Bevy             | 0.17         | 遊戲引擎 (ECS 架構) |
| bevy_rapier3d    | 0.32         | 3D 物理引擎         |
| rand             | 0.9          | 隨機數生成          |
| serde/serde_json | 1.0          | 存檔序列化          |
| dirs             | 6.0          | 跨平台路徑          |

## 常用命令

```bash
# 開發模式運行 (動態鏈接，快速編譯)
cargo run

# 發布模式運行 (最佳性能)
cargo run --release

# 構建發布版本
cargo build --release

# 檢查編譯錯誤
cargo check

# 格式化代碼
cargo fmt

# 靜態分析
cargo clippy
```

## 專案結構

```
island-rampage/
├── src/
│   ├── main.rs              # 主程式入口，App 配置，系統執行順序
│   ├── camera/              # 攝影機系統
│   │   └── systems.rs       # 第三人稱視角跟隨、輸入控制
│   ├── combat/              # 戰鬥系統
│   │   ├── components.rs    # WeaponType, WeaponStats, DamageEvent
│   │   ├── shooting.rs      # 射擊、後座力、換彈
│   │   └── damage.rs        # 傷害計算、爆頭判定、Ragdoll
│   ├── ai/                  # AI 系統
│   │   ├── components.rs    # AiState, AiPerception
│   │   ├── systems.rs       # 感知、決策、攻擊
│   │   └── squad.rs         # 小隊協調、角色分配
│   ├── pedestrian/          # 行人系統
│   │   ├── components.rs    # PedState, WitnessState
│   │   └── systems.rs       # 路徑、恐慌波、報警
│   ├── wanted/              # 通緝系統
│   │   ├── components.rs    # CrimeType, WantedLevel
│   │   └── systems.rs       # 犯罪追蹤、警察生成
│   ├── core/                # 核心系統
│   │   ├── resources.rs     # GameState, WorldTime, CameraSettings
│   │   └── events.rs        # 遊戲事件定義
│   ├── mission/             # 任務系統
│   │   ├── data.rs          # MissionManager, 任務定義
│   │   ├── systems.rs       # 任務邏輯、標記動畫
│   │   └── story_systems.rs # 劇情任務、對話、過場
│   ├── player/              # 玩家系統
│   │   ├── components.rs    # Player 組件
│   │   └── systems.rs       # 移動、跳躍、上下車、閃避
│   ├── ui/                  # UI 系統
│   │   ├── components.rs    # UI 組件定義
│   │   └── systems.rs       # HUD、小地圖、武器輪盤、GPS
│   ├── vehicle/             # 載具系統
│   │   ├── components.rs    # Vehicle, NpcVehicle, VehicleType
│   │   └── systems.rs       # 車輛物理、NPC AI、甩尾
│   ├── world/               # 世界系統
│   │   ├── components.rs    # Building, StreetLight, NeonSign
│   │   ├── setup.rs         # 世界初始化、地形、道路
│   │   ├── time_weather.rs  # 日夜循環、天氣、閃電
│   │   └── interior.rs      # 室內進出系統
│   └── audio/               # 音效系統
│       ├── components.rs    # AudioManager, EngineSound
│       └── systems.rs       # 背景音樂、引擎音效、環境音效
├── assets/
│   ├── fonts/               # 中文字體 (STHeiti, NotoSansTC)
│   ├── models/              # 3D 模型
│   ├── textures/roads/      # 道路貼圖 (asphalt, paving, brick)
│   └── audio/               # 音效/音樂
├── Cargo.toml
└── README.md
```

## 架構說明

### ECS 模式

專案遵循 Bevy ECS (Entity-Component-System) 架構：

- **Components**: 純數據結構，定義於各模塊的 `components.rs`
- **Resources**: 全局共享狀態，定義於 `core/resources.rs`
- **Systems**: 業務邏輯，定義於各模塊的 `systems.rs`

### 模塊依賴關係

```
core (resources, events)
  ↓
player ←→ vehicle (上下車交互)
  ↓         ↓
camera (跟隨玩家或載具)

world → mission (地點檢測)
  ↓
ui (顯示狀態)

combat (玩家攻擊)
```

## 核心系統概要

### 玩家系統 (player/)

```rust
// 玩家組件關鍵屬性
Player {
    speed: 8.0,           // 基本速度
    sprint_speed: 15.0,   // 衝刺速度
    jump_force: 12.0,     // 跳躍力
    health: 100.0,        // 血量
    money: 5000,          // 初始金錢
}
```

控制：WASD 移動、Q/E 視角旋轉、Shift 衝刺、Space 跳躍、E 上下車

### 載具系統 (vehicle/)

```rust
// 載具類型與參數
VehicleType::Scooter  // 機車: 最高速 25.0, 加速 15.0
VehicleType::Car      // 汽車: 最高速 35.0, 加速 12.0
VehicleType::Taxi     // 計程車: 最高速 30.0, 加速 11.0
```

NPC 車輛 AI 狀態：Cruising → Braking → Stopped → Reversing

### 世界系統 (world/)

- **地圖**: 300×300 單位，模擬西門町
- **道路**: 中華路、西寧南路、漢中街、武昌街、峨嵋街、成都路
- **日夜循環**: 0-24 小時，影響光照和路燈

**天氣類型**: `Clear`, `Cloudy`, `Rainy`, `Foggy`

**天氣影響物理和感知**:
| 天氣 | 視距乘數 | 牽引力乘數 |
|------|----------|------------|
| Clear | 1.0 | 1.0 |
| Cloudy | 0.95 | 1.0 |
| Rainy | 0.6-0.8 | 0.7 |
| Foggy | 0.3-0.5 | 0.9 |

### 任務系統 (mission/)

```rust
MissionType::Delivery  // 送貨任務
MissionType::Taxi      // 出租車任務
MissionType::Race      // 競賽任務
MissionType::Explore   // 探索任務
```

### UI 系統 (ui/)

- **HUD**: 左上角血量條、金錢、控制提示
- **小地圖**: 右上角 300×300px，可縮放
- **大地圖**: M 鍵切換
- **暫停菜單**: P 鍵

### AI 系統 (ai/)

**狀態機 (`AiState`)**:

```
Idle/Patrol → Alert (聽到聲音) → Chase (看到玩家) → Attack
                                                    ↓
                               TakingCover (低血量) → Flee (血量 < 20%)
```

**感知參數**:

- FOV: 60°, 視距: 30m, 聽力: 50m
- 天氣影響: 雨天視距 -20%, 霧天視距 -50%

**小隊系統** (`squad.rs`):

- `Rusher` - 正面進攻
- `Flanker` - 側翼包抄
- `Suppressor` - 遠距離壓制
- `Leader` - 小隊長

### 行人系統 (pedestrian/)

**行人狀態**: `Idle` → `Walking` → `Fleeing` / `CallingPolice`

**恐慌傳播** (GTA 5 風格):

```
玩家開槍 → panic_wave_propagation → pedestrian_scream → 群體逃跑
```

**目擊者報警**: 目擊犯罪 → 打電話 3 秒 → 增加通緝等級

### 戰鬥系統 (combat/)

**武器類型**: `Fist`, `Pistol`, `SMG`, `Shotgun`, `Rifle`

**關鍵常數**:

```rust
const HEADSHOT_HEIGHT_THRESHOLD: f32 = 1.5;
const IMPULSE_BULLET: f32 = 350.0;
const IMPULSE_EXPLOSION: f32 = 800.0;
```

**特效**: 槍口閃光、彈道追蹤、血液粒子、浮動傷害數字、護甲破碎

### 通緝系統 (wanted/)

- 1-5 星通緝等級
- 犯罪類型: `Murder`, `Assault`, `VehicleTheft`, `VehicleDamage`, `Speeding`
- 警察 NPC 生成與追捕
- 通緝等級消退機制

## 效能優化模式

### distance_squared 使用方式

**避免 sqrt 開銷**，整個 codebase 使用 `distance_squared` 搭配預計算常數：

```rust
// 正確做法 ✓
const ALERT_DISTANCE_SQ: f32 = 1600.0;  // 40m²
if pos1.distance_squared(pos2) < ALERT_DISTANCE_SQ { ... }

// 避免 ✗
if pos1.distance(pos2) < 40.0 { ... }
```

**常用距離常數**:

| 系統 | 常數                               | 值     | 原始距離 |
| ---- | ---------------------------------- | ------ | -------- |
| AI   | `ALERT_DISTANCE_SQ`                | 1600.0 | 40m      |
| AI   | `COVER_ARRIVAL_SQ`                 | 2.25   | 1.5m     |
| 行人 | `MIN_SPAWN_DISTANCE_SQ`            | 225.0  | 15m      |
| 行人 | `VEHICLE_COLLISION_SQ`             | 6.25   | 2.5m     |
| 任務 | `DELIVERY_INTERACT_DIST_SQ`        | 64.0   | 8m       |
| 載具 | `NPC_WAYPOINT_ARRIVAL_DISTANCE_SQ` | 64.0   | 8m       |

### 安全的 unwrap 替代

```rust
// 使用 let-else pattern
let Some(value) = optional else { continue; };

// 排序使用 total_cmp 而非 partial_cmp
items.sort_by(|a, b| a.distance.total_cmp(&b.distance));
```

## 系統執行順序

系統在 `main.rs` 中按特定順序註冊 (line 122-251):

```
1. 核心/UI 更新 (暫停時仍執行)
   └─ handle_game_events, toggle_pause, update_ui

2. 玩家系統 (明確順序)
   └─ player_input → player_movement → dodge_movement → player_jump

3. 載具系統 (暫停時跳過)
   └─ vehicle_input → vehicle_movement → npc_vehicle_ai

4. 攝影機 (在移動後執行)
   └─ camera_input → camera_auto_follow → camera_follow

5. 任務和世界
   └─ mission_system → update_world_time → update_lighting

6. 天氣效果 (暫停時跳過)
   └─ rain_drops → rain_puddles → lightning

7. 室內系統 (暫停時跳過)
   └─ interior_proximity → interior_enter → interior_hiding

8. 音效 (背景音樂不受暫停影響)
```

## 開發規範

### 代碼風格

1. 使用 `cargo fmt` 格式化代碼
2. 使用 `cargo clippy` 檢查潛在問題
3. 組件名使用 PascalCase，系統名使用 snake_case
4. 模塊按功能劃分，每個模塊有獨立的 `mod.rs`

### 添加新功能

#### 添加新載具類型

1. 在 `vehicle/components.rs` 的 `VehicleType` 枚舉添加變體
2. 在 `Vehicle::new()` 添加對應的物理參數
3. 在 `vehicle/systems.rs` 的生成函數中添加車輛

#### 添加新任務

1. 在 `mission/data.rs` 的 `MissionManager::new()` 添加任務定義
2. 任務需指定：類型、標題、描述、獎勵、時間限制、起點終點座標

#### 擴展地圖

1. 在 `world/setup.rs` 的 `setup_world()` 添加新地形和建築
2. 使用 `spawn_road()` 添加道路
3. 在 `world/components.rs` 添加新建築類型 (如需)

### 性能考量

1. 開發時使用 `cargo run` (dynamic_linking 加速編譯)
2. 依賴套件使用 opt-level = 3 優化
3. 發布版使用 LTO 和 codegen-units = 1

## 遊戲控制

| 按鍵  | 步行模式  | 駕駛模式  |
| ----- | --------- | --------- |
| W/S   | 前進/後退 | 加速/倒車 |
| A/D   | 左右平移  | 左右轉向  |
| Q/E   | 視角旋轉  | 視角旋轉  |
| Shift | 衝刺切換  | -         |
| Space | 跳躍      | 煞車      |
| E     | 上車      | 下車      |
| R     | 攻擊      | -         |
| M     | 大地圖    | 大地圖    |
| P     | 暫停      | 暫停      |

## 已完成功能

- [x] 專案建立和 Bevy 0.17 配置
- [x] 基本 3D 場景和西門町地形
- [x] 玩家移動、跳躍、衝刺
- [x] 載具系統（汽車、計程車、公車）
- [x] 第三人稱攝影機跟隨
- [x] UI/HUD 系統（血量、金錢、小地圖）
- [x] 日夜循環和動態光照
- [x] 基本送貨任務系統
- [x] NPC 車輛 AI（巡航、避障、倒車）
- [x] 建築物生成系統
- [x] **機車系統** - 傾斜過彎物理、8 台可騎乘機車
- [x] **霓虹燈招牌系統** - 9 個發光招牌、閃爍動畫、日夜亮度變化
- [x] **音效系統** - 日/夜背景音樂切換、引擎音效、3D 空間環境音效

## 已知問題

1. NPC 車輛可能在複雜路口卡住
2. 跳躍物理需要更精確的接地檢測
3. 小地圖在極端縮放時顯示異常

## 資源文件說明

### 字體

- `STHeiti.ttc`: 繁體中文黑體 (主要 UI)
- `NotoSansTC-Medium.otf`: 思源黑體 (備用)

### 貼圖

- `asphalt.jpg`: 柏油路
- `paving.jpg`: 行人徒步區
- `brick.jpg`: 磚塊路面

## 關鍵實作檔案

| 檔案                        | 說明                             |
| --------------------------- | -------------------------------- |
| `src/main.rs`               | 插件註冊、系統執行順序 (250+ 行) |
| `src/ai/systems.rs`         | AI 感知與決策邏輯                |
| `src/pedestrian/systems.rs` | 恐慌波傳播、目擊者報警           |
| `src/combat/shooting.rs`    | 射擊系統、後座力計算             |
| `src/combat/damage.rs`      | 傷害計算、爆頭判定               |
| `src/vehicle/systems.rs`    | 載具物理、NPC 車輛 AI            |
| `src/world/time_weather.rs` | 天氣系統、動態影響               |

## 參考文檔

- [Bevy Book](https://bevyengine.org/learn/book/introduction/)
- [Bevy Cheat Book](https://bevy-cheatbook.github.io/)
- [bevy_rapier Documentation](https://rapier.rs/docs/)
