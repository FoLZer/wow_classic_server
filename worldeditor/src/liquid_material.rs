use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat},
    shader::ShaderRef,
};
use wow_adt::chunks::mcnk::LiquidType;
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use wow_mpq::PatchChain;

use crate::mpq_read_file;

const LIQUID_FRAME_COUNT: u32 = 30;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LiquidMaterial {
    #[texture(100, dimension = "2d_array")]
    #[sampler(101)]
    pub frames: Handle<Image>,
    #[uniform(102)]
    pub frame_count: u32,
    #[uniform(103)]
    pub add_base_color: u32,
    #[uniform(104)]
    pub uv_scroll: Vec2,
}

impl MaterialExtension for LiquidMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/liquid_shader.wesl".into()
    }
}

#[derive(Clone)]
pub(crate) struct LiquidTexture {
    pub(crate) handle: Handle<Image>,
    pub(crate) frame_count: u32,
}

pub(crate) fn load_liquid_texture(
    liquid_type: LiquidType,
    mpqs: &PatchChain,
    images: &mut Assets<Image>,
) -> LiquidTexture {
    let prefix = match liquid_type {
        LiquidType::Water => "XTextures\\river\\lake_a",
        LiquidType::Ocean => "XTextures\\ocean\\ocean_h",
        LiquidType::Magma => "XTextures\\lava\\lava",
        LiquidType::Slime => "XTextures\\slime\\slime",
    };
    let mut frame_data = Vec::new();
    let mut dimensions = None;

    for frame in 1..=LIQUID_FRAME_COUNT {
        let path = format!("{prefix}.{frame}.blp");
        let Ok(data) = mpq_read_file(mpqs, &path) else {
            if frame == 1 {
                warn!("Classic liquid texture sequence was not found: {path}");
            }
            break;
        };
        let Ok(blp) = load_blp_from_buf(&data) else {
            warn!("Unable to parse Classic liquid texture {path}");
            break;
        };
        let frame_dimensions = (blp.header.width, blp.header.height);
        if dimensions.is_some_and(|dimensions| dimensions != frame_dimensions) {
            warn!("Classic liquid texture sequence changes dimensions at {path}");
            break;
        }
        let Ok(decoded) = blp_to_image(&blp, 0) else {
            warn!("Unable to decode Classic liquid texture {path}");
            break;
        };
        dimensions = Some(frame_dimensions);
        frame_data.extend_from_slice(&decoded.into_rgba8().into_vec());
    }

    let (width, height) = dimensions.unwrap_or((1, 1));
    let frame_count = if frame_data.is_empty() {
        frame_data.extend_from_slice(&fallback_color(liquid_type));
        1
    } else {
        (frame_data.len() / (width * height * 4) as usize) as u32
    };
    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: frame_count,
        },
        TextureDimension::D2,
        if matches!(liquid_type, LiquidType::Water | LiquidType::Ocean) {
            TextureFormat::Rgba8Unorm
        } else {
            TextureFormat::Rgba8UnormSrgb
        },
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(frame_data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });

    LiquidTexture {
        handle: images.add(image),
        frame_count,
    }
}

fn fallback_color(liquid_type: LiquidType) -> [u8; 4] {
    match liquid_type {
        LiquidType::Water => [45, 112, 145, 255],
        LiquidType::Ocean => [35, 83, 125, 255],
        LiquidType::Magma => [255, 92, 18, 255],
        LiquidType::Slime => [70, 132, 42, 255],
    }
}
