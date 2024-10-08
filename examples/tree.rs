use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use primitives::{Circle, Primitive};
use smesh::{
    adapters::bevy::{DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};

fn generate_tree() -> SMeshResult<SMesh> {
    let (mut smesh, face) = Circle { segments: 8 }.generate()?;
    let face = smesh.extrude_faces(vec![face.face])?;
    smesh.translate(face, Vec3::Y)?;
    smesh.recalculate_normals()?;
    Ok(smesh)
}
fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let smesh = generate_tree().unwrap();
    let v0 = smesh.vertices().next().unwrap();
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(smesh.clone())),
            material: materials.add(StandardMaterial::from(Color::rgb(1.0, 0.4, 0.4))),
            ..default()
        },
        DebugRenderSMesh {
            mesh: smesh,
            selection: Selection::Vertex(v0),
            visible: true,
        },
    ));

    // Light
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(vec3(3.0, 3.0, 4.0)),
        ..default()
    });

    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 1.5, 5.0)),
            ..default()
        },
        PanOrbitCamera::default(),
    ));
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 300.0,
        })
        .insert_resource(Msaa::Sample4)
        .add_plugins((
            DefaultPlugins,
            PanOrbitCameraPlugin,
            SMeshDebugDrawPlugin,
            // WorldInspectorPlugin::default(),
        ))
        .add_systems(Startup, init_system)
        .run();
}
