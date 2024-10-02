use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::prelude::*;
use transform::Pivot;

fn build_cube() -> SMeshResult<SMesh> {
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
    smesh.make_face(vec![v0, v1, v2, v3])?;
    // Right
    smesh.make_face(vec![v1, v5, v6, v2])?;
    // Back
    smesh.make_face(vec![v5, v4, v7, v6])?;
    // Left
    smesh.make_face(vec![v4, v0, v3, v7])?;
    // Top
    smesh.make_face(vec![v3, v2, v6, v7])?;
    // Bottom
    smesh.make_face(vec![v4, v5, v1, v0])?;

    // Insert another vertex into the top halfedge
    let v = smesh.add_vertex(vec3(0.0, 1.3, 0.3));
    let top_he = v3.halfedge_to(v2).run(&smesh)?;
    smesh.insert_vertex(top_he, v)?;
    smesh.recalculate_normals()?;
    Ok(smesh)
}

fn build_extrude_mesh() -> SMeshResult<SMesh> {
    let mut smesh = SMesh::new();
    let v0 = smesh.add_vertex(vec3(-1.0, -1.0, 0.0));
    let v1 = smesh.add_vertex(vec3(1.0, -1.0, 0.0));
    let v2 = smesh.add_vertex(vec3(1.0, -1.0, -1.5));
    let v3 = smesh.add_vertex(vec3(-1.0, -1.0, -1.5));
    let face = smesh.make_face(vec![v0, v1, v2, v3])?;
    let face = smesh.extrude(face)?;
    smesh
        .translate(face, vec3(0.0, 1.0, 0.0))?
        .scale(face, Vec3::splat(0.6), Pivot::SelectionCog)?
        .rotate(face, Quat::from_rotation_y(PI / 10.0), Pivot::SelectionCog)?;
    let face = smesh.extrude(face)?;
    smesh
        .translate(face, vec3(0.0, 1.2, 0.0))?
        .scale(face, Vec3::splat(1.8), Pivot::SelectionCog)?
        .rotate(face, Quat::from_rotation_y(PI / 10.0), Pivot::SelectionCog)?;
    let face = smesh.extrude(face)?;
    smesh
        .translate(face, vec3(0.0, 1.2, 0.0))?
        .scale(face, Vec3::splat(0.5), Pivot::SelectionCog)?
        .rotate(face, Quat::from_rotation_y(PI / 10.0), Pivot::SelectionCog)?;

    smesh.recalculate_normals()?;
    Ok(smesh)
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = build_cube().unwrap();
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(cube)),
        material: materials.add(StandardMaterial::from(Color::rgb(1.0, 0.4, 0.4))),
        ..default()
    });

    let extrude_mesh = build_extrude_mesh().unwrap();
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(extrude_mesh)),
        material: materials.add(StandardMaterial::from(Color::rgb(0.4, 0.4, 1.0))),
        transform: Transform::from_translation(vec3(3.0, 0.0, 0.0)),
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
