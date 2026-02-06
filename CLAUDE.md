# CLAUDE.md

Claude Code 在此專案中的工作指引。

## AI 助理技能

撰寫程式碼前，參閱 `.agent/skills/` 中的專門指南：

| 技能 | 路徑 |
|------|------|
| Rust 專家 | `.agent/skills/rust-expert/SKILL.md` |
| Bevy 架構師 | `.agent/skills/bevy-architect/SKILL.md` |
| 遊戲數學與物理 | `.agent/skills/game-math-physicist/SKILL.md` |
| 資源管理員 | `.agent/skills/asset-manager/SKILL.md` |

## 專案概述

**島嶼狂飆 (Island Rampage)** — 以台灣西門町為舞台的 GTA 風格 3D 開放世界動作遊戲。

| 技術 | 版本 | 用途 |
|------|------|------|
| Rust | 2021 Edition | 程式語言 |
| Bevy | 0.17 | ECS 遊戲引擎 |
| bevy_rapier3d | 0.32 | 3D 物理引擎 |
| serde/serde_json | 1.0 | 存檔系統 |

**規模**：121 個 .rs 檔案、~61,700 行、235 個單元測試、0 clippy warnings

## 常用指令

```bash
cargo run                    # 開發模式（動態連結）
cargo run --release          # 發布模式（最佳效能）
cargo check                  # 編譯檢查
cargo test                   # 執行 235 個單元測試
cargo test economy::tests    # 特定模組測試
cargo clippy                 # 靜態分析
cargo fmt                    # 格式化
```

## 架構

### 模組結構（15 個頂層模組）

```
src/
├── core/               # 空間哈希、數學工具、物件池、狀態機
│   ├── camera.rs         # CameraSettings, RecoilState, CameraShake
│   ├── pool.rs           # EntityPool 泛用物件池
│   ├── spatial_hash.rs   # O(1) 鄰近查詢
│   └── weather.rs        # WeatherType, WeatherState
├── player/             # 移動、跳躍、閃避、攀爬、上下車
├── vehicle/            # 物理、NPC AI、偷車、改裝、視覺效果
│   ├── effects.rs        # 漂移煙霧、火焰、輪胎痕跡
│   ├── spawning.rs       # 車輛生成
│   ├── traffic_lights.rs # 交通號誌
│   └── vehicle_damage.rs # 損壞系統
├── combat/             # 射擊、掩體、爆炸物、傷害、布娃娃
├── wanted/             # 5 星通緝、警察 AI、直升機、路障
│   └── police_vehicle/   # 警車 AI + 生成
├── pedestrian/         # 行人 AI、恐慌波、目擊者
│   └── systems/          # 7 個子模組（lifecycle, animation, reactions, pathfinding_grid, daily_behavior, witnesses, panic_propagation）
├── environment/        # 可破壞物件、碎片物件池
├── economy/            # 金錢、商店系統
├── mission/            # 劇情任務、對話、過場動畫（15 個檔案）
├── world/              # 地圖生成、天氣、隨機事件
│   ├── buildings/        # 商業、娛樂、服務、通用建築
│   └── time_weather/     # 3 個子模組（lighting, city_visuals, weather_effects）
├── ui/                 # HUD、小地圖、武器輪盤、GPS（18 個檔案）
├── camera/             # 第三人稱跟隨、震動
├── audio/              # 背景音樂、引擎聲、3D 空間音效
├── save/               # 非同步 IO、JSON 序列化
└── ai/                 # 敵人 AI：感知、決策、戰鬥、掩護、小隊
```

### 關鍵模式

#### 1. Bevy 0.17 Message 模式

```rust
// Plugin::build()
app.add_message::<DamageEvent>();

// 系統中
fn my_system(mut events: MessageReader<DamageEvent>) {
    for event in events.read() { ... }
}
```

#### 2. 空間哈希網格（O(1) 鄰近查詢）

位於 `core/spatial_hash.rs`，三個預定義網格：

| 資源 | 網格大小 | 用途 |
|------|----------|------|
| `VehicleSpatialHash` | 15.0m | 行人碰撞檢測 |
| `PedestrianSpatialHash` | 10.0m | 恐慌波傳播 |
| `PoliceSpatialHash` | 20.0m | 玩家偵測 |

```rust
fn my_system(mut grid: ResMut<VehicleSpatialHash>) {
    grid.clear();                                    // 每幀清空
    grid.insert(entity, transform.translation);      // 插入
    let nearby = grid.query_radius(center, 10.0);    // 查詢 O(k)
}
```

#### 3. 物件池（碎片）

位於 `environment/components.rs`，兩階段獲取確保安全重用：

```rust
if let Some(entity) = pool.acquire() {
    if let Ok((mut debris, ...)) = query.get_mut(entity) {
        pool.confirm_acquire(entity);  // 確認
    }
}
```

#### 4. 距離平方優化

永遠使用 `distance_squared` 搭配預計算常數：

```rust
const ALERT_DISTANCE_SQ: f32 = 1600.0;  // 40m
if pos1.distance_squared(pos2) < ALERT_DISTANCE_SQ { ... }
```

#### 5. Query 衝突解決

多個 Query 存取相同組件時，使用 `Without<T>` 消除歧義：

```rust
pub fn system(
    player: Query<&Transform, With<Player>>,
    heli: Query<&Transform, (With<PoliceHelicopter>, Without<Player>)>,
    spotlight: Query<&mut Transform, (With<Spotlight>, Without<Player>, Without<PoliceHelicopter>)>,
)
```

#### 6. SystemParam 模式（超過 16 個參數）

```rust
#[derive(SystemParam)]
pub struct DamageSystemResources<'w> {
    combat_state: ResMut<'w, CombatState>,
    // ...多個 resource 欄位
}

pub fn damage_system(res: DamageSystemResources, query: Query<...>) { ... }
```

### 系統執行順序

```
1. 核心/UI（暫停時仍執行）
   └─ toggle_pause, update_ui

2. 玩家（明確排序）
   └─ player_input → dodge → movement → jump

3. 載具（暫停時跳過）
   └─ vehicle_input → movement → npc_ai

4. 攝影機（在移動之後）
   └─ camera_input → camera_follow

5. 戰鬥（暫停時跳過，顯式 .after() 依賴）
   └─ shooting → damage → death → ragdoll → effects

6. 天氣/世界
   └─ weather_input（不暫停）→ particles（暫停）
```

暫停控制：`.run_if(|ui: Res<UiState>| !ui.paused)`

### 測試覆蓋

| 模組 | 測試數 | 覆蓋範圍 |
|------|--------|----------|
| combat | 88 | 武器、傷害、護甲、布娃娃、出血 |
| economy | 47 | 錢包、商店、ATM |
| ai | 26 | 狀態轉換、感知、逃跑 |
| wanted | 22 | 通緝等級、警察狀態、搜索區 |
| pedestrian/panic | 18 | 恐慌波傳播、尖叫冷卻 |
| save | 17 | 序列化、存檔路徑 |
| core/spatial_hash | 12 | 插入、查詢、邊界 |
| player/climb | 5 | 攀爬類型、緩動函數 |
| **合計** | **235** | |

## 關鍵檔案速查

| 系統 | 檔案 |
|------|------|
| 空間哈希 | `src/core/spatial_hash.rs` |
| 戰鬥插件 | `src/combat/mod.rs` |
| 傷害計算 | `src/combat/damage.rs` |
| 爆炸物 | `src/combat/explosives.rs` |
| 掩體 | `src/combat/cover.rs` |
| 警用直升機 | `src/wanted/police_helicopter.rs` |
| 偷車 | `src/vehicle/theft.rs` |
| 車輛改裝 | `src/vehicle/modifications.rs` |
| 車輛效果 | `src/vehicle/effects.rs` |
| 行人生命週期 | `src/pedestrian/systems/lifecycle.rs` |
| 恐慌系統 | `src/pedestrian/panic.rs` |
| 目擊者系統 | `src/pedestrian/systems/witnesses.rs` |
| 世界生成 | `src/world/setup.rs` |
| 天氣效果 | `src/world/time_weather/weather_effects.rs` |
| 可破壞物件 | `src/environment/systems.rs` |

## 操作方式

| 按鍵 | 步行 | 駕駛 |
|------|------|------|
| WASD | 移動 | 轉向/加速 |
| Space | 跳躍 | 煞車 |
| Shift | 衝刺 | 氮氣 |
| Q/E | 斜向前進 | - |
| F | 互動（上下車/任務/商店/門） | 下車 |
| X | 偷車 | - |
| R | 換彈 | - |
| G | 投擲爆炸物 | - |
| 1-4 | 切換武器 | - |
| Tab | 武器輪盤 | - |
| M | 地圖 | 地圖 |
| Esc | 暫停 | 暫停 |

## 驗證

```bash
cargo check && cargo test && cargo clippy
```
