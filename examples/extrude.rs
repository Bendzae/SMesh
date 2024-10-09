use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use itertools::Itertools;
use smesh::prelude::*;
use transform::Pivot;

fn extrude_faces() -> SMeshResult<SMesh> {
    // Construct SMesh
    let mut smesh = SMesh::new();
    // Make some connected faces
    let v0 = smesh.add_vertex(vec3(-1.0, 0.0, 1.0));
    let v1 = smesh.add_vertex(vec3(1.0, 0.0, 1.0));
    let v2 = smesh.add_vertex(vec3(1.0, 0.0, -1.0));
    let v3 = smesh.add_vertex(vec3(-1.0, 0.0, -1.0));

    let v4 = smesh.add_vertex(vec3(3.0, 0.0, 1.0));
    let v5 = smesh.add_vertex(vec3(3.0, 0.0, -1.0));

    let v6 = smesh.add_vertex(vec3(-1.0, 0.0, -3.0));
    let v7 = smesh.add_vertex(vec3(1.0, 0.0, -3.0));

    let f0 = smesh.make_face(vec![v0, v1, v2, v3])?;
    let f1 = smesh.make_face(vec![v1, v4, v5, v2])?;
    let f2 = smesh.make_face(vec![v3, v2, v7, v6])?;

    // Extrude all faces and move them up
    let faces = smesh.extrude_faces(vec![f0, f1, f2])?;
    smesh.translate(faces.clone(), Vec3::Y * 2.0)?;

    // Extrude two of the new faces again, move and scale them
    let faces = smesh.extrude_faces(faces[..2].to_vec())?;
    smesh.translate(faces.clone(), Vec3::Y * 2.0)?.scale(
        faces,
        Vec3::splat(0.7),
        Pivot::SelectionCog,
    )?;

    smesh.translate(smesh.vertices().collect_vec(), vec3(-1.0, 0.0, 1.0))?;
    smesh.scale(
        smesh.vertices().collect_vec(),
        Vec3::splat(0.5),
        Pivot::Origin,
    )?;
    smesh.recalculate_normals()?;
    Ok(smesh)
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let extrude_mesh = extrude_faces().unwrap();
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(extrude_mesh)),
        material: materials.add(StandardMaterial::from(Color::rgb(0.4, 0.4, 1.0))),
        transform: Transform::from_translation(vec3(0.0, 0.0, 0.0)),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(vec3(3.0, 3.0, 4.0)),
        ..default()
    });

    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 5.0, 7.0)),
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
            brightness: 300.0,
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
