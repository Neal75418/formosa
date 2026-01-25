# CLAUDE.md

此檔案為 Claude Code (claude.ai/code) 在此程式庫中工作時提供指引。

## AI 助理技能

> **重要**：此專案使用專門的 AI 技能。
> 撰寫程式碼前，請先參閱 `.agent/skills/` 中的指南：
>
> - **Rust 專家**：`.agent/skills/rust-expert/SKILL.md`
> - **Bevy 架構師**：`.agent/skills/bevy-architect/SKILL.md`
> - **遊戲數學與物理**：`.agent/skills/game-math-physicist/SKILL.md`
> - **資源管理員**：`.agent/skills/asset-manager/SKILL.md`

## 專案概述

**島嶼狂飆 (Island Rampage)** - 以台灣為舞台的 GTA 風格 3D 開放世界動作遊戲，使用 Rust 和 Bevy 0.17 開發。

| 技術               | 版本           | 用途       |
|------------------|--------------|----------|
| Rust             | 2021 Edition | 程式語言     |
| Bevy             | 0.17         | ECS 遊戲引擎 |
| bevy_rapier3d    | 0.32         | 3D 物理引擎  |
| serde/serde_json | 1.0          | 存檔系統     |

## 常用指令

```bash
cargo run                    # 開發模式（動態連結，編譯較快）
cargo run --release          # 發布模式（最佳效能）
cargo check                  # 檢查編譯錯誤
cargo test                   # 執行所有 178 個單元測試
cargo test economy::tests    # 執行特定模組測試
cargo clippy                 # 靜態分析
cargo fmt                    # 格式化程式碼
```

## 架構

### 模組結構

```
src/
├── core/           # 資源、事件、空間哈希網格
├── player/         # 移動、跳躍、閃避、上下車
├── vehicle/        # 物理、NPC AI、偷車、改裝
├── combat/         # 射擊、掩體、爆炸物、傷害
├── wanted/         # 警察 AI、直升機、路障
├── pedestrian/     # 恐慌波、目擊者系統
├── environment/    # 可破壞物件、碎片物件池
├── economy/        # 金錢、商店系統
├── mission/        # 任務、對話、過場動畫
├── world/          # 地圖、天氣、隨機事件
├── ui/             # HUD、小地圖、武器輪盤
├── camera/         # 第三人稱跟隨、震動
├── audio/          # 背景音樂、引擎聲、3D 空間音效
└── save/           # 非同步 IO、JSON 序列化
```

### 關鍵模式

#### 1. Bevy 0.17 Message 模式

事件使用 `add_message::<T>()` 而非 `add_event::<T>()`：

```rust
// 在 Plugin::build() 中
app.add_message::<DamageEvent>();

// 在系統中
fn my_system(mut events: MessageReader<DamageEvent>) {
    for event in events.read() { ... }
}
```

#### 2. 空間哈希網格（O(1) 鄰近查詢）

位於 `core/spatial_hash.rs`。三個預定義網格：

| 資源                      | 網格大小  | 用途     |
|-------------------------|-------|--------|
| `VehicleSpatialHash`    | 15.0m | 行人碰撞檢測 |
| `PedestrianSpatialHash` | 10.0m | 恐慌波傳播  |
| `PoliceSpatialHash`     | 20.0m | 玩家偵測   |

使用模式：
```rust
fn my_system(mut grid: ResMut<VehicleSpatialHash>, query: Query<...>) {
    // 1. 每幀清空
    grid.clear();

    // 2. 插入所有實體
    for (entity, transform) in query.iter() {
        grid.insert(entity, transform.translation);
    }

    // 3. 查詢附近實體 - O(k) 而非 O(n²)
    let nearby = grid.query_radius(center, 10.0);
}
```

#### 3. 物件池模式（碎片）

位於 `environment/components.rs`（`DebrisPool`）。兩階段獲取確保安全重用：

```rust
// 從池中獲取
if let Some(entity) = pool.acquire() {
    if let Ok((mut debris, ...)) = query.get_mut(entity) {
        // 重用成功 - 確認獲取
        pool.confirm_acquire(entity);
    } else {
        // 實體無效 - 不放回池中
        warn!("池中實體 {:?} 無效", entity);
    }
}
```

#### 4. 距離平方優化

**永遠使用 `distance_squared`** 搭配預計算常數：

```rust
// 正確 ✓
const ALERT_DISTANCE_SQ: f32 = 1600.0;  // 40m²
if pos1.distance_squared(pos2) < ALERT_DISTANCE_SQ { ... }

// 避免 ✗
if pos1.distance(pos2) < 40.0 { ... }
```

常用常數：
| 系統 | 常數 | 值 | 距離 |
|------|------|-----|------|
| AI | `ALERT_DISTANCE_SQ` | 1600.0 | 40m |
| 行人 | `VEHICLE_COLLISION_SQ` | 6.25 | 2.5m |
| 任務 | `DELIVERY_INTERACT_DIST_SQ` | 64.0 | 8m |

#### 5. Query 衝突解決

當多個 Query 存取相同組件時，使用 `Without<T>`：

```rust
// 三個 Query 都需要 Transform - 使用 Without 使其不重疊
pub fn spotlight_tracking_system(
    player_query: Query<&Transform, With<Player>>,
    helicopter_query: Query<(&Transform, &PoliceHelicopter), Without<Player>>,
    mut spotlight_query: Query<(&mut Transform, &HelicopterSpotlight), (Without<PoliceHelicopter>, Without<Player>)>,
)
```

### 系統執行順序

系統在 `main.rs` 中以明確順序註冊：

```
1. 核心/UI（暫停時仍執行）
   └─ handle_game_events, toggle_pause, update_ui

2. 玩家（明確鏈式）
   └─ player_input → dodge_detection → player_movement → player_jump

3. 載具（暫停時跳過）
   └─ vehicle_input → vehicle_movement → npc_vehicle_ai

4. 攝影機（在移動之後）
   └─ camera_input → camera_follow（在玩家/載具移動之後）

5. 戰鬥（暫停時跳過）
   └─ shooting → damage → death → ragdoll → effects

6. 天氣/世界
   └─ weather_input（不暫停）→ particles（暫停）
```

使用 `.run_if(|ui: Res<UiState>| !ui.paused)` 實現暫停感知系統。

### 插件模式

每個主要系統都是一個 Plugin（參見 `combat/mod.rs`）：

```rust
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_message::<DamageEvent>()
            .init_resource::<CombatState>()
            .add_systems(Startup, setup_combat_visuals)
            .add_systems(Update, (
                system_a,
                system_b,
            ).chain().run_if(...));
    }
}
```

## 開發狀態

### 已完成（Phase 1-6）

- **戰鬥**：射擊、近戰（棍棒/刀具）、爆炸物（手榴彈/汽油彈/C4）、掩體系統、車上射擊
- **載具**：多種類型、偷車動畫、改裝（引擎/變速箱/懸吊/煞車/輪胎/裝甲）、氮氣加速、損壞系統
- **通緝**：5 星等級、警察 AI、警用直升機（探照燈）、路障、投降/逮捕
- **開放世界**：西門町場景、可破壞環境、行人 AI（恐慌波）、交通系統、隨機事件
- **經濟**：金錢系統、任務評分、商店系統
- **存檔**：非同步 IO、JSON 序列化
- **優化**：空間哈希網格、碎片物件池
- **重構**：AI/Combat 模組拆分、高複雜度函數優化、配置提取

### 未來規劃（Phase 7+）

詳見計畫檔案 `~/.claude/plans/dapper-splashing-kazoo.md`：
- 手機系統（任務接取、聯絡人、GPS）
- 游泳/潛水
- 車內廣播電台
- 攀爬/跑酷
- 多角色切換

## 關鍵檔案

| 系統    | 主要檔案                              |
|-------|-----------------------------------|
| 空間哈希  | `src/core/spatial_hash.rs`        |
| 戰鬥插件  | `src/combat/mod.rs`               |
| 爆炸物   | `src/combat/explosives.rs`        |
| 掩體系統  | `src/combat/cover.rs`             |
| 警用直升機 | `src/wanted/police_helicopter.rs` |
| 偷車    | `src/vehicle/theft.rs`            |
| 車輛改裝  | `src/vehicle/modifications.rs`    |
| 可破壞物件 | `src/environment/systems.rs`      |
| 碎片物件池 | `src/environment/components.rs`   |

## 操作方式

| 按鍵    | 步行                | 駕駛    |
|-------|-------------------|-------|
| WASD  | 移動                | 轉向/加速 |
| Space | 跳躍                | 煞車    |
| Shift | 衝刺                | 氮氣    |
| Q/E   | 斜向前進（左前/右前）       | -     |
| F     | 情境互動（上下車/任務/商店/門） | 下車    |
| X     | 偷車                | -     |
| R     | 換彈                | -     |
| G     | 投擲爆炸物             | -     |
| 1-4   | 切換武器              | -     |
| Tab   | 武器輪盤              | -     |
| M     | 地圖                | 地圖    |
| Esc   | 暫停                | 暫停    |

## 驗證指令

```bash
cargo check && cargo test && cargo clippy
```
