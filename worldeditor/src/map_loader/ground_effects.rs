use std::{collections::HashMap, io::Cursor};

use bevy::prelude::*;
use wow_adt::RootAdt;
use wow_mpq::PatchChain;

use super::CHUNK_SIZE;

const DETAIL_CELLS_PER_CHUNK: usize = 8;
const MIN_DOODADS_PER_CHUNK: usize = 8;
const MAX_DOODADS_PER_CHUNK: usize = 24;

#[derive(Clone)]
pub(super) struct GroundEffectPlacement {
    pub(super) filename: String,
    pub(super) transform: Transform,
}

#[derive(Clone)]
struct GroundEffect {
    models: [Option<String>; 4],
    density: usize,
}

#[derive(Clone, Default)]
pub(super) struct GroundEffectData {
    effects: HashMap<u32, GroundEffect>,
}

#[derive(Clone)]
pub(super) struct GroundEffectSource {
    chunks: Vec<GroundEffectChunk>,
    seed: u32,
}

#[derive(Clone)]
struct GroundEffectChunk {
    position: [f32; 3],
    heights: Vec<f32>,
    effect_ids: Vec<u32>,
    predominant_texture: [u8; 16],
    no_effect_doodad: [u8; 8],
    holes: [bool; 64],
}

impl GroundEffectData {
    pub(super) fn load(mpqs: &PatchChain) -> Self {
        let Ok(doodad_data) = mpqs.read_file_concurrent("DBFilesClient\\GroundEffectDoodad.dbc")
        else {
            warn!("GroundEffectDoodad.dbc was not found; terrain foliage is disabled");
            return Self::default();
        };
        let Ok(texture_data) = mpqs.read_file_concurrent("DBFilesClient\\GroundEffectTexture.dbc")
        else {
            warn!("GroundEffectTexture.dbc was not found; terrain foliage is disabled");
            return Self::default();
        };
        let Ok(doodads) = dbc_reader::read_dbc::<_, dbc_structs::GroundEffectDoodad>(
            &mut Cursor::new(doodad_data),
        ) else {
            warn!("GroundEffectDoodad.dbc could not be parsed; terrain foliage is disabled");
            return Self::default();
        };
        let Ok(textures) = dbc_reader::read_dbc::<_, dbc_structs::GroundEffectTexture>(
            &mut Cursor::new(texture_data),
        ) else {
            warn!("GroundEffectTexture.dbc could not be parsed; terrain foliage is disabled");
            return Self::default();
        };

        let mut model_by_tag = HashMap::new();
        for doodad in doodads.get_records().iter() {
            let Ok(path) = doodad.doodadpath.to_str() else {
                continue;
            };
            if !path.is_empty() {
                let path = ground_effect_model_path(path);
                model_by_tag.insert(doodad.doodad_id_tag, path);
            }
        }

        let effects: HashMap<u32, GroundEffect> = textures
            .get_records()
            .iter()
            .filter_map(|texture| {
                let models = std::array::from_fn(|index| {
                    model_by_tag.get(&texture.doodad_id[index]).cloned()
                });
                (models.iter().any(Option::is_some) && texture.id >= 0).then_some((
                    texture.id as u32,
                    GroundEffect {
                        models,
                        density: texture.density.max(0) as usize,
                    },
                ))
            })
            .collect();

        info!("Loaded {} terrain ground effects", effects.len());
        Self { effects }
    }

    pub(super) fn source(&self, adt: &RootAdt, seed: u32) -> GroundEffectSource {
        let chunks = adt
            .mcnk_chunks
            .iter()
            .filter_map(|chunk| {
                let heights = chunk.heights.as_ref()?.heights.clone();
                let effect_ids = chunk
                    .layers
                    .as_ref()?
                    .layers
                    .iter()
                    .map(|layer| layer.effect_id)
                    .collect();
                let mut predominant_texture = [0; 16];
                predominant_texture[..8].copy_from_slice(&chunk.header.pred_tex);
                predominant_texture[8..].copy_from_slice(&chunk.header.no_effect_doodad);
                let holes = std::array::from_fn(|index| {
                    let x = index % DETAIL_CELLS_PER_CHUNK;
                    let z = index / DETAIL_CELLS_PER_CHUNK;
                    chunk.header.is_hole_low_res(x / 2, z / 2)
                });
                Some(GroundEffectChunk {
                    position: chunk.header.position,
                    heights,
                    effect_ids,
                    predominant_texture,
                    no_effect_doodad: chunk.header.unknown_8bytes,
                    holes,
                })
            })
            .collect();
        GroundEffectSource { chunks, seed }
    }

    pub(super) fn placements_near(
        &self,
        source: &GroundEffectSource,
        camera_position: Vec2,
        distance: f32,
    ) -> Vec<GroundEffectPlacement> {
        source
            .chunks
            .iter()
            .enumerate()
            .flat_map(|(chunk_index, chunk)| {
                self.chunk_placements(
                    chunk,
                    source.seed ^ chunk_index as u32,
                    camera_position,
                    distance,
                )
            })
            .collect()
    }

    fn chunk_placements(
        &self,
        chunk: &GroundEffectChunk,
        seed: u32,
        camera_position: Vec2,
        distance: f32,
    ) -> Vec<GroundEffectPlacement> {
        let mut placements = Vec::new();
        for cell_z in 0..DETAIL_CELLS_PER_CHUNK {
            for cell_x in 0..DETAIL_CELLS_PER_CHUNK {
                if classic_no_effect_doodad(chunk, cell_x, cell_z)
                    || chunk.holes[cell_z * DETAIL_CELLS_PER_CHUNK + cell_x]
                {
                    continue;
                }
                let layer_index = classic_predominant_texture(chunk, cell_x, cell_z) as usize;
                let Some(effect) = chunk
                    .effect_ids
                    .get(layer_index)
                    .and_then(|effect_id| self.effects.get(effect_id))
                else {
                    continue;
                };
                let count = classic_density(effect.density);
                for instance in 0..count {
                    let cell_index = cell_z * DETAIL_CELLS_PER_CHUNK + cell_x;
                    let hash = mix_hash(seed ^ cell_index as u32, instance as u32);
                    let local_x = (cell_x as f32 + hash_unit(hash)) * CHUNK_SIZE
                        / DETAIL_CELLS_PER_CHUNK as f32;
                    let local_z = (cell_z as f32 + hash_unit(mix_hash(hash, 1))) * CHUNK_SIZE
                        / DETAIL_CELLS_PER_CHUNK as f32;
                    let position =
                        Vec2::new(chunk.position[1] - local_x, chunk.position[0] - local_z);
                    if position.distance_squared(camera_position) > distance.powi(2) {
                        continue;
                    }
                    let Some(filename) = ground_effect_model(effect, hash).map(str::to_owned)
                    else {
                        continue;
                    };
                    let height = terrain_height(chunk, local_x, local_z);
                    let yaw = hash_unit(mix_hash(hash, 2)) * std::f32::consts::TAU;
                    placements.push(GroundEffectPlacement {
                        filename,
                        transform: Transform {
                            translation: Vec3::new(
                                position.x,
                                chunk.position[2] + height,
                                position.y,
                            ),
                            rotation: Quat::from_rotation_y(yaw + std::f32::consts::PI),
                            scale: Vec3::ONE,
                        },
                    });
                }
            }
        }
        placements
    }
}

fn ground_effect_model(effect: &GroundEffect, hash: u32) -> Option<&str> {
    effect.models[hash as usize % effect.models.len()].as_deref()
}

fn classic_predominant_texture(chunk: &GroundEffectChunk, x: usize, z: usize) -> u8 {
    decode_classic_predominant_texture(&chunk.predominant_texture, x, z)
}

fn decode_classic_predominant_texture(map: &[u8; 16], x: usize, z: usize) -> u8 {
    let byte_index = z * 2 + x / 4;
    let byte = map[byte_index];
    (byte >> ((3 - x % 4) * 2)) & 0x3
}

fn classic_no_effect_doodad(chunk: &GroundEffectChunk, x: usize, z: usize) -> bool {
    decode_classic_no_effect_doodad(&chunk.no_effect_doodad, x, z)
}

fn decode_classic_no_effect_doodad(bitmap: &[u8; 8], x: usize, z: usize) -> bool {
    (bitmap[z] >> x) & 1 != 0
}

fn classic_density(density: usize) -> usize {
    density.clamp(MIN_DOODADS_PER_CHUNK, MAX_DOODADS_PER_CHUNK)
}

fn terrain_height(chunk: &GroundEffectChunk, local_x: f32, local_z: f32) -> f32 {
    let grid_x = (local_x / CHUNK_SIZE * 8.0).clamp(0.0, 7.999);
    let grid_z = (local_z / CHUNK_SIZE * 8.0).clamp(0.0, 7.999);
    let x = grid_x.floor() as usize;
    let z = grid_z.floor() as usize;
    let tx = grid_x.fract();
    let tz = grid_z.fract();
    let heights = &chunk.heights;
    let top_left = heights[z * 17 + x];
    let top_right = heights[z * 17 + x + 1];
    let bottom_left = heights[(z + 1) * 17 + x];
    let bottom_right = heights[(z + 1) * 17 + x + 1];
    let center = heights[z * 17 + 9 + x];
    interpolate_cell_height(
        [top_left, top_right, bottom_left, bottom_right, center],
        tx,
        tz,
    )
}

fn interpolate_cell_height(heights: [f32; 5], x: f32, z: f32) -> f32 {
    let [top_left, top_right, bottom_left, bottom_right, center] = heights;
    if z <= x && x + z <= 1.0 {
        (1.0 - x - z) * top_left + (x - z) * top_right + 2.0 * z * center
    } else if z <= x {
        (x - z) * top_right + (x + z - 1.0) * bottom_right + 2.0 * (1.0 - x) * center
    } else if x + z >= 1.0 {
        (z - x) * bottom_left + (x + z - 1.0) * bottom_right + 2.0 * (1.0 - z) * center
    } else {
        (1.0 - x - z) * top_left + (z - x) * bottom_left + 2.0 * x * center
    }
}

fn mix_hash(mut value: u32, salt: u32) -> u32 {
    value ^= salt.wrapping_mul(0x9e37_79b9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

fn hash_unit(value: u32) -> f32 {
    value as f32 / u32::MAX as f32
}

fn ground_effect_model_path(path: &str) -> String {
    if path.contains(['\\', '/']) {
        path.replace('/', "\\")
    } else {
        format!("World\\NoDXT\\Detail\\{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_all_eight_classic_predominant_cells() {
        let mut map = [0; 16];
        map[0] = 0b00_01_10_11;
        map[1] = 0b11_10_01_00;
        map[8] = 0b10_11_00_01;

        assert_eq!(decode_classic_predominant_texture(&map, 0, 0), 0);
        assert_eq!(decode_classic_predominant_texture(&map, 3, 0), 3);
        assert_eq!(decode_classic_predominant_texture(&map, 4, 0), 3);
        assert_eq!(decode_classic_predominant_texture(&map, 7, 0), 0);
        assert_eq!(decode_classic_predominant_texture(&map, 0, 4), 2);
        assert_eq!(decode_classic_predominant_texture(&map, 3, 4), 1);
    }

    #[test]
    fn reads_classic_no_effect_bitmap_from_header_tail() {
        let mut bitmap = [0; 8];
        bitmap[2] = 0b1000_0001;

        assert!(decode_classic_no_effect_doodad(&bitmap, 0, 2));
        assert!(!decode_classic_no_effect_doodad(&bitmap, 1, 2));
        assert!(decode_classic_no_effect_doodad(&bitmap, 7, 2));
    }

    #[test]
    fn applies_classic_density_limits() {
        assert_eq!(classic_density(0), 8);
        assert_eq!(classic_density(16), 16);
        assert_eq!(classic_density(25), 24);
    }

    #[test]
    fn deterministic_hash_stays_in_unit_range() {
        let value = hash_unit(mix_hash(42, 7));
        assert!((0.0..=1.0).contains(&value));
        assert_eq!(value, hash_unit(mix_hash(42, 7)));
    }

    #[test]
    fn preserves_empty_ground_effect_slots() {
        let effect = GroundEffect {
            models: [Some("grass.m2".to_owned()), None, None, None],
            density: 8,
        };

        assert_eq!(ground_effect_model(&effect, 0), Some("grass.m2"));
        assert_eq!(ground_effect_model(&effect, 1), None);
        assert_eq!(ground_effect_model(&effect, 2), None);
        assert_eq!(ground_effect_model(&effect, 3), None);
    }

    #[test]
    fn filters_generated_instances_to_camera_radius() {
        let data = GroundEffectData {
            effects: HashMap::from([(
                7,
                GroundEffect {
                    models: std::array::from_fn(|_| Some("grass.m2".to_owned())),
                    density: 8,
                },
            )]),
        };
        let source = GroundEffectSource {
            seed: 1,
            chunks: vec![GroundEffectChunk {
                position: [0.0, 0.0, 0.0],
                heights: vec![0.0; 145],
                effect_ids: vec![7],
                predominant_texture: [0; 16],
                no_effect_doodad: [0; 8],
                holes: [false; 64],
            }],
        };

        let placements = data.placements_near(&source, Vec2::ZERO, 5.0);

        assert!(!placements.is_empty());
        assert!(
            placements
                .iter()
                .all(|placement| { placement.transform.translation.xz().length_squared() <= 25.0 })
        );
    }

    #[test]
    fn terrain_height_uses_center_vertex() {
        let heights = [0.0, 0.0, 0.0, 0.0, 10.0];

        assert_eq!(interpolate_cell_height(heights, 0.5, 0.5), 10.0);
        assert_eq!(interpolate_cell_height(heights, 0.0, 0.0), 0.0);
        assert_eq!(interpolate_cell_height(heights, 1.0, 1.0), 0.0);
    }

    #[test]
    fn qualifies_ground_effect_model_basenames() {
        assert_eq!(
            ground_effect_model_path("AtcGra04.mdx"),
            "World\\NoDXT\\Detail\\AtcGra04.mdx"
        );
        assert_eq!(
            ground_effect_model_path("World/Plants/Grass.m2"),
            "World\\Plants\\Grass.m2"
        );
    }
}
