//! A C-shaped house massing with a central courtyard: the wings are extruded out
//! of a single cube, then each section gets a hip roof (inset its top + raise it).

use std::collections::HashSet;

use bevy::prelude::*;
use bevy_hotpatching_experiments::prelude::*;
use bevy_inspector_egui::{
    inspector_options::ReflectInspectorOptions, quick::ResourceInspectorPlugin, InspectorOptions,
};
use glam::vec3;

use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use primitives::Primitive;
use smesh::{
    adapters::bevy::{DebugDrawMode, DebugRenderSMesh, Selection, ShowcaseCamera, ShowcasePlugin},
    prelude::*,
};
use transform::Pivot;

#[derive(Reflect, Resource, InspectorOptions, Clone)]
#[reflect(Resource, InspectorOptions)]
struct CourtyardParameters {
    /// Depth of the back wing along X, in metres (also every wing's thickness).
    #[inspector(min = 3.0, max = 8.0)]
    pub wing_thickness: f32,
    /// Height of the massing, in metres.
    #[inspector(min = 2.5, max = 6.0)]
    pub height: f32,
    /// Length of the back wing along Z, in metres (the spine the arms grow from).
    #[inspector(min = 8.0, max = 24.0)]
    pub length: f32,
    /// How far the two side wings reach out along +X, in metres.
    #[inspector(min = 4.0, max = 16.0)]
    pub arm_length: f32,
    /// Number of extrude steps used to grow each wing.
    #[inspector(min = 1, max = 6)]
    pub arm_steps: usize,
    /// How far the central roof face is inset before it is raised (0 = none).
    #[inspector(min = 0.0, max = 0.9)]
    pub roof_inset: f32,
    /// How high the central roof is pulled up, in metres.
    #[inspector(min = 0.0, max = 4.0)]
    pub roof_height: f32,
}

impl Default for CourtyardParameters {
    fn default() -> Self {
        Self {
            wing_thickness: 5.0,
            height: 3.0,
            length: 15.0,
            arm_length: 8.0,
            arm_steps: 3,
            roof_inset: 0.35,
            roof_height: 2.0,
        }
    }
}

#[derive(Component)]
struct CourtyardTag;

/// Largest footprint dimension, in showcase units, after the building-sized
/// mesh is scaled down to fit the floor and lighting.
const DISPLAY_SIZE: f32 = 4.0;

/// Roof a flat top region: merge its up-facing faces into one face (dissolving
/// the cuts between them), then inset that face and pull it up into a ridge that
/// runs along the region's long axis. `in_region` picks the faces to roof.
fn roof_region<F: Fn(&SMesh, FaceId) -> bool>(
    mesh: &mut SMesh,
    in_region: F,
    inset: f32,
    height: f32,
) -> SMeshResult<()> {
    if height <= 0.0 {
        return Ok(());
    }
    let region: Vec<FaceId> = mesh
        .faces_facing(Vec3::Y, 0.1)
        .into_iter()
        .filter(|&f| in_region(mesh, f))
        .collect();
    if region.is_empty() {
        return Ok(());
    }

    // Remember the region's vertices so the merged face can be found afterwards.
    // Re-selecting by normal is unreliable: merging introduces collinear vertices
    // that make the face's computed normal degenerate.
    let region_verts: HashSet<VertexId> = region.iter().flat_map(|&f| f.vertices(mesh)).collect();

    // Dissolve every edge shared between two faces of the region so it becomes
    // a single face (one halfedge per shared edge).
    let mut shared: Vec<HalfedgeId> = Vec::new();
    for &f in &region {
        for he in f.halfedges(mesh).collect::<Vec<_>>() {
            let opp = he.opposite().run(mesh)?;
            let neighbour_in = opp
                .face()
                .run(mesh)
                .ok()
                .map(|a| region.contains(&a))
                .unwrap_or(false);
            if neighbour_in && !shared.contains(&he) && !shared.contains(&opp) {
                shared.push(he);
            }
        }
    }
    for he in shared {
        if mesh.is_removal_ok(he).is_ok() {
            mesh.remove_edge(he)?;
        }
    }

    // The merged roof face is the one built entirely from the region's vertices.
    let merged = mesh.faces().collect::<Vec<_>>().into_iter().find(|&f| {
        let vs: Vec<VertexId> = f.vertices(mesh).collect();
        !vs.is_empty() && vs.iter().all(|v| region_verts.contains(v))
    });
    if let Some(top) = merged {
        let ridge = mesh.inset(top, inset)?;
        mesh.translate(ridge, Vec3::Y * height)?;
    }
    Ok(())
}

fn generate_courtyard(params: &CourtyardParameters) -> SMeshResult<SMesh> {
    // Start from a cube pre-split into 3 cells along Z, so the +X side is three
    // stacked quads. The two end quads become the roots of the wings.
    let (mut mesh, _) = primitives::Cube {
        subdivision: glam::U16Vec3::new(1, 1, 3),
    }
    .generate()?;

    // Stretch the unit cube into the back wing (the spine of the C).
    let all = mesh.select_all();
    mesh.scale(
        all,
        vec3(params.wing_thickness, params.height, params.length),
        Pivot::Origin,
    )?;

    // Pick the two outer +X faces (skip the middle one — that wall stays as the
    // back of the courtyard).
    let z_threshold = params.length * 0.25;
    let wing_faces: Vec<FaceId> = mesh
        .faces_facing(Vec3::X, 0.1)
        .into_iter()
        .filter(|&f| {
            mesh.get_face_centroid(f)
                .map(|c| c.z.abs() > z_threshold)
                .unwrap_or(false)
        })
        .collect();

    // Grow each wing outward by extruding its root face a few times. The two
    // wings are disjoint, so extrude them one at a time.
    let step = params.arm_length / params.arm_steps as f32;
    for mut face in wing_faces {
        for _ in 0..params.arm_steps {
            face = mesh.extrude(face)?;
            mesh.translate(face, Vec3::X * step)?;
        }
    }

    // Roofs: a ridge along the spine over the back wing, and a ridge along each
    // wing. Spine tops sit within the spine's X span; each wing's tops sit further
    // out along +X, split by Z sign. Centroids are read fresh inside each call,
    // so roofing one region doesn't disturb the others.
    let half_x = params.wing_thickness * 0.5;
    let centroid = |mesh: &SMesh, f: FaceId| mesh.get_face_centroid(f).unwrap_or(Vec3::ZERO);
    roof_region(
        &mut mesh,
        |m, f| centroid(m, f).x.abs() < half_x,
        params.roof_inset,
        params.roof_height,
    )?;
    roof_region(
        &mut mesh,
        |m, f| {
            let c = centroid(m, f);
            c.x > half_x && c.z > 0.0
        },
        params.roof_inset,
        params.roof_height,
    )?;
    roof_region(
        &mut mesh,
        |m, f| {
            let c = centroid(m, f);
            c.x > half_x && c.z < 0.0
        },
        params.roof_inset,
        params.roof_height,
    )?;

    // Params are in metres, so the mesh is building-sized. Scale the whole thing
    // down to a display size that suits the showcase floor/lights, then sit it on
    // the ground plane.
    let dims = mesh.describe().dimensions;
    let max_dim = dims.max_element().max(f32::EPSILON);
    let all = mesh.select_all();
    mesh.scale(all, Vec3::splat(DISPLAY_SIZE / max_dim), Pivot::Origin)?;

    let min_y = mesh.describe().bounding_box.0.y;
    let all = mesh.select_all();
    mesh.translate(all, Vec3::Y * -min_y)?;

    mesh.recalculate_normals()?;
    Ok(mesh)
}

#[hot]
fn update_courtyard_system(
    params: Res<CourtyardParameters>,
    tagged: Query<Entity, With<CourtyardTag>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut hot_events: MessageReader<HotPatched>,
) {
    let hot_patched = hot_events.read().count() > 0;
    if params.is_changed() || hot_patched {
        for e in &tagged {
            let smesh = generate_courtyard(&params).unwrap();
            let v0 = smesh.vertices().next().unwrap();
            commands.entity(e).insert((
                Mesh3d(meshes.add(Mesh::from(smesh.clone()))),
                DebugRenderSMesh {
                    mesh: smesh,
                    selection: Selection::Vertex(v0),
                    draw_mode: DebugDrawMode::Wireframe,
                },
            ));
        }
    }
}

fn init_system(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(CourtyardParameters::default());

    commands.spawn((
        CourtyardTag,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.70, 0.66),
            perceptual_roughness: 0.8,
            ..default()
        })),
    ));

    commands.spawn((
        ShowcaseCamera::bundle(),
        PanOrbitCamera {
            focus: vec3(1.0, 0.4, 0.0),
            radius: Some(7.0),
            yaw: Some(0.9),
            pitch: Some(0.7),
            ..default()
        },
    ));
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            SimpleSubsecondPlugin::default(),
            ShowcasePlugin,
            PanOrbitCameraPlugin,
            EguiPlugin::default(),
        ))
        .add_plugins(ResourceInspectorPlugin::<CourtyardParameters>::default())
        .add_systems(Startup, init_system)
        .add_systems(Update, update_courtyard_system)
        .register_type::<CourtyardParameters>()
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courtyard_is_valid_c_shape() {
        let params = CourtyardParameters::default();
        let mesh = generate_courtyard(&params).unwrap();
        let report = mesh.describe();
        let validation = mesh.validate();

        eprintln!("{}", report);
        eprintln!("{}", validation);

        assert!(report.is_manifold, "massing should be manifold");
        assert!(report.is_closed, "massing should be a closed solid");

        for issue in &validation.issues {
            match issue {
                MeshIssue::IsolatedVertex { .. } => panic!("has isolated vertices"),
                MeshIssue::BrokenConnectivity { .. } => panic!("broken connectivity: {}", issue),
                MeshIssue::NonManifoldVertex { .. } => panic!("non-manifold: {}", issue),
                _ => {}
            }
        }

        // Fitted to the display size and sitting on the floor.
        assert!(
            (report.dimensions.max_element() - DISPLAY_SIZE).abs() < 0.01,
            "largest dimension should be ~{DISPLAY_SIZE}, got {}",
            report.dimensions.max_element()
        );
        assert!(
            report.bounding_box.0.y.abs() < 0.01,
            "base should rest on the floor, got min y {}",
            report.bounding_box.0.y
        );

        // Real-world proportions should survive the uniform scale: length is the
        // longest axis and the footprint reads as a C (wider than one wing).
        assert!(
            report.dimensions.z >= report.dimensions.x,
            "length (z) should be the footprint's long axis"
        );
        assert!(
            report.dimensions.x > params.wing_thickness / params.length * DISPLAY_SIZE * 1.5,
            "footprint should be C-shaped, not a single wing"
        );
    }

    #[test]
    #[cfg(feature = "preview")]
    fn courtyard_preview() {
        let mesh = generate_courtyard(&CourtyardParameters::default()).unwrap();
        eprintln!("{}", mesh.describe());
        let opts = smesh::smesh::preview::PreviewOptions::default()
            .with_size(1024, 1024)
            .with_wireframe();
        let paths = mesh.save_preview_with_options(&opts, "/tmp").unwrap();
        eprintln!("Saved previews: {:?}", paths);
    }
}
