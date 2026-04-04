use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, Selection, ShowcaseCamera, ShowcasePlugin},
    prelude::*,
};
use primitives::Primitive;

/// Two cubes placed touching, combined and welded to form a single connected mesh.
fn make_welded_cubes() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;

    let (mut other, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;

    // Move the second cube so it touches the first on the +X side
    let all = other.select_all();
    other.translate(all, vec3(1.0, 0.0, 0.0))?;

    let verts_before = mesh.vertices().len() + other.vertices().len();
    mesh.combine_with(other)?;

    let merges = mesh.weld_vertices(0.01)?;

    println!(
        "Welded cubes: {} vertices before, {} after ({} merges)",
        verts_before,
        mesh.vertices().len(),
        merges
    );

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// A row of cylinders welded together at their touching caps.
fn make_welded_cylinders() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Cylinder {
        segments: 8,
        height: 1.0,
        radius: 0.4,
    }
    .generate()?;

    for i in 1..3 {
        let (mut cyl, _) = primitives::Cylinder {
            segments: 8,
            height: 1.0,
            radius: 0.4,
        }
        .generate()?;
        let all = cyl.select_all();
        cyl.translate(all, vec3(0.0, i as f32 * 1.0, 0.0))?;
        mesh.combine_with(cyl)?;
    }

    let merges = mesh.weld_vertices(0.01)?;
    println!("Welded cylinder stack: {} merges", merges);

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Before/after comparison: unwelded combined cubes (left) vs welded (right).
fn make_unwelded_cubes() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;

    let (mut other, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;
    let all = other.select_all();
    other.translate(all, vec3(1.0, 0.0, 0.0))?;

    mesh.combine_with(other)?;
    // No weld - vertices remain duplicated
    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Unwelded cubes (left) - for comparison
    let unwelded = make_unwelded_cubes().unwrap();
    let v0 = unwelded.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(unwelded.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.3, 0.3),
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_translation(vec3(-3.5, 0.5, 0.0)),
        DebugRenderSMesh {
            mesh: unwelded,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));

    // Welded cubes (center)
    let welded = make_welded_cubes().unwrap();
    let v0 = welded.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(welded.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.7, 0.3),
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_translation(vec3(0.0, 0.5, 0.0)),
        DebugRenderSMesh {
            mesh: welded,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));

    // Welded cylinder stack (right)
    let cylinders = make_welded_cylinders().unwrap();
    let v0 = cylinders.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(cylinders.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.4, 0.8),
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_translation(vec3(3.5, 0.5, 0.0)),
        DebugRenderSMesh {
            mesh: cylinders,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));

    // Camera
    commands.spawn((
        ShowcaseCamera::bundle(),
        PanOrbitCamera {
            focus: vec3(0.0, 0.8, 0.0),
            radius: Some(8.0),
            yaw: Some(0.3),
            pitch: Some(0.4),
            ..default()
        },
    ));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ShowcasePlugin, PanOrbitCameraPlugin))
        .add_systems(Startup, init_system)
        .run();
}
