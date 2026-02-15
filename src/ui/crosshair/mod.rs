//! 準星與彈藥系統 (GTA 風格)
//!
//! 包含：準星 UI、命中標記、彈藥顯示、武器切換動畫、準星動態效果

mod setup;
mod updates;

pub use setup::*;
pub use updates::*;

use bevy::prelude::*;

pub(super) struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_crosshair.in_set(super::UiSetup))
            .add_systems(
                Update,
                (
                    update_crosshair,
                    update_hit_marker,
                    update_ammo_display,
                    update_ammo_visual_grid,
                    update_weapon_switch_animation.after(update_ammo_display),
                    update_crosshair_dynamics,
                )
                    .in_set(super::UiActive),
            );
    }
}
