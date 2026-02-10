//! 音效組件

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
