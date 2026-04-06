use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};
use transform::Pivot;

/// Subdivided cube with no spherize — the baseline.
fn cube_original() -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::splat(3),
    }
    .generate()?;
    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Subdivided cube spherized at 50%.
fn cube_half_spherize() -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::splat(3),
    }
    .generate()?;

    let all: Vec<VertexId> = mesh.vertices().collect();
    mesh.spherize(all, 0.5, Pivot::Origin, None)?;
    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Subdivided cube fully spherized — becomes a sphere.
fn cube_full_spherize() -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::splat(3),
    }
    .generate()?;

    let all: Vec<VertexId> = mesh.vertices().collect();
    mesh.spherize(all, 1.0, Pivot::Origin, None)?;
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
    // Original cube (left)
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        cube_original().unwrap(),
        Color::srgb(0.6, 0.6, 0.6),
        vec3(-3.5, 0.0, 0.0),
    );

    // 50% spherize (center)
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        cube_half_spherize().unwrap(),
        Color::srgb(0.3, 0.6, 0.9),
        vec3(0.0, 0.0, 0.0),
    );

    // 100% spherize (right)
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        cube_full_spherize().unwrap(),
        Color::srgb(0.9, 0.4, 0.3),
        vec3(3.5, 0.0, 0.0),
    );

    // Light
    commands.spawn((
        PointLight::default(),
        Transform::from_translation(vec3(3.0, 5.0, 6.0)),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(vec3(0.0, 3.0, 10.0)),
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
