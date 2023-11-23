use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;
use smesh::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Selection {
    Vertex(VertexId),
    Halfedge(HalfedgeId),
    Face(FaceId),
    None,
}

#[derive(Component)]
struct DebugRenderSMesh {
    pub mesh: SMesh,
    pub selection: Selection,
}

fn init_system(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    // Spawn SMesh
    let mut mesh = SMesh::new();
    let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
    let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
    let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
    let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));

    let v4 = mesh.add_vertex(vec3(0.0, -2.0, 0.0));
    let _ = mesh.add_face(vec![v0, v1, v2, v3]);
    let _ = mesh.add_face(vec![v0, v4, v1]);

    // let v4 = mesh.add_vertex(vec3(1.0, 2.0, 0.0));
    // mesh.add_edge(v2, v4);

    let test_he = v0.halfedge_to(v1).run(&mesh).unwrap();
    commands.spawn((
        DebugRenderSMesh {
            mesh,
            selection: Selection::Halfedge(test_he),
        },
        TransformBundle::default(),
    ));

    // UI
    let style = TextStyle {
        font_size: 32.0,
        ..default()
    };

    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                column_gap: Val::Px(5.0),
                padding: UiRect::all(Val::Px(5.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for s in [
                "N: next halfedge",
                "P: previous halfedge",
                "O: opposite halfedge",
                "R: cw rotated halfedge",
                "V: Source Vertex",
            ]
            .iter()
            {
                parent.spawn(TextBundle::from_section(*s, style.clone()));
            }
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

fn debug_draw_smesh_system(q_smesh: Query<(&DebugRenderSMesh, &Transform)>, mut gizmos: Gizmos) {
    for (debug_smesh, t) in &q_smesh {
        debug_draw_smesh(debug_smesh, t, &mut gizmos)
            .unwrap_or_else(|e| warn!("Error while drawing mesh: {:?}", e));
    }
}
fn debug_draw_smesh(
    debug_smesh: &DebugRenderSMesh,
    t: &Transform,
    mut gizmos: &mut Gizmos,
) -> SMeshResult<()> {
    let mesh = &debug_smesh.mesh;

    // Verts
    for (v_id, v) in mesh.vertices().iter() {
        let v_pos = t.transform_point(*mesh.positions.get(v_id).unwrap());
        let color = if debug_smesh.selection == Selection::Vertex(v_id) {
            Color::ORANGE_RED
        } else {
            Color::GREEN
        };
        gizmos.sphere(v_pos, Quat::IDENTITY, 0.08, color);
    }
    // Halfedges
    for (he_id, he) in mesh.halfedges().iter() {
        let he = he_id;
        let opposite = he.opposite().run(mesh);
        let v_src = he.src_vert().run(mesh);
        let v_dst = he.dst_vert().run(mesh);
        let v_src_pos = t.transform_point(*mesh.positions.get(v_src?).unwrap());
        let v_dst_pos = t.transform_point(*mesh.positions.get(v_dst?).unwrap());
        let color = if debug_smesh.selection == Selection::Halfedge(he_id) {
            Color::ORANGE_RED
        } else {
            Color::TURQUOISE
        };
        draw_halfedge(&mut gizmos, v_src_pos, v_dst_pos, color);
        let color = if debug_smesh.selection == Selection::Halfedge(opposite?) {
            Color::ORANGE_RED
        } else {
            Color::TURQUOISE
        };
        draw_halfedge(&mut gizmos, v_dst_pos, v_src_pos, color);
    }
    Ok(())
}

fn draw_halfedge(gizmos: &mut Gizmos, v0: Vec3, v1: Vec3, color: Color) {
    let dir = (v1 - v0).normalize();
    let normal = Vec3::Z; // TODO
    let offset = dir.cross(normal) * 0.05;
    let line_start = v0 - offset + dir * 0.1;
    let line_end = v1 - offset - dir * 0.1;
    gizmos.line(line_start, line_end, color);
    gizmos.line(line_end - dir * 0.05 - offset * 0.5, line_end, color);
    gizmos.line(line_end - dir * 0.05 + offset * 0.5, line_end, color);
}

fn change_selection_system(input: Res<Input<KeyCode>>, mut q_smesh: Query<&mut DebugRenderSMesh>) {
    change_selection_inner(&input, &mut q_smesh)
        .unwrap_or_else(|e| warn!("Error while trying to perform mesh operation: {:?}", e));
}
fn change_selection_inner(
    input: &Res<Input<KeyCode>>,
    q_smesh: &mut Query<&mut DebugRenderSMesh>,
) -> SMeshResult<()> {
    for mut d in q_smesh.iter_mut() {
        match d.selection {
            Selection::Vertex(id) => {
                if input.just_pressed(KeyCode::N) {
                    d.selection = Selection::Halfedge(id.halfedge().run(&d.mesh)?);
                }
            }
            Selection::Halfedge(id) => {
                if input.just_pressed(KeyCode::N) {
                    d.selection = Selection::Halfedge(id.next().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::P) {
                    d.selection = Selection::Halfedge(id.prev().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::O) {
                    d.selection = Selection::Halfedge(id.opposite().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::R) {
                    d.selection = Selection::Halfedge(id.cw_rotated_neighbour().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::V) {
                    d.selection = Selection::Vertex(id.vert().run(&d.mesh)?);
                }
                if input.just_pressed(KeyCode::S) {
                    let mesh = &mut d.mesh;
                    let v0 = id.src_vert().run(mesh)?;
                    let v1 = id.dst_vert().run(mesh)?;
                    let pos = (mesh.positions[v0] + mesh.positions[v1]) / 2.0;
                    let v = mesh.add_vertex(pos);
                    let he = mesh.insert_vertex(id, v);
                    match he {
                        Ok(he) => {
                            d.selection = Selection::Halfedge(he);
                        }
                        Err(e) => {
                            error!("{:?}", e)
                        }
                    }
                }
            }
            Selection::Face(_) => {}
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

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 0.3,
        })
        .insert_resource(Msaa::Sample4)
        .add_plugins((
            DefaultPlugins,
            PanOrbitCameraPlugin,
            // WorldInspectorPlugin::default(),
        ))
        .add_systems(Startup, init_system)
        .add_systems(
            Update,
            (
                debug_draw_smesh_system,
                change_selection_system,
                selection_log_system,
            ),
        )
        .run();
}
