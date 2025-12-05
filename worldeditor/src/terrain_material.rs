use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};

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
    pub alpha: Handle<Image>
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_shader.wgsl".into()
    }
}
