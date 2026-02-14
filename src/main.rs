//! # 🏝️ 島嶼狂飆 Island Rampage
//!
//! 一款以台灣為舞台的 3D 開放世界動作冒險遊戲

// Bevy ECS 系統常見的 lint 豁免
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
// dead_code 警告已改為逐模組標記，見各模組內的 #[allow(dead_code)]

// ============================================================================
// 模組
// ============================================================================
mod ai;
mod audio;
mod camera;
mod combat;
mod core;
mod economy;
mod environment;
mod mission;
mod pedestrian;
mod player;
mod save;
mod ui;
mod vehicle;
mod wanted;
mod world;

use bevy::prelude::*;
use bevy::window::PresentMode;
// MonitorSelection 已移除：BorderlessFullscreen 在 macOS 26 有 bug
use bevy_rapier3d::prelude::*;

// 開發工具（僅 Debug 模式）
#[cfg(all(debug_assertions, feature = "dev_tools"))]
use bevy_inspector_egui::bevy_egui::EguiPlugin;
#[cfg(all(debug_assertions, feature = "dev_tools"))]
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    let mut app = App::new();

    app
        // === 插件 ===
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "🏝️ 島嶼狂飆 Island Rampage".into(),
                resolution: (1920u32, 1080u32).into(), // 1080p (Bevy 0.17: u32)
                mode: bevy::window::WindowMode::Windowed, // 視窗模式（BorderlessFullscreen 在 macOS 26 有 bug）
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default());

    // === 🎨 開發工具（僅 Debug 模式，Release 自動移除）===
    #[cfg(all(debug_assertions, feature = "dev_tools"))]
    {
        use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

        app.add_plugins(RapierDebugRenderPlugin::default()); // Rapier 碰撞箱可視化
        app.add_plugins(EguiPlugin::default()); // Egui 插件（Inspector 的依賴）
        app.add_plugins(WorldInspectorPlugin::new()); // 即時編輯器
        // Picking 已包含在 DefaultPlugins 中，不需額外加入

        app.add_plugins(FrameTimeDiagnosticsPlugin::default()); // FPS 診斷（Bevy 內建）
        app.add_plugins(LogDiagnosticsPlugin::default()); // 在 console 顯示 FPS
    }

    app
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
        // === 經濟系統插件 ===
        .add_plugins(economy::EconomyPlugin)
        // === 存檔系統插件 ===
        .add_plugins(save::SavePlugin)
        // === 玩家/載具/世界/UI 插件 ===
        .add_plugins(player::PlayerPlugin)
        .add_plugins(vehicle::VehiclePlugin)
        .add_plugins(world::WorldPlugin)
        .add_plugins(ui::UiPlugin)
        // === 狀態 ===
        .init_state::<core::AppState>()
        // === 資源 ===
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.15)))
        .insert_resource(core::GameState::default())
        .insert_resource(core::WorldTime::default())
        .insert_resource(core::WeatherState::default()) // 天氣系統
        .insert_resource(core::CameraSettings::default())
        .insert_resource(core::PlayerStats::default())
        .insert_resource(mission::MissionManager::default())
        .insert_resource(ui::UiState::default())
        .insert_resource(ui::NotificationQueue::default())
        .insert_resource(audio::AudioManager::default())
        .init_resource::<world::WindowUpdateTimer>() // 窗戶更新計時器
        .init_resource::<core::RecoilState>() // 後座力狀態
        .init_resource::<core::CameraShake>() // 攝影機震動
        .init_resource::<core::CinematicState>() // 電影模式狀態
        .init_resource::<ui::DamageIndicatorState>() // 受傷指示器狀態
        .init_resource::<ui::HudAnimationState>() // HUD 動畫狀態
        .init_resource::<ui::CrosshairDynamics>() // 準星動態狀態
        .init_resource::<ui::WeaponSwitchAnimation>() // 武器切換動畫狀態
        .init_resource::<ui::FloatingDamageTracker>() // 浮動傷害數字追蹤器
        .init_resource::<ui::WeaponWheelState>() // 武器輪盤狀態
        .init_resource::<ui::GpsNavigationState>() // GPS 導航狀態
        .init_resource::<audio::FootstepTimer>() // 腳步音效計時器
        .init_resource::<audio::AudioVehicleState>() // 車輛音效狀態追蹤
        .init_resource::<audio::RadioManager>() // 電台管理器
        .init_resource::<audio::PlayerGroundSurface>() // 玩家腳下材質
        .init_resource::<world::LightningState>() // 閃電狀態
        .init_resource::<core::InteractionState>() // 互動輸入狀態 (F)
        // 互動輸入更新（每幀）
        .add_systems(PreUpdate, core::update_interaction_state)
        // 系統群組排序
        .configure_sets(
            Update,
            (
                core::GameSet::Player,
                core::GameSet::Vehicle,
                core::GameSet::World,
                core::GameSet::Ui,
            )
                .chain(),
        )
        // 互動優先序設定
        .configure_sets(
            Update,
            (
                core::InteractionSet::Vehicle,
                core::InteractionSet::Mission,
                core::InteractionSet::Economy,
                core::InteractionSet::Interior,
            )
                .chain(),
        )
        // === 啟動系統 ===
        .add_systems(
            Startup,
            (
                mission::spawn_mission_markers,
                audio::setup_audio,
                audio::setup_weapon_sounds,  // 武器音效初始化
                audio::setup_vehicle_sounds, // 車輛音效初始化
                audio::setup_player_sounds,  // 玩家音效初始化
                audio::setup_ui_sounds,      // UI 音效初始化
                audio::setup_police_radio,   // 警察無線電初始化
                audio::setup_npc_dialogue,   // NPC 對話初始化
                camera::setup_cinematic_letterbox, // 電影模式 Letterbox UI
            ),
        )
        // === 更新系統 ===
        .add_systems(
            Update,
            (
                camera::camera_input,
                camera::camera_auto_follow
                    .after(camera::camera_input)
                    .after(player::player_movement),
                camera::camera_follow
                    .after(camera::camera_auto_follow)
                    .after(vehicle::vehicle_physics_integration_system),
                camera::dynamic_fov_system
                    .after(camera::camera_input),
                camera::recoil_and_shake_update_system,
                camera::cinematic_camera_system,
                camera::cinematic_letterbox_system
                    .after(camera::cinematic_camera_system),
                camera::cinematic_hud_toggle_system
                    .after(camera::cinematic_letterbox_system),
                mission::mission_system.in_set(core::InteractionSet::Mission),
                mission::mission_marker_animation,
            )
                .run_if(in_state(core::AppState::InGame)),
        )
        // 音效系統 - 背景音樂不受暫停影響
        .add_systems(Update, audio::update_background_music)
        // 音效系統 - 引擎聲和環境音（暫停時跳過）
        .add_systems(
            Update,
            (
                audio::auto_attach_engine_sounds,
                audio::update_engine_sounds
                    .after(audio::auto_attach_engine_sounds),
                audio::update_ambient_sounds,
            )
                .run_if(in_state(core::AppState::InGame)),
        )
        // 音效整合系統 — 事件驅動觸發音效
        .add_systems(
            Update,
            (
                audio::audio_wanted_level_system,
                audio::audio_mission_event_system,
                audio::audio_vehicle_enter_exit_system,
                audio::detect_ground_surface_system,
                audio::audio_footstep_system
                    .after(audio::detect_ground_surface_system),
                audio::radio_input_system,
                audio::radio_playback_system
                    .after(audio::radio_input_system),
                audio::radio_station_name_timer,
                audio::police_radio_chatter_system,
                audio::npc_dialogue_cooldown_system,
                audio::npc_dialogue_trigger_system
                    .after(audio::npc_dialogue_cooldown_system),
            )
                .run_if(in_state(core::AppState::InGame)),
        )
        .run();
}
