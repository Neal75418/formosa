//! 遊戲內通知系統（GTA 風格）
//!
//! 取代 println! 輸出，在畫面右上角顯示遊戲訊息

use bevy::prelude::*;
use std::collections::VecDeque;

// === 常數設定 ===
const MAX_NOTIFICATIONS: usize = 5;      // 最多同時顯示的通知數量
const DEFAULT_DURATION: f32 = 3.0;       // 預設顯示時間（秒）
const FADE_DURATION: f32 = 0.5;          // 淡出動畫時間（秒）
const NOTIFICATION_HEIGHT: f32 = 40.0;   // 每則通知的高度（加大）
const NOTIFICATION_SPACING: f32 = 6.0;   // 通知間距
const NOTIFICATION_WIDTH: f32 = 320.0;   // 通知寬度（加寬）
const NOTIFICATION_MARGIN: f32 = 16.0;   // 邊緣間距

// === GTA 風格通知顏色常數 ===
/// 通知外發光透明度
const NOTIF_GLOW_ALPHA: f32 = 0.15;
/// 通知邊框透明度
const NOTIF_BORDER_ALPHA: f32 = 0.7;

// === 通知類型 ===
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotificationType {
    Info,      // 一般訊息 (白色)
    Success,   // 成功 (綠色)
    Warning,   // 警告 (黃色)
    Error,     // 錯誤 (紅色)
}

impl NotificationType {
    /// 取得對應的文字顏色（GTA 風格）
    pub fn text_color(&self) -> Color {
        match self {
            NotificationType::Info => Color::srgb(0.95, 0.95, 0.98),
            NotificationType::Success => Color::srgb(0.35, 0.95, 0.45),
            NotificationType::Warning => Color::srgb(1.0, 0.85, 0.25),
            NotificationType::Error => Color::srgb(1.0, 0.35, 0.35),
        }
    }

    /// 取得對應的背景顏色（GTA 風格深色）
    pub fn bg_color(&self) -> Color {
        match self {
            NotificationType::Info => Color::srgba(0.08, 0.08, 0.12, 0.92),
            NotificationType::Success => Color::srgba(0.06, 0.15, 0.08, 0.92),
            NotificationType::Warning => Color::srgba(0.15, 0.12, 0.04, 0.92),
            NotificationType::Error => Color::srgba(0.18, 0.06, 0.06, 0.92),
        }
    }

    /// 取得對應的邊框顏色（GTA 風格）
    pub fn border_color(&self) -> Color {
        match self {
            NotificationType::Info => Color::srgba(0.4, 0.4, 0.5, NOTIF_BORDER_ALPHA),
            NotificationType::Success => Color::srgba(0.25, 0.7, 0.35, NOTIF_BORDER_ALPHA),
            NotificationType::Warning => Color::srgba(0.8, 0.65, 0.2, NOTIF_BORDER_ALPHA),
            NotificationType::Error => Color::srgba(0.8, 0.25, 0.25, NOTIF_BORDER_ALPHA),
        }
    }

    /// 取得對應的外發光顏色（GTA 風格）
    pub fn glow_color(&self) -> Color {
        match self {
            NotificationType::Info => Color::srgba(0.3, 0.3, 0.4, NOTIF_GLOW_ALPHA),
            NotificationType::Success => Color::srgba(0.2, 0.6, 0.3, NOTIF_GLOW_ALPHA),
            NotificationType::Warning => Color::srgba(0.7, 0.55, 0.15, NOTIF_GLOW_ALPHA),
            NotificationType::Error => Color::srgba(0.7, 0.2, 0.2, NOTIF_GLOW_ALPHA),
        }
    }

    /// 取得對應的圖示
    pub fn icon(&self) -> &'static str {
        match self {
            NotificationType::Info => "ℹ️",
            NotificationType::Success => "✅",
            NotificationType::Warning => "⚠️",
            NotificationType::Error => "❌",
        }
    }
}

// === 通知資料結構 ===
#[derive(Clone)]
pub struct Notification {
    pub message: String,
    pub notification_type: NotificationType,
    pub duration: f32,
    pub elapsed: f32,
}

impl Notification {
    /// 建立一般訊息
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Info,
            duration: DEFAULT_DURATION,
            elapsed: 0.0,
        }
    }

    /// 建立成功訊息
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Success,
            duration: DEFAULT_DURATION,
            elapsed: 0.0,
        }
    }

    /// 建立警告訊息
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Warning,
            duration: DEFAULT_DURATION + 1.0, // 警告多顯示 1 秒
            elapsed: 0.0,
        }
    }

    /// 建立錯誤訊息
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            notification_type: NotificationType::Error,
            duration: DEFAULT_DURATION + 2.0, // 錯誤多顯示 2 秒
            elapsed: 0.0,
        }
    }

    /// 設定自訂持續時間
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// 計算目前的透明度（淡出效果）
    pub fn alpha(&self) -> f32 {
        let remaining = self.duration - self.elapsed;
        if remaining > FADE_DURATION {
            1.0
        } else if remaining > 0.0 {
            remaining / FADE_DURATION
        } else {
            0.0
        }
    }

    /// 是否已過期
    pub fn is_expired(&self) -> bool {
        self.elapsed >= self.duration
    }
}

// === 通知佇列資源 ===
#[derive(Resource, Default)]
pub struct NotificationQueue {
    pub notifications: VecDeque<Notification>,
    /// 版本號：每次新增或移除通知時遞增，用於優化 UI 更新
    pub version: u32,
}

impl NotificationQueue {
    /// 新增通知
    pub fn push(&mut self, notification: Notification) {
        self.notifications.push_back(notification);
        self.version = self.version.wrapping_add(1);
        // 超過最大數量時移除最舊的
        while self.notifications.len() > MAX_NOTIFICATIONS {
            self.notifications.pop_front();
        }
    }

    /// 新增一般訊息的便捷方法
    pub fn info(&mut self, message: impl Into<String>) {
        self.push(Notification::info(message));
    }

    /// 新增成功訊息的便捷方法
    pub fn success(&mut self, message: impl Into<String>) {
        self.push(Notification::success(message));
    }

    /// 新增警告訊息的便捷方法
    pub fn warning(&mut self, message: impl Into<String>) {
        self.push(Notification::warning(message));
    }

    /// 新增錯誤訊息的便捷方法
    pub fn error(&mut self, message: impl Into<String>) {
        self.push(Notification::error(message));
    }
}

// === UI 組件 ===

/// 通知容器標記
#[derive(Component)]
pub struct NotificationContainer;

/// 單則通知的 UI 實體標記
#[derive(Component)]
pub struct NotificationUI {
    pub index: usize,
}

// === 系統 ===

/// 初始化通知容器 UI
pub fn setup_notification_ui(
    mut commands: Commands,
) {
    // 建立通知容器（左上角 - GTA 5 風格）
    // 避免與右側的小地圖/時間/金錢/天氣 UI 重疊
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(NOTIFICATION_MARGIN),
            top: Val::Px(20.0), // 螢幕左上角
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart, // 左對齊
            row_gap: Val::Px(NOTIFICATION_SPACING),
            ..default()
        },
        NotificationContainer,
    ));
}

/// 更新通知系統（GTA 風格優化版：只在通知列表變化時重建 UI）
#[allow(clippy::too_many_arguments)]
pub fn update_notifications(
    mut commands: Commands,
    time: Res<Time>,
    mut notification_queue: ResMut<NotificationQueue>,
    container_query: Query<Entity, With<NotificationContainer>>,
    notification_ui_query: Query<Entity, With<NotificationUI>>,
    mut notification_colors: Query<(&NotificationUI, &mut BackgroundColor, &mut BorderColor, &Children)>,
    mut text_colors: Query<&mut TextColor>,
    chinese_font: Option<Res<super::ChineseFont>>,
    mut last_version: Local<u32>,
) {
    // 更新所有通知的經過時間
    for notification in notification_queue.notifications.iter_mut() {
        notification.elapsed += time.delta_secs();
    }

    // 移除過期的通知（並更新版本號）
    let old_len = notification_queue.notifications.len();
    notification_queue.notifications.retain(|n| !n.is_expired());
    if notification_queue.notifications.len() != old_len {
        notification_queue.version = notification_queue.version.wrapping_add(1);
    }

    // 檢查是否需要重建 UI
    let needs_rebuild = *last_version != notification_queue.version;

    if needs_rebuild {
        *last_version = notification_queue.version;

        // 取得容器
        let Ok(container) = container_query.single() else {
            return;
        };

        // 取得字體（如果有載入）
        let font = chinese_font.as_ref().map(|f| f.font.clone());

        // 清除現有的通知 UI
        for entity in &notification_ui_query {
            commands.entity(entity).despawn();
        }

        // 重建通知 UI（GTA 風格）
        for (index, notification) in notification_queue.notifications.iter().enumerate() {
            let alpha = notification.alpha();
            let notif_type = notification.notification_type;
            let text_color = notif_type.text_color().with_alpha(alpha);
            let bg_color = notif_type.bg_color().with_alpha(alpha * 0.92);
            let border_color = notif_type.border_color().with_alpha(alpha * NOTIF_BORDER_ALPHA);
            let glow_color = notif_type.glow_color().with_alpha(alpha * NOTIF_GLOW_ALPHA);

            // 外發光層
            let glow_entity = commands.spawn((
                Node {
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(glow_color),
                BorderRadius::all(Val::Px(8.0)),
                NotificationUI { index },
            )).id();

            // 建立主通知卡片
            let notification_entity = commands.spawn((
                Node {
                    width: Val::Px(NOTIFICATION_WIDTH),
                    min_height: Val::Px(NOTIFICATION_HEIGHT),
                    padding: UiRect::new(Val::Px(12.0), Val::Px(12.0), Val::Px(10.0), Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(bg_color),
                BorderColor::all(border_color),
                BorderRadius::all(Val::Px(6.0)),
            )).id();

            // 建立圖示
            let icon_entity = if let Some(ref font_handle) = font {
                commands.spawn((
                    Text::new(notif_type.icon()),
                    TextFont {
                        font: font_handle.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                )).id()
            } else {
                commands.spawn((
                    Text::new(notif_type.icon()),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                )).id()
            };

            // 建立文字
            let text_entity = if let Some(ref font_handle) = font {
                commands.spawn((
                    Text::new(&notification.message),
                    TextFont {
                        font: font_handle.clone(),
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(text_color),
                )).id()
            } else {
                commands.spawn((
                    Text::new(&notification.message),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(text_color),
                )).id()
            };

            // 組裝：圖示 + 文字加入卡片
            commands.entity(notification_entity).add_child(icon_entity);
            commands.entity(notification_entity).add_child(text_entity);
            // 卡片加入發光層
            commands.entity(glow_entity).add_child(notification_entity);

            // 將發光層（包含卡片）加入容器
            commands.entity(container).add_child(glow_entity);
        }
    } else {
        // 只更新現有通知的顏色（用於淡出效果）
        // 注意：NotificationUI 現在在 glow 層上，需要遍歷子層更新
        for (notif_ui, mut bg_color, _border_color, children) in &mut notification_colors {
            if let Some(notification) = notification_queue.notifications.get(notif_ui.index) {
                let alpha = notification.alpha();
                let notif_type = notification.notification_type;
                let glow_col = notif_type.glow_color().with_alpha(alpha * NOTIF_GLOW_ALPHA);

                // 更新發光層背景色
                *bg_color = BackgroundColor(glow_col);

                // 更新子元素（卡片和文字）的顏色
                for child in children.iter() {
                    if let Ok(mut text_color) = text_colors.get_mut(child) {
                        let text_col = notif_type.text_color().with_alpha(alpha);
                        *text_color = TextColor(text_col);
                    }
                }
            }
        }
    }
}
