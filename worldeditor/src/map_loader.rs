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
use image::GenericImageView;
use wow_adt::{Adt, McnkChunk};
use wow_blp::{BlpImage, convert::blp_to_image, parser::load_blp_from_buf};
use wow_mpq::PatchChain;
use wow_wdt::{WdtFile, WdtReader, chunks::MphdFlags, version::WowVersion};

use crate::{
    mpq_read_file,
    terrain_material::{AnimationData, TerrainMaterial},
};

const ADT_CELLS_PER_GRID: usize = 16;

pub fn load_map(
    mpqs: &mut PatchChain,
    mut commands: Commands,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images_res: ResMut<Assets<Image>>,
    index: usize,
) {
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

    let terrain_data = TerrainData::load_from_wdt(&wdt, directory, mpqs, &mut images_res);

    terrain_data.spawn(
        &mut commands,
        &mut terrain_materials,
        &mut meshes,
        &mut images_res,
    );
}

struct TerrainData {
    data: Vec<TerrainMeshData>,
}

impl TerrainData {
    //const LOAD_ADT_RANGE: std::ops::Range<usize> = 0..64;
    const LOAD_ADT_RANGE: std::ops::Range<usize> = 0..14;

    pub fn load_from_wdt(
        wdt: &WdtFile,
        map_name: &str,
        mpqs: &mut PatchChain,
        images_res: &mut ResMut<Assets<Image>>,
    ) -> Self {
        let mut data = Vec::with_capacity(
            Self::LOAD_ADT_RANGE.len()
                * Self::LOAD_ADT_RANGE.len()
                * ADT_CELLS_PER_GRID
                * ADT_CELLS_PER_GRID,
        );

        let mut texture_handles_map: HashMap<String, Handle<Image>> = HashMap::new();

        for y in Self::LOAD_ADT_RANGE {
            for x in Self::LOAD_ADT_RANGE {
                if !wdt.main.entries[y][x].has_adt() {
                    continue;
                }
                let map_path = format!("World\\Maps\\{}\\{}_{}_{}.adt", map_name, map_name, x, y);
                let map_file_buf = mpqs.read_file(&map_path).unwrap();
                let adt = Adt::from_reader(&mut Cursor::new(map_file_buf)).unwrap();

                data.extend(
                    adt_to_meshes(
                        &adt,
                        map_name,
                        wdt.mphd.flags.contains(MphdFlags::ADT_HAS_BIG_ALPHA),
                        &mut texture_handles_map,
                        images_res,
                        mpqs,
                    )
                    .into_iter()
                    .flatten(),
                );
            }
        }

        Self { data }
    }

    pub fn spawn(
        self,
        commands: &mut Commands,
        terrain_materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
        meshes: &mut ResMut<Assets<Mesh>>,
        images_res: &mut ResMut<Assets<Image>>,
    ) {
        let mut materials_map: HashMap<
            ([Option<TerrainTextureLayer>; 4], Handle<Image>),
            Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
        > = HashMap::new();

        let mut transparent_image = None;

        let bundles = self
            .data
            .into_iter()
            .map(|v| {
                let mesh = meshes.add(v.mesh.clone());

                const CHUNK_SIZE: f32 = 33.3334;

                let position = (v.position_x, v.position_y, v.position_z);
                let material = v.into_material(
                    &mut materials_map,
                    terrain_materials,
                    &mut transparent_image,
                    images_res,
                );
                (
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(position.0, position.1, position.2)
                        .with_rotation(Quat::from_rotation_y(-std::f32::consts::PI))
                        .with_scale(Vec3 {
                            x: CHUNK_SIZE / 8.0,
                            y: 1.0,
                            z: CHUNK_SIZE / 8.0,
                        }),
                    VisibilityRange::abrupt(0.0, 2000.0),
                )
            })
            .collect::<Vec<_>>();

        commands.spawn_batch(bundles);
    }
}

fn adt_to_meshes(
    adt: &Adt,
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

            let mtex = adt.mtex.as_ref().unwrap();

            let images = std::array::from_fn(|i| {
                if let Some(layer) = chunk.texture_layers.get(i) {
                    let texture_id = layer.texture_id;
                    let filepath = &mtex.filenames[texture_id as usize];

                    let (animation_direction, animation_speed) = {
                        let is_animation_enabled = (layer.flags & 0x40) != 0;
                        if is_animation_enabled {
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

                            let direction_index = layer.flags & 0b111;
                            let speed = (layer.flags >> 3) & 0b111;
                            (DIRECTIONS[direction_index as usize], speed)
                        } else {
                            (IVec2::new(0, 0), 0)
                        }
                    };

                    match texture_handles_map.entry(filepath.clone()) {
                        Entry::Occupied(entry) => Some(TerrainTextureLayer {
                            texture: entry.get().clone(),
                            animation_direction,
                            animation_speed,
                        }),
                        Entry::Vacant(entry) => {
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
                            entry.insert(handle.clone());
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

            let do_not_fix_alpha_flag = (chunk.flags & 0x8000) != 0;
            let combined_alpha_map =
                chunk.get_combined_alpha_map(has_big_alpha, !do_not_fix_alpha_flag);
            let mut alpha_image = Image::new(
                Extent3d {
                    width: 64,
                    height: 64,
                    ..Default::default()
                },
                TextureDimension::D2,
                combined_alpha_map.as_slice().to_vec(),
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            );
            alpha_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                ..Default::default()
            });

            TerrainMeshData {
                mesh,
                position_x: x,
                position_y: y,
                position_z: z,
                texture_layers: images,
                alpha_texture: images_res.add(alpha_image),
                fix_alpha: chunk.flags & 0x8000 != 0
            }
        })
    })
}

const HEIGHTMAP_SIZE: usize = 145;

fn chunk_to_mesh(chunk: &McnkChunk) -> (Mesh, f32, f32, f32) {
    let heights = &chunk.height_map;

    let mut vertices = vec![[-10.0; 3]; HEIGHTMAP_SIZE];
    let mut uvs = vec![[0.0; 2]; HEIGHTMAP_SIZE];
    let mut indices = vec![[[0; 3]; 4]; 8 * 8];

    for y in 0..8 {
        //outer
        for x in 0..9 {
            let i = y * 17 + x;
            vertices[i] = [x as f32, heights[i], y as f32];
            uvs[i] = [x as f32, y as f32];
        }
        //inner
        for x in 0..8 {
            let i = y * 17 + 9 + x;
            vertices[i] = [(x as f32 + 0.5), heights[i], (y as f32 + 0.5)];
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
        vertices[i] = [x as f32, heights[i], y as f32];
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
        .with_computed_normals(),
        chunk.position[0],
        chunk.position[1],
        chunk.position[2],
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
                TextureFormat::Rgba8Unorm,
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
                data.extend(image.pixels().map(|(_, _, pixel)| pixel.0).flatten());
            }
            let mut image = Image::new(
                Extent3d {
                    width: blp.header.width,
                    height: blp.header.height,
                    ..Default::default()
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8Unorm,
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
                data.extend(image.pixels().map(|(_, _, pixel)| pixel.0).flatten());
            }
            let mut image = Image::new(
                Extent3d {
                    width: blp.header.width,
                    height: blp.header.height,
                    ..Default::default()
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8Unorm,
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
                TextureFormat::Bc1RgbaUnorm,
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
                TextureFormat::Bc2RgbaUnorm,
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
                TextureFormat::Bc3RgbaUnorm,
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
