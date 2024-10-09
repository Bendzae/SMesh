use core::f32;

use bevy::{math::cubic_splines::LinearSpline, prelude::*};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::{i32, vec3};

use primitives::{Circle, Primitive};
use smesh::{
    adapters::bevy::{DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};
use transform::Pivot;

fn generate_tree() -> SMeshResult<SMesh> {
    let height = 3.0;
    let min_radius = 0.1;
    let max_radius = 0.3;
    let res_y = 3;
    let number_of_curve_points = (height * res_y as f32).floor() as usize;
    let control_points = &[
        vec3(0.0, 0.0, 0.0),
        vec3(0.2, 0.5, 0.1),
        vec3(0.0, 1.0, 0.3),
    ];
    let curve = LinearSpline::new(control_points).to_curve();

    let (mut smesh, data) = Circle { segments: 8 }.generate()?;

    let mut faces = vec![data.face];
    for p in curve.iter_positions(number_of_curve_points) {
        let pos = vec3(p.x, p.y * height, p.z);
        let new_faces = smesh.extrude_faces(faces.clone())?;
        smesh
            .set_position(new_faces.clone(), pos, Pivot::SelectionCog)?
            .scale(new_faces.clone(), Vec3::splat(0.9), Pivot::SelectionCog)?
            .rotate(
                new_faces.clone(),
                Quat::from_rotation_y(f32::consts::PI / 8.0),
                Pivot::SelectionCog,
            )?;

        faces = new_faces;
    }

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
