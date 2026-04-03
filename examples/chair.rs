use std::f32::consts::{FRAC_PI_4, PI};

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

// === Victorian Chair Dimensions (meters) ===

// Seat
const SEAT_WIDTH: f32 = 0.48;
const SEAT_DEPTH: f32 = 0.44;
const SEAT_THICKNESS: f32 = 0.035;
const SEAT_HEIGHT: f32 = 0.46;

// Seat edge molding
const MOLDING_OVERHANG: f32 = 0.008;
const MOLDING_HEIGHT: f32 = 0.012;

// Legs (cylindrical)
const LEG_RADIUS: f32 = 0.016;
const LEG_SEGMENTS: usize = 8;
const LEG_INSET: f32 = 0.04;

// Leg turned details (decorative bulges)
const LEG_BULGE_RADIUS: f32 = 0.024;
const LEG_BULGE_HEIGHT: f32 = 0.025;

// Apron (decorative frame under seat)
const APRON_HEIGHT: f32 = 0.055;
const APRON_THICKNESS: f32 = 0.018;

// Backrest
const BACK_POST_WIDTH: f32 = 0.028;
const BACK_POST_DEPTH: f32 = 0.028;
const BACK_TOTAL_HEIGHT: f32 = 0.52;

// Top rail
const TOP_RAIL_HEIGHT: f32 = 0.055;
const TOP_RAIL_DEPTH: f32 = 0.032;

// Crown piece (decorative arch above top rail)
const CROWN_HEIGHT: f32 = 0.025;
const CROWN_WIDTH: f32 = 0.28;
const CROWN_DEPTH: f32 = 0.02;

// Finials (spheres on top of back posts)
const FINIAL_RADIUS: f32 = 0.018;
const FINIAL_SUBDIVISIONS: usize = 1;

// Splats (two vertical decorative pieces)
const SPLAT_WIDTH: f32 = 0.06;
const SPLAT_DEPTH: f32 = 0.018;
const SPLAT_GAP: f32 = 0.04; // gap between the two splats

// Cross rails
const CROSS_RAIL_HEIGHT: f32 = 0.022;
const CROSS_RAIL_DEPTH: f32 = 0.022;

// Stretchers
const STRETCHER_SIZE: f32 = 0.018;
const STRETCHER_Y: f32 = 0.09;

// Mid stretcher (connects side stretchers)
const MID_STRETCHER_Y: f32 = 0.12;

/// Helper: create a box with given dimensions centered at a position.
fn make_box(width: f32, height: f32, depth: f32, position: Vec3) -> SMeshResult<SMesh> {
    let (mut part, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;
    let all = part.select_all();
    part.scale(all.clone(), vec3(width, height, depth), Pivot::Origin)?;
    part.translate(all, position)?;
    Ok(part)
}

/// Helper: create a cylinder at a position.
fn make_cylinder(
    radius: f32,
    height: f32,
    segments: usize,
    position: Vec3,
) -> SMeshResult<SMesh> {
    let (mut cyl, _) = primitives::Cylinder {
        segments,
        height,
        radius,
    }
    .generate()?;
    let all = cyl.select_all();
    cyl.translate(all, position)?;
    Ok(cyl)
}

/// Helper: create a sphere at a position.
fn make_sphere(radius: f32, subdivisions: usize, position: Vec3) -> SMeshResult<SMesh> {
    let (mut sphere, _) = primitives::Icosphere { subdivisions }.generate()?;
    let all = sphere.select_all();
    sphere.scale(all.clone(), Vec3::splat(radius * 2.0), Pivot::Origin)?;
    sphere.translate(all, position)?;
    Ok(sphere)
}

/// Build a turned leg: a cylinder with decorative bulge rings.
fn make_turned_leg(
    radius: f32,
    height: f32,
    segments: usize,
    position: Vec3,
) -> SMeshResult<SMesh> {
    let mut leg = SMesh::new();

    // Main shaft
    let shaft = make_cylinder(radius, height, segments, position)?;
    leg.combine_with(shaft)?;

    // Upper bulge (just below the seat)
    let upper_bulge_y = position.y + height / 2.0 - height * 0.15;
    let bulge = make_cylinder(
        LEG_BULGE_RADIUS,
        LEG_BULGE_HEIGHT,
        segments,
        vec3(position.x, upper_bulge_y, position.z),
    )?;
    leg.combine_with(bulge)?;

    // Middle bulge
    let mid_bulge_y = position.y;
    let mid_bulge = make_cylinder(
        LEG_BULGE_RADIUS * 0.85,
        LEG_BULGE_HEIGHT * 0.8,
        segments,
        vec3(position.x, mid_bulge_y, position.z),
    )?;
    leg.combine_with(mid_bulge)?;

    // Lower bulge (near the foot)
    let lower_bulge_y = position.y - height / 2.0 + height * 0.12;
    let lower_bulge = make_cylinder(
        LEG_BULGE_RADIUS * 0.75,
        LEG_BULGE_HEIGHT * 0.7,
        segments,
        vec3(position.x, lower_bulge_y, position.z),
    )?;
    leg.combine_with(lower_bulge)?;

    // Small foot pad at the very bottom
    let foot_y = position.y - height / 2.0 + 0.005;
    let foot = make_cylinder(
        radius * 1.4,
        0.01,
        segments,
        vec3(position.x, foot_y, position.z),
    )?;
    leg.combine_with(foot)?;

    Ok(leg)
}

fn generate_chair() -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();

    let seat_top_y = SEAT_HEIGHT + SEAT_THICKNESS;
    let seat_center_y = SEAT_HEIGHT + SEAT_THICKNESS * 0.5;
    let back_z = -SEAT_DEPTH / 2.0;

    // === Seat ===
    let seat = make_box(
        SEAT_WIDTH,
        SEAT_THICKNESS,
        SEAT_DEPTH,
        vec3(0.0, seat_center_y, 0.0),
    )?;
    mesh.combine_with(seat)?;

    // Seat edge molding (slightly wider/deeper strip around bottom edge of seat)
    let molding_y = SEAT_HEIGHT + MOLDING_HEIGHT / 2.0;
    let molding = make_box(
        SEAT_WIDTH + MOLDING_OVERHANG * 2.0,
        MOLDING_HEIGHT,
        SEAT_DEPTH + MOLDING_OVERHANG * 2.0,
        vec3(0.0, molding_y, 0.0),
    )?;
    mesh.combine_with(molding)?;

    eprintln!("=== After seat ===\n{}", mesh.describe());

    // === Turned Legs (4 at corners) ===
    let leg_positions = [
        vec3(
            SEAT_WIDTH / 2.0 - LEG_INSET,
            SEAT_HEIGHT / 2.0,
            SEAT_DEPTH / 2.0 - LEG_INSET,
        ),
        vec3(
            -(SEAT_WIDTH / 2.0 - LEG_INSET),
            SEAT_HEIGHT / 2.0,
            SEAT_DEPTH / 2.0 - LEG_INSET,
        ),
        vec3(
            SEAT_WIDTH / 2.0 - LEG_INSET,
            SEAT_HEIGHT / 2.0,
            -(SEAT_DEPTH / 2.0 - LEG_INSET),
        ),
        vec3(
            -(SEAT_WIDTH / 2.0 - LEG_INSET),
            SEAT_HEIGHT / 2.0,
            -(SEAT_DEPTH / 2.0 - LEG_INSET),
        ),
    ];

    for pos in &leg_positions {
        let leg = make_turned_leg(LEG_RADIUS, SEAT_HEIGHT, LEG_SEGMENTS, *pos)?;
        mesh.combine_with(leg)?;
    }

    eprintln!("=== After legs ===\n{}", mesh.describe());
    let report = mesh.describe();
    assert!(
        report.bounding_box.0.y.abs() < 0.01,
        "Legs should reach y≈0, got y={}",
        report.bounding_box.0.y
    );

    // === Seat Apron (decorative frame under seat) ===
    let apron_y = SEAT_HEIGHT - APRON_HEIGHT / 2.0;
    let apron_inner_w = SEAT_WIDTH - LEG_INSET * 2.0;
    let apron_inner_d = SEAT_DEPTH - LEG_INSET * 2.0;

    // Front apron
    mesh.combine_with(make_box(
        apron_inner_w,
        APRON_HEIGHT,
        APRON_THICKNESS,
        vec3(0.0, apron_y, SEAT_DEPTH / 2.0 - LEG_INSET),
    )?)?;
    // Back apron
    mesh.combine_with(make_box(
        apron_inner_w,
        APRON_HEIGHT,
        APRON_THICKNESS,
        vec3(0.0, apron_y, -(SEAT_DEPTH / 2.0 - LEG_INSET)),
    )?)?;
    // Left apron
    mesh.combine_with(make_box(
        APRON_THICKNESS,
        APRON_HEIGHT,
        apron_inner_d,
        vec3(-(SEAT_WIDTH / 2.0 - LEG_INSET), apron_y, 0.0),
    )?)?;
    // Right apron
    mesh.combine_with(make_box(
        APRON_THICKNESS,
        APRON_HEIGHT,
        apron_inner_d,
        vec3(SEAT_WIDTH / 2.0 - LEG_INSET, apron_y, 0.0),
    )?)?;

    // Small decorative trim strip along bottom of front apron
    let trim_y = apron_y - APRON_HEIGHT / 2.0 - 0.004;
    mesh.combine_with(make_box(
        apron_inner_w + 0.006,
        0.008,
        APRON_THICKNESS + 0.004,
        vec3(0.0, trim_y, SEAT_DEPTH / 2.0 - LEG_INSET),
    )?)?;

    eprintln!("=== After apron ===\n{}", mesh.describe());

    // === Backrest ===
    let post_height = BACK_TOTAL_HEIGHT;
    let post_center_y = seat_top_y + post_height / 2.0;
    let post_x = SEAT_WIDTH / 2.0 - LEG_INSET;
    let back_post_z = back_z + BACK_POST_DEPTH / 2.0;

    // Left back post
    mesh.combine_with(make_box(
        BACK_POST_WIDTH,
        post_height,
        BACK_POST_DEPTH,
        vec3(-post_x, post_center_y, back_post_z),
    )?)?;
    // Right back post
    mesh.combine_with(make_box(
        BACK_POST_WIDTH,
        post_height,
        BACK_POST_DEPTH,
        vec3(post_x, post_center_y, back_post_z),
    )?)?;

    // Finials (small spheres on top of back posts)
    let finial_y = seat_top_y + post_height + FINIAL_RADIUS * 0.7;
    mesh.combine_with(make_sphere(
        FINIAL_RADIUS,
        FINIAL_SUBDIVISIONS,
        vec3(-post_x, finial_y, back_post_z),
    )?)?;
    mesh.combine_with(make_sphere(
        FINIAL_RADIUS,
        FINIAL_SUBDIVISIONS,
        vec3(post_x, finial_y, back_post_z),
    )?)?;

    // Top rail (wide horizontal piece across the top of backrest)
    let rail_span = post_x * 2.0 + BACK_POST_WIDTH;
    let top_rail_y = seat_top_y + post_height - TOP_RAIL_HEIGHT / 2.0;
    mesh.combine_with(make_box(
        rail_span,
        TOP_RAIL_HEIGHT,
        TOP_RAIL_DEPTH,
        vec3(0.0, top_rail_y, back_post_z),
    )?)?;

    // Crown piece (small decorative arch above center of top rail)
    let crown_y = top_rail_y + TOP_RAIL_HEIGHT / 2.0 + CROWN_HEIGHT / 2.0;
    mesh.combine_with(make_box(
        CROWN_WIDTH,
        CROWN_HEIGHT,
        CROWN_DEPTH,
        vec3(0.0, crown_y, back_post_z),
    )?)?;

    // Lower cross rail
    let lower_rail_y = seat_top_y + 0.04 + CROSS_RAIL_HEIGHT / 2.0;
    let inner_span = post_x * 2.0 - BACK_POST_WIDTH;
    mesh.combine_with(make_box(
        inner_span,
        CROSS_RAIL_HEIGHT,
        CROSS_RAIL_DEPTH,
        vec3(0.0, lower_rail_y, back_post_z),
    )?)?;

    // Upper cross rail (between splats and top rail)
    let upper_rail_y = top_rail_y - TOP_RAIL_HEIGHT / 2.0 - CROSS_RAIL_HEIGHT / 2.0 - 0.005;
    mesh.combine_with(make_box(
        inner_span,
        CROSS_RAIL_HEIGHT,
        CROSS_RAIL_DEPTH,
        vec3(0.0, upper_rail_y, back_post_z),
    )?)?;

    // Two vertical splats (decorative panels)
    let splat_bottom = lower_rail_y + CROSS_RAIL_HEIGHT / 2.0;
    let splat_top = upper_rail_y - CROSS_RAIL_HEIGHT / 2.0;
    let splat_h = splat_top - splat_bottom;
    let splat_cy = splat_bottom + splat_h / 2.0;

    // Left splat
    mesh.combine_with(make_box(
        SPLAT_WIDTH,
        splat_h,
        SPLAT_DEPTH,
        vec3(-(SPLAT_GAP / 2.0 + SPLAT_WIDTH / 2.0), splat_cy, back_post_z),
    )?)?;
    // Right splat
    mesh.combine_with(make_box(
        SPLAT_WIDTH,
        splat_h,
        SPLAT_DEPTH,
        vec3(SPLAT_GAP / 2.0 + SPLAT_WIDTH / 2.0, splat_cy, back_post_z),
    )?)?;

    // Small diamond/lozenge decorative piece between the splats
    let diamond_y = splat_cy;
    let diamond_size = 0.03;
    let (mut diamond, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;
    let dall = diamond.select_all();
    diamond.scale(
        dall.clone(),
        vec3(diamond_size, diamond_size, SPLAT_DEPTH * 0.8),
        Pivot::Origin,
    )?;
    diamond.rotate(
        dall.clone(),
        Quat::from_rotation_z(PI / 4.0),
        Pivot::Origin,
    )?;
    diamond.translate(dall, vec3(0.0, diamond_y, back_post_z))?;
    mesh.combine_with(diamond)?;

    eprintln!("=== After backrest ===\n{}", mesh.describe());

    // === Stretchers (H-stretcher pattern) ===
    let stretcher_span_x = (SEAT_WIDTH / 2.0 - LEG_INSET) * 2.0;
    let stretcher_span_z = (SEAT_DEPTH / 2.0 - LEG_INSET) * 2.0;

    // Front stretcher
    mesh.combine_with(make_box(
        stretcher_span_x,
        STRETCHER_SIZE,
        STRETCHER_SIZE,
        vec3(0.0, STRETCHER_Y, SEAT_DEPTH / 2.0 - LEG_INSET),
    )?)?;
    // Back stretcher
    mesh.combine_with(make_box(
        stretcher_span_x,
        STRETCHER_SIZE,
        STRETCHER_SIZE,
        vec3(0.0, STRETCHER_Y, -(SEAT_DEPTH / 2.0 - LEG_INSET)),
    )?)?;
    // Left side stretcher
    mesh.combine_with(make_box(
        STRETCHER_SIZE,
        STRETCHER_SIZE,
        stretcher_span_z,
        vec3(-(SEAT_WIDTH / 2.0 - LEG_INSET), STRETCHER_Y, 0.0),
    )?)?;
    // Right side stretcher
    mesh.combine_with(make_box(
        STRETCHER_SIZE,
        STRETCHER_SIZE,
        stretcher_span_z,
        vec3(SEAT_WIDTH / 2.0 - LEG_INSET, STRETCHER_Y, 0.0),
    )?)?;
    // Center cross stretcher (connecting the two side stretchers)
    mesh.combine_with(make_box(
        stretcher_span_x,
        STRETCHER_SIZE,
        STRETCHER_SIZE,
        vec3(0.0, MID_STRETCHER_Y, 0.0),
    )?)?;

    eprintln!("=== After stretchers ===\n{}", mesh.describe());

    // === Tag regions ===
    let back_faces = mesh.faces_facing(Vec3::NEG_Z, FRAC_PI_4);
    let backrest_region: Vec<FaceId> = back_faces
        .into_iter()
        .filter(|f| {
            let c = mesh.get_face_centroid(*f).unwrap_or(Vec3::ZERO);
            c.y > seat_top_y
        })
        .collect();
    mesh.tag(backrest_region, "backrest");

    mesh.tag(
        mesh.faces_facing(Vec3::NEG_Y, 0.1)
            .into_iter()
            .filter(|f| {
                let c = mesh.get_face_centroid(*f).unwrap_or(Vec3::ZERO);
                c.y < 0.02
            })
            .collect::<Vec<_>>(),
        "feet",
    );

    // === Final Verification ===
    let final_report = mesh.describe();
    eprintln!("\n=== Final Victorian Chair ===\n{}", final_report);

    let total_height = final_report.dimensions.y;
    let expected_min = seat_top_y + BACK_TOTAL_HEIGHT;
    eprintln!(
        "Total height: {:.3}m (expected ≥{:.3}m)",
        total_height, expected_min
    );
    assert!(
        total_height >= expected_min - 0.05,
        "Chair too short: {:.3}m",
        total_height
    );

    let validation = mesh.validate();
    eprintln!("=== Validation ===\n{}", validation);
    eprintln!("Tags: {:?}", mesh.tag_names());

    mesh.recalculate_normals()?;

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
            base_color: Color::srgb(0.40, 0.22, 0.10),
            perceptual_roughness: 0.65,
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
            base_color: Color::srgb(0.22, 0.20, 0.18),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(10.0)),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI / 3.0, -PI / 4.0)),
    ));

    commands.spawn((
        PointLight {
            intensity: 200_000.0,
            ..default()
        },
        Transform::from_translation(vec3(-3.0, 4.0, 5.0)),
    ));

    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        Transform::from_translation(vec3(0.7, 0.7, 1.0))
            .looking_at(vec3(0.0, 0.4, -0.05), Vec3::Y),
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

        assert!(report.vertex_count > 200, "Too few vertices: {}", report.vertex_count);
        assert!(report.face_count > 100, "Too few faces: {}", report.face_count);
        assert!(report.is_manifold, "Chair should be manifold");

        for issue in &validation.issues {
            match issue {
                MeshIssue::IsolatedVertex { .. } => panic!("Has isolated vertices"),
                MeshIssue::BrokenConnectivity { .. } => panic!("Broken connectivity: {}", issue),
                _ => {}
            }
        }

        assert!(report.dimensions.x > 0.3 && report.dimensions.x < 0.7, "Width off");
        assert!(report.dimensions.y > 0.9 && report.dimensions.y < 1.3, "Height off");
        assert!(report.dimensions.z > 0.3 && report.dimensions.z < 0.6, "Depth off");
    }
}
