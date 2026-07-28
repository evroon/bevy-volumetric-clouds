//! A minimal example featuring clouds.
use bevy::{camera::Hdr, prelude::*};
use bevy_volumetric_clouds::CloudsPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CloudsPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Hdr));
}
