use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};
use transform::Pivot;

/// Create a subdivided cube with bumps, then smooth it N iterations.
fn bumpy_cube(smooth_iterations: usize) -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::splat(4),
    }
    .generate()?;

    // Spherize first to get a rounder base shape
    let all: Vec<VertexId> = mesh.vertices().collect();
    mesh.spherize(all, 0.6, Pivot::Origin, None)?;

    // Add bumps by pushing every Nth vertex outward
    let verts: Vec<VertexId> = mesh.vertices().collect();
    for (i, &v) in verts.iter().enumerate() {
        if i % 5 == 0 {
            let pos = v.position(&mesh)?;
            let dir = pos.normalize_or_zero();
            mesh.positions.insert(v, pos + dir * 0.3);
        }
    }

    if smooth_iterations > 0 {
        let all: Vec<VertexId> = mesh.vertices().collect();
        mesh.smooth(all, smooth_iterations, 0.5, false)?;
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
    // No smoothing — bumpy
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        bumpy_cube(0).unwrap(),
        Color::srgb(0.9, 0.4, 0.3),
        vec3(-4.0, 0.0, 0.0),
    );

    // 3 iterations
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        bumpy_cube(3).unwrap(),
        Color::srgb(0.9, 0.7, 0.3),
        vec3(-1.3, 0.0, 0.0),
    );

    // 10 iterations
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        bumpy_cube(10).unwrap(),
        Color::srgb(0.3, 0.7, 0.5),
        vec3(1.3, 0.0, 0.0),
    );

    // 30 iterations — very smooth (and noticeably shrunk)
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        bumpy_cube(30).unwrap(),
        Color::srgb(0.3, 0.5, 0.9),
        vec3(4.0, 0.0, 0.0),
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
