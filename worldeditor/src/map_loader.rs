use std::{
    collections::{HashMap, hash_map::Entry},
    io::Cursor,
};

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
use wow_blp::{BlpImage, convert::blp_to_image, parser::load_blp_from_buf};
use wow_mpq::PatchChain;
use wow_wdt::{WdtFile, WdtReader, chunks::MphdFlags, version::WowVersion};

use crate::{
    MPQResource,
    combined_alpha_map::CombinedAlphaMap,
    mpq_read_file,
    terrain_material::{AnimationData, TerrainMaterial},
};

const ADT_CELLS_PER_GRID: usize = 16;
const ADT_GRID_SIZE: usize = 64;
const CHUNK_SIZE: f32 = 33.3334;
const ADT_SIZE: f32 = CHUNK_SIZE * ADT_CELLS_PER_GRID as f32;
const ADT_HALF_DIAGONAL: f32 = ADT_SIZE * std::f32::consts::FRAC_1_SQRT_2;
const STREAM_BUFFER: f32 = ADT_SIZE;
const STREAM_UPDATE_DISTANCE: f32 = CHUNK_SIZE * 0.5;

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
        texture_handles: HashMap::new(),
        transparent_image: None,
        last_update_position: None,
        view_distance,
    });
}

struct LoadedAdt {
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    alpha_textures: Vec<Handle<Image>>,
}

#[derive(Resource)]
pub struct TerrainMap {
    wdt: WdtFile,
    map_name: String,
    has_big_alpha: bool,
    loaded_adts: HashMap<(usize, usize), LoadedAdt>,
    texture_handles: HashMap<String, Handle<Image>>,
    transparent_image: Option<Handle<Image>>,
    last_update_position: Option<Vec2>,
    view_distance: f32,
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

    if terrain.last_update_position.is_some_and(|position| {
        position.distance_squared(camera_position) < STREAM_UPDATE_DISTANCE.powi(2)
    }) {
        return;
    }
    terrain.last_update_position = Some(camera_position);

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
        for entity in loaded_adt.entities {
            commands.entity(entity).despawn();
        }
        for mesh in loaded_adt.meshes {
            meshes.remove(mesh.id());
        }
        for material in loaded_adt.materials {
            terrain_materials.remove(material.id());
        }
        for alpha_texture in loaded_adt.alpha_textures {
            images.remove(alpha_texture.id());
        }
    }

    for (_, x, y) in adts_to_load {
        let map_path = format!(
            "World\\Maps\\{}\\{}_{}_{}.adt",
            terrain.map_name, terrain.map_name, x, y
        );
        let map_file_buf = mpqs.mpqs.read_file(&map_path).unwrap();
        let adt = parse_adt(&mut Cursor::new(map_file_buf)).unwrap();
        let ParsedAdt::Root(adt) = adt else { panic!() };
        let map_name = terrain.map_name.clone();
        let has_big_alpha = terrain.has_big_alpha;
        let chunk_data = adt_to_meshes(
            &adt,
            &map_name,
            has_big_alpha,
            &mut terrain.texture_handles,
            &mut images,
            &mut mpqs.mpqs,
        );
        let generated_center = chunk_data
            .iter()
            .flatten()
            .fold(Vec2::ZERO, |center, chunk| {
                center + Vec2::new(chunk.position_x, chunk.position_z)
            })
            / (ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID) as f32
            - Vec2::splat(CHUNK_SIZE * 0.5);
        debug_assert!(
            generated_center.distance(adt_center(x, y)) < CHUNK_SIZE,
            "ADT {x}.{y} generated at {generated_center}, outside its streaming cell"
        );
        let mut materials_map = HashMap::new();
        let mut loaded_adt = LoadedAdt {
            entities: Vec::with_capacity(ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID),
            meshes: Vec::with_capacity(ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID),
            materials: Vec::with_capacity(ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID),
            alpha_textures: Vec::with_capacity(ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID),
        };

        for chunk in chunk_data.into_iter().flatten() {
            let position = (chunk.position_x, chunk.position_y, chunk.position_z);
            loaded_adt.alpha_textures.push(chunk.alpha_texture.clone());
            let mesh = meshes.add(chunk.mesh.clone());
            let material = chunk.into_material(
                &mut materials_map,
                &mut terrain_materials,
                &mut terrain.transparent_image,
                &mut images,
            );
            let entity = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(position.0, position.1, position.2)
                        .with_rotation(Quat::from_rotation_y(-std::f32::consts::PI))
                        .with_scale(Vec3::new(CHUNK_SIZE / 8.0, 1.0, CHUNK_SIZE / 8.0)),
                    VisibilityRange::abrupt(0.0, terrain.view_distance),
                ))
                .id();
            loaded_adt.entities.push(entity);
            loaded_adt.meshes.push(mesh);
            loaded_adt.materials.push(material);
        }
        terrain.loaded_adts.insert((x, y), loaded_adt);
    }
}

fn adt_center(x: usize, y: usize) -> Vec2 {
    Vec2::new((31.5 - x as f32) * ADT_SIZE, (31.5 - y as f32) * ADT_SIZE)
}

fn adt_to_meshes(
    adt: &RootAdt,
    map_name: &str,
    has_big_alpha: bool,
    texture_handles_map: &mut HashMap<String, Handle<Image>>,
    images_res: &mut ResMut<Assets<Image>>,
    mpqs: &mut PatchChain,
) -> [[TerrainMeshData; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID] {
    std::array::from_fn(|chunk_x| {
        std::array::from_fn(|chunk_y| {
            let chunk_index = chunk_x * ADT_CELLS_PER_GRID + chunk_y;
            let chunk = &adt.mcnk_chunks[chunk_index];
            let (mesh, x, y, z) = chunk_to_mesh(chunk);

            let images = std::array::from_fn(|i| {
                if let Some(Some(layer)) = chunk.layers.as_ref().map(|c| c.layers.get(i)) {
                    let texture_id = layer.texture_id;
                    let filepath = &adt.textures[texture_id as usize];

                    let (animation_direction, animation_speed) = {
                        if layer.flags.animation_enabled() {
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

                            (
                                DIRECTIONS[layer.flags.animation_rotation() as usize],
                                layer.flags.animation_speed() as u32,
                            )
                        } else {
                            (IVec2::new(0, 0), 0)
                        }
                    };

                    match texture_handles_map.get(filepath) {
                        Some(texture) => Some(TerrainTextureLayer {
                            texture: texture.clone(),
                            animation_direction,
                            animation_speed,
                        }),
                        None => {
                            let blp = {
                                let file_buf = match mpq_read_file(mpqs, filepath) {
                                    Ok(v) => v,
                                    Err(wow_mpq::Error::FileNotFound(_)) => {
                                        error!(
                                            "BLP wasn't found for map {}. filepath: {}",
                                            map_name, filepath
                                        );
                                        return None;
                                    }
                                    Err(e) => {
                                        panic!("{:?}", e);
                                    }
                                };

                                load_blp_from_buf(&file_buf).unwrap()
                            };

                            let image = blp_to_image_with_mipmaps(&blp);

                            let handle = images_res.add(image);
                            texture_handles_map.insert(filepath.clone(), handle.clone());
                            Some(TerrainTextureLayer {
                                texture: handle,
                                animation_direction,
                                animation_speed,
                            })
                        }
                    }
                } else {
                    None
                }
            });

            let combined_alpha_map = CombinedAlphaMap::new(
                chunk,
                has_big_alpha,
                !chunk.header.flags.do_not_fix_alpha_map(),
            );
            let mut alpha_image = Image::new(
                Extent3d {
                    width: 64,
                    height: 64,
                    ..Default::default()
                },
                TextureDimension::D2,
                combined_alpha_map.into_vec(),
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            );
            alpha_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                ..Default::default()
            });

            TerrainMeshData {
                mesh,
                position_x: x,
                position_y: y,
                position_z: z,
                texture_layers: images,
                alpha_texture: images_res.add(alpha_image),
                fix_alpha: chunk.header.flags.do_not_fix_alpha_map(),
            }
        })
    })
}

const HEIGHTMAP_SIZE: usize = 145;

fn chunk_to_mesh(chunk: &McnkChunk) -> (Mesh, f32, f32, f32) {
    let heights = chunk.heights.as_ref().unwrap();
    let normals_chunk = chunk.normals.as_ref().unwrap();

    let mut vertices = vec![[-10.0; 3]; HEIGHTMAP_SIZE];
    let mut normals = vec![[0.0; 3]; HEIGHTMAP_SIZE];
    let mut uvs = vec![[0.0; 2]; HEIGHTMAP_SIZE];
    let mut indices = vec![[[0; 3]; 4]; 8 * 8];

    for y in 0..8 {
        //outer
        for x in 0..9 {
            let i = y * 17 + x;
            vertices[i] = [x as f32, heights.heights[i], y as f32];
            normals[i] = [
                -normals_chunk.normals[i].z as f32 / 127.0,
                normals_chunk.normals[i].y as f32 / 127.0,
                -normals_chunk.normals[i].x as f32 / 127.0,
            ];
            uvs[i] = [x as f32, y as f32];
        }
        //inner
        for x in 0..8 {
            let i = y * 17 + 9 + x;
            vertices[i] = [(x as f32 + 0.5), heights.heights[i], (y as f32 + 0.5)];
            normals[i] = [
                -normals_chunk.normals[i].z as f32 / 127.0,
                normals_chunk.normals[i].y as f32 / 127.0,
                -normals_chunk.normals[i].x as f32 / 127.0,
            ];
            uvs[i] = [(x as f32 + 0.5), (y as f32 + 0.5)];

            let top_left = (i - 9) as u16;
            let top_right = (i - 8) as u16;
            let bottom_left = (i + 8) as u16;
            let bottom_right = (i + 9) as u16;
            indices[y * 8 + x] = [
                //[top_left, top_right, i as u16],
                //[i as u16, top_right, bottom_right],
                //[bottom_right, bottom_left, i as u16],
                //[i as u16, bottom_left, top_left],
                [top_left, i as u16, top_right],
                [i as u16, bottom_right, top_right],
                [bottom_right, i as u16, bottom_left],
                [i as u16, top_left, bottom_left],
            ];
        }
    }
    for x in 0..9 {
        let y = 8;
        let i = y * 17 + x;
        vertices[i] = [x as f32, heights.heights[i], y as f32];
        normals[i] = [
            -normals_chunk.normals[i].z as f32 / 127.0,
            normals_chunk.normals[i].y as f32 / 127.0,
            -normals_chunk.normals[i].x as f32 / 127.0,
        ];
        uvs[i] = [x as f32, y as f32];
    }

    (
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U16(
            indices.into_iter().flatten().flatten().collect(),
        ))
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals),
        //.with_computed_normals(),
        chunk.header.position[1],
        chunk.header.position[2],
        chunk.header.position[0],
    )
}

fn get_or_generate_transparent_image(
    transparent_image: &mut Option<Handle<Image>>,
    images_res: &mut ResMut<Assets<Image>>,
) -> Handle<Image> {
    transparent_image
        .get_or_insert_with(|| {
            images_res.add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    ..Default::default()
                },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            ))
        })
        .clone()
}

pub struct TerrainMeshData {
    pub mesh: Mesh,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub texture_layers: [Option<TerrainTextureLayer>; 4],
    pub alpha_texture: Handle<Image>,
    pub fix_alpha: bool,
}

impl TerrainMeshData {
    pub fn into_material(
        self,
        materials_map: &mut HashMap<
            ([Option<TerrainTextureLayer>; 4], Handle<Image>),
            Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
        >,
        terrain_materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
        transparent_image: &mut Option<Handle<Image>>,
        images_res: &mut ResMut<Assets<Image>>,
    ) -> Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>> {
        match materials_map.entry((self.texture_layers.clone(), self.alpha_texture.clone())) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                let layer_1 =
                    self.texture_layers[0]
                        .clone()
                        .unwrap_or_else(|| TerrainTextureLayer {
                            texture: get_or_generate_transparent_image(
                                transparent_image,
                                images_res,
                            ),
                            animation_direction: IVec2::new(0, 0),
                            animation_speed: 0,
                        });
                let layer_2 =
                    self.texture_layers[1]
                        .clone()
                        .unwrap_or_else(|| TerrainTextureLayer {
                            texture: get_or_generate_transparent_image(
                                transparent_image,
                                images_res,
                            ),
                            animation_direction: IVec2::new(0, 0),
                            animation_speed: 0,
                        });
                let layer_3 =
                    self.texture_layers[2]
                        .clone()
                        .unwrap_or_else(|| TerrainTextureLayer {
                            texture: get_or_generate_transparent_image(
                                transparent_image,
                                images_res,
                            ),
                            animation_direction: IVec2::new(0, 0),
                            animation_speed: 0,
                        });
                let layer_4 =
                    self.texture_layers[3]
                        .clone()
                        .unwrap_or_else(|| TerrainTextureLayer {
                            texture: get_or_generate_transparent_image(
                                transparent_image,
                                images_res,
                            ),
                            animation_direction: IVec2::new(0, 0),
                            animation_speed: 0,
                        });

                let handle = terrain_materials.add(ExtendedMaterial {
                    base: StandardMaterial {
                        base_color: Color::WHITE,
                        ..Default::default()
                    },
                    extension: TerrainMaterial {
                        layer_1: layer_1.texture,
                        layer_2: layer_2.texture,
                        layer_3: layer_3.texture,
                        layer_4: layer_4.texture,
                        alpha: self.alpha_texture,
                        animation_data: AnimationData::new(
                            [
                                layer_1.animation_direction,
                                layer_2.animation_direction,
                                layer_3.animation_direction,
                                layer_4.animation_direction,
                            ],
                            [
                                layer_1.animation_speed,
                                layer_2.animation_speed,
                                layer_3.animation_speed,
                                layer_4.animation_speed,
                            ],
                        ),
                        fix_alpha: if self.fix_alpha { 1 } else { 0 },
                    },
                });
                entry.insert(handle.clone());
                handle
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TerrainTextureLayer {
    pub texture: Handle<Image>,
    pub animation_direction: IVec2,
    pub animation_speed: u32,
}

fn blp_to_image_with_mipmaps(blp: &BlpImage) -> Image {
    match &blp.content {
        wow_blp::BlpContent::Jpeg(blp_jpeg) => todo!(),
        wow_blp::BlpContent::Raw1(blp_raw1) => {
            let mut data = Vec::new();
            for mipmap_level in 0..blp_raw1.images.len() {
                let image = blp_to_image(&blp, mipmap_level).unwrap();
                data.extend_from_slice(&image.into_rgba8().into_vec());
                //data.extend(image.pixels().map(|(_, _, pixel)| pixel.0).flatten());
            }
            let mut image = Image::new(
                Extent3d {
                    width: blp.header.width,
                    height: blp.header.height,
                    ..Default::default()
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.texture_descriptor.mip_level_count = blp_raw1.images.len() as u32;
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..Default::default()
            });

            image
        }
        wow_blp::BlpContent::Raw3(blp_raw3) => {
            let mut data = Vec::new();
            for mipmap_level in 0..blp_raw3.images.len() {
                let image = blp_to_image(&blp, mipmap_level).unwrap();
                data.extend_from_slice(&image.into_rgba8().into_vec());
                //data.extend(image.pixels().map(|(_, _, pixel)| pixel.0).flatten());
            }
            let mut image = Image::new(
                Extent3d {
                    width: blp.header.width,
                    height: blp.header.height,
                    ..Default::default()
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.texture_descriptor.mip_level_count = blp_raw3.images.len() as u32;
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..Default::default()
            });

            image
        }
        wow_blp::BlpContent::Dxt1(blp_dxtn) => {
            let mut data = Vec::new();
            for image in &blp_dxtn.images {
                data.extend(&image.content);
            }
            let mut image = Image::new(
                Extent3d {
                    width: blp.header.width,
                    height: blp.header.height,
                    ..Default::default()
                },
                TextureDimension::D2,
                data,
                TextureFormat::Bc1RgbaUnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.texture_descriptor.mip_level_count = blp_dxtn.images.len() as u32;
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..Default::default()
            });

            image
        }
        wow_blp::BlpContent::Dxt3(blp_dxtn) => {
            let mut data = Vec::new();
            for image in &blp_dxtn.images {
                data.extend(&image.content);
            }
            let mut image = Image::new(
                Extent3d {
                    width: blp.header.width,
                    height: blp.header.height,
                    ..Default::default()
                },
                TextureDimension::D2,
                data,
                TextureFormat::Bc2RgbaUnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.texture_descriptor.mip_level_count = blp_dxtn.images.len() as u32;
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..Default::default()
            });

            image
        }
        wow_blp::BlpContent::Dxt5(blp_dxtn) => {
            let mut data = Vec::new();
            for image in &blp_dxtn.images {
                data.extend(&image.content);
            }
            let mut image = Image::new(
                Extent3d {
                    width: blp.header.width,
                    height: blp.header.height,
                    ..Default::default()
                },
                TextureDimension::D2,
                data,
                TextureFormat::Bc3RgbaUnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.texture_descriptor.mip_level_count = blp_dxtn.images.len() as u32;
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..Default::default()
            });

            image
        }
    }
}
