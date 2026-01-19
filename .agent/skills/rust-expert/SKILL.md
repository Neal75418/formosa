---
name: rust-expert
description: Expert guidance on Rust idioms, ownership, and performance
---

# Rust Expert Guide (Game Dev Edition)

本專案 (`island-rampage`) 使用 Rust 開發，效能與安全性是核心。

## 1. Zero-Cost Abstractions

- **Iterators**:
  - 優先使用 Iterator Chain (`.map()`, `.filter()`, `.collect()`) 而非 `for` loop，通常編譯器能優化得更好。

- **Newtype Pattern**:
  - 為遊戲中的數值建立強型別 (e.g., `struct Health(f32)`)，避免 `f32` 混用導致邏輯錯誤。

## 2. Memory Management

- **Copy vs Clone**:
  - 對於小的 Component (如座標、狀態 enum)，derive `Copy`。
  - 對於大物件 (如 Mesh, Texture 資料)，善用 `Arc` 或 `Handle` 來共享所有權，避免 deep clone。

## 3. Error Handling

- **Result & Option**:
  - 遊戲邏輯中盡量避免 `panic!` (除了初始化階段)。
  - 使用 `expect("reason")` 明確指出為何這裡認為不會失敗。
  - 在 System 中，如果發生非致命錯誤，使用 `warn!` log 出來並略過該 frame 的處理，不要讓遊戲崩潰。
