use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::prelude::*;
use transform::Pivot;

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Construct SMesh
    let mut smesh = SMesh::new();
    let v0 = smesh.add_vertex(vec3(-1.0, -1.0, 0.0));
    let v1 = smesh.add_vertex(vec3(1.0, -1.0, 0.0));
    let v2 = smesh.add_vertex(vec3(1.0, 1.0, 0.0));
    let v3 = smesh.add_vertex(vec3(-1.0, 1.0, 0.0));

    let v4 = smesh.add_vertex(vec3(-1.0, -1.0, -1.0));
    let v5 = smesh.add_vertex(vec3(1.0, -1.0, -1.0));
    let v6 = smesh.add_vertex(vec3(1.0, 1.0, -1.0));
    let v7 = smesh.add_vertex(vec3(-1.0, 1.0, -1.0));

    // Front
    smesh.make_face(vec![v0, v1, v2, v3]).unwrap();
    // Right
    smesh.make_face(vec![v1, v5, v6, v2]).unwrap();
    // Back
    smesh.make_face(vec![v5, v4, v7, v6]).unwrap();
    // Left
    smesh.make_face(vec![v4, v0, v3, v7]).unwrap();
    // Top
    smesh.make_face(vec![v3, v2, v6, v7]).unwrap();
    // Bottom
    smesh.make_face(vec![v4, v5, v1, v0]).unwrap();

    let v = smesh.add_vertex(vec3(0.0, 1.3, 0.3));

    let top_he = v3.halfedge_to(v2).run(&smesh).unwrap();
    smesh.insert_vertex(top_he, v).unwrap();

    // let test_he = v0.halfedge_to(v1).run(&mesh).unwrap();
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(smesh)),
        material: materials.add(StandardMaterial::from(Color::rgb(1.0, 0.4, 0.4))),
        ..default()
    });

    // Extrude edge test
    let mut smesh = SMesh::new();
    let v0 = smesh.add_vertex(vec3(-1.0, -1.0, 0.0));
    let v1 = smesh.add_vertex(vec3(1.0, -1.0, 0.0));
    let v2 = smesh.add_vertex(vec3(2.0, -1.0, -1.0));
    let face = smesh.make_face(vec![v0, v1, v2]).unwrap();
    let face = smesh.extrude(face).unwrap();
    smesh.translate(face, vec3(0.0, 1.0, -0.3)).unwrap();
    smesh.scale(face, Vec3::splat(0.6), Pivot::SelectionCog).unwrap();
    smesh.rotate(face, Quat::from_rotation_y(PI / 10.0), Pivot::Zero).unwrap();
    let face = smesh.extrude(face).unwrap();
    smesh.translate(face, vec3(0.0, 1.2, -0.3)).unwrap();
    smesh.scale(face, Vec3::splat(0.8), Pivot::SelectionCog).unwrap();
    smesh.rotate(face, Quat::from_rotation_y(PI / 10.0), Pivot::Zero).unwrap();
    // let e2 = smesh.extrude_edge(e1).unwrap();
    // smesh.translate(e2, vec3(0.0, 0.5, -0.4)).unwrap();
    // let e3 = smesh.extrude_edge(e2).unwrap();
    // smesh.translate(e3, vec3(0.0, 0.3, -0.4)).unwrap();

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(smesh)),
        material: materials.add(StandardMaterial::from(Color::rgb(0.4, 0.4, 1.0))),
        transform: Transform::from_translation(vec3(2.2, 0.0, 0.0)),
        ..default()
    });

    // Extrude faces test
    // let mut smesh = SMesh::new();
    // let v0 = smesh.add_vertex(vec3(-1.0, -1.0, 0.0));
    // let v1 = smesh.add_vertex(vec3(-1.0, -1.0, 1.0));
    // let v2 = smesh.add_vertex(vec3(1.0, -1.0, 1.0));
    // let v3 = smesh.add_vertex(vec3(1.0, -1.0, 0.0));
    //
    // let f0 = smesh.make_face(vec![v0, v1, v2, v3]).unwrap();
    // smesh.extrude_faces(vec![f0], 1.0).unwrap();
    //
    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(smesh)),
    //     material: materials.add(StandardMaterial::from(Color::rgb(0.8, 0.8, 0.8))),
    //     transform: Transform::from_translation(vec3(-2.2, 0.0, 0.0)),
    //     ..default()
    // });
    // Light
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(vec3(3.0, 3.0, 4.0)),
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
