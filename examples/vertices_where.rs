use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugRenderSMesh, Selection, ShowcaseCamera, ShowcasePlugin},
    prelude::*,
};
use primitives::Primitive;
use transform::Pivot;

/// Select the top-center vertices of a subdivided cube and pull them up to form a peaked roof.
fn make_peaked_box() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Cube {
        subdivision: glam::U16Vec3::new(2, 1, 1),
    }
    .generate()?;

    // With 2 subdivisions on X, there are vertices at x=0 on the top face
    let peak_verts = mesh.vertices_where(|pos| pos.y > 0.4 && pos.x.abs() < 0.01);
    mesh.translate(peak_verts, vec3(0.0, 0.5, 0.0))?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Scale just the bottom ring of a cylinder outward to form a cone-like shape.
fn make_flared_cylinder() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Cylinder {
        segments: 12,
        height: 1.5,
        radius: 0.4,
    }
    .generate()?;

    // Select the bottom ring
    let bottom = mesh.vertices_where(|pos| pos.y < -0.5);
    mesh.scale(bottom, vec3(2.0, 1.0, 2.0), Pivot::SelectionCog)?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Move one quadrant of a sphere's vertices outward for a bulge effect.
fn make_bulged_sphere() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Icosphere { subdivisions: 2 }.generate()?;

    // Select vertices in the +X, +Y quadrant
    let quadrant = mesh.vertices_where(|pos| pos.x > 0.1 && pos.y > 0.1);
    let verts = quadrant.resolve_to_vertices(&mesh)?;
    for v in verts {
        let pos = v.position(&mesh)?;
        let outward = pos.normalize() * 0.3;
        mesh.positions.insert(v, pos + outward);
    }

    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Peaked box
    let peaked = make_peaked_box().unwrap();
    let v0 = peaked.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(peaked.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.4, 0.2),
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_translation(vec3(-2.5, 0.5, 0.0)),
        DebugRenderSMesh {
            mesh: peaked,
            selection: Selection::Vertex(v0),
            visible: false,
        },
    ));

    // Flared cylinder
    let flared = make_flared_cylinder().unwrap();
    let v0 = flared.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(flared.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.6, 0.8),
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_translation(vec3(0.0, 0.75, 0.0)),
        DebugRenderSMesh {
            mesh: flared,
            selection: Selection::Vertex(v0),
            visible: false,
        },
    ));

    // Bulged sphere
    let bulged = make_bulged_sphere().unwrap();
    let v0 = bulged.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(bulged.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.8, 0.4),
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_translation(vec3(2.5, 0.5, 0.0)),
        DebugRenderSMesh {
            mesh: bulged,
            selection: Selection::Vertex(v0),
            visible: false,
        },
    ));

    // Camera
    commands.spawn((
        ShowcaseCamera::bundle(),
        PanOrbitCamera {
            focus: vec3(0.0, 0.6, 0.0),
            radius: Some(6.0),
            yaw: Some(0.4),
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
