use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, Selection, ShowcaseCamera, ShowcasePlugin},
    prelude::*,
};
use primitives::Primitive;
use transform::Pivot;

/// Loop cut a cube, then extrude the resulting faces to create a window-like opening.
fn make_wall_with_opening() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;

    // Scale into a wall shape
    let all = mesh.select_all();
    mesh.scale(all, vec3(3.0, 2.0, 0.3), Pivot::Origin)?;

    // Find a horizontal edge to loop cut (an edge running along X on a side face)
    let he = mesh
        .halfedges()
        .find(|&h| {
            if h.is_boundary(&mesh) {
                return false;
            }
            let face = h.face().run(&mesh).ok();
            if face.map(|f| f.valence(&mesh)).unwrap_or(0) != 4 {
                return false;
            }
            // Find an edge whose direction is mostly along X
            let src = h.src_vert().position(&mesh).unwrap_or_default();
            let dst = h.dst_vert().position(&mesh).unwrap_or_default();
            let dir = (dst - src).normalize();
            dir.x.abs() > 0.8
        })
        .unwrap();

    mesh.loop_cut(he, 0.4)?;
    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Loop cut a cylinder to add detail rings.
fn make_ringed_cylinder() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Cylinder {
        segments: 12,
        height: 2.0,
        radius: 0.5,
    }
    .generate()?;

    // Find a vertical edge on the cylinder body (along Y axis)
    let he = mesh
        .halfedges()
        .find(|&h| {
            if h.is_boundary(&mesh) {
                return false;
            }
            let face = h.face().run(&mesh).ok();
            if face.map(|f| f.valence(&mesh)).unwrap_or(0) != 4 {
                return false;
            }
            let src = h.src_vert().position(&mesh).unwrap_or_default();
            let dst = h.dst_vert().position(&mesh).unwrap_or_default();
            let dir = (dst - src).normalize();
            dir.y.abs() > 0.8
        })
        .unwrap();

    // Cut at 1/3 height
    let sel = mesh.loop_cut(he, 0.33)?;

    // Scale the lower ring outward for a bulge effect
    let ring_verts = sel.resolve_to_vertices(&mesh)?;
    let new_verts: Vec<VertexId> = ring_verts
        .into_iter()
        .filter(|v| {
            let pos = v.position(&mesh).unwrap_or_default();
            // New ring vertices sit at ~1/3 height (-1.0 + 2.0*0.33 ≈ -0.34)
            (pos.y - (-1.0 + 2.0 * 0.33)).abs() < 0.2
        })
        .collect();
    mesh.scale(new_verts, vec3(1.3, 1.0, 1.3), Pivot::SelectionCog)?;

    // Cut again at the midpoint of the upper half.
    // After the first cut, vertical edges were split. Find one in the upper portion.
    let he2 = mesh
        .halfedges()
        .find(|&h| {
            if h.is_boundary(&mesh) {
                return false;
            }
            let face = h.face().run(&mesh).ok();
            if face.map(|f| f.valence(&mesh)).unwrap_or(0) != 4 {
                return false;
            }
            let src = h.src_vert().position(&mesh).unwrap_or_default();
            let dst = h.dst_vert().position(&mesh).unwrap_or_default();
            let dir = (dst - src).normalize();
            // Vertical edge where both endpoints are above the first cut
            dir.y.abs() > 0.5 && src.y.min(dst.y) > -0.5
        })
        .unwrap();

    mesh.loop_cut(he2, 0.5)?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Multiple loop cuts on a simple quad grid.
fn make_subdivided_quad() -> SMeshResult<SMesh> {
    let (mut mesh, _) = primitives::Quad.generate()?;

    // Scale up
    let all = mesh.select_all();
    mesh.scale(all, vec3(2.0, 1.0, 2.0), Pivot::Origin)?;

    // We need a quad face, find an edge
    let he = mesh.halfedges().find(|&h| !h.is_boundary(&mesh)).unwrap();
    mesh.loop_cut(he, 0.33)?;

    // Cut again perpendicular - find an edge in the other direction
    let he2 = mesh
        .halfedges()
        .find(|&h| {
            if h.is_boundary(&mesh) {
                return false;
            }
            let face = h.face().run(&mesh).ok();
            if face.map(|f| f.valence(&mesh)).unwrap_or(0) != 4 {
                return false;
            }
            let src = h.src_vert().position(&mesh).unwrap_or_default();
            let dst = h.dst_vert().position(&mesh).unwrap_or_default();
            let dir = (dst - src).normalize();
            dir.z.abs() > 0.8
        })
        .unwrap();
    mesh.loop_cut(he2, 0.5)?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Wall with loop cut
    let wall = make_wall_with_opening().unwrap();
    let v0 = wall.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(wall.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.5, 0.3),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_translation(vec3(-3.0, 1.0, 0.0)),
        DebugRenderSMesh {
            mesh: wall,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));

    // Ringed cylinder
    let cylinder = make_ringed_cylinder().unwrap();
    let v0 = cylinder.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(cylinder.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.5, 0.7),
            perceptual_roughness: 0.4,
            ..default()
        })),
        Transform::from_translation(vec3(0.0, 1.0, 0.0)),
        DebugRenderSMesh {
            mesh: cylinder,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));

    // Subdivided quad
    let quad = make_subdivided_quad().unwrap();
    let v0 = quad.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(quad.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.7, 0.4),
            perceptual_roughness: 0.6,
            ..default()
        })),
        Transform::from_translation(vec3(3.0, 0.01, 0.0)),
        DebugRenderSMesh {
            mesh: quad,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));

    // Camera
    commands.spawn((
        ShowcaseCamera::bundle(),
        PanOrbitCamera {
            focus: vec3(0.0, 0.8, 0.0),
            radius: Some(7.0),
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
