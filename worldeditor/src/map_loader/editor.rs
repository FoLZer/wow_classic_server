use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
};

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
            .init_resource::<DirtyTerrainMeshes>()
            .add_systems(Update, (draw_selected_chunk_outline, scale_height_points));
    }
}

#[derive(Resource, Default)]
struct DirtyTerrainMeshes(HashSet<Handle<Mesh>>);

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
    pub(super) fn retains_adt(&self, coordinates: (usize, usize)) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|selected| selected.coordinates == coordinates)
            || self.edited_adts.contains_key(&coordinates)
    }
}

struct SelectedTerrainChunk {
    adt: RootAdt,
    coordinates: (usize, usize),
    chunk_index: usize,
    edit_mesh: Handle<Mesh>,
    point_entities: Vec<Entity>,
    seam_links: HashMap<usize, Vec<SeamVertex>>,
}

#[derive(Clone, Copy)]
struct SeamVertex {
    coordinates: (usize, usize),
    chunk_index: usize,
    vertex_index: usize,
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
    mpqs: Res<MPQResource>,
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
        let Ok(map_file_buf) = mpqs.mpqs.read_file_concurrent(&map_path) else {
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

    if let Some(loaded_adt) = terrain.loaded_adts.get_mut(&coordinates) {
        if loaded_adt.mesh != edit_mesh {
            meshes.remove(loaded_adt.mesh.id());
            loaded_adt.mesh = edit_mesh.clone();
        }
    }

    let neighbor_coordinates = neighboring_adts_for_chunk(&adt, coordinates, chunk_index);
    for neighbor_coordinates in neighbor_coordinates {
        ensure_editable_neighbor(
            neighbor_coordinates,
            &mut editor,
            &mut terrain,
            &mpqs.mpqs,
            &mut adt_entities,
            &mut meshes,
        );
    }
    let seam_links = build_seam_links(&adt, coordinates, chunk_index, &editor.edited_adts);

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
                .observe(finish_height_drag)
                .id()
        })
        .collect();

    editor.selected = Some(SelectedTerrainChunk {
        adt,
        coordinates,
        chunk_index,
        edit_mesh,
        point_entities,
        seam_links,
    });
}

fn neighboring_adts_for_chunk(
    adt: &RootAdt,
    coordinates: (usize, usize),
    chunk_index: usize,
) -> HashSet<(usize, usize)> {
    let mut neighbors = HashSet::new();
    let chunk = &adt.mcnk_chunks[chunk_index];
    for vertex_index in 0..EDIT_HEIGHTMAP_SIZE {
        if !is_chunk_edge_vertex(vertex_index) {
            continue;
        }
        for adjacent in
            adjacent_adts_at_position(coordinates, heightmap_point_world(chunk, vertex_index).xz())
        {
            if adjacent != coordinates {
                neighbors.insert(adjacent);
            }
        }
    }
    neighbors
}

fn ensure_editable_neighbor(
    coordinates: (usize, usize),
    editor: &mut TerrainEditor,
    terrain: &mut TerrainMap,
    mpqs: &wow_mpq::PatchChain,
    adt_entities: &mut Query<(&TerrainAdt, &mut Mesh3d)>,
    meshes: &mut Assets<Mesh>,
) {
    if editor.edited_adts.contains_key(&coordinates) {
        return;
    }
    let map_name = terrain.map_name.clone();
    let Some(loaded_adt) = terrain.loaded_adts.get_mut(&coordinates) else {
        return;
    };
    let map_path = format!(
        "World\\Maps\\{}\\{}_{}_{}.adt",
        map_name, map_name, coordinates.0, coordinates.1
    );
    let Ok(map_file_buf) = mpqs.read_file_concurrent(&map_path) else {
        return;
    };
    let Ok(ParsedAdt::Root(adt)) = parse_adt(&mut Cursor::new(map_file_buf)) else {
        return;
    };
    let adt = *adt;
    let edit_mesh = meshes.add(adt_to_edit_mesh(
        &adt,
        adt_center(coordinates.0, coordinates.1),
    ));
    meshes.remove(loaded_adt.mesh.id());
    loaded_adt.mesh = edit_mesh.clone();
    if let Ok((_, mut mesh)) = adt_entities.get_mut(loaded_adt.entity) {
        mesh.0 = edit_mesh;
    }
    editor.edited_adts.insert(coordinates, adt);
}

fn build_seam_links(
    selected_adt: &RootAdt,
    selected_coordinates: (usize, usize),
    selected_chunk_index: usize,
    edited_adts: &HashMap<(usize, usize), RootAdt>,
) -> HashMap<usize, Vec<SeamVertex>> {
    let mut links = HashMap::new();
    for vertex_index in 0..EDIT_HEIGHTMAP_SIZE {
        if !is_chunk_edge_vertex(vertex_index) {
            continue;
        }
        let seam_position = heightmap_point_world(
            &selected_adt.mcnk_chunks[selected_chunk_index],
            vertex_index,
        )
        .xz();
        let mut point_links = matching_seam_vertices(selected_adt, seam_position)
            .into_iter()
            .map(|(linked_chunk, linked_vertex)| SeamVertex {
                coordinates: selected_coordinates,
                chunk_index: linked_chunk,
                vertex_index: linked_vertex,
            })
            .collect::<Vec<_>>();
        for coordinates in adjacent_adts_at_position(selected_coordinates, seam_position) {
            if coordinates == selected_coordinates {
                continue;
            }
            let Some(adt) = edited_adts.get(&coordinates) else {
                continue;
            };
            point_links.extend(matching_seam_vertices(adt, seam_position).into_iter().map(
                |(linked_chunk, linked_vertex)| SeamVertex {
                    coordinates,
                    chunk_index: linked_chunk,
                    vertex_index: linked_vertex,
                },
            ));
        }
        links.insert(vertex_index, point_links);
    }
    links
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
    keyboard: Res<ButtonInput<KeyCode>>,
    terrain: Res<TerrainMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut dirty_meshes: ResMut<DirtyTerrainMeshes>,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    drag.propagate(false);
    let Ok((point, mut point_transform)) = points.get_mut(drag.entity) else {
        return;
    };
    let Some(selected) = editor.selected.as_ref() else {
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

    let selected_coordinates = selected.coordinates;
    let selected_mesh = selected.edit_mesh.clone();
    let snapping_disabled = keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let target_world_height = point_transform.translation.y + height_delta;
    let links = if snapping_disabled || !is_chunk_edge_vertex(point.vertex_index) {
        vec![SeamVertex {
            coordinates: selected_coordinates,
            chunk_index: point.chunk_index,
            vertex_index: point.vertex_index,
        }]
    } else {
        selected
            .seam_links
            .get(&point.vertex_index)
            .cloned()
            .unwrap_or_else(|| {
                vec![SeamVertex {
                    coordinates: selected_coordinates,
                    chunk_index: point.chunk_index,
                    vertex_index: point.vertex_index,
                }]
            })
    };

    let mut linked_vertices: HashMap<(usize, usize), Vec<(usize, usize)>> = HashMap::new();
    for link in links {
        linked_vertices
            .entry(link.coordinates)
            .or_default()
            .push((link.chunk_index, link.vertex_index));
    }

    if let Some(selected_vertices) = linked_vertices.remove(&selected_coordinates)
        && let Some(selected) = editor.selected.as_mut()
    {
        apply_world_height(
            &mut selected.adt,
            &selected_mesh,
            &selected_vertices,
            target_world_height,
            &mut meshes,
        );
        dirty_meshes.0.insert(selected_mesh);
    }
    point_transform.translation.y = target_world_height;

    for (coordinates, vertices) in linked_vertices {
        let Some(adt) = editor.edited_adts.get_mut(&coordinates) else {
            continue;
        };
        let Some(mesh) = terrain
            .loaded_adts
            .get(&coordinates)
            .map(|loaded_adt| loaded_adt.mesh.clone())
        else {
            continue;
        };
        apply_world_height(adt, &mesh, &vertices, target_world_height, &mut meshes);
        dirty_meshes.0.insert(mesh);
    }
}

fn finish_height_drag(
    drag_end: On<Pointer<DragEnd>>,
    mut dirty_meshes: ResMut<DirtyTerrainMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if drag_end.button != PointerButton::Primary {
        return;
    }
    for mesh_handle in dirty_meshes.0.drain() {
        if let Some(mut mesh) = meshes.get_mut(mesh_handle.id()) {
            mesh.compute_area_weighted_normals();
        }
    }
}

fn is_chunk_edge_vertex(vertex_index: usize) -> bool {
    if vertex_index >= 8 * 17 {
        return true;
    }
    let row = vertex_index / 17;
    let column = vertex_index % 17;
    column < 9 && (row == 0 || column == 0 || column == 8)
}

fn matching_seam_vertices(adt: &RootAdt, seam_position: Vec2) -> Vec<(usize, usize)> {
    const POSITION_EPSILON_SQUARED: f32 = 0.01 * 0.01;
    adt.mcnk_chunks
        .iter()
        .enumerate()
        .flat_map(|(chunk_index, chunk)| {
            (0..EDIT_HEIGHTMAP_SIZE).filter_map(move |vertex_index| {
                (is_chunk_edge_vertex(vertex_index)
                    && heightmap_point_world(chunk, vertex_index)
                        .xz()
                        .distance_squared(seam_position)
                        <= POSITION_EPSILON_SQUARED)
                    .then_some((chunk_index, vertex_index))
            })
        })
        .collect()
}

fn adjacent_adts_at_position(
    selected_coordinates: (usize, usize),
    seam_position: Vec2,
) -> Vec<(usize, usize)> {
    const ADT_SIZE: f32 = CHUNK_SIZE * 16.0;
    const EDGE_EPSILON: f32 = 0.01;
    let mut coordinates = Vec::with_capacity(4);
    for x in selected_coordinates.0.saturating_sub(1)..=(selected_coordinates.0 + 1).min(63) {
        for y in selected_coordinates.1.saturating_sub(1)..=(selected_coordinates.1 + 1).min(63) {
            let center = adt_center(x, y);
            if (seam_position.x - center.x).abs() <= ADT_SIZE * 0.5 + EDGE_EPSILON
                && (seam_position.y - center.y).abs() <= ADT_SIZE * 0.5 + EDGE_EPSILON
            {
                coordinates.push((x, y));
            }
        }
    }
    coordinates
}

fn apply_world_height(
    adt: &mut RootAdt,
    mesh_handle: &Handle<Mesh>,
    vertices: &[(usize, usize)],
    world_height: f32,
    meshes: &mut Assets<Mesh>,
) {
    for &(chunk_index, vertex_index) in vertices {
        let chunk = &mut adt.mcnk_chunks[chunk_index];
        chunk.heights.as_mut().unwrap().heights[vertex_index] =
            world_height - chunk.header.position[2];
    }

    let Some(mut mesh) = meshes.get_mut(mesh_handle.id()) else {
        return;
    };
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for &(chunk_index, vertex_index) in vertices {
            positions[chunk_index * EDIT_HEIGHTMAP_SIZE + vertex_index][1] = world_height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_outer_heightmap_vertices_are_seam_points() {
        for index in 0..9 {
            assert!(is_chunk_edge_vertex(index));
        }
        for index in 8 * 17..EDIT_HEIGHTMAP_SIZE {
            assert!(is_chunk_edge_vertex(index));
        }
        assert!(is_chunk_edge_vertex(17));
        assert!(is_chunk_edge_vertex(25));
        assert!(!is_chunk_edge_vertex(9));
        assert!(!is_chunk_edge_vertex(21));
        assert!(!is_chunk_edge_vertex(135));
    }

    #[test]
    fn adt_boundary_positions_include_both_tiles() {
        let selected = (32, 32);
        let selected_center = adt_center(selected.0, selected.1);
        let seam = selected_center + Vec2::new(CHUNK_SIZE * 8.0, 0.0);
        let adjacent = adjacent_adts_at_position(selected, seam);

        assert!(adjacent.contains(&selected));
        assert!(adjacent.contains(&(31, 32)));
        assert_eq!(adjacent.len(), 2);
    }

    #[test]
    fn adt_corner_positions_include_four_tiles() {
        let selected = (32, 32);
        let selected_center = adt_center(selected.0, selected.1);
        let corner = selected_center + Vec2::splat(CHUNK_SIZE * 8.0);
        let adjacent = adjacent_adts_at_position(selected, corner);

        assert_eq!(adjacent.len(), 4);
        assert!(adjacent.contains(&(31, 31)));
        assert!(adjacent.contains(&(31, 32)));
        assert!(adjacent.contains(&(32, 31)));
        assert!(adjacent.contains(&selected));
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
