# 🏝️ 島嶼狂飆 Island Rampage

> 一款以台灣為舞台的 3D 開放世界動作冒險遊戲
> A GTA-style open-world action game set in Taiwan

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Bevy](https://img.shields.io/badge/Bevy_0.17-232326?style=for-the-badge)
![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)

## 🎮 遊戲簡介

在霓虹燈閃爍的台灣街頭，體驗最道地的開放世界冒險！

從西門町的繁華街道開始，駕駛各式車輛、與警察周旋、完成任務、累積財富。

## ✨ 已實現功能

### 🎯 戰鬥系統
- **射擊** - 多種武器（手槍、步槍、霰彈槍、衝鋒槍）
- **近戰** - 棍棒、刀具
- **爆炸物** - 手榴彈、汽油彈、C4 炸藥
- **掩體** - AI 掩護點系統
- **車上射擊** - 駕駛中射擊

### 🚗 載具系統
- **多種車輛** - 轎車、跑車、SUV、機車
- **偷車動畫** - 完整的偷車流程
- **車輛改裝** - 引擎/變速箱/懸吊/煞車/輪胎/裝甲升級
- **氮氣加速** - 瞬間爆發加速
- **車輛損壞** - 視覺損壞效果

### 👮 通緝系統
- **5 星通緝等級** - 從輕微犯罪到全城追捕
- **警車追逐** - AI 警車包圍戰術
- **警用直升機** - 5 星時出動，探照燈追蹤
- **路障系統** - 動態路障封鎖
- **投降/逮捕** - 可選擇投降或逃跑

### 🌆 開放世界
- **西門町場景** - 霓虹招牌、街道、建築
- **可破壞環境** - 玻璃窗、木製障礙、金屬物件
- **行人 AI** - 恐慌反應、逃跑行為
- **交通系統** - 紅綠燈、車流
- **隨機事件** - 街頭搶劫、車禍等

### 💰 經濟與進度
- **金錢系統** - 賺取與消費
- **任務系統** - 劇情任務、評分機制
- **存檔系統** - 非同步 IO、JSON 序列化

### ⚡ 效能優化
- **空間哈希** - O(1) 碰撞/視野檢測
- **物件池** - 碎片重用，減少記憶體分配

## 🛠️ 技術棧

| 項目 | 技術 |
|------|------|
| 語言 | Rust 🦀 |
| 引擎 | [Bevy](https://bevyengine.org/) 0.17 |
| 物理 | bevy_rapier3d |
| 風格 | Low-poly 霓虹風 |

## 🚀 開發

### 環境需求

- Rust 1.75+
- 支援 Vulkan/Metal/DX12 的顯示卡

### 運行遊戲

```bash
# 開發模式
cargo run

# 發布模式（最佳效能）
cargo run --release

# 測試
cargo test
```

### 操作方式

| 按鍵 | 動作 |
|------|------|
| WASD | 移動 |
| Space | 跳躍 |
| Shift | 衝刺 / 氮氣加速 |
| 滑鼠左鍵 | 射擊 / 攻擊 |
| 滑鼠右鍵 | 瞄準 |
| R | 換彈 |
| E | 互動 / 上下車 |
| F | 偷車 |
| G | 投擲爆炸物 |
| 1-4 | 切換武器 |
| Tab | 武器輪盤 |
| Esc | 暫停選單 |

## 📍 開發進度

### ✅ Phase 1：核心系統
- [x] 玩家移動與控制
- [x] 金錢/購物系統
- [x] 存檔/讀取系統
- [x] 任務評分機制

### ✅ Phase 2：戰鬥系統
- [x] 射擊系統
- [x] 掩體系統
- [x] 車上射擊
- [x] 爆炸物

### ✅ Phase 3：通緝系統
- [x] 警車追逐 AI
- [x] 路障系統
- [x] 投降/逮捕機制

### ✅ Phase 4：開放世界
- [x] 隨機事件
- [x] 可破壞環境
- [x] 偷車動畫

### ✅ Phase 5：進階功能
- [x] 警用直升機
- [x] 近戰武器
- [x] 車輛改裝系統
- [x] 空間哈希優化

### 🔮 Phase 6+：未來規劃
- [ ] 手機系統（任務接取、地圖）
- [ ] 游泳/潛水
- [ ] 車內廣播電台
- [ ] 攀爬系統
- [ ] 多角色切換

## 📂 專案結構

```
island-rampage/
├── src/
│   ├── main.rs              # 程式入口
│   ├── core/                # 核心系統（資源、事件、空間哈希）
│   ├── player/              # 玩家控制
│   ├── combat/              # 戰鬥系統（射擊、爆炸、掩體）
│   ├── vehicle/             # 載具系統（駕駛、改裝、偷車）
│   ├── wanted/              # 通緝系統（警察、直升機、路障）
│   ├── pedestrian/          # 行人 AI
│   ├── economy/             # 經濟系統
│   ├── mission/             # 任務系統
│   ├── save/                # 存檔系統
│   ├── world/               # 世界生成
│   ├── environment/         # 環境互動（可破壞物件）
│   ├── ui/                  # 使用者介面
│   ├── audio/               # 音效系統
│   └── ai/                  # AI 系統
├── assets/                  # 遊戲資源
├── Cargo.toml              # Rust 依賴
└── README.md               # 本文件
```

## 🧪 測試

```bash
# 執行所有測試（178 個）
cargo test

# 執行特定模組測試
cargo test --package island-rampage economy::tests

# 檢查程式碼
cargo clippy
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
- [Rapier](https://rapier.rs/) - 高效能物理引擎
- 台灣這片美麗的土地 🇹🇼

---

Made with ❤️ in Taiwan
