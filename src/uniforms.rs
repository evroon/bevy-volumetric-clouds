use bevy::{
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_resource::{AsBindGroup, ShaderType, UniformBuffer},
    },
};

#[derive(Clone, Resource, ExtractResource, Reflect, ShaderType)]
#[reflect(Resource, Default)]
pub(crate) struct CloudsUniform {
    pub clouds_base_scale: f32,
    pub clouds_raymarch_steps_count: u32,
    pub clouds_bottom_height: f32,
    pub clouds_top_height: f32,
    pub clouds_coverage: f32,
    pub clouds_density: f32,
    pub clouds_detail_scale: f32,
    pub clouds_detail_strength: f32,
    pub clouds_base_edge_softness: f32,
    pub clouds_bottom_softness: f32,
    pub clouds_shadow_raymarch_steps_count: u32,
    pub clouds_shadow_raymarch_step_size: f32,
    pub clouds_shadow_raymarch_step_multiply: f32,
    pub clouds_ambient_color_top: Vec4,
    pub clouds_ambient_color_bottom: Vec4,
    pub clouds_min_transmittance: f32,
    pub planet_radius: f32,
    pub forward_scattering_g: f32,
    pub backward_scattering_g: f32,
    pub scattering_lerp: f32,
    pub sun_dir: Vec4,
    pub sun_color: Vec4,
    pub camera_translation: Vec3,
    pub time: f32,
    pub reprojection_strength: f32,
    pub render_resolution: Vec2,
    pub inverse_camera_view: Mat4,
    pub inverse_camera_projection: Mat4,
    pub wind_displacement: Vec3,
}

impl Default for CloudsUniform {
    fn default() -> Self {
        Self {
            clouds_raymarch_steps_count: 0,
            clouds_shadow_raymarch_steps_count: 0,
            planet_radius: 0.0,
            clouds_bottom_height: 0.,
            clouds_top_height: 0.,
            clouds_coverage: 0.0,
            clouds_detail_strength: 0.0,
            clouds_base_edge_softness: 0.0,
            clouds_bottom_softness: 0.0,
            clouds_density: 0.0,
            clouds_shadow_raymarch_step_size: 0.0,
            clouds_shadow_raymarch_step_multiply: 0.0,
            forward_scattering_g: 0.0,
            backward_scattering_g: 0.0,
            scattering_lerp: 0.0,
            clouds_ambient_color_top: Vec4::ZERO,
            clouds_ambient_color_bottom: Vec4::ZERO,
            clouds_min_transmittance: 0.0,
            clouds_base_scale: 0.0,
            clouds_detail_scale: 0.0,
            sun_dir: Vec4::ZERO,
            sun_color: Vec4::ZERO,
            camera_translation: Vec3::ZERO,
            time: 0.0,
            reprojection_strength: 0.95,
            render_resolution: Vec2::new(1920.0, 1080.0),
            inverse_camera_view: Mat4::IDENTITY,
            inverse_camera_projection: Mat4::IDENTITY,
            wind_displacement: Vec3::new(-11.0, 0.0, 23.0),
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct CloudsUniformBuffer {
    pub buffer: UniformBuffer<CloudsUniform>,
}

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub(crate) struct CloudsImage {
    #[storage_texture(0, image_format = Rgba32Float, access = ReadWrite)]
    pub cloud_render_image: Handle<Image>,

    #[storage_texture(1, image_format = Rgba32Float, access = ReadWrite)]
    pub cloud_atlas_image: Handle<Image>,

    #[storage_texture(2, image_format = Rgba32Float, access = ReadWrite, dimension = "3d")]
    pub cloud_worley_image: Handle<Image>,

    #[storage_texture(3, image_format = Rgba32Float, access = ReadWrite)]
    pub sky_image: Handle<Image>,
}
