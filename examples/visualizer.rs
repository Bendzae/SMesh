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

    // let test_he = mesh.q(v0).halfedge().id().unwrap();
    let test_he = mesh.q(v0).halfedge_to(v1).id().unwrap();
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

fn debug_draw_smesh(q_smesh: Query<(&DebugRenderSMesh, &Transform)>, mut gizmos: Gizmos) {
    for (debug_smesh, t) in &q_smesh {
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
            let opposite = mesh.q(he_id).opposite().id().unwrap();
            let v_src = mesh.q(he_id).src_vert().id().unwrap();
            let v_dst = mesh.q(he_id).dst_vert().id().unwrap();
            let v_src_pos = t.transform_point(*mesh.positions.get(v_src).unwrap());
            let v_dst_pos = t.transform_point(*mesh.positions.get(v_dst).unwrap());
            let color = if debug_smesh.selection == Selection::Halfedge(he_id) {
                Color::ORANGE_RED
            } else {
                Color::TURQUOISE
            };
            draw_halfedge(&mut gizmos, v_src_pos, v_dst_pos, color);
            let color = if debug_smesh.selection == Selection::Halfedge(opposite) {
                Color::ORANGE_RED
            } else {
                Color::TURQUOISE
            };
            draw_halfedge(&mut gizmos, v_dst_pos, v_src_pos, color);
        }
    }
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
    for mut debug_smesh in q_smesh.iter_mut() {
        match debug_smesh.selection {
            Selection::Vertex(id) => {
                if input.just_pressed(KeyCode::N) {
                    debug_smesh.selection =
                        Selection::Halfedge(debug_smesh.mesh.q(id).halfedge().id().unwrap());
                }
            }
            Selection::Halfedge(id) => {
                if input.just_pressed(KeyCode::N) {
                    debug_smesh.selection =
                        Selection::Halfedge(debug_smesh.mesh.q(id).next().id().unwrap());
                }
                if input.just_pressed(KeyCode::P) {
                    debug_smesh.selection =
                        Selection::Halfedge(debug_smesh.mesh.q(id).prev().id().unwrap());
                }
                if input.just_pressed(KeyCode::O) {
                    debug_smesh.selection =
                        Selection::Halfedge(debug_smesh.mesh.q(id).opposite().id().unwrap());
                }
                if input.just_pressed(KeyCode::R) {
                    debug_smesh.selection = Selection::Halfedge(
                        debug_smesh.mesh.q(id).cw_rotated_neighbour().id().unwrap(),
                    );
                }
                if input.just_pressed(KeyCode::V) {
                    debug_smesh.selection =
                        Selection::Vertex(debug_smesh.mesh.q(id).vert().id().unwrap());
                }
                if input.just_pressed(KeyCode::S) {
                    let m = &mut debug_smesh.mesh;
                    let v0 = m.q(id).src_vert().id().unwrap();
                    let v1 = m.q(id).dst_vert().id().unwrap();
                    let pos = (m.positions[v0] + m.positions[v1]) / 2.0;
                    let v = debug_smesh.mesh.add_vertex(pos);
                    let he = debug_smesh.mesh.insert_vertex(id, v);
                    match he {
                        Ok(he) => {
                            debug_smesh.selection = Selection::Halfedge(he);
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
                debug_draw_smesh,
                change_selection_system,
                selection_log_system,
            ),
        )
        .run();
}
