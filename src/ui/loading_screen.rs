//! Loading Screen 載入畫面 (GTA 5 風格)
//!
//! 遊戲啟動時顯示深色背景、遊戲標題、進度條與輪播提示文字。
//! 所有關鍵資產（字體、紋理）載入完成且最少顯示 2 秒後轉場至 InGame。

use bevy::prelude::*;

use super::components::ChineseFont;
use crate::core::AppState;

// ============================================================================
// 常數
// ============================================================================

/// 最低顯示時間（秒）
const MIN_DISPLAY_TIME: f32 = 2.0;
/// 載入超時時間（秒）— 超過此時間強制進入遊戲
const MAX_LOADING_TIME: f32 = 30.0;
/// 提示文字輪播間隔（秒）
const TIP_INTERVAL: f32 = 5.0;
/// 進度條寬度
const PROGRESS_BAR_WIDTH: f32 = 400.0;
/// 進度條高度
const PROGRESS_BAR_HEIGHT: f32 = 8.0;
/// 進度條圓角
const PROGRESS_BAR_RADIUS: f32 = 4.0;

/// 背景色（深色）
const BG_COLOR: Color = Color::srgb(0.02, 0.02, 0.05);
/// 標題色（金色）
const TITLE_COLOR: Color = Color::srgb(0.85, 0.72, 0.3);
/// 副標題色
const SUBTITLE_COLOR: Color = Color::srgba(0.8, 0.8, 0.8, 0.7);
/// 提示文字色
const TIP_COLOR: Color = Color::srgba(0.7, 0.7, 0.7, 0.6);
/// 進度條背景色
const PROGRESS_BG_COLOR: Color = Color::srgba(0.2, 0.2, 0.2, 0.5);
/// 進度條填充色（綠色）
const PROGRESS_FILL_COLOR: Color = Color::srgb(0.2, 0.75, 0.3);

/// 台灣在地化提示文字
const TIPS: &[&str] = &[
    "提示：按 V 切換第一人稱和第三人稱視角",
    "提示：衝刺時體力會消耗，站立不動可以回復",
    "提示：按數字鍵 1-4 快速切換武器",
    "提示：按 C 進入電影模式自由飛行",
    "提示：蹲伏靠近敵人可以發動潛行擊殺",
    "提示：在 ATM 前按 F 可以存取款",
    "提示：通緝度越高，出動的警力等級越高",
    "提示：善用掩體系統可以有效躲避敵人攻擊",
];

// ============================================================================
// 組件標記
// ============================================================================

/// 載入畫面根節點
#[derive(Component)]
struct LoadingScreenRoot;

/// 進度條填充區域
#[derive(Component)]
struct ProgressBarFill;

/// 提示文字
#[derive(Component)]
struct TipText;

// ============================================================================
// 狀態資源
// ============================================================================

/// 載入畫面狀態
#[derive(Resource)]
struct LoadingScreenState {
    /// 已顯示時間
    elapsed: f32,
    /// 當前提示索引
    tip_index: usize,
    /// 提示輪播計時器
    tip_timer: f32,
}

impl Default for LoadingScreenState {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            tip_index: 0,
            tip_timer: 0.0,
        }
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 建立載入畫面 UI
fn setup_loading_screen(mut commands: Commands, font: Res<ChineseFont>) {
    commands.init_resource::<LoadingScreenState>();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BG_COLOR),
            ZIndex(200), // 最高層，覆蓋所有 HUD
            LoadingScreenRoot,
        ))
        .with_children(|root| {
            // 遊戲標題
            root.spawn((
                Text::new("島嶼狂飆"),
                TextFont {
                    font_size: 64.0,
                    font: font.font.clone(),
                    ..default()
                },
                TextColor(TITLE_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            // 英文副標題
            root.spawn((
                Text::new("Island Rampage"),
                TextFont {
                    font_size: 24.0,
                    font: font.font.clone(),
                    ..default()
                },
                TextColor(SUBTITLE_COLOR),
                Node {
                    margin: UiRect::bottom(Val::Px(60.0)),
                    ..default()
                },
            ));

            // 進度條容器
            root.spawn((
                Node {
                    width: Val::Px(PROGRESS_BAR_WIDTH),
                    height: Val::Px(PROGRESS_BAR_HEIGHT),
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(PROGRESS_BG_COLOR),
                BorderRadius::all(Val::Px(PROGRESS_BAR_RADIUS)),
            ))
            .with_children(|bar| {
                // 進度條填充
                bar.spawn((
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(PROGRESS_FILL_COLOR),
                    BorderRadius::all(Val::Px(PROGRESS_BAR_RADIUS)),
                    ProgressBarFill,
                ));
            });

            // 提示文字
            root.spawn((
                Text::new(TIPS[0]),
                TextFont {
                    font_size: 18.0,
                    font: font.font.clone(),
                    ..default()
                },
                TextColor(TIP_COLOR),
                TipText,
            ));
        });
}

/// 載入畫面更新（偵測資產載入完成 + 最低顯示時間）
fn loading_screen_update(
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    font: Res<ChineseFont>,
    mut state: ResMut<LoadingScreenState>,
    mut next_state: ResMut<NextState<AppState>>,
    mut fill_query: Query<&mut Node, With<ProgressBarFill>>,
    mut tip_query: Query<&mut Text, With<TipText>>,
) {
    let dt = time.delta_secs();
    state.elapsed += dt;

    // 檢查字體是否載入完成
    let font_loaded = asset_server.is_loaded_with_dependencies(font.font.id());

    // 進度計算（模擬進度：50% 來自時間，50% 來自資產載入）
    let time_progress = (state.elapsed / MIN_DISPLAY_TIME).min(1.0);
    let asset_progress = if font_loaded { 1.0 } else { 0.5 };
    let overall_progress = (time_progress * 0.5 + asset_progress * 0.5).min(1.0);

    // 更新進度條
    if let Ok(mut fill_node) = fill_query.single_mut() {
        fill_node.width = Val::Percent(overall_progress * 100.0);
    }

    // 輪播提示文字
    state.tip_timer += dt;
    if state.tip_timer >= TIP_INTERVAL {
        state.tip_timer = 0.0;
        state.tip_index = (state.tip_index + 1) % TIPS.len();
        if let Ok(mut text) = tip_query.single_mut() {
            *text = Text::new(TIPS[state.tip_index]);
        }
    }

    // 轉場條件：(最低時間 + 資產已載入) 或 超時強制轉場
    if state.elapsed >= MIN_DISPLAY_TIME && font_loaded {
        info!("📦 載入完成，轉場至 InGame（{:.1}s）", state.elapsed);
        next_state.set(AppState::InGame);
    } else if state.elapsed >= MAX_LOADING_TIME {
        error!(
            "⚠️ 載入超時（{:.0}s），強制轉場至 InGame",
            state.elapsed
        );
        next_state.set(AppState::InGame);
    }
}

/// 清理載入畫面
fn cleanup_loading_screen(
    mut commands: Commands,
    query: Query<Entity, With<LoadingScreenRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<LoadingScreenState>();
}

// ============================================================================
// Plugin
// ============================================================================

pub(super) struct LoadingScreenPlugin;

impl Plugin for LoadingScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            setup_loading_screen.in_set(super::UiSetup),
        )
        .add_systems(
            Update,
            loading_screen_update.run_if(in_state(AppState::Loading)),
        )
        .add_systems(OnExit(AppState::Loading), cleanup_loading_screen);
    }
}
