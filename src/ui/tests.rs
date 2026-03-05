//! UI 系統單元測試

use super::notification::{Notification, NotificationQueue, NotificationType};
use bevy::color::Alpha;

// ============================================================================
// NotificationType 測試
// ============================================================================

#[test]
fn notification_type_text_colors() {
    use bevy::color::Color;

    // 測試每種類型的文字顏色是否正確設置
    assert_eq!(
        NotificationType::Info.text_color(),
        Color::srgb(0.95, 0.95, 0.98)
    );
    assert_eq!(
        NotificationType::Success.text_color(),
        Color::srgb(0.35, 0.95, 0.45)
    );
    assert_eq!(
        NotificationType::Warning.text_color(),
        Color::srgb(1.0, 0.85, 0.25)
    );
    assert_eq!(
        NotificationType::Error.text_color(),
        Color::srgb(1.0, 0.35, 0.35)
    );
}

#[test]
fn notification_type_bg_colors_have_alpha() {
    // 測試背景色包含透明度
    let info_bg = NotificationType::Info.bg_color();
    let success_bg = NotificationType::Success.bg_color();

    // 背景應該有透明度（alpha = 0.92）
    assert!((info_bg.alpha() - 0.92).abs() < 0.01);
    assert!((success_bg.alpha() - 0.92).abs() < 0.01);
}

#[test]
fn notification_type_icons() {
    // 測試每種類型的圖示
    assert_eq!(NotificationType::Info.icon(), "ℹ️");
    assert_eq!(NotificationType::Success.icon(), "✅");
    assert_eq!(NotificationType::Warning.icon(), "⚠️");
    assert_eq!(NotificationType::Error.icon(), "❌");
}

// ============================================================================
// Notification 測試
// ============================================================================

#[test]
fn notification_info_creation() {
    let notif = Notification::info("測試訊息");

    assert_eq!(notif.message, "測試訊息");
    assert_eq!(notif.notification_type, NotificationType::Info);
    assert_eq!(notif.duration, 3.0); // DEFAULT_DURATION
    assert_eq!(notif.elapsed, 0.0);
}

#[test]
fn notification_success_creation() {
    let notif = Notification::success("成功");

    assert_eq!(notif.notification_type, NotificationType::Success);
    assert_eq!(notif.duration, 3.0);
}

#[test]
fn notification_warning_has_longer_duration() {
    let notif = Notification::warning("警告");

    assert_eq!(notif.notification_type, NotificationType::Warning);
    assert_eq!(notif.duration, 4.0); // DEFAULT_DURATION + 1.0
}

#[test]
fn notification_error_has_longest_duration() {
    let notif = Notification::error("錯誤");

    assert_eq!(notif.notification_type, NotificationType::Error);
    assert_eq!(notif.duration, 5.0); // DEFAULT_DURATION + 2.0
}

#[test]
fn notification_with_custom_duration() {
    let notif = Notification::info("自訂時間").with_duration(10.0);

    assert_eq!(notif.duration, 10.0);
}

#[test]
fn notification_alpha_full_at_start() {
    let notif = Notification::info("測試");

    // 剛建立時應該完全不透明
    assert!((notif.alpha() - 1.0).abs() < 0.01);
}

#[test]
fn notification_alpha_fading() {
    let mut notif = Notification::info("測試");

    // 模擬經過 2.7 秒（剩餘 0.3 秒，進入淡出階段）
    notif.elapsed = 2.7;

    // 剩餘時間 = 0.3 秒，FADE_DURATION = 0.5 秒
    // alpha = 0.3 / 0.5 = 0.6
    assert!((notif.alpha() - 0.6).abs() < 0.01);
}

#[test]
fn notification_alpha_zero_when_expired() {
    let mut notif = Notification::info("測試");
    notif.elapsed = 5.0; // 超過 duration

    assert_eq!(notif.alpha(), 0.0);
}

#[test]
fn notification_is_not_expired_initially() {
    let notif = Notification::info("測試");

    assert!(!notif.is_expired());
}

#[test]
fn notification_is_expired_after_duration() {
    let mut notif = Notification::info("測試");
    notif.elapsed = 3.0; // 正好 duration

    assert!(notif.is_expired());
}

#[test]
fn notification_is_expired_when_overtime() {
    let mut notif = Notification::info("測試");
    notif.elapsed = 5.0; // 超過 duration

    assert!(notif.is_expired());
}

// ============================================================================
// NotificationQueue 測試
// ============================================================================

#[test]
fn notification_queue_starts_empty() {
    let queue = NotificationQueue::default();

    assert_eq!(queue.notifications.len(), 0);
    assert_eq!(queue.version, 0);
}

#[test]
fn notification_queue_push() {
    let mut queue = NotificationQueue::default();
    let notif = Notification::info("測試");

    queue.push(notif);

    assert_eq!(queue.notifications.len(), 1);
    assert_eq!(queue.version, 1); // 版本號應該遞增
}

#[test]
fn notification_queue_version_increments() {
    let mut queue = NotificationQueue::default();

    queue.push(Notification::info("1"));
    assert_eq!(queue.version, 1);

    queue.push(Notification::info("2"));
    assert_eq!(queue.version, 2);

    queue.push(Notification::info("3"));
    assert_eq!(queue.version, 3);
}

#[test]
fn notification_queue_max_capacity() {
    let mut queue = NotificationQueue::default();

    // 新增 7 個通知（超過 MAX_NOTIFICATIONS = 5）
    for i in 0..7 {
        queue.push(Notification::info(format!("通知 {i}")));
    }

    // 應該只保留最新的 5 個
    assert_eq!(queue.notifications.len(), 5);

    // 最舊的 2 個（"通知 0" 和 "通知 1"）應該被移除
    assert_eq!(queue.notifications[0].message, "通知 2");
    assert_eq!(queue.notifications[4].message, "通知 6");
}

#[test]
fn notification_queue_info_convenience() {
    let mut queue = NotificationQueue::default();

    queue.info("資訊");

    assert_eq!(queue.notifications.len(), 1);
    assert_eq!(
        queue.notifications[0].notification_type,
        NotificationType::Info
    );
    assert_eq!(queue.notifications[0].message, "資訊");
}

#[test]
fn notification_queue_success_convenience() {
    let mut queue = NotificationQueue::default();

    queue.success("成功");

    assert_eq!(queue.notifications.len(), 1);
    assert_eq!(
        queue.notifications[0].notification_type,
        NotificationType::Success
    );
}

#[test]
fn notification_queue_warning_convenience() {
    let mut queue = NotificationQueue::default();

    queue.warning("警告");

    assert_eq!(queue.notifications.len(), 1);
    assert_eq!(
        queue.notifications[0].notification_type,
        NotificationType::Warning
    );
}

#[test]
fn notification_queue_error_convenience() {
    let mut queue = NotificationQueue::default();

    queue.error("錯誤");

    assert_eq!(queue.notifications.len(), 1);
    assert_eq!(
        queue.notifications[0].notification_type,
        NotificationType::Error
    );
}

#[test]
fn notification_queue_version_wrapping() {
    let mut queue = NotificationQueue {
        version: u32::MAX - 1,
        ..Default::default()
    };

    queue.push(Notification::info("1"));
    assert_eq!(queue.version, u32::MAX);

    // 測試 wrapping_add 正確處理溢出
    queue.push(Notification::info("2"));
    assert_eq!(queue.version, 0);
}

// ============================================================================
// InteractionPromptState 測試
// ============================================================================

use super::components::InteractionPromptState;

#[test]
fn interaction_prompt_state_starts_hidden() {
    let state = InteractionPromptState::default();

    assert!(!state.visible);
    assert_eq!(state.fade_progress, 0.0);
}

#[test]
fn interaction_prompt_state_show() {
    let mut state = InteractionPromptState::default();

    state.show("按 E 開門".to_string(), "E");

    assert!(state.visible);
    assert_eq!(state.text, "按 E 開門");
    assert_eq!(state.key, "E");
}

#[test]
fn interaction_prompt_state_hide() {
    let mut state = InteractionPromptState::default();

    state.show("測試".to_string(), "F");
    assert!(state.visible);

    state.hide();
    // hide() 不會立即設置 visible = false，而是設置 target_visibility = 0.0
    assert_eq!(state.target_visibility, 0.0);
}

#[test]
fn interaction_prompt_state_fade_in() {
    let mut state = InteractionPromptState::default();

    state.show("測試".to_string(), "F");

    // 模擬 0.1 秒的淡入（FADE_SPEED = 8.0）
    state.update(0.1);

    // fade_progress 應該從 0 增加到 0.8 (0 + 8.0 * 0.1)
    assert!((state.fade_progress - 0.8).abs() < 0.01);

    // 再更新 0.05 秒，應該達到 1.0（上限）
    state.update(0.05);
    assert!((state.fade_progress - 1.0).abs() < 0.01);
}

#[test]
fn interaction_prompt_state_fade_out() {
    let mut state = InteractionPromptState::default();

    state.show("測試".to_string(), "F");
    state.fade_progress = 1.0; // 假設已完全顯示

    state.hide();

    // 模擬 0.1 秒的淡出（FADE_SPEED = 8.0）
    state.update(0.1);

    // fade_progress 應該從 1.0 減少到 0.2 (1.0 - 8.0 * 0.1)
    assert!((state.fade_progress - 0.2).abs() < 0.01);
}

#[test]
fn interaction_prompt_state_becomes_invisible_when_faded() {
    let mut state = InteractionPromptState::default();

    state.show("測試".to_string(), "F");
    state.visible = true;
    state.fade_progress = 0.02;

    state.hide();
    state.update(0.1); // 淡出到 <= 0.0

    // 當 fade_progress <= 0.01 且 target_visibility <= 0.0 時，visible 應該為 false
    assert!(!state.visible);
    assert_eq!(state.fade_progress, 0.0);
}

#[test]
fn interaction_prompt_state_fade_clamped() {
    let mut state = InteractionPromptState::default();

    state.show("測試".to_string(), "F");

    // 更新超長時間，應該限制在 1.0
    state.update(10.0);
    assert_eq!(state.fade_progress, 1.0);

    state.hide();

    // 淡出超長時間，應該限制在 0.0
    state.update(10.0);
    assert_eq!(state.fade_progress, 0.0);
}

// ============================================================================
// GPS 轉彎方向測試
// ============================================================================

#[test]
fn gps_turn_direction_straight() {
    use super::components::GpsTurnDirection;
    assert_eq!(
        GpsTurnDirection::from_angle(0.0),
        GpsTurnDirection::Straight
    );
    assert_eq!(
        GpsTurnDirection::from_angle(0.1),
        GpsTurnDirection::Straight
    );
    assert_eq!(
        GpsTurnDirection::from_angle(-0.1),
        GpsTurnDirection::Straight
    );
}

#[test]
fn gps_turn_direction_left_right() {
    use super::components::GpsTurnDirection;
    assert_eq!(GpsTurnDirection::from_angle(1.0), GpsTurnDirection::Right);
    assert_eq!(GpsTurnDirection::from_angle(-1.0), GpsTurnDirection::Left);
}

#[test]
fn gps_turn_direction_uturn() {
    use super::components::GpsTurnDirection;
    assert_eq!(GpsTurnDirection::from_angle(3.0), GpsTurnDirection::UTurn);
    assert_eq!(GpsTurnDirection::from_angle(-3.0), GpsTurnDirection::UTurn);
}

#[test]
fn gps_turn_direction_symbols() {
    use super::components::GpsTurnDirection;
    assert!(!GpsTurnDirection::Straight.symbol().is_empty());
    assert!(!GpsTurnDirection::Left.symbol().is_empty());
    assert!(!GpsTurnDirection::Right.symbol().is_empty());
    assert!(!GpsTurnDirection::UTurn.symbol().is_empty());
    assert!(!GpsTurnDirection::Arrived.symbol().is_empty());
}

#[test]
fn gps_turn_direction_labels() {
    use super::components::GpsTurnDirection;
    assert_eq!(GpsTurnDirection::Straight.label(), "直行");
    assert_eq!(GpsTurnDirection::Left.label(), "左轉");
    assert_eq!(GpsTurnDirection::Right.label(), "右轉");
}

#[test]
fn gps_navigation_clear_resets_turn() {
    use super::components::{GpsNavigationState, GpsTurnDirection};
    let mut gps = GpsNavigationState {
        next_turn_direction: GpsTurnDirection::Left,
        distance_to_next_turn: 50.0,
        ..Default::default()
    };
    gps.clear();
    assert_eq!(gps.next_turn_direction, GpsTurnDirection::Straight);
    assert!((gps.distance_to_next_turn - 0.0).abs() < f32::EPSILON);
}

#[test]
fn gps_distance_format() {
    use super::gps_navigation::format_gps_distance;
    assert_eq!(format_gps_distance(500.0), "500 m");
    assert_eq!(format_gps_distance(1500.0), "1.5 km");
    assert_eq!(format_gps_distance(50.0), "50 m");
}
