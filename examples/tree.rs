use core::f32;

use bevy::{math::cubic_splines::LinearSpline, prelude::*};
use bevy_inspector_egui::{
    inspector_options::ReflectInspectorOptions, quick::ResourceInspectorPlugin, InspectorOptions,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use fastrand_contrib::RngExt;
use glam::vec3;

use itertools::Itertools;
use primitives::{Circle, Primitive};
use smesh::{
    adapters::bevy::{DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};
use transform::Pivot;

#[derive(Reflect, Resource, Default, InspectorOptions, Clone)]
#[reflect(Resource, InspectorOptions)]
struct TreeParameters {
    #[inspector(min = 0.0, max = 50.0)]
    pub height: f32,
    #[inspector(min = 0.2, max = 5.0)]
    pub min_radius: f32,
    #[inspector(min = 0.2, max = 5.0)]
    pub max_radius: f32,
    #[inspector(min = 1, max = 10)]
    pub resolution_y: usize,
    pub seed: u64,
}

#[derive(Component)]
struct TreeTag;

fn generate_tree(params: TreeParameters) -> SMeshResult<SMesh> {
    let rng = &mut fastrand::Rng::with_seed(params.seed);

    let number_of_curve_points = (params.height * params.resolution_y as f32).floor() as usize;
    let scale_factor = (params.max_radius - params.min_radius) / number_of_curve_points as f32;
    let control_points = &[
        vec3(0.0, 0.0, 0.0),
        vec3(rng.f32_range(-0.4..0.4), 0.5, rng.f32_range(-0.4..0.4)),
        vec3(rng.f32_range(-0.4..0.4), 1.0, rng.f32_range(-0.4..0.4)),
    ];
    let curve = LinearSpline::new(control_points).to_curve();

    let (mut smesh, data) = Circle { segments: 8 }.generate()?;
    smesh.scale(
        smesh.vertices().collect_vec(),
        Vec3::splat(params.max_radius),
        Pivot::MeshCog,
    )?;

    let mut faces = vec![data.face];
    for p in curve.iter_positions(number_of_curve_points) {
        let pos = vec3(p.x, p.y * params.height, p.z);
        let new_faces = smesh.extrude_faces(faces.clone())?;
        smesh
            .set_position(new_faces.clone(), pos, Pivot::SelectionCog)?
            .scale(
                new_faces.clone(),
                Vec3::splat(1.0 - scale_factor),
                Pivot::SelectionCog,
            )?
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

fn update_tree_system(
    params: Res<TreeParameters>,
    trees: Query<Entity, With<TreeTag>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if params.is_changed() {
        for e in &trees {
            let smesh = generate_tree(params.clone()).unwrap();
            let v0 = smesh.vertices().next().unwrap();
            commands.entity(e).insert((
                meshes.add(Mesh::from(smesh.clone())),
                DebugRenderSMesh {
                    mesh: smesh,
                    selection: Selection::Vertex(v0),
                    visible: false,
                },
            ));
            info!("Regenerated tree");
        }
    }
}

fn init_system(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(TreeParameters {
        height: 3.0,
        min_radius: 0.7,
        max_radius: 1.0,
        resolution_y: 3,
        seed: 0,
    });

    commands.spawn((
        TreeTag,
        PbrBundle {
            material: materials.add(StandardMaterial::from(Color::rgb(1.0, 0.4, 0.4))),
            ..default()
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
            ResourceInspectorPlugin::<TreeParameters>::default(),
            // WorldInspectorPlugin::default(),
        ))
        .add_systems(Startup, init_system)
        .add_systems(Update, update_tree_system)
        .register_type::<TreeParameters>()
        .run();
}
