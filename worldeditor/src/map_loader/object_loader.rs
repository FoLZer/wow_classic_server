use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
};

use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use byteorder::{ByteOrder, LittleEndian};
use wow_adt::{DoodadPlacement, RootAdt, WmoPlacement};
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use wow_m2::parse_m2;
use wow_mpq::PatchChain;
use wow_wmo::{ParsedWmo, discover_wmo_chunks, parse_wmo};

use crate::mpq_read_file;

use super::{ADT_CELLS_PER_GRID, ADT_GRID_SIZE, ADT_SIZE, CHUNK_SIZE};

const MAP_HALF_SIZE: f32 = ADT_GRID_SIZE as f32 * ADT_CELLS_PER_GRID as f32 * CHUNK_SIZE * 0.5;

#[derive(Clone)]
enum ObjectAsset {
    M2(Vec<ObjectPart>),
    Wmo(WmoAsset),
}

#[derive(Clone)]
struct ObjectPart {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Clone)]
struct WmoAsset {
    parts: Vec<ObjectPart>,
    doodad_sets: Vec<Vec<WmoDoodad>>,
}

#[derive(Clone)]
struct WmoDoodad {
    filename: String,
    parts: Vec<ObjectPart>,
    transform: Transform,
}

#[derive(Default)]
pub(super) struct ObjectCache {
    assets: HashMap<String, Option<ObjectAsset>>,
    textures: HashMap<String, Option<Handle<Image>>>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_adt_objects(
    commands: &mut Commands,
    adt_entity: Entity,
    adt: &RootAdt,
    adt_coordinates: (usize, usize),
    adt_center: Vec2,
    mpqs: &mut PatchChain,
    cache: &mut ObjectCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let mut doodads = Vec::new();
    for placement in &adt.doodad_placements {
        if placement_owner(placement.position) != adt_coordinates {
            continue;
        }
        let Some(filename) =
            resolve_filename(&adt.models, &adt.model_indices, placement.name_id as usize)
        else {
            warn!("Doodad {} references an invalid model", placement.unique_id);
            continue;
        };
        if let Some(ObjectAsset::M2(parts)) =
            load_object(filename, mpqs, cache, meshes, materials, images)
        {
            doodads.push((
                filename.to_owned(),
                parts,
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
            resolve_filename(&adt.wmos, &adt.wmo_indices, placement.name_id as usize)
        else {
            warn!("WMO {} references an invalid model", placement.unique_id);
            continue;
        };
        if let Some(ObjectAsset::Wmo(asset)) =
            load_object(filename, mpqs, cache, meshes, materials, images)
        {
            wmos.push((
                filename.to_owned(),
                asset,
                placement.doodad_set as usize,
                wmo_transform(placement, adt_center),
            ));
        }
    }

    commands.entity(adt_entity).with_children(|parent| {
        for (filename, parts, transform) in doodads {
            parent
                .spawn((Name::new(filename), transform, Visibility::default()))
                .with_children(|object| spawn_parts(object, parts));
        }
        for (filename, asset, doodad_set, transform) in wmos {
            let mut doodads = asset.doodad_sets.first().cloned().unwrap_or_default();
            if doodad_set != 0 {
                doodads.extend(
                    asset
                        .doodad_sets
                        .get(doodad_set)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            parent
                .spawn((Name::new(filename), transform, Visibility::default()))
                .with_children(|object| {
                    spawn_parts(object, asset.parts);
                    for doodad in doodads {
                        object
                            .spawn((
                                Name::new(doodad.filename),
                                doodad.transform,
                                Visibility::default(),
                            ))
                            .with_children(|doodad_entity| {
                                spawn_parts(doodad_entity, doodad.parts);
                            });
                    }
                });
        }
    });
}

fn spawn_parts(parent: &mut ChildSpawnerCommands, parts: Vec<ObjectPart>) {
    for part in parts {
        parent.spawn((Mesh3d(part.mesh), MeshMaterial3d(part.material)));
    }
}

fn load_object(
    filename: &str,
    mpqs: &mut PatchChain,
    cache: &mut ObjectCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Option<ObjectAsset> {
    let key = filename.to_ascii_lowercase();
    if let Some(asset) = cache.assets.get(&key) {
        return asset.clone();
    }

    let asset = if key.ends_with(".wmo") {
        load_wmo(filename, mpqs, cache, meshes, materials, images).map(ObjectAsset::Wmo)
    } else {
        load_m2(filename, mpqs, cache, meshes, materials, images).map(ObjectAsset::M2)
    };
    if asset.is_none() {
        warn!("Unable to load world object {filename}");
    }
    cache.assets.insert(key, asset.clone());
    asset
}

fn placement_owner(position: [f32; 3]) -> (usize, usize) {
    (
        (position[0] / ADT_SIZE).floor() as usize,
        (position[2] / ADT_SIZE).floor() as usize,
    )
}

fn load_m2(
    filename: &str,
    mpqs: &mut PatchChain,
    cache: &mut ObjectCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Option<Vec<ObjectPart>> {
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
    Some(build_object_parts(
        mesh_data,
        Color::srgb(0.32, 0.48, 0.24),
        mpqs,
        cache,
        meshes,
        materials,
        images,
    ))
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
            Some(ObjectBatch {
                indices: indices.get(start..end)?.to_vec(),
                texture,
                texture_type: model
                    .textures
                    .get(texture_index)
                    .map_or(0, |texture| texture.texture_type as u32),
                alpha_mode: alpha_mode(material.map_or(0, |material| material.blend_mode.bits())),
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
    const RENDER_FLAGS_DESCRIPTOR_OFFSET: usize = 132;
    const TEXTURE_LOOKUP_DESCRIPTOR_OFFSET: usize = 148;
    const CLASSIC_VERTEX_SIZE: usize = 48;
    const SKIN_BATCH_SIZE: usize = 24;

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
            Some(ObjectBatch {
                indices: indices.get(triangle_start..end)?.to_vec(),
                texture: textures
                    .get(texture_index)
                    .and_then(|texture| texture.filename.clone()),
                texture_type: textures
                    .get(texture_index)
                    .map_or(0, |texture| texture.texture_type),
                alpha_mode: alpha_mode(blend_mode),
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
    fallback_color: Color,
    mpqs: &mut PatchChain,
    cache: &mut ObjectCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Vec<ObjectPart> {
    if mesh_data.batches.is_empty() {
        mesh_data.batches.push(ObjectBatch {
            indices: mesh_data.indices.clone(),
            texture: None,
            texture_type: 0,
            alpha_mode: AlphaMode::Opaque,
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
                .and_then(|filename| load_object_texture(filename, mpqs, cache, images));
            let mut material = StandardMaterial {
                base_color: if texture.is_some() {
                    Color::WHITE
                } else {
                    fallback_color
                },
                base_color_texture: texture,
                alpha_mode: batch.alpha_mode,
                unlit: true,
                ..default()
            };
            if batch.double_sided {
                material.cull_mode = None;
            }
            ObjectPart {
                mesh: meshes.add(build_mesh(
                    mesh_data.positions.clone(),
                    mesh_data.normals.clone(),
                    mesh_data.uvs.clone(),
                    batch.indices,
                )),
                material: materials.add(material),
            }
        })
        .collect()
}

fn load_object_texture(
    filename: &str,
    mpqs: &mut PatchChain,
    cache: &mut ObjectCache,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let key = filename.to_ascii_lowercase();
    if let Some(texture) = cache.textures.get(&key) {
        return texture.clone();
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
            images.add(image)
        });
    cache.textures.insert(key, texture.clone());
    texture
}

fn alpha_mode(blend_mode: u16) -> AlphaMode {
    match blend_mode {
        0 => AlphaMode::Opaque,
        1 => AlphaMode::Mask(0.5),
        _ => AlphaMode::Blend,
    }
}

fn load_wmo(
    filename: &str,
    mpqs: &mut PatchChain,
    cache: &mut ObjectCache,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Option<WmoAsset> {
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
            Color::srgb(0.48, 0.45, 0.39),
            mpqs,
            cache,
            meshes,
            materials,
            images,
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
                    let ObjectAsset::M2(parts) =
                        load_object(&filename, mpqs, cache, meshes, materials, images)?
                    else {
                        return None;
                    };
                    Some(WmoDoodad {
                        filename,
                        parts,
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

    (!object_parts.is_empty()).then_some(WmoAsset {
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

fn resolve_filename<'a>(
    filenames: &'a [String],
    offsets: &[u32],
    name_id: usize,
) -> Option<&'a str> {
    let wanted_offset = *offsets.get(name_id)? as usize;
    let mut offset = 0;
    for filename in filenames {
        if offset == wanted_offset {
            return Some(filename);
        }
        offset += filename.len() + 1;
    }
    None
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
    fn resolves_offset_table_entries() {
        let filenames = vec!["first.m2".to_owned(), "folder\\second.m2".to_owned()];
        let offsets = vec![0, 9];

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
    fn placement_yaw_preserves_up_axis() {
        let transform = placement_transform([0.0; 3], [0.0, 90.0, 0.0], 1.0, Vec2::ZERO);

        assert!((transform.rotation * Vec3::Y).abs_diff_eq(Vec3::Y, 0.0001));
    }

    #[test]
    fn placement_owner_uses_origin_tile() {
        assert_eq!(placement_owner([0.0, 10.0, 0.0]), (0, 0));
        assert_eq!(placement_owner([ADT_SIZE, 10.0, ADT_SIZE * 2.0]), (1, 2));
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
