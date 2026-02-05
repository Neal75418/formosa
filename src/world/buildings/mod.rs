//! 建築系統
//!
//! 處理各種建築類型的生成邏輯

mod commercial;
mod entertainment;
mod generic;
mod services;

pub use commercial::*;
pub use entertainment::*;
pub use generic::*;
pub use services::*;

use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;
use crate::world::{Building, BuildingType};

/// 建築風格枚舉
#[derive(Clone, Copy)]
pub enum BuildingStyle {
    Wannien,
    Donki,
    Eslite,
    ModernGrid,
    Cinema,
    Hotel,
    GameCenter,
    TattooShop,
    FastFood,
    ConvenienceStore,
    StreetWear,
    ClawMachine,
    Daiso,
    Generic,
}

/// 建築基礎參數 (消除重複程式碼用)
pub struct BuildingParams<'a> {
    pub pos: Vec3,
    pub w: f32,
    pub h: f32,
    pub d: f32,
    pub name: &'a str,
}

/// 建築材質設定
pub struct BuildingMaterialConfig {
    pub base_color: Color,
    pub perceptual_roughness: f32,
    pub metallic: f32,
}

impl Default for BuildingMaterialConfig {
    fn default() -> Self {
        Self {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            perceptual_roughness: 0.8,
            metallic: 0.0,
        }
    }
}

/// 建築類型派發（根據名稱關鍵字匹配）
pub fn match_building_type(name: &str) -> BuildingStyle {
    // 定義匹配規則：(關鍵字列表, 建築風格)
    const PATTERNS: &[(&[&str], BuildingStyle)] = &[
        (&["萬年", "Wannien"], BuildingStyle::Wannien),
        (&["Donki", "唐吉"], BuildingStyle::Donki),
        (&["誠品", "Eslite"], BuildingStyle::Eslite),
        (&["H&M", "UNIQLO"], BuildingStyle::ModernGrid),
        (&["Cinema", "影城"], BuildingStyle::Cinema),
        (&["Hotel", "Just Sleep"], BuildingStyle::Hotel),
        (&["湯姆熊", "遊戲", "彈珠台"], BuildingStyle::GameCenter),
        (&["刺青", "TATTOO"], BuildingStyle::TattooShop),
        (&["麥當勞", "摩斯", "肯德基"], BuildingStyle::FastFood),
        (
            &["全家", "7-ELEVEN", "50嵐"],
            BuildingStyle::ConvenienceStore,
        ),
        (&["潮牌", "古著", "球鞋"], BuildingStyle::StreetWear),
        (&["夾娃娃"], BuildingStyle::ClawMachine),
        (&["大創"], BuildingStyle::Daiso),
    ];

    for (keywords, style) in PATTERNS {
        if keywords.iter().any(|kw| name.contains(kw)) {
            return *style;
        }
    }
    BuildingStyle::Generic
}

/// 生成建築基礎結構（消除重複程式碼）
/// 返回 EntityCommands 以供後續添加子實體
pub fn spawn_building_base<'a>(
    cmd: &'a mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    params: &BuildingParams,
    config: BuildingMaterialConfig,
) -> EntityCommands<'a> {
    cmd.spawn((
        Mesh3d(meshes.add(Cuboid::new(params.w, params.h, params.d))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: config.base_color,
            perceptual_roughness: config.perceptual_roughness,
            metallic: config.metallic,
            ..default()
        })),
        Transform::from_translation(params.pos),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        Collider::cuboid(params.w / 2.0, params.h / 2.0, params.d / 2.0),
        Building {
            name: params.name.to_string(),
            building_type: BuildingType::Shop,
        },
    ))
}

/// 通用建築生成函數：根據名稱派發到專屬邏輯
pub fn spawn_rich_building(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    width: f32,
    height: f32,
    depth: f32,
    name: &str,
) {
    match match_building_type(name) {
        BuildingStyle::Wannien => {
            spawn_wannien(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Donki => {
            spawn_donki(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Eslite => {
            spawn_eslite(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::ModernGrid => {
            spawn_modern_grid(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Cinema => {
            spawn_cinema(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Hotel => {
            spawn_hotel(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::GameCenter => {
            spawn_game_center(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::TattooShop => {
            spawn_tattoo_shop(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::FastFood => {
            spawn_fast_food(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::ConvenienceStore => {
            spawn_convenience_store(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::StreetWear => {
            spawn_streetwear_shop(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::ClawMachine => {
            spawn_claw_machine(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Daiso => {
            spawn_daiso(commands, meshes, materials, pos, width, height, depth, name)
        }
        BuildingStyle::Generic => {
            spawn_generic_building(commands, meshes, materials, pos, width, height, depth, name)
        }
    }
}
