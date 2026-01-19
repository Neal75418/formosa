---
name: game-math-physicist
description: Expert guidance on 3D Math, Vectors, Quaternions, and Rapier Physics
---

# Game Math & Physics Guide

## 1. 3D Math (glam)

Bevy 使用 `glam` 作為數學庫。

- **Quaternions (四元數)**:
  - **嚴禁** 直接操作 Quaternion 的 `x, y, z, w` 分量。
  - 旋轉計算務必已經定義好的方法：`Quat::from_rotation_y()`, `Quat::mul_vec3()`。
  - 避免 Gimbal Lock，不要使用 Euler Angles (歐拉角) 儲存旋轉。

- **Vectors**:
  - 向量正規化 (`Vec3::normalize()`) 前，務必檢查長度是否接近 0 (`Vec3::length_squared() > EPSILON`)，避免 NaN 擴散。

## 2. Physics (Rapier3D)

- **RigidBody**:
  - `RigidBody::Dynamic`: 受力影響 (如主角、車輛)。
  - `RigidBody::Fixed`: 地板、建築物。
  - `RigidBody::KinematicPositionBased`: 平台移動 (無限質量)。

- **Colliders**:
  - 優先使用簡單形狀 (`Collider::cuboid`, `Collider::ball`) 效能最好。
  - 複雜地形使用 `Collider::trimesh` (但很貴，儘量用 Simplified Mesh)。

- **Coordinates**:
  - Rapier 的物理模擬與 Bevy 的渲染 Transform 會自動同步。
  - 不要手動在 `Update` system 裡改 `Transform` 強制移動 Dynamic Body，應該透過 `ExternalImpulse` 或 `Velocity` component 施力。
