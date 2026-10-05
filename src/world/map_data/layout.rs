//! 檢查並解析地圖資料檔，產生系統直接取用的 `MapLayout`

use bevy::prelude::*;

use super::file::MapFile;
use crate::world::MapBounds;

/// 資料檔的一筆錯誤（訊息指出是哪一筆）
#[derive(Debug, Clone, PartialEq)]
pub struct MapError(pub String);

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 一面邊界牆的碰撞盒
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallBox {
    pub center: Vec3,
    pub half_extents: Vec3,
}

/// 解析後的地圖：路名都已換成座標，系統只負責生成
#[derive(Resource, Debug, Clone)]
pub struct MapLayout {
    pub bounds: MapBounds,
    /// 玩家出生點 (x, z)
    pub spawn: Vec2,
    pub ground_center: Vec3,
    pub ground_collider_half_extents: Vec3,
    /// 東、西、南、北
    pub walls: [WallBox; 4],
}

impl MapLayout {
    /// 檢查並解析；有錯時回傳每一筆錯誤
    pub fn from_file(file: &MapFile) -> Result<Self, Vec<MapError>> {
        let mut errors = Vec::new();
        check_bounds_and_spawn(file, &mut errors);
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Self::base(file))
    }

    /// 不需要查路名的部分
    fn base(file: &MapFile) -> Self {
        let b = file.bounds;
        let (ground_x, ground_z) = file.ground.center;
        let (hx, hy, hz) = file.ground.collider_half_extents;
        let w = file.walls;
        let along_z = Vec3::new(w.half_thickness, w.half_height, w.along_z_half_length);
        let along_x = Vec3::new(w.along_x_half_length, w.half_height, w.half_thickness);
        Self {
            bounds: MapBounds {
                min_x: b.min_x,
                max_x: b.max_x,
                min_z: b.min_z,
                max_z: b.max_z,
            },
            spawn: Vec2::new(file.spawn.0, file.spawn.1),
            ground_center: Vec3::new(ground_x, 0.0, ground_z),
            ground_collider_half_extents: Vec3::new(hx, hy, hz),
            walls: [
                WallBox {
                    center: Vec3::new(b.max_x + w.offset, w.center_y, ground_z),
                    half_extents: along_z,
                },
                WallBox {
                    center: Vec3::new(b.min_x - w.offset, w.center_y, ground_z),
                    half_extents: along_z,
                },
                WallBox {
                    center: Vec3::new(ground_x, w.center_y, b.max_z + w.offset),
                    half_extents: along_x,
                },
                WallBox {
                    center: Vec3::new(ground_x, w.center_y, b.min_z - w.offset),
                    half_extents: along_x,
                },
            ],
        }
    }
}

/// 邊界要是正的矩形、出生點要在邊界內
fn check_bounds_and_spawn(file: &MapFile, errors: &mut Vec<MapError>) {
    let b = file.bounds;
    if !(b.min_x < b.max_x && b.min_z < b.max_z) {
        errors.push(MapError(format!("邊界：min 必須小於 max（{b:?}）")));
        return;
    }
    let (x, z) = file.spawn;
    if !(b.min_x..=b.max_x).contains(&x) || !(b.min_z..=b.max_z).contains(&z) {
        errors.push(MapError(format!("出生點 ({x}, {z}) 在邊界外")));
    }
}
