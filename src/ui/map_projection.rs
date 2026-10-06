//! 世界座標 → 小地圖／大地圖 UI 座標的投影（全遊戲只有這一份）

use bevy::prelude::*;

/// 投影參數：每公尺幾 px、世界原點落在 UI 的哪裡
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MapProjection {
    pub scale: f32,
    pub offset: Vec2,
}

/// 小地圖：300 × 300 px，0.9 px/m，世界原點在中央
pub(crate) const MINIMAP: MapProjection = MapProjection {
    scale: 0.9,
    offset: Vec2::new(150.0, 150.0),
};

/// 大地圖：1200 × 800 px，2.0 px/m，世界原點在中央
pub(crate) const FULLMAP: MapProjection = MapProjection {
    scale: 2.0,
    offset: Vec2::new(600.0, 400.0),
};

impl MapProjection {
    /// 世界 (x, z) → UI 座標（x 往右、y 往下）；Z 取負號
    pub(crate) fn project(self, x: f32, z: f32) -> Vec2 {
        Vec2::new(
            x * self.scale + self.offset.x,
            -z * self.scale + self.offset.y,
        )
    }

    /// 世界長度 → UI 長度
    pub(crate) fn length(self, meters: f32) -> f32 {
        meters * self.scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_projects_to_map_center() {
        assert_eq!(MINIMAP.project(0.0, 0.0), Vec2::new(150.0, 150.0));
        assert_eq!(FULLMAP.project(0.0, 0.0), Vec2::new(600.0, 400.0));
    }

    #[test]
    fn lengths_scale_with_projection() {
        assert!((MINIMAP.length(10.0) - 9.0).abs() < 1e-5);
        assert!((FULLMAP.length(10.0) - 20.0).abs() < 1e-5);
    }
}
