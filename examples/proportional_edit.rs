use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};
use transform::{Falloff, Pivot};

/// Create a subdivided, spherized plane and pull the center up with a given falloff.
fn proportional_bump(falloff: Falloff) -> SMeshResult<SMesh> {
    use smesh::smesh::primitives::{Cube, Primitive};
    let (mut mesh, _) = Cube {
        subdivision: glam::U16Vec3::new(10, 1, 10),
    }
    .generate()?;

    // Flatten to a plane by scaling Y to 0
    let all: Vec<VertexId> = mesh.vertices().collect();
    mesh.scale(all, vec3(1.0, 0.01, 1.0), Pivot::Origin)?;

    // Pull the center upward with proportional falloff
    mesh.translate_proportional(
        Vec3::ZERO,
        0.7,
        falloff,
        vec3(0.0, 0.8, 0.0),
    )?;

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
    let configs: Vec<(&str, Falloff, Color)> = vec![
        ("Linear", Falloff::Linear, Color::srgb(0.9, 0.4, 0.3)),
        ("Smooth", Falloff::Smooth, Color::srgb(0.9, 0.7, 0.3)),
        ("Sharp", Falloff::Sharp, Color::srgb(0.3, 0.8, 0.5)),
        ("Sphere", Falloff::Sphere, Color::srgb(0.3, 0.5, 0.9)),
        ("Constant", Falloff::Constant, Color::srgb(0.7, 0.3, 0.9)),
    ];

    let spacing = 3.0;
    let start_x = -(configs.len() as f32 - 1.0) * spacing * 0.5;

    for (i, (_name, falloff, color)) in configs.into_iter().enumerate() {
        spawn_mesh(
            &mut commands,
            &mut meshes,
            &mut materials,
            proportional_bump(falloff).unwrap(),
            color,
            vec3(start_x + i as f32 * spacing, 0.0, 0.0),
        );
    }

    // Light
    commands.spawn((
        PointLight::default(),
        Transform::from_translation(vec3(3.0, 5.0, 6.0)),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(vec3(0.0, 5.0, 10.0)),
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
