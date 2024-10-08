use bevy::color::palettes::css::{BLACK, WHITE};
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use itertools::Itertools;
use primitives::{Icosphere, Primitive};
use smesh::adapters::bevy::{DebugRenderSMesh, SMeshDebugDrawPlugin, Selection};
use smesh::prelude::*;

fn init_system(mut commands: Commands) {
    let (mut smesh, data) = Icosphere { subdivisions: 2 }.generate().unwrap();
    smesh
        .scale(
            smesh.vertices().collect_vec(),
            Vec3::splat(3.0),
            transform::Pivot::MeshCog,
        )
        .unwrap();
    smesh
        .scale(
            smesh.vertices().collect_vec(),
            Vec3::splat(3.0),
            transform::Pivot::MeshCog,
        )
        .unwrap();
    commands.spawn((
        DebugRenderSMesh {
            mesh: smesh,
            selection: Selection::Vertex(data.top_vertex),
            visible: true,
        },
        TransformBundle::from_transform(Transform::from_xyz(0.0, 0.0, 0.0)),
    ));
    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 1.5, 17.0)),
            ..default()
        },
        PanOrbitCamera::default(),
    ));
}

fn main() {
    App::new()
        .insert_resource(ClearColor(BLACK.into()))
        .insert_resource(AmbientLight {
            color: WHITE.into(),
            brightness: 0.3,
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
