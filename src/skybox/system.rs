use std::f32::consts::PI;

use bevy::{light::light_consts::lux::FULL_DAYLIGHT, prelude::*};

#[derive(Component)]
pub struct SkyboxPlane {
    pub orig_translation: Vec3,
}

pub struct SkyboxMaterials<M: Material> {
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
/// Make sure the standard_materials are unlit.
pub fn init_skybox_mesh<M: Material>(
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

pub fn init_skybox_night(
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    let nx_material = StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/skybox/nx.png")),
        unlit: true,
        ..default()
    };
    let ny_material = StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/skybox/ny.png")),
        unlit: true,
        ..default()
    };
    let nz_material = StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/skybox/nz.png")),
        unlit: true,
        ..default()
    };
    let px_material = StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/skybox/px.png")),
        unlit: true,
        ..default()
    };
    let py_material = StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/skybox/py.png")),
        unlit: true,
        ..default()
    };
    let pz_material = StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/skybox/pz.png")),
        unlit: true,
        ..default()
    };

    let materials = SkyboxMaterials {
        nx: MeshMaterial3d(standard_materials.add(nx_material)),
        ny: MeshMaterial3d(standard_materials.add(ny_material)),
        nz: MeshMaterial3d(standard_materials.add(nz_material)),
        px: MeshMaterial3d(standard_materials.add(px_material)),
        py: MeshMaterial3d(standard_materials.add(py_material)),
        pz: MeshMaterial3d(standard_materials.add(pz_material)),
    };

    init_skybox_mesh(&mut commands, meshes, materials);
}

pub fn setup_daylight(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(1.0, 1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            illuminance: FULL_DAYLIGHT,
            // shadows_enabled: true,
            ..default()
        },
    ));
}

pub fn update_skybox_transform(
    camera: Single<(&Transform, &Camera, &Projection), Without<SkyboxPlane>>,
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
