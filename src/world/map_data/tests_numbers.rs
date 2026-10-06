//! 地圖資料的數值檢查：NaN 與無限大也要在讀檔時報錯，並指出是哪一筆

use super::tests::{assert_error, real_file};

#[test]
fn rejects_non_finite_segment_ends() {
    // 「起點要小於終點」寫成 from >= to 的話，NaN 比較永遠是 false，會被放過
    for (road, from, to, needle) in [
        (8, f32::NAN, -7.5, "路段 #8（昆明街）：起點"),
        (8, -49.0, f32::NAN, "路段 #8（昆明街）：起點"),
        (8, f32::NEG_INFINITY, -7.5, "路段 #8（昆明街）：起點"),
        (0, -105.0, f32::INFINITY, "路段 #0（中華路）：起點"),
    ] {
        let mut file = real_file();
        file.roads[road].from = from;
        file.roads[road].to = to;
        assert_error(&file, needle);
    }
}

#[test]
fn rejects_non_finite_segment_width() {
    for (road, width, needle) in [
        (8, f32::NAN, "路段 #8（昆明街）：寬度"),
        (0, f32::INFINITY, "路段 #0（中華路）：寬度"),
    ] {
        let mut file = real_file();
        file.roads[road].width = width;
        assert_error(&file, needle);
    }
}

#[test]
fn rejects_non_finite_segment_position() {
    let mut file = real_file();
    file.roads[8].at = f32::NAN;
    assert_error(&file, "路段 #8（昆明街）：位置");
}

#[test]
fn rejects_non_finite_bounds() {
    let mut file = real_file();
    file.bounds.max_x = f32::INFINITY;
    assert_error(&file, "邊界：");
}

#[test]
fn rejects_non_finite_grid_origin() {
    for origin in [(f32::NAN, -90.0), (-110.0, f32::INFINITY)] {
        let mut file = real_file();
        file.pathfinding_grid.origin = origin;
        assert_error(&file, "A* 網格：原點");
    }
}
