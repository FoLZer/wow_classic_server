use bevy::{
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub layer_1: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    pub layer_2: Handle<Image>,
    #[texture(4)]
    #[sampler[5]]
    pub layer_3: Handle<Image>,
    #[texture(6)]
    #[sampler(7)]
    pub layer_4: Handle<Image>,

    #[texture(8)]
    #[sampler(9)]
    pub alpha: Handle<Image>,

    #[uniform(10)]
    pub animation_data: AnimationData,
}

#[derive(Debug, Clone, ShaderType)]
pub struct AnimationData {
    pub animation_directions: [IVec2Pad; 4],
    pub animation_speed1: u32,
    pub animation_speed2: u32,
    pub animation_speed3: u32,
    pub animation_speed4: u32,
}

impl AnimationData {
    pub fn new(animation_directions: [IVec2; 4], animation_speeds: [u32; 4]) -> Self {
        Self {
            animation_directions: [
                IVec2Pad::new(animation_directions[0]),
                IVec2Pad::new(animation_directions[1]),
                IVec2Pad::new(animation_directions[2]),
                IVec2Pad::new(animation_directions[3]),
            ],
            animation_speed1: animation_speeds[0],
            animation_speed2: animation_speeds[1],
            animation_speed3: animation_speeds[2],
            animation_speed4: animation_speeds[3],
        }
    }
}

#[derive(Debug, Clone, ShaderType)]
pub struct IVec2Pad {
    pub v: IVec2,
    _pad: IVec2
}

impl IVec2Pad {
    pub fn new(v: IVec2) -> Self {
        Self {
            v,
            _pad: IVec2::ZERO
        }
    }
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_shader.wgsl".into()
    }
}
