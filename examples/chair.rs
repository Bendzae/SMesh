use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_inspector_egui::{
    inspector_options::ReflectInspectorOptions, quick::ResourceInspectorPlugin, InspectorOptions,
};
use glam::vec3;

use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use smesh::{
    adapters::bevy::{DebugRenderSMesh, Selection, ShowcaseConfig, ShowcasePlugin},
    prelude::*,
};
use primitives::Primitive;
use transform::Pivot;

#[derive(Reflect, Resource, InspectorOptions, Clone)]
#[reflect(Resource, InspectorOptions)]
struct ChairParameters {
    #[inspector(min = 0.3, max = 0.7)]
    pub seat_width: f32,
    #[inspector(min = 0.3, max = 0.6)]
    pub seat_depth: f32,
    #[inspector(min = 0.3, max = 0.6)]
    pub seat_height: f32,
    #[inspector(min = 0.3, max = 0.8)]
    pub backrest_height: f32,
    #[inspector(min = 0.01, max = 0.03)]
    pub leg_radius: f32,
    #[inspector(min = 4, max = 16)]
    pub leg_segments: usize,
    #[inspector(min = 1, max = 4)]
    pub num_splats: usize,
    pub show_stretchers: bool,
    pub show_finials: bool,
    pub show_crown: bool,
}

impl Default for ChairParameters {
    fn default() -> Self {
        Self {
            seat_width: 0.48,
            seat_depth: 0.44,
            seat_height: 0.46,
            backrest_height: 0.52,
            leg_radius: 0.016,
            leg_segments: 8,
            num_splats: 2,
            show_stretchers: true,
            show_finials: true,
            show_crown: true,
        }
    }
}

#[derive(Component)]
struct ChairTag;

// === Fixed proportional constants ===
const SEAT_THICKNESS: f32 = 0.035;
const MOLDING_OVERHANG: f32 = 0.008;
const MOLDING_HEIGHT: f32 = 0.012;
const LEG_INSET: f32 = 0.04;
const LEG_BULGE_SCALE: f32 = 1.5;
const LEG_BULGE_HEIGHT: f32 = 0.025;
const APRON_HEIGHT: f32 = 0.055;
const APRON_THICKNESS: f32 = 0.018;
const BACK_POST_WIDTH: f32 = 0.028;
const BACK_POST_DEPTH: f32 = 0.028;
const TOP_RAIL_HEIGHT: f32 = 0.055;
const TOP_RAIL_DEPTH: f32 = 0.032;
const CROWN_HEIGHT: f32 = 0.025;
const CROWN_DEPTH: f32 = 0.02;
const FINIAL_RADIUS: f32 = 0.018;
const SPLAT_DEPTH: f32 = 0.018;
const CROSS_RAIL_HEIGHT: f32 = 0.022;
const CROSS_RAIL_DEPTH: f32 = 0.022;
const STRETCHER_SIZE: f32 = 0.018;
const STRETCHER_Y: f32 = 0.09;

fn make_box(width: f32, height: f32, depth: f32, position: Vec3) -> SMeshResult<SMesh> {
    let (mut part, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = part.select_all();
    part.scale(all.clone(), vec3(width, height, depth), Pivot::Origin)?;
    part.translate(all, position)?;
    Ok(part)
}

fn make_cylinder(radius: f32, height: f32, segments: usize, position: Vec3) -> SMeshResult<SMesh> {
    let (mut cyl, _) = primitives::Cylinder { segments, height, radius }.generate()?;
    let all = cyl.select_all();
    cyl.translate(all, position)?;
    Ok(cyl)
}

fn make_sphere(radius: f32, subdivisions: usize, position: Vec3) -> SMeshResult<SMesh> {
    let (mut sphere, _) = primitives::Icosphere { subdivisions }.generate()?;
    let all = sphere.select_all();
    sphere.scale(all.clone(), Vec3::splat(radius * 2.0), Pivot::Origin)?;
    sphere.translate(all, position)?;
    Ok(sphere)
}

fn make_turned_leg(radius: f32, height: f32, segments: usize, position: Vec3) -> SMeshResult<SMesh> {
    let mut leg = SMesh::new();
    let bulge_r = radius * LEG_BULGE_SCALE;

    leg.combine_with(make_cylinder(radius, height, segments, position)?)?;

    let upper_y = position.y + height / 2.0 - height * 0.15;
    leg.combine_with(make_cylinder(bulge_r, LEG_BULGE_HEIGHT, segments, vec3(position.x, upper_y, position.z))?)?;

    leg.combine_with(make_cylinder(bulge_r * 0.85, LEG_BULGE_HEIGHT * 0.8, segments, vec3(position.x, position.y, position.z))?)?;

    let lower_y = position.y - height / 2.0 + height * 0.12;
    leg.combine_with(make_cylinder(bulge_r * 0.75, LEG_BULGE_HEIGHT * 0.7, segments, vec3(position.x, lower_y, position.z))?)?;

    let foot_y = position.y - height / 2.0 + 0.005;
    leg.combine_with(make_cylinder(radius * 1.4, 0.01, segments, vec3(position.x, foot_y, position.z))?)?;

    Ok(leg)
}

fn generate_chair(params: &ChairParameters) -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();

    let seat_top_y = params.seat_height + SEAT_THICKNESS;
    let seat_center_y = params.seat_height + SEAT_THICKNESS * 0.5;
    let back_z = -params.seat_depth / 2.0;

    // Seat
    mesh.combine_with(make_box(params.seat_width, SEAT_THICKNESS, params.seat_depth, vec3(0.0, seat_center_y, 0.0))?)?;

    // Seat edge molding
    let molding_y = params.seat_height + MOLDING_HEIGHT / 2.0;
    mesh.combine_with(make_box(
        params.seat_width + MOLDING_OVERHANG * 2.0, MOLDING_HEIGHT, params.seat_depth + MOLDING_OVERHANG * 2.0,
        vec3(0.0, molding_y, 0.0),
    )?)?;

    // Turned legs
    let leg_positions = [
        vec3( params.seat_width / 2.0 - LEG_INSET, params.seat_height / 2.0,  params.seat_depth / 2.0 - LEG_INSET),
        vec3(-params.seat_width / 2.0 + LEG_INSET, params.seat_height / 2.0,  params.seat_depth / 2.0 - LEG_INSET),
        vec3( params.seat_width / 2.0 - LEG_INSET, params.seat_height / 2.0, -params.seat_depth / 2.0 + LEG_INSET),
        vec3(-params.seat_width / 2.0 + LEG_INSET, params.seat_height / 2.0, -params.seat_depth / 2.0 + LEG_INSET),
    ];
    for pos in &leg_positions {
        mesh.combine_with(make_turned_leg(params.leg_radius, params.seat_height, params.leg_segments, *pos)?)?;
    }

    // Apron
    let apron_y = params.seat_height - APRON_HEIGHT / 2.0;
    let apron_inner_w = params.seat_width - LEG_INSET * 2.0;
    let apron_inner_d = params.seat_depth - LEG_INSET * 2.0;
    mesh.combine_with(make_box(apron_inner_w, APRON_HEIGHT, APRON_THICKNESS, vec3(0.0, apron_y, params.seat_depth / 2.0 - LEG_INSET))?)?;
    mesh.combine_with(make_box(apron_inner_w, APRON_HEIGHT, APRON_THICKNESS, vec3(0.0, apron_y, -(params.seat_depth / 2.0 - LEG_INSET)))?)?;
    mesh.combine_with(make_box(APRON_THICKNESS, APRON_HEIGHT, apron_inner_d, vec3(-(params.seat_width / 2.0 - LEG_INSET), apron_y, 0.0))?)?;
    mesh.combine_with(make_box(APRON_THICKNESS, APRON_HEIGHT, apron_inner_d, vec3(params.seat_width / 2.0 - LEG_INSET, apron_y, 0.0))?)?;

    // Front apron trim
    let trim_y = apron_y - APRON_HEIGHT / 2.0 - 0.004;
    mesh.combine_with(make_box(apron_inner_w + 0.006, 0.008, APRON_THICKNESS + 0.004, vec3(0.0, trim_y, params.seat_depth / 2.0 - LEG_INSET))?)?;

    // Backrest posts
    let post_height = params.backrest_height;
    let post_center_y = seat_top_y + post_height / 2.0;
    let post_x = params.seat_width / 2.0 - LEG_INSET;
    let back_post_z = back_z + BACK_POST_DEPTH / 2.0;
    mesh.combine_with(make_box(BACK_POST_WIDTH, post_height, BACK_POST_DEPTH, vec3(-post_x, post_center_y, back_post_z))?)?;
    mesh.combine_with(make_box(BACK_POST_WIDTH, post_height, BACK_POST_DEPTH, vec3(post_x, post_center_y, back_post_z))?)?;

    // Finials
    if params.show_finials {
        let finial_y = seat_top_y + post_height + FINIAL_RADIUS * 0.7;
        mesh.combine_with(make_sphere(FINIAL_RADIUS, 1, vec3(-post_x, finial_y, back_post_z))?)?;
        mesh.combine_with(make_sphere(FINIAL_RADIUS, 1, vec3(post_x, finial_y, back_post_z))?)?;
    }

    // Top rail
    let rail_span = post_x * 2.0 + BACK_POST_WIDTH;
    let top_rail_y = seat_top_y + post_height - TOP_RAIL_HEIGHT / 2.0;
    mesh.combine_with(make_box(rail_span, TOP_RAIL_HEIGHT, TOP_RAIL_DEPTH, vec3(0.0, top_rail_y, back_post_z))?)?;

    // Crown piece
    if params.show_crown {
        let crown_width = rail_span * 0.6;
        let crown_y = top_rail_y + TOP_RAIL_HEIGHT / 2.0 + CROWN_HEIGHT / 2.0;
        mesh.combine_with(make_box(crown_width, CROWN_HEIGHT, CROWN_DEPTH, vec3(0.0, crown_y, back_post_z))?)?;
    }

    // Cross rails
    let lower_rail_y = seat_top_y + 0.04 + CROSS_RAIL_HEIGHT / 2.0;
    let inner_span = post_x * 2.0 - BACK_POST_WIDTH;
    mesh.combine_with(make_box(inner_span, CROSS_RAIL_HEIGHT, CROSS_RAIL_DEPTH, vec3(0.0, lower_rail_y, back_post_z))?)?;
    let upper_rail_y = top_rail_y - TOP_RAIL_HEIGHT / 2.0 - CROSS_RAIL_HEIGHT / 2.0 - 0.005;
    mesh.combine_with(make_box(inner_span, CROSS_RAIL_HEIGHT, CROSS_RAIL_DEPTH, vec3(0.0, upper_rail_y, back_post_z))?)?;

    // Splats
    let splat_bottom = lower_rail_y + CROSS_RAIL_HEIGHT / 2.0;
    let splat_top = upper_rail_y - CROSS_RAIL_HEIGHT / 2.0;
    let splat_h = splat_top - splat_bottom;
    let splat_cy = splat_bottom + splat_h / 2.0;
    let n = params.num_splats;
    if n > 0 {
        let available = inner_span - 0.02;
        let splat_width = (available / (n as f32 * 1.5 + 0.5)).min(0.08);
        let total_splats_width = splat_width * n as f32;
        let total_gaps = available - total_splats_width;
        let gap = total_gaps / (n as f32 + 1.0);
        for i in 0..n {
            let x = -available / 2.0 + gap + splat_width / 2.0 + i as f32 * (splat_width + gap);
            mesh.combine_with(make_box(splat_width, splat_h, SPLAT_DEPTH, vec3(x, splat_cy, back_post_z))?)?;
        }
        if n >= 2 {
            let diamond_size = 0.03;
            let (mut diamond, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
            let dall = diamond.select_all();
            diamond.scale(dall.clone(), vec3(diamond_size, diamond_size, SPLAT_DEPTH * 0.8), Pivot::Origin)?;
            diamond.rotate(dall.clone(), Quat::from_rotation_z(PI / 4.0), Pivot::Origin)?;
            diamond.translate(dall, vec3(0.0, splat_cy, back_post_z))?;
            mesh.combine_with(diamond)?;
        }
    }

    // Stretchers
    if params.show_stretchers {
        let span_x = (params.seat_width / 2.0 - LEG_INSET) * 2.0;
        let span_z = (params.seat_depth / 2.0 - LEG_INSET) * 2.0;
        mesh.combine_with(make_box(span_x, STRETCHER_SIZE, STRETCHER_SIZE, vec3(0.0, STRETCHER_Y, params.seat_depth / 2.0 - LEG_INSET))?)?;
        mesh.combine_with(make_box(span_x, STRETCHER_SIZE, STRETCHER_SIZE, vec3(0.0, STRETCHER_Y, -(params.seat_depth / 2.0 - LEG_INSET)))?)?;
        mesh.combine_with(make_box(STRETCHER_SIZE, STRETCHER_SIZE, span_z, vec3(-(params.seat_width / 2.0 - LEG_INSET), STRETCHER_Y, 0.0))?)?;
        mesh.combine_with(make_box(STRETCHER_SIZE, STRETCHER_SIZE, span_z, vec3(params.seat_width / 2.0 - LEG_INSET, STRETCHER_Y, 0.0))?)?;
    }

    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn update_chair_system(
    params: Res<ChairParameters>,
    chairs: Query<Entity, With<ChairTag>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if params.is_changed() {
        for e in &chairs {
            let smesh = generate_chair(&params).unwrap();
            let v0 = smesh.vertices().next().unwrap();
            commands.entity(e).insert((
                Mesh3d(meshes.add(Mesh::from(smesh.clone()))),
                DebugRenderSMesh {
                    mesh: smesh,
                    selection: Selection::Vertex(v0),
                    visible: false,
                },
            ));
        }
    }
}

fn init_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<ShowcaseConfig>,
) {
    commands.insert_resource(ChairParameters::default());

    commands.spawn((
        ChairTag,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.40, 0.22, 0.10),
            perceptual_roughness: 0.65,
            ..default()
        })),
    ));

    // Camera with orbit controls
    let cam_offset = Vec3::new(0.5, 0.4, 0.7) * config.camera_distance;
    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        Transform::from_translation(config.look_at + cam_offset)
            .looking_at(config.look_at, Vec3::Y),
        PanOrbitCamera::default(),
    ));
}

fn main() {
    App::new()
        .insert_resource(ShowcaseConfig {
            look_at: Vec3::new(0.0, 0.4, -0.05),
            camera_distance: 2.5,
        })
        .add_plugins((DefaultPlugins, ShowcasePlugin, PanOrbitCameraPlugin, EguiPlugin::default()))
        .add_plugins(ResourceInspectorPlugin::<ChairParameters>::default())
        .add_systems(Startup, init_system)
        .add_systems(Update, update_chair_system)
        .register_type::<ChairParameters>()
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chair_generation_produces_valid_mesh() {
        let mesh = generate_chair(&ChairParameters::default()).unwrap();
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
        assert!(report.dimensions.y > 0.8 && report.dimensions.y < 1.3, "Height off");
        assert!(report.dimensions.z > 0.3 && report.dimensions.z < 0.6, "Depth off");
    }

    #[test]
    fn chair_with_custom_params() {
        let params = ChairParameters {
            seat_width: 0.55,
            seat_depth: 0.50,
            num_splats: 3,
            show_finials: false,
            show_crown: false,
            show_stretchers: false,
            ..Default::default()
        };
        let mesh = generate_chair(&params).unwrap();
        let report = mesh.describe();
        assert!(report.is_manifold);
        assert!((report.dimensions.x - 0.55).abs() < 0.02, "Width should match param");
    }
}
