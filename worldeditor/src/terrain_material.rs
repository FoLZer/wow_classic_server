use bevy::{
    pbr::MaterialExtension,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    #[texture(100)]
    #[sampler(101)]
    pub layer_1: Handle<Image>,
    #[texture(102)]
    #[sampler(103)]
    pub layer_2: Handle<Image>,
    #[texture(104)]
    #[sampler(105)]
    pub layer_3: Handle<Image>,
    #[texture(106)]
    #[sampler(107)]
    pub layer_4: Handle<Image>,

    #[texture(108)]
    #[sampler(109)]
    pub alpha: Handle<Image>,

    #[uniform(110)]
    pub animation_data: AnimationData,

    #[uniform(111)]
    pub fix_alpha: u32,
}

#[derive(Debug, Clone, ShaderType)]
pub struct AnimationData {
    animation_directions: [IVec2Compact; 2],
    pub animation_speed1: u32,
    pub animation_speed2: u32,
    pub animation_speed3: u32,
    pub animation_speed4: u32,
}

impl AnimationData {
    pub fn new(animation_directions: [IVec2; 4], animation_speeds: [u32; 4]) -> Self {
        Self {
            animation_directions: [
                IVec2Compact {
                    v1: animation_directions[0],
                    v2: animation_directions[1],
                },
                IVec2Compact {
                    v1: animation_directions[2],
                    v2: animation_directions[3],
                },
            ],
            animation_speed1: animation_speeds[0],
            animation_speed2: animation_speeds[1],
            animation_speed3: animation_speeds[2],
            animation_speed4: animation_speeds[3],
        }
    }
}

#[derive(Debug, Clone, ShaderType)]
struct IVec2Compact {
    pub v1: IVec2,
    pub v2: IVec2,
}

impl MaterialExtension for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_shader.wgsl".into()
    }

    //fn prepass_fragment_shader() -> ShaderRef {
    //    "shaders/terrain_shader.wgsl".into()
    //}
}
