//! 建築外牆
//!
//! 用程式畫的窗戶貼圖與統一色盤，套用到掛了 [`FacadeShell`] 的建築主體。
//! 貼圖依每面牆的實際尺寸重複，夜間亮燈沿用 [`BuildingWindow`] 系統（只有窗戶位置會發光）。

use crate::world::{Building, BuildingWindow};
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// 一個開間的寬度（公尺）
pub const BAY_WIDTH: f32 = 3.0;
/// 一層樓的高度（公尺）
pub const FLOOR_HEIGHT: f32 = 3.5;
/// 貼圖橫向包含幾個開間（重複前的變化量）
const TILE_BAYS: u32 = 4;
/// 貼圖縱向包含幾層樓
const TILE_FLOORS: u32 = 4;
/// 每個開間／樓層在貼圖上的像素
const BAY_PX: u32 = 64;
/// 貼圖邊長（像素）
pub const FACADE_TEXTURE_SIZE: u32 = BAY_PX * TILE_BAYS;

/// 貼圖重複一次涵蓋的寬度／高度（公尺）
const TILE_WIDTH: f32 = BAY_WIDTH * TILE_BAYS as f32;
const TILE_HEIGHT: f32 = FLOOR_HEIGHT * TILE_FLOORS as f32;
/// 屋頂取樣點：第一個開間左上角的牆面像素
const ROOF_UV: [f32; 2] = [
    2.0 / FACADE_TEXTURE_SIZE as f32,
    2.0 / FACADE_TEXTURE_SIZE as f32,
];

/// 外牆粗糙度：水泥／磁磚牆是霧面，不能沿用各風格原本偏亮的玻璃、金屬質感
pub const WALL_ROUGHNESS: f32 = 0.85;

/// 台北公寓牆面色盤（低飽和）
pub const FACADE_PALETTE: [Color; 6] = [
    Color::srgb(0.80, 0.78, 0.74), // 灰白
    Color::srgb(0.78, 0.72, 0.60), // 米黃
    Color::srgb(0.60, 0.60, 0.58), // 水泥灰
    Color::srgb(0.70, 0.56, 0.48), // 淡磚紅
    Color::srgb(0.64, 0.68, 0.64), // 灰綠
    Color::srgb(0.72, 0.66, 0.66), // 粉灰
];

/// 標記：要貼外牆的建築主體
#[derive(Component)]
pub struct FacadeShell;

/// 店名雜湊（FNV-1a 32-bit），用來決定顏色、外型，每次啟動結果都一樣
pub fn name_hash(name: &str) -> u32 {
    name.bytes().fold(0x811C_9DC5, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    })
}

/// 依店名從色盤挑牆面顏色
pub fn facade_palette_color(name: &str) -> Color {
    FACADE_PALETTE[(name_hash(name) % FACADE_PALETTE.len() as u32) as usize]
}

/// 外牆 UV：依面的朝向投影，讓貼圖按實際尺寸重複；屋頂取一個純牆面像素
pub fn facade_uv(position: Vec3, normal: Vec3, min_y: f32) -> [f32; 2] {
    if normal.y.abs() > 0.5 {
        return ROOF_UV;
    }
    let horizontal = if normal.x.abs() > normal.z.abs() {
        position.z
    } else {
        position.x
    };
    // 往上走 v 變小，貼圖的上方才會朝上；從地面起算，一樓不會被切掉
    [horizontal / TILE_WIDTH, (min_y - position.y) / TILE_HEIGHT]
}

/// 外牆貼圖像素（RGBA8，sRGB）
pub struct FacadePixels {
    /// 底色貼圖（牆、窗框、玻璃）
    pub albedo: Vec<u8>,
    /// 夜間發光遮罩（只有亮燈的窗戶玻璃是白的）
    pub emissive: Vec<u8>,
}

const WALL: [u8; 4] = [236, 236, 232, 255];
const SLAB: [u8; 4] = [196, 196, 192, 255];
const FRAME: [u8; 4] = [214, 216, 218, 255];
const GLASS: [u8; 4] = [38, 44, 56, 255];
const SILL: [u8; 4] = [168, 168, 165, 255];
const BARS: [u8; 4] = [92, 94, 98, 255];
const AC_BODY: [u8; 4] = [206, 206, 200, 255];
const AC_GRILLE: [u8; 4] = [140, 140, 136, 255];
const LIT: [u8; 4] = [255, 255, 255, 255];
const UNLIT: [u8; 4] = [0, 0, 0, 255];

/// 開間內的區塊：相對開間左上角的像素範圍 (x0, y0, x1, y1)，不含 x1、y1
type Rect = (u32, u32, u32, u32);
const FRAME_RECT: Rect = (10, 8, 54, 42);
const GLASS_RECT: Rect = (12, 10, 52, 40);
const SILL_RECT: Rect = (8, 42, 56, 45);
const SLAB_RECT: Rect = (0, 60, 64, 64);
const AC_RECT: Rect = (55, 26, 63, 38);
const AC_GRILLE_RECT: Rect = (56, 29, 62, 30);
/// 鐵窗直條的間距（像素）
const BAR_SPACING: usize = 6;

/// 在第 bx 個開間、第 fy 層塗一個矩形
fn fill(buf: &mut [u8], bx: u32, fy: u32, rect: Rect, color: [u8; 4]) {
    let (x0, y0, x1, y1) = rect;
    for y in y0..y1 {
        for x in x0..x1 {
            let px = bx * BAY_PX + x;
            let py = fy * BAY_PX + y;
            let i = ((py * FACADE_TEXTURE_SIZE + px) * 4) as usize;
            buf[i..i + 4].copy_from_slice(&color);
        }
    }
}

/// 畫外牆貼圖：4×4 個開間，每個開間有窗戶，部分加鐵窗、冷氣，部分夜間亮燈
pub fn facade_pixels() -> FacadePixels {
    let texels = (FACADE_TEXTURE_SIZE * FACADE_TEXTURE_SIZE) as usize;
    let mut albedo = WALL.repeat(texels);
    let mut emissive = UNLIT.repeat(texels);

    for fy in 0..TILE_FLOORS {
        for bx in 0..TILE_BAYS {
            // 固定種子：每次畫出來都一樣
            let roll = name_hash(&format!("bay{bx}-{fy}"));
            let lit = roll % 100 < 40;
            let has_bars = (roll >> 8) % 10 < 3;
            let has_ac = (roll >> 16) % 10 < 4;

            fill(&mut albedo, bx, fy, SLAB_RECT, SLAB);
            fill(&mut albedo, bx, fy, FRAME_RECT, FRAME);
            fill(&mut albedo, bx, fy, GLASS_RECT, GLASS);
            fill(&mut albedo, bx, fy, SILL_RECT, SILL);
            if lit {
                fill(&mut emissive, bx, fy, GLASS_RECT, LIT);
            }
            if has_bars {
                let (x0, y0, x1, y1) = GLASS_RECT;
                for x in (x0..x1).step_by(BAR_SPACING) {
                    fill(&mut albedo, bx, fy, (x, y0, x + 1, y1), BARS);
                    fill(&mut emissive, bx, fy, (x, y0, x + 1, y1), UNLIT);
                }
            }
            if has_ac {
                fill(&mut albedo, bx, fy, AC_RECT, AC_BODY);
                fill(&mut albedo, bx, fy, AC_GRILLE_RECT, AC_GRILLE);
            }
        }
    }

    FacadePixels { albedo, emissive }
}

/// 外牆貼圖資源
#[derive(Resource)]
pub struct FacadeTextures {
    pub albedo: Handle<Image>,
    pub emissive: Handle<Image>,
}

impl FromWorld for FacadeTextures {
    fn from_world(world: &mut World) -> Self {
        let pixels = facade_pixels();
        let mut images = world.resource_mut::<Assets<Image>>();
        Self {
            albedo: images.add(facade_image(pixels.albedo)),
            emissive: images.add(facade_image(pixels.emissive)),
        }
    }
}

/// 貼圖的 mip 層數：256 → 128 → … → 1
const FACADE_MIP_LEVELS: u32 = FACADE_TEXTURE_SIZE.trailing_zeros() + 1;

/// 把 size×size 的 RGBA8 縮成一半（每 2×2 個像素取平均）
fn downsample_rgba(src: &[u8], size: u32) -> Vec<u8> {
    let half = size / 2;
    let mut out = Vec::with_capacity((half * half * 4) as usize);
    for y in 0..half {
        for x in 0..half {
            for c in 0..4 {
                let at = |dx: u32, dy: u32| {
                    u32::from(src[(((2 * y + dy) * size + 2 * x + dx) * 4 + c) as usize])
                };
                let sum = at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1);
                out.push(((sum + 2) / 4) as u8);
            }
        }
    }
    out
}

/// 原圖接上所有縮小層。Bevy 不會在執行期自動產生 mip，
/// 沒有的話遠處、斜看的牆面在鏡頭移動時會閃爍、出現摩爾紋
fn with_mip_chain(base: &[u8]) -> Vec<u8> {
    let mut all = base.to_vec();
    let mut level = base.to_vec();
    let mut size = FACADE_TEXTURE_SIZE;
    while size > 1 {
        level = downsample_rgba(&level, size);
        size /= 2;
        all.extend_from_slice(&level);
    }
    all
}

/// 把像素包成可重複貼的貼圖
fn facade_image(data: Vec<u8>) -> Image {
    let chain = with_mip_chain(&data);
    // Image::new 要求資料剛好一層，建好後再換成完整的 mip 鏈
    let mut image = Image::new(
        Extent3d {
            width: FACADE_TEXTURE_SIZE,
            height: FACADE_TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = FACADE_MIP_LEVELS;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        // 街道窄、牆面多半斜著看，異向過濾避免遠處窗戶糊成一片
        anisotropy_clamp: 16,
        ..default()
    });
    image
}

/// 依外牆投影規則重寫 mesh 的 UV（座標用 mesh 本身的區域座標）
fn apply_facade_uvs(mesh: &mut Mesh) {
    let Some(positions) = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(VertexAttributeValues::as_float3)
    else {
        return;
    };
    let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        return;
    };
    let min_y = positions.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
    let uvs: Vec<[f32; 2]> = positions
        .iter()
        .zip(normals)
        .map(|(p, n)| facade_uv(Vec3::from(*p), Vec3::from(*n), min_y))
        .collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

/// 替剛生成、掛了 [`FacadeShell`] 的建築套上外牆貼圖、色盤，並接上夜間亮燈
pub fn apply_building_facades(
    mut commands: Commands,
    shells: Query<
        (
            Entity,
            &Building,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
        ),
        Added<FacadeShell>,
    >,
    children: Query<&Children>,
    parts: Query<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>,
    textures: Res<FacadeTextures>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, building, mesh, material) in &shells {
        let Some(mat) = materials.get_mut(&material.0) else {
            continue;
        };
        mat.base_color = facade_palette_color(&building.name);
        mat.base_color_texture = Some(textures.albedo.clone());
        mat.emissive_texture = Some(textures.emissive.clone());
        mat.emissive = LinearRgba::BLACK;
        mat.perceptual_roughness = WALL_ROUGHNESS;
        mat.metallic = 0.0;

        if let Some(m) = meshes.get_mut(&mesh.0) {
            apply_facade_uvs(m);
        }
        // 共用主體材質的子物件（例如階梯樓的上層）也要投影，否則貼圖會被拉伸
        for child in children.iter_descendants(entity) {
            if let Ok((child_mesh, child_mat)) = parts.get(child) {
                if child_mat.0 == material.0 {
                    if let Some(m) = meshes.get_mut(&child_mesh.0) {
                        apply_facade_uvs(m);
                    }
                }
            }
        }

        // 夜間亮燈：既有的窗戶系統只調整發光強度，遮罩決定哪些窗戶會亮
        commands
            .entity(entity)
            .insert(BuildingWindow::residential());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 窗戶玻璃中心的像素座標（第 bx 個開間、第 fy 層）
    fn glass_center(bx: u32, fy: u32) -> (u32, u32) {
        (bx * BAY_PX + BAY_PX / 2, fy * BAY_PX + BAY_PX / 3)
    }

    fn pixel(buf: &[u8], x: u32, y: u32) -> [u8; 4] {
        let i = ((y * FACADE_TEXTURE_SIZE + x) * 4) as usize;
        [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
    }

    fn luminance(p: [u8; 4]) -> u32 {
        u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
    }

    /// 只掛資產、不開視窗的最小 App，跑外牆系統
    fn facade_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<Image>()
            .init_resource::<FacadeTextures>()
            .add_systems(Update, apply_building_facades);
        app
    }

    fn building(name: &str) -> Building {
        Building {
            name: name.to_string(),
            building_type: crate::world::BuildingType::Shop,
        }
    }

    /// 依外牆投影規則算出某個 mesh 每個頂點應有的 UV
    fn expected_uvs(mesh: &Mesh) -> Vec<[f32; 2]> {
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("mesh 沒有法線");
        };
        let min_y = positions.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        positions
            .iter()
            .zip(normals)
            .map(|(p, n)| facade_uv(Vec3::from(*p), Vec3::from(*n), min_y))
            .collect()
    }

    fn uvs(mesh: &Mesh) -> Vec<[f32; 2]> {
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("mesh 沒有 UV");
        };
        uv.clone()
    }

    #[test]
    fn apply_facades_textures_marked_shell_and_shared_material_children() {
        let mut app = facade_app();
        let (shell_mesh, upper_mesh, mat, sign_mesh, sign_mat) = {
            let world = app.world_mut();
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            let shell_mesh = meshes.add(Cuboid::new(6.0, 7.0, 2.0));
            let upper_mesh = meshes.add(Cuboid::new(3.0, 3.5, 1.0));
            let sign_mesh = meshes.add(Cuboid::new(2.0, 1.0, 0.2));
            let mut mats = world.resource_mut::<Assets<StandardMaterial>>();
            // 某些風格原本是光滑、帶金屬感的材質，套外牆後要變霧面
            let mat = mats.add(StandardMaterial {
                perceptual_roughness: 0.2,
                metallic: 0.3,
                ..default()
            });
            let sign_mat = mats.add(StandardMaterial::default());
            (shell_mesh, upper_mesh, mat, sign_mesh, sign_mat)
        };
        let shell = app
            .world_mut()
            .spawn((
                building("測試大樓"),
                FacadeShell,
                Mesh3d(shell_mesh.clone()),
                MeshMaterial3d(mat.clone()),
            ))
            .with_children(|p| {
                // 共用主體材質的上層（例如階梯樓）
                p.spawn((Mesh3d(upper_mesh.clone()), MeshMaterial3d(mat.clone())));
                // 自己材質的招牌：不能被動到
                p.spawn((Mesh3d(sign_mesh.clone()), MeshMaterial3d(sign_mat.clone())));
            })
            .id();
        let sign_uvs_before = uvs(app
            .world()
            .resource::<Assets<Mesh>>()
            .get(&sign_mesh)
            .unwrap());

        app.update();

        let world = app.world();
        let tex = world.resource::<FacadeTextures>();
        let m = world
            .resource::<Assets<StandardMaterial>>()
            .get(&mat)
            .unwrap();
        assert_eq!(m.base_color_texture.as_ref(), Some(&tex.albedo));
        assert_eq!(m.emissive_texture.as_ref(), Some(&tex.emissive));
        assert_eq!(m.base_color, facade_palette_color("測試大樓"));
        assert_eq!(m.perceptual_roughness, WALL_ROUGHNESS);
        assert_eq!(m.metallic, 0.0);
        let meshes = world.resource::<Assets<Mesh>>();
        for h in [&shell_mesh, &upper_mesh] {
            let mesh = meshes.get(h).unwrap();
            assert_eq!(uvs(mesh), expected_uvs(mesh));
        }
        assert_eq!(uvs(meshes.get(&sign_mesh).unwrap()), sign_uvs_before);
        let sign = world
            .resource::<Assets<StandardMaterial>>()
            .get(&sign_mat)
            .unwrap();
        assert!(sign.base_color_texture.is_none());
        assert!(world.get::<BuildingWindow>(shell).is_some());
    }

    #[test]
    fn apply_facades_skips_buildings_without_shell_marker() {
        let mut app = facade_app();
        let (mesh, mat) = {
            let world = app.world_mut();
            let mesh = world
                .resource_mut::<Assets<Mesh>>()
                .add(Cuboid::new(6.0, 7.0, 2.0));
            let mat = world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial::default());
            (mesh, mat)
        };
        // 例如停車場：有 Building 但沒有 FacadeShell
        let garage = app
            .world_mut()
            .spawn((
                building("停車場"),
                Mesh3d(mesh),
                MeshMaterial3d(mat.clone()),
            ))
            .id();

        app.update();

        let m = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&mat)
            .unwrap();
        assert!(m.base_color_texture.is_none());
        assert!(app.world().get::<BuildingWindow>(garage).is_none());
    }

    #[test]
    fn facade_textures_are_registered_and_repeat() {
        let app = facade_app();
        let world = app.world();
        let tex = world.resource::<FacadeTextures>();
        let images = world.resource::<Assets<Image>>();
        for h in [&tex.albedo, &tex.emissive] {
            let image = images.get(h).expect("貼圖要加進 Assets<Image>");
            assert_eq!(image.width(), FACADE_TEXTURE_SIZE);
            let bevy::image::ImageSampler::Descriptor(desc) = &image.sampler else {
                panic!("要自訂 sampler 才能重複貼圖");
            };
            assert_eq!(desc.address_mode_u, bevy::image::ImageAddressMode::Repeat);
            assert_eq!(desc.address_mode_v, bevy::image::ImageAddressMode::Repeat);
        }
    }

    #[test]
    fn downsample_averages_two_by_two_blocks() {
        // 2×2：黑、白、黑、白 → 1×1 的灰
        let src = [
            0, 0, 0, 255, 255, 255, 255, 255, //
            0, 0, 0, 255, 255, 255, 255, 255,
        ];
        assert_eq!(downsample_rgba(&src, 2), vec![128, 128, 128, 255]);
    }

    #[test]
    fn downsample_keeps_quadrants_in_place() {
        // 4×4 的四個象限各一色 → 2×2 每格對應原本的象限（抓列寬、x／y 對調之類的索引錯誤）
        const RED: [u8; 4] = [255, 0, 0, 255];
        const GREEN: [u8; 4] = [0, 255, 0, 255];
        const BLUE: [u8; 4] = [0, 0, 255, 255];
        const WHITE: [u8; 4] = [255, 255, 255, 255];
        let mut src = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let color = match (x < 2, y < 2) {
                    (true, true) => RED,
                    (false, true) => GREEN,
                    (true, false) => BLUE,
                    (false, false) => WHITE,
                };
                src.extend_from_slice(&color);
            }
        }
        assert_eq!(downsample_rgba(&src, 4), [RED, GREEN, BLUE, WHITE].concat());
    }

    #[test]
    fn facade_textures_have_full_mip_chain() {
        let app = facade_app();
        let world = app.world();
        let tex = world.resource::<FacadeTextures>();
        let images = world.resource::<Assets<Image>>();
        let expected_len: usize = (0..FACADE_MIP_LEVELS)
            .map(|level| ((FACADE_TEXTURE_SIZE >> level).pow(2) * 4) as usize)
            .sum();
        for h in [&tex.albedo, &tex.emissive] {
            let image = images.get(h).unwrap();
            assert_eq!(image.texture_descriptor.mip_level_count, 9);
            assert_eq!(image.data.as_ref().unwrap().len(), expected_len);
            let bevy::image::ImageSampler::Descriptor(desc) = &image.sampler else {
                panic!("要自訂 sampler");
            };
            assert_eq!(desc.mipmap_filter, bevy::image::ImageFilterMode::Linear);
            assert_eq!(desc.anisotropy_clamp, 16);
        }
    }

    #[test]
    fn name_hash_is_fnv1a() {
        // FNV-1a 32-bit 的已知值：不依賴 std 的雜湊（那個不保證跨版本穩定）
        assert_eq!(name_hash(""), 0x811C_9DC5);
        assert_eq!(name_hash("a"), 0xE40C_292C);
    }

    #[test]
    fn palette_color_is_stable_and_from_palette() {
        let c = facade_palette_color("麥當勞");
        assert_eq!(c, facade_palette_color("麥當勞"));
        assert!(FACADE_PALETTE.contains(&c));
    }

    #[test]
    fn palette_spreads_across_names() {
        let names = [
            "麥當勞",
            "全家便利",
            "摩斯漢堡",
            "獅子林",
            "萬年",
            "誠品武昌",
            "紅樓",
            "鴨肉扁",
        ];
        let mut used: Vec<Color> = Vec::new();
        for n in names {
            let c = facade_palette_color(n);
            if !used.contains(&c) {
                used.push(c);
            }
        }
        assert!(used.len() >= 3, "8 個店名只用到 {} 種顏色", used.len());
    }

    #[test]
    fn facade_uv_front_face_tiles_by_world_size() {
        let n = Vec3::Z;
        // 往右 6 公尺 = 2 個開間 = 半張貼圖
        assert_eq!(facade_uv(Vec3::new(6.0, 0.0, 1.0), n, 0.0), [0.5, 0.0]);
        // 往上 7 公尺 = 2 層樓 = 半張貼圖（往上走 v 變小，貼圖的上方朝上）
        assert_eq!(facade_uv(Vec3::new(0.0, 7.0, 1.0), n, 0.0), [0.0, -0.5]);
    }

    #[test]
    fn facade_uv_side_face_uses_depth_axis() {
        // 側面（法線朝 X）用 z 座標決定橫向位置
        assert_eq!(
            facade_uv(Vec3::new(5.0, 0.0, 3.0), Vec3::X, 0.0),
            [0.25, 0.0]
        );
    }

    #[test]
    fn facade_uv_floors_follow_building_height() {
        // 20 公尺高的樓，從地面到屋頂經過 20 / 3.5 層
        let h = 20.0;
        let bottom = facade_uv(Vec3::new(0.0, -h / 2.0, 1.0), Vec3::Z, -h / 2.0);
        let top = facade_uv(Vec3::new(0.0, h / 2.0, 1.0), Vec3::Z, -h / 2.0);
        let floors = (bottom[1] - top[1]) * TILE_FLOORS as f32;
        assert!(
            (floors - h / FLOOR_HEIGHT).abs() < 1e-4,
            "floors = {floors}"
        );
    }

    #[test]
    fn facade_uv_roof_is_constant_wall_texel() {
        let a = facade_uv(Vec3::new(1.0, 10.0, 2.0), Vec3::Y, 0.0);
        let b = facade_uv(Vec3::new(-7.0, 10.0, 5.0), Vec3::Y, 0.0);
        assert_eq!(a, b);
        // 屋頂取樣的像素必須是牆，不是窗
        let px = facade_pixels();
        let x = (a[0] * FACADE_TEXTURE_SIZE as f32) as u32;
        let y = (a[1] * FACADE_TEXTURE_SIZE as f32) as u32;
        let (gx, gy) = glass_center(0, 0);
        assert!(luminance(pixel(&px.albedo, x, y)) > luminance(pixel(&px.albedo, gx, gy)));
    }

    #[test]
    fn facade_pixels_have_texture_size() {
        let px = facade_pixels();
        let len = (FACADE_TEXTURE_SIZE * FACADE_TEXTURE_SIZE * 4) as usize;
        assert_eq!(px.albedo.len(), len);
        assert_eq!(px.emissive.len(), len);
    }

    #[test]
    fn facade_glass_is_darker_than_wall() {
        let px = facade_pixels();
        let (gx, gy) = glass_center(1, 2);
        let wall = pixel(&px.albedo, 1, 1);
        assert!(luminance(pixel(&px.albedo, gx, gy)) * 3 < luminance(wall));
    }

    #[test]
    fn emissive_only_some_windows_lit_and_never_on_wall() {
        let px = facade_pixels();
        let mut lit = 0;
        for fy in 0..TILE_FLOORS {
            for bx in 0..TILE_BAYS {
                let (gx, gy) = glass_center(bx, fy);
                if luminance(pixel(&px.emissive, gx, gy)) > 0 {
                    lit += 1;
                }
                // 開間左上角是牆，不能發光
                assert_eq!(
                    luminance(pixel(&px.emissive, bx * BAY_PX + 1, fy * BAY_PX + 1)),
                    0
                );
            }
        }
        let total = TILE_BAYS * TILE_FLOORS;
        assert!(lit > 0 && lit < total, "亮燈窗戶 {lit}/{total}");
    }
}
