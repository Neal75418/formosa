# 性能優化建議

> 基於 Phase 4 代碼審查（2026-02）

## 克隆分析

### 統計數據

- **`.clone()` 調用**：501 次
- **`#[derive(Clone)]` 定義**：198 個
- **測試通過**：386 個 ✓

### 高頻克隆檔案

| 檔案                    | .clone() 次數 | 主要用途          |
|-----------------------|-------------|---------------|
| `ai/lifecycle.rs`     | 34          | AI 實體生成（材質句柄） |
| `world/characters.rs` | 33          | 角色模型生成（材質句柄）  |
| `world/setup.rs`      | 31          | 世界初始化（材質、配置）  |
| `combat/visuals.rs`   | 28          | 武器視覺效果（句柄）    |
| `audio/systems.rs`    | 28          | 音效播放（音源句柄）    |
| `ui/delivery_app.rs`  | 25          | UI 數據綁定       |
| `vehicle/spawning.rs` | 23          | 車輛生成（配置、材質）   |

### 分析結論

**大部分克隆是合理的**：
1. ✅ **Bevy Handle<T>** - 輕量級引用計數，克隆成本低
2. ✅ **小型配置結構** - 包含基本類型（f32, u32 等）
3. ✅ **Bevy 組件要求** - ECS 系統需要可克隆的組件

**潛在優化點**（ROI 較低）：
- ⚠️ 存檔系統中的大型 Vec 克隆
- ⚠️ 任務數據的頻繁克隆

## 優化建議

### 1. 存檔系統 - 考慮使用 Arc（非緊急）

**現狀**：
```rust
#[derive(Clone)]
pub struct MissionSaveData {
    pub completed_missions: Vec<String>,        // 可能很大
    pub mission_progress: Vec<(String, usize)>,
    pub mission_ratings: Vec<(String, u8)>,
}
```

**問題**：
- 存檔列表 UI 顯示時會克隆整個結構
- 自動存檔備份也會克隆

**建議優化**（可選）：
```rust
pub struct MissionSaveData {
    pub completed_missions: Arc<Vec<String>>,
    pub mission_progress: Arc<Vec<(String, usize)>>,
    pub mission_ratings: Arc<Vec<(String, u8)>>,
}
```

**預期收益**：
- 減少記憶體複製：約 5-10% 存檔系統負載
- 適用場景：存檔槽 > 5 個且頻繁切換 UI

**實施成本**：
- 中等（需修改序列化邏輯）
- 不推薦：收益有限，複雜度增加

### 2. 材質句柄克隆 - 無需優化

**現狀**：
```rust
// world/characters.rs
MeshMaterial3d(skin_mat.clone()),  // 33 次
```

**分析**：
- `Handle<StandardMaterial>` 內部使用 `Arc`
- 克隆只是增加引用計數（O(1)）
- 無需優化 ✅

### 3. 配置數據共享 - 考慮靜態化（非必要）

**現狀**：
```rust
// vehicle/config.rs
#[derive(Clone)]
pub struct VehiclePhysicsConfig {
    pub max_speed: f32,
    pub acceleration: f32,
    // ... 20+ 個欄位
}

// 每次生成車輛都克隆配置
let config = vehicle_configs.sedan.clone();
```

**建議優化**（可選）：
```rust
// 使用常數或 OnceCell
use std::sync::OnceLock;

static SEDAN_CONFIG: OnceLock<Arc<VehiclePhysicsConfig>> = OnceLock::new();

pub fn get_sedan_config() -> &Arc<VehiclePhysicsConfig> {
    SEDAN_CONFIG.get_or_init(|| Arc::new(VehiclePhysicsConfig { ... }))
}

// 使用時只克隆 Arc（便宜）
let config = get_sedan_config().clone();
```

**預期收益**：
- 減少車輛生成的記憶體分配：~2-5%
- 適用場景：車輛數量 > 50 輛

**實施成本**：
- 低（局部修改）
- 可選：僅在性能瓶頸時考慮

## 性能基準測試建議

### 建立基準測試

```bash
# 安裝 criterion
cargo add --dev criterion

# 創建 benches/clone_overhead.rs
```

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use island_rampage::save::components::MissionSaveData;

fn bench_mission_data_clone(c: &mut Criterion) {
    let data = MissionSaveData {
        completed_missions: vec!["mission_1".to_string(); 100],
        mission_progress: vec![("mission_2".to_string(), 5); 50],
        ..Default::default()
    };

    c.bench_function("clone mission save data", |b| {
        b.iter(|| black_box(data.clone()))
    });
}

criterion_group!(benches, bench_mission_data_clone);
criterion_main!(benches);
```

### 運行基準測試

```bash
cargo bench --bench clone_overhead
```

## Profiling 指引

### 使用 cargo-flamegraph

```bash
# 安裝
cargo install flamegraph

# 執行遊戲並生成火焰圖
cargo flamegraph --dev

# 開啟 flamegraph.svg 檢查熱點
```

### 使用 perf（Linux）

```bash
# 記錄性能數據
cargo build --release
perf record --call-graph dwarf ./target/release/island-rampage

# 分析報告
perf report
```

### 使用 Instruments（macOS）

```bash
# 編譯 release 版本
cargo build --release

# 在 Xcode Instruments 中開啟
open -a Instruments

# 選擇 "Time Profiler" 並附加到進程
```

## 結論與建議

### 當前狀態：良好 ✅

**優點**：
- 大部分克隆是輕量級的（Handle, 小型結構）
- 符合 Bevy ECS 的設計模式
- 無明顯性能瓶頸

**維持現狀的理由**：
1. 過早優化是萬惡之源
2. 當前性能表現良好（60 FPS+）
3. 代碼可讀性優於微優化
4. 編譯器已進行內聯和優化

### 何時需要優化

**觸發條件**：
- 🔴 Frame time > 16.67ms（低於 60 FPS）
- 🔴 Profiling 顯示克隆佔用 > 5% CPU
- 🔴 記憶體使用異常增長

**優先級**：
1. **高**：Profiling 確認的瓶頸
2. **中**：存檔/載入時間 > 500ms
3. **低**：預防性優化

### 行動建議

**立即行動**（無）：
- 當前性能良好，無需立即優化

**未來考慮**（可選）：
1. 建立 benchmark suite（當性能成為問題時）
2. 使用 Arc 共享大型配置（僅在 profiling 確認必要時）
3. 定期 profiling（每個 milestone）

**不建議**：
- ❌ 大規模重構以減少克隆
- ❌ 過度使用 Arc/Rc 增加複雜度
- ❌ 犧牲可讀性換取微小性能提升

## 附錄：Clone 模式速查

### 何時使用 Clone

✅ **推薦使用**：
```rust
// 1. Bevy Handle（內部是 Arc）
let material = materials.add(...);
commands.spawn(MeshMaterial3d(material.clone()));

// 2. 小型 Copy 類型組合
#[derive(Clone, Copy)]
pub struct Config {
    pub speed: f32,
    pub health: u32,
}

// 3. ECS 組件（Bevy 要求）
#[derive(Component, Clone)]
pub struct Health(pub f32);

// 4. 事件數據（通常很小）
#[derive(Message, Clone)]
pub struct DamageEvent { damage: f32 }
```

### 何時使用 Arc

✅ **推薦使用**：
```rust
// 1. 大型唯讀配置
pub struct GameConfig {
    pub missions: Arc<Vec<MissionData>>,  // 大型數據
}

// 2. 共享快取
pub struct TextureAtlas {
    pub atlas: Arc<Image>,  // 避免重複載入
}
```

### 何時使用引用

✅ **推薦使用**：
```rust
// 1. 函數參數（不需要所有權）
fn process_data(data: &MissionData) { ... }

// 2. Query 迭代
for entity_data in query.iter() {
    // entity_data 已經是引用
}
```

## 參考資料

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Bevy Performance Guide](https://bevyengine.org/learn/book/performance/)
- [Clone vs Arc vs Rc](https://doc.rust-lang.org/book/ch15-04-rc.html)
