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

/// Chair dimensions in meters (real-world reference)
const SEAT_WIDTH: f32 = 0.45;
const SEAT_DEPTH: f32 = 0.42;
const SEAT_THICKNESS: f32 = 0.04;
const SEAT_HEIGHT: f32 = 0.46;
const LEG_THICKNESS: f32 = 0.035;
const BACKREST_HEIGHT: f32 = 0.40;
const BACKREST_THICKNESS: f32 = 0.03;

fn generate_chair() -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();

    // --- Seat ---
    // Build a flat box for the seat using a subdivided cube (2x1x2 = 4 quads on top/bottom)
    let (mut seat, _) = primitives::Cube {
        subdivision: glam::U16Vec3::new(2, 1, 2),
    }
    .generate()?;

    // Cube is 1x1x1 centered at origin. Scale to seat dimensions.
    let all = seat.select_all();
    seat.scale(
        all.clone(),
        vec3(SEAT_WIDTH, SEAT_THICKNESS, SEAT_DEPTH),
        Pivot::Origin,
    )?;
    // Place seat bottom at SEAT_HEIGHT
    seat.translate(all, vec3(0.0, SEAT_HEIGHT + SEAT_THICKNESS * 0.5, 0.0))?;

    mesh.combine_with(seat)?;

    // Verify seat
    let seat_report = mesh.describe();
    eprintln!("=== Seat ===\n{}", seat_report);
    assert!(
        (seat_report.dimensions.x - SEAT_WIDTH).abs() < 0.01,
        "Seat width should be ~{}m, got {}",
        SEAT_WIDTH,
        seat_report.dimensions.x
    );

    // --- Legs ---
    // Find the 4 bottom faces of the seat
    let bottom_faces = mesh.faces_facing(Vec3::NEG_Y, FRAC_PI_4);
    eprintln!("Bottom faces found: {}", bottom_faces.len());
    assert_eq!(bottom_faces.len(), 4, "Expected 4 bottom quads from 2x1x2 cube");

    // Extrude each face, then scale it to leg cross-section and extend downward
    for face in &bottom_faces {
        // Extrude gives us a new face at the same position
        let leg_top = mesh.extrude(*face)?;
        // Scale to leg cross-section (fraction of quarter-seat size)
        let leg_scale = LEG_THICKNESS / (SEAT_WIDTH * 0.5);
        mesh.scale(leg_top, vec3(leg_scale, 1.0, leg_scale), Pivot::SelectionCog)?;

        // Extrude down for the full leg length
        let leg_bottom = mesh.extrude(leg_top)?;
        mesh.translate(leg_bottom, vec3(0.0, -SEAT_HEIGHT, 0.0))?;
    }

    mesh.tag(
        mesh.faces_facing(Vec3::NEG_Y, 0.1),
        "leg_bottoms",
    );

    eprintln!("=== After legs ===\n{}", mesh.describe());

    // Verify legs reach the ground
    let report = mesh.describe();
    assert!(
        report.bounding_box.0.y.abs() < 0.01,
        "Leg bottoms should reach y≈0, got y={}",
        report.bounding_box.0.y
    );

    // --- Backrest ---
    // Strategy: find the top faces at the back edge of the seat, extrude them
    // upward (they're horizontal, so extruding up creates a proper slab).
    let top_faces = mesh.faces_facing(Vec3::Y, FRAC_PI_4);
    let back_top_faces: Vec<FaceId> = top_faces
        .into_iter()
        .filter(|f| {
            let c = mesh.get_face_centroid(*f).unwrap_or(Vec3::ZERO);
            // Back half of the seat (negative Z is the back)
            c.z < 0.0
        })
        .collect();
    eprintln!("Back top faces for backrest: {}", back_top_faces.len());

    if !back_top_faces.is_empty() {
        // Extrude upward for the backrest height
        let backrest_faces = mesh.extrude_faces(back_top_faces)?;
        mesh.translate(backrest_faces.clone(), vec3(0.0, BACKREST_HEIGHT, 0.0))?;

        // Scale the backrest to be thin (compress in Z to backrest thickness)
        // relative to its own center
        let backrest_z_scale = BACKREST_THICKNESS / (SEAT_DEPTH * 0.5);
        mesh.scale(
            backrest_faces.clone(),
            vec3(1.0, 1.0, backrest_z_scale),
            Pivot::SelectionCog,
        )?;

        // Shift backrest to sit at the back edge of the seat
        // Currently centered in the back half, need to move it to z = -SEAT_DEPTH/2
        let br_report = mesh.describe_selection(backrest_faces.clone());
        let backrest_center_z = br_report.center.z;
        let target_z = -SEAT_DEPTH / 2.0 + BACKREST_THICKNESS / 2.0;
        let shift_z = target_z - backrest_center_z;
        mesh.translate(backrest_faces.clone(), vec3(0.0, 0.0, shift_z))?;

        mesh.tag(backrest_faces, "backrest_top");
    }

    // Tag all backrest-region faces by spatial query
    let all_back_facing = mesh.faces_facing(Vec3::NEG_Z, FRAC_PI_4);
    let backrest_region: Vec<FaceId> = all_back_facing
        .into_iter()
        .filter(|f| {
            let c = mesh.get_face_centroid(*f).unwrap_or(Vec3::ZERO);
            c.y > SEAT_HEIGHT + SEAT_THICKNESS
        })
        .collect();
    mesh.tag(backrest_region, "backrest");

    // --- Verification ---
    let final_report = mesh.describe();
    eprintln!("\n=== Final Chair ===\n{}", final_report);

    // Check backrest dimensions
    if let Some(backrest_sel) = mesh.take_tag("backrest_top") {
        let br = mesh.describe_selection(backrest_sel);
        eprintln!("=== Backrest Top ===\n{}", br);
    }

    let validation = mesh.validate();
    eprintln!("=== Validation ===\n{}", validation);

    // Final dimension sanity checks
    let total_height = final_report.dimensions.y;
    let expected_height = SEAT_HEIGHT + SEAT_THICKNESS + BACKREST_HEIGHT;
    eprintln!(
        "Total height: {:.3}m (expected ~{:.3}m)",
        total_height, expected_height
    );
    assert!(
        (total_height - expected_height).abs() < 0.05,
        "Chair height {:.3} should be ~{:.3}",
        total_height,
        expected_height
    );

    eprintln!("Tags: {:?}", mesh.tag_names());

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
        Transform::from_translation(vec3(0.8, 0.7, 1.0)).looking_at(vec3(0.0, 0.35, 0.0), Vec3::Y),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chair_generation_produces_valid_mesh() {
        let mesh = generate_chair().unwrap();
        let report = mesh.describe();
        let validation = mesh.validate();

        // Should have reasonable geometry
        assert!(report.vertex_count > 30, "Too few vertices: {}", report.vertex_count);
        assert!(report.face_count > 20, "Too few faces: {}", report.face_count);
        assert!(report.is_manifold, "Chair should be manifold");
        assert_eq!(report.connected_components, 1, "Chair should be one connected piece");

        // No critical validation issues (isolated verts, broken connectivity)
        for issue in &validation.issues {
            match issue {
                MeshIssue::IsolatedVertex { .. } => panic!("Has isolated vertices"),
                MeshIssue::BrokenConnectivity { .. } => panic!("Broken connectivity: {}", issue),
                _ => {}
            }
        }

        // Dimensions should be chair-shaped
        assert!(report.dimensions.x > 0.3, "Too narrow");
        assert!(report.dimensions.x < 0.6, "Too wide");
        assert!(report.dimensions.y > 0.7, "Too short");
        assert!(report.dimensions.y < 1.2, "Too tall");
    }
}
