# Changelog

All notable changes to this project will be documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/) · Commits: [Conventional Commits](https://www.conventionalcommits.org/)

---

## [Unreleased]

### Changed

- **地圖改由資料檔驅動**：道路、斑馬線、號誌、NPC 車路線、建築、行人網格、小地圖道路與邊界改讀 `assets/levels/ximending.ron`，程式從路網推算位置；讀檔時檢查路名與交會，寫錯時啟動直接報錯並指出是哪一筆
- **專案更名為 formosa**：repo 與 Cargo 套件名改為 `formosa`；遊戲顯示名「島嶼狂飆」與存檔資料夾 `IslandRampage` 不變
- **中文字型改用 Noto Sans TC Medium**（SIL OFL 1.1，授權檔 `assets/fonts/NotoSansTC-OFL.txt`），取代從 macOS 系統複製來的 STHeiti（系統字型不宜隨 repo 散布）
- **移除未使用的貼圖** `assets/textures/roads/brick.jpg`
- **修正新版 clippy 回報的 39 處警告**（Rust 1.98 回報 17 處；1.99 加上新的 assert_is_empty 共 39 處）：map_unwrap_or、manual_midpoint、collapsible_match、manual_is_variant_and、assert_is_empty；本機以 CI 同款指令（Rust 1.99、`-D warnings`）驗證為 0
- **全專案 clippy pedantic lint 清理**：1,517 個 pedantic warnings → 0，222 個檔案（+7,470/-4,557 行）
- **全 codebase 壞氣味修復**：God Function 拆分、SystemParam 重構、dead_code 清理
- **移除 31 個驗證過的死碼欄位與常數**：37 個檔案，淨刪 304 行（逐一 grep 驗證非預留功能）
- **CI 條件式磁碟清理**：self-hosted runner 空間不足 2GB 時自動執行 cargo clean

### Added

- **天空與大氣霧**：天空改成程式產生的漸層天空盒（天頂、地平線色依時間內插；夜晚地平線是城市光害的暗橘紫，日出日落有暖色光暈；沿用原本的天氣調色）；霧一律開啟（晴天能見度 300 m，天氣越差越濃），霧色等於地平線色，遠處建築與地面融進天空，街道盡頭不再看到地面截斷；地面平板視覺上放大到 2000 m（碰撞不變）；月亮不吃距離霧，改依天氣淡出（晴天清楚、陰天半透明、雨霧沙塵看不到）
- **建築外牆貼圖**：程式繪製 4×4 開間的窗戶圖集（窗框、玻璃、鐵窗、冷氣室外機，含 mip 鏈）與夜間發光遮罩，依牆面實際尺寸重複；牆面改用 6 色低飽和台北公寓色盤（依店名決定）；套用到 `spawn_building_base` 的 11 種風格與 generic 建築；夜間亮燈改成每棟樓只擲一次骰子，時段不變就不會每 7 秒整片開關
- **BRP 遙控測試模式**：`cargo brp`（`brp` feature，僅 Debug）開啟 Bevy Remote Protocol（localhost:15702），遊戲在背景也能送按鍵、截圖、查實體；存檔改寫到暫存目錄
- **狙擊槍 + RPG 武器系統**：SniperRifle（85 傷、200m 射程、狙擊鏡 FOV 15度）、RPG（投射物飛行 + 碰撞爆炸、80m/s 彈速、10m 爆炸半徑）
- **隱匿擊殺系統**：StealthTakedownPhase 三階段動畫（接近→執行→完成，共 1.0s）、背後條件判定、10 倍傷害加成、相機震動特寫
- **玩家游泳系統**：水中 WASD 移動、Space 上浮/Ctrl 下潛、Shift 快游、體力消耗、憋氣計時、溺水自傷
- **載具視覺變形系統**：6 部位碰撞變形（引擎蓋/前後保險桿/左右側板/車頂）、材質暗化（lerp 至焦黑色）、離散縮放 + 位移
- **車內廣播電台系統**：8 個台灣主題頻道、Q/E 快捷切換、音量淡入淡出、手機開啟時自動靜音
- **股票市場手機 App**：行情/持倉/交易三分頁、6 支台灣主題股票即時行情、買賣交易 UI
- **車輛改裝商店手機 App**：6 項改裝類別 UI（引擎/變速箱/懸吊/煞車/輪胎/裝甲）、等級/價格/效果顯示、購買互動系統
- **PhoneContentCleanupQueries SystemParam**：重構手機 UI 清理查詢，減少系統參數複雜度（14→10）
- **測試覆蓋擴展**：從 386 增加到 817 個單元測試

### Fixed

- **走路時警察看不到你、不會追捕，直升機打不到你**：警察視線、警察開槍和逮捕前的視線檢查、直升機射擊的射線，起點從身體中心抬高（警察抬 1.5 m、直升機在槍口），方向和長度卻還是從身體中心算，整條射線平行偏移。警察的視線從玩家頭頂飛過，空曠處也看不到你（實測正前方 10 m 也看不到；只有緊貼（約 1 m 內）路燈、牆這類比頭高的東西時會誤判成看得到），一直停在警戒狀態、不會追捕也不會開槍，通緝只會隨時間消退；開槍和逮捕的檢查反而擋不住矮牆、圍籬；直升機的子彈偏離玩家，懸停在玩家北邊 10～30 m 都打不到（測試）。改成方向和長度都從射線起點算到玩家，打到玩家本人（或命中點就在玩家身上）才算看得到、打得到，玩家貼著牆時牆會擋住
- **投降後會被開槍、通緝清掉後警察還在打、跟丟玩家也不去找**：警察開槍不看玩家有沒有投降、通緝是不是已經清掉；追捕中的警察一進入開槍距離（25 m）就轉成交戰，不會走到 2 m 的逮捕距離；逮捕完成那一刻投降就解除，BUSTED 畫面播完、被帶走之前圍在旁邊的警察會開槍；交戰中的警察要離開 37.5 m 以上才會離開交戰，看不到玩家也留在原地。改成玩家投降時警察和直升機停火、警察走到逮捕距離內等逮捕，投降一直維持到被帶走；舉手後離開原地 0.5 m（走開、上車、被打退）就算放棄投降，逮捕途中移動算拒捕；逮捕完成前死掉就放下手、這次逮捕作廢（BUSTED 開始後才死，照樣會被送到警局）；舉手時不能開槍、丟或引爆爆炸物；通緝清掉時警戒、追捕、交戰中的警察收隊回巡邏；交戰中看不到玩家就改成搜索
- **攀爬會把剛倒下的敵人屍體當成障礙物、角色切換中也能攀爬**：偵測多排除屍體的碰撞群組（新增具名常數 `COLLISION_GROUP_RAGDOLL`）；角色切換動畫期間不偵測攀爬（目前遊戲裡還沒有解鎖其他角色的入口）
- **攀爬只會往背後爬**：偵測用的是 `forward()`（角色背後），面對牆按 Space 只會原地跳，障礙物在背後才會攀爬；攀爬時往東西方向的轉向是反的；落點只算到落腳面上 0.1（站立應是 0.7），爬上平台時身體陷進平台。改成偵測角色正面、轉向面向攀爬方向、落點是落腳面加站立高度；在車上（Space 是手煞車）和死亡時不偵測攀爬、偵測不把行人和敵人當成障礙物；自動攀爬改成只在衝刺時觸發（原本門檻是速度 8，一般走路就有 10，改看正面後走路撞到販賣機就會自己爬上去）
- **在陸地上被當成游泳**：游泳偵測只看身體高度低於 0.5，可是地圖上根本沒有水；攀爬時身體一壓低就判定入水（實測短暫入水；上車那條已由下面「騎過機車」那項補了例外），而且出水要高過 1.0，站回地面高度也解不掉。改成要在水域範圍裡才會入水、離開水域就解除；西門町目前沒有水域
- **死掉後卡在 WASTED、擊殺慢動作讓遊戲凍住**：WASTED／BUSTED 和 kill cam（爆頭、打死最後一個敵人、3 連殺）用 `TimeUpdateStrategy::ManualDuration` 做慢動作，它連真實時間也一起換成那段時長，每幀的時間越縮越短：WASTED 遲遲播不完（實測 40 秒沒結束）、期間不能重生；kill cam 一觸發遊戲時間就停住、再也不恢復（實測 20 秒後角色仍不能動）。改成調整遊戲時間（`Time<Virtual>`）的速度，計時用真實時間
- **騎過機車就一直處於游泳狀態**：游泳偵測只看玩家高度低於 0.5，上機車時玩家高度被設成機車的 0.4 就判定入水；進座位時被打死、機車不見或剛坐上就被打死，玩家也會停在機車的高度。出水要高過 1.0、在陸地上永遠解不掉，之後走路被擋掉、改用游泳移動。改成在車上或上下車途中不做游泳偵測，中止上車或清掉在車上的狀態時比照下車把玩家放回地面高度
- **在車上死掉或車不見了，之後一直卡在「在車上」**：在車上被打死，重生後被拉回車上、繼續開車；車爆炸消失，玩家一直隱形（重生後也是）、走不動；上車途中被打死，重生後動畫跑完又被拉進車裡；上車途中車不見，隱藏的玩家不會再出現。改成在車上但人死了或車不見了時，比照下車完成清掉狀態、交還車子、讓玩家出現；上車途中死了就中止上車
- **開車時小地圖標記停在上車的地方、上下車時車子被彈開**：開車時沒人更新玩家位置，玩家一直留在上車點；玩家碰撞體也還開著，上車完成、下車動畫開始時都和車子重疊，車被擠開 1〜10 m。改成開車時玩家跟著車、面向車頭，在車上的整段期間（含下車動畫）關掉玩家碰撞體；下車射線也排除玩家
- **警車沒有生在後方或側面**：生成方向把角色背後（`forward()`）當成前方；改用角色面向，開車時（角色面向車頭）警車生在車子後方或側面
- **走向車門時背對車子、走到車子另一邊的門**：角色正面是 +Z，轉向卻用 −Z 的慣例算；選車門時左右判反，玩家一律穿過車身走到對面那側。改成面向車子、走玩家這一側的門
- **上不了車**：上車前檢查路徑的射線從玩家身體裡面出發、第一個打到的是自己，汽車和機車都被判定擋住；改成排除玩家自己
- **T 字路口在沒有路的那一邊也畫了斑馬線**：斑馬線只畫在真的有路的那幾邊
- **斑馬線條紋方向錯了 90°**：條紋改成橫跨整條路，不再只蓋住路中間、伸進路口
- **走在外圍道路外側那半邊的行人會突然消失**：行人的越界線從外圍道路中線改成地圖邊界；逃跑範圍跟著改成邊界往內 5 m
- **小地圖、大地圖的玩家箭頭不會轉**：旋轉改寫到 UI 的 `UiTransform`（Bevy 0.17 的 UI 不讀 3D `Transform`），箭頭跟著玩家面向轉；面向改取角色模型的正面（本地 +Z）
- **GPS 方向箭頭從不顯示、轉彎提示左右顛倒**：箭頭的查詢要求 UI 節點沒有的 `Transform`，所以一直隱藏；面向誤用 `forward()`，正對目的地時提示「迴轉」；角度也限制在 ±180° 內，不會因為跨過某個方向就變成「迴轉」
- **天氣圖示的太陽光芒、命中標記的 X 不會斜**：旋轉同樣改寫到 `UiTransform`；光芒改成沿半徑朝外
- **受傷方向指示器前後、左右顛倒**：同樣誤用 `forward()` 當玩家面向，敵人在正前方時亮下緣、在右邊時亮左緣；改用角色模型的正面
- **小地圖、大地圖南北顛倒**：北（漢口街）改在上方；GPS 目的地標記一起修正
- **凌晨夜間光從地平線下往上照**：0–6 點地面只剩環境光；整晚改沿用入夜（18–24 點）的方向，從上方沿著南北向街道照，凌晨跟入夜一樣亮
- **下午太陽跑到地平線下**：太陽旋轉的組合順序錯誤，12:00–18:00 光線從地平線下往上照，整個下午到黃昏地面只剩環境光；白天改為先抬仰角再轉方位角（正午最高）

### Changed

- ModShop icon 統一為 ASCII 風格（W），與其他 Phone App 一致
- handle_mod_shop_buttons 簡化為純事件發送，信任系統層驗證
- Unified markdown style with Mermaid architecture diagrams
- Updated README.md, CLAUDE.md, LICENSE, .gitignore
- Added CHANGELOG.md and PR template

### Fixed

- ModShop UI 競態條件（改為事件驅動 ModificationCompleteEvent 通知）
- Runtime unwrap() 使用（改用 next.price() 安全取值）
- ModShop 購買後 UI 未自動刷新（新增 wallet.is_changed() 檢測）
- ModShop UI 重複驗證邏輯（簡化 handle_mod_shop_buttons，-22 行）
- Clippy `field_reassign_with_default` 錯誤（story_manager.rs）
- Clippy `assertions_on_constants` 錯誤（audio/components.rs）
- 手機開啟時數字鍵同時觸發電台切換的輸入衝突

---

## [0.1.0] — 2026-01-19

### Added

- **Phase 1** — Core systems: player control, economy, save/load
- **Phase 2** — Combat: shooting, cover, explosives, damage, ragdoll
- **Phase 3** — Wanted system: 5-star levels, police AI, helicopter, roadblocks
- **Phase 4** — Open world: random events, destructible environment, car theft
- **Phase 5** — Advanced features: helicopter, melee, vehicle mods, performance
- **Phase 6** — Code quality: module splitting, complexity optimization
- **Phase 7** — Architecture refactor: God Module splitting, component decomposition
- **Phase 8** — Test coverage: 329 unit tests across all core modules

### Fixed

- CI sccache install permission issue (switched to `~/.cargo/bin`)

### Technical

- Rust 2021 Edition + Bevy 0.17 + bevy_rapier3d 0.32
- 140 `.rs` files, ~62,800 lines of code
- Spatial hash grid for O(1) proximity queries
- Async save/load with JSON serialization
- GitHub Actions CI with sccache
