//! A demo example featuring clouds and a ground plane.
//!
//! - Enable the `fly_camera` feature to be able to control the camera with keyboard and mouse.
//! - Enable the `debug` feature to be able to control the clouds settings using an `egui` UI.
use bevy::prelude::*;
use bevy::render::view::Hdr;
#[cfg(feature = "debug")]
use bevy_egui::EguiPlugin;
#[cfg(feature = "fly_camera")]
use bevy_volumetric_clouds::fly_camera::{FlyCam, FlyCameraPlugin};
use bevy_volumetric_clouds::{CloudCamera, CloudsPlugin};

fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if focus.focused && input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            CloudsPlugin,
            #[cfg(feature = "fly_camera")]
            FlyCameraPlugin,
            #[cfg(feature = "debug")]
            EguiPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, close_on_esc)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        CloudCamera,
        #[cfg(feature = "fly_camera")]
        FlyCam,
        Transform::from_translation(Vec3::new(0.0, 3.0, 0.0)).looking_to(Vec3::X, Vec3::Y),
    ));

    // Spawn ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1e4)))),
        MeshMaterial3d(std_materials.add(Color::srgb_u8(124, 144, 255))),
    ));
}
