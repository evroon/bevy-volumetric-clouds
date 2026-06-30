//! A minimal example featuring clouds.
use bevy::{prelude::*, render::view::Hdr};
use bevy_volumetric_clouds::{CloudCamera, CloudsPlugin};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CloudsPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Hdr, CloudCamera));
}
