//! 玩家死亡時清掉通緝：不然重生後還是同樣星數，警察、直升機又追過來，一直被打死

use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

use super::*;
use crate::combat::RespawnState;
use crate::core::AppState;

/// 5 星、正在被追（有搜索範圍、最後目擊位置）的場景
fn wanted_app(is_dead: bool) -> App {
    let mut app = App::new();
    app.insert_resource(RespawnState {
        is_dead,
        ..default()
    })
    .insert_resource(WantedLevel {
        stars: 5,
        heat: 105.0,
        cooldown_timer: 3.0,
        player_last_seen_pos: Some(Vec3::X),
        player_visible: true,
        search_center: Some(Vec3::X),
        search_timer: 30.0,
        ..default()
    })
    .add_message::<WantedLevelChanged>();
    app
}

fn clear_on_death(app: &mut App) {
    app.world_mut()
        .run_system_once(clear_wanted_on_death_system)
        .expect("跑得起來");
}

/// 送出的星數變化（舊, 新）
fn star_changes(app: &mut App) -> Vec<(u8, u8)> {
    app.world_mut()
        .resource_mut::<Messages<WantedLevelChanged>>()
        .drain()
        .map(|change| (change.old_stars, change.new_stars))
        .collect()
}

#[test]
fn dying_clears_the_wanted_level() {
    let mut app = wanted_app(true);
    clear_on_death(&mut app);
    let wanted = app.world().resource::<WantedLevel>();
    assert_eq!(wanted.stars, 0);
    assert_eq!(wanted.heat, 0.0);
    assert_eq!(wanted.cooldown_timer, 0.0);
    assert_eq!(wanted.player_last_seen_pos, None);
    assert!(!wanted.player_visible);
    assert_eq!(wanted.search_center, None);
    assert_eq!(wanted.search_timer, 0.0);
    assert_eq!(star_changes(&mut app), [(5, 0)]);
}

#[test]
fn the_wanted_level_stays_while_the_player_is_alive() {
    let mut app = wanted_app(false);
    clear_on_death(&mut app);
    let wanted = app.world().resource::<WantedLevel>();
    assert_eq!((wanted.stars, wanted.heat), (5, 105.0));
    assert_eq!(star_changes(&mut app), []);
}

/// 死掉之後才加上的熱度（例如死前犯的罪晚一步才處理），重生前也要清掉
#[test]
fn heat_gained_while_dead_is_cleared_too() {
    let mut app = wanted_app(true);
    clear_on_death(&mut app);
    app.world_mut().resource_mut::<WantedLevel>().add_heat(45.0);
    clear_on_death(&mut app);
    let wanted = app.world().resource::<WantedLevel>();
    assert_eq!((wanted.stars, wanted.heat), (0, 0.0));
    assert_eq!(star_changes(&mut app), [(5, 0), (2, 0)]);
}

/// 已經清掉之後，死亡期間每一幀不要再送星數變化
#[test]
fn a_cleared_wanted_level_sends_no_more_changes() {
    let mut app = wanted_app(true);
    clear_on_death(&mut app);
    star_changes(&mut app);
    clear_on_death(&mut app);
    assert_eq!(star_changes(&mut app), []);
}

/// label 這個排程裡有沒有名為 system 的系統
fn registered_in(app: &App, label: impl ScheduleLabel, system: &str) -> bool {
    let suffix = format!("::{system}");
    app.get_schedule(label).is_some_and(|schedule| {
        schedule
            .graph()
            .systems
            .iter()
            .any(|(_, registered, _)| registered.name().to_string().ends_with(&suffix))
    })
}

#[test]
fn the_wanted_plugin_clears_the_wanted_level_on_death() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .init_state::<AppState>()
        .add_plugins(WantedPlugin);
    assert!(registered_in(&app, Update, "clear_wanted_on_death_system"));
}

/// 從 1 星死掉也要送出星數變化
#[test]
fn dying_at_one_star_reports_the_change() {
    let mut app = wanted_app(true);
    app.world_mut().resource_mut::<WantedLevel>().stars = 1;
    clear_on_death(&mut app);
    assert_eq!(star_changes(&mut app), [(1, 0)]);
}

/// 清通緝要排在處理犯罪、目擊報警之後：同一幀才加上的熱度也要清掉，
/// 不然那一幀之後警察會看到大於 0 星
#[test]
fn clearing_on_death_runs_after_crimes_and_witness_reports() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .init_state::<AppState>()
        .add_plugins(WantedPlugin);
    let mut schedule = app
        .world_mut()
        .resource_mut::<Schedules>()
        .remove(Update)
        .expect("有 Update");
    schedule.initialize(app.world_mut()).expect("排得出來");
    let order: Vec<(bevy::ecs::schedule::SystemKey, String)> = schedule
        .systems()
        .expect("已經初始化")
        .map(|(key, system)| (key, system.name().to_string()))
        .collect();
    let find = |name: &str| {
        let suffix = format!("::{name}");
        order
            .iter()
            .position(|(_, system)| system.ends_with(&suffix))
            .unwrap_or_else(|| panic!("排程裡沒有 {name}"))
    };
    let clear = find("clear_wanted_on_death_system");
    for earlier in ["process_crime_events", "process_witness_reports"] {
        let earlier_index = find(earlier);
        let unordered = schedule
            .graph()
            .conflicting_systems()
            .iter()
            .any(|(a, b, _)| {
                let pair = [*a, *b];
                pair.contains(&order[earlier_index].0) && pair.contains(&order[clear].0)
            });
        assert!(!unordered, "{earlier} 和清通緝之間沒有排序");
        assert!(earlier_index < clear, "清通緝排在 {earlier} 之前");
    }
}
