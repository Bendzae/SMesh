use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::prelude::*;

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn SMesh
    let mut smesh = SMesh::new();
    let v0 = smesh.add_vertex(vec3(-1.0, -1.0, 0.0));
    let v1 = smesh.add_vertex(vec3(1.0, -1.0, 1.0));
    let v2 = smesh.add_vertex(vec3(1.0, 1.0, 0.0));
    let v3 = smesh.add_vertex(vec3(-1.0, 1.0, 0.0));

    let v4 = smesh.add_vertex(vec3(0.0, -2.0, 0.0));
    let _ = smesh.add_face(vec![v0, v1, v2, v3]);
    let _ = smesh.add_face(vec![v0, v4, v1]);

    let v0 = smesh.vertices().keys().next().unwrap();

    // let test_he = v0.halfedge_to(v1).run(&mesh).unwrap();
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(smesh)),
        material: materials.add(StandardMaterial::from(Color::rgb(1.0, 0.2, 0.2))),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(vec3(1.0, 1.0, 3.0)),
        ..default()
    });

    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 1.5, 5.0)),
            ..default()
        },
        PanOrbitCamera::default(),
    ));
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
        })
        .insert_resource(Msaa::Sample4)
        .add_plugins((
            DefaultPlugins,
            PanOrbitCameraPlugin,
            // WorldInspectorPlugin::default(),
        ))
        .add_systems(Startup, init_system)
        .run();
}
