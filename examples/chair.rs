use std::f32::consts::FRAC_PI_4;

use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};
use primitives::Primitive;
use transform::Pivot;

fn generate_chair() -> SMeshResult<SMesh> {
    // --- Seat ---
    // Start with a 2x1x2 subdivided cube so we get 4 quads on top/bottom
    let (mut mesh, _) = primitives::Cube {
        subdivision: glam::U16Vec3::new(2, 1, 2),
    }
    .generate()?;

    // Scale to seat proportions: wide, thin, slightly deep
    let all = mesh.select_all();
    mesh.scale(all.clone(), vec3(2.0, 0.15, 2.0), Pivot::Origin)?;

    // Lift seat to chair height
    mesh.translate(all, Vec3::Y * 0.45)?;

    // Use describe() to verify seat geometry
    let report = mesh.describe();
    println!("=== After creating seat ===");
    println!("{}", report);

    // --- Legs ---
    // Use faces_facing to find the 4 bottom quads
    let bottom_faces = mesh.faces_facing(Vec3::NEG_Y, FRAC_PI_4);
    println!(
        "Found {} bottom faces for legs",
        bottom_faces.len()
    );

    // Extrude each bottom face into a leg
    for face in &bottom_faces {
        let top = mesh.extrude(*face)?;
        // Scale inward to make thin leg
        mesh.scale(top, vec3(0.3, 1.0, 0.3), Pivot::SelectionCog)?;
        // Extrude down for leg length
        let bottom = mesh.extrude(top)?;
        mesh.translate(bottom, Vec3::NEG_Y * 0.45)?;
    }

    // Tag the leg tips for potential future use
    let leg_tips = mesh.faces_facing(Vec3::NEG_Y, 0.1);
    mesh.tag(leg_tips.clone(), "leg_bottoms");

    println!("=== After adding legs ===");
    println!("{}", mesh.describe());

    // --- Backrest ---
    // Find the two back faces (facing -Z)
    let back_faces = mesh.faces_facing(Vec3::NEG_Z, FRAC_PI_4);
    println!("Found {} back-facing faces", back_faces.len());

    // We want the two faces on the back of the seat (not the leg sides).
    // Filter to only faces near the top of the seat (y > 0.4)
    let back_seat_faces: Vec<FaceId> = back_faces
        .into_iter()
        .filter(|f| {
            let centroid = mesh.get_face_centroid(*f).unwrap_or(Vec3::ZERO);
            centroid.y > 0.4
        })
        .collect();

    println!(
        "Filtered to {} back seat faces for backrest",
        back_seat_faces.len()
    );

    if !back_seat_faces.is_empty() {
        // Extrude the back faces up and slightly back to form the backrest
        let backrest_faces = mesh.extrude_faces(back_seat_faces)?;
        mesh.translate(backrest_faces.clone(), vec3(0.0, 0.6, -0.05))?;
        mesh.tag(backrest_faces, "backrest");
    }

    // --- Validation ---
    let validation = mesh.validate();
    println!("\n=== Validation ===");
    println!("{}", validation);

    // --- Final report ---
    println!("=== Final chair ===");
    println!("{}", mesh.describe());

    // Show tagged groups
    println!("Tags: {:?}", mesh.tag_names());

    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn init_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chair_mesh = generate_chair().unwrap();
    let v0 = chair_mesh.vertices().next().unwrap();

    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(chair_mesh.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.35, 0.15),
            perceptual_roughness: 0.8,
            ..default()
        })),
        DebugRenderSMesh {
            mesh: chair_mesh,
            selection: Selection::Vertex(v0),
            visible: false,
        },
    ));

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.3, 0.3),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(10.0)),
    ));

    // Lights
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            std::f32::consts::PI / 3.0,
            -std::f32::consts::PI / 4.0,
        )),
    ));

    commands.spawn((
        PointLight {
            intensity: 200_000.0,
            ..default()
        },
        Transform::from_translation(vec3(-3.0, 4.0, 5.0)),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        Transform::from_translation(vec3(2.0, 2.0, 3.0)).looking_at(vec3(0.0, 0.4, 0.0), Vec3::Y),
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
            EguiPlugin::default(),
        ))
        .add_systems(Startup, init_system)
        .run();
}
