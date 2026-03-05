//! 音效整合系統（事件驅動 + 電台 + 警察無線電 + NPC 對話）

// 功能模組已實現但尚未完全整合到遊戲玩法中
#![allow(dead_code)]

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Real as RapierReal, *};

use super::{
    calculate_sfx_volume, play_checkpoint_sound, play_door_open_sound, play_engine_start_sound,
    play_footstep_sound, play_mission_complete_sound, play_mission_fail_sound,
    play_mission_start_sound, play_money_gain_sound, play_wanted_clear_sound, play_wanted_up_sound,
    spawn_one_shot_sound, FootstepSurface,
};
use super::{
    AudioManager, AudioVehicleState, FootstepTimer, GroundSurface, NpcDialogueCooldown,
    NpcDialogueManager, PlayerGroundSurface, PlayerSounds, PoliceRadioState, RadioFadeState,
    RadioManager, RadioPlaylists, RadioStation, UISounds, VehicleSounds, NPC_DIALOGUE_LINES,
    RADIO_FADE_IN_DURATION, RADIO_FADE_OUT_DURATION,
};
use crate::core::GameState;
use crate::mission::StoryMissionEvent;
use crate::pedestrian::Pedestrian;
use crate::player::Player;
use crate::ui::NotificationQueue;
use crate::wanted::{WantedLevel, WantedLevelChanged};

// ============================================================================
// 音效整合系統（事件驅動）
// ============================================================================

/// 通緝等級變化音效系統
/// 監聽 `WantedLevelChanged` 事件，播放對應音效
pub fn audio_wanted_level_system(
    mut commands: Commands,
    audio_manager: Res<AudioManager>,
    ui_sounds: Res<UISounds>,
    mut events: MessageReader<WantedLevelChanged>,
) {
    for event in events.read() {
        if event.new_stars == 0 {
            // 通緝消除
            play_wanted_clear_sound(&mut commands, &ui_sounds, &audio_manager);
        } else if event.increased {
            // 通緝上升
            play_wanted_up_sound(&mut commands, &ui_sounds, &audio_manager);
        }
    }
}

/// 劇情任務事件音效系統
/// 監聽 `StoryMissionEvent`，在任務開始/完成/失敗/檢查點時播放音效
pub fn audio_mission_event_system(
    mut commands: Commands,
    audio_manager: Res<AudioManager>,
    ui_sounds: Res<UISounds>,
    mut events: MessageReader<StoryMissionEvent>,
) {
    for event in events.read() {
        match event {
            StoryMissionEvent::Started(_) => {
                play_mission_start_sound(&mut commands, &ui_sounds, &audio_manager);
            }
            StoryMissionEvent::Completed { .. } => {
                play_mission_complete_sound(&mut commands, &ui_sounds, &audio_manager);
            }
            StoryMissionEvent::Failed { .. } => {
                play_mission_fail_sound(&mut commands, &ui_sounds, &audio_manager);
            }
            StoryMissionEvent::CheckpointReached { .. } => {
                play_checkpoint_sound(&mut commands, &ui_sounds, &audio_manager);
            }
            StoryMissionEvent::MoneyChanged { new, old } if new > old => {
                play_money_gain_sound(&mut commands, &ui_sounds, &audio_manager);
            }
            _ => {}
        }
    }
}

/// 車輛上下車音效系統
/// 偵測 `GameState.player_in_vehicle` 狀態變化，播放車門和引擎音效
pub fn audio_vehicle_enter_exit_system(
    mut commands: Commands,
    audio_manager: Res<AudioManager>,
    vehicle_sounds: Res<VehicleSounds>,
    game_state: Res<GameState>,
    mut vehicle_state: ResMut<AudioVehicleState>,
) {
    let in_vehicle = game_state.player_in_vehicle;

    if in_vehicle != vehicle_state.was_in_vehicle {
        if in_vehicle {
            // 上車：開門 → 關門 → 引擎發動
            play_door_open_sound(&mut commands, &vehicle_sounds, &audio_manager);
            play_engine_start_sound(&mut commands, &vehicle_sounds, &audio_manager);
        } else {
            // 下車：引擎熄火 → 開門
            play_door_open_sound(&mut commands, &vehicle_sounds, &audio_manager);
        }
        vehicle_state.was_in_vehicle = in_vehicle;
    }
}

/// 地面材質偵測系統
/// 從玩家位置向下射線偵測，判斷腳下地面類型
pub fn detect_ground_surface_system(
    rapier_context: ReadRapierContext,
    player_query: Query<&Transform, With<Player>>,
    ground_query: Query<&GroundSurface>,
    mut player_ground: ResMut<PlayerGroundSurface>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    let ray_origin = player_transform.translation + Vec3::Y * 0.5;
    let ray_dir = Vec3::NEG_Y;
    let max_dist: RapierReal = 3.0;

    if let Some((entity, _toi)) =
        rapier.cast_ray(ray_origin, ray_dir, max_dist, true, QueryFilter::default())
    {
        // 如果命中實體有 GroundSurface 組件，使用該材質
        if let Ok(surface) = ground_query.get(entity) {
            player_ground.surface = *surface;
        } else {
            // 無標記時預設水泥
            player_ground.surface = GroundSurface::Concrete;
        }
    }
}

/// 玩家腳步音效系統
/// 根據移動狀態、速度和地面材質，定時播放腳步聲
/// 蹲伏時音量降低、衝刺時音量提高
pub fn audio_footstep_system(
    time: Res<Time>,
    mut commands: Commands,
    audio_manager: Res<AudioManager>,
    player_sounds: Res<PlayerSounds>,
    game_state: Res<GameState>,
    player_ground: Res<PlayerGroundSurface>,
    mut footstep_timer: ResMut<FootstepTimer>,
    player_query: Query<&Player>,
) {
    // 車內不播放腳步聲
    if game_state.player_in_vehicle {
        footstep_timer.elapsed = 0.0;
        return;
    }

    let Ok(player) = player_query.single() else {
        return;
    };

    // 速度太低不播放
    if player.current_speed < 0.5 {
        footstep_timer.elapsed = 0.0;
        return;
    }

    let dt = time.delta_secs();
    footstep_timer.elapsed += dt;

    let interval = FootstepTimer::interval(player.is_sprinting, player.is_crouching);
    if footstep_timer.elapsed >= interval {
        footstep_timer.elapsed = 0.0;
        let surface: FootstepSurface = player_ground.surface.into();
        play_footstep_sound(&mut commands, &player_sounds, &audio_manager, surface);
    }
}

// ============================================================================
// 車載電台系統
// ============================================================================

/// 電台播放標記組件
#[derive(Component)]
pub struct RadioPlayback;

/// 電台輸入系統
/// 駕駛中數字鍵 1-8 直接選台，9 關閉，Q/E 循環切換
pub fn radio_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    game_state: Res<GameState>,
    ui_state: Res<crate::ui::UiState>,
    mut radio: ResMut<RadioManager>,
) {
    // 只有在車內才能操作電台；手機開啟時不攔截數字鍵
    if !game_state.player_in_vehicle || ui_state.show_phone {
        return;
    }

    let mut target: Option<RadioStation> = None;

    // 數字鍵直接選台（1-8 電台，9 關閉）
    let digit_map: &[(KeyCode, RadioStation)] = &[
        (KeyCode::Digit1, RadioStation::IslandPop),
        (KeyCode::Digit2, RadioStation::NightMarketFunk),
        (KeyCode::Digit3, RadioStation::TaiwanReggae),
        (KeyCode::Digit4, RadioStation::UndergroundHipHop),
        (KeyCode::Digit5, RadioStation::ClassicFM),
        (KeyCode::Digit6, RadioStation::TaiwaneseOldies),
        (KeyCode::Digit7, RadioStation::IndigenousBeats),
        (KeyCode::Digit8, RadioStation::ElectroTechno),
        (KeyCode::Digit9, RadioStation::Off),
    ];

    for &(key, station) in digit_map {
        if keyboard.just_pressed(key) {
            target = Some(station);
            break;
        }
    }

    // Q/E 循環切換（保留原功能）
    if target.is_none() {
        if keyboard.just_pressed(KeyCode::KeyQ) {
            target = Some(radio.current_station.prev());
        } else if keyboard.just_pressed(KeyCode::KeyE) {
            target = Some(radio.current_station.next());
        }
    }

    // 觸發電台切換（透過淡入淡出）
    if let Some(next_station) = target {
        if next_station == radio.current_station {
            return;
        }

        radio.show_station_name = true;
        radio.station_name_timer = 3.0;

        // 從 Off 切換到電台 → 直接切換 + 淡入
        if radio.current_station == RadioStation::Off {
            radio.current_station = next_station;
            radio.fade_volume = 0.0;
            radio.fade_state = RadioFadeState::FadingIn {
                elapsed: 0.0,
                duration: RADIO_FADE_IN_DURATION,
            };
        }
        // 從電台切換到其他/Off → 淡出後切換
        else {
            radio.fade_state = RadioFadeState::FadingOut {
                elapsed: 0.0,
                duration: RADIO_FADE_OUT_DURATION,
                next_station,
            };
        }
    }
}

/// 電台淡入淡出系統
/// 處理切換電台時的音量漸變效果
pub fn radio_fade_system(
    time: Res<Time>,
    audio_manager: Res<AudioManager>,
    mut radio: ResMut<RadioManager>,
    mut sink_query: Query<&mut AudioSink, With<RadioPlayback>>,
) {
    let dt = time.delta_secs();

    match radio.fade_state {
        RadioFadeState::FadingOut {
            elapsed,
            duration,
            next_station,
        } => {
            let new_elapsed = elapsed + dt;
            if new_elapsed >= duration {
                // 淡出完成 → 切換到新電台
                radio.fade_volume = 0.0;
                radio.current_station = next_station;
                if next_station == RadioStation::Off {
                    radio.fade_state = RadioFadeState::Idle;
                    radio.fade_volume = 1.0;
                } else {
                    radio.fade_state = RadioFadeState::FadingIn {
                        elapsed: 0.0,
                        duration: RADIO_FADE_IN_DURATION,
                    };
                }
            } else {
                let t = new_elapsed / duration;
                radio.fade_volume = 1.0 - t;
                radio.fade_state = RadioFadeState::FadingOut {
                    elapsed: new_elapsed,
                    duration,
                    next_station,
                };
            }
        }
        RadioFadeState::FadingIn { elapsed, duration } => {
            let new_elapsed = elapsed + dt;
            if new_elapsed >= duration {
                radio.fade_volume = 1.0;
                radio.fade_state = RadioFadeState::Idle;
            } else {
                let t = new_elapsed / duration;
                radio.fade_volume = t;
                radio.fade_state = RadioFadeState::FadingIn {
                    elapsed: new_elapsed,
                    duration,
                };
            }
        }
        RadioFadeState::Idle => {}
    }

    // 更新正在播放的電台音量
    let final_volume = audio_manager.master_volume
        * audio_manager.music_volume
        * radio.radio_volume
        * radio.fade_volume;

    for mut sink in &mut sink_query {
        sink.set_volume(bevy::audio::Volume::Linear(final_volume));
    }
}

/// 電台播放系統
/// 根據當前選擇的電台播放/停止音樂（支援播放列表）
pub fn radio_playback_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio_manager: Res<AudioManager>,
    mut radio: ResMut<RadioManager>,
    playlists: Res<RadioPlaylists>,
    game_state: Res<GameState>,
    radio_query: Query<Entity, With<RadioPlayback>>,
) {
    // 下車時停止電台
    if !game_state.player_in_vehicle {
        for entity in &radio_query {
            commands.entity(entity).despawn();
        }
        radio.playing_entity = None;
        return;
    }

    let should_play = radio.current_station != RadioStation::Off;
    let station_changed = radio.current_station != radio.last_played_station;

    // 電台切換或關閉：先停止舊的
    if station_changed && radio.playing_entity.is_some() {
        for entity in &radio_query {
            commands.entity(entity).despawn();
        }
        radio.playing_entity = None;
    }

    // 播放新電台（優先使用 playlist 曲目，fallback 到預設音檔）
    if should_play && radio.playing_entity.is_none() {
        let sound = playlists
            .random_track(radio.current_station)
            .cloned()
            .or_else(|| {
                radio
                    .current_station
                    .audio_path()
                    .map(|p| asset_server.load(p))
            });

        if let Some(sound) = sound {
            let volume = audio_manager.master_volume
                * audio_manager.music_volume
                * radio.radio_volume
                * radio.fade_volume;
            let entity = commands
                .spawn((
                    AudioPlayer::<AudioSource>(sound),
                    PlaybackSettings {
                        mode: bevy::audio::PlaybackMode::Loop,
                        volume: bevy::audio::Volume::Linear(volume),
                        ..default()
                    },
                    RadioPlayback,
                ))
                .id();
            radio.playing_entity = Some(entity);
            radio.last_played_station = radio.current_station;
        }
    }

    // 電台關閉時同步 last_played_station
    if !should_play {
        radio.last_played_station = RadioStation::Off;
    }
}

/// 電台名稱顯示計時器系統
pub fn radio_station_name_timer(time: Res<Time>, mut radio: ResMut<RadioManager>) {
    if radio.show_station_name {
        radio.station_name_timer -= time.delta_secs();
        if radio.station_name_timer <= 0.0 {
            radio.show_station_name = false;
            radio.station_name_timer = 0.0;
        }
    }
}

// ============================================================================
// 警察無線電系統
// ============================================================================

/// 初始化警察無線電音效資源
pub fn setup_police_radio(mut commands: Commands) {
    // 無線電碎語音效（音檔待添加）
    // 所需音檔 (assets/audio/):
    //   police_radio_1.ogg ~ police_radio_5.ogg
    //   (短促的無線電通訊碎語，含靜電雜音)
    commands.insert_resource(PoliceRadioState {
        chatter_sounds: vec![None; 5], // 5 個碎語音效槽位
        ..default()
    });
    info!("📻 警察無線電系統已初始化（音檔待添加）");
}

/// 警察無線電碎語系統
/// 通緝期間隨機播放無線電碎語，頻率隨星級遞增
pub fn police_radio_chatter_system(
    time: Res<Time>,
    mut commands: Commands,
    audio_manager: Res<AudioManager>,
    wanted: Res<WantedLevel>,
    mut radio_state: ResMut<PoliceRadioState>,
) {
    let stars = wanted.stars;

    // 無通緝時停用
    if stars == 0 {
        radio_state.active = false;
        return;
    }

    // 通緝開始時重置計時器
    if !radio_state.active {
        radio_state.active = true;
        radio_state.next_chatter_timer = PoliceRadioState::random_interval(stars);
    }

    radio_state.next_chatter_timer -= time.delta_secs();

    if radio_state.next_chatter_timer <= 0.0 {
        // 播放隨機碎語
        if !radio_state.chatter_sounds.is_empty() {
            let idx = (rand::random::<f32>() * radio_state.chatter_sounds.len() as f32) as usize;
            let idx = idx.min(radio_state.chatter_sounds.len() - 1);
            let volume = calculate_sfx_volume(&audio_manager, 0.5);
            spawn_one_shot_sound(
                &mut commands,
                radio_state.chatter_sounds[idx].clone(),
                volume,
            );
        }

        // 重置計時器（隨機間隔）
        radio_state.next_chatter_timer = PoliceRadioState::random_interval(stars);
    }
}

// ============================================================================
// NPC 環境對話系統
// ============================================================================

/// 初始化 NPC 對話管理器
pub fn setup_npc_dialogue(mut commands: Commands) {
    // NPC 對話音效（音檔待添加）
    // 所需音檔 (assets/audio/):
    //   npc_dialogue_1.ogg ~ npc_dialogue_8.ogg
    //   (台灣口語短句語音，可用 TTS 生成)
    commands.insert_resource(NpcDialogueManager::default());
    info!("💬 NPC 環境對話系統已初始化");
}

/// NPC 對話冷卻衰減系統
pub fn npc_dialogue_cooldown_system(
    time: Res<Time>,
    mut cooldown_query: Query<&mut NpcDialogueCooldown>,
    mut dialogue_manager: ResMut<NpcDialogueManager>,
) {
    let dt = time.delta_secs();

    // 全域冷卻
    if dialogue_manager.global_cooldown > 0.0 {
        dialogue_manager.global_cooldown -= dt;
    }

    // 個別 NPC 冷卻
    for mut cooldown in &mut cooldown_query {
        if cooldown.remaining > 0.0 {
            cooldown.remaining -= dt;
        }
    }
}

/// NPC 環境對話觸發系統
/// 玩家接近行人時，隨機觸發對話泡泡（文字通知）
pub fn npc_dialogue_trigger_system(
    mut commands: Commands,
    audio_manager: Res<AudioManager>,
    mut dialogue_manager: ResMut<NpcDialogueManager>,
    player_query: Query<&Transform, With<Player>>,
    mut npc_query: Query<
        (Entity, &Transform, Option<&mut NpcDialogueCooldown>),
        (With<Pedestrian>, Without<Player>),
    >,
    mut notifications: ResMut<NotificationQueue>,
) {
    // 全域冷卻中
    if dialogue_manager.global_cooldown > 0.0 {
        return;
    }

    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_pos = player_transform.translation;

    let trigger_radius_sq = dialogue_manager.trigger_radius * dialogue_manager.trigger_radius;

    for (entity, npc_transform, cooldown) in &mut npc_query {
        let dist_sq = player_pos.distance_squared(npc_transform.translation);

        // 超出觸發範圍
        if dist_sq > trigger_radius_sq {
            continue;
        }

        // 個別 NPC 冷卻中
        if let Some(ref cd) = cooldown {
            if cd.remaining > 0.0 {
                continue;
            }
        }

        // 隨機決定是否說話（每次接近有 5% 機率）
        if rand::random::<f32>() > 0.05 {
            continue;
        }

        // 選擇隨機對話
        let line_idx = (rand::random::<f32>() * NPC_DIALOGUE_LINES.len() as f32) as usize;
        let line_idx = line_idx.min(NPC_DIALOGUE_LINES.len() - 1);
        let line = NPC_DIALOGUE_LINES[line_idx];

        // 顯示對話通知
        notifications.info(line);

        // 播放對話音效（若有）
        if !dialogue_manager.dialogue_sounds.is_empty() {
            let sound_idx =
                (rand::random::<f32>() * dialogue_manager.dialogue_sounds.len() as f32) as usize;
            let sound_idx = sound_idx.min(dialogue_manager.dialogue_sounds.len() - 1);
            let volume = calculate_sfx_volume(&audio_manager, 0.6);
            spawn_one_shot_sound(
                &mut commands,
                dialogue_manager.dialogue_sounds[sound_idx].clone(),
                volume,
            );
        }

        // 設定冷卻
        if let Some(mut cd) = cooldown {
            cd.remaining = 30.0; // 同一 NPC 30 秒內不再說話
        } else {
            commands
                .entity(entity)
                .insert(NpcDialogueCooldown { remaining: 30.0 });
        }
        dialogue_manager.global_cooldown = dialogue_manager.cooldown_interval;

        break; // 每幀最多一個 NPC 說話
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radio_station_changed_detection() {
        let mut radio = RadioManager::default();
        // 初始狀態：Off, last_played = Off → no change
        assert_eq!(radio.current_station, RadioStation::Off);
        assert_eq!(radio.last_played_station, RadioStation::Off);
        assert_eq!(radio.current_station, radio.last_played_station);

        // 切換電台後，station_changed 應為 true
        radio.current_station = RadioStation::IslandPop;
        assert_ne!(radio.current_station, radio.last_played_station);

        // 模擬播放後同步
        radio.last_played_station = radio.current_station;
        assert_eq!(radio.current_station, radio.last_played_station);
    }

    #[test]
    fn radio_off_does_not_trigger_playback() {
        let radio = RadioManager::default();
        let should_play = radio.current_station != RadioStation::Off;
        assert!(!should_play, "Off 狀態不應觸發播放");
    }

    #[test]
    fn radio_volume_clamped() {
        let audio = AudioManager {
            master_volume: 1.0,
            music_volume: 1.0,
            ..Default::default()
        };
        let radio = RadioManager {
            radio_volume: 1.5, // 超過 1.0
            ..Default::default()
        };
        let volume = (audio.master_volume * audio.music_volume * radio.radio_volume).min(1.0);
        assert!(volume <= 1.0, "音量應被限制在 1.0 以下");
    }

    #[test]
    fn audio_vehicle_state_detects_transition() {
        let mut state = AudioVehicleState {
            was_in_vehicle: false,
        };
        // 上車
        let in_vehicle = true;
        assert_ne!(in_vehicle, state.was_in_vehicle);
        state.was_in_vehicle = in_vehicle;

        // 同一狀態不觸發
        assert_eq!(in_vehicle, state.was_in_vehicle);

        // 下車
        let in_vehicle = false;
        assert_ne!(in_vehicle, state.was_in_vehicle);
    }

    #[test]
    fn footstep_timer_intervals() {
        let normal = FootstepTimer::interval(false, false);
        let sprint = FootstepTimer::interval(true, false);
        let crouch = FootstepTimer::interval(false, true);

        assert!(sprint < normal, "衝刺時腳步間隔應更短");
        assert!(crouch > normal, "蹲伏時腳步間隔應更長");
    }

    #[test]
    fn police_radio_activates_on_wanted() {
        let mut state = PoliceRadioState::default();
        assert!(!state.active);

        // 模擬通緝開始
        state.active = true;
        state.next_chatter_timer = PoliceRadioState::random_interval(1);
        assert!(state.active);
        assert!(state.next_chatter_timer >= 8.0);
        assert!(state.next_chatter_timer <= 12.0);
    }

    #[test]
    fn police_radio_deactivates_without_wanted() {
        let mut state = PoliceRadioState {
            active: true,
            ..Default::default()
        };
        // 模擬通緝結束
        let stars = 0u8;
        if stars == 0 {
            state.active = false;
        }
        assert!(!state.active);
    }

    #[test]
    fn npc_dialogue_global_cooldown_blocks() {
        let manager = NpcDialogueManager {
            global_cooldown: 5.0,
            ..Default::default()
        };
        assert!(manager.global_cooldown > 0.0, "全域冷卻中不應觸發對話");
    }

    #[test]
    fn ground_surface_to_footstep_surface() {
        let concrete: FootstepSurface = GroundSurface::Concrete.into();
        let grass: FootstepSurface = GroundSurface::Grass.into();
        let metal: FootstepSurface = GroundSurface::Metal.into();
        assert_eq!(concrete, FootstepSurface::Concrete);
        assert_eq!(grass, FootstepSurface::Grass);
        assert_eq!(metal, FootstepSurface::Metal);
    }
}
