use core::f32::consts::PI;

use bevy::{light::light_consts::lux::FULL_DAYLIGHT, prelude::*};

use crate::CloudCamera;

#[derive(Component)]
pub(crate) struct SkyboxPlane {
    pub orig_translation: Vec3,
}

pub(crate) struct SkyboxMaterials<M: Material> {
    pub nx: MeshMaterial3d<M>,
    pub ny: MeshMaterial3d<M>,
    pub nz: MeshMaterial3d<M>,
    pub px: MeshMaterial3d<M>,
    pub py: MeshMaterial3d<M>,
    pub pz: MeshMaterial3d<M>,
}

impl<M: Material> SkyboxMaterials<M> {
    pub fn from_one_material(material: MeshMaterial3d<M>) -> Self {
        Self {
            nx: material.clone(),
            ny: material.clone(),
            nz: material.clone(),
            px: material.clone(),
            py: material.clone(),
            pz: material.clone(),
        }
    }
}

/// Spawn 6 sides of a cube with front faces facing inwards, representing the sky
///
/// Make sure the `standard_materials` are unlit.
pub(crate) fn init_skybox_mesh<M: Material>(
    commands: &mut Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    standard_materials: SkyboxMaterials<M>,
) {
    let box_size = 1.0;

    let mesh = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(box_size)));

    // negative x
    commands.spawn((
        Mesh3d(mesh.clone()),
        standard_materials.nx,
        Transform::from_translation(Vec3::new(-box_size, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_z(-PI * 0.5) * Quat::from_rotation_y(PI * 0.5)),
        SkyboxPlane {
            orig_translation: Vec3::new(-box_size, 0.0, 0.0),
        },
    ));

    // negative y
    commands.spawn((
        Mesh3d(mesh.clone()),
        standard_materials.ny,
        Transform::from_translation(Vec3::new(0.0, -box_size, 0.0)),
        SkyboxPlane {
            orig_translation: Vec3::new(0.0, -box_size, 0.0),
        },
    ));

    // negative z
    commands.spawn((
        Mesh3d(mesh.clone()),
        standard_materials.nz,
        Transform::from_translation(Vec3::new(0.0, 0.0, -box_size))
            .with_rotation(Quat::from_rotation_x(PI * 0.5)),
        SkyboxPlane {
            orig_translation: Vec3::new(0.0, 0.0, -box_size),
        },
    ));

    // positive x
    commands.spawn((
        Mesh3d(mesh.clone()),
        standard_materials.px,
        Transform::from_translation(Vec3::new(box_size, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_z(PI * 0.5) * Quat::from_rotation_y(-PI * 0.5)),
        SkyboxPlane {
            orig_translation: Vec3::new(box_size, 0.0, 0.0),
        },
    ));

    // positive y
    commands.spawn((
        Mesh3d(mesh.clone()),
        standard_materials.py,
        Transform::from_translation(Vec3::new(0.0, box_size, 0.0))
            .with_rotation(Quat::from_rotation_z(PI) * Quat::from_rotation_y(PI)),
        SkyboxPlane {
            orig_translation: Vec3::new(0.0, box_size, 0.0),
        },
    ));

    // positive z
    commands.spawn((
        Mesh3d(mesh.clone()),
        standard_materials.pz,
        Transform::from_translation(Vec3::new(0.0, 0.0, box_size))
            .with_rotation(Quat::from_rotation_x(-PI * 0.5) * Quat::from_rotation_y(PI)),
        SkyboxPlane {
            orig_translation: Vec3::new(0.0, 0.0, box_size),
        },
    ));
}

pub(crate) fn setup_daylight(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(1.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            illuminance: FULL_DAYLIGHT,
            ..default()
        },
    ));
}

pub(crate) fn update_skybox_transform(
    camera: Single<
        (&Transform, &Camera, &Projection),
        (Without<SkyboxPlane>, With<CloudCamera>),
    >,
    mut skybox: Query<(&mut Transform, &SkyboxPlane)>,
) {
    let far = match camera.2 {
        Projection::Perspective(pers) => pers.far,
        _ => {
            panic!("unexpected projection")
        }
    };
    let scale = far * 4.0;

    for (mut transform, plane) in skybox.iter_mut() {
        transform.scale = Vec3::splat(scale);
        transform.translation = camera.0.translation + plane.orig_translation * scale;
    }
}
