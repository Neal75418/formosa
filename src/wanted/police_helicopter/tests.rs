//! 直升機的射線瞄準玩家、對玩家投降與通緝清掉停火（真 Rapier 射線）

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::components::*;
use super::*;
use crate::combat::{CombatVisuals, DamageEvent};
use crate::wanted::test_support::*;
use crate::wanted::WantedLevel;

/// 每一槍都擲中的命中率（打不打得到人只看射線）
const ALWAYS_HITS: HelicopterAccuracy = HelicopterAccuracy {
    base_hit_chance: 1.0,
    distance_hit_penalty: 0.0,
    min_hit_chance: 1.0,
};

/// 每一槍都擲不中的命中率
const NEVER_HITS: HelicopterAccuracy = HelicopterAccuracy {
    base_hit_chance: 0.0,
    distance_hit_penalty: 0.0,
    min_hit_chance: 0.0,
};

/// stars 星、直升機每一槍都擲中的場景（打不打得到人只看射線）
fn helicopter_app(stars: u8) -> App {
    let mut app = rapier_app();
    let mut meshes = Assets::<Mesh>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    app.insert_resource(CombatVisuals::new(&mut meshes, &mut materials))
        .insert_resource(WantedLevel { stars, ..default() })
        .insert_resource(ALWAYS_HITS)
        .add_message::<DamageEvent>();
    app
}

/// 懸停高度、在玩家（原點）北邊 distance 公尺
fn hovering_north(distance: f32) -> Vec3 {
    Vec3::new(0.0, HELICOPTER_HOVER_ALTITUDE, -distance)
}

/// 攻擊中的直升機停在 at，機頭朝 nose_toward（同遊戲只轉水平方向）
fn spawn_attacking_helicopter(app: &mut App, at: Vec3, nose_toward: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Transform::from_translation(at).looking_at(nose_toward.with_y(at.y), Vec3::Y),
            PoliceHelicopter {
                state: HelicopterState::Attacking,
                search_timer: 1.0,
                last_hit_time: f32::NEG_INFINITY,
                ..default()
            },
        ))
        .id()
}

/// 跑直升機 AI，回傳直升機有沒有看到玩家（看到才會把搜索計時歸零）
fn helicopter_sees_player(app: &mut App, helicopter: Entity) -> bool {
    settle(app);
    app.world_mut()
        .run_system_once(helicopter_ai_system)
        .expect("跑得起來");
    app.world()
        .get::<PoliceHelicopter>(helicopter)
        .expect("直升機還在")
        .search_timer
        == 0.0
}

/// 跑直升機射擊，回傳 (有沒有開火, 打到玩家的傷害事件數)；開火才會重設射擊冷卻
fn helicopter_shoots(app: &mut App, helicopter: Entity, player: Entity) -> (bool, usize) {
    settle(app);
    app.world_mut()
        .run_system_once(helicopter_combat_system)
        .expect("跑得起來");
    let fired = app
        .world()
        .get::<PoliceHelicopter>(helicopter)
        .expect("直升機還在")
        .fire_cooldown
        > 0.0;
    let hits = app
        .world_mut()
        .resource_mut::<Messages<DamageEvent>>()
        .drain()
        .filter(|event| event.target == player)
        .count();
    (fired, hits)
}

#[test]
fn helicopter_hits_the_player_in_the_open() {
    for distance in [10.0, 20.0, 30.0] {
        let mut app = helicopter_app(5);
        let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
        let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(distance), Vec3::ZERO);
        assert_eq!(
            helicopter_shoots(&mut app, helicopter, player),
            (true, 1),
            "北邊 {distance} m"
        );
    }
}

#[test]
fn helicopter_hits_the_player_with_its_nose_turned_away() {
    // 機頭背對玩家時槍口比機身中心離玩家更遠：射線長度要從槍口算，從機身中心算會不夠長
    let mut app = helicopter_app(5);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    let at = hovering_north(30.0);
    let helicopter = spawn_attacking_helicopter(&mut app, at, at * 2.0);
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (true, 1));
}

#[test]
fn a_roof_hides_the_player_from_the_helicopter() {
    let mut app = helicopter_app(5);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 6.0, 0.0),
        Vec3::new(30.0, 0.2, 30.0),
    );
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert!(!helicopter_sees_player(&mut app, helicopter));
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (true, 0));
}

#[test]
fn helicopter_holds_fire_at_a_surrendered_player() {
    let mut app = helicopter_app(5);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    surrender(&mut app, player);
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (false, 0));
}

#[test]
fn helicopter_holds_fire_once_the_wanted_level_clears() {
    let mut app = helicopter_app(0);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (false, 0));
}

/// 擲骰沒中的子彈打在玩家旁邊，就算看得到玩家也不扣血
#[test]
fn a_shot_that_misses_the_roll_deals_no_damage() {
    let mut app = helicopter_app(5);
    app.insert_resource(NEVER_HITS);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert_eq!(helicopter_shoots(&mut app, helicopter, player), (true, 0));
}

/// 命中率貼身 25%，越遠越低，最低 8%
#[test]
fn helicopter_hit_chance_drops_with_distance() {
    let accuracy = HelicopterAccuracy::default();
    assert!((accuracy.hit_chance(0.0) - 0.25).abs() < 1e-6);
    assert!((accuracy.hit_chance(50.0) - 0.13).abs() < 1e-6);
    assert!(accuracy.hit_chance(30.0) > accuracy.hit_chance(45.0));
    assert!((accuracy.hit_chance(500.0) - 0.08).abs() < 1e-6);
}

/// 攻擊中的直升機會一路靠到玩家頭頂附近才停（水平約 10 m、高約 41 m）
const OVERHEAD: Vec3 = Vec3::new(0.0, 41.0, -10.0);

/// 從頭頂附近（機頭朝玩家）開槍時，槍口到站著的玩家的距離
fn overhead_muzzle_distance() -> f32 {
    (calc_muzzle_position(OVERHEAD, Dir3::Z) - Vec3::new(0.0, 0.8, 0.0)).length()
}

/// 照預設命中率、每秒 8 發、每發 8，一架停在頭頂附近的直升機平均約 10 秒打死滿血、站在空曠處的玩家
/// （60 fps 實際每秒 7.5 發，約 11 秒，見 known-issues；每次的實際時間看運氣）。沒有命中率時不到 2 秒
#[test]
fn one_helicopter_overhead_takes_about_ten_seconds_on_average() {
    let damage_per_second = HELICOPTER_FIRE_RATE
        * HELICOPTER_BULLET_DAMAGE
        * HelicopterAccuracy::default().hit_chance(overhead_muzzle_distance());
    let seconds_to_kill = 100.0 / damage_per_second;
    assert!(
        (8.0..16.0).contains(&seconds_to_kill),
        "平均 {seconds_to_kill} 秒"
    );
    let without_misses = 100.0 / (HELICOPTER_FIRE_RATE * HELICOPTER_BULLET_DAMAGE);
    assert!(without_misses < 2.0, "沒有命中率時 {without_misses} 秒");
}

/// 從 at 開 shots 槍（機頭朝玩家、每一槍前把冷卻歸零），回傳打中玩家的比例和槍口到玩家的距離
fn hit_rate_from(at: Vec3, shots: usize) -> (f32, f32) {
    let mut app = helicopter_app(5);
    app.insert_resource(HelicopterAccuracy::default());
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    let helicopter = spawn_attacking_helicopter(&mut app, at, Vec3::ZERO);
    settle(&mut app);
    let mut hits = 0;
    for _ in 0..shots {
        app.world_mut()
            .get_mut::<PoliceHelicopter>(helicopter)
            .expect("直升機還在")
            .fire_cooldown = 0.0;
        app.world_mut()
            .run_system_once(helicopter_combat_system)
            .expect("跑得起來");
        hits += app
            .world_mut()
            .resource_mut::<Messages<DamageEvent>>()
            .drain()
            .filter(|event| event.target == player)
            .count();
    }
    let forward = Dir3::new((Vec3::ZERO - at).with_y(0.0)).expect("不在玩家正上方");
    let distance = (calc_muzzle_position(at, forward) - Vec3::new(0.0, 0.8, 0.0)).length();
    (hits as f32 / shots as f32, distance)
}

/// 射擊系統照距離擲骰：從頭頂附近（約 15%）和很近的地方（約 23%；高 10 m 低於遊戲最低高度，
/// 只是為了拉開距離差）各開 4000 槍，打中的比例要接近那個距離的命中率（容許 ±3 個百分點，約 5 個標準差）。
/// 分不出距離是從槍口還是機身中心算（兩者差不到 0.5 個百分點）
#[test]
fn shots_hit_at_the_hit_chance_for_the_distance() {
    for at in [OVERHEAD, Vec3::new(0.0, 10.0, -5.0)] {
        let (rate, distance) = hit_rate_from(at, 4000);
        let expected = HelicopterAccuracy::default().hit_chance(distance);
        assert!(
            (rate - expected).abs() < 0.03,
            "在 {at}：打中 {rate}，{distance} m 的命中率 {expected}"
        );
    }
}

/// 沒擲中的子彈瞄玩家旁邊 1.5～3 m（水平、高度不變）的點
#[test]
fn a_missed_shot_aims_one_and_a_half_to_three_meters_beside_the_player() {
    let player = Vec3::new(2.0, 0.8, -3.0);
    let horizontal = |point: Vec3| (point - player).with_y(0.0).length();
    assert!((horizontal(calc_miss_point(player, 0.3, 0.0)) - 1.5).abs() < 1e-5);
    assert!((horizontal(calc_miss_point(player, 0.3, 0.999_999)) - 3.0).abs() < 1e-4);
    assert_eq!(calc_miss_point(player, 0.3, 0.5).y, player.y);
    assert!(calc_miss_point(player, 0.0, 0.5).distance(calc_miss_point(player, 0.5, 0.5)) > 4.0);
}

/// 彈道（起點、終點）：彈道畫成中心在中點、沿 Y 軸拉長的方塊
fn tracer_segments(app: &mut App) -> Vec<(Vec3, Vec3)> {
    app.world_mut()
        .query_filtered::<&Transform, With<crate::combat::BulletTracer>>()
        .iter(app.world())
        .map(|tracer| {
            let half = tracer.rotation * Vec3::Y * (tracer.scale.y / 2.0);
            (tracer.translation - half, tracer.translation + half)
        })
        .collect()
}

/// 點到線段的最短距離
fn distance_to_segment(point: Vec3, (start, end): (Vec3, Vec3)) -> f32 {
    let along = end - start;
    let t = ((point - start).dot(along) / along.length_squared()).clamp(0.0, 1.0);
    (start + along * t).distance(point)
}

/// 沒擲中的子彈從玩家身邊飛過（看得出打偏），擲中的子彈畫到玩家身上
#[test]
fn a_missed_shot_visibly_flies_past_the_player() {
    for (accuracy, misses) in [(NEVER_HITS, true), (ALWAYS_HITS, false)] {
        let mut app = helicopter_app(5);
        app.insert_resource(accuracy);
        let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
        let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
        helicopter_shoots(&mut app, helicopter, player);
        app.update();
        let player_center = Vec3::new(0.0, 0.8, 0.0);
        let tracers = tracer_segments(&mut app);
        assert_eq!(tracers.len(), 1, "彈道數");
        let closest = distance_to_segment(player_center, tracers[0]);
        assert_eq!(closest > 1.0, misses, "彈道離玩家 {closest} m");
    }
}

/// 遊戲啟動時要有命中率資源，直升機射擊系統才跑得起來
#[test]
fn helicopter_setup_provides_the_accuracy() {
    let mut world = World::new();
    world.init_resource::<Assets<Mesh>>();
    world.init_resource::<Assets<StandardMaterial>>();
    world
        .run_system_once(setup_helicopter_visuals)
        .expect("跑得起來");
    world.flush();
    assert!(world.contains_resource::<HelicopterAccuracy>());
}

#[test]
fn helicopter_sees_the_player_in_the_open() {
    let mut app = helicopter_app(5);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert!(helicopter_sees_player(&mut app, helicopter));
}

/// 貼著牆躲（直升機那一側的牆面離玩家中心 0.35 m）：射線打到牆的地方離瞄準點約 0.75 m，
/// 容許範圍要小於這個距離才擋得住（原本的 1.0 會當成看得到）
#[test]
fn a_wall_the_player_hugs_hides_them_from_the_helicopter() {
    let mut app = helicopter_app(5);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 1.85, -0.3),
        Vec3::new(3.0, 1.75, 0.05),
    );
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert!(!helicopter_sees_player(&mut app, helicopter));
}

/// 開車時射線先打到車頂：打到玩家坐的車就算看到
#[test]
fn helicopter_sees_a_player_driving_a_car() {
    let mut app = helicopter_app(5);
    spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Car);
    let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
    assert!(helicopter_sees_player(&mut app, helicopter));
}

/// 開車時玩家的碰撞體關著，子彈打到的是車：打到玩家坐的車就算打中玩家
#[test]
fn helicopter_hits_a_player_driving_a_car() {
    for vehicle in [TestVehicle::Car, TestVehicle::Scooter] {
        let mut app = helicopter_app(5);
        let (player, _) = spawn_player_in_vehicle(&mut app, Vec3::ZERO, vehicle);
        let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(20.0), Vec3::ZERO);
        assert_eq!(helicopter_shoots(&mut app, helicopter, player), (true, 1));
    }
}

/// 直升機和步警一樣：站著瞄頸部；蹲下、或躲在矮掩體後沒探出時瞄身體中心。在 45 m 外（射線比較平），
/// 玩家前方 0.6 m 有頂端離地 1.4 m 的牆：站著頭和脖子露出來看得到，蹲下、躲矮掩體就看不到
#[test]
fn helicopter_aims_at_the_neck_unless_the_player_keeps_low() {
    for (stance, keeps_low) in [("站著", false), ("蹲下", true), ("躲矮掩體", true)] {
        let mut app = helicopter_app(5);
        let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
        app.world_mut()
            .get_mut::<crate::player::Player>(player)
            .expect("有玩家")
            .is_crouching = stance == "蹲下";
        if stance == "躲矮掩體" {
            app.world_mut()
                .entity_mut(player)
                .insert(crate::combat::PlayerCoverState {
                    is_in_cover: true,
                    cover_type: crate::combat::PlayerCoverType::Low,
                    peek_state: crate::combat::PeekState::Hidden,
                    ..default()
                });
        }
        spawn_block(
            &mut app,
            Vec3::new(0.0, 0.8, -0.6),
            Vec3::new(3.0, 0.7, 0.05),
        );
        let helicopter = spawn_attacking_helicopter(&mut app, hovering_north(45.0), Vec3::ZERO);
        assert_eq!(
            helicopter_sees_player(&mut app, helicopter),
            !keeps_low,
            "{stance}"
        );
    }
}

/// 直升機的視線只算靜態物體和車輛，行人擋在中間不算（直升機高 10 m 低於遊戲最低高度 25 m，
/// 只是為了讓射線平到穿過行人）
#[test]
fn a_pedestrian_in_between_does_not_hide_the_player_from_the_helicopter() {
    let mut app = helicopter_app(5);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.8, -1.0),
        bevy_rapier3d::prelude::RigidBody::KinematicPositionBased,
        bevy_rapier3d::prelude::Collider::capsule_y(0.45, 0.25),
        crate::core::PEDESTRIAN_COLLISION_GROUPS,
    ));
    let helicopter = spawn_attacking_helicopter(&mut app, Vec3::new(0.0, 10.0, -45.0), Vec3::ZERO);
    assert!(helicopter_sees_player(&mut app, helicopter));
}

/// 坐在車上時，沒擲中的子彈即使落在車身上也不算打中玩家（偏移 1.5～3 m，汽車半長 2 m，
/// 有些偏移點還在車身範圍內）
#[test]
fn a_missed_shot_that_lands_on_the_players_car_does_not_hit_them() {
    let mut app = helicopter_app(5);
    app.insert_resource(NEVER_HITS);
    let (player, _) = spawn_player_in_vehicle(&mut app, Vec3::ZERO, TestVehicle::Car);
    let helicopter = spawn_attacking_helicopter(&mut app, OVERHEAD, Vec3::ZERO);
    settle(&mut app);
    let mut hits = 0;
    for _ in 0..200 {
        app.world_mut()
            .get_mut::<PoliceHelicopter>(helicopter)
            .expect("直升機還在")
            .fire_cooldown = 0.0;
        app.world_mut()
            .run_system_once(helicopter_combat_system)
            .expect("跑得起來");
        hits += app
            .world_mut()
            .resource_mut::<Messages<DamageEvent>>()
            .drain()
            .filter(|event| event.target == player)
            .count();
    }
    assert_eq!(hits, 0);
}
