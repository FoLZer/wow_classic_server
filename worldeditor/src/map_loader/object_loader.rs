use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    sync::{Arc, Mutex},
};

use bevy::{
    asset::RenderAssetUsages,
    camera::primitives::{Frustum, Sphere},
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    math::Affine2,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    pbr::ExtendedMaterial,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use byteorder::{ByteOrder, LittleEndian};
use dashmap::DashMap;
use wow_adt::chunks::mcnk::LiquidType;
use wow_adt::{DoodadPlacement, RootAdt, WmoPlacement};
use wow_blp::{convert::blp_to_image, parser::load_blp_from_buf};
use wow_m2::{M2ParticleEmitter, ParticleEmitter, parse_m2};
use wow_mpq::PatchChain;
use wow_wmo::{ParsedWmo, discover_wmo_chunks, parse_wmo};

use crate::{
    liquid_material::{LiquidMaterial, LiquidTexture, load_liquid_texture},
    map_loader::AdtPosition,
    mpq_read_file,
};

use super::{
    ADT_CELLS_PER_GRID, ADT_GRID_SIZE, ADT_SIZE, CHUNK_SIZE, ground_effects::GroundEffectPlacement,
};

const MAP_HALF_SIZE: f32 = ADT_GRID_SIZE as f32 * ADT_CELLS_PER_GRID as f32 * CHUNK_SIZE * 0.5;
const PARTICLE_UPDATE_INTERVAL: f32 = 1.0 / 30.0;
const PARTICLE_VIEW_DISTANCE: f32 = 1_000.0;

enum PreparedObjectAsset {
    M2(PreparedM2Asset),
    Wmo(PreparedWmoAsset),
}

#[derive(Clone)]
enum ObjectAsset {
    M2(M2Asset),
    Wmo(WmoAsset),
}

struct PreparedM2Asset {
    parts: Vec<PreparedObjectPart>,
    particles: Vec<PreparedParticleEmitter>,
}

#[derive(Clone)]
struct M2Asset {
    parts: Vec<ObjectPart>,
    particles: Vec<ParticleEmitterTemplate>,
}

struct PreparedParticleEmitter {
    emitter: ParticleEmitterDefinition,
    texture_key: Option<String>,
    animation: Option<Arc<M2SkinAnimation>>,
}

#[derive(Clone)]
struct ParticleEmitterTemplate {
    emitter: ParticleEmitterDefinition,
    material: Handle<StandardMaterial>,
    animation: Option<Arc<M2SkinAnimation>>,
}

#[derive(Clone)]
enum ParticleEmitterDefinition {
    Parsed(M2ParticleEmitter),
    Classic(ClassicParticleEmitter),
}

#[derive(Clone)]
struct ClassicParticleEmitter {
    flags: u32,
    position: Vec3,
    bone_index: u16,
    blending_type: u16,
    emitter_type: u16,
    rows: u16,
    columns: u16,
    emission_speed: f32,
    speed_variation: f32,
    vertical_range: f32,
    horizontal_range: f32,
    gravity: f32,
    lifespan: f32,
    emission_rate: f32,
    area_length: f32,
    area_width: f32,
    midpoint: f32,
    colors: [[f32; 4]; 3],
    scales: [f32; 3],
}

enum ParticleEmitterRuntime {
    Parsed(ParticleEmitter),
    Classic(ClassicParticleRuntime),
}

struct ClassicParticleRuntime {
    definition: ClassicParticleEmitter,
    particles: Vec<ClassicParticle>,
    emission_remainder: f32,
    random_state: u64,
}

struct ClassicParticle {
    position: Vec3,
    velocity: Vec3,
    age: f32,
    lifespan: f32,
}

#[derive(Component)]
pub(crate) struct M2ParticleSystem {
    emitter: ParticleEmitterRuntime,
    mesh: Handle<Mesh>,
    animation: Option<Arc<M2SkinAnimation>>,
    render_data: Vec<ParticleRenderData>,
    update_accumulator: f32,
    bounds_radius: f32,
    mesh_is_empty: bool,
}

struct PreparedObjectPart {
    mesh: Mutex<Option<Mesh>>,
    animation: Option<Arc<M2SkinAnimation>>,
    texture_animation: Option<Arc<M2TextureAnimation>>,
    opacity_animation: Option<Arc<M2OpacityAnimation>>,
    uvs: Vec<[f32; 2]>,
    texture_key: Option<String>,
    double_sided: bool,
    opacity: f32,
    alpha_mode: AlphaMode,
}

struct M2SkinAnimation {
    vertices: Vec<AnimatedVertex>,
    bones: Vec<AnimatedBone>,
    duration_ms: u32,
    start_ms: u32,
}

struct AnimatedVertex {
    position: Vec3,
    normal: Vec3,
    weights: [f32; 4],
    bones: [u8; 4],
}

struct AnimatedBone {
    parent: i16,
    pivot: Vec3,
    translation: Option<AnimationTrack<Vec3>>,
    rotation: Option<AnimationTrack<Quat>>,
    scale: Option<AnimationTrack<Vec3>>,
}

struct AnimationTrack<T> {
    timestamps: Vec<u32>,
    values: Vec<T>,
}

struct M2TextureAnimation {
    translation_u: Option<AnimationTrack<f32>>,
    translation_v: Option<AnimationTrack<f32>>,
    rotation: Option<AnimationTrack<f32>>,
    scale_u: Option<AnimationTrack<f32>>,
    scale_v: Option<AnimationTrack<f32>>,
    duration_ms: u32,
    start_ms: u32,
}

struct M2OpacityAnimation {
    opacity: AnimationTrack<f32>,
    duration_ms: u32,
    start_ms: u32,
}

struct AnimatedMesh {
    mesh: Handle<Mesh>,
    animation: Option<Arc<M2SkinAnimation>>,
    texture_animation: Option<Arc<M2TextureAnimation>>,
    uvs: Vec<[f32; 2]>,
}

struct AnimatedMaterial {
    material: Handle<StandardMaterial>,
    animation: Arc<M2OpacityAnimation>,
}

#[derive(Default)]
struct ObjectAnimations {
    elapsed_seconds: f32,
    meshes: Vec<AnimatedMesh>,
    materials: Vec<AnimatedMaterial>,
    bone_transform_cache: HashMap<usize, Vec<Mat4>>,
}

#[derive(Clone)]
struct ObjectPart {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

struct PreparedWmoAsset {
    parts: Vec<PreparedObjectPart>,
    liquids: Vec<PreparedWmoLiquid>,
    doodad_sets: Vec<Vec<PreparedWmoDoodad>>,
}

#[derive(Clone)]
struct WmoAsset {
    parts: Vec<ObjectPart>,
    liquids: Vec<WmoLiquidPart>,
    doodad_sets: Vec<Vec<WmoDoodad>>,
}

struct PreparedWmoLiquid {
    mesh: Mutex<Option<Mesh>>,
    liquid_type: LiquidType,
}

#[derive(Clone)]
struct WmoLiquidPart {
    mesh: Handle<Mesh>,
    material: Handle<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
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
    liquid_textures: [Option<LiquidTexture>; 4],
    liquid_materials: [Option<Handle<ExtendedMaterial<StandardMaterial, LiquidMaterial>>>; 4],
    animations: ObjectAnimations,
}

impl ObjectCache {
    pub(super) fn unload_assets(
        &mut self,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        liquid_materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
        images: &mut Assets<Image>,
    ) {
        let mut visited = HashSet::new();
        for asset in self.assets.drain().filter_map(|(_, asset)| asset) {
            unload_object_asset(&asset, &mut visited, meshes, materials, liquid_materials);
        }
        for image in self.textures.drain().filter_map(|(_, image)| image) {
            images.remove(image.id());
        }
        for texture in self.liquid_textures.iter_mut().filter_map(Option::take) {
            images.remove(texture.handle.id());
        }
        for material in self.liquid_materials.iter_mut().filter_map(Option::take) {
            liquid_materials.remove(material.id());
        }
        self.animations = ObjectAnimations::default();
    }
}

fn unload_object_asset(
    asset: &Arc<ObjectAsset>,
    visited: &mut HashSet<usize>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    liquid_materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
) {
    if !visited.insert(Arc::as_ptr(asset) as usize) {
        return;
    }
    match asset.as_ref() {
        ObjectAsset::M2(asset) => {
            for part in &asset.parts {
                meshes.remove(part.mesh.id());
                materials.remove(part.material.id());
            }
            for particle in &asset.particles {
                materials.remove(particle.material.id());
            }
        }
        ObjectAsset::Wmo(asset) => {
            for part in &asset.parts {
                meshes.remove(part.mesh.id());
                materials.remove(part.material.id());
            }
            for liquid in &asset.liquids {
                meshes.remove(liquid.mesh.id());
                liquid_materials.remove(liquid.material.id());
            }
            for doodad in asset.doodad_sets.iter().flatten() {
                unload_object_asset(&doodad.asset, visited, meshes, materials, liquid_materials);
            }
        }
    }
}

pub(crate) fn animate_objects(
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut map: ResMut<super::TerrainMap>,
    camera: Query<(&GlobalTransform, &Frustum), With<Camera3d>>,
    mut particles: Query<(
        Entity,
        &GlobalTransform,
        &mut Visibility,
        &mut M2ParticleSystem,
    )>,
) {
    let animations = &mut map.object_cache.animations;
    animations.elapsed_seconds += time.delta_secs();
    let elapsed_ms = (animations.elapsed_seconds * 1000.0) as u32;
    let mut evaluated_animations = HashSet::new();
    for animated_mesh in &animations.meshes {
        let Some(mut mesh) = meshes.get_mut(&animated_mesh.mesh) else {
            continue;
        };
        if let Some(animation) = &animated_mesh.animation {
            let animation_key = Arc::as_ptr(animation) as usize;
            let transforms = animations
                .bone_transform_cache
                .entry(animation_key)
                .or_default();
            if evaluated_animations.insert(animation_key) {
                animation.write_bone_transforms(elapsed_ms, transforms);
            }
            let mut positions = Vec::with_capacity(animation.vertices.len());
            let mut normals = Vec::with_capacity(animation.vertices.len());
            for vertex in &animation.vertices {
                let mut position = Vec3::ZERO;
                let mut normal = Vec3::ZERO;
                let mut total_weight = 0.0;
                for influence in 0..4 {
                    let weight = vertex.weights[influence];
                    if weight == 0.0 {
                        continue;
                    }
                    let transform = transforms
                        .get(vertex.bones[influence] as usize)
                        .copied()
                        .unwrap_or(Mat4::IDENTITY);
                    position += transform.transform_point3(vertex.position) * weight;
                    normal += transform.transform_vector3(vertex.normal) * weight;
                    total_weight += weight;
                }
                if total_weight > 0.0 {
                    position /= total_weight;
                    normal /= total_weight;
                } else {
                    position = vertex.position;
                    normal = vertex.normal;
                }
                if !position.is_finite() {
                    position = vertex.position;
                }
                if !normal.is_finite() || normal.length_squared() <= f32::EPSILON {
                    normal = vertex.normal;
                }
                positions.push(position.to_array());
                normals.push(normal.normalize_or_zero().to_array());
            }
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        }
        if let Some(animation) = &animated_mesh.texture_animation {
            let transform = animation.transform(elapsed_ms);
            let uvs = animated_mesh
                .uvs
                .iter()
                .map(|uv| transform.transform_point2(Vec2::from_array(*uv)).to_array())
                .collect::<Vec<_>>();
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
    }
    for animated_material in &animations.materials {
        let Some(mut material) = materials.get_mut(&animated_material.material) else {
            continue;
        };
        material
            .base_color
            .set_alpha(animated_material.animation.sample(elapsed_ms));
    }

    let Ok((camera_transform, camera_frustum)) = camera.single() else {
        return;
    };
    let camera_position = camera_transform.translation();
    let camera_right = camera_transform.right().as_vec3();
    let camera_up = camera_transform.up().as_vec3();
    let identity = Mat4::IDENTITY.to_cols_array();
    for (entity, global_transform, mut visibility, mut system) in &mut particles {
        let emitter_position = global_transform.translation();
        let bounds_radius =
            system.bounds_radius * global_transform.compute_transform().scale.max_element();
        let bounds = Sphere {
            center: emitter_position.into(),
            radius: bounds_radius,
        };
        let is_visible = emitter_position.distance_squared(camera_position)
            <= (PARTICLE_VIEW_DISTANCE + bounds_radius).powi(2)
            && camera_frustum.intersects_sphere(&bounds, true);
        if !is_visible {
            *visibility = Visibility::Hidden;
            if !system.mesh_is_empty {
                system.render_data.clear();
                if let Some(mut mesh) = meshes.get_mut(&system.mesh) {
                    update_particle_mesh(&mut mesh, &[], Vec3::X, Vec3::Y, Mat4::IDENTITY);
                }
                system.mesh_is_empty = true;
            }
            system.update_accumulator = 0.0;
            continue;
        }
        *visibility = Visibility::Inherited;
        if system.update_accumulator < 0.0 {
            system.update_accumulator =
                (entity.index_u32() % 4) as f32 * PARTICLE_UPDATE_INTERVAL / 4.0;
        }
        system.update_accumulator += time.delta_secs();
        if system.update_accumulator < PARTICLE_UPDATE_INTERVAL {
            continue;
        }
        let particle_delta = PARTICLE_UPDATE_INTERVAL;
        system.update_accumulator -= PARTICLE_UPDATE_INTERVAL;
        let bone_transform = system
            .animation
            .as_ref()
            .and_then(|animation| {
                let animation_key = Arc::as_ptr(animation) as usize;
                let transforms = animations
                    .bone_transform_cache
                    .entry(animation_key)
                    .or_default();
                if evaluated_animations.insert(animation_key) {
                    animation.write_bone_transforms(elapsed_ms, transforms);
                }
                transforms
                    .get(system.emitter.bone_index() as usize)
                    .copied()
            })
            .unwrap_or(Mat4::IDENTITY);
        system
            .emitter
            .update(particle_delta, &identity, bone_transform);
        let M2ParticleSystem {
            emitter,
            mesh,
            render_data,
            ..
        } = &mut *system;
        emitter.write_render_data(render_data);
        let inverse = global_transform.affine().inverse();
        let right = inverse.transform_vector3(camera_right).normalize_or_zero();
        let up = inverse.transform_vector3(camera_up).normalize_or_zero();
        let Some(mut mesh) = meshes.get_mut(mesh) else {
            continue;
        };
        update_particle_mesh(
            &mut mesh,
            render_data,
            right,
            up,
            emitter.render_transform(bone_transform),
        );
        system.mesh_is_empty = render_data.is_empty();
    }
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
    liquid_materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
    images: &mut Assets<Image>,
    mpqs: &PatchChain,
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
                liquid_materials,
                images,
                mpqs,
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
                liquid_materials,
                images,
                mpqs,
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
            let ObjectAsset::M2(asset) = asset.as_ref() else {
                continue;
            };
            parent
                .spawn((Name::new(filename), transform, Visibility::default()))
                .with_children(|object| spawn_m2_contents(object, asset, meshes));
        }
        for (filename, asset, doodad_set, transform) in wmos {
            let ObjectAsset::Wmo(asset) = asset.as_ref() else {
                continue;
            };
            parent
                .spawn((Name::new(filename), transform, Visibility::default()))
                .with_children(|object| spawn_wmo_contents(object, asset, doodad_set, meshes));
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

pub(super) fn load_ground_effects(
    placements: Vec<GroundEffectPlacement>,
    mpqs: &PatchChain,
    cache: &PreparedObjectCache,
) -> ObjectsLoadResult {
    let mut doodads = Vec::new();
    for placement in placements {
        if let Some(asset) = load_object(&placement.filename, mpqs, cache)
            && matches!(asset.as_ref(), PreparedObjectAsset::M2(_))
        {
            doodads.push((placement.filename, asset, placement.transform));
        }
    }
    ObjectsLoadResult {
        doodads,
        wmos: Vec::new(),
    }
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
            Pickable::IGNORE,
        ));
    }
}

fn spawn_m2_contents(
    parent: &mut ChildSpawnerCommands,
    asset: &M2Asset,
    meshes: &mut Assets<Mesh>,
) {
    spawn_parts(parent, &asset.parts);
    for template in &asset.particles {
        let mesh = meshes.add(empty_particle_mesh());
        parent.spawn((
            Name::new("M2 particle emitter"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(template.material.clone()),
            Pickable::IGNORE,
            M2ParticleSystem {
                bounds_radius: template.emitter.bounds_radius(),
                emitter: ParticleEmitterRuntime::new(&template.emitter),
                mesh,
                animation: template.animation.clone(),
                render_data: Vec::new(),
                update_accumulator: -1.0,
                mesh_is_empty: true,
            },
        ));
    }
}

fn spawn_wmo_contents(
    parent: &mut ChildSpawnerCommands,
    asset: &WmoAsset,
    doodad_set: usize,
    meshes: &mut Assets<Mesh>,
) {
    spawn_parts(parent, &asset.parts);
    for liquid in &asset.liquids {
        parent.spawn((
            Name::new("Classic WMO liquid"),
            Mesh3d(liquid.mesh.clone()),
            MeshMaterial3d(liquid.material.clone()),
            Pickable::IGNORE,
        ));
    }
    if let Some(doodads) = asset.doodad_sets.first() {
        spawn_wmo_doodads(parent, doodads, meshes);
    }
    if doodad_set != 0
        && let Some(doodads) = asset.doodad_sets.get(doodad_set)
    {
        spawn_wmo_doodads(parent, doodads, meshes);
    }
}

fn spawn_wmo_doodads(
    parent: &mut ChildSpawnerCommands,
    doodads: &[WmoDoodad],
    meshes: &mut Assets<Mesh>,
) {
    for doodad in doodads {
        let ObjectAsset::M2(asset) = doodad.asset.as_ref() else {
            continue;
        };
        parent
            .spawn((
                Name::new(doodad.filename.clone()),
                doodad.transform,
                Visibility::default(),
            ))
            .with_children(|doodad_entity| spawn_m2_contents(doodad_entity, asset, meshes));
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
    liquid_materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
    images: &mut Assets<Image>,
    mpqs: &PatchChain,
) -> Option<Arc<ObjectAsset>> {
    let key = filename.to_ascii_lowercase();
    if let Some(asset) = cache.assets.get(&key) {
        return asset.clone();
    }

    let asset = match prepared.as_ref() {
        PreparedObjectAsset::M2(m2) => ObjectAsset::M2(M2Asset {
            parts: finalize_object_parts(
                &m2.parts,
                Color::srgb(0.32, 0.48, 0.24),
                prepared_cache,
                cache,
                meshes,
                materials,
                images,
            ),
            particles: finalize_particle_emitters(
                &m2.particles,
                prepared_cache,
                cache,
                materials,
                images,
            ),
        }),
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
                                liquid_materials,
                                images,
                                mpqs,
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
            let liquids = wmo
                .liquids
                .iter()
                .filter_map(|liquid| {
                    let mesh = meshes.add(liquid.mesh.lock().ok()?.take()?);
                    let material = object_liquid_material(
                        liquid.liquid_type,
                        mpqs,
                        cache,
                        liquid_materials,
                        images,
                    );
                    Some(WmoLiquidPart { mesh, material })
                })
                .collect();
            ObjectAsset::Wmo(WmoAsset {
                parts,
                liquids,
                doodad_sets,
            })
        }
    };
    let asset = Arc::new(asset);
    cache.assets.insert(key, Some(asset.clone()));
    Some(asset)
}

fn object_liquid_material(
    liquid_type: LiquidType,
    mpqs: &PatchChain,
    cache: &mut ObjectCache,
    materials: &mut Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
    images: &mut Assets<Image>,
) -> Handle<ExtendedMaterial<StandardMaterial, LiquidMaterial>> {
    let liquid_index = liquid_type as usize;
    let texture = cache.liquid_textures[liquid_index]
        .get_or_insert_with(|| load_liquid_texture(liquid_type, mpqs, images))
        .clone();
    cache.liquid_materials[liquid_index]
        .get_or_insert_with(|| {
            materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: match liquid_type {
                        LiquidType::Water => Color::srgba(0.3, 0.3, 0.4, 0.68),
                        LiquidType::Ocean => Color::srgba(0.2, 0.3, 0.35, 0.72),
                        LiquidType::Magma | LiquidType::Slime => Color::WHITE,
                    },
                    alpha_mode: if liquid_type == LiquidType::Magma {
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
                        liquid_type,
                        LiquidType::Water | LiquidType::Ocean
                    )),
                    uv_scroll: if matches!(liquid_type, LiquidType::Magma | LiquidType::Slime) {
                        Vec2::X
                    } else {
                        Vec2::ZERO
                    },
                },
            })
        })
        .clone()
}

fn finalize_particle_emitters(
    emitters: &[PreparedParticleEmitter],
    prepared_cache: &PreparedObjectCache,
    cache: &mut ObjectCache,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Vec<ParticleEmitterTemplate> {
    emitters
        .iter()
        .map(|prepared| {
            let texture = prepared.texture_key.as_deref().and_then(|key| {
                if let Some(texture) = cache.textures.get(key) {
                    return texture.clone();
                }
                let texture = prepared_cache
                    .textures
                    .get_mut(key)
                    .and_then(|mut value| value.take())
                    .map(|image| images.add(image));
                cache.textures.insert(key.to_owned(), texture.clone());
                texture
            });
            let material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: texture,
                alpha_mode: prepared.emitter.alpha_mode(),
                cull_mode: None,
                unlit: !prepared.emitter.is_lit(),
                ..default()
            });
            ParticleEmitterTemplate {
                emitter: prepared.emitter.clone(),
                material,
                animation: prepared.animation.clone(),
            }
        })
        .collect()
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
                perceptual_roughness: 0.82,
                unlit: !cfg!(feature = "realistic-lighting"),
                ..default()
            };
            if part.double_sided {
                material.cull_mode = None;
            }
            let mesh = meshes.add(part.mesh.lock().unwrap().take()?);
            if part.animation.is_some() || part.texture_animation.is_some() {
                cache.animations.meshes.push(AnimatedMesh {
                    mesh: mesh.clone(),
                    animation: part.animation.clone(),
                    texture_animation: part.texture_animation.clone(),
                    uvs: part.uvs.clone(),
                });
            }
            let material = materials.add(material);
            if let Some(animation) = &part.opacity_animation {
                cache.animations.materials.push(AnimatedMaterial {
                    material: material.clone(),
                    animation: animation.clone(),
                });
            }
            Some(ObjectPart { mesh, material })
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
) -> Option<PreparedM2Asset> {
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
    if (mesh_data.positions.is_empty() || mesh_data.indices.is_empty())
        && mesh_data.particles.is_empty()
    {
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
    Some(build_m2_asset(mesh_data, mpqs, cache))
}

struct M2MeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    animation: Option<Arc<M2SkinAnimation>>,
    indices: Vec<u32>,
    batches: Vec<ObjectBatch>,
    particles: Vec<PreparedParticleEmitter>,
}

struct ObjectBatch {
    indices: Vec<u32>,
    texture: Option<String>,
    texture_type: u32,
    texture_animation: Option<Arc<M2TextureAnimation>>,
    opacity_animation: Option<Arc<M2OpacityAnimation>>,
    alpha_mode: AlphaMode,
    opacity: f32,
    double_sided: bool,
}

fn library_m2_mesh_data(data: &[u8]) -> Result<M2MeshData, String> {
    let format = parse_m2(&mut Cursor::new(data)).map_err(|error| error.to_string())?;
    let model = format.model();
    let skin = match model.parse_embedded_skin(data, 0) {
        Ok(skin) => Some(skin),
        Err(_) if !model.particle_emitters.is_empty() => None,
        Err(error) => return Err(error.to_string()),
    };
    let indices = skin.as_ref().map_or_else(Vec::new, |skin| {
        skin.get_resolved_indices()
            .into_iter()
            .map(u32::from)
            .collect::<Vec<_>>()
    });
    let batches = skin.as_ref().map_or_else(Vec::new, |skin| {
        skin.batches()
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
                let texture_animation = model
                    .raw_data
                    .texture_animation_lookup
                    .get(batch.texture_transform_combo_index as usize)
                    .copied()
                    .filter(|index| *index != u16::MAX)
                    .and_then(|index| m2_texture_animation(model, index as usize));
                let opacity_animation = model
                    .raw_data
                    .transparency_lookup_table
                    .get(batch.texture_weight_combo_index as usize)
                    .and_then(|animation_index| {
                        m2_opacity_animation(model, *animation_index as usize)
                    });
                let opacity = opacity_animation
                    .as_ref()
                    .map_or(1.0, |animation| animation.sample(0));
                Some(ObjectBatch {
                    indices: indices.get(start..end)?.to_vec(),
                    texture,
                    texture_type: model
                        .textures
                        .get(texture_index)
                        .map_or(0, |texture| texture.texture_type as u32),
                    texture_animation,
                    alpha_mode: alpha_mode_with_animation(
                        material.map_or(0, |material| material.blend_mode.bits()),
                        opacity,
                        opacity_animation
                            .as_deref()
                            .is_some_and(M2OpacityAnimation::requires_blending),
                    ),
                    opacity,
                    opacity_animation,
                    double_sided: material
                        .is_some_and(|material| material.flags.bits() & 0x04 != 0),
                })
            })
            .collect()
    });
    let animation = m2_skin_animation(model, data);
    let particles = model
        .particle_emitters
        .iter()
        .map(|emitter| {
            let texture = model
                .textures
                .get(emitter.texture_index as usize)
                .map(|texture| texture.filename.string.to_string_lossy())
                .filter(|filename| !filename.is_empty());
            PreparedParticleEmitter {
                emitter: ParticleEmitterDefinition::Parsed(emitter.clone()),
                texture_key: texture,
                animation: animation.clone(),
            }
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
        animation,
        indices,
        batches,
        particles,
    })
}

fn m2_opacity_animation(
    model: &wow_m2::M2Model,
    animation_index: usize,
) -> Option<Arc<M2OpacityAnimation>> {
    let (sequence_index, sequence) = model
        .animations
        .iter()
        .enumerate()
        .find(|(_, animation)| animation.animation_id == 0)
        .or_else(|| model.animations.iter().enumerate().next())?;
    let (start_ms, duration_ms) = match sequence.end_timestamp {
        Some(end) => (
            sequence.start_timestamp,
            end.saturating_sub(sequence.start_timestamp),
        ),
        None => (0, sequence.start_timestamp),
    };
    if duration_ms == 0 {
        return None;
    }
    let raw = model
        .raw_data
        .transparency_animation_data
        .iter()
        .find(|track| track.animation_index == animation_index)?;
    let offset = sequence_index * 8;
    let (start, end) = raw
        .interpolation_ranges
        .get(offset..offset + 8)
        .map(|range| {
            (
                LittleEndian::read_u32(&range[..4]) as usize,
                LittleEndian::read_u32(&range[4..]) as usize + 1,
            )
        })
        .unwrap_or((0, raw.timestamps.len() / 4));
    let timestamps = raw
        .timestamps
        .chunks_exact(4)
        .skip(start)
        .take(end.saturating_sub(start))
        .map(LittleEndian::read_u32)
        .collect::<Vec<_>>();
    let values = fixed_i16_alpha_values(&raw.values, start, timestamps.len());
    (!timestamps.is_empty()
        && timestamps.len() == values.len()
        && values.iter().all(|value| value.is_finite()))
    .then(|| {
        Arc::new(M2OpacityAnimation {
            opacity: AnimationTrack { timestamps, values },
            duration_ms,
            start_ms,
        })
    })
}

fn m2_texture_animation(
    model: &wow_m2::M2Model,
    animation_index: usize,
) -> Option<Arc<M2TextureAnimation>> {
    use wow_m2::model::TextureTrackType;

    let (sequence_index, sequence) = model
        .animations
        .iter()
        .enumerate()
        .find(|(_, animation)| animation.animation_id == 0)
        .or_else(|| model.animations.iter().enumerate().next())?;
    let (start_ms, duration_ms) = match sequence.end_timestamp {
        Some(end) => (
            sequence.start_timestamp,
            end.saturating_sub(sequence.start_timestamp),
        ),
        None => (0, sequence.start_timestamp),
    };
    if duration_ms == 0 {
        return None;
    }
    let find_track = |track_type| {
        model
            .raw_data
            .texture_animation_data
            .iter()
            .find(|track| {
                track.animation_index == animation_index && track.track_type == track_type
            })
            .and_then(|track| float_track(track, sequence_index))
    };
    let animation = M2TextureAnimation {
        translation_u: find_track(TextureTrackType::TranslationU),
        translation_v: find_track(TextureTrackType::TranslationV),
        rotation: find_track(TextureTrackType::Rotation),
        scale_u: find_track(TextureTrackType::ScaleU),
        scale_v: find_track(TextureTrackType::ScaleV),
        duration_ms,
        start_ms,
    };
    (animation.translation_u.is_some()
        || animation.translation_v.is_some()
        || animation.rotation.is_some()
        || animation.scale_u.is_some()
        || animation.scale_v.is_some())
    .then(|| Arc::new(animation))
}

fn float_track(
    raw: &wow_m2::model::TextureAnimationRaw,
    sequence: usize,
) -> Option<AnimationTrack<f32>> {
    let offset = sequence * 8;
    let (start, end) = raw
        .interpolation_ranges
        .get(offset..offset + 8)
        .map(|range| {
            (
                LittleEndian::read_u32(&range[..4]) as usize,
                LittleEndian::read_u32(&range[4..]) as usize + 1,
            )
        })
        .unwrap_or((0, raw.timestamps.len() / 4));
    let timestamps = raw
        .timestamps
        .chunks_exact(4)
        .skip(start)
        .take(end.saturating_sub(start))
        .map(LittleEndian::read_u32)
        .collect::<Vec<_>>();
    let values = raw
        .values
        .chunks_exact(4)
        .skip(start)
        .take(timestamps.len())
        .map(LittleEndian::read_f32)
        .collect::<Vec<_>>();
    (!timestamps.is_empty()
        && timestamps.len() == values.len()
        && values.iter().all(|value| value.is_finite()))
    .then_some(AnimationTrack { timestamps, values })
}

fn m2_skin_animation(model: &wow_m2::M2Model, data: &[u8]) -> Option<Arc<M2SkinAnimation>> {
    use wow_m2::model::TrackType;

    let (sequence_index, sequence) = model
        .animations
        .iter()
        .enumerate()
        .find(|(_, animation)| animation.animation_id == 0)
        .or_else(|| model.animations.iter().enumerate().next())?;
    let (start_ms, duration_ms) = match sequence.end_timestamp {
        Some(end) => (
            sequence.start_timestamp,
            end.saturating_sub(sequence.start_timestamp),
        ),
        None => (0, sequence.start_timestamp),
    };
    if duration_ms == 0 || model.bones.is_empty() {
        return None;
    }

    let find_track = |bone_index, track_type| {
        model
            .raw_data
            .bone_animation_data
            .iter()
            .find(|track| track.bone_index == bone_index && track.track_type == track_type)
    };
    let bones = model
        .bones
        .iter()
        .enumerate()
        .map(|(bone_index, bone)| AnimatedBone {
            parent: bone.parent_bone,
            pivot: Vec3::from_array(wow_to_bevy(bone.pivot.x, bone.pivot.y, bone.pivot.z)),
            translation: find_track(bone_index, TrackType::Translation)
                .and_then(|track| vec3_track(track, sequence_index)),
            rotation: find_track(bone_index, TrackType::Rotation)
                .and_then(|track| quat_track(track, sequence_index, model.header.version, data)),
            scale: find_track(bone_index, TrackType::Scale)
                .and_then(|track| vec3_track(track, sequence_index)),
        })
        .collect::<Vec<_>>();
    if bones
        .iter()
        .all(|bone| bone.translation.is_none() && bone.rotation.is_none() && bone.scale.is_none())
    {
        return None;
    }
    let vertices = model
        .vertices
        .iter()
        .map(|vertex| AnimatedVertex {
            position: Vec3::from_array(wow_to_bevy(
                vertex.position.x,
                vertex.position.y,
                vertex.position.z,
            )),
            normal: Vec3::from_array(wow_to_bevy(
                vertex.normal.x,
                vertex.normal.y,
                vertex.normal.z,
            )),
            weights: vertex.bone_weights.map(|weight| f32::from(weight) / 255.0),
            bones: vertex.bone_indices,
        })
        .collect();
    Some(Arc::new(M2SkinAnimation {
        vertices,
        bones,
        duration_ms,
        start_ms,
    }))
}

fn track_range(track: &wow_m2::model::BoneAnimationRaw, sequence: usize) -> (usize, usize) {
    let Some(ranges) = track.ranges.as_deref() else {
        return (0, track.timestamps.len() / 4);
    };
    let offset = sequence * 8;
    let Some(bytes) = ranges.get(offset..offset + 8) else {
        return (0, 0);
    };
    let start = LittleEndian::read_u32(&bytes[..4]) as usize;
    let end = LittleEndian::read_u32(&bytes[4..]) as usize;
    (start, end.saturating_add(1))
}

fn vec3_track(
    raw: &wow_m2::model::BoneAnimationRaw,
    sequence: usize,
) -> Option<AnimationTrack<Vec3>> {
    let (start, end) = track_range(raw, sequence);
    let timestamps = raw
        .timestamps
        .chunks_exact(4)
        .skip(start)
        .take(end.saturating_sub(start))
        .map(LittleEndian::read_u32)
        .collect::<Vec<_>>();
    let values = raw
        .values
        .chunks_exact(12)
        .skip(start)
        .take(timestamps.len())
        .map(|bytes| {
            Vec3::from_array(wow_to_bevy(
                LittleEndian::read_f32(&bytes[..4]),
                LittleEndian::read_f32(&bytes[4..8]),
                LittleEndian::read_f32(&bytes[8..]),
            ))
        })
        .collect::<Vec<_>>();
    (!timestamps.is_empty()
        && timestamps.len() == values.len()
        && values.iter().all(|value| value.is_finite()))
    .then_some(AnimationTrack { timestamps, values })
}

fn quat_track(
    raw: &wow_m2::model::BoneAnimationRaw,
    sequence: usize,
    version: u32,
    source: &[u8],
) -> Option<AnimationTrack<Quat>> {
    let (start, end) = track_range(raw, sequence);
    let timestamps = raw
        .timestamps
        .chunks_exact(4)
        .skip(start)
        .take(end.saturating_sub(start))
        .map(LittleEndian::read_u32)
        .collect::<Vec<_>>();
    let value_size = if version <= 257 { 16 } else { 8 };
    let value_count = raw.values.len() / 8;
    let values_data = if value_size == 16 {
        let start = raw.original_values_offset as usize;
        source.get(start..start.checked_add(value_count.checked_mul(value_size)?)?)?
    } else {
        &raw.values
    };
    let values = values_data
        .chunks_exact(value_size)
        .skip(start)
        .take(timestamps.len())
        .map(|bytes| match value_size {
            16 => wow_quat_to_bevy(
                LittleEndian::read_f32(&bytes[..4]),
                LittleEndian::read_f32(&bytes[4..8]),
                LittleEndian::read_f32(&bytes[8..12]),
                LittleEndian::read_f32(&bytes[12..]),
            ),
            _ => wow_quat_to_bevy(
                decompress_quat_component(LittleEndian::read_i16(&bytes[..2])),
                decompress_quat_component(LittleEndian::read_i16(&bytes[2..4])),
                decompress_quat_component(LittleEndian::read_i16(&bytes[4..6])),
                decompress_quat_component(LittleEndian::read_i16(&bytes[6..])),
            ),
        })
        .collect::<Vec<_>>();
    (!timestamps.is_empty() && timestamps.len() == values.len())
        .then_some(AnimationTrack { timestamps, values })
}

fn decompress_quat_component(value: i16) -> f32 {
    let value = i32::from(value);
    if value < 0 {
        (value + 32768) as f32 / 32767.0
    } else {
        (value - 32767) as f32 / 32767.0
    }
}

fn wow_quat_to_bevy(x: f32, y: f32, z: f32, w: f32) -> Quat {
    let rotation = Quat::from_xyzw(y, z, x, w);
    if rotation.is_finite() && rotation.length_squared() > f32::EPSILON {
        rotation.normalize()
    } else {
        Quat::IDENTITY
    }
}

impl M2SkinAnimation {
    #[cfg(test)]
    fn bone_transforms(&self, elapsed_ms: u32) -> Vec<Mat4> {
        let mut transforms = Vec::new();
        self.write_bone_transforms(elapsed_ms, &mut transforms);
        transforms
    }

    fn write_bone_transforms(&self, elapsed_ms: u32, transforms: &mut Vec<Mat4>) {
        let timestamp = self.start_ms + elapsed_ms % self.duration_ms;
        transforms.clear();
        transforms.resize(self.bones.len(), Mat4::IDENTITY);
        for (index, bone) in self.bones.iter().enumerate() {
            let translation = bone
                .translation
                .as_ref()
                .map_or(Vec3::ZERO, |track| track.sample_vec3(timestamp));
            let rotation = bone
                .rotation
                .as_ref()
                .map_or(Quat::IDENTITY, |track| track.sample_quat(timestamp));
            let scale = bone
                .scale
                .as_ref()
                .map_or(Vec3::ONE, |track| track.sample_vec3(timestamp));
            let local = Mat4::from_translation(bone.pivot)
                * Mat4::from_scale_rotation_translation(scale, rotation, translation)
                * Mat4::from_translation(-bone.pivot);
            let transform = bone
                .parent
                .try_into()
                .ok()
                .and_then(|parent: usize| transforms.get(parent).copied())
                .unwrap_or(Mat4::IDENTITY)
                * local;
            transforms[index] = if transform.is_finite() {
                transform
            } else {
                Mat4::IDENTITY
            };
        }
    }
}

impl<T> AnimationTrack<T> {
    fn sample_indices(&self, timestamp: u32) -> (usize, usize, f32) {
        let upper = self.timestamps.partition_point(|value| *value <= timestamp);
        let (left, right) = if upper == 0 {
            (0, 0)
        } else if upper == self.timestamps.len() {
            (upper - 1, upper - 1)
        } else {
            (upper - 1, upper)
        };
        let span = self.timestamps[right].saturating_sub(self.timestamps[left]);
        let factor = if span == 0 {
            0.0
        } else {
            timestamp.saturating_sub(self.timestamps[left]) as f32 / span as f32
        };
        (left, right, factor)
    }
}

impl AnimationTrack<Vec3> {
    fn sample_vec3(&self, timestamp: u32) -> Vec3 {
        let (left, right, factor) = self.sample_indices(timestamp);
        self.values[left].lerp(self.values[right], factor)
    }
}

impl AnimationTrack<Quat> {
    fn sample_quat(&self, timestamp: u32) -> Quat {
        let (left, right, factor) = self.sample_indices(timestamp);
        self.values[left].slerp(self.values[right], factor)
    }
}

impl AnimationTrack<f32> {
    fn sample_f32(&self, timestamp: u32) -> f32 {
        let (left, right, factor) = self.sample_indices(timestamp);
        self.values[left] + (self.values[right] - self.values[left]) * factor
    }
}

impl M2TextureAnimation {
    fn transform(&self, elapsed_ms: u32) -> Affine2 {
        let timestamp = self.start_ms + elapsed_ms % self.duration_ms;
        let translation = Vec2::new(
            self.translation_u
                .as_ref()
                .map_or(0.0, |track| track.sample_f32(timestamp)),
            self.translation_v
                .as_ref()
                .map_or(0.0, |track| track.sample_f32(timestamp)),
        );
        let rotation = self
            .rotation
            .as_ref()
            .map_or(0.0, |track| track.sample_f32(timestamp));
        let scale = Vec2::new(
            self.scale_u
                .as_ref()
                .map_or(1.0, |track| track.sample_f32(timestamp)),
            self.scale_v
                .as_ref()
                .map_or(1.0, |track| track.sample_f32(timestamp)),
        );
        Affine2::from_translation(Vec2::splat(0.5) + translation)
            * Affine2::from_angle(rotation)
            * Affine2::from_scale(scale)
            * Affine2::from_translation(Vec2::splat(-0.5))
    }
}

impl M2OpacityAnimation {
    fn sample(&self, elapsed_ms: u32) -> f32 {
        let timestamp = self.start_ms + elapsed_ms % self.duration_ms;
        self.opacity.sample_f32(timestamp).clamp(0.0, 1.0)
    }

    fn requires_blending(&self) -> bool {
        self.opacity.values.iter().any(|opacity| *opacity < 1.0)
    }
}

fn classic_m2_mesh_data(data: &[u8]) -> Result<M2MeshData, String> {
    const VERTICES_DESCRIPTOR_OFFSET: usize = 68;
    const VIEWS_DESCRIPTOR_OFFSET: usize = 76;
    const TEXTURES_DESCRIPTOR_OFFSET: usize = 92;
    const TRANSPARENCY_ANIMATIONS_DESCRIPTOR_OFFSET: usize = 100;
    const TEXTURE_ANIMATIONS_DESCRIPTOR_OFFSET: usize = 116;
    const RENDER_FLAGS_DESCRIPTOR_OFFSET: usize = 132;
    const TEXTURE_LOOKUP_DESCRIPTOR_OFFSET: usize = 148;
    const TRANSPARENCY_LOOKUP_DESCRIPTOR_OFFSET: usize = 164;
    const TEXTURE_ANIMATION_LOOKUP_DESCRIPTOR_OFFSET: usize = 172;
    const CLASSIC_VERTEX_SIZE: usize = 48;
    const SKIN_BATCH_SIZE: usize = 24;
    const TRANSPARENCY_ANIMATION_SIZE: usize = 28;
    const TEXTURE_ANIMATION_SIZE: usize = 84;

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
    let particle_animation = classic_m2_particle_animation(data);
    let mut particles = read_classic_particle_emitters(data, &textures)?;
    for particle in &mut particles {
        particle.animation = particle_animation.clone();
    }
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
    let (texture_animation_count, texture_animation_offset) =
        read_array_descriptor(data, TEXTURE_ANIMATIONS_DESCRIPTOR_OFFSET)?;
    checked_array_range(
        data,
        texture_animation_offset,
        texture_animation_count,
        TEXTURE_ANIMATION_SIZE,
    )?;
    let (texture_animation_lookup_count, texture_animation_lookup_offset) =
        read_array_descriptor(data, TEXTURE_ANIMATION_LOOKUP_DESCRIPTOR_OFFSET)?;
    let texture_animation_lookup = read_u16_array(
        data,
        texture_animation_lookup_offset,
        texture_animation_lookup_count,
    )?;
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
            let texture_transform_combo_index = read_u16(data, offset + 22).ok()? as usize;
            let texture_animation = texture_animation_lookup
                .get(texture_transform_combo_index)
                .copied()
                .filter(|index| *index != u16::MAX)
                .and_then(|index| {
                    classic_m2_texture_animation(
                        data,
                        texture_animation_offset,
                        texture_animation_count,
                        index as usize,
                    )
                });
            let opacity_animation = classic_m2_opacity_animation(
                data,
                &transparency_lookup,
                transparency_animation_offset,
                transparency_animation_count,
                texture_weight_combo_index,
            );
            let opacity = opacity_animation
                .as_ref()
                .map_or(1.0, |animation| animation.sample(0));
            Some(ObjectBatch {
                indices: indices.get(triangle_start..end)?.to_vec(),
                texture: textures
                    .get(texture_index)
                    .and_then(|texture| texture.filename.clone()),
                texture_type: textures
                    .get(texture_index)
                    .map_or(0, |texture| texture.texture_type),
                texture_animation,
                alpha_mode: alpha_mode_with_animation(
                    blend_mode,
                    opacity,
                    opacity_animation
                        .as_deref()
                        .is_some_and(M2OpacityAnimation::requires_blending),
                ),
                opacity,
                opacity_animation,
                double_sided: flags & 0x04 != 0,
            })
        })
        .collect();

    Ok(M2MeshData {
        positions,
        normals,
        uvs,
        animation: None,
        indices,
        batches,
        particles,
    })
}

fn classic_m2_particle_animation(data: &[u8]) -> Option<Arc<M2SkinAnimation>> {
    const BONES_DESCRIPTOR_OFFSET: usize = 52;
    const BONE_SIZE: usize = 108;
    let (bone_count, bone_offset) = read_array_descriptor(data, BONES_DESCRIPTOR_OFFSET).ok()?;
    checked_array_range(data, bone_offset, bone_count, BONE_SIZE).ok()?;

    let bones = (0..bone_count)
        .map(|index| {
            let bone = bone_offset + index * BONE_SIZE;
            Some(AnimatedBone {
                parent: LittleEndian::read_i16(data.get(bone + 8..bone + 10)?),
                pivot: Vec3::from_array(wow_to_bevy(
                    read_f32(data, bone + 96).ok()?,
                    read_f32(data, bone + 100).ok()?,
                    read_f32(data, bone + 104).ok()?,
                )),
                translation: classic_bone_vec3_track(data, bone + 12),
                rotation: classic_bone_quat_track(data, bone + 40),
                scale: classic_bone_vec3_track(data, bone + 68),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let start_ms = bones
        .iter()
        .flat_map(|bone| {
            [
                bone.translation
                    .as_ref()
                    .and_then(|track| track.timestamps.first()),
                bone.rotation
                    .as_ref()
                    .and_then(|track| track.timestamps.first()),
                bone.scale
                    .as_ref()
                    .and_then(|track| track.timestamps.first()),
            ]
            .into_iter()
            .flatten()
            .copied()
        })
        .min()?;
    let end_ms = bones
        .iter()
        .flat_map(|bone| {
            [
                bone.translation
                    .as_ref()
                    .and_then(|track| track.timestamps.last()),
                bone.rotation
                    .as_ref()
                    .and_then(|track| track.timestamps.last()),
                bone.scale
                    .as_ref()
                    .and_then(|track| track.timestamps.last()),
            ]
            .into_iter()
            .flatten()
            .copied()
        })
        .max()?;
    let duration_ms = end_ms.saturating_sub(start_ms);
    (duration_ms > 0).then(|| {
        Arc::new(M2SkinAnimation {
            vertices: Vec::new(),
            bones,
            duration_ms,
            start_ms,
        })
    })
}

fn classic_bone_track_data<'a>(
    data: &'a [u8],
    track: usize,
    value_size: usize,
) -> Option<(Vec<u32>, &'a [u8])> {
    let (timestamp_count, timestamp_offset) = read_array_descriptor(data, track + 12).ok()?;
    let (value_count, value_offset) = read_array_descriptor(data, track + 20).ok()?;
    if timestamp_count == 0 || timestamp_count != value_count {
        return None;
    }
    let timestamp_bytes =
        data.get(timestamp_offset..timestamp_offset.checked_add(timestamp_count * 4)?)?;
    let values = data.get(value_offset..value_offset.checked_add(value_count * value_size)?)?;
    Some((
        timestamp_bytes
            .chunks_exact(4)
            .map(LittleEndian::read_u32)
            .collect(),
        values,
    ))
}

fn classic_bone_vec3_track(data: &[u8], track: usize) -> Option<AnimationTrack<Vec3>> {
    let (timestamps, values) = classic_bone_track_data(data, track, 12)?;
    let values = values
        .chunks_exact(12)
        .map(|bytes| {
            Vec3::from_array(wow_to_bevy(
                LittleEndian::read_f32(&bytes[..4]),
                LittleEndian::read_f32(&bytes[4..8]),
                LittleEndian::read_f32(&bytes[8..]),
            ))
        })
        .collect::<Vec<_>>();
    values
        .iter()
        .all(|value| value.is_finite())
        .then_some(AnimationTrack { timestamps, values })
}

fn classic_bone_quat_track(data: &[u8], track: usize) -> Option<AnimationTrack<Quat>> {
    let (timestamps, values) = classic_bone_track_data(data, track, 16)?;
    let values = values
        .chunks_exact(16)
        .map(|bytes| {
            wow_quat_to_bevy(
                LittleEndian::read_f32(&bytes[..4]),
                LittleEndian::read_f32(&bytes[4..8]),
                LittleEndian::read_f32(&bytes[8..12]),
                LittleEndian::read_f32(&bytes[12..]),
            )
        })
        .collect::<Vec<_>>();
    values
        .iter()
        .all(|value| value.is_finite())
        .then_some(AnimationTrack { timestamps, values })
}

fn read_classic_particle_emitters(
    data: &[u8],
    textures: &[M2TextureReference],
) -> Result<Vec<PreparedParticleEmitter>, String> {
    const PARTICLE_DESCRIPTOR_OFFSET: usize = 316;
    const PARTICLE_SIZE: usize = 504;
    let (count, offset) = read_array_descriptor(data, PARTICLE_DESCRIPTOR_OFFSET)?;
    checked_array_range(data, offset, count, PARTICLE_SIZE)?;
    (0..count)
        .map(|index| {
            let record = offset + index * PARTICLE_SIZE;
            let texture_index = read_u16(data, record + 22)?;
            let track = |relative| classic_particle_float_track(data, record + relative);
            let mut colors = [[1.0; 4]; 3];
            for (key, color) in colors.iter_mut().enumerate() {
                let value = record + 336 + key * 4;
                color[0] = f32::from(data[value + 2]) / 255.0;
                color[1] = f32::from(data[value + 1]) / 255.0;
                color[2] = f32::from(data[value]) / 255.0;
                color[3] = f32::from(data[value + 3]) / 255.0;
            }
            let emitter = ClassicParticleEmitter {
                flags: read_u32(data, record + 4)?,
                position: Vec3::new(
                    read_f32(data, record + 8)?,
                    read_f32(data, record + 12)?,
                    read_f32(data, record + 16)?,
                ),
                bone_index: read_u16(data, record + 20)?,
                blending_type: read_u16(data, record + 40)?,
                emitter_type: read_u16(data, record + 42)?,
                rows: read_u16(data, record + 48)?.max(1),
                columns: read_u16(data, record + 50)?.max(1),
                emission_speed: track(52)?,
                speed_variation: track(80)?,
                vertical_range: track(108)?,
                horizontal_range: track(136)?,
                gravity: track(164)?,
                lifespan: track(192)?,
                emission_rate: track(220)?,
                area_length: track(248)?,
                area_width: track(276)?,
                midpoint: read_f32(data, record + 332)?.clamp(0.0, 1.0),
                colors,
                scales: [
                    read_f32(data, record + 348)?,
                    read_f32(data, record + 352)?,
                    read_f32(data, record + 356)?,
                ],
            };
            if !emitter.lifespan.is_finite()
                || emitter.lifespan <= 0.0
                || !emitter.emission_rate.is_finite()
                || emitter.emission_rate <= 0.0
            {
                return Err(format!("invalid Classic particle emitter {index}"));
            }
            Ok(PreparedParticleEmitter {
                texture_key: textures
                    .get(texture_index as usize)
                    .and_then(|texture| texture.filename.clone()),
                emitter: ParticleEmitterDefinition::Classic(emitter),
                animation: None,
            })
        })
        .collect()
}

fn classic_particle_float_track(data: &[u8], offset: usize) -> Result<f32, String> {
    let (count, values_offset) = read_array_descriptor(data, offset + 20)?;
    if count == 0 {
        return Ok(0.0);
    }
    let value = read_f32(data, values_offset)?;
    value
        .is_finite()
        .then_some(value)
        .ok_or_else(|| format!("non-finite Classic particle track at {offset}"))
}

fn classic_m2_texture_animation(
    data: &[u8],
    animations_offset: usize,
    animation_count: usize,
    animation_index: usize,
) -> Option<Arc<M2TextureAnimation>> {
    const TEXTURE_ANIMATION_SIZE: usize = 84;
    const ANIMATION_BLOCK_SIZE: usize = 28;

    if animation_index >= animation_count {
        return None;
    }
    let animation = animations_offset.checked_add(animation_index * TEXTURE_ANIMATION_SIZE)?;
    let translation =
        classic_texture_track(data, animation, 12, |bytes| LittleEndian::read_f32(bytes));
    let rotation = classic_texture_track(data, animation + ANIMATION_BLOCK_SIZE, 16, |bytes| {
        let z = LittleEndian::read_f32(&bytes[8..12]);
        let w = LittleEndian::read_f32(&bytes[12..16]);
        2.0 * z.atan2(w)
    });
    let scale = classic_texture_track(
        data,
        animation + ANIMATION_BLOCK_SIZE * 2,
        12,
        LittleEndian::read_f32,
    );
    let duration_ms = translation
        .as_ref()
        .into_iter()
        .chain(rotation.as_ref())
        .chain(scale.as_ref())
        .filter_map(|track| track.timestamps.last().copied())
        .max()?;
    if duration_ms == 0 {
        return None;
    }

    Some(Arc::new(M2TextureAnimation {
        translation_u: translation.as_ref().map(|track| track.component(0)),
        translation_v: translation.as_ref().map(|track| track.component(1)),
        rotation: rotation.map(|track| track.component(0)),
        scale_u: scale.as_ref().map(|track| track.component(0)),
        scale_v: scale.as_ref().map(|track| track.component(1)),
        duration_ms,
        start_ms: 0,
    }))
}

fn classic_texture_track(
    data: &[u8],
    track_offset: usize,
    value_size: usize,
    read_component: impl Fn(&[u8]) -> f32,
) -> Option<AnimationTrack<Vec3>> {
    let (timestamp_count, timestamp_offset) =
        read_array_descriptor(data, track_offset + 12).ok()?;
    let (value_count, value_offset) = read_array_descriptor(data, track_offset + 20).ok()?;
    if timestamp_count == 0 || timestamp_count != value_count {
        return None;
    }
    checked_array_range(data, timestamp_offset, timestamp_count, 4).ok()?;
    checked_array_range(data, value_offset, value_count, value_size).ok()?;
    let timestamps = (0..timestamp_count)
        .map(|index| LittleEndian::read_u32(&data[timestamp_offset + index * 4..]))
        .collect::<Vec<_>>();
    let values = (0..value_count)
        .map(|index| {
            let offset = value_offset + index * value_size;
            let bytes = &data[offset..offset + value_size];
            Vec3::new(
                read_component(bytes),
                if value_size >= 8 {
                    LittleEndian::read_f32(&bytes[4..8])
                } else {
                    0.0
                },
                if value_size == 12 {
                    LittleEndian::read_f32(&bytes[8..12])
                } else {
                    0.0
                },
            )
        })
        .collect::<Vec<_>>();
    values
        .iter()
        .all(|value| value.is_finite())
        .then_some(AnimationTrack { timestamps, values })
}

impl AnimationTrack<Vec3> {
    fn component(&self, component: usize) -> AnimationTrack<f32> {
        AnimationTrack {
            timestamps: self.timestamps.clone(),
            values: self.values.iter().map(|value| value[component]).collect(),
        }
    }
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
fn build_m2_asset(
    mut mesh_data: M2MeshData,
    mpqs: &PatchChain,
    cache: &PreparedObjectCache,
) -> PreparedM2Asset {
    if mesh_data.batches.is_empty() {
        mesh_data.batches.push(ObjectBatch {
            indices: mesh_data.indices.clone(),
            texture: None,
            texture_type: 0,
            texture_animation: None,
            opacity_animation: None,
            alpha_mode: AlphaMode::Opaque,
            opacity: 1.0,
            double_sided: false,
        });
    }

    let animation = mesh_data.animation.clone();
    let particles = mesh_data
        .particles
        .into_iter()
        .map(|mut emitter| {
            emitter.texture_key = emitter
                .texture_key
                .as_deref()
                .map(|filename| load_object_texture(filename, mpqs, cache));
            emitter
        })
        .collect();
    let parts = mesh_data
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
                animation: animation.clone(),
                texture_animation: batch.texture_animation,
                opacity_animation: batch.opacity_animation,
                uvs: mesh_data.uvs.clone(),
                texture_key: texture,
                double_sided: batch.double_sided,
                opacity: batch.opacity,
                alpha_mode: batch.alpha_mode,
            }
        })
        .collect();
    PreparedM2Asset { parts, particles }
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

fn parsed_particle_alpha_mode(blend_mode: u8) -> AlphaMode {
    match blend_mode {
        0 => AlphaMode::Opaque,
        1 => AlphaMode::Mask(0.5),
        2 => AlphaMode::Blend,
        3 => AlphaMode::Add,
        4 => AlphaMode::Multiply,
        _ => AlphaMode::Blend,
    }
}

fn classic_particle_alpha_mode(blend_mode: u16) -> AlphaMode {
    match blend_mode {
        0 => AlphaMode::Opaque,
        1 | 4 => AlphaMode::Add,
        2 => AlphaMode::Blend,
        3 => AlphaMode::Mask(0.5),
        _ => AlphaMode::Blend,
    }
}

impl ParticleEmitterDefinition {
    fn alpha_mode(&self) -> AlphaMode {
        match self {
            Self::Parsed(emitter) => parsed_particle_alpha_mode(emitter.blending_type),
            Self::Classic(emitter) => classic_particle_alpha_mode(emitter.blending_type),
        }
    }

    fn is_lit(&self) -> bool {
        match self {
            Self::Parsed(emitter) => emitter.flags.contains(wow_m2::M2ParticleFlags::LIGHTING),
            Self::Classic(emitter) => emitter.flags & 0x400 != 0,
        }
    }

    fn bounds_radius(&self) -> f32 {
        let (position, area, speed, gravity, lifespan, size) = match self {
            Self::Parsed(emitter) => (
                Vec3::new(emitter.position.x, emitter.position.y, emitter.position.z).length(),
                emitter
                    .emission_area_length
                    .abs()
                    .max(emitter.emission_area_width.abs()),
                emitter
                    .emission_velocity
                    .abs()
                    .max(emitter.initial_velocity.abs())
                    * (1.0 + emitter.speed_variation.abs()),
                emitter.gravity.abs(),
                emitter.lifetime.abs().max(emitter.max_lifetime.abs()),
                emitter
                    .initial_size
                    .abs()
                    .max(emitter.max_initial_size.abs()),
            ),
            Self::Classic(emitter) => (
                emitter.position.length(),
                emitter.area_length.abs().max(emitter.area_width.abs()),
                emitter.emission_speed.abs() * (1.0 + emitter.speed_variation.abs()),
                emitter.gravity.abs(),
                emitter.lifespan.abs(),
                emitter.scales.into_iter().map(f32::abs).fold(0.0, f32::max),
            ),
        };
        let radius = position + area + speed * lifespan + 0.5 * gravity * lifespan.powi(2) + size;
        if radius.is_finite() {
            radius.clamp(25.0, PARTICLE_VIEW_DISTANCE)
        } else {
            100.0
        }
    }
}

struct ParticleRenderData {
    position: Vec3,
    color: [f32; 4],
    size: Vec2,
    uv_min: Vec2,
    uv_size: Vec2,
}

impl ParticleEmitterRuntime {
    fn new(definition: &ParticleEmitterDefinition) -> Self {
        match definition {
            ParticleEmitterDefinition::Parsed(emitter) => {
                Self::Parsed(ParticleEmitter::new(emitter))
            }
            ParticleEmitterDefinition::Classic(emitter) => Self::Classic(ClassicParticleRuntime {
                definition: emitter.clone(),
                particles: Vec::new(),
                emission_remainder: 0.0,
                random_state: 0x4d59_5df4_d0f3_3173,
            }),
        }
    }

    fn bone_index(&self) -> u16 {
        match self {
            Self::Parsed(emitter) => emitter.bone_index(),
            Self::Classic(emitter) => emitter.definition.bone_index,
        }
    }

    fn update(&mut self, delta_seconds: f32, identity: &[f32; 16], bone_transform: Mat4) {
        match self {
            Self::Parsed(emitter) => emitter.update(delta_seconds * 1000.0, identity, identity),
            Self::Classic(emitter) => emitter.update(delta_seconds, bone_transform),
        }
    }

    fn render_transform(&self, bone_transform: Mat4) -> Mat4 {
        match self {
            Self::Classic(emitter) if emitter.definition.flags & 0x10 != 0 => Mat4::IDENTITY,
            _ => bone_transform,
        }
    }

    fn write_render_data(&self, output: &mut Vec<ParticleRenderData>) {
        output.clear();
        match self {
            Self::Parsed(emitter) => output.extend(
                emitter
                    .fill_texture_data()
                    .chunks_exact(wow_m2::TEXELS_PER_PARTICLE * 4)
                    .take(emitter.particle_count())
                    .map(|particle| ParticleRenderData {
                        position: Vec3::new(-particle[0], particle[2], particle[1]),
                        color: [particle[4], particle[5], particle[6], particle[7]],
                        size: Vec2::new(particle[8], particle[9]),
                        uv_min: Vec2::new(particle[12], particle[13]),
                        uv_size: Vec2::ONE,
                    }),
            ),
            Self::Classic(emitter) => emitter.write_render_data(output),
        }
    }
}

impl ClassicParticleRuntime {
    fn update(&mut self, delta_seconds: f32, bone_transform: Mat4) {
        let delta_seconds = delta_seconds.min(0.1);
        self.emission_remainder += self.definition.emission_rate * delta_seconds;
        let spawn_count = self.emission_remainder.floor().min(256.0) as usize;
        self.emission_remainder -= spawn_count as f32;
        for _ in 0..spawn_count {
            self.spawn(bone_transform);
        }
        let gravity = Vec3::new(0.0, -self.definition.gravity, 0.0);
        self.particles.retain_mut(|particle| {
            particle.age += delta_seconds;
            if particle.age >= particle.lifespan {
                return false;
            }
            particle.velocity += gravity * delta_seconds;
            particle.position += particle.velocity * delta_seconds;
            true
        });
    }

    fn spawn(&mut self, bone_transform: Mat4) {
        let random_a = self.random_signed();
        let random_b = self.random_signed();
        let speed = self.definition.emission_speed
            * (1.0 + self.definition.speed_variation * self.random_signed());
        let (position, direction) = if self.definition.emitter_type == 2 {
            let azimuth = random_a * self.definition.horizontal_range;
            let elevation = random_b * self.definition.vertical_range;
            let direction = Vec3::new(
                elevation.cos() * azimuth.sin(),
                elevation.cos() * azimuth.cos(),
                elevation.sin(),
            );
            let radius = self.definition.area_length
                + self.random_unit() * (self.definition.area_width - self.definition.area_length);
            (direction * radius, direction)
        } else {
            let position = Vec3::new(
                random_a * self.definition.area_length * 0.5,
                random_b * self.definition.area_width * 0.5,
                0.0,
            );
            let polar = self.random_signed() * self.definition.vertical_range;
            let azimuth = self.random_signed() * self.definition.horizontal_range;
            (
                position,
                Vec3::new(
                    polar.sin() * azimuth.cos(),
                    polar.sin() * azimuth.sin(),
                    polar.cos(),
                ),
            )
        };
        let position = Vec3::from_array(wow_to_bevy(
            self.definition.position.x + position.x,
            self.definition.position.y + position.y,
            self.definition.position.z + position.z,
        ));
        let velocity = Vec3::from_array(wow_to_bevy(direction.x, direction.y, direction.z))
            .normalize_or_zero()
            * speed;
        let world_space = self.definition.flags & 0x10 != 0;
        self.particles.push(ClassicParticle {
            position: if world_space {
                bone_transform.transform_point3(position)
            } else {
                position
            },
            velocity: if world_space {
                bone_transform.transform_vector3(velocity)
            } else {
                velocity
            },
            age: 0.0,
            lifespan: self.definition.lifespan,
        });
    }

    fn random_unit(&mut self) -> f32 {
        self.random_state = self
            .random_state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.random_state >> 40) as u32) as f32 / 16_777_216.0
    }

    fn random_signed(&mut self) -> f32 {
        self.random_unit() * 2.0 - 1.0
    }

    fn write_render_data(&self, output: &mut Vec<ParticleRenderData>) {
        let rows = f32::from(self.definition.rows);
        let columns = f32::from(self.definition.columns);
        let uv_size = Vec2::new(1.0 / columns, 1.0 / rows);
        output.extend(self.particles.iter().enumerate().map(|(index, particle)| {
            let life = (particle.age / particle.lifespan).clamp(0.0, 1.0);
            let color = life_ramp(
                life,
                self.definition.midpoint,
                self.definition.colors.map(Vec4::from_array),
            )
            .to_array();
            let scale = life_ramp(life, self.definition.midpoint, self.definition.scales);
            let tile = index as u32
                % (u32::from(self.definition.rows) * u32::from(self.definition.columns));
            let column = tile % u32::from(self.definition.columns);
            let row = tile / u32::from(self.definition.columns);
            ParticleRenderData {
                position: particle.position,
                color,
                size: Vec2::splat(scale.abs().max(0.01)),
                uv_min: Vec2::new(column as f32 / columns, row as f32 / rows),
                uv_size,
            }
        }));
    }
}

fn life_ramp<T>(life: f32, midpoint: f32, values: [T; 3]) -> T
where
    T: Copy + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    if life <= midpoint && midpoint > f32::EPSILON {
        let factor = life / midpoint;
        values[0] * (1.0 - factor) + values[1] * factor
    } else if midpoint < 1.0 {
        let factor = (life - midpoint) / (1.0 - midpoint);
        values[1] * (1.0 - factor) + values[2] * factor
    } else {
        values[2]
    }
}

fn alpha_mode_with_opacity(blend_mode: u16, opacity: f32) -> AlphaMode {
    match alpha_mode(blend_mode) {
        AlphaMode::Opaque if opacity < 1.0 => AlphaMode::Blend,
        alpha_mode => alpha_mode,
    }
}

fn alpha_mode_with_animation(blend_mode: u16, opacity: f32, animated: bool) -> AlphaMode {
    match alpha_mode_with_opacity(blend_mode, opacity) {
        AlphaMode::Opaque if animated => AlphaMode::Blend,
        alpha_mode => alpha_mode,
    }
}

fn classic_m2_opacity_animation(
    data: &[u8],
    transparency_lookup: &[u16],
    animation_offset: usize,
    animation_count: usize,
    texture_weight_combo_index: usize,
) -> Option<Arc<M2OpacityAnimation>> {
    const TRANSPARENCY_ANIMATION_SIZE: usize = 28;
    const TIMESTAMPS_DESCRIPTOR_OFFSET: usize = 12;
    const VALUES_DESCRIPTOR_OFFSET: usize = 20;

    let animation_index = *transparency_lookup.get(texture_weight_combo_index)? as usize;
    if animation_index >= animation_count {
        return None;
    }
    let animation =
        animation_offset.checked_add(animation_index.checked_mul(TRANSPARENCY_ANIMATION_SIZE)?)?;
    let (timestamp_count, timestamp_offset) =
        read_array_descriptor(data, animation + TIMESTAMPS_DESCRIPTOR_OFFSET).ok()?;
    let (value_count, value_offset) =
        read_array_descriptor(data, animation + VALUES_DESCRIPTOR_OFFSET).ok()?;
    if timestamp_count == 0 || timestamp_count != value_count {
        return None;
    }
    checked_array_range(data, timestamp_offset, timestamp_count, 4).ok()?;
    checked_array_range(data, value_offset, value_count, 2).ok()?;
    let timestamps = (0..timestamp_count)
        .map(|index| LittleEndian::read_u32(&data[timestamp_offset + index * 4..]))
        .collect::<Vec<_>>();
    let values = (0..value_count)
        .filter_map(|index| fixed_i16_alpha(&data[value_offset + index * 2..]))
        .collect::<Vec<_>>();
    let duration_ms = timestamps.last().copied()?;
    (duration_ms > 0 && timestamps.len() == values.len()).then(|| {
        Arc::new(M2OpacityAnimation {
            opacity: AnimationTrack { timestamps, values },
            duration_ms,
            start_ms: 0,
        })
    })
}

fn fixed_i16_alpha(values: &[u8]) -> Option<f32> {
    let value = i16::from_le_bytes(values.get(..2)?.try_into().ok()?);
    Some((f32::from(value) / f32::from(i16::MAX)).clamp(0.0, 1.0))
}

fn fixed_i16_alpha_values(values: &[u8], start: usize, count: usize) -> Vec<f32> {
    values
        .chunks_exact(2)
        .skip(start)
        .take(count)
        .filter_map(fixed_i16_alpha)
        .collect()
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
    let mut liquids = Vec::new();
    let mut visible_doodads = HashSet::new();
    for group_index in 0..root.n_groups {
        let group_filename = format!("{stem}_{group_index:03}.wmo");
        let Some(group_data) = mpq_read_file(mpqs, &group_filename).ok() else {
            warn!("Unable to read WMO group {group_filename}");
            continue;
        };
        let Some(ParsedWmo::Group(group)) = parse_wmo(&mut Cursor::new(&group_data)).ok() else {
            warn!("Unable to parse WMO group {group_filename}");
            continue;
        };
        let material_liquid_type = wmo_chunk_payload(&group_data, b"QILM")
            .or_else(|| wmo_chunk_payload(&group_data, b"MLIQ"))
            .and_then(|payload| payload.get(28..30))
            .map(LittleEndian::read_u16)
            .and_then(|material_id| root.materials.get(material_id as usize))
            .and_then(|material| root.texture_offset_index_map.get(&material.texture_1))
            .and_then(|texture_index| root.textures.get(*texture_index as usize))
            .and_then(|texture| wmo_liquid_type_from_texture(texture));
        liquids.extend(
            wmo_liquid_meshes(&group_data, group.group_liquid, material_liquid_type)
                .into_iter()
                .map(|(liquid_type, mesh)| PreparedWmoLiquid {
                    mesh: Mutex::new(Some(mesh)),
                    liquid_type,
                }),
        );
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
                    texture_animation: None,
                    opacity_animation: None,
                    alpha_mode: alpha_mode(material.blend_mode as u16),
                    opacity: 1.0,
                    double_sided: material.flags & 0x04 != 0,
                })
            })
            .collect();
        object_parts.extend(
            build_m2_asset(
                M2MeshData {
                    positions,
                    normals,
                    uvs,
                    animation: None,
                    indices,
                    batches,
                    particles: Vec::new(),
                },
                mpqs,
                cache,
            )
            .parts,
        );
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
        liquids,
        doodad_sets,
    })
}

fn wmo_liquid_meshes(
    data: &[u8],
    group_liquid: u32,
    material_liquid_type: Option<LiquidType>,
) -> Vec<(LiquidType, Mesh)> {
    const HEADER_SIZE: usize = 30;
    const VERTEX_SIZE: usize = 8;
    const TILE_SIZE: f32 = CHUNK_SIZE / 8.0;

    let Some(payload) =
        wmo_chunk_payload(data, b"QILM").or_else(|| wmo_chunk_payload(data, b"MLIQ"))
    else {
        return Vec::new();
    };
    if payload.len() < HEADER_SIZE {
        return Vec::new();
    }

    let vertex_columns = LittleEndian::read_u32(&payload[0..4]) as usize;
    let vertex_rows = LittleEndian::read_u32(&payload[4..8]) as usize;
    let tile_columns = LittleEndian::read_u32(&payload[8..12]) as usize;
    let tile_rows = LittleEndian::read_u32(&payload[12..16]) as usize;
    if vertex_columns != tile_columns.saturating_add(1)
        || vertex_rows != tile_rows.saturating_add(1)
        || tile_columns == 0
        || tile_rows == 0
    {
        return Vec::new();
    }

    let Some(vertex_count) = vertex_columns.checked_mul(vertex_rows) else {
        return Vec::new();
    };
    let Some(tile_count) = tile_columns.checked_mul(tile_rows) else {
        return Vec::new();
    };
    let Some(vertices_size) = vertex_count.checked_mul(VERTEX_SIZE) else {
        return Vec::new();
    };
    let Some(tile_flags_offset) = HEADER_SIZE.checked_add(vertices_size) else {
        return Vec::new();
    };
    if payload.len() < tile_flags_offset.saturating_add(tile_count) {
        return Vec::new();
    }

    let corner_x = LittleEndian::read_f32(&payload[16..20]);
    let corner_y = LittleEndian::read_f32(&payload[20..24]);
    let vertices_data = &payload[HEADER_SIZE..tile_flags_offset];
    let tile_flags = &payload[tile_flags_offset..tile_flags_offset + tile_count];
    let mut positions = std::array::from_fn::<_, 4, _>(|_| Vec::new());
    let mut uvs = std::array::from_fn::<_, 4, _>(|_| Vec::new());
    let mut indices = std::array::from_fn::<_, 4, _>(|_| Vec::new());

    for row in 0..tile_rows {
        for column in 0..tile_columns {
            let tile_flag = tile_flags[row * tile_columns + column];
            if tile_flag & 0x0f == 0x0f {
                continue;
            }
            let liquid_type = wmo_liquid_type(group_liquid, tile_flag, material_liquid_type);
            let type_index = liquid_type as usize;
            let vertex_offset = positions[type_index].len() as u32;
            for (x, y) in [
                (column, row),
                (column, row + 1),
                (column + 1, row),
                (column + 1, row + 1),
            ] {
                let source = (y * vertex_columns + x) * VERTEX_SIZE;
                let height = LittleEndian::read_f32(&vertices_data[source + 4..source + 8]);
                positions[type_index].push(wow_to_bevy(
                    corner_x + x as f32 * TILE_SIZE,
                    corner_y + y as f32 * TILE_SIZE,
                    height,
                ));
                uvs[type_index].push(if liquid_type == LiquidType::Magma {
                    [
                        LittleEndian::read_i16(&vertices_data[source..source + 2]) as f32
                            * (3.0 / 256.0),
                        LittleEndian::read_i16(&vertices_data[source + 2..source + 4]) as f32
                            * (3.0 / 256.0),
                    ]
                } else {
                    [x as f32, y as f32]
                });
            }
            indices[type_index].extend_from_slice(&[
                vertex_offset,
                vertex_offset + 1,
                vertex_offset + 2,
                vertex_offset + 2,
                vertex_offset + 1,
                vertex_offset + 3,
            ]);
        }
    }

    [
        LiquidType::Water,
        LiquidType::Ocean,
        LiquidType::Magma,
        LiquidType::Slime,
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(type_index, liquid_type)| {
        (!indices[type_index].is_empty()).then(|| {
            let vertex_count = positions[type_index].len();
            (
                liquid_type,
                Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::default(),
                )
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    std::mem::take(&mut positions[type_index]),
                )
                .with_inserted_attribute(
                    Mesh::ATTRIBUTE_NORMAL,
                    vec![[0.0, 1.0, 0.0]; vertex_count],
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, std::mem::take(&mut uvs[type_index]))
                .with_inserted_indices(Indices::U32(std::mem::take(&mut indices[type_index]))),
            )
        })
    })
    .collect()
}

fn wmo_chunk_payload<'a>(data: &'a [u8], id: &[u8; 4]) -> Option<&'a [u8]> {
    data.windows(4)
        .position(|window| window == id)
        .and_then(|offset| {
            let size = LittleEndian::read_u32(data.get(offset + 4..offset + 8)?) as usize;
            data.get(offset + 8..offset + 8 + size)
        })
}

fn wmo_liquid_type(
    group_liquid: u32,
    tile_flag: u8,
    material_liquid_type: Option<LiquidType>,
) -> LiquidType {
    let basic_type = if group_liquid == 15 {
        if let Some(liquid_type) = material_liquid_type {
            return liquid_type;
        }
        u32::from(tile_flag & 0x0f)
    } else {
        group_liquid & 0x03
    };
    match basic_type {
        1 => LiquidType::Ocean,
        2 => LiquidType::Magma,
        3 => LiquidType::Slime,
        _ => LiquidType::Water,
    }
}

fn wmo_liquid_type_from_texture(texture: &str) -> Option<LiquidType> {
    let texture = texture.to_ascii_lowercase();
    if texture.contains("lava") || texture.contains("magma") {
        Some(LiquidType::Magma)
    } else if texture.contains("slime") {
        Some(LiquidType::Slime)
    } else if texture.contains("ocean") {
        Some(LiquidType::Ocean)
    } else if texture.contains("water") || texture.contains("river") {
        Some(LiquidType::Water)
    } else {
        None
    }
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

fn empty_particle_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0; 3]])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0; 3]])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0; 2]])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0; 4]])
    .with_inserted_indices(Indices::U32(Vec::new()))
}

#[allow(clippy::too_many_arguments)]
fn update_particle_mesh(
    mesh: &mut Mesh,
    data: &[ParticleRenderData],
    right: Vec3,
    up: Vec3,
    bone_transform: Mat4,
) {
    let normal = right.cross(up).normalize_or_zero();

    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    positions.clear();
    positions.reserve((data.len() * 4).max(1));
    if data.is_empty() {
        positions.push([0.0; 3]);
    }
    positions.extend(data.iter().flat_map(|particle| {
        let center = bone_transform.transform_point3(particle.position);
        let horizontal = right * particle.size.x;
        let vertical = up * particle.size.y;
        [
            (center - horizontal - vertical).to_array(),
            (center + horizontal - vertical).to_array(),
            (center + horizontal + vertical).to_array(),
            (center - horizontal + vertical).to_array(),
        ]
    }));

    let Some(VertexAttributeValues::Float32x3(normals)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    else {
        return;
    };
    normals.clear();
    normals.reserve((data.len() * 4).max(1));
    if data.is_empty() {
        normals.push([0.0; 3]);
    }
    for _ in data {
        normals.extend([normal.to_array(); 4]);
    }

    let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
    else {
        return;
    };
    uvs.clear();
    uvs.reserve((data.len() * 4).max(1));
    if data.is_empty() {
        uvs.push([0.0; 2]);
    }
    for particle in data {
        let uv = particle.uv_min;
        let uv_end = uv + particle.uv_size;
        uvs.extend([
            [uv.x, uv_end.y],
            [uv_end.x, uv_end.y],
            [uv_end.x, uv.y],
            [uv.x, uv.y],
        ]);
    }

    let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
    else {
        return;
    };
    colors.clear();
    colors.reserve((data.len() * 4).max(1));
    if data.is_empty() {
        colors.push([0.0; 4]);
    }
    for particle in data {
        colors.extend([particle.color; 4]);
    }

    let Some(Indices::U32(indices)) = mesh.indices_mut() else {
        return;
    };
    indices.clear();
    indices.reserve(data.len() * 6);
    for particle_index in 0..data.len() {
        let vertex = (particle_index * 4) as u32;
        indices.extend([
            vertex,
            vertex + 1,
            vertex + 2,
            vertex,
            vertex + 2,
            vertex + 3,
        ]);
    }
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
    fn parses_classic_m2_bone_rotation() {
        let mut data = vec![0_u8; 280];
        data[52..56].copy_from_slice(&1_u32.to_le_bytes());
        data[56..60].copy_from_slice(&128_u32.to_le_bytes());
        data[136..138].copy_from_slice(&(-1_i16).to_le_bytes());
        data[180..184].copy_from_slice(&2_u32.to_le_bytes());
        data[184..188].copy_from_slice(&240_u32.to_le_bytes());
        data[188..192].copy_from_slice(&2_u32.to_le_bytes());
        data[192..196].copy_from_slice(&248_u32.to_le_bytes());
        data[240..244].copy_from_slice(&3_333_u32.to_le_bytes());
        data[244..248].copy_from_slice(&6_667_u32.to_le_bytes());
        for (offset, value) in [
            (260, 1.0_f32),
            (272, std::f32::consts::FRAC_1_SQRT_2),
            (276, std::f32::consts::FRAC_1_SQRT_2),
        ] {
            data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }

        let animation = classic_m2_particle_animation(&data).unwrap();
        assert_eq!(animation.start_ms, 3_333);
        assert_eq!(animation.duration_ms, 3_334);
        let transform = animation.bone_transforms(3_333)[0];
        assert!(
            transform
                .transform_vector3(Vec3::X)
                .abs_diff_eq(Vec3::NEG_Z, 0.01)
        );
    }

    #[test]
    fn parses_classic_m2_texture_translation() {
        let mut data = vec![0_u8; 160];
        data[12..16].copy_from_slice(&2_u32.to_le_bytes());
        data[16..20].copy_from_slice(&128_u32.to_le_bytes());
        data[20..24].copy_from_slice(&2_u32.to_le_bytes());
        data[24..28].copy_from_slice(&136_u32.to_le_bytes());
        data[128..132].copy_from_slice(&0_u32.to_le_bytes());
        data[132..136].copy_from_slice(&1_000_u32.to_le_bytes());
        data[152..156].copy_from_slice(&(-1.0_f32).to_le_bytes());

        let animation = classic_m2_texture_animation(&data, 0, 1, 0).unwrap();
        let uv = animation.transform(500).transform_point2(Vec2::ZERO);
        assert!(uv.abs_diff_eq(Vec2::new(0.0, -0.5), 0.0001));
    }

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
    fn converts_wow_particle_blend_modes() {
        assert_eq!(parsed_particle_alpha_mode(0), AlphaMode::Opaque);
        assert_eq!(parsed_particle_alpha_mode(1), AlphaMode::Mask(0.5));
        assert_eq!(parsed_particle_alpha_mode(4), AlphaMode::Multiply);
        assert_eq!(classic_particle_alpha_mode(1), AlphaMode::Add);
        assert_eq!(classic_particle_alpha_mode(2), AlphaMode::Blend);
        assert_eq!(classic_particle_alpha_mode(3), AlphaMode::Mask(0.5));
        assert_eq!(classic_particle_alpha_mode(4), AlphaMode::Add);
    }

    #[test]
    fn builds_camera_facing_particle_quad() {
        let mut mesh = empty_particle_mesh();
        let data = [ParticleRenderData {
            position: Vec3::ZERO,
            color: [1.0, 0.5, 0.25, 0.75],
            size: Vec2::splat(2.0),
            uv_min: Vec2::ZERO,
            uv_size: Vec2::ONE,
        }];

        update_particle_mesh(&mut mesh, &data, Vec3::X, Vec3::Y, Mat4::IDENTITY);

        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        assert_eq!(positions.len(), 4);
        assert_eq!(mesh.indices().unwrap().len(), 6);
        let bevy::mesh::VertexAttributeValues::Float32x3(positions) = positions else {
            panic!("particle positions must be Float32x3");
        };
        assert_eq!(positions[0], [-2.0, -2.0, 0.0]);
        assert_eq!(positions[2], [2.0, 2.0, 0.0]);
        let position_capacity = positions.capacity();
        let Indices::U32(indices) = mesh.indices().unwrap() else {
            panic!("particle indices must be U32");
        };
        let index_capacity = indices.capacity();

        update_particle_mesh(&mut mesh, &[], Vec3::X, Vec3::Y, Mat4::IDENTITY);

        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("particle positions must be Float32x3");
        };
        assert_eq!(positions.capacity(), position_capacity);
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("particle indices must be U32");
        };
        assert_eq!(indices.capacity(), index_capacity);
    }

    #[test]
    fn decodes_classic_wmo_magma_liquid() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&2_u32.to_le_bytes());
        payload.extend_from_slice(&2_u32.to_le_bytes());
        payload.extend_from_slice(&1_u32.to_le_bytes());
        payload.extend_from_slice(&1_u32.to_le_bytes());
        payload.extend_from_slice(&10.0_f32.to_le_bytes());
        payload.extend_from_slice(&20.0_f32.to_le_bytes());
        payload.extend_from_slice(&0.0_f32.to_le_bytes());
        payload.extend_from_slice(&0_u16.to_le_bytes());
        for (s, t) in [(0_i16, 0_i16), (256, 0), (0, 256), (256, 256)] {
            payload.extend_from_slice(&s.to_le_bytes());
            payload.extend_from_slice(&t.to_le_bytes());
            payload.extend_from_slice(&5.0_f32.to_le_bytes());
        }
        payload.push(2);
        let mut data = b"QILM".to_vec();
        data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        data.extend_from_slice(&payload);

        let mut liquids = wmo_liquid_meshes(&data, 15, None);

        assert_eq!(liquids.len(), 1);
        let (liquid_type, mesh) = liquids.pop().unwrap();
        assert_eq!(liquid_type, LiquidType::Magma);
        assert_eq!(mesh.indices().unwrap().len(), 6);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("WMO liquid positions must be Float32x3");
        };
        assert_eq!(positions[0], [20.0, 5.0, 10.0]);
    }

    #[test]
    fn blackrock_legacy_liquid_uses_lava_material_hint() {
        assert_eq!(
            wmo_liquid_type_from_texture("DUNGEONS\\TEXTURES\\LAVA\\BURNINGSTEPPSLAVA02.BLP"),
            Some(LiquidType::Magma)
        );
        assert_eq!(
            wmo_liquid_type(15, 6, Some(LiquidType::Magma)),
            LiquidType::Magma
        );
    }

    #[test]
    fn classic_particle_runtime_emits_visible_particles() {
        let definition = ClassicParticleEmitter {
            flags: 0,
            position: Vec3::ZERO,
            bone_index: 0,
            blending_type: 2,
            emitter_type: 1,
            rows: 1,
            columns: 1,
            emission_speed: 1.0,
            speed_variation: 0.0,
            vertical_range: 0.0,
            horizontal_range: 0.0,
            gravity: 0.0,
            lifespan: 1.0,
            emission_rate: 100.0,
            area_length: 1.0,
            area_width: 1.0,
            midpoint: 0.5,
            colors: [[1.0; 4]; 3],
            scales: [1.0; 3],
        };
        let mut runtime =
            ParticleEmitterRuntime::new(&ParticleEmitterDefinition::Classic(definition));

        runtime.update(0.05, &[0.0; 16], Mat4::IDENTITY);
        let mut particles = Vec::new();
        runtime.write_render_data(&mut particles);

        assert_eq!(particles.len(), 5);
        assert!(
            particles
                .iter()
                .all(|particle| particle.position.is_finite())
        );
        assert!(
            particles
                .iter()
                .all(|particle| particle.size.cmpgt(Vec2::ZERO).all())
        );
    }

    #[test]
    fn classic_world_space_particles_keep_spawn_transform() {
        let definition = ClassicParticleEmitter {
            flags: 0x10,
            position: Vec3::ZERO,
            bone_index: 0,
            blending_type: 2,
            emitter_type: 2,
            rows: 1,
            columns: 1,
            emission_speed: 0.0,
            speed_variation: 0.0,
            vertical_range: std::f32::consts::PI,
            horizontal_range: 0.0,
            gravity: 0.0,
            lifespan: 1.0,
            emission_rate: 100.0,
            area_length: 1.0,
            area_width: 1.0,
            midpoint: 0.5,
            colors: [[1.0; 4]; 3],
            scales: [1.0; 3],
        };
        let mut runtime = ClassicParticleRuntime {
            definition,
            particles: Vec::new(),
            emission_remainder: 0.0,
            random_state: 1,
        };

        runtime.update(0.1, Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2));
        assert!(
            runtime
                .particles
                .iter()
                .all(|particle| particle.position.y.abs() < 0.0001)
        );
        runtime.update(0.1, Mat4::IDENTITY);
        assert!(
            runtime.particles[..10]
                .iter()
                .all(|particle| particle.position.y.abs() < 0.0001)
        );
        assert!(
            runtime.particles[10..]
                .iter()
                .all(|particle| particle.position.z.abs() < 0.0001)
        );
    }

    #[test]
    fn classic_particle_lighting_uses_the_0x400_flag() {
        let definition = |flags| {
            ParticleEmitterDefinition::Classic(ClassicParticleEmitter {
                flags,
                position: Vec3::ZERO,
                bone_index: 0,
                blending_type: 4,
                emitter_type: 2,
                rows: 1,
                columns: 1,
                emission_speed: 0.0,
                speed_variation: 0.0,
                vertical_range: 0.0,
                horizontal_range: 0.0,
                gravity: 0.0,
                lifespan: 1.0,
                emission_rate: 1.0,
                area_length: 0.0,
                area_width: 0.0,
                midpoint: 0.5,
                colors: [[1.0; 4]; 3],
                scales: [1.0; 3],
            })
        };

        assert!(!definition(0x39).is_lit());
        assert!(definition(0x400).is_lit());
    }

    #[test]
    fn decodes_m2_fixed_point_opacity() {
        assert_eq!(fixed_i16_alpha(&[0, 0]), Some(0.0));
        assert_eq!(fixed_i16_alpha(&i16::MAX.to_le_bytes()), Some(1.0));
        assert!((fixed_i16_alpha(&1638_i16.to_le_bytes()).unwrap() - 0.05).abs() < 0.0001);

        let values = [0_i16, i16::MAX]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(fixed_i16_alpha_values(&values, 0, 2), vec![0.0, 1.0]);
    }

    #[test]
    fn interpolates_animation_tracks() {
        let track = AnimationTrack {
            timestamps: vec![100, 300],
            values: vec![Vec3::ZERO, Vec3::new(4.0, 2.0, 0.0)],
        };

        assert_eq!(track.sample_vec3(0), Vec3::ZERO);
        assert_eq!(track.sample_vec3(200), Vec3::new(2.0, 1.0, 0.0));
        assert_eq!(track.sample_vec3(400), Vec3::new(4.0, 2.0, 0.0));
    }

    #[test]
    fn animates_m2_texture_coordinates() {
        let animation = M2TextureAnimation {
            translation_u: None,
            translation_v: Some(AnimationTrack {
                timestamps: vec![0, 1_000],
                values: vec![0.0, 1.0],
            }),
            rotation: None,
            scale_u: None,
            scale_v: None,
            duration_ms: 1_000,
            start_ms: 0,
        };

        let uv = animation
            .transform(500)
            .transform_point2(Vec2::new(0.25, 0.25));
        assert!(uv.abs_diff_eq(Vec2::new(0.25, 0.75), 0.0001));
    }

    #[test]
    fn animates_m2_material_opacity() {
        let animation = M2OpacityAnimation {
            opacity: AnimationTrack {
                timestamps: vec![0, 1_000],
                values: vec![0.0, 1.0],
            },
            duration_ms: 1_000,
            start_ms: 0,
        };

        assert!((animation.sample(500) - 0.5).abs() < 0.0001);
        assert!(animation.requires_blending());

        let opaque_animation = M2OpacityAnimation {
            opacity: AnimationTrack {
                timestamps: vec![0, 1_000],
                values: vec![1.0, 1.0],
            },
            duration_ms: 1_000,
            start_ms: 0,
        };
        assert!(!opaque_animation.requires_blending());
    }

    #[test]
    fn converts_m2_rotation_to_bevy_basis() {
        let wow_rotation = Quat::from_rotation_x(0.7);
        let bevy_rotation = wow_quat_to_bevy(
            wow_rotation.x,
            wow_rotation.y,
            wow_rotation.z,
            wow_rotation.w,
        );
        let wow_vector = Vec3::new(2.0, 3.0, 5.0);
        let converted_vector =
            Vec3::from_array(wow_to_bevy(wow_vector.x, wow_vector.y, wow_vector.z));
        let rotated_wow_vector = wow_rotation * wow_vector;
        let expected = Vec3::from_array(wow_to_bevy(
            rotated_wow_vector.x,
            rotated_wow_vector.y,
            rotated_wow_vector.z,
        ));

        assert!((bevy_rotation * converted_vector).abs_diff_eq(expected, 0.0001));
    }

    #[test]
    fn decodes_m2_compressed_identity_rotation() {
        assert_eq!(decompress_quat_component(32767), 0.0);
        assert_eq!(decompress_quat_component(-1), 1.0);
        assert_eq!(
            wow_quat_to_bevy(
                decompress_quat_component(32767),
                decompress_quat_component(32767),
                decompress_quat_component(32767),
                decompress_quat_component(-1),
            ),
            Quat::IDENTITY
        );
    }

    #[test]
    fn composes_parent_bone_animation() {
        let animation = M2SkinAnimation {
            vertices: Vec::new(),
            duration_ms: 1000,
            start_ms: 0,
            bones: vec![
                AnimatedBone {
                    parent: -1,
                    pivot: Vec3::ZERO,
                    translation: Some(AnimationTrack {
                        timestamps: vec![0],
                        values: vec![Vec3::X],
                    }),
                    rotation: None,
                    scale: None,
                },
                AnimatedBone {
                    parent: 0,
                    pivot: Vec3::ZERO,
                    translation: Some(AnimationTrack {
                        timestamps: vec![0],
                        values: vec![Vec3::Y],
                    }),
                    rotation: None,
                    scale: None,
                },
            ],
        };

        let transforms = animation.bone_transforms(0);
        assert_eq!(
            transforms[1].transform_point3(Vec3::ZERO),
            Vec3::X + Vec3::Y
        );
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
