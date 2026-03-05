//! # 🏝️ 島嶼狂飆 Island Rampage
//!
//! 一款以台灣為舞台的 3D 開放世界動作冒險遊戲

// Lint 政策由 Cargo.toml [lints.clippy] 統一管理

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
        // === 攝影機/音效插件 ===
        .add_plugins(camera::CameraPlugin)
        .add_plugins(audio::AudioPlugin)
        // === 玩家/載具/世界/UI 插件 ===
        .add_plugins(player::PlayerPlugin)
        .add_plugins(vehicle::VehiclePlugin)
        .add_plugins(world::WorldPlugin)
        .add_plugins(ui::UiPlugin)
        // === 狀態 ===
        .init_state::<core::AppState>()
        // === 資源（僅保留全域性資源，其餘由各 Plugin 自行管理）===
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.15)))
        .insert_resource(core::GameState::default())
        .insert_resource(mission::MissionManager::default())
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
        .add_systems(Startup, mission::spawn_mission_markers)
        // === 更新系統 ===
        .add_systems(
            Update,
            (
                mission::mission_system.in_set(core::InteractionSet::Mission),
                mission::mission_marker_animation,
            )
                .run_if(in_state(core::AppState::InGame)),
        )
        .run();
}
