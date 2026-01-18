# 🏝️ 島嶼狂飆 Island Rampage

> 一款以台灣為舞台的 3D 開放世界動作冒險遊戲
> A 3D open-world action-adventure game set in Taiwan

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Bevy](https://img.shields.io/badge/Bevy-232326?style=for-the-badge)
![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)

## 🎮 遊戲簡介

在霓虹燈閃爍的台灣街頭，騎上機車，展開你的環島冒險！

從繁華的台北信義區到南台灣的墾丁海灘，體驗最道地的台灣風情。

## ✨ 特色

- 🇹🇼 **真實台灣場景** - 以台灣各大城市為原型
- 🛵 **機車文化** - 最道地的台灣移動方式
- 🌃 **霓虹都市** - Low-poly 風格的霓虹夜景
- 🗺️ **環島公路** - 探索整個台灣！

## 🛠️ 技術棧

- **語言**：Rust 🦀
- **引擎**：[Bevy](https://bevyengine.org/) 0.15
- **風格**：Low-poly 霓虹風

## 🚀 開發

### 環境需求

- Rust 1.75+
- 支援 Vulkan/Metal/DX12 的顯示卡

### 運行遊戲

```bash
# 開發模式（較快編譯）
cargo run

# 發布模式（最佳效能）
cargo run --release
```

### 操作方式

| 按鍵 | 動作 |
|----|----|
| W  | 前進 |
| S  | 後退 |
| A  | 左移 |
| D  | 右移 |

## 📍 開發路線圖

### Phase 1：核心原型 🚧

- [x] 專案建立
- [x] 基本 3D 場景
- [x] 玩家移動
- [ ] 機車系統
- [ ] 基本物理

### Phase 2：台北市

- [ ] 西門町區域
- [ ] 信義區 + 101
- [ ] 捷運系統
- [ ] 日夜循環

### Phase 3：環島公路

- [ ] 高雄市
- [ ] 環島公路
- [ ] 鄉村風景

### Phase 4：完整台灣

- [ ] 花東地區
- [ ] 各地特色任務
- [ ] 完整環島體驗

## 📂 專案結構

```
island-rampage/
├── src/
│   └── main.rs          # 主程式
├── assets/              # 遊戲資源
│   ├── models/          # 3D 模型
│   ├── textures/        # 材質貼圖
│   └── audio/           # 音效音樂
├── Cargo.toml           # Rust 依賴
└── README.md            # 本文件
```

## 🤝 貢獻

歡迎任何形式的貢獻！

- 🐛 回報 Bug
- 💡 提出新功能建議
- 🎨 貢獻美術資源
- 🔧 提交 Pull Request

## 📜 授權

MIT License - 詳見 [LICENSE](LICENSE)

## 🙏 致謝

- [Bevy Engine](https://bevyengine.org/) - 優秀的 Rust 遊戲引擎
- 台灣這片美麗的土地 🇹🇼

---

Made with ❤️ in Taiwan
