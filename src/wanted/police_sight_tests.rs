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
    // 警察貼在旁邊（0.7 m）也逮捕得到
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_officer(&mut app, Vec3::new(0.0, 0.0, -0.7), PoliceState::Alerted);
    assert!(arrest_starts(&mut app, player));
}

#[test]
fn police_cannot_arrest_across_a_fence() {
    // 隔著比警察眼睛高的圍籬：離地 1.7 m（在玩家前方 0.7 m）、離地 1.6 m（在玩家前方 1 m）
    for (distance, half_height) in [(0.7, 0.85), (1.0, 0.8)] {
        let mut app = police_app(2);
        let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
        spawn_block(
            &mut app,
            Vec3::new(0.0, 0.1 + half_height, -distance),
            Vec3::new(3.0, half_height, 0.05),
        );
        spawn_officer(&mut app, Vec3::new(0.0, 0.0, -1.9), PoliceState::Alerted);
        assert!(
            !arrest_starts(&mut app, player),
            "圍籬在玩家前方 {distance} m"
        );
    }
}

#[test]
fn police_arrest_across_a_waist_high_railing() {
    // 隔著離地 0.9 m 的欄杆（在玩家前方 0.5 m）：玩家站著舉手，警察看得到上半身，逮捕得到
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 0.55, -0.5),
        Vec3::new(3.0, 0.45, 0.1),
    );
    spawn_officer(&mut app, Vec3::new(0.0, 0.0, -1.9), PoliceState::Alerted);
    assert!(arrest_starts(&mut app, player));
}

#[test]
fn police_cannot_see_through_a_wall_the_player_is_hugging() {
    // 玩家貼著牆（牆背面離玩家身體 0.1 m）：牆就在射線終點前，不能被當成打到玩家
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
fn police_see_a_crouching_player_right_next_to_them() {
    // 警察貼在蹲著的玩家旁邊（0.6 m）：射線從眼睛往下斜，從膠囊側面上方打進去，
    // 命中點離瞄準點超過容許範圍，打到的是玩家就算看得到
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    app.world_mut()
        .get_mut::<crate::player::Player>(player)
        .expect("有玩家")
        .is_crouching = true;
    spawn_officer(&mut app, Vec3::new(0.0, 0.0, -0.6), PoliceState::Pursuing);
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

/// 玩家的姿勢
#[derive(Clone, Copy, Debug)]
enum Stance {
    Standing,
    Crouching,
    /// 躲在矮掩體後、沒探出
    InLowCover,
    /// 從矮掩體後往上探出
    PeekingOverLowCover,
    /// 躲在高掩體後（站著）
    InHighCover,
    /// 防禦：不在掩體中、掩體類型卻還留著矮掩體（遊戲離開掩體時會清掉）
    LeftLowCover,
}

/// 玩家前方 1 m 有頂端離地 0.8 m 的矮障礙物（像停著的機車），交戰中的警察在北邊 distance 公尺，
/// 回傳 (警察看不看得到, 開不開槍)
fn behind_a_waist_high_obstacle(stance: Stance, distance: f32) -> (bool, bool) {
    behind_an_obstacle(stance, distance, 0.8)
}

/// 玩家前方 1 m 有頂端離地 height 公尺的障礙物，交戰中的警察在北邊 distance 公尺，
/// 回傳 (警察看不看得到, 開不開槍)
fn behind_an_obstacle(stance: Stance, distance: f32, height: f32) -> (bool, bool) {
    use crate::combat::{PeekState, PlayerCoverState, PlayerCoverType};
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    let cover = |is_in_cover, cover_type, peek_state| PlayerCoverState {
        is_in_cover,
        cover_type,
        peek_state,
        ..default()
    };
    let cover_state = match stance {
        Stance::Standing => None,
        Stance::Crouching => {
            app.world_mut()
                .get_mut::<crate::player::Player>(player)
                .expect("有玩家")
                .is_crouching = true;
            None
        }
        Stance::InLowCover => Some(cover(true, PlayerCoverType::Low, PeekState::Hidden)),
        Stance::PeekingOverLowCover => {
            Some(cover(true, PlayerCoverType::Low, PeekState::PeekingUp))
        }
        Stance::InHighCover => Some(cover(true, PlayerCoverType::High, PeekState::Hidden)),
        Stance::LeftLowCover => Some(cover(false, PlayerCoverType::Low, PeekState::Hidden)),
    };
    if let Some(cover_state) = cover_state {
        app.world_mut().entity_mut(player).insert(cover_state);
    }
    spawn_block(
        &mut app,
        Vec3::new(0.0, 0.1 + height / 2.0, -1.0),
        Vec3::new(1.0, height / 2.0, 0.2),
    );
    let officer = spawn_officer(&mut app, Vec3::NEG_Z * distance, PoliceState::Engaging);
    (
        police_see_player(&mut app),
        officer_fires(&mut app, officer),
    )
}

#[test]
fn police_aim_at_the_neck_unless_the_player_keeps_low() {
    // 站著時頭和脖子露在矮障礙物上面，看得到也打得到；蹲下或躲在矮掩體後沒探出時藏得住
    for (stance, exposed) in [
        (Stance::Standing, true),
        (Stance::Crouching, false),
        (Stance::InLowCover, false),
        (Stance::PeekingOverLowCover, true),
        (Stance::InHighCover, true),
        (Stance::LeftLowCover, true),
    ] {
        assert_eq!(
            behind_a_waist_high_obstacle(stance, 20.0),
            (exposed, exposed),
            "{stance:?}"
        );
    }
}

#[test]
fn police_see_a_standing_player_whose_head_shows_over_a_chest_high_obstacle() {
    // 障礙物和肩膀差不多高（離地 1.0 m），擋得住軀幹和肩膀、擋不住頭和脖子：站著的玩家看得到
    assert_eq!(
        behind_an_obstacle(Stance::Standing, 20.0, 1.0),
        (true, true)
    );
}

#[test]
fn police_eyes_are_at_head_height() {
    // 警察眼睛在頭部高度（離地約 1.2 m）：蹲在矮障礙物後的玩家，5 m 外越不過障礙物看到，
    // 3 m 內從障礙物上方看得到（眼睛太高會遠遠就越過障礙物，太低則近距離也看不到）
    assert_eq!(
        behind_a_waist_high_obstacle(Stance::Crouching, 5.0),
        (false, false)
    );
    assert_eq!(
        behind_a_waist_high_obstacle(Stance::Crouching, 3.0),
        (true, true)
    );
}

#[test]
fn police_arrest_a_surrendered_player_standing_on_a_ledge_above_them() {
    // 玩家站在離地 1.15 m 的平台（像爬上車頂）邊緣舉手，警察站在平台下、水平距離 0.5 m：射線往上斜得很厲害，
    // 從膠囊側面下方打進去，命中點離瞄準點超過容許範圍，打到的是玩家就算看得到
    let mut app = police_app(2);
    let player = spawn_player_on_foot(&mut app, Vec3::ZERO);
    spawn_block(
        &mut app,
        Vec3::new(0.0, 0.675, 1.45),
        Vec3::new(3.0, 0.575, 1.55),
    );
    app.world_mut()
        .get_mut::<Transform>(player)
        .expect("有玩家")
        .translation = Vec3::new(0.0, 1.96, 0.0);
    spawn_officer(&mut app, Vec3::new(0.0, 0.0, -0.5), PoliceState::Alerted);
    assert!(arrest_starts(&mut app, player));
}
