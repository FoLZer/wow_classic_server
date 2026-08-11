use std::{collections::HashMap, io::Cursor};

use bevy::{
    color::palettes::css::{ORANGE, YELLOW},
    mesh::VertexAttributeValues,
    prelude::*,
};
use wow_adt::{ParsedAdt, RootAdt, parse_adt};

use crate::MPQResource;

use super::{
    CHUNK_SIZE, TerrainAdt, TerrainMap,
    geometry::{EDIT_HEIGHTMAP_SIZE, adt_center, adt_to_edit_mesh, heightmap_point_world},
};

pub struct TerrainEditorPlugin;

impl Plugin for TerrainEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainEditor>()
            .add_systems(Update, (draw_selected_chunk_outline, scale_height_points));
    }
}

#[derive(Resource, Default)]
pub(crate) struct TerrainEditor {
    selected: Option<SelectedTerrainChunk>,
    edited_adts: HashMap<(usize, usize), RootAdt>,
    point_mesh: Option<Handle<Mesh>>,
    point_material: Option<Handle<StandardMaterial>>,
    active_point_material: Option<Handle<StandardMaterial>>,
    active_point: Option<Entity>,
}

impl TerrainEditor {
    pub(super) fn selected_coordinates(&self) -> Option<(usize, usize)> {
        self.selected.as_ref().map(|selected| selected.coordinates)
    }
}

struct SelectedTerrainChunk {
    adt: RootAdt,
    coordinates: (usize, usize),
    chunk_index: usize,
    edit_mesh: Handle<Mesh>,
    point_entities: Vec<Entity>,
}

#[derive(Component)]
struct HeightMapPoint {
    chunk_index: usize,
    vertex_index: usize,
}

pub(super) fn select_adt_chunk(
    mut click: On<Pointer<Click>>,
    mut commands: Commands,
    mut adt_entities: Query<(&TerrainAdt, &mut Mesh3d)>,
    mut editor: ResMut<TerrainEditor>,
    mut terrain: ResMut<TerrainMap>,
    mut mpqs: ResMut<MPQResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if click.button != PointerButton::Primary || click.count != 2 {
        return;
    }
    let Some(hit_position) = click.hit.position else {
        return;
    };
    click.propagate(false);

    if let Some(previous) = editor.selected.take() {
        for point_entity in previous.point_entities {
            commands.entity(point_entity).despawn();
        }
        if let Some(loaded_adt) = terrain.loaded_adts.get_mut(&previous.coordinates) {
            if loaded_adt.mesh != previous.edit_mesh {
                meshes.remove(loaded_adt.mesh.id());
                loaded_adt.mesh = previous.edit_mesh.clone();
            }
        }
        editor
            .edited_adts
            .insert(previous.coordinates, previous.adt);
        editor.active_point = None;
    }

    let Ok((adt_coordinates, mut entity_mesh)) = adt_entities.get_mut(click.entity) else {
        return;
    };
    let coordinates = (adt_coordinates.x as usize, adt_coordinates.y as usize);
    if !terrain.loaded_adts.contains_key(&coordinates) {
        return;
    }
    let adt = if let Some(adt) = editor.edited_adts.remove(&coordinates) {
        adt
    } else {
        let map_path = format!(
            "World\\Maps\\{}\\{}_{}_{}.adt",
            terrain.map_name, terrain.map_name, coordinates.0, coordinates.1
        );
        let Ok(map_file_buf) = mpqs.mpqs.read_file(&map_path) else {
            return;
        };
        let Ok(ParsedAdt::Root(adt)) = parse_adt(&mut Cursor::new(map_file_buf)) else {
            return;
        };
        *adt
    };
    let edit_mesh = meshes.add(adt_to_edit_mesh(
        &adt,
        adt_center(coordinates.0, coordinates.1),
    ));
    entity_mesh.0 = edit_mesh.clone();
    let chunk_index = adt
        .mcnk_chunks
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            let left_center = Vec2::new(
                left.header.position[1] - CHUNK_SIZE * 0.5,
                left.header.position[0] - CHUNK_SIZE * 0.5,
            );
            let right_center = Vec2::new(
                right.header.position[1] - CHUNK_SIZE * 0.5,
                right.header.position[0] - CHUNK_SIZE * 0.5,
            );
            left_center
                .distance_squared(hit_position.xz())
                .total_cmp(&right_center.distance_squared(hit_position.xz()))
        })
        .map(|(index, _)| index)
        .unwrap();

    let point_mesh = editor
        .point_mesh
        .get_or_insert_with(|| meshes.add(Sphere::new(1.0)))
        .clone();
    let point_material = editor
        .point_material
        .get_or_insert_with(|| {
            materials.add(StandardMaterial {
                base_color: YELLOW.into(),
                unlit: true,
                ..Default::default()
            })
        })
        .clone();
    editor.active_point_material.get_or_insert_with(|| {
        materials.add(StandardMaterial {
            base_color: ORANGE.into(),
            unlit: true,
            ..Default::default()
        })
    });

    let point_entities = (0..EDIT_HEIGHTMAP_SIZE)
        .map(|vertex_index| {
            commands
                .spawn((
                    HeightMapPoint {
                        chunk_index,
                        vertex_index,
                    },
                    Mesh3d(point_mesh.clone()),
                    MeshMaterial3d(point_material.clone()),
                    Transform::from_translation(heightmap_point_world(
                        &adt.mcnk_chunks[chunk_index],
                        vertex_index,
                    )),
                ))
                .observe(select_height_point)
                .observe(drag_height_point)
                .id()
        })
        .collect();

    editor.selected = Some(SelectedTerrainChunk {
        adt,
        coordinates,
        chunk_index,
        edit_mesh,
        point_entities,
    });
}

fn select_height_point(
    mut press: On<Pointer<Press>>,
    mut editor: ResMut<TerrainEditor>,
    mut points: Query<&mut MeshMaterial3d<StandardMaterial>, With<HeightMapPoint>>,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    press.propagate(false);
    if let Some(previous) = editor.active_point
        && let Ok(mut material) = points.get_mut(previous)
        && let Some(point_material) = &editor.point_material
    {
        material.0 = point_material.clone();
    }
    if let Ok(mut material) = points.get_mut(press.entity)
        && let Some(active_material) = &editor.active_point_material
    {
        material.0 = active_material.clone();
    }
    editor.active_point = Some(press.entity);
}

fn drag_height_point(
    mut drag: On<Pointer<Drag>>,
    mut editor: ResMut<TerrainEditor>,
    mut points: Query<(&HeightMapPoint, &mut Transform)>,
    camera: Query<(&Camera, &GlobalTransform, &Projection), With<Camera3d>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    drag.propagate(false);
    let Ok((point, mut point_transform)) = points.get_mut(drag.entity) else {
        return;
    };
    let Some(selected) = editor.selected.as_mut() else {
        return;
    };
    if point.chunk_index != selected.chunk_index {
        return;
    }
    let Ok((camera, camera_transform, projection)) = camera.single() else {
        return;
    };
    let viewport_height = camera.logical_viewport_size().map_or(1080.0, |size| size.y);
    let distance = camera_transform
        .translation()
        .distance(point_transform.translation);
    let world_units_per_pixel = match projection {
        Projection::Perspective(perspective) => {
            2.0 * distance * (perspective.fov * 0.5).tan() / viewport_height
        }
        Projection::Orthographic(orthographic) => orthographic.area.height() / viewport_height,
        _ => distance / viewport_height,
    };
    let height_delta = (-drag.delta.y * world_units_per_pixel).clamp(-CHUNK_SIZE, CHUNK_SIZE);

    let chunk = &mut selected.adt.mcnk_chunks[point.chunk_index];
    let heights = chunk.heights.as_mut().unwrap();
    heights.heights[point.vertex_index] += height_delta;
    point_transform.translation.y = chunk.header.position[2] + heights.heights[point.vertex_index];

    if let Some(mut mesh) = meshes.get_mut(selected.edit_mesh.id()) {
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            positions[point.chunk_index * EDIT_HEIGHTMAP_SIZE + point.vertex_index][1] =
                point_transform.translation.y;
        }
        mesh.compute_area_weighted_normals();
    }
}

fn draw_selected_chunk_outline(editor: Res<TerrainEditor>, mut gizmos: Gizmos) {
    let Some(selected) = &editor.selected else {
        return;
    };
    let chunk = &selected.adt.mcnk_chunks[selected.chunk_index];
    let mut boundary = Vec::with_capacity(33);
    boundary.extend(0..9);
    boundary.extend((1..=8).map(|y| y * 17 + 8));
    boundary.extend((0..8).rev().map(|x| 8 * 17 + x));
    boundary.extend((1..8).rev().map(|y| y * 17));
    boundary.push(0);

    for edge in boundary.windows(2) {
        gizmos.line(
            heightmap_point_world(chunk, edge[0]) + Vec3::Y * 0.15,
            heightmap_point_world(chunk, edge[1]) + Vec3::Y * 0.15,
            ORANGE,
        );
    }
}

fn scale_height_points(
    camera: Query<(&Camera, &GlobalTransform, &Projection), With<Camera3d>>,
    mut points: Query<&mut Transform, With<HeightMapPoint>>,
) {
    let Ok((camera, camera_transform, projection)) = camera.single() else {
        return;
    };
    let viewport_height = camera.logical_viewport_size().map_or(1080.0, |size| size.y);
    for mut transform in &mut points {
        let distance = camera_transform
            .translation()
            .distance(transform.translation);
        let world_units_per_pixel = match projection {
            Projection::Perspective(perspective) => {
                2.0 * distance * (perspective.fov * 0.5).tan() / viewport_height
            }
            Projection::Orthographic(orthographic) => orthographic.area.height() / viewport_height,
            _ => distance / viewport_height,
        };
        transform.scale = Vec3::splat((world_units_per_pixel * 4.0).clamp(0.08, 20.0));
    }
}
