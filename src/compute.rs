use std::borrow::Cow;

use bevy::{
    ecs::system::ResMut,
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderSystems,
        extract_resource::ExtractResourcePlugin,
        render_asset::RenderAssets,
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{
            AsBindGroup, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, ShaderStages, binding_types::uniform_buffer,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
    },
};

use super::{
    images::IMAGE_SIZE,
    uniforms::{CloudsImage, CloudsUniform, CloudsUniformBuffer},
};

const WORKGROUP_SIZE: u32 = 8;

#[derive(Resource, Clone, Copy)]
pub struct CameraMatrices {
    pub translation: Vec3,
    pub inverse_camera_view: Mat4,
    pub inverse_camera_projection: Mat4,
}

#[derive(Resource, Clone, Copy)]
pub struct CloudsConfig {
    pub march_steps: u32,
    pub self_shadow_steps: u32,
    pub earth_radius: f32,
    pub bottom: f32,
    pub top: f32,
    pub coverage: f32,
    pub detail_strength: f32,
    pub base_edge_softness: f32,
    pub bottom_softness: f32,
    pub density: f32,
    pub shadow_march_step_size: f32,
    pub shadow_march_step_multiply: f32,
    pub forward_scattering_g: f32,
    pub backward_scattering_g: f32,
    pub scattering_lerp: f32,
    pub ambient_color_top: Vec4,
    pub ambient_color_bottom: Vec4,
    pub min_transmittance: f32,
    pub base_scale: f32,
    pub detail_scale: f32,
    pub sun_dir: Vec4,
    pub sun_color: Vec4,
    pub camera_translation: Vec3,
    pub debug: f32,
    pub time: f32,
    pub reprojection_strength: f32,
    pub ui_visible: bool,
    pub render_resolution: Vec2,
    pub wind_velocity: Vec3,
    pub wind_displacement: Vec3,
}

impl Default for CloudsConfig {
    fn default() -> Self {
        let sun_dir = Vec3::new(-0.7, 0.5, 0.75).normalize();
        Self {
            march_steps: 12,
            self_shadow_steps: 6,
            earth_radius: 6_371_000.0,
            bottom: 1250.0,
            top: 2400.0,
            coverage: 0.5,
            detail_strength: 0.27,
            base_edge_softness: 0.1,
            bottom_softness: 0.25,
            density: 0.03,
            shadow_march_step_size: 10.0,
            shadow_march_step_multiply: 1.3,
            forward_scattering_g: 0.8,
            backward_scattering_g: -0.2,
            scattering_lerp: 0.5,
            ambient_color_top: Vec4::new(149.0, 167.0, 200.0, 0.0) * (1.5 / 225.0),
            ambient_color_bottom: Vec4::new(39.0, 67.0, 87.0, 0.0) * (1.5 / 225.0),
            min_transmittance: 0.1,
            base_scale: 1.5,
            detail_scale: 42.0,
            sun_dir: Vec4::new(sun_dir.x, sun_dir.y, sun_dir.z, 0.0),
            sun_color: Vec4::new(1.0, 0.9, 0.85, 1.0) * 1.4,
            camera_translation: Vec3::new(3980.0, 730.0, -2650.0),
            debug: 1.0,
            time: 0.0,
            reprojection_strength: 0.90,
            ui_visible: true,
            render_resolution: Vec2::new(1920.0, 1080.0),
            wind_velocity: Vec3::new(-1.1, 0.0, 2.3),
            wind_displacement: Vec3::ZERO,
        }
    }
}

#[derive(Resource)]
pub struct CloudsUniformBindGroup(BindGroup);

#[derive(Resource)]
pub struct CloudsImageBindGroup(BindGroup);

#[expect(clippy::too_many_arguments)]
pub(crate) fn prepare_uniforms_bind_group(
    mut commands: Commands,
    pipeline: Res<CloudsPipeline>,
    render_queue: Res<RenderQueue>,
    mut clouds_uniform_buffer: ResMut<CloudsUniformBuffer>,
    camera: ResMut<CameraMatrices>,
    clouds_config: Res<CloudsConfig>,
    render_device: Res<RenderDevice>,
    time: Res<Time>,
) {
    let buffer = clouds_uniform_buffer.buffer.get_mut();

    buffer.march_steps = clouds_config.march_steps;
    buffer.self_shadow_steps = clouds_config.self_shadow_steps;
    buffer.earth_radius = clouds_config.earth_radius;
    buffer.bottom = clouds_config.bottom;
    buffer.top = clouds_config.top;
    buffer.coverage = clouds_config.coverage;
    buffer.detail_strength = clouds_config.detail_strength;
    buffer.base_edge_softness = clouds_config.base_edge_softness;
    buffer.bottom_softness = clouds_config.bottom_softness;
    buffer.density = clouds_config.density;
    buffer.shadow_march_step_size = clouds_config.shadow_march_step_size;
    buffer.shadow_march_step_multiply = clouds_config.shadow_march_step_multiply;
    buffer.forward_scattering_g = clouds_config.forward_scattering_g;
    buffer.backward_scattering_g = clouds_config.backward_scattering_g;
    buffer.scattering_lerp = clouds_config.scattering_lerp;
    buffer.ambient_color_top = clouds_config.ambient_color_top;
    buffer.ambient_color_bottom = clouds_config.ambient_color_bottom;
    buffer.min_transmittance = clouds_config.min_transmittance;
    buffer.base_scale = clouds_config.base_scale;
    buffer.detail_scale = clouds_config.detail_scale;
    buffer.sun_dir = clouds_config.sun_dir;
    buffer.sun_color = clouds_config.sun_color;
    buffer.camera_translation = camera.translation;
    buffer.time = time.elapsed_secs_wrapped();
    buffer.reprojection_strength = clouds_config.reprojection_strength;
    buffer.inverse_camera_view = camera.inverse_camera_view;
    buffer.inverse_camera_projection = camera.inverse_camera_projection;
    buffer.wind_displacement += time.delta_secs() * clouds_config.wind_velocity;

    clouds_uniform_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);

    let bind_group_uniforms = render_device.create_bind_group(
        None,
        &pipeline.uniform_bind_group_layout,
        &BindGroupEntries::single(clouds_uniform_buffer.buffer.binding().unwrap().clone()),
    );
    commands.insert_resource(CloudsUniformBindGroup(bind_group_uniforms));
}

pub(crate) fn prepare_textures_bind_group(
    mut commands: Commands,
    pipeline: Res<CloudsPipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    clouds_image: Res<CloudsImage>,
    render_device: Res<RenderDevice>,
) {
    let cloud_render_view = gpu_images.get(&clouds_image.cloud_render_image).unwrap();
    let cloud_atlas_view = gpu_images.get(&clouds_image.cloud_atlas_image).unwrap();
    let cloud_worley_view = gpu_images.get(&clouds_image.cloud_worley_image).unwrap();
    let sky_view = gpu_images.get(&clouds_image.sky_image).unwrap();

    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((
            &cloud_render_view.texture_view,
            &cloud_atlas_view.texture_view,
            &cloud_worley_view.texture_view,
            &sky_view.texture_view,
        )),
    );
    commands.insert_resource(CloudsImageBindGroup(bind_group));
}

#[derive(Resource)]
pub struct CloudsPipeline {
    pub texture_bind_group_layout: BindGroupLayout,
    pub uniform_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for CloudsPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = CloudsImage::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/clouds_compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let entries = BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (uniform_buffer::<CloudsUniform>(false),),
        );

        let uniform_bind_group_layout =
            render_device.create_bind_group_layout("uniform_bind_group_layout", &entries);

        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: None,
            layout: vec![
                uniform_bind_group_layout.clone(),
                texture_bind_group_layout.clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some(Cow::from("init")),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            zero_initialize_workgroup_memory: false,
            label: None,
            layout: vec![
                uniform_bind_group_layout.clone(),
                texture_bind_group_layout.clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Some(Cow::from("update")),
        });

        CloudsPipeline {
            texture_bind_group_layout,
            uniform_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum CloudsState {
    Loading,
    Init,
    Update,
}

struct CloudsNode {
    state: CloudsState,
}

impl Default for CloudsNode {
    fn default() -> Self {
        Self {
            state: CloudsState::Loading,
        }
    }
}

impl Node for CloudsNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<CloudsPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            CloudsState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = CloudsState::Init;
                }
            }
            CloudsState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = CloudsState::Update;
                }
            }
            CloudsState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let texture_bind_group = &world.resource::<CloudsImageBindGroup>().0;
        let uniform_bind_group = &world.resource::<CloudsUniformBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<CloudsPipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, uniform_bind_group, &[]);
        pass.set_bind_group(1, texture_bind_group, &[]);

        match self.state {
            CloudsState::Loading => {}
            CloudsState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(
                    IMAGE_SIZE / WORKGROUP_SIZE,
                    IMAGE_SIZE / WORKGROUP_SIZE,
                    1,
                );
            }
            CloudsState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(
                    IMAGE_SIZE / WORKGROUP_SIZE,
                    IMAGE_SIZE / WORKGROUP_SIZE,
                    1,
                );
            }
        }
        Ok(())
    }
}

pub struct CloudsComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CloudsLabel;

impl Plugin for CloudsComputePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<CloudsImage>::default());
        app.add_plugins(ExtractResourcePlugin::<CloudsUniform>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_textures_bind_group.in_set(RenderSystems::PrepareResources),
        );
        render_app.add_systems(
            Render,
            prepare_uniforms_bind_group.in_set(RenderSystems::PrepareResources),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(CloudsLabel, CloudsNode::default());
        render_graph.add_node_edge(CloudsLabel, bevy::render::graph::CameraDriverLabel);

        render_app.add_systems(
            ExtractSchedule,
            (extract_clouds_config, extract_time, extract_camera_matrices),
        );
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<CloudsPipeline>();
        render_app.init_resource::<CloudsUniformBuffer>();
    }
}

fn extract_clouds_config(mut commands: Commands, config: Extract<Res<CloudsConfig>>) {
    commands.insert_resource(**config);
}

fn extract_time(mut commands: Commands, time: Extract<Res<Time>>) {
    commands.insert_resource(**time);
}

fn extract_camera_matrices(mut commands: Commands, camera: Extract<Res<CameraMatrices>>) {
    commands.insert_resource(**camera);
}
