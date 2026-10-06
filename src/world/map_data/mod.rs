//! 地圖資料：讀 `assets/levels/ximending.ron`（只存「決定」），解析成系統直接取用的 `MapLayout`
//!
//! - `file`：資料檔格式
//! - `layout`：檢查、解析、查詢
//! - `geometry`：從路網推算位置的規則

mod file;
mod geometry;
mod layout;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_numbers;

pub use file::*;
pub use geometry::*;
pub use layout::*;

use bevy::prelude::*;

/// 西門町地圖資料檔（編進執行檔）
const XIMENDING_RON: &str = include_str!("../../../assets/levels/ximending.ron");

/// 解析地圖資料；有錯時回傳每一筆錯誤
pub fn load_map(text: &str) -> Result<MapLayout, Vec<MapError>> {
    let file: MapFile = bevy::asset::ron::from_str(text)
        .map_err(|e| vec![MapError(format!("資料檔格式錯誤：{e}"))])?;
    MapLayout::from_file(&file)
}

/// 西門町地圖；資料檔有錯時直接 panic，遊戲一啟動就看得到是哪一筆
pub fn ximending_layout() -> MapLayout {
    load_map(XIMENDING_RON).unwrap_or_else(|errors| {
        let list: Vec<String> = errors.iter().map(ToString::to_string).collect();
        panic!("assets/levels/ximending.ron 有誤：\n{}", list.join("\n"))
    })
}

/// 載入西門町地圖並插入所有地圖 resource：`WorldPlugin` 與需要真實地圖的測試共用
pub fn install_map(app: &mut App) {
    let layout = ximending_layout();
    app.insert_resource(layout.bounds.clone())
        .insert_resource(layout);
}
