use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use wow_adt::{
    McnkChunk, RootAdt,
    chunks::mcnk::{LiquidType, MclqChunk},
};

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

pub(super) struct LiquidMesh {
    pub(super) liquid_type: LiquidType,
    pub(super) mesh: Mesh,
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

pub(super) fn adt_liquids_to_meshes(adt: &RootAdt, center: Vec2) -> Vec<LiquidMesh> {
    let mut vertices = std::array::from_fn::<_, 4, _>(|_| Vec::new());
    let mut uvs = std::array::from_fn::<_, 4, _>(|_| Vec::new());
    let mut indices = std::array::from_fn::<_, 4, _>(|_| Vec::new());

    for chunk in &adt.mcnk_chunks {
        let Some(liquid) = chunk.liquid.as_ref() else {
            continue;
        };
        let liquid_index = liquid_type_index(liquid.liquid_type);
        append_liquid_geometry(
            liquid,
            chunk.header.position,
            center,
            &mut vertices[liquid_index],
            &mut uvs[liquid_index],
            &mut indices[liquid_index],
        );
    }

    [
        LiquidType::Water,
        LiquidType::Ocean,
        LiquidType::Magma,
        LiquidType::Slime,
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(liquid_index, liquid_type)| {
        (!indices[liquid_index].is_empty()).then(|| LiquidMesh {
            liquid_type,
            mesh: build_mesh(
                std::mem::take(&mut vertices[liquid_index]),
                vec![[0.0, 1.0, 0.0]; uvs[liquid_index].len()],
                std::mem::take(&mut uvs[liquid_index]),
                std::mem::take(&mut indices[liquid_index]),
            ),
        })
    })
    .collect()
}

fn append_liquid_geometry(
    liquid: &MclqChunk,
    chunk_position: [f32; 3],
    center: Vec2,
    vertices: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let horizontal_scale = CHUNK_SIZE / 8.0;
    for row in 0..8 {
        for column in 0..8 {
            if !liquid_tile_is_visible(liquid.tile_flags[row * 8 + column]) {
                continue;
            }

            let vertex_offset = vertices.len() as u32;
            for (x, y) in [
                (column, row),
                (column, row + 1),
                (column + 1, row),
                (column + 1, row + 1),
            ] {
                let liquid_vertex = &liquid.vertices[y * 9 + x];
                vertices.push([
                    chunk_position[1] - x as f32 * horizontal_scale - center.x,
                    liquid_vertex.height,
                    chunk_position[0] - y as f32 * horizontal_scale - center.y,
                ]);
                uvs.push(if liquid.liquid_type == LiquidType::Magma {
                    [
                        liquid_vertex.magma_s() as f32 * (3.0 / 256.0),
                        liquid_vertex.magma_t() as f32 * (3.0 / 256.0),
                    ]
                } else {
                    [x as f32, y as f32]
                });
            }
            indices.extend_from_slice(&[
                vertex_offset,
                vertex_offset + 1,
                vertex_offset + 2,
                vertex_offset + 2,
                vertex_offset + 1,
                vertex_offset + 3,
            ]);
        }
    }
}

fn liquid_tile_is_visible(flags: u8) -> bool {
    !matches!(flags & 0x0f, 0x08 | 0x0f)
}

fn liquid_type_index(liquid_type: LiquidType) -> usize {
    liquid_type as usize
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

#[cfg(test)]
mod tests {
    use super::*;
    use wow_adt::chunks::mcnk::LiquidVertex;

    #[test]
    fn liquid_geometry_uses_absolute_heights_and_tile_mask() {
        let mut tile_flags = [0x0f; 64];
        tile_flags[0] = 0x04;
        let liquid = MclqChunk {
            min_height: 12.0,
            max_height: 20.0,
            vertices: (0..81)
                .map(|index| LiquidVertex {
                    union_data: [0; 4],
                    height: index as f32,
                })
                .collect(),
            tile_flags,
            liquid_type: LiquidType::Water,
        };
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        append_liquid_geometry(
            &liquid,
            [100.0, 200.0, 300.0],
            Vec2::new(10.0, 20.0),
            &mut vertices,
            &mut uvs,
            &mut indices,
        );

        assert_eq!(vertices.len(), 4);
        assert_eq!(indices, [0, 1, 2, 2, 1, 3]);
        assert_eq!(vertices[0], [190.0, 0.0, 80.0]);
        assert_eq!(vertices[1][1], 9.0);
        assert_eq!(uvs, [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]]);
    }

    #[test]
    fn classic_no_render_tile_values_are_hidden() {
        assert!(!liquid_tile_is_visible(0x0f));
        assert!(!liquid_tile_is_visible(0x08));
        assert!(liquid_tile_is_visible(0x04));
    }
}
