//! # 🏝️ 島嶼狂飆 Island Rampage
//!
//! 一款以台灣為舞台的 3D 開放世界動作冒險遊戲

// Bevy ECS 系統常見的 lint 豁免
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
// 程式碼風格 lint 豁免（保持可讀性優先）
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::wildcard_in_or_patterns)]
#![allow(clippy::never_loop)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::op_ref)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::needless_borrow)]

// === 模組 ===
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

fn main() {
    App::new()
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
        .init_resource::<ui::DamageIndicatorState>() // 受傷指示器狀態
        .init_resource::<ui::HudAnimationState>() // HUD 動畫狀態
        .init_resource::<ui::CrosshairDynamics>() // 準星動態狀態
        .init_resource::<ui::WeaponSwitchAnimation>() // 武器切換動畫狀態
        .init_resource::<ui::FloatingDamageTracker>() // 浮動傷害數字追蹤器
        .init_resource::<ui::WeaponWheelState>() // 武器輪盤狀態
        .init_resource::<ui::GpsNavigationState>() // GPS 導航狀態
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
                camera::recoil_and_shake_update_system,
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
            (audio::update_engine_sounds, audio::update_ambient_sounds)
                .run_if(in_state(core::AppState::InGame)),
        )
        .run();
}
