//! Bevy adapter: render and debug-draw SMesh inside a Bevy app.
//!
//! Available when the `bevy_adapter` feature is enabled (the default).
//!
//! What this module provides:
//!
//! - `impl From<SMesh> for bevy::Mesh` — drop any SMesh straight into a
//!   `Mesh3d` handle by way of `Assets<Mesh>`.
//! - [`SMeshDebugDrawPlugin`] — renders wireframes and selection gizmos for
//!   any entity with a [`DebugRenderSMesh`] component; useful during
//!   procedural iteration.
//! - [`Selection`] / [`DebugDrawMode`] — per-entity controls for the plugin.
//! - Triangulated [`VertexIndexUvBuffers`] as the conversion intermediate.
//!
//! Typical usage:
//!
//! ```no_run
//! # use bevy::prelude::*;
//! # use smesh::prelude::*;
//! # use smesh::adapters::bevy::*;
//! # fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>,
//! #          mut mats: ResMut<Assets<StandardMaterial>>) {
//! # let smesh: SMesh = unimplemented!();
//! commands.spawn((
//!     Mesh3d(meshes.add(Mesh::from(smesh.clone()))),
//!     MeshMaterial3d(mats.add(StandardMaterial::default())),
//!     DebugRenderSMesh { mesh: smesh, selection: Selection::None, draw_mode: DebugDrawMode::Wireframe },
//! ));
//! # }
//! ```

use attribute::CustomAttributeMapOps;
use bevy::{
    app::{Plugin, Update}, asset::RenderAssetUsages, color::{
        palettes::css::{GREEN, ORANGE_RED, TURQUOISE, YELLOW},
        Srgba,
    }, input::ButtonInput, log::{info, warn}, math::Isometry3d, mesh::{Indices, PrimitiveTopology}, prelude::*, text::TextFont, transform::components::Transform, ui::{FlexDirection, Node, UiRect, Val}
};
use glam::{bool, Vec2, Vec3};
use itertools::Itertools;

use crate::prelude::*;

impl From<SMesh> for Mesh {
    fn from(smesh: SMesh) -> Self {
        let buffers = smesh.to_buffers().unwrap();
        let vertex_count = buffers.positions.len();

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, buffers.positions)
        .with_inserted_indices(Indices::U32(buffers.indices));

        if buffers.uvs.len() == vertex_count {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, buffers.uvs);
        }
        if buffers.normals.len() == vertex_count {
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, buffers.normals);
        }
        mesh
    }
}

/// Triangulated vertex buffer form of a mesh — position/normal/UV streams
/// plus an index list suitable for uploading to a GPU.
///
/// Used as the conversion intermediate when producing a `bevy::Mesh` from
/// an [`SMesh`]. Vertices are duplicated per-face-corner (so seams in UVs
/// and normals are preserved), and the index list is just
/// `[0, 1, 2, ..., positions.len() - 1]`.
#[derive(Clone, Debug)]
pub struct VertexIndexUvBuffers {
    /// World-space vertex positions, one entry per triangle corner.
    pub positions: Vec<Vec3>,
    /// Corresponding vertex normals. May be empty if the source mesh has no
    /// face normals populated.
    pub normals: Vec<Vec3>,
    /// Corresponding UVs. May be empty if the source mesh has no UVs.
    pub uvs: Vec<Vec2>,
    /// Triangle indices (length = `3 * triangle_count`). Each element indexes
    /// [`positions`](Self::positions) / [`normals`](Self::normals) /
    /// [`uvs`](Self::uvs).
    pub indices: Vec<u32>,
}

impl SMesh {
    fn to_buffers(&self) -> Result<VertexIndexUvBuffers, SMeshError> {
        let mut positions = vec![];
        let mut uvs = vec![];
        let mut normals = vec![];

        for face_id in self.faces() {
            let face_normal = self.face_normals.as_ref().map(|n| n[face_id]);
            let vertices: Vec<VertexId> = face_id.vertices(self).collect();
            let halfedges: Vec<HalfedgeId> = face_id.halfedges(self).collect();

            let v1 = vertices[0];
            let he1 = halfedges[0];

            for (i, (&v2, &v3)) in vertices[1..].iter().tuple_windows().enumerate() {
                let he2 = halfedges[i + 1];
                let he3 = halfedges[i + 2];

                // Always duplicate vertices for each triangle
                positions.push(self.positions[v1]);
                positions.push(self.positions[v2]);
                positions.push(self.positions[v3]);

                // Try per-halfedge UVs first, fall back to per-vertex UVs
                if let Some(he_uvs) = self.halfedge_uvs.as_ref() {
                    // Check if all three halfedges have UVs
                    if let (Some(&uv1), Some(&uv2), Some(&uv3)) =
                        (he_uvs.get(he1), he_uvs.get(he2), he_uvs.get(he3))
                    {
                        uvs.push(uv1);
                        uvs.push(uv2);
                        uvs.push(uv3);
                    } else if let Some(vertex_uvs) = self.vertex_uvs.as_ref() {
                        // Fall back to per-vertex UVs
                        uvs.push(vertex_uvs[v1]);
                        uvs.push(vertex_uvs[v2]);
                        uvs.push(vertex_uvs[v3]);
                    }
                } else if let Some(vertex_uvs) = self.vertex_uvs.as_ref() {
                    // Use per-vertex UVs
                    uvs.push(vertex_uvs[v1]);
                    uvs.push(vertex_uvs[v2]);
                    uvs.push(vertex_uvs[v3]);
                }

                if let Some(normal) = face_normal {
                    normals.push(normal);
                    normals.push(normal);
                    normals.push(normal);
                }
            }
        }

        Ok(VertexIndexUvBuffers {
            indices: (0u32..positions.len() as u32).collect(),
            positions,
            uvs,
            normals,
        })
    }
}

/// Bevy plugin that renders wireframes, selection gizmos, and an info UI
/// for any entity carrying a [`DebugRenderSMesh`] component.
///
/// Keyboard bindings (active when a selection exists):
/// - `N` — move selection to a neighbour
/// - `V` / `E` / `F` — change the selected element kind
/// - `M` — cycle draw mode
pub struct SMeshDebugDrawPlugin;

impl Plugin for SMeshDebugDrawPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            Update,
            (
                debug_draw_smesh_system,
                change_selection_system,
                selection_log_system,
                update_ui_system,
            ),
        );
    }
}

/// Currently-selected element for debug rendering / interactive inspection.
///
/// One of a vertex, halfedge, face, or nothing. Use `Selection::None` on
/// entities that should still be debug-drawn but have no active selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Selection {
    /// A single vertex.
    Vertex(VertexId),
    /// A single halfedge.
    Halfedge(HalfedgeId),
    /// A single face.
    Face(FaceId),
    /// No active selection.
    None,
}

/// How much detail [`SMeshDebugDrawPlugin`] should draw for an entity.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DebugDrawMode {
    /// Disable debug drawing entirely.
    #[default]
    Off,
    /// Draw only the wireframe.
    Wireframe,
    /// Wireframe plus selection gizmos and supplementary helpers.
    Full,
}

/// Component that activates [`SMeshDebugDrawPlugin`] for an entity.
///
/// Carry a clone of the displayed mesh alongside the rendered
/// `Mesh3d`; the plugin uses `mesh` as the source of truth for wireframe /
/// selection visualisation.
#[derive(Component)]
pub struct DebugRenderSMesh {
    /// Clone of the mesh to visualise. Keep in sync with the GPU mesh if
    /// you're editing in place.
    pub mesh: SMesh,
    /// Currently highlighted element, or [`Selection::None`].
    pub selection: Selection,
    /// How much to draw.
    pub draw_mode: DebugDrawMode,
}

#[derive(Component)]
struct UiTag;

fn debug_draw_smesh_system(q_smesh: Query<(&DebugRenderSMesh, &Transform)>, mut gizmos: Gizmos) {
    for (debug_smesh, t) in &q_smesh {
        match debug_smesh.draw_mode {
            DebugDrawMode::Off => {}
            DebugDrawMode::Wireframe => {
                debug_draw_wireframe(debug_smesh, t, &mut gizmos)
                    .unwrap_or_else(|e| warn!("Error while drawing wireframe: {:?}", e));
            }
            DebugDrawMode::Full => {
                debug_draw_smesh(debug_smesh, t, &mut gizmos)
                    .unwrap_or_else(|e| warn!("Error while drawing mesh: {:?}", e));
            }
        }
    }
}

fn debug_draw_wireframe(
    debug_smesh: &DebugRenderSMesh,
    t: &Transform,
    gizmos: &mut Gizmos,
) -> SMeshResult<()> {
    let mesh = &debug_smesh.mesh;
    use std::collections::HashSet;
    let mut drawn_edges: HashSet<(VertexId, VertexId)> = HashSet::new();

    for he_id in mesh.halfedges() {
        let v_src = he_id.src_vert().run(mesh)?;
        let v_dst = he_id.dst_vert().run(mesh)?;
        let key = if v_src < v_dst { (v_src, v_dst) } else { (v_dst, v_src) };
        if !drawn_edges.insert(key) {
            continue;
        }
        let p0 = t.transform_point(*mesh.positions.get(v_src).unwrap());
        let p1 = t.transform_point(*mesh.positions.get(v_dst).unwrap());
        let is_boundary = he_id.is_boundary(mesh)
            || he_id.opposite().run(mesh).map(|o| o.is_boundary(mesh)).unwrap_or(true);
        let color = if is_boundary { ORANGE_RED } else { GREEN };
        gizmos.line(p0, p1, color);
    }
    Ok(())
}

fn debug_draw_smesh(
    debug_smesh: &DebugRenderSMesh,
    t: &Transform,
    gizmos: &mut Gizmos,
) -> SMeshResult<()> {
    let mesh = &debug_smesh.mesh;
    // Verts
    for v_id in mesh.vertices() {
        let v_pos = t.transform_point(*mesh.positions.get(v_id).unwrap());
        let color = if debug_smesh.selection == Selection::Vertex(v_id) {
            ORANGE_RED
        } else {
            GREEN
        };
        gizmos.sphere(Isometry3d::from_translation(v_pos), 0.035, color);
        gizmos.arrow(v_pos, v_pos + v_id.normal(mesh)? * 0.07, color);
    }
    // Halfedges
    for he_id in mesh.halfedges() {
        let he = he_id;
        let v_src = he.src_vert().run(mesh)?;
        let v_dst = he.dst_vert().run(mesh)?;
        let v_src_pos = t.transform_point(*mesh.positions.get(v_src).unwrap());
        let v_dst_pos = t.transform_point(*mesh.positions.get(v_dst).unwrap());
        let color = if debug_smesh.selection == Selection::Halfedge(he_id) {
            ORANGE_RED
        } else {
            match mesh.attribute::<HalfedgeId>("debug") {
                Some(debug) => {
                    let t: Option<String> = debug.get(he_id);
                    if let Some(_color) = t {
                        YELLOW
                    } else {
                        TURQUOISE
                    }
                }
                None => TURQUOISE,
            }
        };
        let mut edge_normal = ((v_src.normal(mesh)? + v_dst.normal(mesh)?) / 2.0).normalize();
        if edge_normal.is_nan() {
            edge_normal = Vec3::Y;
        }
        draw_halfedge(gizmos, v_src_pos, v_dst_pos, edge_normal, color);
    }
    // Faces
    for face_id in mesh.faces() {
        let vertex_positions = face_id
            .vertices(mesh)
            .map(|v| *mesh.positions.get(v).unwrap());
        let count = vertex_positions.clone().count() as f32;
        let relative_center = vertex_positions.fold(Vec3::ZERO, |acc, pos| acc + pos) / count;
        let center = t.transform_point(relative_center);
        let color = if debug_smesh.selection == Selection::Face(face_id) {
            ORANGE_RED
        } else {
            YELLOW
        };
        gizmos.sphere(Isometry3d::from_translation(center), 0.02, color);
        gizmos.arrow(center, center + face_id.normal(mesh)? * 0.3, color);
    }
    Ok(())
}

fn draw_halfedge(gizmos: &mut Gizmos, v0: Vec3, v1: Vec3, normal: Vec3, color: Srgba) {
    let dir = (v1 - v0).normalize();
    let offset = dir.cross(normal) * 0.02;
    let line_start = v0 - offset + dir * 0.1;
    let line_end = v1 - offset - dir * 0.1;
    gizmos.line(line_start, line_end, color);
    gizmos.line(line_end - dir * 0.05 - offset * 0.5, line_end, color);
    gizmos.line(line_end - dir * 0.05 + offset * 0.5, line_end, color);
}

fn change_selection_system(
    input: Res<ButtonInput<KeyCode>>,
    mut q_smesh: Query<&mut DebugRenderSMesh>,
) {
    change_selection_inner(&input, &mut q_smesh)
        .unwrap_or_else(|e| warn!("Error while trying to perform mesh operation: {:?}", e));
}
fn change_selection_inner(
    input: &Res<ButtonInput<KeyCode>>,
    q_smesh: &mut Query<&mut DebugRenderSMesh>,
) -> SMeshResult<()> {
    for mut d in q_smesh.iter_mut() {
        if input.just_pressed(KeyCode::KeyH) {
            d.draw_mode = match d.draw_mode {
                DebugDrawMode::Off => DebugDrawMode::Wireframe,
                DebugDrawMode::Wireframe => DebugDrawMode::Full,
                DebugDrawMode::Full => DebugDrawMode::Off,
            };
        }
        if d.draw_mode == DebugDrawMode::Off {
            continue;
        }
        match d.selection {
            Selection::Vertex(id) => {
                if input.just_pressed(KeyCode::KeyN) {
                    d.selection = Selection::Halfedge(id.halfedge().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::KeyD) {
                    d.mesh.delete_vertex(id)?;
                    d.selection = Selection::Vertex(d.mesh.vertices().next().unwrap());
                }
            }
            Selection::Halfedge(id) => {
                if input.just_pressed(KeyCode::KeyN) {
                    d.selection = Selection::Halfedge(id.next().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::KeyP) {
                    d.selection = Selection::Halfedge(id.prev().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::KeyO) {
                    d.selection = Selection::Halfedge(id.opposite().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::KeyR) {
                    d.selection = Selection::Halfedge(id.cw_rotated_neighbour().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::KeyV) {
                    d.selection = Selection::Vertex(id.src_vert().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::KeyF) {
                    d.selection = Selection::Face(id.face().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::KeyS) {
                    let mesh = &mut d.mesh;
                    let v0 = id.src_vert().run(mesh)?;
                    let v1 = id.dst_vert().run(mesh)?;
                    let pos = (mesh.positions[v0] + mesh.positions[v1]) / 2.0;
                    let v = mesh.add_vertex(pos);
                    let he = mesh.insert_vertex(id, v);
                    mesh.recalculate_normals()?;
                    match he {
                        Ok(he) => {
                            d.selection = Selection::Halfedge(he);
                        }
                        Err(e) => {
                            warn!("{:?}", e)
                        }
                    }
                }
                if input.just_pressed(KeyCode::KeyD) {
                    let v = id.src_vert().run(&d.mesh)?;
                    d.mesh.delete_only_edge(id)?;
                    d.selection = Selection::Vertex(v);
                }
            }
            Selection::Face(id) => {
                if input.just_pressed(KeyCode::KeyD) {
                    d.mesh.delete_face(id)?;
                    d.selection = Selection::Vertex(d.mesh.vertices().next().unwrap());
                }
                if input.just_pressed(KeyCode::KeyN) {
                    d.selection = Selection::Halfedge(id.halfedge().run(&d.mesh)?);
                }
            }
            Selection::None => {}
        }
    }
    Ok(())
}

fn selection_log_system(q_sel: Query<&DebugRenderSMesh, Changed<DebugRenderSMesh>>) {
    for d in q_sel.iter() {
        info!("Selected: {:?}", d.selection);
    }
}

fn update_ui_system(
    q_sel: Query<&DebugRenderSMesh, Changed<DebugRenderSMesh>>,
    q_ui: Query<Entity, With<UiTag>>,
    mut commands: Commands,
) {
    for d in q_sel.iter() {
        let values: Vec<&str> = match d.selection {
            Selection::Vertex(_) => {
                vec!["N: outgoing halfedge", "D: delete vertex"]
            }
            Selection::Halfedge(_) => {
                vec![
                    "N: next halfedge",
                    "P: previous halfedge",
                    "O: opposite halfedge",
                    "R: cw rotated halfedge",
                    "V: Source Vertex",
                    "D: Delete edge",
                    "S: Split edge",
                ]
            }
            Selection::Face(_) => {
                vec!["N: associated halfedge", "D: Delete face"]
            }
            Selection::None => {
                vec![]
            }
        };
        if let Ok(e) = q_ui.single() {
            commands.entity(e).despawn();
        }

        commands
            .spawn((
                UiTag,
                Node {
                    flex_direction: FlexDirection::Column,
                    column_gap: Val::Px(5.0),
                    padding: UiRect::all(Val::Px(5.0)),
                    ..default()
                },
            ))
            .with_children(|builder| {
                let mode_label = match d.draw_mode {
                    DebugDrawMode::Off => "H: debug gizmos [off]",
                    DebugDrawMode::Wireframe => "H: debug gizmos [wireframe]",
                    DebugDrawMode::Full => "H: debug gizmos [full]",
                };
                builder.spawn((
                    Text::new(mode_label),
                    TextFont::from_font_size(32.0),
                ));
                if d.draw_mode == DebugDrawMode::Full {
                    for s in &values {
                        builder.spawn((Text::new(*s), TextFont::from_font_size(32.0)));
                    }
                }
            });
    }
}

// === Showcase Plugin ===

/// Plugin that sets up a reusable showcase scene for displaying meshes.
///
/// Provides:
/// - Dark background
/// - Warm ambient light
/// - Three-point lighting (key, fill, rim)
/// - Dark wood ground plane
/// - `SMeshDebugDrawPlugin`
///
/// Examples should spawn their own camera (with orbit controls etc.).
///
/// # Example
/// ```ignore
/// App::new()
///     .add_plugins((DefaultPlugins, ShowcasePlugin))
///     .add_systems(Startup, |mut commands: Commands, ...| {
///         commands.spawn((Camera3d::default(), Transform::from_xyz(1.0, 1.0, 1.0)));
///         commands.spawn((Mesh3d(mesh_handle), MeshMaterial3d(material_handle)));
///     })
///     .run();
/// ```
/// Marker component for showcase cameras.
///
/// Provides Camera3d, HDR, SSAO (Ultra), TAA, and MSAA off.
/// Spawn with a `PanOrbitCamera` to control the view:
///
/// ```ignore
/// commands.spawn((
///     ShowcaseCamera,
///     PanOrbitCamera { focus: vec3(0.0, 0.5, 0.0), radius: Some(2.0), .. },
/// ));
/// ```
#[derive(Component)]
#[require(
    Camera3d,
    bevy::render::view::Hdr,
    bevy::anti_alias::taa::TemporalAntiAliasing,
)]
pub struct ShowcaseCamera;

impl ShowcaseCamera {
    /// Returns the full set of components needed for a showcase camera.
    pub fn bundle() -> impl Bundle {
        (
            ShowcaseCamera,
            Msaa::Off,
            bevy::pbr::ScreenSpaceAmbientOcclusion {
                quality_level: bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
                ..Default::default()
            },
        )
    }
}

pub struct ShowcasePlugin;

impl Plugin for ShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::BLACK))
            .insert_resource(GlobalAmbientLight {
                color: Color::srgb(0.95, 0.90, 0.80),
                brightness: 150.0,
                ..default()
            })
            .add_plugins(SMeshDebugDrawPlugin)
            .add_systems(Startup, showcase_setup);
    }
}

fn showcase_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    use std::f32::consts::PI;

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.12, 0.08),
            perceptual_roughness: 0.85,
            reflectance: 0.3,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(10.0)),
    ));

    // Key light — warm directional from upper-left
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY * 1.5,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.95, 0.85),
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI / 4.0, -PI / 3.5)),
    ));

    // Fill light — cooler point light from the right
    commands.spawn((
        PointLight {
            intensity: 150_000.0,
            color: Color::srgb(0.85, 0.9, 1.0),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::new(2.5, 2.0, 1.5)),
    ));

    // Rim light — warm accent from behind
    commands.spawn((
        PointLight {
            intensity: 100_000.0,
            color: Color::srgb(1.0, 0.85, 0.6),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(Vec3::new(-1.0, 1.5, -2.0)),
    ));

}
