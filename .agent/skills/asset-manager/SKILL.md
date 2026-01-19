---
name: asset-manager
description: Expert guidance on 3D Assets, GLTF workflow, and Audio
---

# Asset Manager Guide

## 1. Asset Loading

- **Format**: 3D 模型統一使用 `.glb` (Binary glTF)。
- **Async Loading**:
  - 使用 `AssetServer.load("models/player.glb#Scene0")`。
  - 處理 Loading State：在進入 `AppState::InGame` 前，確保所有必要資源已載入 (可使用 `bevy_asset_loader` crate)。

## 2. Directory Structure

```
assets/
  ├── models/      # 角色、場景 (.glb)
  ├── textures/    # UI、粒子貼圖 (.png, .ktx2)
  ├── audio/       # 音效、BGM (.ogg, .wav)
  ├── levels/      # 關卡設定檔 (.ron)
  └── shaders/     # 自定義 Shader (.wgsl)
```

## 3. Performance

- **Level of Detail (LOD)**:
  - 對於遠景物件 (如遠處的 101 大樓)，準備低多邊形版本。
- **Texture Compression**:
  - 盡可能使用壓縮紋理格式 (KTX2 / DDS) 以減少 GPU VRAM 佔用。
