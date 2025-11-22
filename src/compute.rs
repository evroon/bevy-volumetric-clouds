use bevy::{
    asset::load_embedded_asset,
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
/// Controls the compute shader which renders the volumetric clouds.
use std::borrow::Cow;

use crate::config::CloudsConfig;

use super::{
    images::IMAGE_SIZE,
    uniforms::{CloudsImage, CloudsUniform, CloudsUniformBuffer},
};

const WORKGROUP_SIZE: u32 = 8;

#[derive(Resource, Clone, Copy)]
pub(crate) struct CameraMatrices {
    pub translation: Vec3,
    pub inverse_camera_view: Mat4,
    pub inverse_camera_projection: Mat4,
}

#[derive(Resource)]
struct CloudsUniformBindGroup(BindGroup);

#[derive(Resource)]
struct CloudsImageBindGroup(BindGroup);

#[expect(clippy::too_many_arguments)]
fn prepare_uniforms_bind_group(
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

    buffer.clouds_raymarch_steps_count = clouds_config.clouds_raymarch_steps_count;
    buffer.planet_radius = clouds_config.planet_radius;
    buffer.clouds_bottom_height = clouds_config.clouds_bottom_height;
    buffer.clouds_top_height = clouds_config.clouds_top_height;
    buffer.clouds_coverage = clouds_config.clouds_coverage;
    buffer.clouds_detail_strength = clouds_config.clouds_detail_strength;
    buffer.clouds_base_edge_softness = clouds_config.clouds_base_edge_softness;
    buffer.clouds_bottom_softness = clouds_config.clouds_bottom_softness;
    buffer.clouds_density = clouds_config.clouds_density;
    buffer.clouds_shadow_raymarch_steps_count = clouds_config.clouds_shadow_raymarch_steps_count;
    buffer.clouds_shadow_raymarch_step_size = clouds_config.clouds_shadow_raymarch_step_size;
    buffer.clouds_shadow_raymarch_step_multiply =
        clouds_config.clouds_shadow_raymarch_step_multiply;
    buffer.forward_scattering_g = clouds_config.forward_scattering_g;
    buffer.backward_scattering_g = clouds_config.backward_scattering_g;
    buffer.scattering_lerp = clouds_config.scattering_lerp;
    buffer.clouds_ambient_color_top = clouds_config.clouds_ambient_color_top;
    buffer.clouds_ambient_color_bottom = clouds_config.clouds_ambient_color_bottom;
    buffer.clouds_min_transmittance = clouds_config.clouds_min_transmittance;
    buffer.clouds_base_scale = clouds_config.clouds_base_scale;
    buffer.clouds_detail_scale = clouds_config.clouds_detail_scale;
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

fn prepare_textures_bind_group(
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

/// The compute shading pipeline
///
/// Note that the compute shader is loaded in [`CloudsShaderPlugin`] so this resource depends on
/// that plugin.
#[derive(Resource)]
struct CloudsPipeline {
    texture_bind_group_layout: BindGroupLayout,
    uniform_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for CloudsPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = CloudsImage::bind_group_layout(render_device);
        let shader = load_embedded_asset!(world, "shaders/clouds_compute.wgsl");
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

/// A plugin for the compute shader which renders clouds.
pub(crate) struct CloudsComputePlugin;

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
