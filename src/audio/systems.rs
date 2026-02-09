//! 音效系統
#![allow(dead_code)]

// Bevy 系統需要 Res<T> 按值傳遞
#![allow(clippy::needless_pass_by_value)]

use bevy::prelude::*;
use crate::core::WorldTime;
use crate::vehicle::Vehicle;
use crate::player::Player;
use super::{AudioManager, EngineSound, AmbientSound, WeaponSounds};
use crate::combat::WeaponType;

/// 初始化音效資源
pub fn setup_audio(
    mut commands: Commands,
) {
    // 音效系統初始化
    // 注意：音檔需要放在 assets/audio/ 目錄下
    // 目前暫時不載入 BGM，等音檔準備好後再啟用
    // 可載入的檔案：
    //   - audio/bgm_day.ogg   (日間背景音樂)
    //   - audio/bgm_night.ogg (夜間背景音樂)

    commands.insert_resource(AudioManager {
        day_bgm: None,
        night_bgm: None,
        ..default()
    });

    info!("🔊 音效系統已初始化（BGM 待加入）");
}

/// 更新背景音樂（日夜切換）
pub fn update_background_music(
    world_time: Res<WorldTime>,
    mut audio_manager: ResMut<AudioManager>,
    mut commands: Commands,
    audio_query: Query<Entity, With<BackgroundMusic>>,
) {
    let hour = world_time.hour;
    let is_night = !(6.0..=18.0).contains(&hour);

    // 只在日夜切換時更新
    if audio_manager.is_night != is_night {
        audio_manager.is_night = is_night;

        // 停止當前 BGM
        for entity in &audio_query {
            commands.entity(entity).despawn();
        }

        // 播放新 BGM
        let bgm_handle = if is_night {
            audio_manager.night_bgm.clone()
        } else {
            audio_manager.day_bgm.clone()
        };

        if let Some(handle) = bgm_handle {
            let volume = audio_manager.master_volume * audio_manager.music_volume;
            commands.spawn((
                AudioPlayer::<AudioSource>(handle),
                PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: bevy::audio::Volume::Linear(volume),
                    ..default()
                },
                BackgroundMusic,
            ));
        }
    }
}

/// 背景音樂標記組件
#[derive(Component)]
pub struct BackgroundMusic;

/// 更新引擎音效
/// 根據載具速度調整音高和音量
pub fn update_engine_sounds(
    audio_manager: Res<AudioManager>,
    mut vehicle_query: Query<(&Vehicle, &EngineSound, &mut AudioSink), With<Vehicle>>,
) {
    for (vehicle, engine, mut sink) in &mut vehicle_query {
        // 計算速度比例 (0.0 ~ 1.0)
        let speed_ratio = (vehicle.current_speed / vehicle.max_speed).clamp(0.0, 1.0);

        // 計算音高：怠速 -> 全速
        let pitch = engine.base_pitch + (engine.max_pitch - engine.base_pitch) * speed_ratio;

        // 計算音量：怠速較小聲，加速時變大聲
        let volume = engine.base_volume * (0.3 + 0.7 * speed_ratio);
        let final_volume = volume * audio_manager.master_volume * audio_manager.sfx_volume;

        // 更新音效參數
        sink.set_speed(pitch);
        sink.set_volume(bevy::audio::Volume::Linear(final_volume));
    }
}

/// 更新環境音效（3D 空間音效）
/// 根據玩家距離調整音量
pub fn update_ambient_sounds(
    audio_manager: Res<AudioManager>,
    player_query: Query<&Transform, With<Player>>,
    mut ambient_query: Query<(&Transform, &AmbientSound, &mut AudioSink)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation;

    for (transform, ambient, mut sink) in &mut ambient_query {
        let distance = player_pos.distance(transform.translation);

        // 計算音量衰減
        let volume = if distance > ambient.range {
            0.0
        } else {
            // 線性衰減
            let attenuation = 1.0 - (distance / ambient.range);
            ambient.volume * attenuation * audio_manager.master_volume * audio_manager.sfx_volume
        };

        sink.set_volume(bevy::audio::Volume::Linear(volume));
    }
}

/// 播放一次性音效（非空間）
pub fn play_sound_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    audio_manager: &AudioManager,
    sound_path: &'static str, // Bevy 0.17: 需要 'static 生命週期
) {
    let sound = asset_server.load(sound_path);
    let volume = audio_manager.master_volume * audio_manager.sfx_volume;

    commands.spawn((
        AudioPlayer::<AudioSource>(sound),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Despawn,
            volume: bevy::audio::Volume::Linear(volume),
            ..default()
        },
    ));
}

/// 生成機車引擎音效
pub fn spawn_scooter_engine_sound(
    commands: &mut Commands,
    asset_server: &AssetServer,
    scooter_entity: Entity,
) {
    let engine_sound = asset_server.load("audio/engine_scooter.ogg");

    commands.entity(scooter_entity).with_children(|parent| {
        parent.spawn((
            AudioPlayer::<AudioSource>(engine_sound),
            PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: bevy::audio::Volume::Linear(0.0), // 初始靜音，由 update_engine_sounds 控制
                paused: true,
                ..default()
            },
            EngineSound::scooter(),
        ));
    });
}

/// 生成環境音效點
pub fn spawn_ambient_sound_point(
    commands: &mut Commands,
    asset_server: &AssetServer,
    position: Vec3,
    ambient_config: AmbientSound,
    sound_path: &'static str, // Bevy 0.17: 需要 'static 生命週期
) {
    let sound = asset_server.load(sound_path);

    commands.spawn((
        Transform::from_translation(position),
        GlobalTransform::default(),
        AudioPlayer::<AudioSource>(sound),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Loop,
            volume: bevy::audio::Volume::Linear(0.0), // 初始靜音，由距離控制
            ..default()
        },
        ambient_config,
        Name::new("AmbientSound"),
    ));
}

// ============================================================================
// 音效播放共用輔助函數
// ============================================================================

/// 播放一次性音效（共用邏輯）
/// 消除重複的 `if let Some(handle) = ... { commands.spawn(...) }` 模式
#[inline]
fn spawn_one_shot_sound(
    commands: &mut Commands,
    handle: Option<Handle<AudioSource>>,
    volume: f32,
) {
    let Some(handle) = handle else { return };
    commands.spawn((
        AudioPlayer::<AudioSource>(handle),
        PlaybackSettings {
            mode: bevy::audio::PlaybackMode::Despawn,
            volume: bevy::audio::Volume::Linear(volume),
            ..default()
        },
    ));
}

/// 計算音效音量
#[inline]
fn calculate_sfx_volume(audio_manager: &AudioManager, multiplier: f32) -> f32 {
    (audio_manager.master_volume * audio_manager.sfx_volume * multiplier).min(1.0)
}

// ============================================================================
// 武器音效系統
// ============================================================================

/// 初始化武器音效資源
pub fn setup_weapon_sounds(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
) {
    // 武器音效系統（音檔待添加）
    // 當音檔準備好後，取消註解下方程式碼並移除 None
    //
    // 所需音檔 (assets/audio/):
    //   gun_pistol.ogg, gun_smg.ogg, gun_shotgun.ogg, gun_rifle.ogg
    //   reload_start.ogg, reload_finish.ogg, weapon_switch.ogg
    //   hit_flesh.ogg, hit_metal.ogg, headshot.ogg
    //   punch_whoosh.ogg, punch_hit.ogg, empty_clip.ogg
    //
    // 來源建議: freesound.org (CC0 授權)

    let weapon_sounds = WeaponSounds {
        pistol_fire: None,
        smg_fire: None,
        shotgun_fire: None,
        rifle_fire: None,
        empty_clip: None,
        reload_start: None,
        reload_finish: None,
        weapon_switch: None,
        hit_flesh: None,
        hit_metal: None,
        headshot: None,
        punch_whoosh: None,
        punch_hit: None,
    };

    commands.insert_resource(weapon_sounds);
    info!("🔫 武器音效系統已初始化（音檔待添加）");
}

/// 播放武器槍聲
/// 根據武器類型選擇對應的槍聲
pub fn play_weapon_fire_sound(
    commands: &mut Commands,
    weapon_sounds: &WeaponSounds,
    audio_manager: &AudioManager,
    weapon_type: WeaponType,
) {
    let sound_handle = match weapon_type {
        WeaponType::Pistol => weapon_sounds.pistol_fire.clone(),
        WeaponType::SMG => weapon_sounds.smg_fire.clone(),
        WeaponType::Shotgun => weapon_sounds.shotgun_fire.clone(),
        WeaponType::Rifle => weapon_sounds.rifle_fire.clone(),
        // 近戰武器使用揮擊音效
        WeaponType::Fist | WeaponType::Staff | WeaponType::Knife => weapon_sounds.punch_whoosh.clone(),
    };

    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, sound_handle, volume);
}

/// 播放換彈音效
pub fn play_reload_sound(
    commands: &mut Commands,
    weapon_sounds: &WeaponSounds,
    audio_manager: &AudioManager,
    is_finish: bool,
) {
    let sound_handle = if is_finish {
        weapon_sounds.reload_finish.clone()
    } else {
        weapon_sounds.reload_start.clone()
    };

    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, sound_handle, volume);
}

/// 播放命中音效
pub fn play_hit_sound(
    commands: &mut Commands,
    weapon_sounds: &WeaponSounds,
    audio_manager: &AudioManager,
    is_headshot: bool,
) {
    // 爆頭有專屬音效
    let sound_handle = if is_headshot {
        weapon_sounds.headshot.clone()
    } else {
        weapon_sounds.hit_flesh.clone()
    };

    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, sound_handle, volume);
}

/// 播放武器切換音效
pub fn play_weapon_switch_sound(
    commands: &mut Commands,
    weapon_sounds: &WeaponSounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 0.7); // 切換聲稍微小聲
    spawn_one_shot_sound(commands, weapon_sounds.weapon_switch.clone(), volume);
}

/// 播放空彈匣音效
pub fn play_empty_clip_sound(
    commands: &mut Commands,
    weapon_sounds: &WeaponSounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, weapon_sounds.empty_clip.clone(), volume);
}

// ============================================================================
// 車輛音效系統 (GTA 5 風格)
// ============================================================================

use super::{VehicleSounds, PlayerSounds, UISounds};

/// 初始化車輛音效資源
pub fn setup_vehicle_sounds(mut commands: Commands) {
    // 車輛音效（音檔待添加）
    // 所需音檔 (assets/audio/):
    //   door_open.ogg, door_close.ogg
    //   engine_start.ogg, engine_stop.ogg
    //   horn.ogg, tire_screech.ogg
    //   collision_light.ogg, collision_heavy.ogg
    //   explosion.ogg
    commands.insert_resource(VehicleSounds::default());
    info!("🚗 車輛音效系統已初始化（音檔待添加）");
}

/// 初始化玩家音效資源
pub fn setup_player_sounds(mut commands: Commands) {
    // 玩家音效（音檔待添加）
    // 所需音檔 (assets/audio/):
    //   footstep_concrete.ogg, footstep_grass.ogg, footstep_metal.ogg
    //   jump.ogg, land.ogg, dodge_roll.ogg
    //   hurt.ogg, death.ogg
    commands.insert_resource(PlayerSounds::default());
    info!("🏃 玩家音效系統已初始化（音檔待添加）");
}

/// 初始化 UI 音效資源
pub fn setup_ui_sounds(mut commands: Commands) {
    // UI 音效（音檔待添加）
    // 所需音檔（assets/audio/）：
    //   mission_start.ogg、mission_complete.ogg、mission_fail.ogg
    //   checkpoint.ogg、money_gain.ogg
    //   wanted_up.ogg、wanted_clear.ogg
    commands.insert_resource(UISounds::default());
    info!("🎮 UI 音效系統已初始化（音檔待添加）");
}

/// 播放車門開啟音效
pub fn play_door_open_sound(
    commands: &mut Commands,
    vehicle_sounds: &VehicleSounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 0.8);
    spawn_one_shot_sound(commands, vehicle_sounds.door_open.clone(), volume);
}

/// 播放車門關閉音效
pub fn play_door_close_sound(
    commands: &mut Commands,
    vehicle_sounds: &VehicleSounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 0.9);
    spawn_one_shot_sound(commands, vehicle_sounds.door_close.clone(), volume);
}

/// 播放引擎發動音效
pub fn play_engine_start_sound(
    commands: &mut Commands,
    vehicle_sounds: &VehicleSounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, vehicle_sounds.engine_start.clone(), volume);
}

/// 播放輪胎打滑音效
pub fn play_tire_screech_sound(
    commands: &mut Commands,
    vehicle_sounds: &VehicleSounds,
    audio_manager: &AudioManager,
    intensity: f32, // 0.0 ~ 1.0
) {
    let volume = calculate_sfx_volume(audio_manager, intensity * 0.7);
    spawn_one_shot_sound(commands, vehicle_sounds.tire_screech.clone(), volume);
}

/// 播放碰撞音效
pub fn play_collision_sound(
    commands: &mut Commands,
    vehicle_sounds: &VehicleSounds,
    audio_manager: &AudioManager,
    is_heavy: bool,
) {
    let handle = if is_heavy {
        vehicle_sounds.collision_heavy.clone()
    } else {
        vehicle_sounds.collision_light.clone()
    };

    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, handle, volume);
}

/// 播放爆炸音效
pub fn play_explosion_sound(
    commands: &mut Commands,
    vehicle_sounds: &VehicleSounds,
    audio_manager: &AudioManager,
) {
    // 爆炸聲更大 (1.2x)，calculate_sfx_volume 已經會 .min(1.0)
    let volume = calculate_sfx_volume(audio_manager, 1.2);
    spawn_one_shot_sound(commands, vehicle_sounds.explosion.clone(), volume);
}

// ============================================================================
// 玩家音效播放
// ============================================================================

/// 播放腳步聲
pub fn play_footstep_sound(
    commands: &mut Commands,
    player_sounds: &PlayerSounds,
    audio_manager: &AudioManager,
    surface: FootstepSurface,
) {
    let handle = match surface {
        FootstepSurface::Concrete => player_sounds.footstep_concrete.clone(),
        FootstepSurface::Grass => player_sounds.footstep_grass.clone(),
        FootstepSurface::Metal => player_sounds.footstep_metal.clone(),
    };

    let volume = calculate_sfx_volume(audio_manager, 0.4);
    spawn_one_shot_sound(commands, handle, volume);
}

/// 腳步表面類型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootstepSurface {
    Concrete,
    Grass,
    Metal,
}

/// 播放受傷音效
pub fn play_hurt_sound(
    commands: &mut Commands,
    player_sounds: &PlayerSounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 0.8);
    spawn_one_shot_sound(commands, player_sounds.hurt.clone(), volume);
}

// ============================================================================
// UI 音效播放
// ============================================================================

/// 播放任務開始音效
pub fn play_mission_start_sound(
    commands: &mut Commands,
    ui_sounds: &UISounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, ui_sounds.mission_start.clone(), volume);
}

/// 播放任務完成音效
pub fn play_mission_complete_sound(
    commands: &mut Commands,
    ui_sounds: &UISounds,
    audio_manager: &AudioManager,
) {
    // 任務完成音效稍大聲 (1.2x)
    let volume = calculate_sfx_volume(audio_manager, 1.2);
    spawn_one_shot_sound(commands, ui_sounds.mission_complete.clone(), volume);
}

/// 播放檢查點音效
pub fn play_checkpoint_sound(
    commands: &mut Commands,
    ui_sounds: &UISounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, ui_sounds.checkpoint.clone(), volume);
}

/// 播放通緝星級增加音效
pub fn play_wanted_up_sound(
    commands: &mut Commands,
    ui_sounds: &UISounds,
    audio_manager: &AudioManager,
) {
    let volume = calculate_sfx_volume(audio_manager, 1.0);
    spawn_one_shot_sound(commands, ui_sounds.wanted_up.clone(), volume);
}
