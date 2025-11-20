use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

const SHADER_ASSET_PATH: &str = "shaders/clouds.wgsl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct CloudsMaterial {
    #[texture(100, visibility(vertex, fragment))]
    #[sampler(101, visibility(vertex, fragment))]
    pub cloud_render_image: Handle<Image>,

    #[texture(102, visibility(vertex, fragment))]
    #[sampler(103, visibility(vertex, fragment))]
    pub cloud_atlas_image: Handle<Image>,

    #[texture(104, visibility(vertex, fragment), dimension = "3d")]
    #[sampler(105, visibility(vertex, fragment))]
    pub cloud_worley_image: Handle<Image>,

    #[texture(106, visibility(vertex, fragment))]
    #[sampler(107, visibility(vertex, fragment))]
    pub sky_image: Handle<Image>,
}

impl Material for CloudsMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
