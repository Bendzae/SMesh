use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_inspector_egui::{
    bevy_egui::EguiPlugin,
    inspector_options::ReflectInspectorOptions, quick::ResourceInspectorPlugin, InspectorOptions,
};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::vec3;

use smesh::{
    adapters::bevy::{DebugRenderSMesh, Selection, ShowcasePlugin},
    prelude::*,
};
use primitives::Primitive;
use transform::Pivot;

#[derive(Reflect, Resource, InspectorOptions, Clone)]
#[reflect(Resource, InspectorOptions)]
struct RockPileParameters {
    #[inspector(min = 1, max = 30)]
    pub num_rocks: usize,
    #[inspector(min = 0.05, max = 1.0)]
    pub pile_radius: f32,
    #[inspector(min = 0.03, max = 0.3)]
    pub rock_size: f32,
    #[inspector(min = 0.0, max = 1.0)]
    pub rock_size_variation: f32,
    #[inspector(min = 0.0, max = 0.3)]
    pub roughness: f32,
    #[inspector(min = 1, max = 3)]
    pub subdivisions: usize,
    pub seed: u64,
}

impl Default for RockPileParameters {
    fn default() -> Self {
        Self {
            num_rocks: 12,
            pile_radius: 0.4,
            rock_size: 0.12,
            rock_size_variation: 0.5,
            roughness: 0.15,
            subdivisions: 2,
            seed: 42,
        }
    }
}

#[derive(Component)]
struct RockPileTag;

/// Simple deterministic RNG (xorshift64) so we don't need external crate in generation.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(1)) // avoid 0
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Random f32 in [0, 1)
    fn f32(&mut self) -> f32 {
        (self.next_u64() % 10000) as f32 / 10000.0
    }

    /// Random f32 in [min, max)
    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.f32() * (max - min)
    }
}

/// Create a single rock: an icosphere with non-uniform scale, random rotation,
/// and vertex displacement for roughness.
fn make_rock(
    base_size: f32,
    size_variation: f32,
    roughness: f32,
    subdivisions: usize,
    position: Vec3,
    rng: &mut Rng,
) -> SMeshResult<SMesh> {
    let (mut rock, _) = primitives::Icosphere { subdivisions }.generate()?;

    // Random non-uniform scale to make rocks oblong/irregular
    let size_mult = 1.0 - size_variation + rng.f32() * size_variation * 2.0;
    let scale_x = base_size * size_mult * rng.range(0.6, 1.4);
    let scale_y = base_size * size_mult * rng.range(0.5, 1.0); // flatter vertically
    let scale_z = base_size * size_mult * rng.range(0.6, 1.4);

    let all = rock.select_all();
    rock.scale(all, vec3(scale_x, scale_y, scale_z), Pivot::Origin)?;

    // Random rotation
    let yaw = rng.range(0.0, PI * 2.0);
    let pitch = rng.range(-0.3, 0.3);
    let roll = rng.range(-0.3, 0.3);
    let rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    let all = rock.select_all();
    rock.rotate(all, rot, Pivot::Origin)?;

    // Vertex displacement for surface roughness
    if roughness > 0.001 {
        let verts: Vec<VertexId> = rock.vertices().collect();
        for v in verts {
            if let Ok(pos) = v.position(&rock) {
                let displacement = pos.normalize_or_zero() * rng.range(-roughness, roughness) * base_size;
                rock.positions.insert(v, pos + displacement);
            }
        }
    }

    // Move to position
    let all = rock.select_all();
    rock.translate(all, position)?;

    Ok(rock)
}

fn generate_rock_pile(params: &RockPileParameters) -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();
    let mut rng = Rng::new(params.seed);

    for i in 0..params.num_rocks {
        // Place rocks in a roughly conical pile:
        // Sort by layer — bottom rocks first, top rocks last
        let t = i as f32 / params.num_rocks.max(1) as f32;

        // Horizontal spread decreases with height (inverted cone / pile shape)
        let layer_radius = params.pile_radius * (1.0 - t * 0.8);
        let angle = rng.range(0.0, PI * 2.0);
        // Bias toward the edges at bottom, center at top
        let dist = layer_radius * rng.f32().sqrt();
        let x = angle.cos() * dist;
        let z = angle.sin() * dist;

        // Height: rocks sit on ground, pile up gradually
        let y = t * t * params.pile_radius * 0.8 + params.rock_size * 0.4;

        let rock = make_rock(
            params.rock_size,
            params.rock_size_variation,
            params.roughness,
            params.subdivisions,
            vec3(x, y, z),
            &mut rng,
        )?;
        mesh.combine_with(rock)?;
    }

    eprintln!("=== Rock Pile ===\n{}", mesh.describe());

    let validation = mesh.validate();
    eprintln!("=== Validation ===\n{}", validation);

    mesh.recalculate_normals()?;

    #[cfg(feature = "preview")]
    {
        use smesh::smesh::preview::PreviewOptions;
        let opts = PreviewOptions::default().with_size(512, 512).with_wireframe();
        let paths = mesh.save_preview_with_options(&opts, "/tmp").unwrap();
        eprintln!("Saved previews: {:?}", paths);
    }

    Ok(mesh)
}

fn update_rocks_system(
    params: Res<RockPileParameters>,
    rocks: Query<Entity, With<RockPileTag>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if params.is_changed() {
        for e in &rocks {
            let smesh = generate_rock_pile(&params).unwrap();
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
) {
    commands.insert_resource(RockPileParameters::default());

    commands.spawn((
        RockPileTag,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.42, 0.38),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));

    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        PanOrbitCamera {
            focus: vec3(0.0, 0.15, 0.0),
            radius: Some(1.5),
            yaw: Some(0.5),
            pitch: Some(0.4),
            ..default()
        },
    ));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ShowcasePlugin, PanOrbitCameraPlugin, EguiPlugin::default()))
        .add_plugins(ResourceInspectorPlugin::<RockPileParameters>::default())
        .add_systems(Startup, init_system)
        .add_systems(Update, update_rocks_system)
        .register_type::<RockPileParameters>()
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rock_pile_generates_valid_mesh() {
        let mesh = generate_rock_pile(&RockPileParameters::default()).unwrap();
        let report = mesh.describe();

        assert!(report.vertex_count > 100, "Too few vertices: {}", report.vertex_count);
        assert!(report.face_count > 50, "Too few faces: {}", report.face_count);
        assert!(report.is_manifold, "Should be manifold");
    }

    #[test]
    fn rock_pile_respects_seed() {
        let params = RockPileParameters::default();
        let mesh1 = generate_rock_pile(&params).unwrap();
        let mesh2 = generate_rock_pile(&params).unwrap();
        let r1 = mesh1.describe();
        let r2 = mesh2.describe();
        // Same seed should produce same geometry
        assert_eq!(r1.vertex_count, r2.vertex_count);
        assert_eq!(r1.face_count, r2.face_count);
    }

    #[test]
    fn rock_pile_with_few_rocks() {
        let params = RockPileParameters {
            num_rocks: 2,
            ..Default::default()
        };
        let mesh = generate_rock_pile(&params).unwrap();
        let report = mesh.describe();
        assert!(report.is_manifold);
        assert_eq!(report.connected_components, 2);
    }
}
