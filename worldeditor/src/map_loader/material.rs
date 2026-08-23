use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use rayon::prelude::*;
use wow_adt::RootAdt;
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use wow_mpq::PatchChain;

use crate::{combined_alpha_map::CombinedAlphaMap, mpq_read_file};

use super::ADT_CELLS_PER_GRID;

const SOURCE_ALPHA_MAP_SIZE: usize = 64;
const ALPHA_MAP_SIZE: usize = 16;

pub(super) struct CachedTerrainTexture {
    layer: u32,
    width: u32,
    height: u32,
    mipmaps: Vec<Vec<u8>>,
}

pub(super) struct PreparedMaterialMaps {
    pub(super) alpha_map: Image,
    pub(super) local_layers: Vec<u16>,
    pub(super) animation_map: Image,
}

pub(super) fn prepare_material_maps(adt: &RootAdt, has_big_alpha: bool) -> PreparedMaterialMaps {
    const ATLAS_SIZE: usize = ADT_CELLS_PER_GRID * ALPHA_MAP_SIZE;
    const DIRECTIONS: [IVec2; 8] = [
        IVec2::new(0, 1),
        IVec2::new(-1, 1),
        IVec2::new(-1, 0),
        IVec2::new(-1, -1),
        IVec2::new(0, -1),
        IVec2::new(1, -1),
        IVec2::new(1, 0),
        IVec2::new(1, 1),
    ];

    let mut alpha_data = vec![0; ATLAS_SIZE * ATLAS_SIZE * 4];
    let mut layer_values = vec![0_u16; ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID * 4];
    let mut animation_data = vec![0; ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID * 4];

    let chunk_materials = adt
        .mcnk_chunks
        .par_iter()
        .map(|chunk| {
            let chunk_alpha = CombinedAlphaMap::new(
                chunk,
                has_big_alpha,
                !chunk.header.flags.do_not_fix_alpha_map(),
            )
            .into_vec();
            let fix_alpha = u8::from(chunk.header.flags.do_not_fix_alpha_map()) * u8::MAX;
            let mut alpha = vec![0; ALPHA_MAP_SIZE * ALPHA_MAP_SIZE * 4];
            for row in 0..ALPHA_MAP_SIZE {
                for column in 0..ALPHA_MAP_SIZE {
                    let target = (row * ALPHA_MAP_SIZE + column) * 4;
                    let source_scale = SOURCE_ALPHA_MAP_SIZE / ALPHA_MAP_SIZE;
                    let source_row = row * source_scale + source_scale / 2;
                    let source_column = column * source_scale + source_scale / 2;
                    for channel in 0..3 {
                        let source =
                            ((source_row * SOURCE_ALPHA_MAP_SIZE + source_column) * 4) + channel;
                        alpha[target + channel] = chunk_alpha[source];
                    }
                    alpha[target + 3] = fix_alpha;
                }
            }

            let mut layers = [0_u16; 4];
            let mut animations = [0_u8; 4];
            for layer_index in 0..4 {
                let Some(layer) = chunk
                    .layers
                    .as_ref()
                    .and_then(|layers| layers.layers.get(layer_index))
                else {
                    continue;
                };
                layers[layer_index] = layer.texture_id as u16 + 1;

                if layer.flags.animation_enabled() {
                    let direction = DIRECTIONS[layer.flags.animation_rotation() as usize];
                    let speed = layer.flags.animation_speed().min(15) as u8;
                    animations[layer_index] =
                        (speed << 4) | (((direction.x + 1) as u8) << 2) | (direction.y + 1) as u8;
                }
            }

            (alpha, layers, animations)
        })
        .collect::<Vec<_>>();

    for chunk_x in 0..ADT_CELLS_PER_GRID {
        for chunk_y in 0..ADT_CELLS_PER_GRID {
            let chunk_index = chunk_x * ADT_CELLS_PER_GRID + chunk_y;
            let (alpha, layers, animations) = &chunk_materials[chunk_index];
            for row in 0..ALPHA_MAP_SIZE {
                let source_start = row * ALPHA_MAP_SIZE * 4;
                let target_start =
                    ((chunk_x * ALPHA_MAP_SIZE + row) * ATLAS_SIZE + chunk_y * ALPHA_MAP_SIZE) * 4;
                alpha_data[target_start..target_start + ALPHA_MAP_SIZE * 4]
                    .copy_from_slice(&alpha[source_start..source_start + ALPHA_MAP_SIZE * 4]);
            }

            let metadata_offset = chunk_index * 4;
            layer_values[metadata_offset..metadata_offset + 4].copy_from_slice(layers);
            animation_data[metadata_offset..metadata_offset + 4].copy_from_slice(animations);
        }
    }

    let mut alpha_map = Image::new(
        Extent3d {
            width: ATLAS_SIZE as u32,
            height: ATLAS_SIZE as u32,
            ..Default::default()
        },
        TextureDimension::D2,
        alpha_data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    alpha_map.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..Default::default()
    });

    let metadata_extent = Extent3d {
        width: ADT_CELLS_PER_GRID as u32,
        height: ADT_CELLS_PER_GRID as u32,
        ..Default::default()
    };
    let animation_map = Image::new(
        metadata_extent,
        TextureDimension::D2,
        animation_data,
        TextureFormat::Rgba8Uint,
        RenderAssetUsages::RENDER_WORLD,
    );

    PreparedMaterialMaps {
        alpha_map,
        local_layers: layer_values,
        animation_map,
    }
}

pub(super) fn global_layer_map(
    local_layers: Vec<u16>,
    adt: &RootAdt,
    texture_cache: &HashMap<String, CachedTerrainTexture>,
) -> Image {
    let layer_data = local_layers
        .into_iter()
        .map(|local_layer| {
            if local_layer == 0 {
                return 0;
            }
            let filepath = &adt.textures[local_layer as usize - 1];
            texture_cache
                .get(filepath)
                .map_or(0, |texture| texture.layer as u16)
        })
        .flat_map(u16::to_le_bytes)
        .collect();
    Image::new(
        Extent3d {
            width: ADT_CELLS_PER_GRID as u32,
            height: ADT_CELLS_PER_GRID as u32,
            ..Default::default()
        },
        TextureDimension::D2,
        layer_data,
        TextureFormat::Rgba16Uint,
        RenderAssetUsages::RENDER_WORLD,
    )
}

pub(super) fn update_texture_array(
    adt: &RootAdt,
    map_name: &str,
    cache: &mut HashMap<String, CachedTerrainTexture>,
    texture_array: &mut Option<Handle<Image>>,
    mpqs: &PatchChain,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let mut changed = false;

    for filepath in &adt.textures {
        if cache.contains_key(filepath) {
            continue;
        }
        let layer = cache.len() as u32 + 1;
        let texture = match mpq_read_file(mpqs, filepath) {
            Ok(file_buf) => {
                let blp = load_blp_from_buf(&file_buf).unwrap();
                CachedTerrainTexture {
                    layer,
                    width: blp.header.width,
                    height: blp.header.height,
                    mipmaps: (0..blp.image_count())
                        .map(|level| blp_to_image(&blp, level).unwrap().into_rgba8().into_vec())
                        .collect(),
                }
            }
            Err(wow_mpq::Error::FileNotFound(_)) => {
                error!("BLP wasn't found for map {map_name}. filepath: {filepath}");
                CachedTerrainTexture {
                    layer,
                    width: 0,
                    height: 0,
                    mipmaps: Vec::new(),
                }
            }
            Err(error) => panic!("{error:?}"),
        };
        cache.insert(filepath.clone(), texture);
        changed = true;
    }

    if !changed && let Some(texture_array) = texture_array.as_ref() {
        return texture_array.clone();
    }

    let mut ordered_textures = cache.values().collect::<Vec<_>>();
    ordered_textures.sort_unstable_by_key(|texture| texture.layer);
    let valid_textures = ordered_textures
        .iter()
        .copied()
        .filter(|texture| texture.width > 0 && !texture.mipmaps.is_empty())
        .collect::<Vec<_>>();
    let width = valid_textures.first().map_or(1, |texture| texture.width);
    let height = valid_textures.first().map_or(1, |texture| texture.height);
    let mip_count = valid_textures
        .iter()
        .filter(|texture| texture.width == width && texture.height == height)
        .map(|texture| texture.mipmaps.len())
        .min()
        .unwrap_or(1);

    let mut data = Vec::new();
    for texture in std::iter::once(None).chain(ordered_textures.into_iter().map(Some)) {
        for level in 0..mip_count {
            let level_width = (width >> level).max(1) as usize;
            let level_height = (height >> level).max(1) as usize;
            if let Some(texture) = texture.filter(|texture| {
                texture.width == width && texture.height == height && texture.mipmaps.len() > level
            }) {
                data.extend_from_slice(&texture.mipmaps[level]);
            } else {
                data.resize(data.len() + level_width * level_height * 4, 0);
            }
        }
    }

    let layer_count = (cache.len() as u32 + 1).max(2);
    data.resize(data.len() * layer_count as usize / (cache.len() + 1), 0);

    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: layer_count,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = mip_count as u32;
    image.data = Some(data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..Default::default()
    });

    match texture_array {
        Some(handle) => {
            *images.get_mut(handle.id()).unwrap() = image;
            handle.clone()
        }
        None => {
            let handle = images.add(image);
            *texture_array = Some(handle.clone());
            handle
        }
    }
}
