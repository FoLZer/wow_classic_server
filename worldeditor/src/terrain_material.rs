use bevy::{
    pbr::MaterialExtension, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainMaterial {
    #[texture(100, dimension = "2d_array")]
    #[sampler(101)]
    pub textures: Handle<Image>,
    #[texture(102)]
    #[sampler(103)]
    pub alpha_map: Handle<Image>,
    #[texture(104, sample_type = "u_int")]
    pub layer_map: Handle<Image>,
    #[texture(105, sample_type = "u_int")]
    pub animation_map: Handle<Image>,
}

impl MaterialExtension for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_shader.wesl".into()
    }
}
