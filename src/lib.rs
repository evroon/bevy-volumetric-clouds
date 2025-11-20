use bevy_egui::EguiPrimaryContextPass;
pub mod compute;
pub mod fly_camera;
pub mod images;
pub mod render;
pub mod shader_utils;
pub mod skybox;
mod ui;
pub mod uniforms;
use bevy::{
    asset::{Handle, uuid_handle},
    prelude::*,
};

use crate::{
    compute::CameraMatrices,
    images::build_images,
    render::CloudsMaterial,
    shader_utils::ShaderCommonPlugin,
    skybox::system::{SkyboxMaterials, init_skybox_mesh, setup_daylight, update_skybox_transform},
    uniforms::CloudsImage,
};

use self::{compute::CloudsComputePlugin, ui::ui_system};

pub const GLOBALS_TYPE_HANDLE: Handle<Shader> =
    uuid_handle!("0973cf27-0c9f-49a9-b818-4b927c013158");

pub const BOX_SIZE: f32 = 1000.0;

pub struct CloudsPlugin;

impl Plugin for CloudsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CloudsComputePlugin,
            ShaderCommonPlugin,
            MaterialPlugin::<CloudsMaterial>::default(),
        ))
        .add_systems(Startup, (clouds_setup, setup_daylight))
        .add_systems(Update, (update_skybox_transform, update_camera_matrices))
        .add_systems(EguiPrimaryContextPass, ui_system);
    }
}

fn clouds_setup(
    mut commands: Commands,
    images: ResMut<Assets<Image>>,
    meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CloudsMaterial>>,
) {
    let (cloud_render_image, cloud_atlas_image, cloud_worley_image, sky_image) =
        build_images(images);

    let material = materials.add(CloudsMaterial {
        cloud_render_image: cloud_render_image.clone(),
        cloud_atlas_image: cloud_atlas_image.clone(),
        cloud_worley_image: cloud_worley_image.clone(),
        sky_image: sky_image.clone(),
    });
    init_skybox_mesh(
        &mut commands,
        meshes,
        SkyboxMaterials::from_one_material(MeshMaterial3d(material.clone())),
    );
    commands.insert_resource(CloudsImage {
        cloud_render_image,
        cloud_atlas_image,
        cloud_worley_image,
        sky_image,
    });
    commands.insert_resource(CameraMatrices {
        translation: Vec3::ZERO,
        inverse_camera_projection: Mat4::IDENTITY,
        inverse_camera_view: Mat4::IDENTITY,
    });
}

fn update_camera_matrices(
    cam_query: Single<(&GlobalTransform, &Camera)>,
    mut config: ResMut<CameraMatrices>,
) {
    let (camera_transform, camera) = *cam_query;
    config.translation = camera_transform.translation();
    config.inverse_camera_view = camera_transform.to_matrix();
    config.inverse_camera_projection = camera.computed.clip_from_view.inverse();
}
