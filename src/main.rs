//! # 🏝️ 島嶼狂飆 Island Rampage
//!
//! 一款以台灣為舞台的 3D 開放世界動作冒險遊戲

// === 模組 ===
mod ai;
mod audio;
mod camera;
mod combat;
mod core;
mod environment;
mod mission;
mod pedestrian;
mod player;
mod ui;
mod vehicle;
mod wanted;
mod world;

use bevy::prelude::*;
// MonitorSelection 已移除：BorderlessFullscreen 在 macOS 26 有 bug
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        // === 插件 ===
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "🏝️ 島嶼狂飆 Island Rampage".into(),
                resolution: (1920u32, 1080u32).into(),  // 1080p (Bevy 0.17: u32)
                mode: bevy::window::WindowMode::Windowed,  // 視窗模式（BorderlessFullscreen 在 macOS 26 有 bug）
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        
        // === 遊戲事件 ===
        .add_message::<core::GameEvent>()
        
        // === 戰鬥插件 ===
        .add_plugins(combat::CombatPlugin)

        // === AI 插件 ===
        .add_plugins(ai::AiPlugin)

        // === 行人插件 ===
        .add_plugins(pedestrian::PedestrianPlugin)

        // === 通緝系統插件 ===
        .add_plugins(wanted::WantedPlugin)

        // === 環境互動插件 ===
        .add_plugins(environment::EnvironmentPlugin)

        // === 劇情任務插件 ===
        .add_plugins(mission::DialogueSystemPlugin)
        .add_plugins(mission::DialogueUIPlugin)
        .add_plugins(mission::CutsceneSystemPlugin)
        .add_plugins(mission::StoryMissionPlugin)

        // === 資源 ===
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.15)))
        .insert_resource(core::GameState::default())
        .insert_resource(core::WorldTime::default())
        .insert_resource(core::WeatherState::default())  // 天氣系統
        .insert_resource(core::CameraSettings::default())
        .insert_resource(core::PlayerStats::default())
        .insert_resource(mission::MissionManager::default())
        .insert_resource(ui::UiState::default())
        .insert_resource(ui::NotificationQueue::default())
        .insert_resource(audio::AudioManager::default())
        .init_resource::<world::WindowUpdateTimer>()  // 窗戶更新計時器
        .init_resource::<player::DoubleTapTracker>()  // 閃避雙擊偵測
        .init_resource::<player::VehicleTransitionState>()  // 車輛進出動畫狀態
        .init_resource::<core::RecoilState>()         // 後座力狀態
        .init_resource::<core::CameraShake>()         // 攝影機震動
        .init_resource::<ui::DamageIndicatorState>()  // 受傷指示器狀態
        .init_resource::<ui::HudAnimationState>()    // HUD 動畫狀態
        .init_resource::<ui::CrosshairDynamics>()    // 準星動態狀態
        .init_resource::<ui::WeaponSwitchAnimation>() // 武器切換動畫狀態
        .init_resource::<ui::FloatingDamageTracker>() // 浮動傷害數字追蹤器
        .init_resource::<ui::WeaponWheelState>()     // 武器輪盤狀態
        .init_resource::<ui::GpsNavigationState>()   // GPS 導航狀態
        .init_resource::<world::LightningState>()    // 閃電狀態

        // === 啟動系統 ===
        .add_systems(Startup, (
            setup_chinese_font,
            world::setup_world,
            mission::spawn_mission_markers,
            audio::setup_audio,
            audio::setup_weapon_sounds,   // 武器音效初始化
            audio::setup_vehicle_sounds,  // 車輛音效初始化
            audio::setup_player_sounds,   // 玩家音效初始化
            audio::setup_ui_sounds,       // UI 音效初始化
        ))
        // NPC 交通需要在 setup_world 之後運行（需要 VehicleMaterials 資源）
        .add_systems(Startup, vehicle::spawn_initial_traffic.after(world::setup_world))
        // 車輛視覺效果初始化
        .add_systems(Startup, vehicle::setup_vehicle_effects)
        // 車輛損壞系統初始化
        .add_systems(Startup, vehicle::setup_vehicle_damage_effects)
        // 紅綠燈系統初始化
        .add_systems(Startup, vehicle::setup_traffic_lights)
        .add_systems(Startup, vehicle::spawn_world_traffic_lights.after(vehicle::setup_traffic_lights))
        .add_systems(Startup, (
            ui::setup_ui,
            ui::setup_delivery_app,
            ui::setup_notification_ui,
            ui::setup_crosshair,
            ui::setup_damage_indicator,
            ui::setup_weather_hud,    // 天氣 HUD
            ui::setup_weapon_wheel,   // 武器輪盤 UI
            ui::setup_interaction_prompt, // 互動提示 UI
            ui::setup_gps_ui,         // GPS 導航 UI
            ui::setup_story_mission_hud, // 劇情任務 HUD
        ).after(setup_chinese_font))
        
        // === 更新系統（分組避免 tuple 限制）===
        // 核心和 UI 第一組
        .add_systems(Update, (
            core::handle_game_events,
            ui::toggle_pause,
            ui::button_hover_effect,       // 按鈕懸停效果
            ui::toggle_map,
            ui::toggle_delivery_app,      // 外送 App 開關
            ui::update_delivery_app,       // 外送 App 更新
            ui::update_ui,
            ui::update_hud,
            ui::update_mission_ui,
            ui::update_minimap,
            ui::minimap_zoom_control,
            ui::update_fullmap,
        ))
        // UI 第二組（地圖標籤、戰鬥 UI）
        .add_systems(Update, (
            ui::setup_world_name_tags,
            ui::update_world_name_tags,
            ui::update_notifications,
            ui::update_crosshair,          // 準星更新
            ui::update_hit_marker,         // 命中標記更新
            ui::update_ammo_display,       // 彈藥顯示更新
            ui::update_ammo_visual_grid,   // 彈藥視覺化網格更新
            ui::update_weapon_switch_animation.after(ui::update_ammo_display), // 武器切換動畫（需在彈藥顯示後執行避免 Query 衝突）
            ui::setup_enemy_health_bars,   // 敵人血條生成
            ui::update_enemy_health_bars,  // 敵人血條更新
            ui::cleanup_enemy_health_bars, // 敵人血條清理
            ui::update_damage_indicator,   // 受傷指示器更新
            ui::update_hud_animations,     // HUD 動畫（低血量脈衝、小地圖掃描）
            ui::update_crosshair_dynamics, // 準星動態效果
        ))
        // 武器輪盤系統
        .add_systems(Update, (
            ui::weapon_wheel_input_system,
            ui::weapon_wheel_update_system,
            ui::weapon_wheel_icon_update_system,
        ))
        // 互動提示系統
        .add_systems(Update, (
            ui::update_interaction_prompt_state,
            ui::update_interaction_prompt_ui,
        ))
        // GPS 導航系統
        .add_systems(Update, (
            ui::update_gps_navigation,
            ui::update_minimap_gps_marker,
            ui::gps_mission_integration,
        ))
        // 劇情任務 HUD 系統
        .add_systems(Update, ui::update_story_mission_hud)
        // 玩家系統（明確執行順序）
        .add_systems(Update, (
            player::player_input,
            player::dodge_detection_system,                      // 閃避偵測
            player::dodge_state_update_system,                   // 閃避狀態更新
            player::player_movement.after(player::dodge_state_update_system),
            player::dodge_movement_system.after(player::dodge_state_update_system),  // 閃避移動
            player::player_jump.after(player::player_movement),  // 跳躍在移動後
            player::enter_exit_vehicle,
            player::vehicle_transition_animation_system.after(player::enter_exit_vehicle),  // 車輛進出動畫
        ))
        // 載具和 NPC（暫停時跳過）
        .add_systems(Update, (
            vehicle::vehicle_input,
            vehicle::vehicle_movement,
            vehicle::npc_vehicle_ai,
        ).run_if(|ui: Res<ui::UiState>| !ui.paused))
        // 車輛視覺效果（漂移煙霧、輪胎痕跡）（暫停時跳過）
        .add_systems(Update, (
            vehicle::drift_smoke_spawn_system,
            vehicle::drift_smoke_update_system,
            vehicle::tire_track_spawn_system,
            vehicle::tire_track_update_system,
        ).run_if(|ui: Res<ui::UiState>| !ui.paused))
        // 車輛損壞系統（暫停時跳過）
        .add_systems(Update, (
            vehicle::vehicle_collision_damage_system,
            vehicle::vehicle_fire_system,
            vehicle::vehicle_damage_effect_system,
            vehicle::vehicle_explosion_system,
            vehicle::vehicle_damage_particle_update_system,
            vehicle::vehicle_damage_event_system,  // 處理子彈/爆炸傷害
        ).run_if(|ui: Res<ui::UiState>| !ui.paused))
        // 紅綠燈系統（不受暫停影響，純視覺）
        .add_systems(Update, vehicle::traffic_light_cycle_system)
        // 攝影機、任務、世界（攝影機在移動後）
        .add_systems(Update, (
            camera::camera_input,
            camera::camera_auto_follow.after(camera::camera_input).after(player::player_movement),
            camera::camera_follow.after(camera::camera_auto_follow).after(vehicle::vehicle_movement),
            camera::recoil_and_shake_update_system,  // 後座力和攝影機震動更新
            mission::mission_system,
            mission::mission_marker_animation,
            world::update_world_time,
            world::update_lighting,
            world::update_neon_signs,
            world::update_building_windows,
        ))
        // 天氣系統 - 輸入和視覺不受暫停影響
        .add_systems(Update, (
            world::weather_input_system,         // F1 切換天氣（暫停時也可切換）
            world::update_sky_color,             // 天空顏色
            world::update_fog_effect,            // 霧效果
            ui::update_weather_hud,              // 天氣 HUD 更新
        ))
        // 天氣系統 - 粒子和動態效果（暫停時跳過）
        .add_systems(Update, (
            world::update_weather_transition,    // 天氣過渡
            world::spawn_rain_drops,             // 雨滴生成
            world::update_rain_drops,            // 雨滴更新
            world::cleanup_rain,                 // 雨滴清理
            world::spawn_rain_puddles,           // 雨水積水生成
            world::update_rain_puddles,          // 雨水積水更新
            world::update_lightning,             // 閃電更新
            world::lightning_visual_effect,      // 閃電視覺效果
        ).run_if(|ui: Res<ui::UiState>| !ui.paused))
        // 室內建築系統（暫停時跳過）
        .add_systems(Update, (
            world::interior_proximity_system,    // 門互動檢測
            world::interior_enter_system,        // 進入/離開室內
            world::interior_hiding_system,       // 室內躲藏效果
            world::door_animation_system,        // 門動畫
        ).run_if(|ui: Res<ui::UiState>| !ui.paused))
        // 音效系統 - 背景音樂不受暫停影響
        .add_systems(Update, audio::update_background_music)
        // 音效系統 - 引擎聲和環境音（暫停時跳過）
        .add_systems(Update, (
            audio::update_engine_sounds,
            audio::update_ambient_sounds,
        ).run_if(|ui: Res<ui::UiState>| !ui.paused))
        .run();
}

/// 載入中文字體
fn setup_chinese_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/STHeiti.ttc");
    commands.insert_resource(ui::ChineseFont { font });
}
