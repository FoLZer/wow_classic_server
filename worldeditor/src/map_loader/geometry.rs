use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use wow_adt::{McnkChunk, RootAdt};

use super::{ADT_CELLS_PER_GRID, CHUNK_SIZE};

pub(super) const EDIT_HEIGHTMAP_SIZE: usize = 145;
const EDIT_INDICES_PER_CHUNK: usize = 8 * 8 * 4 * 3;

struct TerrainChunkGeometry {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u16>,
    position: Vec3,
}

pub(super) fn adt_to_mesh(adt: &RootAdt, center: Vec2) -> Mesh {
    let chunk_count = ADT_CELLS_PER_GRID * ADT_CELLS_PER_GRID;
    let mut vertices = Vec::with_capacity(chunk_count * EDIT_HEIGHTMAP_SIZE);
    let mut normals = Vec::with_capacity(chunk_count * EDIT_HEIGHTMAP_SIZE);
    let mut uvs = Vec::with_capacity(chunk_count * EDIT_HEIGHTMAP_SIZE);
    let mut indices = Vec::with_capacity(chunk_count * EDIT_INDICES_PER_CHUNK);
    let horizontal_scale = CHUNK_SIZE / 8.0;

    for chunk_x in 0..ADT_CELLS_PER_GRID {
        for chunk_y in 0..ADT_CELLS_PER_GRID {
            let geometry =
                generate_chunk_geometry(&adt.mcnk_chunks[chunk_x * ADT_CELLS_PER_GRID + chunk_y]);
            append_chunk_geometry(
                geometry,
                center,
                chunk_x,
                chunk_y,
                horizontal_scale,
                &mut vertices,
                &mut normals,
                &mut uvs,
                &mut indices,
            );
        }
    }

    build_mesh(vertices, normals, uvs, indices)
}

fn append_chunk_geometry(
    geometry: TerrainChunkGeometry,
    center: Vec2,
    chunk_x: usize,
    chunk_y: usize,
    horizontal_scale: f32,
    vertices: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let vertex_offset = vertices.len() as u32;
    vertices.extend(geometry.vertices.into_iter().map(|vertex| {
        [
            geometry.position.x - vertex[0] * horizontal_scale - center.x,
            geometry.position.y + vertex[1],
            geometry.position.z - vertex[2] * horizontal_scale - center.y,
        ]
    }));
    normals.extend(geometry.normals.into_iter().map(|normal| {
        Vec3::new(
            -normal[0] / horizontal_scale,
            normal[1],
            -normal[2] / horizontal_scale,
        )
        .normalize_or_zero()
        .to_array()
    }));
    uvs.extend(
        geometry
            .uvs
            .into_iter()
            .map(|uv| [chunk_y as f32 * 8.0 + uv[0], chunk_x as f32 * 8.0 + uv[1]]),
    );
    indices.extend(
        geometry
            .indices
            .into_iter()
            .map(|index| vertex_offset + index as u32),
    );
}

fn build_mesh(
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
}

fn generate_chunk_geometry(chunk: &McnkChunk) -> TerrainChunkGeometry {
    let heights = chunk.heights.as_ref().unwrap();
    let normals_chunk = chunk.normals.as_ref().unwrap();
    let mut vertices = vec![[0.0; 3]; EDIT_HEIGHTMAP_SIZE];
    let mut normals = vec![[0.0; 3]; EDIT_HEIGHTMAP_SIZE];
    let mut uvs = vec![[0.0; 2]; EDIT_HEIGHTMAP_SIZE];
    let mut indices = Vec::with_capacity(EDIT_INDICES_PER_CHUNK);

    for y in 0..8 {
        for x in 0..9 {
            let index = y * 17 + x;
            vertices[index] = [x as f32, heights.heights[index], y as f32];
            normals[index] = chunk_normal(normals_chunk, index);
            uvs[index] = [x as f32, y as f32];
        }

        for x in 0..8 {
            if chunk.header.is_hole_low_res(x / 2, y / 2) {
                continue;
            }

            let index = y * 17 + 9 + x;
            vertices[index] = [x as f32 + 0.5, heights.heights[index], y as f32 + 0.5];
            normals[index] = chunk_normal(normals_chunk, index);
            uvs[index] = [x as f32 + 0.5, y as f32 + 0.5];

            let top_left = (index - 9) as u16;
            let top_right = (index - 8) as u16;
            let bottom_left = (index + 8) as u16;
            let bottom_right = (index + 9) as u16;
            indices.extend_from_slice(&[
                top_left,
                index as u16,
                top_right,
                index as u16,
                bottom_right,
                top_right,
                bottom_right,
                index as u16,
                bottom_left,
                index as u16,
                top_left,
                bottom_left,
            ]);
        }
    }

    for x in 0..9 {
        let index = 8 * 17 + x;
        vertices[index] = [x as f32, heights.heights[index], 8.0];
        normals[index] = chunk_normal(normals_chunk, index);
        uvs[index] = [x as f32, 8.0];
    }

    TerrainChunkGeometry {
        vertices,
        normals,
        uvs,
        indices,
        position: Vec3::new(
            chunk.header.position[1],
            chunk.header.position[2],
            chunk.header.position[0],
        ),
    }
}

fn chunk_normal(normals: &wow_adt::chunks::McnrChunk, index: usize) -> [f32; 3] {
    [
        -normals.normals[index].z as f32 / 127.0,
        normals.normals[index].y as f32 / 127.0,
        -normals.normals[index].x as f32 / 127.0,
    ]
}

pub(super) fn heightmap_point_world(chunk: &McnkChunk, vertex_index: usize) -> Vec3 {
    let (x, z) = if vertex_index >= 8 * 17 {
        ((vertex_index - 8 * 17) as f32, 8.0)
    } else {
        let row = vertex_index / 17;
        let column = vertex_index % 17;
        if column < 9 {
            (column as f32, row as f32)
        } else {
            ((column - 9) as f32 + 0.5, row as f32 + 0.5)
        }
    };
    Vec3::new(
        chunk.header.position[1] - x * CHUNK_SIZE / 8.0,
        chunk.header.position[2] + chunk.heights.as_ref().unwrap().heights[vertex_index],
        chunk.header.position[0] - z * CHUNK_SIZE / 8.0,
    )
}
