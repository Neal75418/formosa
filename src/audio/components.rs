//! 音效組件

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;

/// 音效管理器資源
#[derive(Resource)]
pub struct AudioManager {
    /// 當前是否為夜間
    pub is_night: bool,
    /// 主音量 (0.0 ~ 1.0)
    pub master_volume: f32,
    /// 音樂音量 (0.0 ~ 1.0)
    pub music_volume: f32,
    /// 音效音量 (0.0 ~ 1.0)
    pub sfx_volume: f32,
    /// 日間背景音樂 handle
    pub day_bgm: Option<Handle<AudioSource>>,
    /// 夜間背景音樂 handle
    pub night_bgm: Option<Handle<AudioSource>>,
    /// 當前播放的 BGM 實體
    pub current_bgm_entity: Option<Entity>,
}

impl Default for AudioManager {
    fn default() -> Self {
        Self {
            is_night: false,
            master_volume: 1.0,
            music_volume: 0.5,
            sfx_volume: 0.8,
            day_bgm: None,
            night_bgm: None,
            current_bgm_entity: None,
        }
    }
}

/// 引擎音效組件
/// 附加到載具上，根據速度調整音高和音量
#[derive(Component)]
pub struct EngineSound {
    /// 基礎音高
    pub base_pitch: f32,
    /// 最大音高（全速時）
    pub max_pitch: f32,
    /// 基礎音量
    pub base_volume: f32,
    /// 引擎類型
    pub engine_type: EngineType,
}

impl Default for EngineSound {
    fn default() -> Self {
        Self {
            base_pitch: 1.0,
            max_pitch: 2.5,
            base_volume: 0.6,
            engine_type: EngineType::Scooter,
        }
    }
}

impl EngineSound {
    /// 機車引擎（125cc 速克達）
    pub fn scooter() -> Self {
        Self {
            base_pitch: 0.8,
            max_pitch: 2.0,
            base_volume: 0.5,
            engine_type: EngineType::Scooter,
        }
    }

    /// 汽車引擎
    pub fn car() -> Self {
        Self {
            base_pitch: 0.6,
            max_pitch: 1.8,
            base_volume: 0.7,
            engine_type: EngineType::Car,
        }
    }

    /// 公車引擎（柴油）
    pub fn bus() -> Self {
        Self {
            base_pitch: 0.4,
            max_pitch: 1.2,
            base_volume: 0.9,
            engine_type: EngineType::Bus,
        }
    }
}

/// 引擎類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineType {
    Scooter,  // 機車
    Car,      // 汽車
    Bus,      // 公車
}

/// 環境音效組件
/// 用於空間音效（3D 定位音效）
#[derive(Component)]
pub struct AmbientSound {
    /// 音效類型
    pub sound_type: AmbientSoundType,
    /// 音量
    pub volume: f32,
    /// 是否循環
    pub looping: bool,
    /// 音效範圍（超過此距離聽不到）
    pub range: f32,
}

impl Default for AmbientSound {
    fn default() -> Self {
        Self {
            sound_type: AmbientSoundType::Crowd,
            volume: 0.5,
            looping: true,
            range: 30.0,
        }
    }
}

impl AmbientSound {
    /// 人群喧嘩聲
    pub fn crowd() -> Self {
        Self {
            sound_type: AmbientSoundType::Crowd,
            volume: 0.4,
            looping: true,
            range: 40.0,
        }
    }

    /// 交通噪音
    pub fn traffic() -> Self {
        Self {
            sound_type: AmbientSoundType::Traffic,
            volume: 0.3,
            looping: true,
            range: 50.0,
        }
    }

    /// 音樂播放（店家）
    pub fn store_music() -> Self {
        Self {
            sound_type: AmbientSoundType::StoreMusic,
            volume: 0.5,
            looping: true,
            range: 15.0,
        }
    }

    /// 夜市叫賣聲
    pub fn night_market() -> Self {
        Self {
            sound_type: AmbientSoundType::NightMarket,
            volume: 0.6,
            looping: true,
            range: 35.0,
        }
    }
}

/// 環境音效類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmbientSoundType {
    Crowd,       // 人群喧嘩
    Traffic,     // 交通噪音
    StoreMusic,  // 店家音樂
    NightMarket, // 夜市叫賣
    Construction,// 施工噪音
    Birds,       // 鳥叫聲（日間）
    Insects,     // 蟲鳴（夜間）
}

/// 音效觸發器組件
/// 進入範圍時播放音效
#[derive(Component)]
pub struct SoundTrigger {
    /// 觸發範圍
    pub trigger_radius: f32,
    /// 音效資源路徑
    pub sound_path: String,
    /// 是否已觸發
    pub triggered: bool,
    /// 冷卻時間（秒）
    pub cooldown: f32,
    /// 剩餘冷卻
    pub cooldown_remaining: f32,
}

// ============================================================================
// 武器音效系統
// ============================================================================

/// 武器音效資源（預載入所有槍聲）
#[derive(Resource, Default)]
pub struct WeaponSounds {
    /// 手槍槍聲
    pub pistol_fire: Option<Handle<AudioSource>>,
    /// 衝鋒槍槍聲
    pub smg_fire: Option<Handle<AudioSource>>,
    /// 霰彈槍槍聲
    pub shotgun_fire: Option<Handle<AudioSource>>,
    /// 步槍槍聲
    pub rifle_fire: Option<Handle<AudioSource>>,
    /// 空彈匣聲
    pub empty_clip: Option<Handle<AudioSource>>,
    /// 換彈開始聲
    pub reload_start: Option<Handle<AudioSource>>,
    /// 換彈完成聲
    pub reload_finish: Option<Handle<AudioSource>>,
    /// 武器切換聲
    pub weapon_switch: Option<Handle<AudioSource>>,
    /// 子彈命中肉體聲
    pub hit_flesh: Option<Handle<AudioSource>>,
    /// 子彈命中金屬聲
    pub hit_metal: Option<Handle<AudioSource>>,
    /// 爆頭音效
    pub headshot: Option<Handle<AudioSource>>,
    /// 揮拳聲
    pub punch_whoosh: Option<Handle<AudioSource>>,
    /// 揮拳命中聲
    pub punch_hit: Option<Handle<AudioSource>>,
}

/// 武器音效類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponSoundType {
    /// 槍聲（依武器類型）
    Fire,
    /// 空彈匣
    EmptyClip,
    /// 換彈開始
    ReloadStart,
    /// 換彈完成
    ReloadFinish,
    /// 武器切換
    WeaponSwitch,
    /// 命中肉體
    HitFlesh,
    /// 命中金屬
    HitMetal,
    /// 爆頭
    Headshot,
    /// 揮拳（空揮）
    PunchWhoosh,
    /// 揮拳命中
    PunchHit,
}

// ============================================================================
// 車輛互動音效 (GTA 5 風格)
// ============================================================================

/// 車輛互動音效資源
#[derive(Resource, Default)]
pub struct VehicleSounds {
    /// 開門聲
    pub door_open: Option<Handle<AudioSource>>,
    /// 關門聲
    pub door_close: Option<Handle<AudioSource>>,
    /// 發動引擎聲
    pub engine_start: Option<Handle<AudioSource>>,
    /// 熄火聲
    pub engine_stop: Option<Handle<AudioSource>>,
    /// 喇叭聲
    pub horn: Option<Handle<AudioSource>>,
    /// 輪胎打滑聲
    pub tire_screech: Option<Handle<AudioSource>>,
    /// 碰撞聲（輕微）
    pub collision_light: Option<Handle<AudioSource>>,
    /// 碰撞聲（嚴重）
    pub collision_heavy: Option<Handle<AudioSource>>,
    /// 爆炸聲
    pub explosion: Option<Handle<AudioSource>>,
}

// ============================================================================
// 玩家互動音效
// ============================================================================

/// 玩家音效資源
#[derive(Resource, Default)]
pub struct PlayerSounds {
    /// 腳步聲（水泥）
    pub footstep_concrete: Option<Handle<AudioSource>>,
    /// 腳步聲（草地）
    pub footstep_grass: Option<Handle<AudioSource>>,
    /// 腳步聲（金屬）
    pub footstep_metal: Option<Handle<AudioSource>>,
    /// 跳躍聲
    pub jump: Option<Handle<AudioSource>>,
    /// 落地聲
    pub land: Option<Handle<AudioSource>>,
    /// 翻滾聲
    pub dodge_roll: Option<Handle<AudioSource>>,
    /// 受傷呻吟聲
    pub hurt: Option<Handle<AudioSource>>,
    /// 死亡聲
    pub death: Option<Handle<AudioSource>>,
}

// ============================================================================
// UI 音效
// ============================================================================

/// UI 音效資源
#[derive(Resource, Default)]
pub struct UISounds {
    /// 任務開始提示音
    pub mission_start: Option<Handle<AudioSource>>,
    /// 任務完成提示音
    pub mission_complete: Option<Handle<AudioSource>>,
    /// 任務失敗提示音
    pub mission_fail: Option<Handle<AudioSource>>,
    /// 檢查點通過音
    pub checkpoint: Option<Handle<AudioSource>>,
    /// 金錢獲得音
    pub money_gain: Option<Handle<AudioSource>>,
    /// 通緝星級增加音
    pub wanted_up: Option<Handle<AudioSource>>,
    /// 通緝消除音
    pub wanted_clear: Option<Handle<AudioSource>>,
}

// ============================================================================
// 音效整合狀態追蹤
// ============================================================================

/// 腳步音效計時器資源
#[derive(Resource)]
pub struct FootstepTimer {
    /// 距離上次腳步聲的累計時間
    pub elapsed: f32,
}

impl Default for FootstepTimer {
    fn default() -> Self {
        Self { elapsed: 0.0 }
    }
}

impl FootstepTimer {
    /// 根據移動狀態取得腳步間隔
    pub fn interval(is_sprinting: bool, is_crouching: bool) -> f32 {
        if is_crouching {
            0.8
        } else if is_sprinting {
            0.3
        } else {
            0.5
        }
    }
}

/// 車輛音效狀態追蹤資源
/// 用於偵測「上車/下車」狀態變化以觸發音效
#[derive(Resource, Default)]
pub struct AudioVehicleState {
    /// 上一幀玩家是否在車內
    pub was_in_vehicle: bool,
}

/// 地面材質標記組件
/// 附加到地面碰撞體上，用於腳步聲材質偵測
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GroundSurface {
    /// 水泥/柏油路（預設）
    #[default]
    Concrete,
    /// 草地/泥土
    Grass,
    /// 金屬（鐵皮/橋面）
    Metal,
}

/// 玩家當前踩踏地面類型資源
#[derive(Resource, Default)]
pub struct PlayerGroundSurface {
    /// 當前腳下材質
    pub surface: GroundSurface,
}

impl From<GroundSurface> for super::systems::FootstepSurface {
    fn from(gs: GroundSurface) -> Self {
        match gs {
            GroundSurface::Concrete => super::systems::FootstepSurface::Concrete,
            GroundSurface::Grass => super::systems::FootstepSurface::Grass,
            GroundSurface::Metal => super::systems::FootstepSurface::Metal,
        }
    }
}

// ============================================================================
// 車載電台系統 (GTA 5 風格)
// ============================================================================

/// 電台頻道
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RadioStation {
    /// 寶島流行樂 — 華語流行
    IslandPop,
    /// 夜市放克 — Funk / Disco
    NightMarketFunk,
    /// 台灣雷鬼 — Reggae / Dub
    TaiwanReggae,
    /// 地下嘻哈 — Hip-Hop / Rap
    UndergroundHipHop,
    /// 古典 FM — Classical
    ClassicFM,
    /// 關閉電台
    Off,
}

impl RadioStation {
    /// 顯示名稱
    pub fn label(self) -> &'static str {
        match self {
            Self::IslandPop => "寶島流行樂",
            Self::NightMarketFunk => "夜市放克",
            Self::TaiwanReggae => "台灣雷鬼",
            Self::UndergroundHipHop => "地下嘻哈",
            Self::ClassicFM => "古典 FM",
            Self::Off => "關閉電台",
        }
    }

    /// 對應的音檔路徑
    pub fn audio_path(self) -> Option<&'static str> {
        match self {
            Self::IslandPop => Some("audio/radio_pop.ogg"),
            Self::NightMarketFunk => Some("audio/radio_funk.ogg"),
            Self::TaiwanReggae => Some("audio/radio_reggae.ogg"),
            Self::UndergroundHipHop => Some("audio/radio_hiphop.ogg"),
            Self::ClassicFM => Some("audio/radio_classical.ogg"),
            Self::Off => None,
        }
    }

    /// 所有可播放的電台（不含 Off）
    pub fn all_stations() -> &'static [RadioStation] {
        &[
            RadioStation::IslandPop,
            RadioStation::NightMarketFunk,
            RadioStation::TaiwanReggae,
            RadioStation::UndergroundHipHop,
            RadioStation::ClassicFM,
        ]
    }

    /// 下一個電台（循環切換，含 Off）
    pub fn next(self) -> RadioStation {
        match self {
            Self::IslandPop => Self::NightMarketFunk,
            Self::NightMarketFunk => Self::TaiwanReggae,
            Self::TaiwanReggae => Self::UndergroundHipHop,
            Self::UndergroundHipHop => Self::ClassicFM,
            Self::ClassicFM => Self::Off,
            Self::Off => Self::IslandPop,
        }
    }

    /// 上一個電台（反向循環）
    pub fn prev(self) -> RadioStation {
        match self {
            Self::IslandPop => Self::Off,
            Self::NightMarketFunk => Self::IslandPop,
            Self::TaiwanReggae => Self::NightMarketFunk,
            Self::UndergroundHipHop => Self::TaiwanReggae,
            Self::ClassicFM => Self::UndergroundHipHop,
            Self::Off => Self::ClassicFM,
        }
    }
}

/// 電台管理器資源
#[derive(Resource)]
pub struct RadioManager {
    /// 當前選擇的電台
    pub current_station: RadioStation,
    /// 上次實際播放的電台（用於偵測切換）
    pub last_played_station: RadioStation,
    /// 當前播放的電台音效實體
    pub playing_entity: Option<Entity>,
    /// 電台音量（獨立於主音量）
    pub radio_volume: f32,
    /// 是否顯示電台切換提示
    pub show_station_name: bool,
    /// 電台名稱顯示倒計時
    pub station_name_timer: f32,
}

impl Default for RadioManager {
    fn default() -> Self {
        Self {
            current_station: RadioStation::Off,
            last_played_station: RadioStation::Off,
            playing_entity: None,
            radio_volume: 0.6,
            show_station_name: false,
            station_name_timer: 0.0,
        }
    }
}

// ============================================================================
// 警察無線電系統
// ============================================================================

/// 警察無線電音效資源
#[derive(Resource)]
pub struct PoliceRadioState {
    /// 距離下次無線電碎語的倒計時
    pub next_chatter_timer: f32,
    /// 無線電音效清單（隨機播放）
    pub chatter_sounds: Vec<Option<Handle<AudioSource>>>,
    /// 是否啟用
    pub active: bool,
}

impl Default for PoliceRadioState {
    fn default() -> Self {
        Self {
            next_chatter_timer: 5.0,
            chatter_sounds: Vec::new(),
            active: false,
        }
    }
}

impl PoliceRadioState {
    /// 根據通緝星級計算無線電間隔（星級越高越頻繁）
    /// 1★ = 8-12 秒, 2★ = 6-10 秒, 3★ = 4-7 秒, 4★ = 2-5 秒, 5★ = 1-3 秒
    pub fn interval_for_stars(stars: u8) -> (f32, f32) {
        match stars {
            0 => (999.0, 999.0), // 不播放
            1 => (8.0, 12.0),
            2 => (6.0, 10.0),
            3 => (4.0, 7.0),
            4 => (2.0, 5.0),
            _ => (1.0, 3.0), // 5★+
        }
    }

    /// 產生隨機間隔
    pub fn random_interval(stars: u8) -> f32 {
        let (min, max) = Self::interval_for_stars(stars);
        min + rand::random::<f32>() * (max - min)
    }
}

// ============================================================================
// NPC 環境對話系統
// ============================================================================

/// NPC 環境對話短句（台灣口語）
pub const NPC_DIALOGUE_LINES: &[&str] = &[
    "今天天氣真好欸！",
    "肚子好餓喔…",
    "等等要去哪裡吃？",
    "哎呀，好累喔！",
    "你看那邊那個人…",
    "走快一點啦！",
    "手機沒電了啦…",
    "好無聊喔～",
    "欸等一下！",
    "要不要喝飲料？",
    "哇，好熱喔！",
    "這條路好像不太對…",
    "齁，又塞車了！",
    "你有沒有聽到什麼？",
    "今天上班好累…",
    "晚上要不要去夜市？",
];

/// NPC 環境對話管理器資源
#[derive(Resource)]
pub struct NpcDialogueManager {
    /// 觸發半徑（玩家接近 NPC 的距離）
    pub trigger_radius: f32,
    /// 全域冷卻計時器（防止太密集的對話）
    pub global_cooldown: f32,
    /// 全域冷卻間隔
    pub cooldown_interval: f32,
    /// 對話音效（若有音效檔可用）
    pub dialogue_sounds: Vec<Option<Handle<AudioSource>>>,
}

impl Default for NpcDialogueManager {
    fn default() -> Self {
        Self {
            trigger_radius: 8.0,
            global_cooldown: 0.0,
            cooldown_interval: 10.0, // 至少間隔 10 秒
            dialogue_sounds: Vec::new(),
        }
    }
}

/// NPC 對話冷卻組件
/// 附加到行人上，防止同一個 NPC 連續說話
#[derive(Component, Default)]
pub struct NpcDialogueCooldown {
    /// 剩餘冷卻時間
    pub remaining: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radio_station_next_cycles_through_all() {
        let mut station = RadioStation::IslandPop;
        let mut visited = vec![station];
        for _ in 0..5 {
            station = station.next();
            visited.push(station);
        }
        // IslandPop → NightMarketFunk → TaiwanReggae → UndergroundHipHop → ClassicFM → Off
        assert_eq!(visited.len(), 6);
        assert_eq!(visited[5], RadioStation::Off);
        // Off.next() 回到 IslandPop
        assert_eq!(RadioStation::Off.next(), RadioStation::IslandPop);
    }

    #[test]
    fn radio_station_prev_cycles_backwards() {
        assert_eq!(RadioStation::IslandPop.prev(), RadioStation::Off);
        assert_eq!(RadioStation::Off.prev(), RadioStation::ClassicFM);
        assert_eq!(RadioStation::ClassicFM.prev(), RadioStation::UndergroundHipHop);
    }

    #[test]
    fn radio_station_next_prev_inverse() {
        for station in RadioStation::all_stations() {
            assert_eq!(station.next().prev(), *station);
            assert_eq!(station.prev().next(), *station);
        }
        // Off 也滿足
        assert_eq!(RadioStation::Off.next().prev(), RadioStation::Off);
    }

    #[test]
    fn radio_all_stations_count() {
        assert_eq!(RadioStation::all_stations().len(), 5);
    }

    #[test]
    fn radio_off_has_no_audio_path() {
        assert!(RadioStation::Off.audio_path().is_none());
    }

    #[test]
    fn radio_stations_have_audio_paths() {
        for station in RadioStation::all_stations() {
            assert!(station.audio_path().is_some(), "{:?} should have audio path", station);
        }
    }

    #[test]
    fn radio_station_labels_not_empty() {
        for station in RadioStation::all_stations() {
            assert!(!station.label().is_empty());
        }
        assert!(!RadioStation::Off.label().is_empty());
    }

    #[test]
    fn footstep_intervals() {
        let sprint = FootstepTimer::interval(true, false);
        let walk = FootstepTimer::interval(false, false);
        let crouch = FootstepTimer::interval(false, true);
        assert!(sprint < walk);
        assert!(walk < crouch);
    }

    #[test]
    fn radio_manager_defaults_to_off() {
        let manager = RadioManager::default();
        assert_eq!(manager.current_station, RadioStation::Off);
        assert!(manager.playing_entity.is_none());
    }

    #[test]
    fn engine_presets_different_pitch_ranges() {
        let scooter = EngineSound::scooter();
        let car = EngineSound::car();
        let bus = EngineSound::bus();
        // 機車基礎音高最高，公車最低
        assert!(scooter.base_pitch > car.base_pitch);
        assert!(car.base_pitch > bus.base_pitch);
        // 機車最大音高最高，公車最低
        assert!(scooter.max_pitch > car.max_pitch);
        assert!(car.max_pitch > bus.max_pitch);
    }

    #[test]
    fn engine_presets_bus_loudest() {
        let scooter = EngineSound::scooter();
        let car = EngineSound::car();
        let bus = EngineSound::bus();
        // 公車引擎聲最大
        assert!(bus.base_volume > car.base_volume);
        assert!(car.base_volume > scooter.base_volume);
    }

    #[test]
    fn engine_type_matches_preset() {
        assert_eq!(EngineSound::scooter().engine_type, EngineType::Scooter);
        assert_eq!(EngineSound::car().engine_type, EngineType::Car);
        assert_eq!(EngineSound::bus().engine_type, EngineType::Bus);
    }

    #[test]
    fn ground_surface_to_footstep_surface() {
        use crate::audio::FootstepSurface;
        assert_eq!(FootstepSurface::from(GroundSurface::Concrete), FootstepSurface::Concrete);
        assert_eq!(FootstepSurface::from(GroundSurface::Grass), FootstepSurface::Grass);
        assert_eq!(FootstepSurface::from(GroundSurface::Metal), FootstepSurface::Metal);
    }

    #[test]
    fn player_ground_surface_defaults_to_concrete() {
        let pgs = PlayerGroundSurface::default();
        assert_eq!(pgs.surface, GroundSurface::Concrete);
    }

    #[test]
    fn police_radio_interval_decreases_with_stars() {
        let (min_1, max_1) = PoliceRadioState::interval_for_stars(1);
        let (min_3, max_3) = PoliceRadioState::interval_for_stars(3);
        let (min_5, max_5) = PoliceRadioState::interval_for_stars(5);
        // 星級越高，間隔越短
        assert!(min_1 > min_3);
        assert!(min_3 > min_5);
        assert!(max_1 > max_3);
        assert!(max_3 > max_5);
    }

    #[test]
    fn police_radio_zero_stars_huge_interval() {
        let (min, _max) = PoliceRadioState::interval_for_stars(0);
        assert!(min > 100.0); // 0 星不播放
    }

    #[test]
    fn police_radio_random_interval_in_range() {
        for stars in 1..=5 {
            let (min, max) = PoliceRadioState::interval_for_stars(stars);
            for _ in 0..50 {
                let interval = PoliceRadioState::random_interval(stars);
                assert!(interval >= min, "stars={stars}, interval={interval} < min={min}");
                assert!(interval <= max, "stars={stars}, interval={interval} > max={max}");
            }
        }
    }

    #[test]
    fn police_radio_default_inactive() {
        let state = PoliceRadioState::default();
        assert!(!state.active);
    }

    #[test]
    fn npc_dialogue_lines_not_empty() {
        assert!(!NPC_DIALOGUE_LINES.is_empty());
        for line in NPC_DIALOGUE_LINES {
            assert!(!line.is_empty());
        }
    }

    #[test]
    fn npc_dialogue_lines_count() {
        assert!(NPC_DIALOGUE_LINES.len() >= 10, "至少 10 句對話");
    }

    #[test]
    fn npc_dialogue_defaults() {
        let manager = NpcDialogueManager::default();
        assert!(manager.trigger_radius > 0.0);
        assert!(manager.cooldown_interval > 0.0);
        assert_eq!(manager.global_cooldown, 0.0);
        assert_eq!(NpcDialogueCooldown::default().remaining, 0.0);
    }
}
