mod editor;
mod geometry;
mod material;

use std::{collections::HashMap, io::Cursor, time::Instant};

use bevy::{
    camera::visibility::VisibilityRange,
    pbr::ExtendedMaterial,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};
use wow_adt::{ParsedAdt, RootAdt, parse_adt};
use wow_mpq::PatchChain;
use wow_wdt::{WdtFile, WdtReader, chunks::MphdFlags, version::WowVersion};

use crate::{MPQResource, terrain_material::TerrainMaterial};

pub use editor::TerrainEditorPlugin;
use editor::{TerrainEditor, select_adt_chunk};
use geometry::{adt_center, adt_to_overview_mesh};
use material::{
    CachedTerrainTexture, PreparedMaterialMaps, global_layer_map, prepare_material_maps,
    update_texture_array,
};

pub(super) const ADT_CELLS_PER_GRID: usize = 16;
const ADT_GRID_SIZE: usize = 64;
pub(super) const CHUNK_SIZE: f32 = 33.3334;
const ADT_SIZE: f32 = CHUNK_SIZE * ADT_CELLS_PER_GRID as f32;
const ADT_HALF_DIAGONAL: f32 = ADT_SIZE * std::f32::consts::FRAC_1_SQRT_2;
const STREAM_BUFFER: f32 = ADT_SIZE;
const STREAM_UPDATE_DISTANCE: f32 = CHUNK_SIZE * 0.5;
const ADTS_STARTED_PER_FRAME: usize = 2;
const MAX_PENDING_ADTS: usize = 32;

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
        Ok(value) => value,
        Err(wow_mpq::Error::FileNotFound(_)) => {
            error!("WDT wasn't found for map {}", directory);
            return;
        }
        Err(error) => panic!("{error:?}"),
    };
    let wdt = WdtReader::new(&mut Cursor::new(wdt_file_buf), WowVersion::Classic)
        .read()
        .unwrap();

    commands.insert_resource(TerrainMap {
        has_big_alpha: wdt.mphd.flags.contains(MphdFlags::ADT_HAS_BIG_ALPHA),
        wdt,
        map_name: directory.to_owned(),
        loaded_adts: HashMap::new(),
        loading_adts: HashMap::new(),
        texture_cache: HashMap::new(),
        texture_array: None,
        last_update_position: None,
        loading: true,
        metrics: TerrainLoadMetrics::new(),
        view_distance,
    });
}

struct LoadedAdt {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    images: [Handle<Image>; 3],
}

struct TerrainLoadMetrics {
    count: usize,
    started_at: Instant,
    completion_reported: bool,
}

impl TerrainLoadMetrics {
    fn new() -> Self {
        Self {
            count: 0,
            started_at: Instant::now(),
            completion_reported: false,
        }
    }
}

struct PreparedAdt {
    x: usize,
    y: usize,
    adt: RootAdt,
    mesh: Mesh,
    material_maps: PreparedMaterialMaps,
}

#[derive(Resource)]
pub struct TerrainMap {
    wdt: WdtFile,
    map_name: String,
    has_big_alpha: bool,
    loaded_adts: HashMap<(usize, usize), LoadedAdt>,
    loading_adts: HashMap<(usize, usize), Task<PreparedAdt>>,
    texture_cache: HashMap<String, CachedTerrainTexture>,
    texture_array: Option<Handle<Image>>,
    last_update_position: Option<Vec2>,
    loading: bool,
    metrics: TerrainLoadMetrics,
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
    editor: Res<TerrainEditor>,
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
                || terrain.loading_adts.contains_key(&(x, y))
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
            (!editor.retains_adt((x, y))
                && adt_center(x, y).distance_squared(camera_position) > unload_distance_squared)
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

    terrain.loading_adts.retain(|&(x, y), _| {
        adt_center(x, y).distance_squared(camera_position) <= unload_distance_squared
    });

    let ready_adts = terrain
        .loading_adts
        .iter_mut()
        .filter_map(|(&coordinates, task)| check_ready(task).map(|adt| (coordinates, adt)))
        .collect::<Vec<_>>();

    for (coordinates, prepared) in ready_adts {
        terrain.loading_adts.remove(&coordinates);
        finalize_adt(
            prepared,
            coordinates,
            &mut commands,
            &mut terrain,
            &mut mpqs.mpqs,
            &mut terrain_materials,
            &mut meshes,
            &mut images,
        );
    }

    let available_task_slots = MAX_PENDING_ADTS.saturating_sub(terrain.loading_adts.len());
    let adts_to_start = adts_to_load
        .len()
        .min(ADTS_STARTED_PER_FRAME)
        .min(available_task_slots);
    for (_, x, y) in adts_to_load.iter().take(adts_to_start).copied() {
        let map_path = format!(
            "World\\Maps\\{}\\{}_{}_{}.adt",
            terrain.map_name, terrain.map_name, x, y
        );
        let map_file_buf = mpqs.mpqs.read_file(&map_path).unwrap();
        let has_big_alpha = terrain.has_big_alpha;
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let adt = parse_adt(&mut Cursor::new(map_file_buf)).unwrap();
            let ParsedAdt::Root(adt) = adt else { panic!() };
            let adt = *adt;
            let mesh = adt_to_overview_mesh(&adt, adt_center(x, y));
            let material_maps = prepare_material_maps(&adt, has_big_alpha);
            PreparedAdt {
                x,
                y,
                adt,
                mesh,
                material_maps,
            }
        });
        terrain.loading_adts.insert((x, y), task);
        terrain.metrics.completion_reported = false;
    }

    terrain.loading = adts_to_load.len() > adts_to_start || !terrain.loading_adts.is_empty();
    report_loading_progress(&mut terrain);
}

#[allow(clippy::too_many_arguments)]
fn finalize_adt(
    prepared: PreparedAdt,
    coordinates: (usize, usize),
    commands: &mut Commands,
    terrain: &mut TerrainMap,
    mpqs: &mut PatchChain,
    terrain_materials: &mut Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
) {
    let map_name = terrain.map_name.clone();
    let texture_array = update_texture_array(
        &prepared.adt,
        &map_name,
        &mut terrain.texture_cache,
        &mut terrain.texture_array,
        mpqs,
        images,
    );
    let layer_map = global_layer_map(
        prepared.material_maps.local_layers,
        &prepared.adt,
        &terrain.texture_cache,
    );
    let alpha_map = images.add(prepared.material_maps.alpha_map);
    let layer_map = images.add(layer_map);
    let animation_map = images.add(prepared.material_maps.animation_map);
    let mesh = meshes.add(prepared.mesh);
    let material = terrain_materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            ..Default::default()
        },
        extension: TerrainMaterial {
            textures: texture_array,
            alpha_map: alpha_map.clone(),
            layer_map: layer_map.clone(),
            animation_map: animation_map.clone(),
        },
    });
    let center = adt_center(prepared.x, prepared.y);
    let entity = commands
        .spawn((
            TerrainAdt {
                x: prepared.x as u8,
                y: prepared.y as u8,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(center.x, 0.0, center.y),
            VisibilityRange {
                use_aabb: true,
                ..VisibilityRange::abrupt(0.0, terrain.view_distance)
            },
        ))
        .observe(select_adt_chunk)
        .id();
    terrain.loaded_adts.insert(
        coordinates,
        LoadedAdt {
            entity,
            mesh,
            material,
            images: [alpha_map, layer_map, animation_map],
        },
    );
    terrain.metrics.count += 1;
    if terrain.metrics.count.is_multiple_of(100) {
        let elapsed = terrain.metrics.started_at.elapsed().as_secs_f64();
        info!(
            "Loaded {} ADTs in {:.2}s ({:.1} ADTs/s)",
            terrain.metrics.count,
            elapsed,
            terrain.metrics.count as f64 / elapsed,
        );
    }
}

fn report_loading_progress(terrain: &mut TerrainMap) {
    if !terrain.loading && !terrain.metrics.completion_reported {
        let elapsed = terrain.metrics.started_at.elapsed().as_secs_f64();
        info!(
            "Finished loading {} ADTs in {:.2}s ({:.1} ADTs/s)",
            terrain.metrics.count,
            elapsed,
            terrain.metrics.count as f64 / elapsed,
        );
        terrain.metrics.completion_reported = true;
    }
}
