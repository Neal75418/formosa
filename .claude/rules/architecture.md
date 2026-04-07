---
paths:
  - "src/main.rs"
  - "src/*/mod.rs"
  - "src/lib.rs"
---

# 架構圖

## 分層架構

```mermaid
graph TD
    subgraph L5["<b>Presentation</b>"]
        ui["ui — HUD · 小地圖 · 武器輪盤 · GPS<br><small>39 個檔案</small>"]
        audio["audio — BGM · 引擎聲 · 3D 音效"]
        camera["camera — 跟隨 · 震動 · 後座力"]
    end

    subgraph L4["<b>Game Systems</b>"]
        wanted["wanted — 5 星通緝 · 警車 AI · 直升機 · 路障<br><small>20 個檔案</small>"]
        mission["mission — 劇情 · 對話 · 過場動畫<br><small>32 個檔案</small>"]
        economy["economy — 商店 · ATM · 股市 · 賭場"]
        env["environment — 可破壞物件 · 碎片池"]
    end

    subgraph L3["<b>Entities</b>"]
        player["player — 移動 · 跳躍 · 閃避 · 攀爬"]
        vehicle["vehicle — 物理 · 偷車 · 改裝 · 效果<br><small>28 個檔案</small>"]
        combat["combat — 射擊 · 爆炸 · 掩體 · 布娃娃<br><small>29 個檔案</small>"]
        ai_mod["ai — 感知 · 決策 · 小隊 · 掩護<br><small>16 個檔案</small>"]
        ped["pedestrian — 尋路 · 恐慌 · 目擊者<br><small>20 個檔案</small>"]
    end

    subgraph L2["<b>World</b>"]
        world["world — 西門町 · 建築 · 天氣 · 隨機事件<br><small>25 個檔案</small>"]
    end

    subgraph L1["<b>Core</b>"]
        core["core — AppState · 空間哈希 · 物件池 · 數學<br><small>8 個檔案</small>"]
    end

    subgraph L0["<b>Persistence</b>"]
        save["save — 非同步 IO · JSON 序列化"]
    end

    ui & audio & camera --> player & vehicle & combat
    wanted --> player & vehicle & combat & ai_mod
    mission --> player & vehicle & combat & economy
    env --> combat
    player & vehicle & combat & ai_mod & ped --> core
    world --> core
    save --> player & vehicle & combat & economy & mission
```

## 系統執行順序

```mermaid
flowchart LR
    subgraph GameSet["<b>GameSet（依序）</b>"]
        direction LR
        GS1["Player"] --> GS2["Vehicle"] --> GS3["World"] --> GS4["Ui"]
    end

    subgraph InteractionSet["<b>InteractionSet（優先序）</b>"]
        direction LR
        IS1["Vehicle"] --> IS2["Mission"] --> IS3["Economy"] --> IS4["Interior"]
    end

    subgraph Pipeline["<b>主迴圈管線</b>"]
        direction TB
        PreUpdate["PreUpdate<br><small>update_interaction_state</small>"]
        Update["Update<br><small>GameSet + InteractionSet</small>"]
        Camera_S["Camera<br><small>input → auto_follow → follow → shake</small>"]
        PreUpdate --> Update --> Camera_S
    end
```

暫停控制：`.run_if(|ui: Res<UiState>| !ui.paused)`

## 17 個 Bevy Plugins

```mermaid
graph LR
    subgraph Entities
        PlayerPlugin
        VehiclePlugin
        CombatPlugin
        AiPlugin
        PedestrianPlugin
    end

    subgraph Systems
        WantedPlugin
        EconomyPlugin
        EnvironmentPlugin
    end

    subgraph Mission_P["Mission（4 個子插件）"]
        DialogueSystemPlugin
        DialogueUIPlugin
        CutsceneSystemPlugin
        StoryMissionPlugin
    end

    subgraph Media["媒體"]
        CameraPlugin
        AudioPlugin
    end

    subgraph Infra["基礎設施"]
        WorldPlugin
        UiPlugin
        SavePlugin
    end
```

> 全部 17 個模組皆使用 Plugin 形式，統一在 `main.rs` 中以 `.add_plugins()` 註冊。
