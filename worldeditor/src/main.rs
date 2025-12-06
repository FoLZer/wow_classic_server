mod terrain_material;

use std::{
    collections::{HashMap, hash_map::Entry},
    io::Cursor,
    path::PathBuf,
    str::FromStr,
};

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use serde::{Deserialize, Serialize};
use wow_adt::{Adt, McnkChunk};
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use wow_mpq::PatchChain;
use wow_wdt::{WdtReader, chunks::MphdFlags, version::WowVersion};

use crate::terrain_material::{AnimationData, TerrainMaterial};

#[derive(Deserialize, Serialize)]
struct AppSettings {
    mpq_directory_path: PathBuf,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mpq_directory_path: PathBuf::from_str("./Data").unwrap(),
        }
    }
}

fn main() {
    let config: AppSettings = confy::load_path("./worldeditor_config.toml").unwrap();
     
    let mpqs = PatchChain::from_archives_parallel(vec![
        (config.mpq_directory_path.join("patch-2.MPQ"), 101),
        (config.mpq_directory_path.join("patch.MPQ"), 100),
        (config.mpq_directory_path.join("wmo.MPQ"), 8),
        (config.mpq_directory_path.join("texture.MPQ"), 7),
        (config.mpq_directory_path.join("terrain.MPQ"), 6),
        (config.mpq_directory_path.join("speech.MPQ"), 5),
        (config.mpq_directory_path.join("sound.MPQ"), 4),
        (config.mpq_directory_path.join("model.MPQ"), 3),
        (config.mpq_directory_path.join("misc.MPQ"), 2),
        (config.mpq_directory_path.join("dbc.MPQ"), 1),
        (config.mpq_directory_path.join("base.MPQ"), 0),
    ])
    .unwrap();

    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<TerrainMaterial>::default()))
        .add_plugins(FreeCameraPlugin)
        //.add_plugins(EguiPlugin::default())
        //.add_plugins(WorldInspectorPlugin::new())
        .insert_resource(MPQResource { mpqs })
        .add_systems(Startup, setup)
        .run();
}

#[derive(Resource)]
struct MPQResource {
    pub mpqs: PatchChain,
}

fn setup(
    mut commands: Commands,
    materials: ResMut<Assets<StandardMaterial>>,
    terrain_materials: ResMut<Assets<TerrainMaterial>>,
    meshes: ResMut<Assets<Mesh>>,
    images: ResMut<Assets<Image>>,
    mut mpqs_res: ResMut<MPQResource>,
) {
    commands.spawn((
        Camera3d::default(),
        FreeCamera {
            walk_speed: 50.0,
            run_speed: 600.0,
            ..Default::default()
        },
    ));

    load_map(
        &mut mpqs_res.mpqs,
        commands,
        materials,
        terrain_materials,
        meshes,
        images,
        0,
    );
}

const ADT_CELLS_PER_GRID: usize = 16;

fn load_map(
    mpqs: &mut PatchChain,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
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

    let mut texture_handles_map: HashMap<String, Handle<Image>> = HashMap::new();

    let map_meshes: Vec<
        Vec<
            Option<
                [[TerrainMeshData; ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID],
            >,
        >,
    > = (0..64)
        //(30..35)
        .map(|y| {
            (0..64)
                //(30..35)
                .map(|x| {
                    if !wdt.main.entries[y][x].has_adt() {
                        return None;
                    }
                    let map_path =
                        format!("World\\Maps\\{}\\{}_{}_{}.adt", directory, directory, x, y);
                    let map_file_buf = mpqs.read_file(&map_path).unwrap();
                    let adt = Adt::from_reader(&mut Cursor::new(map_file_buf)).unwrap();

                    Some(std::array::from_fn(|chunk_x| {
                        std::array::from_fn(|chunk_y| {
                            let chunk_index = chunk_x * ADT_CELLS_PER_GRID + chunk_y;
                            let chunk = &adt.mcnk_chunks[chunk_index];
                            let (mesh, x, y, z) = chunk_to_mesh(chunk);

                            let mtex = adt.mtex.as_ref().unwrap();

                            let images = std::array::from_fn(|i| {
                                if let Some(layer) = chunk
                                .texture_layers
                                .get(i)
                            {
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
                                    Entry::Occupied(entry) => {
                                        Some(TerrainTextureLayer {
                                            texture: entry.get().clone(),
                                            animation_direction,
                                            animation_speed 
                                        })
                                    }
                                    Entry::Vacant(entry) => {
                                        let blp = {
                                            let file_buf = match mpq_read_file(mpqs, filepath) {
                                                Ok(v) => v,
                                                Err(wow_mpq::Error::FileNotFound(_)) => {
                                                    error!(
                                                        "BLP wasn't found for map {}. filepath: {}",
                                                        directory, filepath
                                                    );
                                                    return None;
                                                }
                                                Err(e) => {
                                                    panic!("{:?}", e);
                                                }
                                            };

                                            load_blp_from_buf(&file_buf).unwrap()
                                        };
                                        let texture = blp_to_image(&blp, 0).unwrap();
                                        let mut image = Image::from_dynamic(
                                            texture,
                                            false,
                                            RenderAssetUsages::RENDER_WORLD,
                                        );
                                        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                                            mag_filter: ImageFilterMode::Linear,
                                            min_filter: ImageFilterMode::Linear,
                                            ..Default::default()
                                        });
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
                            let combined_alpha_map = chunk.get_combined_alpha_map(wdt.mphd.flags.contains(MphdFlags::ADT_HAS_BIG_ALPHA), !do_not_fix_alpha_flag);
                            let mut alpha_image = Image::new(
                                Extent3d {
                                    width: 64,
                                    height: 64,
                                    ..Default::default()
                                },
                                TextureDimension::D2,
                                combined_alpha_map.as_slice().to_vec(),
                                TextureFormat::Rgba8Unorm,
                                RenderAssetUsages::RENDER_WORLD
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
                            }
                        })
                    }))
                })
                .collect()
        })
        .collect();

    let default_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });
    let mut materials_map: HashMap<([Option<TerrainTextureLayer>; 4], Handle<Image>), Handle<TerrainMaterial>> =
        HashMap::new();

    let mut transparent_image = None;

    for v1 in map_meshes.into_iter() {
        for v2 in v1.into_iter() {
            let Some(v2) = v2 else {
                continue;
            };

            for v3 in v2.into_iter() {
                for v4 in v3.into_iter() {
                    let mesh = meshes.add(v4.mesh.clone());

                    const CHUNK_SIZE: f32 = 33.3333;

                    if v4.texture_layers.iter().all(|v| v.is_none()) {
                        commands.spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(default_material.clone()),
                            Transform::from_xyz(v4.position_x, v4.position_y, v4.position_z)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::PI))
                                .with_scale(Vec3 {
                                    x: CHUNK_SIZE / 8.0,
                                    y: 1.0,
                                    z: CHUNK_SIZE / 8.0,
                                }),
                        ));
                    } else {
                        let material = match materials_map.entry((v4.texture_layers.clone(), v4.alpha_texture.clone())) {
                            Entry::Occupied(entry) => entry.get().clone(),
                            Entry::Vacant(entry) => {
                                let layer_1 = v4.texture_layers[0].as_ref().unwrap();
                                let layer_2 = v4.texture_layers[1].clone().unwrap_or_else(|| TerrainTextureLayer {
                                    texture: get_or_generate_transparent_image(
                                        &mut transparent_image,
                                        &mut images_res,
                                    ),
                                    animation_direction: IVec2::new(0, 0),
                                    animation_speed: 0
                                });
                                let layer_3 = v4.texture_layers[2].clone().unwrap_or_else(|| TerrainTextureLayer {
                                    texture: get_or_generate_transparent_image(
                                        &mut transparent_image,
                                        &mut images_res,
                                    ),
                                    animation_direction: IVec2::new(0, 0),
                                    animation_speed: 0
                                });
                                let layer_4 = v4.texture_layers[3].clone().unwrap_or_else(|| TerrainTextureLayer {
                                    texture: get_or_generate_transparent_image(
                                        &mut transparent_image,
                                        &mut images_res,
                                    ),
                                    animation_direction: IVec2::new(0, 0),
                                    animation_speed: 0
                                });

                                let handle = terrain_materials.add(TerrainMaterial {
                                    layer_1: layer_1.texture.clone(),
                                    layer_2: layer_2.texture.clone(),
                                    layer_3: layer_3.texture.clone(),
                                    layer_4: layer_4.texture.clone(),
                                    alpha: v4.alpha_texture,
                                    animation_data: AnimationData::new(
                                        [layer_1.animation_direction, layer_2.animation_direction, layer_3.animation_direction, layer_4.animation_direction],
                                        [layer_1.animation_speed, layer_2.animation_speed, layer_3.animation_speed, layer_4.animation_speed],
                                    )
                                });
                                entry.insert(handle.clone());
                                handle
                            }
                        };

                        commands.spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(material),
                            Transform::from_xyz(v4.position_x, v4.position_y, v4.position_z)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::PI))
                                .with_scale(Vec3 {
                                    x: CHUNK_SIZE / 8.0,
                                    y: 1.0,
                                    z: CHUNK_SIZE / 8.0,
                                }),
                        ));
                    }
                }
            }
        }
    }
}

const HEIGHTMAP_SIZE: usize = 145;

fn chunk_to_mesh(chunk: &McnkChunk) -> (Mesh, f32, f32, f32) {
    let heights = &chunk.height_map;

    let mut vertices = vec![[-10.0; 3]; HEIGHTMAP_SIZE];
    let mut uvs = vec![[0.0; 2]; HEIGHTMAP_SIZE];
    //let mut indices = vec![[[1000; 3]; 4]; 8 * 8];
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

// Some file names appear to be uppercased inside mpqs, this function tries to handle this case
fn mpq_read_file(mpqs: &mut PatchChain, filepath: &str) -> Result<Vec<u8>, wow_mpq::Error> {
    match mpqs.read_file(filepath) {
        Ok(v) => Ok(v),
        Err(wow_mpq::Error::FileNotFound(_)) => {
            let filepath = {
                if let Some((left, right)) = filepath.rsplit_once("\\") {
                    // Only uppercase the filename, without the extension if present
                    let right = if let Some((left, right)) = right.rsplit_once('.') {
                        format!("{}.{right}", left.to_uppercase())
                    } else {
                        right.to_uppercase()
                    };
                    format!("{left}\\{}", right)
                } else {
                    filepath.to_uppercase()
                }
            };
            mpqs.read_file(&filepath)
        }
        Err(e) => Err(e),
    }
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
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TerrainTextureLayer {
    pub texture: Handle<Image>,
    pub animation_direction: IVec2,
    pub animation_speed: u32,
}
