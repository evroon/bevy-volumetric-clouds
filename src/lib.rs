#![doc = include_str!("../README.md")]

mod compute;
/// Controls the compute shader which renders the volumetric clouds.
pub mod config;
/// A utility plugin to control the camera using keyboard and mouse.
#[cfg(feature = "fly_camera")]
pub mod fly_camera;
mod images;
mod render;
mod skybox;
#[cfg(feature = "debug")]
mod ui;
mod uniforms;
use bevy::{prelude::*, render::extract_component::ExtractComponent};

#[cfg(feature = "debug")]
use self::ui::ui_system;
#[cfg(feature = "debug")]
use bevy_egui::EguiPrimaryContextPass;

use crate::{
    compute::CameraMatrices,
    config::CloudsConfig,
    images::build_images,
    render::{CloudsMaterial, CloudsShaderPlugin},
    skybox::{SkyboxMaterials, init_skybox_mesh, setup_daylight, update_skybox_transform},
    uniforms::CloudsImage,
};

use self::compute::CloudsComputePlugin;

/// Marker component for cameras that should render volumetric clouds.
/// **Note:** Currently only supports one CloudCamera at a time.
#[derive(Component, Clone, Copy)]
pub struct CloudCamera;

impl ExtractComponent for CloudCamera {
    type QueryData = ();
    type QueryFilter = With<CloudCamera>;
    type Out = CloudCamera;

    fn extract_component(_: ()) -> Option<Self::Out> {
        Some(CloudCamera)
    }
}
/// A plugin for rendering clouds.
///
/// The configuration of the clouds can be changed using the [`CloudsConfig`] resource.
pub struct CloudsPlugin;

impl Plugin for CloudsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CloudsConfig::default())
            .add_plugins((CloudsComputePlugin, CloudsShaderPlugin))
            .add_systems(Startup, (clouds_setup, setup_daylight))
            .add_systems(
                PostUpdate,
                (update_skybox_transform, update_camera_matrices)
                    .after(TransformSystems::Propagate),
            );
        #[cfg(feature = "debug")]
        app.add_systems(EguiPrimaryContextPass, ui_system);
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
    cam_query: Single<(&GlobalTransform, &Camera), With<CloudCamera>>,
    mut config: ResMut<CameraMatrices>,
) {
    let (camera_transform, camera) = *cam_query;
    config.translation = camera_transform.translation();
    config.inverse_camera_view = camera_transform.to_matrix();
    config.inverse_camera_projection = camera.computed.clip_from_view.inverse();
}
