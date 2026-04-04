use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, Selection, ShowcaseCamera, ShowcasePlugin},
    prelude::*,
};
use primitives::Primitive;

fn spawn_primitive(
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
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.7,
            ..default()
        })),
        Transform::from_translation(position),
        DebugRenderSMesh {
            mesh: smesh,
            selection: Selection::Vertex(v0),
            draw_mode: DebugDrawMode::Off,
        },
    ));
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spacing = 2.0;

    // Cube
    let (cube, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()
    .unwrap();
    spawn_primitive(
        &mut commands,
        &mut meshes,
        &mut materials,
        cube,
        Color::srgb(0.8, 0.3, 0.3),
        vec3(-2.0 * spacing, 0.5, 0.0),
    );

    // Icosphere
    let (sphere, _) = primitives::Icosphere { subdivisions: 2 }.generate().unwrap();
    spawn_primitive(
        &mut commands,
        &mut meshes,
        &mut materials,
        sphere,
        Color::srgb(0.3, 0.8, 0.3),
        vec3(-spacing, 0.5, 0.0),
    );

    // Cylinder
    let (cylinder, _) = primitives::Cylinder {
        segments: 16,
        height: 1.0,
        radius: 0.5,
    }
    .generate()
    .unwrap();
    spawn_primitive(
        &mut commands,
        &mut meshes,
        &mut materials,
        cylinder,
        Color::srgb(0.3, 0.3, 0.8),
        vec3(0.0, 0.5, 0.0),
    );

    // Wedge
    let (wedge, _) = primitives::Wedge {
        width: 1.0,
        height: 1.0,
        depth: 1.0,
    }
    .generate()
    .unwrap();
    spawn_primitive(
        &mut commands,
        &mut meshes,
        &mut materials,
        wedge,
        Color::srgb(0.8, 0.6, 0.2),
        vec3(spacing, 0.0, 0.0),
    );

    // Circle
    let (circle, _) = primitives::Circle { segments: 12 }.generate().unwrap();
    spawn_primitive(
        &mut commands,
        &mut meshes,
        &mut materials,
        circle,
        Color::srgb(0.6, 0.3, 0.8),
        vec3(2.0 * spacing, 0.01, 0.0),
    );

    // Quad
    let (quad, _) = primitives::Quad.generate().unwrap();
    spawn_primitive(
        &mut commands,
        &mut meshes,
        &mut materials,
        quad,
        Color::srgb(0.3, 0.7, 0.7),
        vec3(3.0 * spacing, 0.01, 0.0),
    );

    // Camera
    commands.spawn((
        ShowcaseCamera::bundle(),
        PanOrbitCamera {
            focus: vec3(1.0, 0.5, 0.0),
            radius: Some(8.0),
            yaw: Some(0.3),
            pitch: Some(0.5),
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
