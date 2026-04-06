use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};

/// Cube with N iterations of smooth subdivision.
fn smooth_cube(iterations: usize) -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;

    let all: Vec<VertexId> = mesh.vertices().collect();
    mesh.smooth_subdivide(all, iterations)?;
    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Cube extruded twice (up then sideways), then smooth subdivided.
fn extruded_cube(iterations: usize) -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;

    // Find the face with highest Y centroid (top face)
    let top_face = mesh
        .faces()
        .max_by(|a, b| {
            let ca = mesh.get_face_centroid(*a).unwrap_or_default().y;
            let cb = mesh.get_face_centroid(*b).unwrap_or_default().y;
            ca.partial_cmp(&cb).unwrap()
        })
        .unwrap();
    let top = mesh.extrude(top_face)?;
    mesh.translate(top, vec3(0.0, 1.0, 0.0))?;

    // Find the face with highest X centroid (side face)
    let side_face = mesh
        .faces()
        .max_by(|a, b| {
            let ca = mesh.get_face_centroid(*a).unwrap_or_default().x;
            let cb = mesh.get_face_centroid(*b).unwrap_or_default().x;
            ca.partial_cmp(&cb).unwrap()
        })
        .unwrap();
    let side = mesh.extrude(side_face)?;
    mesh.translate(side, vec3(1.0, 0.0, 0.0))?;

    if iterations > 0 {
        let all: Vec<VertexId> = mesh.vertices().collect();
        mesh.smooth_subdivide(all, iterations)?;
    }

    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn spawn_mesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    smesh: SMesh,
    color: Color,
    position: Vec3,
) {
    let v0 = smesh.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(smesh.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial::from(color))),
        Transform::from_translation(position),
        DebugRenderSMesh {
            mesh: smesh,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 0 iterations — plain cube
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        smooth_cube(0).unwrap(),
        Color::srgb(0.6, 0.6, 0.6),
        vec3(-5.0, 0.0, 0.0),
    );

    // 1 iteration
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        smooth_cube(1).unwrap(),
        Color::srgb(0.9, 0.4, 0.3),
        vec3(-2.0, 0.0, 0.0),
    );

    // 2 iterations
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        smooth_cube(2).unwrap(),
        Color::srgb(0.3, 0.7, 0.5),
        vec3(1.0, 0.0, 0.0),
    );

    // 3 iterations
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        smooth_cube(3).unwrap(),
        Color::srgb(0.3, 0.5, 0.9),
        vec3(4.0, 0.0, 0.0),
    );

    // --- Bottom row: extruded cubes ---

    // 0 iterations
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        extruded_cube(0).unwrap(),
        Color::srgb(0.6, 0.6, 0.6),
        vec3(-5.0, -4.0, 0.0),
    );

    // 1 iteration
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        extruded_cube(1).unwrap(),
        Color::srgb(0.9, 0.4, 0.3),
        vec3(-2.0, -4.0, 0.0),
    );

    // 2 iterations
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        extruded_cube(2).unwrap(),
        Color::srgb(0.3, 0.7, 0.5),
        vec3(1.0, -4.0, 0.0),
    );

    // 3 iterations
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        extruded_cube(3).unwrap(),
        Color::srgb(0.3, 0.5, 0.9),
        vec3(4.0, -4.0, 0.0),
    );

    // Light
    commands.spawn((
        PointLight::default(),
        Transform::from_translation(vec3(3.0, 5.0, 6.0)),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(vec3(0.0, 3.0, 12.0)),
        PanOrbitCamera::default(),
        Msaa::Sample4,
    ));
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 300.0,
            ..default()
        })
        .add_plugins((
            DefaultPlugins,
            PanOrbitCameraPlugin,
            SMeshDebugDrawPlugin,
            EguiPlugin::default(),
        ))
        .add_systems(Startup, init_system)
        .run();
}
