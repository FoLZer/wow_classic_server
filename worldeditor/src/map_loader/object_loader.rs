use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    sync::{Arc, Mutex},
};

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use byteorder::{ByteOrder, LittleEndian};
use dashmap::DashMap;
use wow_adt::{DoodadPlacement, RootAdt, WmoPlacement};
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use wow_m2::parse_m2;
use wow_mpq::PatchChain;
use wow_wmo::{ParsedWmo, discover_wmo_chunks, parse_wmo};

use crate::{map_loader::AdtPosition, mpq_read_file};

use super::{ADT_CELLS_PER_GRID, ADT_GRID_SIZE, ADT_SIZE, CHUNK_SIZE};

const MAP_HALF_SIZE: f32 = ADT_GRID_SIZE as f32 * ADT_CELLS_PER_GRID as f32 * CHUNK_SIZE * 0.5;

enum PreparedObjectAsset {
    M2(Vec<PreparedObjectPart>),
    Wmo(PreparedWmoAsset),
}

#[derive(Clone)]
enum ObjectAsset {
    M2(Vec<ObjectPart>),
    Wmo(WmoAsset),
}

struct PreparedObjectPart {
    mesh: Mutex<Option<Mesh>>,
    texture_key: Option<String>,
    double_sided: bool,
    opacity: f32,
    alpha_mode: AlphaMode,
}

#[derive(Clone)]
struct ObjectPart {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

struct PreparedWmoAsset {
    parts: Vec<PreparedObjectPart>,
    doodad_sets: Vec<Vec<PreparedWmoDoodad>>,
}

#[derive(Clone)]
struct WmoAsset {
    parts: Vec<ObjectPart>,
    doodad_sets: Vec<Vec<WmoDoodad>>,
}

struct PreparedWmoDoodad {
    filename: String,
    asset: Arc<PreparedObjectAsset>,
    transform: Transform,
}

#[derive(Clone)]
struct WmoDoodad {
    filename: String,
    asset: Arc<ObjectAsset>,
    transform: Transform,
}

#[derive(Default)]
pub(super) struct PreparedObjectCache {
    assets: DashMap<String, Option<Arc<PreparedObjectAsset>>>,
    textures: DashMap<String, Option<Image>>,
}

#[derive(Default)]
pub(super) struct ObjectCache {
    assets: HashMap<String, Option<Arc<ObjectAsset>>>,
    textures: HashMap<String, Option<Handle<Image>>>,
}

#[derive(Clone)]
pub(super) struct AdtObjectPlacements {
    models: Vec<String>,
    model_indices: Vec<u32>,
    doodad_placements: Vec<DoodadPlacement>,
    wmos: Vec<String>,
    wmo_indices: Vec<u32>,
    wmo_placements: Vec<WmoPlacement>,
}

impl AdtObjectPlacements {
    pub(super) fn from_adt(adt: &RootAdt) -> Self {
        Self {
            models: adt.models.clone(),
            model_indices: adt.model_indices.clone(),
            doodad_placements: adt.doodad_placements.clone(),
            wmos: adt.wmos.clone(),
            wmo_indices: adt.wmo_indices.clone(),
            wmo_placements: adt.wmo_placements.clone(),
        }
    }
}

pub(super) struct ObjectsLoadResult {
    doodads: Vec<(String, Arc<PreparedObjectAsset>, Transform)>,
    wmos: Vec<(String, Arc<PreparedObjectAsset>, usize, Transform)>,
}

pub(super) struct WorldWmoPlacement {
    pub filename: String,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub doodad_set: u16,
    pub scale: u16,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_prepared_adt_objects(
    commands: &mut Commands,
    objects: ObjectsLoadResult,
    adt_center: Vec2,
    prepared_cache: &PreparedObjectCache,
    cache: &mut ObjectCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Entity {
    let doodads = objects
        .doodads
        .into_iter()
        .filter_map(|(filename, asset, transform)| {
            finalize_object_asset(
                &filename,
                asset,
                prepared_cache,
                cache,
                meshes,
                materials,
                images,
            )
            .map(|asset| (filename, asset, transform))
        })
        .collect::<Vec<_>>();
    let wmos = objects
        .wmos
        .into_iter()
        .filter_map(|(filename, asset, doodad_set, transform)| {
            finalize_object_asset(
                &filename,
                asset,
                prepared_cache,
                cache,
                meshes,
                materials,
                images,
            )
            .map(|asset| (filename, asset, doodad_set, transform))
        })
        .collect::<Vec<_>>();

    let objects_entity = commands
        .spawn((
            Name::new("ADT objects"),
            adt_object_root_transform(adt_center),
            Visibility::default(),
        ))
        .id();
    commands.entity(objects_entity).with_children(|parent| {
        for (filename, asset, transform) in doodads {
            let ObjectAsset::M2(parts) = asset.as_ref() else {
                continue;
            };
            parent
                .spawn((Name::new(filename), transform, Visibility::default()))
                .with_children(|object| spawn_parts(object, parts));
        }
        for (filename, asset, doodad_set, transform) in wmos {
            let ObjectAsset::Wmo(asset) = asset.as_ref() else {
                continue;
            };
            parent
                .spawn((Name::new(filename), transform, Visibility::default()))
                .with_children(|object| spawn_wmo_contents(object, asset, doodad_set));
        }
    });
    objects_entity
}

pub(super) fn load_adt_objects(
    adt: &AdtObjectPlacements,
    adt_coordinates: AdtPosition,
    adt_center: Vec2,
    mpqs: &PatchChain,
    cache: &PreparedObjectCache,
) -> ObjectsLoadResult {
    let model_filenames = index_filenames(&adt.models);
    let wmo_filenames = index_filenames(&adt.wmos);
    let mut doodads = Vec::new();
    for placement in &adt.doodad_placements {
        if placement_owner(placement.position) != adt_coordinates {
            continue;
        }
        let Some(filename) = resolve_filename(
            &model_filenames,
            &adt.model_indices,
            placement.name_id as usize,
        ) else {
            warn!("Doodad {} references an invalid model", placement.unique_id);
            continue;
        };
        if let Some(asset) = load_object(filename, mpqs, cache)
            && matches!(asset.as_ref(), PreparedObjectAsset::M2(_))
        {
            doodads.push((
                filename.to_owned(),
                asset,
                doodad_transform(placement, adt_center),
            ));
        }
    }

    let mut wmos = Vec::new();
    for placement in &adt.wmo_placements {
        if placement_owner(placement.position) != adt_coordinates {
            continue;
        }
        let Some(filename) =
            resolve_filename(&wmo_filenames, &adt.wmo_indices, placement.name_id as usize)
        else {
            warn!("WMO {} references an invalid model", placement.unique_id);
            continue;
        };
        if let Some(asset) = load_object(filename, mpqs, cache)
            && matches!(asset.as_ref(), PreparedObjectAsset::Wmo(_))
        {
            wmos.push((
                filename.to_owned(),
                asset,
                placement.doodad_set as usize,
                wmo_transform(placement, adt_center),
            ));
        }
    }

    ObjectsLoadResult { doodads, wmos }
}

pub(super) fn load_world_wmos(
    placements: Vec<WorldWmoPlacement>,
    mpqs: &PatchChain,
    cache: &PreparedObjectCache,
) -> ObjectsLoadResult {
    let wmos = placements
        .into_iter()
        .filter_map(|placement| {
            let asset = load_object(&placement.filename, mpqs, cache)?;
            if !matches!(asset.as_ref(), PreparedObjectAsset::Wmo(_)) {
                return None;
            }
            let scale = if placement.scale == 0 {
                1.0
            } else {
                f32::from(placement.scale) / 1024.0
            };
            let transform =
                placement_transform(placement.position, placement.rotation, scale, Vec2::ZERO);
            Some((
                placement.filename,
                asset,
                placement.doodad_set as usize,
                transform,
            ))
        })
        .collect();
    ObjectsLoadResult {
        doodads: Vec::new(),
        wmos,
    }
}

fn spawn_parts(parent: &mut ChildSpawnerCommands, parts: &[ObjectPart]) {
    for part in parts {
        parent.spawn((
            Mesh3d(part.mesh.clone()),
            MeshMaterial3d(part.material.clone()),
        ));
    }
}

fn spawn_wmo_contents(parent: &mut ChildSpawnerCommands, asset: &WmoAsset, doodad_set: usize) {
    spawn_parts(parent, &asset.parts);
    if let Some(doodads) = asset.doodad_sets.first() {
        spawn_wmo_doodads(parent, doodads);
    }
    if doodad_set != 0
        && let Some(doodads) = asset.doodad_sets.get(doodad_set)
    {
        spawn_wmo_doodads(parent, doodads);
    }
}

fn spawn_wmo_doodads(parent: &mut ChildSpawnerCommands, doodads: &[WmoDoodad]) {
    for doodad in doodads {
        let ObjectAsset::M2(parts) = doodad.asset.as_ref() else {
            continue;
        };
        parent
            .spawn((
                Name::new(doodad.filename.clone()),
                doodad.transform,
                Visibility::default(),
            ))
            .with_children(|doodad_entity| spawn_parts(doodad_entity, parts));
    }
}

#[allow(clippy::too_many_arguments)]
fn finalize_object_asset(
    filename: &str,
    prepared: Arc<PreparedObjectAsset>,
    prepared_cache: &PreparedObjectCache,
    cache: &mut ObjectCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Option<Arc<ObjectAsset>> {
    let key = filename.to_ascii_lowercase();
    if let Some(asset) = cache.assets.get(&key) {
        return asset.clone();
    }

    let asset = match prepared.as_ref() {
        PreparedObjectAsset::M2(parts) => ObjectAsset::M2(finalize_object_parts(
            parts,
            Color::srgb(0.32, 0.48, 0.24),
            prepared_cache,
            cache,
            meshes,
            materials,
            images,
        )),
        PreparedObjectAsset::Wmo(wmo) => {
            let parts = finalize_object_parts(
                &wmo.parts,
                Color::srgb(0.48, 0.45, 0.39),
                prepared_cache,
                cache,
                meshes,
                materials,
                images,
            );
            let doodad_sets = wmo
                .doodad_sets
                .iter()
                .map(|doodads| {
                    doodads
                        .iter()
                        .filter_map(|doodad| {
                            finalize_object_asset(
                                &doodad.filename,
                                doodad.asset.clone(),
                                prepared_cache,
                                cache,
                                meshes,
                                materials,
                                images,
                            )
                            .map(|asset| WmoDoodad {
                                filename: doodad.filename.clone(),
                                asset,
                                transform: doodad.transform,
                            })
                        })
                        .collect()
                })
                .collect();
            ObjectAsset::Wmo(WmoAsset { parts, doodad_sets })
        }
    };
    let asset = Arc::new(asset);
    cache.assets.insert(key, Some(asset.clone()));
    Some(asset)
}

#[allow(clippy::too_many_arguments)]
fn finalize_object_parts(
    parts: &[PreparedObjectPart],
    fallback_color: Color,
    prepared_cache: &PreparedObjectCache,
    cache: &mut ObjectCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Vec<ObjectPart> {
    parts
        .iter()
        .filter_map(|part| {
            let texture = part.texture_key.as_deref().and_then(|key| {
                if let Some(texture) = cache.textures.get(key) {
                    return texture.clone();
                }
                let texture = prepared_cache
                    .textures
                    .get_mut(key)
                    .and_then(|mut v| v.take())
                    .map(|image| images.add(image));
                cache.textures.insert(key.to_owned(), texture.clone());
                texture
            });
            let mut material = StandardMaterial {
                base_color: if texture.is_some() {
                    Color::WHITE.with_alpha(part.opacity)
                } else {
                    fallback_color.with_alpha(part.opacity)
                },
                base_color_texture: texture,
                alpha_mode: part.alpha_mode,
                unlit: true,
                ..default()
            };
            if part.double_sided {
                material.cull_mode = None;
            }
            Some(ObjectPart {
                mesh: meshes.add(part.mesh.lock().unwrap().take()?),
                material: materials.add(material),
            })
        })
        .collect()
}

fn load_object(
    filename: &str,
    mpqs: &PatchChain,
    cache: &PreparedObjectCache,
) -> Option<Arc<PreparedObjectAsset>> {
    let key = filename.to_ascii_lowercase();
    if let Some(asset) = cache.assets.get(&key) {
        return asset.clone();
    }

    let asset = if key.ends_with(".wmo") {
        load_wmo(filename, mpqs, cache).map(PreparedObjectAsset::Wmo)
    } else {
        load_m2(filename, mpqs, cache).map(PreparedObjectAsset::M2)
    }
    .map(Arc::new);
    if asset.is_none() {
        warn!("Unable to load world object {filename}");
    }
    cache.assets.insert(key, asset.clone());
    asset
}

fn placement_owner(position: [f32; 3]) -> AdtPosition {
    AdtPosition {
        x: (position[0] / ADT_SIZE).floor() as usize,
        y: (position[2] / ADT_SIZE).floor() as usize,
    }
}

fn load_m2(
    filename: &str,
    mpqs: &PatchChain,
    cache: &PreparedObjectCache,
) -> Option<Vec<PreparedObjectPart>> {
    let data = match mpq_read_file(mpqs, filename) {
        Ok(data) => data,
        Err(error) => {
            let Some(fallback) = m2_fallback_filename(filename) else {
                warn!("Unable to read M2 {filename}: {error}");
                return None;
            };
            mpq_read_file(mpqs, &fallback)
                .inspect_err(|fallback_error| {
                    warn!("Unable to read M2 {filename} (also tried {fallback}): {fallback_error}");
                })
                .ok()?
        }
    };
    let mesh_data = library_m2_mesh_data(&data).or_else(|library_error| {
        classic_m2_mesh_data(&data).map_err(|classic_error| {
            warn!(
                "Unable to parse M2 {filename}: {library_error}; Classic fallback: {classic_error}"
            );
        })
    });
    let mesh_data = mesh_data.ok()?;
    if mesh_data.positions.is_empty() || mesh_data.indices.is_empty() {
        return None;
    }
    let unresolved_types = mesh_data
        .batches
        .iter()
        .filter(|batch| batch.texture.is_none())
        .map(|batch| batch.texture_type)
        .collect::<Vec<_>>();
    if !unresolved_types.is_empty() {
        debug!("M2 {filename} has unresolved replacement textures {unresolved_types:?}");
    }
    Some(build_object_parts(mesh_data, mpqs, cache))
}

struct M2MeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    batches: Vec<ObjectBatch>,
}

struct ObjectBatch {
    indices: Vec<u32>,
    texture: Option<String>,
    texture_type: u32,
    alpha_mode: AlphaMode,
    opacity: f32,
    double_sided: bool,
}

fn library_m2_mesh_data(data: &[u8]) -> Result<M2MeshData, String> {
    let format = parse_m2(&mut Cursor::new(data)).map_err(|error| error.to_string())?;
    let model = format.model();
    let skin = model
        .parse_embedded_skin(data, 0)
        .map_err(|error| error.to_string())?;
    let indices = skin
        .get_resolved_indices()
        .into_iter()
        .map(u32::from)
        .collect::<Vec<_>>();
    let batches = skin
        .batches()
        .iter()
        .filter_map(|batch| {
            let submesh = skin.submeshes().get(batch.skin_section_index as usize)?;
            let start = submesh.triangle_start as usize;
            let end = start.checked_add(submesh.triangle_count as usize)?;
            let texture_index = model
                .raw_data
                .texture_lookup_table
                .get(batch.texture_combo_index as usize)
                .copied()? as usize;
            let texture = model
                .textures
                .get(texture_index)
                .map(|texture| texture.filename.string.to_string_lossy())
                .filter(|filename| !filename.is_empty());
            let material = model.materials.get(batch.material_index as usize);
            let opacity = model
                .raw_data
                .transparency_lookup_table
                .get(batch.texture_weight_combo_index as usize)
                .and_then(|animation_index| {
                    model
                        .raw_data
                        .transparency_animation_data
                        .iter()
                        .find(|animation| animation.animation_index == *animation_index as usize)
                })
                .and_then(|animation| fixed_i16_alpha(&animation.values))
                .unwrap_or(1.0);
            Some(ObjectBatch {
                indices: indices.get(start..end)?.to_vec(),
                texture,
                texture_type: model
                    .textures
                    .get(texture_index)
                    .map_or(0, |texture| texture.texture_type as u32),
                alpha_mode: alpha_mode_with_opacity(
                    material.map_or(0, |material| material.blend_mode.bits()),
                    opacity,
                ),
                opacity,
                double_sided: material.is_some_and(|material| material.flags.bits() & 0x04 != 0),
            })
        })
        .collect();
    Ok(M2MeshData {
        positions: model
            .vertices
            .iter()
            .map(|vertex| wow_to_bevy(vertex.position.x, vertex.position.y, vertex.position.z))
            .collect(),
        normals: model
            .vertices
            .iter()
            .map(|vertex| wow_to_bevy(vertex.normal.x, vertex.normal.y, vertex.normal.z))
            .collect(),
        uvs: model
            .vertices
            .iter()
            .map(|vertex| [vertex.tex_coords.x, vertex.tex_coords.y])
            .collect(),
        indices,
        batches,
    })
}

fn classic_m2_mesh_data(data: &[u8]) -> Result<M2MeshData, String> {
    const VERTICES_DESCRIPTOR_OFFSET: usize = 68;
    const VIEWS_DESCRIPTOR_OFFSET: usize = 76;
    const TEXTURES_DESCRIPTOR_OFFSET: usize = 92;
    const TRANSPARENCY_ANIMATIONS_DESCRIPTOR_OFFSET: usize = 100;
    const RENDER_FLAGS_DESCRIPTOR_OFFSET: usize = 132;
    const TEXTURE_LOOKUP_DESCRIPTOR_OFFSET: usize = 148;
    const TRANSPARENCY_LOOKUP_DESCRIPTOR_OFFSET: usize = 164;
    const CLASSIC_VERTEX_SIZE: usize = 48;
    const SKIN_BATCH_SIZE: usize = 24;
    const TRANSPARENCY_ANIMATION_SIZE: usize = 28;

    if data.get(..4) != Some(b"MD20") {
        return Err("not a legacy MD20 model".to_owned());
    }
    let version = read_u32(data, 4)?;
    if !(256..=263).contains(&version) {
        return Err(format!("unsupported embedded-skin version {version}"));
    }

    let (vertex_count, vertex_offset) = read_array_descriptor(data, VERTICES_DESCRIPTOR_OFFSET)?;
    let (view_count, view_offset) = read_array_descriptor(data, VIEWS_DESCRIPTOR_OFFSET)?;
    if view_count == 0 {
        return Err("model has no embedded skin views".to_owned());
    }
    checked_array_range(data, vertex_offset, vertex_count, CLASSIC_VERTEX_SIZE)?;

    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);
    for index in 0..vertex_count {
        let offset = vertex_offset + index * CLASSIC_VERTEX_SIZE;
        positions.push(wow_to_bevy(
            read_f32(data, offset)?,
            read_f32(data, offset + 4)?,
            read_f32(data, offset + 8)?,
        ));
        normals.push(wow_to_bevy(
            read_f32(data, offset + 20)?,
            read_f32(data, offset + 24)?,
            read_f32(data, offset + 28)?,
        ));
        uvs.push([read_f32(data, offset + 32)?, read_f32(data, offset + 36)?]);
    }

    let (triangle_count, triangle_offset) = read_array_descriptor(data, view_offset + 8)?;
    if !triangle_count.is_multiple_of(3) {
        return Err(format!(
            "skin index count {triangle_count} is not a triangle list"
        ));
    }
    let indices = read_u16_array(data, triangle_offset, triangle_count)?
        .into_iter()
        .map(u32::from)
        .collect::<Vec<_>>();
    if indices.iter().any(|&index| index as usize >= vertex_count) {
        return Err("skin references a vertex outside the model".to_owned());
    }
    if positions.iter().flatten().any(|value| !value.is_finite())
        || normals.iter().flatten().any(|value| !value.is_finite())
        || uvs.iter().flatten().any(|value| !value.is_finite())
    {
        return Err("model contains non-finite vertex data".to_owned());
    }

    let textures = read_m2_textures(data, TEXTURES_DESCRIPTOR_OFFSET)?;
    let (texture_lookup_count, texture_lookup_offset) =
        read_array_descriptor(data, TEXTURE_LOOKUP_DESCRIPTOR_OFFSET)?;
    let texture_lookup = read_u16_array(data, texture_lookup_offset, texture_lookup_count)?;
    let (transparency_animation_count, transparency_animation_offset) =
        read_array_descriptor(data, TRANSPARENCY_ANIMATIONS_DESCRIPTOR_OFFSET)?;
    checked_array_range(
        data,
        transparency_animation_offset,
        transparency_animation_count,
        TRANSPARENCY_ANIMATION_SIZE,
    )?;
    let (transparency_lookup_count, transparency_lookup_offset) =
        read_array_descriptor(data, TRANSPARENCY_LOOKUP_DESCRIPTOR_OFFSET)?;
    let transparency_lookup =
        read_u16_array(data, transparency_lookup_offset, transparency_lookup_count)?;
    let (render_flag_count, render_flag_offset) =
        read_array_descriptor(data, RENDER_FLAGS_DESCRIPTOR_OFFSET)?;
    checked_array_range(data, render_flag_offset, render_flag_count, 4)?;

    let (submesh_count, submesh_offset) = read_array_descriptor(data, view_offset + 24)?;
    let submesh_size = if version < 260 { 32 } else { 48 };
    checked_array_range(data, submesh_offset, submesh_count, submesh_size)?;
    let (batch_count, batch_offset) = read_array_descriptor(data, view_offset + 32)?;
    checked_array_range(data, batch_offset, batch_count, SKIN_BATCH_SIZE)?;
    let batches = (0..batch_count)
        .filter_map(|batch_index| {
            let offset = batch_offset + batch_index * SKIN_BATCH_SIZE;
            let section_index = read_u16(data, offset + 4).ok()? as usize;
            let material_index = read_u16(data, offset + 10).ok()? as usize;
            let texture_combo_index = read_u16(data, offset + 16).ok()? as usize;
            let submesh = submesh_offset + section_index.checked_mul(submesh_size)?;
            let triangle_start = read_u16(data, submesh + 8).ok()? as usize;
            let triangle_count = read_u16(data, submesh + 10).ok()? as usize;
            let end = triangle_start.checked_add(triangle_count)?;
            let texture_index = *texture_lookup.get(texture_combo_index)? as usize;
            let material_offset = render_flag_offset + material_index.checked_mul(4)?;
            let flags = read_u16(data, material_offset).ok()?;
            let blend_mode = read_u16(data, material_offset + 2).ok()?;
            let texture_weight_combo_index = read_u16(data, offset + 20).ok()? as usize;
            let opacity = classic_m2_opacity(
                data,
                &transparency_lookup,
                transparency_animation_offset,
                transparency_animation_count,
                texture_weight_combo_index,
            )
            .unwrap_or(1.0);
            Some(ObjectBatch {
                indices: indices.get(triangle_start..end)?.to_vec(),
                texture: textures
                    .get(texture_index)
                    .and_then(|texture| texture.filename.clone()),
                texture_type: textures
                    .get(texture_index)
                    .map_or(0, |texture| texture.texture_type),
                alpha_mode: alpha_mode_with_opacity(blend_mode, opacity),
                opacity,
                double_sided: flags & 0x04 != 0,
            })
        })
        .collect();

    Ok(M2MeshData {
        positions,
        normals,
        uvs,
        indices,
        batches,
    })
}

struct M2TextureReference {
    texture_type: u32,
    filename: Option<String>,
}

fn read_m2_textures(
    data: &[u8],
    descriptor_offset: usize,
) -> Result<Vec<M2TextureReference>, String> {
    const TEXTURE_SIZE: usize = 16;

    let (count, offset) = read_array_descriptor(data, descriptor_offset)?;
    checked_array_range(data, offset, count, TEXTURE_SIZE)?;
    (0..count)
        .map(|index| {
            let texture_offset = offset + index * TEXTURE_SIZE;
            let texture_type = read_u32(data, texture_offset)?;
            let filename_count = read_u32(data, texture_offset + 8)? as usize;
            let filename_offset = read_u32(data, texture_offset + 12)? as usize;
            if filename_count == 0 {
                return Ok(M2TextureReference {
                    texture_type,
                    filename: None,
                });
            }
            let bytes = data
                .get(filename_offset..filename_offset + filename_count)
                .ok_or_else(|| "M2 texture filename exceeds file size".to_owned())?;
            let filename = String::from_utf8_lossy(bytes)
                .trim_end_matches('\0')
                .to_owned();
            Ok(M2TextureReference {
                texture_type,
                filename: (!filename.is_empty()).then_some(filename),
            })
        })
        .collect()
}

fn read_array_descriptor(data: &[u8], offset: usize) -> Result<(usize, usize), String> {
    Ok((
        read_u32(data, offset)? as usize,
        read_u32(data, offset + 4)? as usize,
    ))
}

fn checked_array_range(
    data: &[u8],
    offset: usize,
    count: usize,
    item_size: usize,
) -> Result<(), String> {
    let size = count
        .checked_mul(item_size)
        .ok_or_else(|| "array size overflow".to_owned())?;
    let end = offset
        .checked_add(size)
        .ok_or_else(|| "array offset overflow".to_owned())?;
    if end > data.len() {
        return Err(format!(
            "array range {offset}..{end} exceeds file size {}",
            data.len()
        ));
    }
    Ok(())
}

fn read_u16_array(data: &[u8], offset: usize, count: usize) -> Result<Vec<u16>, String> {
    checked_array_range(data, offset, count, 2)?;
    Ok((0..count)
        .map(|index| LittleEndian::read_u16(&data[offset + index * 2..]))
        .collect())
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, String> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| format!("missing u16 at offset {offset}"))?;
    Ok(LittleEndian::read_u16(bytes))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| format!("missing u32 at offset {offset}"))?;
    Ok(LittleEndian::read_u32(bytes))
}

fn read_f32(data: &[u8], offset: usize) -> Result<f32, String> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| format!("missing f32 at offset {offset}"))?;
    Ok(LittleEndian::read_f32(bytes))
}

#[allow(clippy::too_many_arguments)]
fn build_object_parts(
    mut mesh_data: M2MeshData,
    mpqs: &PatchChain,
    cache: &PreparedObjectCache,
) -> Vec<PreparedObjectPart> {
    if mesh_data.batches.is_empty() {
        mesh_data.batches.push(ObjectBatch {
            indices: mesh_data.indices.clone(),
            texture: None,
            texture_type: 0,
            alpha_mode: AlphaMode::Opaque,
            opacity: 1.0,
            double_sided: false,
        });
    }

    mesh_data
        .batches
        .into_iter()
        .filter(|batch| !batch.indices.is_empty())
        .map(|batch| {
            let texture = batch
                .texture
                .as_deref()
                .map(|filename| load_object_texture(filename, mpqs, cache));
            PreparedObjectPart {
                mesh: Mutex::new(Some(build_mesh(
                    mesh_data.positions.clone(),
                    mesh_data.normals.clone(),
                    mesh_data.uvs.clone(),
                    batch.indices,
                ))),
                texture_key: texture,
                double_sided: batch.double_sided,
                opacity: batch.opacity,
                alpha_mode: batch.alpha_mode,
            }
        })
        .collect()
}

fn load_object_texture(filename: &str, mpqs: &PatchChain, cache: &PreparedObjectCache) -> String {
    let key = filename.to_ascii_lowercase();
    if cache.textures.contains_key(&key) {
        return key;
    }

    let texture = mpq_read_file(mpqs, filename)
        .map_err(|error| warn!("Unable to read object texture {filename}: {error}"))
        .ok()
        .and_then(|data| {
            load_blp_from_buf(&data)
                .map_err(|error| warn!("Unable to parse object texture {filename}: {error}"))
                .ok()
        })
        .and_then(|blp| {
            blp_to_image(&blp, 0)
                .map_err(|error| warn!("Unable to decode object texture {filename}: {error}"))
                .ok()
                .map(|decoded| (blp, decoded))
        })
        .map(|(blp, decoded)| {
            let mut image = Image::new(
                Extent3d {
                    width: blp.header.width,
                    height: blp.header.height,
                    ..default()
                },
                TextureDimension::D2,
                decoded.into_rgba8().into_vec(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                mag_filter: ImageFilterMode::Linear,
                min_filter: ImageFilterMode::Linear,
                mipmap_filter: ImageFilterMode::Linear,
                ..default()
            });
            image
        });
    cache.textures.insert(key.clone(), texture);
    key
}

fn alpha_mode(blend_mode: u16) -> AlphaMode {
    match blend_mode {
        0 => AlphaMode::Opaque,
        1 => AlphaMode::Mask(0.5),
        2 => AlphaMode::Blend,
        3 | 4 | 7 => AlphaMode::Add,
        5 | 6 => AlphaMode::Multiply,
        _ => AlphaMode::Blend,
    }
}

fn alpha_mode_with_opacity(blend_mode: u16, opacity: f32) -> AlphaMode {
    match alpha_mode(blend_mode) {
        AlphaMode::Opaque if opacity < 1.0 => AlphaMode::Blend,
        alpha_mode => alpha_mode,
    }
}

fn classic_m2_opacity(
    data: &[u8],
    transparency_lookup: &[u16],
    animation_offset: usize,
    animation_count: usize,
    texture_weight_combo_index: usize,
) -> Option<f32> {
    const TRANSPARENCY_ANIMATION_SIZE: usize = 28;
    const VALUES_DESCRIPTOR_OFFSET: usize = 20;

    let animation_index = *transparency_lookup.get(texture_weight_combo_index)? as usize;
    if animation_index >= animation_count {
        return None;
    }
    let animation =
        animation_offset.checked_add(animation_index.checked_mul(TRANSPARENCY_ANIMATION_SIZE)?)?;
    let (value_count, value_offset) =
        read_array_descriptor(data, animation + VALUES_DESCRIPTOR_OFFSET).ok()?;
    (value_count > 0)
        .then(|| data.get(value_offset..value_offset + 2))
        .flatten()
        .and_then(fixed_i16_alpha)
}

fn fixed_i16_alpha(values: &[u8]) -> Option<f32> {
    let value = i16::from_le_bytes(values.get(..2)?.try_into().ok()?);
    Some((f32::from(value) / f32::from(i16::MAX)).clamp(0.0, 1.0))
}

fn load_wmo(
    filename: &str,
    mpqs: &PatchChain,
    cache: &PreparedObjectCache,
) -> Option<PreparedWmoAsset> {
    let root_data = mpq_read_file(mpqs, filename).ok()?;
    let doodad_names = wmo_doodad_names(&root_data);
    let ParsedWmo::Root(root) = parse_wmo(&mut Cursor::new(&root_data)).ok()? else {
        return None;
    };
    let stem = filename.get(..filename.len().checked_sub(4)?)?;
    let mut object_parts = Vec::new();
    let mut visible_doodads = HashSet::new();
    for group_index in 0..root.n_groups {
        let group_filename = format!("{stem}_{group_index:03}.wmo");
        let Some(group_data) = mpq_read_file(mpqs, &group_filename).ok() else {
            warn!("Unable to read WMO group {group_filename}");
            continue;
        };
        let Some(ParsedWmo::Group(group)) = parse_wmo(&mut Cursor::new(group_data)).ok() else {
            warn!("Unable to parse WMO group {group_filename}");
            continue;
        };
        visible_doodads.extend(group.doodad_refs.iter().copied().map(usize::from));
        if group.vertex_positions.is_empty() || group.vertex_indices.is_empty() {
            continue;
        }
        let positions = group
            .vertex_positions
            .iter()
            .map(|vertex| wow_to_bevy(vertex.x, vertex.y, vertex.z))
            .collect::<Vec<_>>();
        let normals = group
            .vertex_positions
            .iter()
            .enumerate()
            .map(|(index, _)| {
                group
                    .vertex_normals
                    .get(index)
                    .map_or([0.0, 1.0, 0.0], |normal| {
                        wow_to_bevy(normal.x, normal.y, normal.z)
                    })
            })
            .collect();
        let uvs = group
            .vertex_positions
            .iter()
            .enumerate()
            .map(|(index, _)| {
                group
                    .texture_coords
                    .get(index)
                    .map_or([0.0, 0.0], |uv| [uv.u, uv.v])
            })
            .collect();
        let indices = group
            .vertex_indices
            .iter()
            .copied()
            .map(u32::from)
            .collect::<Vec<_>>();
        let batches = group
            .render_batches
            .iter()
            .filter_map(|batch| {
                let start = batch.start_index as usize;
                let end = start.checked_add(batch.count as usize)?;
                let material = root.materials.get(batch.material_id as usize)?;
                let texture_index = root
                    .texture_offset_index_map
                    .get(&material.texture_1)
                    .copied()? as usize;
                Some(ObjectBatch {
                    indices: indices.get(start..end)?.to_vec(),
                    texture: root.textures.get(texture_index).cloned(),
                    texture_type: 0,
                    alpha_mode: alpha_mode(material.blend_mode as u16),
                    opacity: 1.0,
                    double_sided: material.flags & 0x04 != 0,
                })
            })
            .collect();
        object_parts.extend(build_object_parts(
            M2MeshData {
                positions,
                normals,
                uvs,
                indices,
                batches,
            },
            mpqs,
            cache,
        ));
    }

    let doodad_sets = root
        .doodad_sets
        .iter()
        .map(|set| {
            let start = set.start_index as usize;
            let end = start
                .saturating_add(set.count as usize)
                .min(root.doodad_defs.len());
            (start..end)
                .filter(|index| visible_doodads.is_empty() || visible_doodads.contains(index))
                .filter_map(|index| {
                    let definition = root.doodad_defs.get(index)?;
                    let filename = doodad_names.get(&definition.name_index())?.clone();
                    let asset = load_object(&filename, mpqs, cache)?;
                    if !matches!(asset.as_ref(), PreparedObjectAsset::M2(_)) {
                        return None;
                    }
                    Some(PreparedWmoDoodad {
                        filename,
                        asset,
                        transform: wmo_doodad_transform(
                            definition.position,
                            definition.orientation,
                            definition.scale,
                        ),
                    })
                })
                .collect()
        })
        .collect();

    (!object_parts.is_empty()).then_some(PreparedWmoAsset {
        parts: object_parts,
        doodad_sets,
    })
}

fn wmo_doodad_names(data: &[u8]) -> HashMap<u32, String> {
    let Ok(discovery) = discover_wmo_chunks(&mut Cursor::new(data)) else {
        return HashMap::new();
    };
    let Some(chunk) = discovery
        .chunks
        .iter()
        .find(|chunk| chunk.id.as_str() == "MODN")
    else {
        return HashMap::new();
    };
    let start = chunk.offset as usize + 8;
    let Some(bytes) = data.get(start..start.saturating_add(chunk.size as usize)) else {
        return HashMap::new();
    };

    let mut names = HashMap::new();
    let mut offset = 0;
    while offset < bytes.len() {
        if bytes[offset] == 0 {
            offset += 1;
            continue;
        }
        let end = bytes[offset..]
            .iter()
            .position(|byte| *byte == 0)
            .map_or(bytes.len(), |end| offset + end);
        names.insert(
            offset as u32,
            String::from_utf8_lossy(&bytes[offset..end]).into_owned(),
        );
        offset = end.saturating_add(1);
    }
    names
}

fn wmo_doodad_transform(position: [f32; 3], orientation: [f32; 4], scale: f32) -> Transform {
    Transform {
        translation: Vec3::from_array(wow_to_bevy(position[0], position[1], position[2])),
        rotation: Quat::from_xyzw(
            orientation[1],
            orientation[2],
            orientation[0],
            orientation[3],
        )
        .normalize(),
        scale: Vec3::splat(scale),
    }
}

fn build_mesh(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn index_filenames(filenames: &[String]) -> HashMap<usize, &str> {
    let mut offset = 0;
    filenames
        .iter()
        .map(|filename| {
            let entry = (offset, filename.as_str());
            offset += filename.len() + 1;
            entry
        })
        .collect()
}

fn resolve_filename<'a>(
    filenames: &HashMap<usize, &'a str>,
    offsets: &[u32],
    name_id: usize,
) -> Option<&'a str> {
    let wanted_offset = *offsets.get(name_id)? as usize;
    filenames.get(&wanted_offset).copied()
}

fn doodad_transform(placement: &DoodadPlacement, center: Vec2) -> Transform {
    placement_transform(
        placement.position,
        placement.rotation,
        placement.get_scale(),
        center,
    )
}

fn wmo_transform(placement: &WmoPlacement, center: Vec2) -> Transform {
    let scale = if placement.scale == 0 {
        1.0
    } else {
        placement.get_scale()
    };
    placement_transform(placement.position, placement.rotation, scale, center)
}

fn placement_transform(
    position: [f32; 3],
    rotation: [f32; 3],
    scale: f32,
    center: Vec2,
) -> Transform {
    let placement_rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
        * Quat::from_rotation_x((rotation[1] - 180.0).to_radians())
        * Quat::from_rotation_y(-rotation[0].to_radians())
        * Quat::from_rotation_z((rotation[2] - 90.0).to_radians());
    Transform {
        translation: Vec3::new(
            MAP_HALF_SIZE - position[0] - center.x,
            position[1],
            MAP_HALF_SIZE - position[2] - center.y,
        ),
        rotation: placement_rotation.normalize(),
        scale: Vec3::splat(scale),
    }
}

fn adt_object_root_transform(center: Vec2) -> Transform {
    Transform::from_xyz(center.x, 0.0, center.y)
}

fn wow_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [y, z, x]
}

fn m2_fallback_filename(filename: &str) -> Option<String> {
    let stem = filename.get(..filename.len().checked_sub(4)?)?;
    matches!(
        filename
            .get(filename.len() - 4..)?
            .to_ascii_lowercase()
            .as_str(),
        ".mdx" | ".mdl"
    )
    .then(|| format!("{stem}.m2"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_wow_object_blend_modes() {
        assert_eq!(alpha_mode(0), AlphaMode::Opaque);
        assert_eq!(alpha_mode(1), AlphaMode::Mask(0.5));
        assert_eq!(alpha_mode(2), AlphaMode::Blend);
        assert_eq!(alpha_mode(3), AlphaMode::Add);
        assert_eq!(alpha_mode(4), AlphaMode::Add);
        assert_eq!(alpha_mode(5), AlphaMode::Multiply);
        assert_eq!(alpha_mode(6), AlphaMode::Multiply);
        assert_eq!(alpha_mode(7), AlphaMode::Add);
        assert_eq!(alpha_mode_with_opacity(0, 0.0), AlphaMode::Blend);
        assert_eq!(alpha_mode_with_opacity(0, 1.0), AlphaMode::Opaque);
    }

    #[test]
    fn decodes_m2_fixed_point_opacity() {
        assert_eq!(fixed_i16_alpha(&[0, 0]), Some(0.0));
        assert_eq!(fixed_i16_alpha(&i16::MAX.to_le_bytes()), Some(1.0));
        assert!((fixed_i16_alpha(&1638_i16.to_le_bytes()).unwrap() - 0.05).abs() < 0.0001);
    }

    #[test]
    fn resolves_offset_table_entries() {
        let filenames = vec!["first.m2".to_owned(), "folder\\second.m2".to_owned()];
        let offsets = vec![0, 9];
        let filenames = index_filenames(&filenames);

        assert_eq!(
            resolve_filename(&filenames, &offsets, 1),
            Some("folder\\second.m2")
        );
    }

    #[test]
    fn converts_world_position_to_adt_local_position() {
        let transform = placement_transform(
            [100.0, 200.0, 30.0],
            [0.0; 3],
            2.0,
            Vec2::new(MAP_HALF_SIZE - 150.0, MAP_HALF_SIZE - 80.0),
        );

        assert_eq!(transform.translation, Vec3::new(50.0, 200.0, 50.0));
        assert_eq!(transform.scale, Vec3::splat(2.0));
        assert!(
            (transform.rotation * Vec3::Y).abs_diff_eq(Vec3::Y, 0.0001),
            "zero placement rotation must preserve the up axis"
        );
        assert!(
            (transform.rotation * Vec3::X).abs_diff_eq(Vec3::NEG_X, 0.0001),
            "zero placement rotation must correct the model basis"
        );
    }

    #[test]
    fn adt_object_root_restores_world_position() {
        let center = Vec2::new(MAP_HALF_SIZE - 150.0, MAP_HALF_SIZE - 80.0);
        let local = placement_transform([100.0, 200.0, 30.0], [0.0; 3], 1.0, center);
        let world_translation =
            adt_object_root_transform(center).transform_point(local.translation);

        assert_eq!(
            world_translation,
            Vec3::new(MAP_HALF_SIZE - 100.0, 200.0, MAP_HALF_SIZE - 30.0)
        );
    }

    #[test]
    fn placement_yaw_preserves_up_axis() {
        let transform = placement_transform([0.0; 3], [0.0, 90.0, 0.0], 1.0, Vec2::ZERO);

        assert!((transform.rotation * Vec3::Y).abs_diff_eq(Vec3::Y, 0.0001));
    }

    #[test]
    fn placement_owner_uses_origin_tile() {
        assert_eq!(
            placement_owner([0.0, 10.0, 0.0]),
            AdtPosition { x: 0, y: 0 }
        );
        assert_eq!(
            placement_owner([ADT_SIZE, 10.0, ADT_SIZE * 2.0]),
            AdtPosition { x: 1, y: 2 }
        );
    }

    #[test]
    fn converts_wmo_local_doodad_transform() {
        let transform = wmo_doodad_transform([1.0, 2.0, 3.0], [0.0, 0.0, 0.0, 1.0], 0.5);

        assert_eq!(transform.translation, Vec3::new(2.0, 3.0, 1.0));
        assert_eq!(transform.rotation, Quat::IDENTITY);
        assert_eq!(transform.scale, Vec3::splat(0.5));
    }

    #[test]
    fn converts_classic_mdx_reference_to_m2() {
        assert_eq!(
            m2_fallback_filename("World\\Tree.MDX").as_deref(),
            Some("World\\Tree.m2")
        );
        assert_eq!(
            m2_fallback_filename("World\\Chair.MDL").as_deref(),
            Some("World\\Chair.m2")
        );
        assert_eq!(m2_fallback_filename("World\\Bridge.wmo"), None);
    }
}
