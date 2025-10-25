use bevy::color::palettes::css::WHITE;
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use smesh::prelude::*;
use smesh::smesh::primitives::{Cube, Icosphere, Primitive};
use smesh::smesh::uv_operations::ProjectionAxis;

#[derive(Component)]
struct UVMesh {
    primitive_type: PrimitiveType,
}

#[derive(Resource)]
struct UVDemoState {
    current_method: usize,
    methods: Vec<(&'static str, UVMethod)>,
}

enum UVMethod {
    PlanarZ,
    PlanarY,
    CylindricalY,
    Spherical,
    Xatlas,
}

#[derive(Clone, Copy)]
enum PrimitiveType {
    Cube,
    Sphere,
    Cylinder,
}

fn create_primitive(primitive_type: &PrimitiveType) -> SMesh {
    match primitive_type {
        PrimitiveType::Cube => Cube {
            subdivision: glam::U16Vec3::new(4, 4, 4),
        }
        .generate()
        .unwrap()
        .0,
        PrimitiveType::Sphere => Icosphere { subdivisions: 3 }.generate().unwrap().0,
        PrimitiveType::Cylinder => {
            let segments = 32;
            let height = 1.5;
            let radius = 0.5;
            
            let mut mesh = SMesh::new();
            
            let mut top_verts = Vec::new();
            let mut bottom_verts = Vec::new();
            
            for i in 0..segments {
                let angle = (i as f32 / segments as f32) * std::f32::consts::PI * 2.0;
                let x = angle.cos() * radius;
                let z = angle.sin() * radius;
                
                bottom_verts.push(mesh.add_vertex(glam::Vec3::new(x, -height / 2.0, z)));
                top_verts.push(mesh.add_vertex(glam::Vec3::new(x, height / 2.0, z)));
            }
            
            for i in 0..segments {
                let next = (i + 1) % segments;
                mesh.make_face(vec![
                    bottom_verts[i],
                    top_verts[i],
                    top_verts[next],
                    bottom_verts[next],
                ]).unwrap();
            }
            
            mesh.make_face(bottom_verts.clone()).unwrap();
            mesh.make_face(top_verts.iter().rev().copied().collect()).unwrap();
            
            mesh.recalculate_normals().unwrap();
            
            mesh
        }
    }
}



fn create_checkerboard_texture(images: &mut Assets<Image>) -> Handle<Image> {
    let size = 256;
    let mut data = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let checker = ((x / 32) + (y / 32)) % 2 == 0;
            let idx = ((y * size + x) * 4) as usize;
            let color = if checker { 255 } else { 50 };
            data[idx] = color;
            data[idx + 1] = color;
            data[idx + 2] = color;
            data[idx + 3] = 255;
        }
    }

    let image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );

    images.add(image)
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let texture = create_checkerboard_texture(&mut images);

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        ..default()
    });

    let primitives = [
        (PrimitiveType::Cube, -2.5),
        (PrimitiveType::Sphere, 0.0),
        (PrimitiveType::Cylinder, 2.5),
    ];

    for (primitive_type, x_pos) in primitives {
        let mut smesh = create_primitive(&primitive_type);
        smesh.planar_project_uvs(ProjectionAxis::Z).unwrap();

        let mesh: Mesh = smesh.into();

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x_pos, 0.0, 0.0),
            UVMesh { primitive_type },
        ));
    }

    let methods = vec![
        ("Planar Z", UVMethod::PlanarZ),
        ("Planar Y", UVMethod::PlanarY),
        ("Cylindrical Y", UVMethod::CylindricalY),
        ("Spherical", UVMethod::Spherical),
        ("XAtlas Auto", UVMethod::Xatlas),
    ];

    commands.insert_resource(UVDemoState {
        current_method: 0,
        methods,
    });

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        PanOrbitCamera::default(),
    ));

    commands.spawn((
        Text::new("Press SPACE to cycle UV unwrap methods\n\nCube          Sphere          Cylinder\nCurrent Method: Planar Z"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        TextColor(WHITE.into()),
        TextFont {
            font_size: 20.0,
            ..default()
        },
    ));
}

fn update_uv_method(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<UVDemoState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(&mut Mesh3d, &UVMesh)>,
    mut text_query: Query<&mut Text>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        state.current_method = (state.current_method + 1) % state.methods.len();

        let (method_name, method) = &state.methods[state.current_method];

        for (mut mesh_handle, uv_mesh) in query.iter_mut() {
            let mut smesh = create_primitive(&uv_mesh.primitive_type);

            match method {
                UVMethod::PlanarZ => {
                    smesh.planar_project_uvs(ProjectionAxis::Z).unwrap();
                }
                UVMethod::PlanarY => {
                    smesh.planar_project_uvs(ProjectionAxis::Y).unwrap();
                }
                UVMethod::CylindricalY => {
                    smesh.cylindrical_project_uvs(ProjectionAxis::Y).unwrap();
                }
                UVMethod::Spherical => {
                    smesh
                        .spherical_project_uvs(glam::Vec3::ZERO)
                        .unwrap();
                }
                UVMethod::Xatlas => {
                    smesh.auto_uv_unwrap().unwrap();
                }
            }

            let new_mesh: Mesh = smesh.into();
            *mesh_handle = Mesh3d(meshes.add(new_mesh));
        }

        if let Ok(mut text) = text_query.single_mut() {
            **text = format!(
                "Press SPACE to cycle UV unwrap methods\n\nCube          Sphere          Cylinder\nCurrent Method: {}",
                method_name
            );
        }
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins,
        PanOrbitCameraPlugin,
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, update_uv_method);

    app.run();
}
