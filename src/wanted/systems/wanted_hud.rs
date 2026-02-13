//! 通緝等級 HUD 顯示

use bevy::prelude::*;

use super::super::components::*;
use super::super::events::*;

// ============================================================================
// HUD 輔助函數
// ============================================================================

fn create_wanted_hud(commands: &mut Commands, wanted_stars: u8) {
    let hud_entity = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(5.0),
                ..default()
            },
            WantedHud,
        ))
        .id();

    for i in 0..5u8 {
        let initial_color = if i < wanted_stars {
            Color::srgb(1.0, 0.8, 0.0)
        } else {
            Color::srgba(0.3, 0.3, 0.3, 0.5)
        };

        let mut star = WantedStar::new(i);
        if i < wanted_stars {
            star.trigger_gain();
        }

        let star_entity = commands
            .spawn((
                Node {
                    width: Val::Px(24.0),
                    height: Val::Px(24.0),
                    ..default()
                },
                BackgroundColor(initial_color),
                star,
            ))
            .id();

        commands.entity(hud_entity).add_child(star_entity);
    }
}

fn calc_star_color(star: &WantedStar, wanted: &WantedLevel, time: f32) -> Color {
    let flash_boost = if star.flash_timer > 0.0 {
        let flash_progress = star.flash_timer / 0.5;
        (flash_progress * 10.0).sin().abs()
    } else {
        0.0
    };

    let gain_alpha = if star.is_gaining {
        star.gain_progress
    } else {
        1.0
    };

    if star.index < wanted.stars {
        let pulse = ((time * 3.0 + star.scale_phase).sin() * 0.2 + 0.8) * gain_alpha;

        if star.flash_timer > 0.0 {
            Color::srgb(1.0, 0.9 + flash_boost * 0.1, 0.7 + flash_boost * 0.3)
        } else if wanted.player_visible {
            Color::srgb(1.0, pulse * 0.3, 0.0)
        } else {
            Color::srgb(1.0, pulse, 0.0)
        }
    } else {
        Color::srgba(0.3, 0.3, 0.3, 0.5)
    }
}

fn calc_star_scale(star: &WantedStar, wanted: &WantedLevel, time: f32) -> f32 {
    if star.index < wanted.stars {
        let pulse = (time * 2.0 + star.scale_phase).sin() * 0.1;
        let flash_scale = if star.flash_timer > 0.0 {
            0.2 * (star.flash_timer / 0.5)
        } else {
            0.0
        };
        let gain_scale = if star.is_gaining {
            star.gain_progress
        } else {
            1.0
        };
        (1.0 + pulse + flash_scale) * gain_scale
    } else {
        1.0
    }
}

// ============================================================================
// 系統
// ============================================================================

/// 更新通緝等級 HUD
pub fn update_wanted_hud(
    mut commands: Commands,
    time: Res<Time>,
    wanted: Res<WantedLevel>,
    hud_query: Query<Entity, With<WantedHud>>,
    mut star_query: Query<(Entity, &mut WantedStar, &mut Node)>,
) {
    let t = time.elapsed_secs();
    let dt = time.delta_secs();

    if hud_query.is_empty() && wanted.stars > 0 {
        create_wanted_hud(&mut commands, wanted.stars);
    }

    for (entity, mut star, mut node) in &mut star_query {
        if star.flash_timer > 0.0 {
            star.flash_timer -= dt;
        }

        if star.is_gaining {
            star.gain_progress += dt * 3.0;
            if star.gain_progress >= 1.0 {
                star.gain_progress = 1.0;
                star.is_gaining = false;
            }
        }

        let color = calc_star_color(&star, &wanted, t);
        let scale = calc_star_scale(&star, &wanted, t);

        let base_size = 24.0;
        node.width = Val::Px(base_size * scale);
        node.height = Val::Px(base_size * scale);

        commands.entity(entity).insert(BackgroundColor(color));
    }

    if wanted.stars == 0 {
        for entity in &hud_query {
            commands.entity(entity).despawn();
        }
    }
}

/// 通緝等級變化動畫系統
pub fn wanted_level_change_animation(
    mut level_changed: MessageReader<WantedLevelChanged>,
    mut star_query: Query<&mut WantedStar>,
) {
    for event in level_changed.read() {
        if event.new_stars > event.old_stars {
            for mut star in &mut star_query {
                if star.index >= event.old_stars && star.index < event.new_stars {
                    star.trigger_gain();
                }
            }
        }
    }
}
