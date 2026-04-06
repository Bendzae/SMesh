use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};

/// Cap a boundary ring by extruding each edge individually, then merging to center.
/// This is the standard modeling workflow: extrude edge loop → scale to zero → merge.
fn cap_with_extrude_merge(mesh: &mut SMesh, ring: &[VertexId], center: Vec3) -> SMeshResult<()> {
    // Find the boundary halfedge for each edge in the ring
    // Extrude each one individually — this avoids the closed-loop issue in extrude_edge_chain
    let mut new_verts = Vec::new();
    for i in 0..ring.len() {
        // Find a boundary halfedge touching this ring vertex
        let boundary_he = ring[i]
            .halfedges(mesh)
            .find(|he| he.is_boundary(mesh))
            .ok_or(smesh::prelude::SMeshError::TopologyError)?;

        // Extrude it — creates a quad and returns the new boundary edge
        let new_edge = mesh.extrude_edge(boundary_he)?;
        new_verts.push(new_edge.src_vert().run(mesh)?);
    }

    // Merge all extruded vertices to the center — collapses quads into triangles
    mesh.merge_vertices(&new_verts, center)?;

    Ok(())
}

/// Build a tapered tube by bridging two octagonal rings, capped via extrude+merge.
fn bridged_tube() -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();
    let segments = 8;

    let mut bottom_ring = Vec::new();
    let mut top_ring = Vec::new();

    for i in 0..segments {
        let angle = std::f32::consts::TAU * i as f32 / segments as f32;
        let x = angle.cos();
        let z = angle.sin();
        bottom_ring.push(mesh.add_vertex(vec3(x, 0.0, z)));
        top_ring.push(mesh.add_vertex(vec3(x * 0.5, 3.0, z * 0.5)));
    }

    mesh.bridge_vertices(&bottom_ring, &top_ring)?;

    // Cap both ends: extrude boundary edge loops, merge to center
    cap_with_extrude_merge(&mut mesh, &bottom_ring, vec3(0.0, 0.0, 0.0))?;
    cap_with_extrude_merge(&mut mesh, &top_ring, vec3(0.0, 3.0, 0.0))?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Build a multi-segment tower by bridging three rings, capped via extrude+merge.
fn tower() -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();
    let segments = 6;

    let make_ring = |mesh: &mut SMesh, y: f32, radius: f32| -> Vec<VertexId> {
        (0..segments)
            .map(|i| {
                let angle = std::f32::consts::TAU * i as f32 / segments as f32;
                mesh.add_vertex(vec3(angle.cos() * radius, y, angle.sin() * radius))
            })
            .collect()
    };

    let ring_bottom = make_ring(&mut mesh, 0.0, 1.2);
    let ring_mid = make_ring(&mut mesh, 2.0, 0.8);
    let ring_top = make_ring(&mut mesh, 4.0, 1.0);

    // Bridge bottom to middle, then middle to top
    mesh.bridge_vertices(&ring_bottom, &ring_mid)?;
    mesh.bridge_vertices(&ring_mid, &ring_top)?;

    // Cap the ends: extrude boundary edge loops, merge to center
    cap_with_extrude_merge(&mut mesh, &ring_bottom, vec3(0.0, 0.0, 0.0))?;
    cap_with_extrude_merge(&mut mesh, &ring_top, vec3(0.0, 4.0, 0.0))?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Tapered tube on the left
    let tube = bridged_tube().unwrap();
    let v0 = tube.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(tube.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial::from(Color::srgb(0.3, 0.6, 0.9)))),
        Transform::from_translation(vec3(-3.0, 0.0, 0.0)),
        DebugRenderSMesh {
            mesh: tube,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));

    // Multi-segment tower on the right
    let twr = tower().unwrap();
    let v1 = twr.vertices().next().unwrap();
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(twr.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial::from(Color::srgb(0.9, 0.5, 0.3)))),
        Transform::from_translation(vec3(3.0, 0.0, 0.0)),
        DebugRenderSMesh {
            mesh: twr,
            selection: Selection::Vertex(v1),
            draw_mode: DebugDrawMode::Wireframe,
        },
    ));

    // Light
    commands.spawn((
        PointLight::default(),
        Transform::from_translation(vec3(3.0, 5.0, 6.0)),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(vec3(0.0, 4.0, 10.0)),
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
