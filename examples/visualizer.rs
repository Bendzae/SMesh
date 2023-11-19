use bevy::prelude::*;
use bevy_inspector_egui::egui::Order::Debug;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;
use smesh::mesh_query::EvalMeshQuery;
use smesh::smesh::{FaceId, HalfedgeId, SMesh, VertexId};

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

    // let test_he = mesh.q(v0).halfedge().id().unwrap();
    let test_he = mesh.q(v0).halfedge().cw_rotated_neighbour().id().unwrap();
    commands.spawn((
        DebugRenderSMesh {
            mesh,
            selection: Selection::Halfedge(test_he),
        },
        TransformBundle::default(),
    ));

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
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
            let v_src = he.vertex;
            let v_dst = mesh.q(opposite).vert().id().unwrap();
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
            WorldInspectorPlugin::default(),
        ))
        .add_systems(Startup, init_system)
        .add_systems(Update, debug_draw_smesh)
        .run();
}
