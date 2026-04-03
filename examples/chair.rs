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

    // Inset each bottom face to create the leg cross-section at the right
    // size and position, then extrude straight down. This avoids the flared
    // transition that extrude+scale creates.
    for face in &bottom_faces {
        let face_center = mesh.get_face_centroid(*face)?;

        // Inset to create a smaller face within each bottom quad.
        // amount = how far toward centroid (0=no inset, 1=collapsed).
        // We want the inner face sized to LEG_THICKNESS.
        // Each bottom quad is (SEAT_WIDTH/2) x (SEAT_DEPTH/2).
        // inset amount = 1 - (LEG_THICKNESS / quarter_size)
        let quarter_w = SEAT_WIDTH / 2.0;
        let quarter_d = SEAT_DEPTH / 2.0;
        let avg_quarter = (quarter_w + quarter_d) / 2.0;
        let inset_amount = 1.0 - (LEG_THICKNESS / avg_quarter);
        let leg_face = mesh.inset(*face, inset_amount.clamp(0.1, 0.95))?;

        // Shift the inset face toward the nearest corner of the seat
        let leg_center = mesh.get_face_centroid(leg_face)?;
        let margin = LEG_THICKNESS * 0.5 + 0.005;
        let corner_x = if face_center.x > 0.0 {
            SEAT_WIDTH / 2.0 - margin
        } else {
            -SEAT_WIDTH / 2.0 + margin
        };
        let corner_z = if face_center.z > 0.0 {
            SEAT_DEPTH / 2.0 - margin
        } else {
            -SEAT_DEPTH / 2.0 + margin
        };
        let shift = vec3(corner_x - leg_center.x, 0.0, corner_z - leg_center.z);
        mesh.translate(leg_face, shift)?;

        // Extrude straight down for leg length
        let leg_bottom = mesh.extrude(leg_face)?;
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
    // Build backrest as a separate cube, scaled and positioned, then combined.
    // This avoids the wedge problem from extruding + scaling shared vertices.
    let (mut backrest, _) = primitives::Cube {
        subdivision: glam::U16Vec3::new(1, 1, 1),
    }
    .generate()?;

    let br_all = backrest.select_all();
    backrest.scale(
        br_all.clone(),
        vec3(SEAT_WIDTH, BACKREST_HEIGHT, BACKREST_THICKNESS),
        Pivot::Origin,
    )?;
    // Position: centered on seat width, back edge of seat, rising from seat top
    backrest.translate(
        br_all,
        vec3(
            0.0,
            SEAT_HEIGHT + SEAT_THICKNESS + BACKREST_HEIGHT / 2.0,
            -SEAT_DEPTH / 2.0 + BACKREST_THICKNESS / 2.0,
        ),
    )?;
    mesh.combine_with(backrest)?;

    // --- Verification ---
    let final_report = mesh.describe();
    eprintln!("\n=== Final Chair ===\n{}", final_report);

    // Check backrest dimensions using spatial query
    let backrest_faces = mesh.faces_facing(Vec3::NEG_Z, FRAC_PI_4);
    let backrest_region: Vec<FaceId> = backrest_faces
        .into_iter()
        .filter(|f| {
            let c = mesh.get_face_centroid(*f).unwrap_or(Vec3::ZERO);
            c.y > SEAT_HEIGHT + SEAT_THICKNESS
        })
        .collect();
    mesh.tag(backrest_region.clone(), "backrest");
    let br = mesh.describe_selection(backrest_region);
    eprintln!("=== Backrest ===\n{}", br);

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

    // Render preview images for visual verification
    #[cfg(feature = "preview")]
    {
        use smesh::smesh::preview::{PreviewOptions, PreviewView};
        let opts = PreviewOptions::default()
            .with_size(512, 512)
            .with_wireframe();
        let paths = mesh.save_preview_with_options(&opts, "/tmp").unwrap();
        eprintln!("Saved previews: {:?}", paths);
    }

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
        Transform::from_translation(vec3(0.7, 0.6, 0.8)).looking_at(vec3(0.0, 0.35, -0.1), Vec3::Y),
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
        assert!(report.connected_components <= 2, "Chair should have at most 2 components (body + backrest)");

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
