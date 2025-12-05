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

use crate::terrain_material::TerrainMaterial;

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
            walk_speed: 800.0,
            run_speed: 2500.0,
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
                [[(
                    Mesh,
                    f32,
                    f32,
                    f32,
                    [Option<Handle<Image>>; 4],
                    Handle<Image>,
                ); ADT_CELLS_PER_GRID]; ADT_CELLS_PER_GRID],
            >,
        >,
    > = (0..64)
        //(30..35)
        .map(|y| {
            (0..64)
                //(30..35)
                .map(|x| {
                    if wdt.main.entries[y][x].flags == 0 {
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
                                match texture_handles_map.entry(filepath.clone()) {
                                    Entry::Occupied(entry) => {
                                        Some(entry.get().clone())
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
                                            true,
                                            RenderAssetUsages::RENDER_WORLD,
                                        );
                                        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                                            mag_filter: ImageFilterMode::Linear,
                                            min_filter: ImageFilterMode::Linear,
                                            ..Default::default()
                                        });
                                        let handle = images_res.add(image);
                                        entry.insert(handle.clone());
                                        Some(handle)
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

                            (mesh, x, y, z, images, images_res.add(alpha_image))
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
    let mut materials_map: HashMap<[Option<Handle<Image>>; 4], Handle<TerrainMaterial>> =
        HashMap::new();

    let mut transparent_image = None;

    for v1 in map_meshes.into_iter() {
        for v2 in v1.into_iter() {
            let Some(v2) = v2 else {
                continue;
            };

            for v3 in v2.into_iter() {
                for v4 in v3.into_iter() {
                    let mesh = meshes.add(v4.0.clone());

                    const CHUNK_SIZE: f32 = 33.3333;

                    let images = v4.4;
                    let alpha_image = v4.5;
                    if images.iter().all(|v| v.is_none()) {
                        commands.spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(default_material.clone()),
                            Transform::from_xyz(v4.1, v4.2, v4.3)
                                .with_rotation(Quat::from_rotation_y(-std::f32::consts::PI))
                                .with_scale(Vec3 {
                                    x: CHUNK_SIZE / 8.0,
                                    y: 1.0,
                                    z: CHUNK_SIZE / 8.0,
                                }),
                        ));
                    } else {
                        let material = match materials_map.entry(images.clone()) {
                            Entry::Occupied(entry) => entry.get().clone(),
                            Entry::Vacant(entry) => {
                                let handle = terrain_materials.add(TerrainMaterial {
                                    layer_1: images[0].clone().unwrap(),
                                    layer_2: get_or_generate_transparent_image(
                                        &images[1],
                                        &mut transparent_image,
                                        &mut images_res,
                                    ),
                                    layer_3: get_or_generate_transparent_image(
                                        &images[2],
                                        &mut transparent_image,
                                        &mut images_res,
                                    ),
                                    layer_4: get_or_generate_transparent_image(
                                        &images[3],
                                        &mut transparent_image,
                                        &mut images_res,
                                    ),
                                    alpha: alpha_image,
                                });
                                entry.insert(handle.clone());
                                handle
                            }
                        };

                        commands.spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(material),
                            Transform::from_xyz(v4.1, v4.2, v4.3)
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

    const CHUNK_SIZE: f32 = 8.0;
    for y in 0..8 {
        //outer
        for x in 0..9 {
            let i = y * 17 + x;
            vertices[i] = [x as f32, heights[i], y as f32];
            uvs[i] = [x as f32 / CHUNK_SIZE, y as f32 / CHUNK_SIZE];
        }
        //inner
        for x in 0..8 {
            let i = y * 17 + 9 + x;
            vertices[i] = [(x as f32 + 0.5), heights[i], (y as f32 + 0.5)];
            uvs[i] = [(x as f32 + 0.5) / CHUNK_SIZE, (y as f32 + 0.5) / CHUNK_SIZE];

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
        uvs[i] = [x as f32 / CHUNK_SIZE, y as f32 / CHUNK_SIZE];
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
    image: &Option<Handle<Image>>,
    transparent_image: &mut Option<Handle<Image>>,
    images_res: &mut ResMut<Assets<Image>>,
) -> Handle<Image> {
    match image {
        Some(v) => v.clone(),
        None => transparent_image
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
            .clone(),
    }
}
