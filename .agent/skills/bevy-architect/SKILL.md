---
name: bevy-architect
description: Expert guidance on Bevy Engine (0.17) ECS patterns and architecture
---

# Bevy Architect Guide (v0.17)

本專案使用 Bevy 0.17，請遵守 ECS (Entity Component System) 架構原則。

## 1. Plugin Structure

- **Modularity**:
  - 不要把所有 System 塞在 `main.rs`。
  - 按功能拆分 Plugin (e.g., `PlayerPlugin`, `CameraPlugin`, `PhysicsPlugin`)。
  - 每個 Plugin 負責註冊自己的 Systems, Components 和 Events。

```rust
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (player_movement, player_animation))
           .add_event::<PlayerJumpEvent>();
    }
}
```

## 2. System Queries

- **Query Performance**:
  - 盡量精確 Query (`Query<&Transform, With<Player>>`)。
  - 使用 `Changed<T>` filter 來優化，只處理有變動的 Component。

```rust
fn movement(mut query: Query<&mut Transform, (With<Player>, Changed<Input>)>) { ... }
```

## 3. Resources vs Components

- **Global Config**:
  - 全局唯一的狀態 (如 `GameScore`, `TimeOfDay`) 使用 `Resource`。
  - 實體特有的數據 (如 `Health`, `Velocity`) 使用 `Component`。

## 4. States

- **Game Flow**:
  - 使用 `States` (e.g., `AppState::Menu`, `AppState::InGame`) 來控制 System 的執行時機。
  - `app.add_systems(Update, player_move.run_if(in_state(AppState::InGame)))`
