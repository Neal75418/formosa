//! 步警的射線瞄準玩家：看得到、打得到、逮捕得到，中間有東西擋就不行（真 Rapier 射線；地面頂在 0.1）

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::test_support::*;
use super::*;

/// 玩家在原點，警察站在北邊（−Z）這個距離
const OFFICER_DISTANCE: f32 = 10.0;

/// 跑通緝消退系統（裡面判斷警察看不看得到玩家），回傳有沒有被看到
fn police_see_player(app: &mut App) -> bool {
    settle(app);
    app.world_mut()
        .run_system_once(update_police_spatial_hash_system)
        .expect("跑得起來");
    app.world_mut()
        .run_system_once(wanted_cooldown_system)
        .expect("跑得起來");
    app.world().resource::<WantedLevel>().player_visible
}

/// 玩家已經舉手投降，跑逮捕系統，回傳警察有沒有開始逮捕
fn arrest_starts(app: &mut App, player: Entity) -> bool {
    surrender(app, player);
    settle(app);
    app.world_mut()
        .run_system_once(police_arrest_system)
        .expect("跑得起來");
    app.world()
        .get::<PlayerSurrenderState>(player)
        .expect("玩家還在")
        .being_arrested
}

#[test]
fn police_see_the_player_in_the_open() {
    let mut app = police_app(2);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Pursuing,
    );
    assert!(police_see_player(&mut app));
}

#[test]
fn police_shoot_at_the_player_in_the_open() {
    let mut app = police_app(2);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    let officer = spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Engaging,
    );
    assert!(officer_fires(&mut app, officer));
}

#[test]
fn a_low_wall_hides_the_player_from_police() {
    // 玩家躲在離地 1.2 m 的矮牆後（牆在玩家前方 2 m）：看不到、不開槍
    let mut app = police_app(2);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 0.7, -2.0),
        Vec3::new(3.0, 0.6, 0.2),
    );
    let officer = spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Engaging,
    );
    assert!(!police_see_player(&mut app));
    assert!(!officer_fires(&mut app, officer));
}

#[test]
fn a_wall_hides_the_player_from_police_up_close() {
    // 警察在 3 m 外、玩家前方 1 m 有離地 1.4 m 的牆：看不到、不開槍
    let mut app = police_app(2);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 0.75, -1.0),
        Vec3::new(3.0, 0.75, 0.05),
    );
    let officer = spawn_officer(&mut app, Vec3::new(0.0, 0.0, -3.0), PoliceState::Engaging);
    assert!(!police_see_player(&mut app));
    assert!(!officer_fires(&mut app, officer));
}

#[test]
fn police_arrest_the_surrendered_player_up_close() {
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_officer(&mut app, Vec3::new(0.0, 0.0, -1.9), PoliceState::Alerted);
    assert!(arrest_starts(&mut app, player));
}

#[test]
fn police_arrest_the_surrendered_player_right_next_to_them() {
    // 警察貼在旁邊（0.7 m）：射線很陡、從玩家膠囊頂端打進去，命中點離中心超過容許範圍，打到的是玩家也要算看得到
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_officer(&mut app, Vec3::new(0.0, 0.0, -0.7), PoliceState::Alerted);
    assert!(arrest_starts(&mut app, player));
}

#[test]
fn police_cannot_arrest_across_a_fence() {
    // 隔著離地 1.7 m 的圍籬（在玩家前方 0.7 m）
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 0.95, -0.7),
        Vec3::new(3.0, 0.85, 0.05),
    );
    spawn_officer(&mut app, Vec3::new(0.0, 0.0, -1.9), PoliceState::Alerted);
    assert!(!arrest_starts(&mut app, player));
}

#[test]
fn police_cannot_see_through_a_wall_the_player_is_hugging() {
    // 玩家貼著牆（牆背面離玩家身體 0.1 m）：牆就在射線終點前，不能被當成打到玩家。
    // 警察在 3 m 時射線比較陡，長度要從抬高的起點算；少算約 0.4 m，牆就落進算短了的終點前 0.3 m、被當成玩家
    for (wall, half_height) in [("離地 1.2 m 的矮牆", 0.6), ("3 m 高牆", 1.5)] {
        for distance in [OFFICER_DISTANCE, 3.0] {
            let mut app = police_app(2);
            spawn_player_on_foot(&mut app, Vec3::ZERO);
            spawn_block(
                &mut app,
                Vec3::new(0.0, 0.1 + half_height, -0.45),
                Vec3::new(3.0, half_height, 0.1),
            );
            let officer = spawn_officer(&mut app, Vec3::NEG_Z * distance, PoliceState::Engaging);
            assert!(
                !police_see_player(&mut app),
                "{wall}、警察 {distance} m：看得到"
            );
            assert!(
                !officer_fires(&mut app, officer),
                "{wall}、警察 {distance} m：開槍"
            );
        }
    }
}

#[test]
fn police_see_the_player_right_next_to_them() {
    // 警察貼在旁邊（0.7 m）：射線很陡、從膠囊頂端打進去，命中點離中心較遠，打到的是玩家就算看得到
    let mut app = police_app(2);
    spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_officer(&mut app, Vec3::new(0.0, 0.0, -0.7), PoliceState::Pursuing);
    assert!(police_see_player(&mut app));
}

#[test]
fn police_see_the_player_when_the_ray_hits_nothing() {
    // 射線只到玩家中心，什麼都沒打到就是中間沒有遮擋（膠囊射線偶爾會漏打；下車動畫時玩家碰撞體也還關著）
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    app.world_mut()
        .entity_mut(player)
        .insert(bevy_rapier3d::prelude::ColliderDisabled);
    spawn_officer(
        &mut app,
        Vec3::NEG_Z * OFFICER_DISTANCE,
        PoliceState::Pursuing,
    );
    assert!(police_see_player(&mut app));
}

#[test]
fn police_spot_walk_up_to_and_arrest_a_surrendered_player() {
    // 整條串起來（視線、AI、開槍、逮捕，不手動給「看得到」）：警戒中的警察在 20 m 外（不沿座標軸）
    // 看到已投降的玩家，走過去逮捕到完成，全程不開槍
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    surrender(&mut app, player);
    let officer = spawn_officer(&mut app, Vec3::new(12.0, 0.0, -16.0), PoliceState::Alerted);
    settle(&mut app);
    let mut arrest_done = false;
    let mut fired = false;
    for _ in 0..400 {
        let world = app.world_mut();
        world
            .run_system_once(update_police_spatial_hash_system)
            .expect("跑得起來");
        world
            .run_system_once(wanted_cooldown_system)
            .expect("跑得起來");
        world.run_system_once(police_ai_system).expect("跑得起來");
        world
            .run_system_once(police_combat_system)
            .expect("跑得起來");
        world
            .run_system_once(police_arrest_system)
            .expect("跑得起來");
        fired |= world
            .get::<PoliceOfficer>(officer)
            .expect("警察還在")
            .attack_cooldown
            > 0.0;
        arrest_done = world
            .get::<PlayerSurrenderState>(player)
            .expect("玩家還在")
            .arrest_progress
            >= 1.0;
        app.update();
        if arrest_done {
            break;
        }
    }
    assert!(arrest_done, "20 秒內沒有逮捕完成");
    assert!(!fired, "警察開過槍");
}
