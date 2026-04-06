use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};
use transform::Pivot;

/// Subdivided cube with a region selected and extruded to demonstrate select_region.
fn region_extrude() -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::splat(3),
    }
    .generate()?;

    // Select a region on the top of the cube
    let top_faces = mesh.select_region(
        vec3(0.0, 1.0, 0.0),
        0.8,
        Some((Vec3::Y, std::f32::consts::FRAC_PI_4)),
    );

    // Extrude and translate the selected region upward
    let extruded = mesh.extrude_faces(top_faces)?;
    mesh.translate(extruded.clone(), vec3(0.0, 0.5, 0.0))?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Subdivided cube with a region selected on the side, extruded outward.
fn region_side_bump() -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::splat(3),
    }
    .generate()?;

    // Select faces on the +X side
    let side_faces = mesh.select_region(
        vec3(1.0, 0.0, 0.0),
        0.8,
        Some((Vec3::X, std::f32::consts::FRAC_PI_4)),
    );

    // Extrude outward
    let extruded = mesh.extrude_faces(side_faces)?;
    mesh.translate(extruded, vec3(0.4, 0.0, 0.0))?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Spherized cube with a nose-like bump selected and pulled outward.
fn region_on_sphere() -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::splat(4),
    }
    .generate()?;

    let all: Vec<VertexId> = mesh.vertices().collect();
    mesh.spherize(all, 1.0, Pivot::Origin, Some(1.5))?;

    // Select a small region facing +Z (like a nose)
    let nose_faces = mesh.select_region(
        vec3(0.0, 0.0, 1.5),
        0.7,
        Some((Vec3::Z, std::f32::consts::FRAC_PI_4)),
    );

    let extruded = mesh.extrude_faces(nose_faces)?;
    mesh.translate(extruded, vec3(0.0, 0.0, 0.4))?;

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
    // Top extrude
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        region_extrude().unwrap(),
        Color::srgb(0.3, 0.6, 0.9),
        vec3(-4.0, 0.0, 0.0),
    );

    // Side bump
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        region_side_bump().unwrap(),
        Color::srgb(0.9, 0.5, 0.3),
        vec3(0.0, 0.0, 0.0),
    );

    // Nose on sphere
    spawn_mesh(
        &mut commands,
        &mut meshes,
        &mut materials,
        region_on_sphere().unwrap(),
        Color::srgb(0.3, 0.8, 0.5),
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
