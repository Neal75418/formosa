//! 視覺改裝：烤漆、貼膜、尾翼、輪框

use bevy::prelude::*;

// ============================================================================
// 烤漆顏色
// ============================================================================

/// 烤漆顏色選項（8 色）
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PaintColor {
    /// 原廠色（不改變）
    #[default]
    Stock,
    /// 深紅
    CrimsonRed,
    /// 午夜藍
    MidnightBlue,
    /// 珍珠白
    PearlWhite,
    /// 金屬黑
    MetallicBlack,
    /// 金屬銀
    MetallicSilver,
    /// 森林綠
    ForestGreen,
    /// 日落橙
    SunsetOrange,
}

impl PaintColor {
    /// 取得顏色的 RGB 值
    pub fn to_color(self) -> Color {
        match self {
            PaintColor::Stock => Color::srgb(0.5, 0.5, 0.5),
            PaintColor::CrimsonRed => Color::srgb(0.7, 0.1, 0.1),
            PaintColor::MidnightBlue => Color::srgb(0.05, 0.05, 0.3),
            PaintColor::PearlWhite => Color::srgb(0.95, 0.93, 0.9),
            PaintColor::MetallicBlack => Color::srgb(0.05, 0.05, 0.05),
            PaintColor::MetallicSilver => Color::srgb(0.75, 0.75, 0.78),
            PaintColor::ForestGreen => Color::srgb(0.1, 0.35, 0.1),
            PaintColor::SunsetOrange => Color::srgb(0.9, 0.45, 0.1),
        }
    }

    /// 取得名稱
    pub fn name(&self) -> &'static str {
        match self {
            PaintColor::Stock => "原廠色",
            PaintColor::CrimsonRed => "深紅",
            PaintColor::MidnightBlue => "午夜藍",
            PaintColor::PearlWhite => "珍珠白",
            PaintColor::MetallicBlack => "金屬黑",
            PaintColor::MetallicSilver => "金屬銀",
            PaintColor::ForestGreen => "森林綠",
            PaintColor::SunsetOrange => "日落橙",
        }
    }

    /// 取得價格
    pub fn price(&self) -> i32 {
        match self {
            PaintColor::Stock => 0,
            _ => 3_000,
        }
    }

    /// 所有可選顏色
    pub fn all() -> &'static [PaintColor] {
        &[
            PaintColor::CrimsonRed,
            PaintColor::MidnightBlue,
            PaintColor::PearlWhite,
            PaintColor::MetallicBlack,
            PaintColor::MetallicSilver,
            PaintColor::ForestGreen,
            PaintColor::SunsetOrange,
        ]
    }
}

// ============================================================================
// 車窗貼膜
// ============================================================================

/// 車窗貼膜等級
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowTint {
    /// 無貼膜（透明）
    #[default]
    None,
    /// 淺色（35% 透光）
    Light,
    /// 中等（20% 透光）
    Medium,
    /// 深色（5% 透光）— 非法但帥
    Dark,
    /// 鏡面反射
    Mirror,
}

impl WindowTint {
    /// 取得玻璃材質的基本色
    pub fn to_glass_color(self) -> Color {
        match self {
            WindowTint::None => Color::srgb(0.1, 0.1, 0.1),
            WindowTint::Light => Color::srgb(0.15, 0.15, 0.18),
            WindowTint::Medium => Color::srgb(0.08, 0.08, 0.1),
            WindowTint::Dark => Color::srgb(0.02, 0.02, 0.03),
            WindowTint::Mirror => Color::srgb(0.4, 0.42, 0.45),
        }
    }

    /// 取得名稱
    pub fn name(&self) -> &'static str {
        match self {
            WindowTint::None => "無貼膜",
            WindowTint::Light => "淺色貼膜",
            WindowTint::Medium => "中等貼膜",
            WindowTint::Dark => "深色貼膜",
            WindowTint::Mirror => "鏡面反射",
        }
    }

    /// 取得價格
    pub fn price(&self) -> i32 {
        match self {
            WindowTint::None => 0,
            WindowTint::Light => 1_000,
            WindowTint::Medium => 2_000,
            WindowTint::Dark => 3_500,
            WindowTint::Mirror => 5_000,
        }
    }
}

// ============================================================================
// 尾翼
// ============================================================================

/// 尾翼類型
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpoilerType {
    /// 無尾翼
    #[default]
    None,
    /// 小型唇翼
    LipSpoiler,
    /// 中型尾翼
    MediumWing,
    /// 大型 GT 尾翼
    GtWing,
}

impl SpoilerType {
    /// 取得名稱
    pub fn name(&self) -> &'static str {
        match self {
            SpoilerType::None => "無尾翼",
            SpoilerType::LipSpoiler => "唇翼",
            SpoilerType::MediumWing => "中型尾翼",
            SpoilerType::GtWing => "GT 尾翼",
        }
    }

    /// 取得價格
    pub fn price(&self) -> i32 {
        match self {
            SpoilerType::None => 0,
            SpoilerType::LipSpoiler => 2_000,
            SpoilerType::MediumWing => 5_000,
            SpoilerType::GtWing => 10_000,
        }
    }
}

// ============================================================================
// 輪框
// ============================================================================

/// 輪框類型
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RimType {
    /// 原廠輪框
    #[default]
    Stock,
    /// 輕量化合金
    Alloy,
    /// 多輻式
    MultiSpoke,
    /// 深唇輪框
    DeepDish,
    /// 碳纖維
    CarbonFiber,
}

impl RimType {
    /// 取得名稱
    pub fn name(&self) -> &'static str {
        match self {
            RimType::Stock => "原廠輪框",
            RimType::Alloy => "合金輪框",
            RimType::MultiSpoke => "多輻輪框",
            RimType::DeepDish => "深唇輪框",
            RimType::CarbonFiber => "碳纖維輪框",
        }
    }

    /// 取得價格
    pub fn price(&self) -> i32 {
        match self {
            RimType::Stock => 0,
            RimType::Alloy => 3_000,
            RimType::MultiSpoke => 5_000,
            RimType::DeepDish => 8_000,
            RimType::CarbonFiber => 15_000,
        }
    }
}

// ============================================================================
// 視覺改裝組件
// ============================================================================

/// 車輛視覺改裝組件
#[derive(Component, Default, Clone, Debug)]
pub struct VehicleVisualMods {
    /// 烤漆顏色
    pub paint: PaintColor,
    /// 車窗貼膜
    pub tint: WindowTint,
    /// 尾翼
    pub spoiler: SpoilerType,
    /// 輪框
    pub rims: RimType,
}

impl VehicleVisualMods {
    /// 計算視覺改裝總價值
    pub fn total_value(&self) -> i32 {
        self.paint.price() + self.tint.price() + self.spoiler.price() + self.rims.price()
    }
}

/// 視覺改裝類別
#[derive(Clone, Debug)]
pub enum VisualModPurchase {
    Paint(PaintColor),
    Tint(WindowTint),
    Spoiler(SpoilerType),
    Rims(RimType),
}

impl VisualModPurchase {
    /// 取得價格
    pub fn price(&self) -> i32 {
        match self {
            VisualModPurchase::Paint(p) => p.price(),
            VisualModPurchase::Tint(t) => t.price(),
            VisualModPurchase::Spoiler(s) => s.price(),
            VisualModPurchase::Rims(r) => r.price(),
        }
    }

    /// 取得名稱
    pub fn name(&self) -> String {
        match self {
            VisualModPurchase::Paint(p) => format!("烤漆: {}", p.name()),
            VisualModPurchase::Tint(t) => format!("貼膜: {}", t.name()),
            VisualModPurchase::Spoiler(s) => format!("尾翼: {}", s.name()),
            VisualModPurchase::Rims(r) => format!("輪框: {}", r.name()),
        }
    }
}

/// 購買視覺改裝事件
#[derive(Message)]
pub struct PurchaseVisualModEvent {
    /// 車輛實體
    pub vehicle: Entity,
    /// 改裝項目
    pub modification: VisualModPurchase,
}
