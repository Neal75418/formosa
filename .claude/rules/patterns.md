---
paths:
  - "src/**/*.rs"
---

# 關鍵 Bevy / Rust 模式

## 1. Bevy 0.17 Message 模式

```rust
// Plugin::build()
app.add_message::<DamageEvent>();

// 系統中
fn my_system(mut events: MessageReader<DamageEvent>) {
    for event in events.read() { ... }
}
```

## 2. 空間哈希網格（O(1) 鄰近查詢）

位於 `core/spatial_hash.rs`，三個預定義網格：

| 資源                      | 網格大小  | 用途     |
|-------------------------|-------|--------|
| `VehicleSpatialHash`    | 15.0m | 行人碰撞檢測 |
| `PedestrianSpatialHash` | 10.0m | 恐慌波傳播  |
| `PoliceSpatialHash`     | 20.0m | 玩家偵測   |

```rust
fn my_system(mut grid: ResMut<VehicleSpatialHash>) {
    grid.clear();                                    // 每幀清空
    grid.insert(entity, transform.translation);      // 插入
    let nearby = grid.query_radius(center, 10.0);    // 查詢 O(k)
}
```

## 3. 物件池（碎片）

位於 `environment/components.rs`，兩階段獲取確保安全重用：

```rust
if let Some(entity) = pool.acquire() {
    if let Ok((mut debris, ...)) = query.get_mut(entity) {
        pool.confirm_acquire(entity);  // 確認
    }
}
```

## 4. 距離平方優化

永遠使用 `distance_squared` 搭配預計算常數：

```rust
const ALERT_DISTANCE_SQ: f32 = 1600.0;  // 40m
if pos1.distance_squared(pos2) < ALERT_DISTANCE_SQ { ... }
```

## 5. Query 衝突解決

多個 Query 存取相同組件時，使用 `Without<T>` 消除歧義：

```rust
pub fn system(
    player: Query<&Transform, With<Player>>,
    heli: Query<&Transform, (With<PoliceHelicopter>, Without<Player>)>,
    spotlight: Query<&mut Transform, (With<Spotlight>, Without<Player>, Without<PoliceHelicopter>)>,
)
```

## 6. SystemParam 模式（超過 16 個參數）

```rust
#[derive(SystemParam)]
pub struct DamageSystemResources<'w> {
    combat_state: ResMut<'w, CombatState>,
    // ...多個 resource 欄位
}

pub fn damage_system(res: DamageSystemResources, query: Query<...>) { ... }
```
