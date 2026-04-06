use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};

/// Cap a boundary ring with a triangle fan to a center vertex.
///
/// `ring` is the vertex ring in its original (creation) order.
/// `bottom` controls winding: true for the bottom cap, false for the top cap.
fn cap_with_fan(mesh: &mut SMesh, ring: &[VertexId], center: VertexId, bottom: bool) -> SMeshResult<()> {
    let n = ring.len();
    for i in 0..n {
        let next = (i + 1) % n;
        if bottom {
            // Bottom boundary goes forward in ring order
            mesh.make_triangle(ring[i], ring[next], center)?;
        } else {
            // Top boundary goes reverse in ring order
            mesh.make_triangle(ring[next], ring[i], center)?;
        }
    }
    Ok(())
}

/// Build a tapered tube by bridging two octagonal rings, capped with triangle fans.
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

    // Cap both ends with triangle fans
    let bottom_center = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
    let top_center = mesh.add_vertex(vec3(0.0, 3.0, 0.0));
    cap_with_fan(&mut mesh, &bottom_ring, bottom_center, true)?;
    cap_with_fan(&mut mesh, &top_ring, top_center, false)?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

/// Build a multi-segment tower by bridging three rings, capped top and bottom.
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

    // Cap the ends with triangle fans
    let bottom_center = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
    let top_center = mesh.add_vertex(vec3(0.0, 4.0, 0.0));
    cap_with_fan(&mut mesh, &ring_bottom, bottom_center, true)?;
    cap_with_fan(&mut mesh, &ring_top, top_center, false)?;

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
