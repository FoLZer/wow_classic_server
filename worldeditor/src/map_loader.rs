use std::{collections::HashMap, io::Cursor};

use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::VisibilityRange,
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::{Indices, PrimitiveTopology},
    pbr::ExtendedMaterial,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use wow_adt::{McnkChunk, ParsedAdt, RootAdt, parse_adt};
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use wow_mpq::PatchChain;
use wow_wdt::{WdtFile, WdtReader, chunks::MphdFlags, version::WowVersion};

use crate::{
    MPQResource, combined_alpha_map::CombinedAlphaMap, mpq_read_file,
    terrain_material::TerrainMaterial,
};

const ADT_CELLS_PER_GRID: usize = 16;
const ADT_GRID_SIZE: usize = 64;
const CHUNK_SIZE: f32 = 33.3334;
const ADT_SIZE: f32 = CHUNK_SIZE * ADT_CELLS_PER_GRID as f32;
const ADT_HALF_DIAGONAL: f32 = ADT_SIZE * std::f32::consts::FRAC_1_SQRT_2;
const STREAM_BUFFER: f32 = ADT_SIZE;
const STREAM_UPDATE_DISTANCE: f32 = CHUNK_SIZE * 0.5;
const ADTS_LOADED_PER_FRAME: usize = 1;
const SOURCE_ALPHA_MAP_SIZE: usize = 64;
const ALPHA_MAP_SIZE: usize = 16;

pub fn load_map(mpqs: &mut PatchChain, mut commands: Commands, index: usize, view_distance: f32) {
    let map_dbc = {
        info!("Searching for Map.dbc...");
        let map_buf = mpqs.read_file("DBFilesClient\\Map.dbc").unwrap();

        dbc_reader::read_dbc::<_, dbc_structs::Map>(&mut Cursor::new(map_buf)).unwrap()
    };

    let loaded_map = &map_dbc.get_records()[index];

    let directory = loaded_map.directory.to_str().unwrap();
    info!(
        "Loading map {}. Map name: {}",
        directory,
        loaded_map.map_name_lang.locales[0].to_string_lossy()
    );
    let wdt_file_path = format!("World\\Maps\\{}\\{}.wdt", directory, directory);
    let wdt_file_buf = match mpqs.read_file(&wdt_file_path) {
        Ok(v) => v,
        Err(wow_mpq::Error::FileNotFound(_)) => {
            error!("WDT wasn't found for map {}", directory);
            return;
        }
        Err(e) => {
            panic!("{:?}", e)
        }
    };
    let wdt = WdtReader::new(&mut Cursor::new(wdt_file_buf), WowVersion::Classic)
        .read()
        .unwrap();

    commands.insert_resource(TerrainMap {
        has_big_alpha: wdt.mphd.flags.contains(MphdFlags::ADT_HAS_BIG_ALPHA),
        wdt,
        map_name: directory.to_owned(),
        loaded_adts: HashMap::new(),
        texture_cache: HashMap::new(),
        texture_array: None,
        last_update_position: None,
        loading: true,
        view_distance,
    });
}

struct LoadedAdt {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    images: [Handle<Image>; 3],
}

struct CachedTerrainTexture {
    layer: u32,
    width: u32,
    height: u32,
    mipmaps: Vec<Vec<u8>>,
}

#[derive(Resource)]
pub struct TerrainMap {
    wdt: WdtFile,
    map_name: String,
    has_big_alpha: bool,
    loaded_adts: HashMap<(usize, usize), LoadedAdt>,
    texture_cache: HashMap<String, CachedTerrainTexture>,
    texture_array: Option<Handle<Image>>,
    last_update_position: Option<Vec2>,
    loading: bool,
    view_distance: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainAdt {
    pub x: u8,
    pub y: u8,
}

pub fn stream_terrain_chunks(
    mut commands: Commands,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut terrain: ResMut<TerrainMap>,
    mut mpqs: ResMut<MPQResource>,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(camera_transform) = camera.single() else {
        return;
    };
    let camera_position = camera_transform.translation().xz();

    let camera_moved = terrain.last_update_position.is_none_or(|position| {
        position.distance_squared(camera_position) >= STREAM_UPDATE_DISTANCE.powi(2)
    });
    if !camera_moved && !terrain.loading {
        return;
    }
    if camera_moved {
        terrain.last_update_position = Some(camera_position);
        terrain.loading = true;
    }

    let load_distance = terrain.view_distance + ADT_HALF_DIAGONAL;
    let unload_distance_squared = (load_distance + STREAM_BUFFER).powi(2);
    let mut adts_to_load = Vec::new();
    for y in 0..ADT_GRID_SIZE {
        for x in 0..ADT_GRID_SIZE {
            if !terrain.wdt.main.entries[y][x].has_adt()
                || terrain.loaded_adts.contains_key(&(x, y))
            {
                continue;
            }
            let distance_squared = adt_center(x, y).distance_squared(camera_position);
            if distance_squared <= load_distance.powi(2) {
                adts_to_load.push((distance_squared, x, y));
            }
        }
    }
    adts_to_load.sort_unstable_by(|left, right| left.0.total_cmp(&right.0));

    let adts_to_unload = terrain
        .loaded_adts
        .iter()
        .filter_map(|(&(x, y), _)| {
            (adt_center(x, y).distance_squared(camera_position) > unload_distance_squared)
                .then_some((x, y))
        })
        .collect::<Vec<_>>();

    for coordinates in adts_to_unload {
        let loaded_adt = terrain.loaded_adts.remove(&coordinates).unwrap();
        commands.entity(loaded_adt.entity).despawn();
        meshes.remove(loaded_adt.mesh.id());
        terrain_materials.remove(loaded_adt.material.id());
        for image in loaded_adt.images {
            images.remove(image.id());
        }
    }

    terrain.loading = adts_to_load.len() > ADTS_LOADED_PER_FRAME;
    for (_, x, y) in adts_to_load.into_iter().take(ADTS_LOADED_PER_FRAME) {
        let map_path = format!(
            "World\\Maps\\{}\\{}_{}_{}.adt",
            terrain.map_name, terrain.map_name, x, y
        );
        let map_file_buf = mpqs.mpqs.read_file(&map_path).unwrap();
        let adt = parse_adt(&mut Cursor::new(map_file_buf)).unwrap();
        let ParsedAdt::Root(adt) = adt else { panic!() };
        let center = adt_center(x, y);
        let mesh = meshes.add(adt_to_mesh(&adt, center));
        let map_name = terrain.map_name.clone();
        let texture_array = {
            let TerrainMap {
                texture_cache,
                texture_array,
                ..
            } = &mut *terrain;
            update_texture_array(
                &adt,
                &map_name,
                texture_cache,
                texture_array,
                &mut mpqs.mpqs,
                &mut images,
            )
        };
        let (alpha_map, layer_map, animation_map) =
            adt_material_maps(&adt, terrain.has_big_alpha, &terrain.texture_cache);
        let alpha_map = images.add(alpha_map);
        let layer_map = images.add(layer_map);
        let animation_map = images.add(animation_map);
        let material = terrain_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                ..Default::default()
            },
            extension: TerrainMaterial {
                textures: texture_array.clone(),
                alpha_map: alpha_map.clone(),
                layer_map: layer_map.clone(),
                animation_map: animation_map.clone(),
            },
        });
        let entity = commands
            .spawn((
                TerrainAdt {
                    x: x as u8,
                    y: y as u8,
                },
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(center.x, 0.0, center.y),
                VisibilityRange {
                    use_aabb: true,
                    ..VisibilityRange::abrupt(0.0, terrain.view_distance)
                },
            ))
            .id();
        terrain.loaded_adts.insert(
            (x, y),
            LoadedAdt {
                entity,
                mesh,
                material,
                images: [alpha_map, layer_map, animation_map],
            },
        );
    }
}

fn adt_center(x: usize, y: usize) -> Vec2 {
    Vec2::new((31.5 - x as f32) * ADT_SIZE, (31.5 - y as f32) * ADT_SIZE)
}

fn adt_material_maps(
    adt: &RootAdt,
    has_big_alpha: bool,
    texture_cache: &HashMap<String, CachedTerrainTexture>,
) -> (Image, Image, Image) {
    const ATLAS_SIZE: usize = ADT_CELLS_PER_GRID * ALPHA_MAP_SIZE;
    const DIRECTIONS: [IVec2; 8] = [
        IVec2::new(0, 1),
        IVec2::new(1, 1),
        IVec2::new(1, 0),
        IVec2::new(1, -1),
        IVec2::new(0, -1),
        IVec2::new(-1, -1),
        IVec2::new(-1, 0),
        IVec2::new(-1, 1),
    ];

    let mut alpha_data = vec![0; ATLAS_SIZE * ATLAS_SIZE * 4];
    let mut layer_values = vec![0_u16; ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID * 4];
    let mut animation_data = vec![0; ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID * 4];

    for chunk_x in 0..ADT_CELLS_PER_GRID {
        for chunk_y in 0..ADT_CELLS_PER_GRID {
            let chunk = &adt.mcnk_chunks[chunk_x * ADT_CELLS_PER_GRID + chunk_y];
            let chunk_alpha = CombinedAlphaMap::new(
                chunk,
                has_big_alpha,
                !chunk.header.flags.do_not_fix_alpha_map(),
            )
            .into_vec();
            let fix_alpha = u8::from(chunk.header.flags.do_not_fix_alpha_map()) * u8::MAX;
            for row in 0..ALPHA_MAP_SIZE {
                for column in 0..ALPHA_MAP_SIZE {
                    let target = ((chunk_x * ALPHA_MAP_SIZE + row) * ATLAS_SIZE
                        + chunk_y * ALPHA_MAP_SIZE
                        + column)
                        * 4;
                    let source_scale = SOURCE_ALPHA_MAP_SIZE / ALPHA_MAP_SIZE;
                    let source_row = row * source_scale + source_scale / 2;
                    let source_column = column * source_scale + source_scale / 2;
                    for channel in 0..3 {
                        let source =
                            ((source_row * SOURCE_ALPHA_MAP_SIZE + source_column) * 4) + channel;
                        alpha_data[target + channel] = chunk_alpha[source];
                    }
                    alpha_data[target + 3] = fix_alpha;
                }
            }

            let metadata_offset = (chunk_x * ADT_CELLS_PER_GRID + chunk_y) * 4;
            for layer_index in 0..4 {
                let Some(layer) = chunk
                    .layers
                    .as_ref()
                    .and_then(|layers| layers.layers.get(layer_index))
                else {
                    continue;
                };
                let filepath = &adt.textures[layer.texture_id as usize];
                let texture_layer = texture_cache
                    .get(filepath)
                    .map_or(0, |texture| texture.layer);
                assert!(
                    texture_layer <= u16::MAX as u32,
                    "World has too many textures"
                );
                layer_values[metadata_offset + layer_index] = texture_layer as u16;

                if layer.flags.animation_enabled() {
                    let direction = DIRECTIONS[layer.flags.animation_rotation() as usize];
                    let speed = layer.flags.animation_speed().min(15) as u8;
                    animation_data[metadata_offset + layer_index] =
                        (speed << 4) | (((direction.x + 1) as u8) << 2) | (direction.y + 1) as u8;
                }
            }
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
        RenderAssetUsages::RENDER_WORLD,
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
    let layer_data = layer_values
        .into_iter()
        .flat_map(u16::to_le_bytes)
        .collect();
    let layer_map = Image::new(
        metadata_extent,
        TextureDimension::D2,
        layer_data,
        TextureFormat::Rgba16Uint,
        RenderAssetUsages::RENDER_WORLD,
    );
    let animation_map = Image::new(
        metadata_extent,
        TextureDimension::D2,
        animation_data,
        TextureFormat::Rgba8Uint,
        RenderAssetUsages::RENDER_WORLD,
    );

    (alpha_map, layer_map, animation_map)
}

fn update_texture_array(
    adt: &RootAdt,
    map_name: &str,
    cache: &mut HashMap<String, CachedTerrainTexture>,
    texture_array: &mut Option<Handle<Image>>,
    mpqs: &mut PatchChain,
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

    if !changed {
        return texture_array.as_ref().unwrap().clone();
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

    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: cache.len() as u32 + 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = mip_count as u32;
    image.data = Some(data);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
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

const OVERVIEW_VERTICES_PER_SIDE: usize = 5;
const HEIGHTMAP_SIZE: usize = OVERVIEW_VERTICES_PER_SIDE * OVERVIEW_VERTICES_PER_SIDE;
const INDICES_PER_CHUNK: usize =
    (OVERVIEW_VERTICES_PER_SIDE - 1) * (OVERVIEW_VERTICES_PER_SIDE - 1) * 2 * 3;

struct TerrainChunkGeometry {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u16>,
    position: Vec3,
}

fn adt_to_mesh(adt: &RootAdt, center: Vec2) -> Mesh {
    let chunk_count = ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID;
    let mut vertices = Vec::with_capacity(chunk_count * HEIGHTMAP_SIZE);
    let mut normals = Vec::with_capacity(chunk_count * HEIGHTMAP_SIZE);
    let mut uvs = Vec::with_capacity(chunk_count * HEIGHTMAP_SIZE);
    let mut indices = Vec::with_capacity(chunk_count * INDICES_PER_CHUNK);
    let horizontal_scale = CHUNK_SIZE / 8.0;

    for chunk_x in 0..ADT_CELLS_PER_GRID {
        for chunk_y in 0..ADT_CELLS_PER_GRID {
            let geometry = chunk_geometry(&adt.mcnk_chunks[chunk_x * ADT_CELLS_PER_GRID + chunk_y]);
            let vertex_offset = vertices.len() as u32;
            vertices.extend(geometry.vertices.into_iter().map(|vertex| {
                [
                    geometry.position.x - vertex[0] * horizontal_scale - center.x,
                    geometry.position.y + vertex[1],
                    geometry.position.z - vertex[2] * horizontal_scale - center.y,
                ]
            }));
            normals.extend(geometry.normals.into_iter().map(|normal| {
                Vec3::new(
                    -normal[0] / horizontal_scale,
                    normal[1],
                    -normal[2] / horizontal_scale,
                )
                .normalize_or_zero()
                .to_array()
            }));
            uvs.extend(
                geometry
                    .uvs
                    .into_iter()
                    .map(|uv| [chunk_y as f32 * 8.0 + uv[0], chunk_x as f32 * 8.0 + uv[1]]),
            );
            indices.extend(
                geometry
                    .indices
                    .into_iter()
                    .map(|index| vertex_offset + index as u32),
            );
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}

fn chunk_geometry(chunk: &McnkChunk) -> TerrainChunkGeometry {
    let heights = chunk.heights.as_ref().unwrap();
    let normals_chunk = chunk.normals.as_ref().unwrap();

    let mut vertices = Vec::with_capacity(HEIGHTMAP_SIZE);
    let mut normals = Vec::with_capacity(HEIGHTMAP_SIZE);
    let mut uvs = Vec::with_capacity(HEIGHTMAP_SIZE);
    let mut indices = Vec::with_capacity(INDICES_PER_CHUNK);

    for grid_y in 0..OVERVIEW_VERTICES_PER_SIDE {
        for grid_x in 0..OVERVIEW_VERTICES_PER_SIDE {
            let x = grid_x * 2;
            let y = grid_y * 2;
            let source_index = y * 17 + x;
            vertices.push([x as f32, heights.heights[source_index], y as f32]);
            normals.push([
                -normals_chunk.normals[source_index].z as f32 / 127.0,
                normals_chunk.normals[source_index].y as f32 / 127.0,
                -normals_chunk.normals[source_index].x as f32 / 127.0,
            ]);
            uvs.push([x as f32, y as f32]);
        }
    }

    for y in 0..OVERVIEW_VERTICES_PER_SIDE - 1 {
        for x in 0..OVERVIEW_VERTICES_PER_SIDE - 1 {
            let top_left = (y * OVERVIEW_VERTICES_PER_SIDE + x) as u16;
            let top_right = top_left + 1;
            let bottom_left = top_left + OVERVIEW_VERTICES_PER_SIDE as u16;
            let bottom_right = bottom_left + 1;
            indices.extend_from_slice(&[
                top_left,
                bottom_left,
                top_right,
                top_right,
                bottom_left,
                bottom_right,
            ]);
        }
    }

    TerrainChunkGeometry {
        vertices,
        normals,
        uvs,
        indices,
        position: Vec3::new(
            chunk.header.position[1],
            chunk.header.position[2],
            chunk.header.position[0],
        ),
    }
}
