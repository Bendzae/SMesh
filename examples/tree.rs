use core::f32;
use std::f32::consts::PI;

use bevy::{math::cubic_splines::LinearSpline, prelude::*};
use bevy_inspector_egui::{
    bevy_egui::EguiPlugin,
    inspector_options::ReflectInspectorOptions, quick::ResourceInspectorPlugin, InspectorOptions,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use fastrand_contrib::RngExt;
use glam::vec3;

use itertools::Itertools;
use primitives::{Circle, Icosphere, Primitive};
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
    pub top_radius: f32,
    #[inspector(min = 0.2, max = 5.0)]
    pub bottom_radius: f32,
    #[inspector(min = 1, max = 10)]
    pub resolution_y: usize,
    pub seed: u64,
}

#[derive(Component)]
struct TreeTag;

fn generate_tree(params: TreeParameters) -> SMeshResult<SMesh> {
    let rng = &mut fastrand::Rng::with_seed(params.seed);

    let number_of_curve_points = (params.height * params.resolution_y as f32).floor() as usize;
    let scale_factor = (params.bottom_radius - params.top_radius) / number_of_curve_points as f32;
    let mut control_points = vec![vec3(0.0, 0.0, 0.0)];

    let n_control_points = params.height.floor() as usize;
    // let n_control_points = 3;
    for i in (1..n_control_points) {
        control_points.push(vec3(
            rng.f32_range(-0.5..0.5),
            i as f32 / n_control_points as f32,
            rng.f32_range(-0.5..0.5),
        ));
    }
    let curve = LinearSpline::new(control_points).to_curve().unwrap();
    let mut curve_iter = curve.iter_positions(number_of_curve_points);

    let (mut smesh, data) = Circle { segments: 8 }.generate()?;
    smesh.scale(
        smesh.vertices().collect_vec(),
        Vec3::splat(params.bottom_radius),
        Pivot::MeshCog,
    )?;
    smesh.set_position(
        smesh.vertices().collect_vec(),
        curve_iter.next().unwrap(),
        Pivot::SelectionCog,
    )?;

    let mut faces = vec![data.face];
    let mut top_pos = Vec3::ZERO;
    for p in curve_iter {
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
        top_pos = pos;
    }
    smesh.recalculate_normals()?;

    let (mut crown, _) = Icosphere { subdivisions: 1 }.generate()?;
    let select = crown.select_all();
    crown
        .scale(
            select.clone(),
            vec3(3.0, 1.0, 3.0) * params.height.max(2.0) * 0.3,
            Pivot::SelectionCog,
        )?
        .translate(select, top_pos)?;
    smesh.combine_with(crown)?;
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
                Mesh3d(meshes.add(Mesh::from(smesh.clone()))),
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

fn init_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.insert_resource(TreeParameters {
        height: 4.0,
        top_radius: 0.3,
        bottom_radius: 1.0,
        resolution_y: 3,
        seed: 2,
    });

    commands.spawn((
        TreeTag,
        MeshMaterial3d(materials.add(StandardMaterial {
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default())),
        MeshMaterial3d(materials.add(StandardMaterial {
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(10.0)),
    ));

    // Light
    commands.spawn((
        PointLight {
            intensity: 300_000.0,
            ..default()
        },
        Transform::from_translation(vec3(-5.0, 3.0, 4.0)),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY * 2.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI / 2., -PI / 4.)),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        Transform::from_translation(Vec3::new(0.0, 5.0, 10.0)),
        PanOrbitCamera::default(),
    ));
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 300.0,
            affects_lightmapped_meshes: true,
        })
        .add_plugins((
            DefaultPlugins,
            PanOrbitCameraPlugin,
            SMeshDebugDrawPlugin,
        ))
        .add_plugins(EguiPlugin::default())
        .add_plugins(ResourceInspectorPlugin::<TreeParameters>::default())
        .add_systems(Startup, init_system)
        .add_systems(Update, update_tree_system)
        .register_type::<TreeParameters>()
        .run();
}
