mod editor;
mod geometry;
mod ground_effects;
mod material;
mod object_loader;

use std::{collections::HashMap, io::Cursor, sync::Arc, time::Instant};

use bevy::{
    camera::visibility::VisibilityRange,
    pbr::ExtendedMaterial,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};
use wow_adt::{ParsedAdt, RootAdt, parse_adt};
use wow_mpq::PatchChain;
use wow_wdt::{WdtFile, WdtReader, chunks::MphdFlags, version::WowVersion};

use crate::{
    MPQResource,
    liquid_material::{LiquidMaterial, LiquidTexture, load_liquid_texture},
    map_loader::{
        geometry::{LiquidMesh, adt_liquids_to_meshes, adt_to_mesh},
        object_loader::{
            ObjectsLoadResult, PreparedObjectCache, WorldWmoPlacement, load_adt_objects,
            load_ground_effects, load_world_wmos, spawn_prepared_adt_objects,
        },
    },
    render_controls::RenderSettings,
    terrain_material::TerrainMaterial,
};

pub(crate) use editor::TerrainEditor;
pub use editor::TerrainEditorPlugin;
use editor::select_adt_chunk;
use ground_effects::{GroundEffectData, GroundEffectSource};
use material::{
    CachedTerrainTexture, PreparedMaterialMaps, global_layer_map, prepare_material_maps,
    update_texture_array,
};
pub(crate) use object_loader::animate_objects;
use object_loader::{AdtObjectPlacements, ObjectCache};

pub(super) const ADT_CELLS_PER_GRID: usize = 16;
const ADT_GRID_SIZE: usize = 64;
pub(super) const CHUNK_SIZE: f32 = 33.3334;
const ADT_SIZE: f32 = CHUNK_SIZE * ADT_CELLS_PER_GRID as f32;
const ADT_HALF_DIAGONAL: f32 = ADT_SIZE * std::f32::consts::FRAC_1_SQRT_2;
const STREAM_BUFFER: f32 = ADT_SIZE;
const DETAIL_CELL_SIZE: f32 = CHUNK_SIZE / 8.0;
const STREAM_UPDATE_DISTANCE: f32 = DETAIL_CELL_SIZE * 0.5;
const MAX_PENDING_ADTS: usize = 32;
const MAX_PENDING_OBJECT_ADTS: usize = 1;

pub struct MapOption {
    pub index: usize,
    pub name: String,
}

#[derive(Resource)]
pub struct MapSelection {
    pub maps: Vec<MapOption>,
    pub selected: usize,
    pub requested: Option<usize>,
}

impl MapSelection {
    pub fn new(maps: Vec<MapOption>, selected: usize) -> Self {
        Self {
            maps,
            selected,
            requested: None,
        }
    }

    pub fn selected_label(&self) -> &str {
        self.maps
            .iter()
            .find(|map| map.index == self.selected)
            .map(|map| map.name.as_str())
            .unwrap_or("Unknown map")
    }
}

pub fn available_maps(mpqs: &PatchChain) -> Vec<MapOption> {
    let map_buf = mpqs.read_file_concurrent("DBFilesClient\\Map.dbc").unwrap();
    let map_dbc = dbc_reader::read_dbc::<_, dbc_structs::Map>(&mut Cursor::new(map_buf)).unwrap();
    map_dbc
        .get_records()
        .iter()
        .enumerate()
        .filter_map(|(index, map)| {
            let directory = map.directory.to_str().ok()?.to_owned();
            let wdt_path = format!("World\\Maps\\{directory}\\{directory}.wdt");
            mpqs.read_file_concurrent(&wdt_path).ok()?;
            let localized_name = map.map_name_lang.locales[0].to_string_lossy();
            let name = if localized_name.is_empty() {
                directory.clone()
            } else {
                format!("{localized_name} ({directory})")
            };
            Some(MapOption { index, name })
        })
        .collect()
}

pub fn load_map(mpqs: &PatchChain, commands: &mut Commands, index: usize) -> Option<Transform> {
    let map_dbc = {
        info!("Searching for Map.dbc...");
        let map_buf = mpqs.read_file_concurrent("DBFilesClient\\Map.dbc").unwrap();
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
    let wdt_file_buf = match mpqs.read_file_concurrent(&wdt_file_path) {
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
        loading_objects: HashMap::new(),
        loading_ground_effects: None,
        texture_cache: HashMap::new(),
        texture_array: None,
        liquid_textures: Default::default(),
        object_cache: ObjectCache::default(),
        prepared_object_cache: Arc::new(PreparedObjectCache::default()),
        ground_effects: GroundEffectData::load(mpqs),
        ground_effects_root: None,
        rendered_ground_effects: None,
        world_wmos,
        world_wmos_root: None,
        loading_world_wmos: None,
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
    liquid_meshes: Vec<Handle<Mesh>>,
    liquid_materials: Vec<Handle<ExtendedMaterial<StandardMaterial, LiquidMaterial>>>,
    objects: Option<Entity>,
    object_placements: AdtObjectPlacements,
    ground_effect_source: GroundEffectSource,
}

#[derive(Clone, PartialEq, Eq)]
struct GroundEffectRequest {
    cell: [i32; 2],
    distance_bits: u32,
    adts: Vec<AdtPosition>,
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
    pos: AdtPosition,
    adt: RootAdt,
    mesh: Mesh,
    liquids: Vec<LiquidMesh>,
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
    loaded_adts: HashMap<AdtPosition, LoadedAdt>,
    loading_adts: HashMap<AdtPosition, Task<PreparedAdt>>,
    loading_objects: HashMap<AdtPosition, Task<ObjectsLoadResult>>,
    loading_ground_effects: Option<(GroundEffectRequest, Task<ObjectsLoadResult>)>,
    texture_cache: HashMap<String, CachedTerrainTexture>,
    texture_array: Option<Handle<Image>>,
    liquid_textures: [Option<LiquidTexture>; 4],
    object_cache: ObjectCache,
    prepared_object_cache: Arc<PreparedObjectCache>,
    ground_effects: GroundEffectData,
    ground_effects_root: Option<Entity>,
    rendered_ground_effects: Option<GroundEffectRequest>,
    world_wmos: Vec<WorldWmo>,
    world_wmos_root: Option<Entity>,
    loading_world_wmos: Option<Task<ObjectsLoadResult>>,
    world_wmos_loaded: bool,
    last_update_position: Option<Vec2>,
    loading: bool,
    metrics: TerrainLoadMetrics,
}

#[allow(clippy::too_many_arguments)]
pub fn switch_selected_map(
    mut commands: Commands,
    mut selection: ResMut<MapSelection>,
    terrain: Option<ResMut<TerrainMap>>,
    mpqs: Res<MPQResource>,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    mut liquid_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>>,
    mut object_materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(index) = selection.requested.take() else {
        return;
    };
    if index == selection.selected {
        return;
    }

    if let Some(mut terrain) = terrain {
        for (_, loaded_adt) in terrain.loaded_adts.drain() {
            if let Some(objects) = loaded_adt.objects {
                commands.entity(objects).despawn();
            }
            commands.entity(loaded_adt.entity).despawn();
            meshes.remove(loaded_adt.mesh.id());
            terrain_materials.remove(loaded_adt.material.id());
            for mesh in loaded_adt.liquid_meshes {
                meshes.remove(mesh.id());
            }
            for material in loaded_adt.liquid_materials {
                liquid_materials.remove(material.id());
            }
            for image in loaded_adt.images {
                images.remove(image.id());
            }
        }
        if let Some(root) = terrain.ground_effects_root.take() {
            commands.entity(root).despawn();
        }
        if let Some(root) = terrain.world_wmos_root.take() {
            commands.entity(root).despawn();
        }
        if let Some(texture_array) = terrain.texture_array.take() {
            images.remove(texture_array.id());
        }
        for texture in terrain.liquid_textures.iter_mut().filter_map(Option::take) {
            images.remove(texture.handle.id());
        }
        terrain.object_cache.unload_assets(
            &mut meshes,
            &mut object_materials,
            &mut liquid_materials,
            &mut images,
        );
    }

    commands.remove_resource::<TerrainMap>();
    load_map(&mpqs.mpqs, &mut commands, index);
    selection.selected = index;
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdtPosition {
    pub x: usize,
    pub y: usize,
}

impl AdtPosition {
    pub fn center(&self) -> Vec2 {
        const ADT_SIZE: f32 = CHUNK_SIZE * ADT_CELLS_PER_GRID as f32;
        Vec2::new(
            (31.5 - self.x as f32) * ADT_SIZE,
            (31.5 - self.y as f32) * ADT_SIZE,
        )
    }
}

#[derive(Component)]
pub struct RenderedObject;

#[derive(Component)]
pub struct RenderedGroundEffect;

pub fn stream_terrain_chunks(
    mut commands: Commands,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut terrain: ResMut<TerrainMap>,
    mpqs: Res<MPQResource>,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    mut liquid_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>>,
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
    if !camera_moved
        && !terrain.loading
        && terrain.loading_objects.is_empty()
        && terrain.loading_ground_effects.is_none()
        && terrain.world_wmos_loaded
        && !render_settings.is_changed()
    {
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
            let pos = AdtPosition { x, y };
            if !terrain.wdt.main.entries[y][x].has_adt()
                || terrain.loaded_adts.contains_key(&pos)
                || terrain.loading_adts.contains_key(&pos)
            {
                continue;
            }
            let distance_squared = pos.center().distance_squared(camera_position);
            if distance_squared <= load_distance.powi(2) {
                adts_to_load.push((distance_squared, x, y));
            }
        }
    }
    adts_to_load.sort_unstable_by(|left, right| left.0.total_cmp(&right.0));

    let adts_to_unload = terrain
        .loaded_adts
        .iter()
        .filter_map(|(&pos, _)| {
            (!editor.retains_adt(pos)
                && pos.center().distance_squared(camera_position) > unload_distance_squared)
                .then_some(pos)
        })
        .collect::<Vec<_>>();

    for coordinates in adts_to_unload {
        terrain.loading_objects.remove(&coordinates);
        let loaded_adt = terrain.loaded_adts.remove(&coordinates).unwrap();
        if let Some(objects) = loaded_adt.objects {
            commands.entity(objects).despawn();
        }
        commands.entity(loaded_adt.entity).despawn();
        meshes.remove(loaded_adt.mesh.id());
        terrain_materials.remove(loaded_adt.material.id());
        for mesh in loaded_adt.liquid_meshes {
            meshes.remove(mesh.id());
        }
        for material in loaded_adt.liquid_materials {
            liquid_materials.remove(material.id());
        }
        for image in loaded_adt.images {
            images.remove(image.id());
        }
    }

    terrain.loading_adts.retain(|&pos, _| {
        pos.center().distance_squared(camera_position) <= unload_distance_squared
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
            &mpqs.mpqs,
            &mut terrain_materials,
            &mut liquid_materials,
            &mut meshes,
            &mut images,
            &render_settings,
        );
    }

    stream_world_wmos(
        &mut commands,
        &mut terrain,
        &mpqs.mpqs,
        &mut object_materials,
        &mut liquid_materials,
        &mut meshes,
        &mut images,
        &render_settings,
    );
    if terrain.world_wmos_loaded {
        stream_ground_effects(
            &mut commands,
            &mut terrain,
            camera_position,
            &mpqs.mpqs,
            &mut object_materials,
            &mut liquid_materials,
            &mut meshes,
            &mut images,
            &render_settings,
        );
        stream_adt_objects(
            &mut commands,
            &mut terrain,
            camera_position,
            &mpqs.mpqs,
            &mut object_materials,
            &mut liquid_materials,
            &mut meshes,
            &mut images,
            &render_settings,
        );
    }

    let available_task_slots = MAX_PENDING_ADTS.saturating_sub(terrain.loading_adts.len());
    let adts_to_start = adts_to_load.len().min(available_task_slots);
    for (_, x, y) in adts_to_load.iter().take(adts_to_start).copied() {
        let pos = AdtPosition { x, y };
        let map_path = format!(
            "World\\Maps\\{}\\{}_{}_{}.adt",
            terrain.map_name, terrain.map_name, pos.x, pos.y
        );
        let has_big_alpha = terrain.has_big_alpha;
        let mpqs = mpqs.mpqs.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let map_file_buf = mpqs.read_file_async(&map_path).await.unwrap();
            let adt = parse_adt(&mut Cursor::new(map_file_buf)).unwrap();
            let ParsedAdt::Root(adt) = adt else { panic!() };
            let adt = *adt;
            let mesh = adt_to_mesh(&adt, pos.center());
            let liquids = adt_liquids_to_meshes(&adt, pos.center());
            let material_maps = prepare_material_maps(&adt, has_big_alpha);
            PreparedAdt {
                pos,
                adt,
                mesh,
                liquids,
                material_maps,
            }
        });
        terrain.loading_adts.insert(pos, task);
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

fn stream_world_wmos(
    commands: &mut Commands,
    terrain: &mut TerrainMap,
    mpqs: &Arc<PatchChain>,
    materials: &mut Assets<StandardMaterial>,
    liquid_materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    render_settings: &RenderSettings,
) {
    if terrain.world_wmos_loaded {
        return;
    }

    let ready = terrain.loading_world_wmos.as_mut().and_then(check_ready);
    if let Some(objects) = ready {
        terrain.loading_world_wmos = None;
        let entity = spawn_prepared_adt_objects(
            commands,
            objects,
            Vec2::ZERO,
            &terrain.prepared_object_cache,
            &mut terrain.object_cache,
            meshes,
            materials,
            liquid_materials,
            images,
            mpqs,
        );
        commands.entity(entity).insert((
            RenderedObject,
            if render_settings.render_objects {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        ));
        terrain.world_wmos_root = Some(entity);
        terrain.world_wmos_loaded = true;
        return;
    }

    if terrain.loading_world_wmos.is_none() {
        let placements = terrain
            .world_wmos
            .iter()
            .map(|world_wmo| WorldWmoPlacement {
                filename: world_wmo.filename.clone(),
                position: world_wmo.position,
                rotation: world_wmo.rotation,
                doodad_set: world_wmo.doodad_set,
                scale: world_wmo.scale,
            })
            .collect();
        let mpqs = mpqs.clone();
        let object_cache = terrain.prepared_object_cache.clone();
        terrain.loading_world_wmos = Some(
            AsyncComputeTaskPool::get()
                .spawn(async move { load_world_wmos(placements, &mpqs, &object_cache) }),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn finalize_adt(
    prepared: PreparedAdt,
    coordinates: AdtPosition,
    commands: &mut Commands,
    terrain: &mut TerrainMap,
    mpqs: &PatchChain,
    terrain_materials: &mut Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    liquid_materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
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
            unlit: !cfg!(feature = "realistic-lighting"),
            ..Default::default()
        },
        extension: TerrainMaterial {
            textures: texture_array,
            alpha_map: alpha_map.clone(),
            layer_map: layer_map.clone(),
            animation_map: animation_map.clone(),
        },
    });
    let center = prepared.pos.center();
    let entity = commands
        .spawn((
            prepared.pos,
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
    let mut liquid_mesh_handles = Vec::new();
    let mut liquid_material_handles = Vec::new();
    commands.entity(entity).with_children(|parent| {
        for liquid in prepared.liquids {
            let liquid_index = liquid.liquid_type as usize;
            let texture = terrain.liquid_textures[liquid_index]
                .get_or_insert_with(|| load_liquid_texture(liquid.liquid_type, mpqs, images))
                .clone();
            let mesh = meshes.add(liquid.mesh);
            let material = liquid_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: liquid_color(liquid.liquid_type),
                    alpha_mode: if liquid.liquid_type == wow_adt::chunks::mcnk::LiquidType::Magma {
                        AlphaMode::Opaque
                    } else {
                        AlphaMode::Blend
                    },
                    cull_mode: None,
                    perceptual_roughness: 0.3,
                    unlit: !cfg!(feature = "realistic-lighting"),
                    ..default()
                },
                extension: LiquidMaterial {
                    frames: texture.handle,
                    frame_count: texture.frame_count,
                    add_base_color: u32::from(matches!(
                        liquid.liquid_type,
                        wow_adt::chunks::mcnk::LiquidType::Water
                            | wow_adt::chunks::mcnk::LiquidType::Ocean
                    )),
                    uv_scroll: if matches!(
                        liquid.liquid_type,
                        wow_adt::chunks::mcnk::LiquidType::Magma
                            | wow_adt::chunks::mcnk::LiquidType::Slime
                    ) {
                        Vec2::X
                    } else {
                        Vec2::ZERO
                    },
                },
            });
            parent.spawn((
                Name::new(format!("Classic {:?} liquid", liquid.liquid_type)),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
            ));
            liquid_mesh_handles.push(mesh);
            liquid_material_handles.push(material);
        }
    });
    let seed = ((coordinates.x as u32) << 16) | coordinates.y as u32;
    let ground_effect_source = terrain.ground_effects.source(&prepared.adt, seed);
    let object_placements = AdtObjectPlacements::from_adt(&prepared.adt);
    terrain.loaded_adts.insert(
        coordinates,
        LoadedAdt {
            entity,
            mesh,
            material,
            images: [alpha_map, layer_map, animation_map],
            liquid_meshes: liquid_mesh_handles,
            liquid_materials: liquid_material_handles,
            objects: None,
            object_placements,
            ground_effect_source,
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

fn liquid_color(liquid_type: wow_adt::chunks::mcnk::LiquidType) -> Color {
    use wow_adt::chunks::mcnk::LiquidType;

    match liquid_type {
        LiquidType::Water => Color::srgba(0.3, 0.3, 0.4, 0.68),
        LiquidType::Ocean => Color::srgba(0.2, 0.3, 0.35, 0.72),
        LiquidType::Magma | LiquidType::Slime => Color::WHITE,
    }
}

#[allow(clippy::too_many_arguments)]
fn stream_ground_effects(
    commands: &mut Commands,
    terrain: &mut TerrainMap,
    camera_position: Vec2,
    mpqs: &Arc<PatchChain>,
    materials: &mut Assets<StandardMaterial>,
    liquid_materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    render_settings: &RenderSettings,
) {
    if !render_settings.render_ground_effects {
        terrain.loading_ground_effects = None;
        terrain.rendered_ground_effects = None;
        if let Some(root) = terrain.ground_effects_root.take() {
            commands.entity(root).despawn();
        }
        return;
    }

    if let Some(objects) = terrain
        .loading_ground_effects
        .as_mut()
        .and_then(|(_, task)| check_ready(task))
    {
        let (request, _) = terrain.loading_ground_effects.take().unwrap();
        let root = spawn_prepared_adt_objects(
            commands,
            objects,
            Vec2::ZERO,
            &terrain.prepared_object_cache,
            &mut terrain.object_cache,
            meshes,
            materials,
            liquid_materials,
            images,
            mpqs,
        );
        commands.entity(root).insert((
            Name::new("Ground effects"),
            RenderedGroundEffect,
            Visibility::Inherited,
        ));
        if let Some(previous_root) = terrain.ground_effects_root.replace(root) {
            commands.entity(previous_root).despawn();
        }
        terrain.rendered_ground_effects = Some(request);
    }

    let cell = [
        (camera_position.x / DETAIL_CELL_SIZE).round() as i32,
        (camera_position.y / DETAIL_CELL_SIZE).round() as i32,
    ];
    let sample_center = Vec2::new(
        cell[0] as f32 * DETAIL_CELL_SIZE,
        cell[1] as f32 * DETAIL_CELL_SIZE,
    );
    let ground_effect_distance = render_settings.ground_effect_distance;
    let source_distance = ground_effect_distance + ADT_HALF_DIAGONAL;
    let mut source_adts = terrain
        .loaded_adts
        .keys()
        .copied()
        .filter(|coordinates| {
            coordinates.center().distance_squared(sample_center) <= source_distance.powi(2)
        })
        .collect::<Vec<_>>();
    source_adts.sort_unstable_by_key(|coordinates| (coordinates.y, coordinates.x));
    let request = GroundEffectRequest {
        cell,
        distance_bits: ground_effect_distance.to_bits(),
        adts: source_adts,
    };
    let request_is_current = terrain.rendered_ground_effects.as_ref() == Some(&request)
        || terrain
            .loading_ground_effects
            .as_ref()
            .is_some_and(|(loading_request, _)| loading_request == &request);
    if request_is_current || terrain.loading_ground_effects.is_some() {
        return;
    }

    let placements = request
        .adts
        .iter()
        .flat_map(|coordinates| {
            terrain.ground_effects.placements_near(
                &terrain.loaded_adts[coordinates].ground_effect_source,
                sample_center,
                ground_effect_distance,
            )
        })
        .collect();
    let mpqs = mpqs.clone();
    let object_cache = terrain.prepared_object_cache.clone();
    let task = AsyncComputeTaskPool::get()
        .spawn(async move { load_ground_effects(placements, &mpqs, &object_cache) });
    terrain.loading_ground_effects = Some((request, task));
}

#[allow(clippy::too_many_arguments)]
fn stream_adt_objects(
    commands: &mut Commands,
    terrain: &mut TerrainMap,
    camera_position: Vec2,
    mpqs: &Arc<PatchChain>,
    materials: &mut Assets<StandardMaterial>,
    liquid_materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
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
                && coordinates.center().distance_squared(camera_position) > unload_distance_squared)
                .then(|| (coordinates, loaded_adt.objects.take().unwrap()))
        })
        .collect::<Vec<_>>();
    for (_, entity) in object_adts_to_unload {
        commands.entity(entity).despawn();
    }

    terrain.loading_objects.retain(|coordinates, _| {
        terrain.loaded_adts.contains_key(coordinates)
            && coordinates.center().distance_squared(camera_position) <= unload_distance_squared
    });

    let ready_objects = terrain
        .loading_objects
        .iter_mut()
        .filter_map(|(&coordinates, task)| check_ready(task).map(|objects| (coordinates, objects)))
        .collect::<Vec<_>>();
    for (coordinates, objects) in ready_objects {
        terrain.loading_objects.remove(&coordinates);
        let Some(loaded_adt) = terrain.loaded_adts.get_mut(&coordinates) else {
            continue;
        };
        if loaded_adt.objects.is_some()
            || coordinates.center().distance_squared(camera_position) > unload_distance_squared
        {
            continue;
        }
        loaded_adt.objects = Some(spawn_prepared_adt_objects(
            commands,
            objects,
            coordinates.center(),
            &terrain.prepared_object_cache,
            &mut terrain.object_cache,
            meshes,
            materials,
            liquid_materials,
            images,
            mpqs,
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

    let mut object_adts_to_load = terrain
        .loaded_adts
        .iter()
        .filter_map(|(&coordinates, loaded_adt)| {
            (loaded_adt.objects.is_none()
                && !terrain.loading_objects.contains_key(&coordinates)
                && coordinates.center().distance_squared(camera_position) <= load_distance.powi(2))
            .then_some((
                coordinates.center().distance_squared(camera_position),
                coordinates,
            ))
        })
        .collect::<Vec<_>>();
    object_adts_to_load.sort_unstable_by(|left, right| left.0.total_cmp(&right.0));
    let available_task_slots =
        MAX_PENDING_OBJECT_ADTS.saturating_sub(terrain.loading_objects.len());
    for (_, coordinates) in object_adts_to_load.into_iter().take(available_task_slots) {
        let object_placements = terrain.loaded_adts[&coordinates].object_placements.clone();
        let mpqs = mpqs.clone();
        let object_cache = terrain.prepared_object_cache.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            load_adt_objects(
                &object_placements,
                coordinates,
                coordinates.center(),
                &mpqs,
                &object_cache,
            )
        });
        terrain.loading_objects.insert(coordinates, task);
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
