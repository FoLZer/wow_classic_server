mod editor;
mod geometry;
mod material;
mod object_loader;

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

use crate::{MPQResource, render_controls::RenderSettings, terrain_material::TerrainMaterial};

pub use editor::TerrainEditorPlugin;
use editor::{TerrainEditor, select_adt_chunk};
use geometry::{adt_center, adt_to_overview_mesh};
use material::{
    CachedTerrainTexture, PreparedMaterialMaps, global_layer_map, prepare_material_maps,
    update_texture_array,
};
use object_loader::{AdtObjectPlacements, ObjectCache, spawn_adt_objects, spawn_world_wmo};

pub(super) const ADT_CELLS_PER_GRID: usize = 16;
const ADT_GRID_SIZE: usize = 64;
pub(super) const CHUNK_SIZE: f32 = 33.3334;
const ADT_SIZE: f32 = CHUNK_SIZE * ADT_CELLS_PER_GRID as f32;
const ADT_HALF_DIAGONAL: f32 = ADT_SIZE * std::f32::consts::FRAC_1_SQRT_2;
const STREAM_BUFFER: f32 = ADT_SIZE;
const STREAM_UPDATE_DISTANCE: f32 = CHUNK_SIZE * 0.5;
const MAX_PENDING_ADTS: usize = 32;

pub fn load_map(mpqs: &mut PatchChain, commands: &mut Commands, index: usize) -> Option<Transform> {
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
            return None;
        }
        Err(error) => panic!("{error:?}"),
    };
    let wdt = WdtReader::new(&mut Cursor::new(wdt_file_buf), WowVersion::Classic)
        .read()
        .unwrap();
    let world_wmos = collect_world_wmos(&wdt);
    let wmo_camera_transform = wdt
        .is_wmo_only()
        .then(|| world_wmo_camera_transform(&world_wmos))
        .flatten();
    if wmo_camera_transform.is_some() {
        info!(
            "Map {directory} uses {} world WMO placements",
            world_wmos.len()
        );
    }

    commands.insert_resource(TerrainMap {
        has_big_alpha: wdt.mphd.flags.contains(MphdFlags::ADT_HAS_BIG_ALPHA),
        wdt,
        map_name: directory.to_owned(),
        loaded_adts: HashMap::new(),
        loading_adts: HashMap::new(),
        texture_cache: HashMap::new(),
        texture_array: None,
        object_cache: ObjectCache::default(),
        world_wmos,
        world_wmos_loaded: false,
        last_update_position: None,
        loading: true,
        metrics: TerrainLoadMetrics::new(),
    });
    wmo_camera_transform
}

struct LoadedAdt {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    images: [Handle<Image>; 3],
    objects: Option<Entity>,
    object_placements: AdtObjectPlacements,
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

struct WorldWmo {
    filename: String,
    position: [f32; 3],
    rotation: [f32; 3],
    lower_bounds: [f32; 3],
    upper_bounds: [f32; 3],
    doodad_set: u16,
    scale: u16,
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
    object_cache: ObjectCache,
    world_wmos: Vec<WorldWmo>,
    world_wmos_loaded: bool,
    last_update_position: Option<Vec2>,
    loading: bool,
    metrics: TerrainLoadMetrics,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainAdt {
    pub x: u8,
    pub y: u8,
}

#[derive(Component)]
pub struct RenderedObject;

pub fn stream_terrain_chunks(
    mut commands: Commands,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut terrain: ResMut<TerrainMap>,
    mut mpqs: ResMut<MPQResource>,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    mut object_materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    editor: Res<TerrainEditor>,
    render_settings: Res<RenderSettings>,
) {
    let Ok(camera_transform) = camera.single() else {
        return;
    };
    let camera_position = camera_transform.translation().xz();

    let camera_moved = terrain.last_update_position.is_none_or(|position| {
        position.distance_squared(camera_position) >= STREAM_UPDATE_DISTANCE.powi(2)
    });
    if !camera_moved && !terrain.loading && !render_settings.is_changed() {
        return;
    }
    if camera_moved {
        terrain.last_update_position = Some(camera_position);
        terrain.loading = true;
    }

    let load_distance = render_settings.adt_distance + ADT_HALF_DIAGONAL;
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
        if let Some(objects) = loaded_adt.objects {
            commands.entity(objects).despawn();
        }
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
            &render_settings,
        );
    }

    if !terrain.world_wmos_loaded {
        spawn_world_wmos(
            &mut commands,
            &mut terrain,
            &mut mpqs.mpqs,
            &mut object_materials,
            &mut meshes,
            &mut images,
            &render_settings,
        );
        terrain.world_wmos_loaded = true;
    }

    stream_adt_objects(
        &mut commands,
        &mut terrain,
        camera_position,
        &mut mpqs.mpqs,
        &mut object_materials,
        &mut meshes,
        &mut images,
        &render_settings,
    );

    let available_task_slots = MAX_PENDING_ADTS.saturating_sub(terrain.loading_adts.len());
    let adts_to_start = adts_to_load.len().min(available_task_slots);
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

fn collect_world_wmos(wdt: &WdtFile) -> Vec<WorldWmo> {
    let (Some(mwmo), Some(modf)) = (wdt.mwmo.as_ref(), wdt.modf.as_ref()) else {
        return Vec::new();
    };

    modf.entries
        .iter()
        .filter_map(|placement| {
            let Some(filename) = mwmo.filenames.get(placement.id as usize) else {
                warn!(
                    "WDT WMO {} references missing MWMO entry {}",
                    placement.unique_id, placement.id
                );
                return None;
            };
            Some(WorldWmo {
                filename: filename.clone(),
                position: placement.position,
                rotation: placement.rotation,
                lower_bounds: placement.lower_bounds,
                upper_bounds: placement.upper_bounds,
                doodad_set: placement.doodad_set,
                scale: placement.scale,
            })
        })
        .collect()
}

fn world_wmo_camera_transform(world_wmos: &[WorldWmo]) -> Option<Transform> {
    let (mut lower_bounds, mut upper_bounds) = world_wmo_bounds(world_wmos.first()?);
    for world_wmo in &world_wmos[1..] {
        let (wmo_lower_bounds, wmo_upper_bounds) = world_wmo_bounds(world_wmo);
        lower_bounds = lower_bounds.min(wmo_lower_bounds);
        upper_bounds = upper_bounds.max(wmo_upper_bounds);
    }

    let target = (lower_bounds + upper_bounds) * 0.5;
    let distance = (upper_bounds - lower_bounds).length().max(1_000.0);
    Some(
        Transform::from_translation(target + Vec3::new(distance, distance * 0.6, distance))
            .looking_at(target, Vec3::Y),
    )
}

fn world_wmo_bounds(world_wmo: &WorldWmo) -> (Vec3, Vec3) {
    let first_corner = wmo_position_to_world(world_wmo.lower_bounds);
    let second_corner = wmo_position_to_world(world_wmo.upper_bounds);
    let lower_bounds = first_corner.min(second_corner);
    let upper_bounds = first_corner.max(second_corner);
    (lower_bounds, upper_bounds)
}

fn wmo_position_to_world(position: [f32; 3]) -> Vec3 {
    let map_half_size = ADT_GRID_SIZE as f32 * ADT_SIZE * 0.5;
    Vec3::new(
        map_half_size - position[0],
        position[1],
        map_half_size - position[2],
    )
}

fn spawn_world_wmos(
    commands: &mut Commands,
    terrain: &mut TerrainMap,
    mpqs: &mut PatchChain,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    render_settings: &RenderSettings,
) {
    let (world_wmos, object_cache) = (&terrain.world_wmos, &mut terrain.object_cache);
    for world_wmo in world_wmos {
        if let Some(entity) = spawn_world_wmo(
            commands,
            &world_wmo.filename,
            world_wmo.position,
            world_wmo.rotation,
            world_wmo.doodad_set,
            world_wmo.scale,
            mpqs,
            object_cache,
            meshes,
            materials,
            images,
        ) {
            commands.entity(entity).insert((
                RenderedObject,
                if render_settings.render_objects {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use wow_wdt::chunks::{ModfChunk, ModfEntry, MwmoChunk};

    use super::*;

    #[test]
    fn collects_wdt_world_wmo_placements() {
        let mut wdt = WdtFile::new(WowVersion::Classic);
        wdt.mwmo = Some(MwmoChunk {
            filenames: vec!["World\\Wmo\\first.wmo".to_owned(), "second.wmo".to_owned()],
        });
        wdt.modf = Some(ModfChunk {
            entries: vec![ModfEntry {
                id: 1,
                position: [1.0, 2.0, 3.0],
                rotation: [4.0, 5.0, 6.0],
                doodad_set: 2,
                scale: 1024,
                ..Default::default()
            }],
        });

        let world_wmos = collect_world_wmos(&wdt);

        assert_eq!(world_wmos.len(), 1);
        assert_eq!(world_wmos[0].filename, "second.wmo");
        assert_eq!(world_wmos[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(world_wmos[0].rotation, [4.0, 5.0, 6.0]);
        assert_eq!(world_wmos[0].doodad_set, 2);
        assert_eq!(world_wmos[0].scale, 1024);
    }

    #[test]
    fn frames_the_camera_around_world_wmo_bounds() {
        let world_wmo = WorldWmo {
            filename: "test.wmo".to_owned(),
            position: [0.0; 3],
            rotation: [0.0; 3],
            lower_bounds: [100.0, 10.0, 200.0],
            upper_bounds: [300.0, 110.0, 400.0],
            doodad_set: 0,
            scale: 1024,
        };

        let transform = world_wmo_camera_transform(&[world_wmo]).unwrap();
        let map_half_size = ADT_GRID_SIZE as f32 * ADT_SIZE * 0.5;

        assert_eq!(
            transform.translation,
            Vec3::new(map_half_size + 800.0, 660.0, map_half_size + 700.0)
        );
    }
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
    render_settings: &RenderSettings,
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
                ..VisibilityRange::abrupt(0.0, render_settings.adt_distance)
            },
            if render_settings.render_adts {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ))
        .observe(select_adt_chunk)
        .id();
    let object_placements = AdtObjectPlacements::from_adt(&prepared.adt);
    terrain.loaded_adts.insert(
        coordinates,
        LoadedAdt {
            entity,
            mesh,
            material,
            images: [alpha_map, layer_map, animation_map],
            objects: None,
            object_placements,
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

#[allow(clippy::too_many_arguments)]
fn stream_adt_objects(
    commands: &mut Commands,
    terrain: &mut TerrainMap,
    camera_position: Vec2,
    mpqs: &mut PatchChain,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    render_settings: &RenderSettings,
) {
    let load_distance = render_settings.object_distance + ADT_HALF_DIAGONAL;
    let unload_distance_squared = (load_distance + STREAM_BUFFER).powi(2);
    let object_adts_to_unload = terrain
        .loaded_adts
        .iter_mut()
        .filter_map(|(&coordinates, loaded_adt)| {
            (loaded_adt.objects.is_some()
                && adt_center(coordinates.0, coordinates.1).distance_squared(camera_position)
                    > unload_distance_squared)
                .then(|| (coordinates, loaded_adt.objects.take().unwrap()))
        })
        .collect::<Vec<_>>();
    for (_, entity) in object_adts_to_unload {
        commands.entity(entity).despawn();
    }

    let object_adts_to_load = terrain
        .loaded_adts
        .iter()
        .filter_map(|(&coordinates, loaded_adt)| {
            (loaded_adt.objects.is_none()
                && adt_center(coordinates.0, coordinates.1).distance_squared(camera_position)
                    <= load_distance.powi(2))
            .then_some(coordinates)
        })
        .collect::<Vec<_>>();
    for coordinates in object_adts_to_load {
        let loaded_adt = terrain.loaded_adts.get_mut(&coordinates).unwrap();
        loaded_adt.objects = Some(spawn_adt_objects(
            commands,
            &loaded_adt.object_placements,
            coordinates,
            adt_center(coordinates.0, coordinates.1),
            mpqs,
            &mut terrain.object_cache,
            meshes,
            materials,
            images,
        ));
        commands.entity(loaded_adt.objects.unwrap()).insert((
            RenderedObject,
            if render_settings.render_objects {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ));
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
