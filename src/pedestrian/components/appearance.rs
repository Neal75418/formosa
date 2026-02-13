//! 行人外觀配置

use bevy::prelude::*;

/// 行人外觀配置
#[derive(Clone, Debug)]
pub struct PedestrianAppearance {
    pub skin_color: Color,
    pub shirt_color: Color,
    pub pants_color: Color,
    pub shoe_color: Color,
    pub hair_color: Color,
}

impl PedestrianAppearance {
    /// 隨機生成休閒風格外觀
    pub fn random_casual() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();

        // 膚色變化
        let skin_tone = rng.random_range(0.6..0.9);
        let skin_color = Color::srgb(skin_tone, skin_tone * 0.8, skin_tone * 0.7);

        // 隨機上衣顏色
        let shirt_colors = [
            Color::srgb(0.2, 0.3, 0.6),   // 藍色
            Color::srgb(0.6, 0.2, 0.2),   // 紅色
            Color::srgb(0.2, 0.5, 0.3),   // 綠色
            Color::srgb(0.8, 0.8, 0.8),   // 白色
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.6, 0.5, 0.2),   // 黃褐色
            Color::srgb(0.5, 0.3, 0.5),   // 紫色
        ];
        let shirt_color = shirt_colors[rng.random_range(0..shirt_colors.len())];

        // 隨機褲子顏色
        let pants_colors = [
            Color::srgb(0.1, 0.1, 0.2),   // 深藍牛仔
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.4, 0.35, 0.3),  // 卡其色
            Color::srgb(0.3, 0.3, 0.3),   // 灰色
        ];
        let pants_color = pants_colors[rng.random_range(0..pants_colors.len())];

        // 鞋子顏色
        let shoe_colors = [
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.8, 0.8, 0.8),   // 白色
            Color::srgb(0.4, 0.2, 0.1),   // 棕色
        ];
        let shoe_color = shoe_colors[rng.random_range(0..shoe_colors.len())];

        // 頭髮顏色
        let hair_colors = [
            Color::srgb(0.05, 0.05, 0.05), // 黑色
            Color::srgb(0.2, 0.1, 0.05),   // 深棕
            Color::srgb(0.4, 0.3, 0.2),    // 棕色
        ];
        let hair_color = hair_colors[rng.random_range(0..hair_colors.len())];

        Self {
            skin_color,
            shirt_color,
            pants_color,
            shoe_color,
            hair_color,
        }
    }

    /// 生成上班族風格
    pub fn random_business() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();

        let skin_tone = rng.random_range(0.6..0.9);
        let skin_color = Color::srgb(skin_tone, skin_tone * 0.8, skin_tone * 0.7);

        // 上班族：白襯衫或淺色襯衫
        let shirt_colors = [
            Color::srgb(0.9, 0.9, 0.9),   // 白色
            Color::srgb(0.7, 0.8, 0.9),   // 淺藍
            Color::srgb(0.9, 0.85, 0.8),  // 米色
        ];
        let shirt_color = shirt_colors[rng.random_range(0..shirt_colors.len())];

        // 深色西褲
        let pants_colors = [
            Color::srgb(0.1, 0.1, 0.15),  // 深藍
            Color::srgb(0.1, 0.1, 0.1),   // 黑色
            Color::srgb(0.25, 0.25, 0.25),// 深灰
        ];
        let pants_color = pants_colors[rng.random_range(0..pants_colors.len())];

        Self {
            skin_color,
            shirt_color,
            pants_color,
            shoe_color: Color::srgb(0.1, 0.1, 0.1),
            hair_color: Color::srgb(0.05, 0.05, 0.05),
        }
    }
}
