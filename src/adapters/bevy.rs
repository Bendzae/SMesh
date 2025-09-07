use attribute::CustomAttributeMapOps;
use bevy::{
    app::{Plugin, Update},
    color::{
        palettes::css::{GREEN, ORANGE_RED, TURQUOISE, YELLOW},
        Srgba,
    },
    input::ButtonInput,
    log::{info, warn},
    math::Isometry3d,
    prelude::*,
    render::{
        mesh::{Indices, Mesh, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    text::TextFont,
    transform::components::Transform,
    ui::{FlexDirection, Node, UiRect, Val},
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

/// Classical indexed mesh representation
#[derive(Clone, Debug)]
pub struct VertexIndexUvBuffers {
    /// Vertex positions, one per vertex.
    pub positions: Vec<Vec3>,
    /// Vertex normals, one per vertex.
    pub normals: Vec<Vec3>,
    /// UV coordinated, one per vertex
    pub uvs: Vec<Vec2>,
    /// Indices: 3*N where N is the number of triangles. Indices point to
    /// elements of `positions` and `normals`.
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

            let v1 = vertices[0];

            for (&v2, &v3) in vertices[1..].iter().tuple_windows() {
                positions.push(self.positions[v1]);
                positions.push(self.positions[v2]);
                positions.push(self.positions[v3]);

                if let Some(mesh_uvs) = self.uvs.as_ref() {
                    uvs.push(mesh_uvs[v1]);
                    uvs.push(mesh_uvs[v2]);
                    uvs.push(mesh_uvs[v3]);
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Selection {
    Vertex(VertexId),
    Halfedge(HalfedgeId),
    Face(FaceId),
    None,
}

#[derive(Component)]
pub struct DebugRenderSMesh {
    pub mesh: SMesh,
    pub selection: Selection,
    pub visible: bool,
}

#[derive(Component)]
struct UiTag;

fn debug_draw_smesh_system(q_smesh: Query<(&DebugRenderSMesh, &Transform)>, mut gizmos: Gizmos) {
    for (debug_smesh, t) in &q_smesh {
        if debug_smesh.visible {
            debug_draw_smesh(debug_smesh, t, &mut gizmos)
                .unwrap_or_else(|e| warn!("Error while drawing mesh: {:?}", e));
        }
    }
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
        gizmos.sphere(Isometry3d::from_translation(v_pos), 0.08, color);
        gizmos.arrow(v_pos, v_pos + v_id.normal(mesh)? * 0.2, color);
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
        let relative_center = vertex_positions.fold(Vec3::ZERO, |acc, pos| (acc + pos)) / count;
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
    let offset = dir.cross(normal) * 0.05;
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
            d.visible = !d.visible;
        }
        if !d.visible {
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
                builder.spawn((
                    Text::new("H: hide/show debug gizmos"),
                    TextFont::from_font_size(32.0),
                ));
                if d.visible {
                    for s in &values {
                        builder.spawn((Text::new(*s), TextFont::from_font_size(32.0)));
                    }
                }
            });
    }
}
