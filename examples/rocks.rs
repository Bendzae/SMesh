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
    #[inspector(min = 1, max = 20)]
    pub num_rocks: usize,
    #[inspector(min = 0.1, max = 1.0)]
    pub pile_radius: f32,
    #[inspector(min = 0.03, max = 0.25)]
    pub rock_size: f32,
    #[inspector(min = 0.0, max = 1.0)]
    pub rock_size_variation: f32,
    #[inspector(min = 0.0, max = 0.4)]
    pub roughness: f32,
    #[inspector(min = 1, max = 4)]
    pub num_extrusions: usize,
    #[inspector(min = 0, max = 2)]
    pub subdivisions: usize,
    pub seed: u64,
}

impl Default for RockPileParameters {
    fn default() -> Self {
        Self {
            num_rocks: 10,
            pile_radius: 0.35,
            rock_size: 0.10,
            rock_size_variation: 0.5,
            roughness: 0.12,
            num_extrusions: 3,
            subdivisions: 1,
            seed: 42,
        }
    }
}

#[derive(Component)]
struct RockPileTag;

/// Simple deterministic RNG (xorshift64).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(1))
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn f32(&mut self) -> f32 {
        (self.next_u64() % 10000) as f32 / 10000.0
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.f32() * (max - min)
    }

    /// Pick a random index in [0, n)
    fn index(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Create a single rock by starting from a cube, randomly extruding faces,
/// subdividing, then applying vertex displacement.
fn make_rock(
    base_size: f32,
    size_variation: f32,
    roughness: f32,
    num_extrusions: usize,
    subdivisions: usize,
    rng: &mut Rng,
) -> SMeshResult<SMesh> {
    // Start from a cube
    let (mut rock, _) = primitives::Cube {
        subdivision: glam::U16Vec3::ONE,
    }
    .generate()?;

    // Random non-uniform scale for initial shape variety
    let size_mult = 1.0 - size_variation * 0.5 + rng.f32() * size_variation;
    let sx = base_size * size_mult * rng.range(0.7, 1.3);
    let sy = base_size * size_mult * rng.range(0.5, 0.9); // flatter
    let sz = base_size * size_mult * rng.range(0.7, 1.3);
    let all = rock.select_all();
    rock.scale(all, vec3(sx, sy, sz), Pivot::Origin)?;

    // Randomly extrude some faces to create irregular bumps
    for _ in 0..num_extrusions {
        let faces: Vec<FaceId> = rock.faces().collect();
        if faces.is_empty() {
            break;
        }
        let face = faces[rng.index(faces.len())];

        // Compute face normal direction for extrusion
        let _centroid = rock.get_face_centroid(face)?;
        let face_verts: Vec<VertexId> = face.vertices(&rock).collect();
        if face_verts.len() < 3 {
            continue;
        }
        let positions: Vec<Vec3> = face_verts
            .iter()
            .filter_map(|v| v.position(&rock).ok())
            .collect();
        if positions.len() < 3 {
            continue;
        }
        let e1 = positions[1] - positions[0];
        let e2 = positions[2] - positions[0];
        let normal = e1.cross(e2).normalize_or_zero();
        if normal == Vec3::ZERO {
            continue;
        }

        let top = rock.extrude(face)?;
        let extrude_dist = rng.range(0.3, 0.8) * base_size * size_mult * 0.5;
        rock.translate(top, normal * extrude_dist)?;

        // Slightly scale the extruded face for variety
        let s = rng.range(0.6, 1.1);
        rock.scale(top, Vec3::splat(s), Pivot::SelectionCog)?;
    }

    // Subdivide to smooth out the blocky shape
    for _ in 0..subdivisions {
        let all = rock.select_all();
        rock.subdivide(all)?;
    }

    // Vertex displacement for surface roughness
    if roughness > 0.001 {
        let center = rock.center_of_gravity(rock.select_all())?;
        let verts: Vec<VertexId> = rock.vertices().collect();
        for v in verts {
            if let Ok(pos) = v.position(&rock) {
                let dir = (pos - center).normalize_or_zero();
                let displacement = dir * rng.range(-roughness, roughness) * base_size;
                rock.positions.insert(v, pos + displacement);
            }
        }
    }

    // Random rotation
    let yaw = rng.range(0.0, PI * 2.0);
    let pitch = rng.range(-0.4, 0.4);
    let roll = rng.range(-0.4, 0.4);
    let rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    let all = rock.select_all();
    rock.rotate(all, rot, Pivot::Origin)?;

    // Recalculate normals so combine_with doesn't hit stale face normal keys
    rock.recalculate_normals()?;

    Ok(rock)
}

/// Simple heightfield for stacking rocks. Tracks the maximum Y at grid cells.
struct Heightfield {
    cells: Vec<f32>,
    resolution: usize,
    extent: f32, // half-size of the grid
}

impl Heightfield {
    fn new(extent: f32, resolution: usize) -> Self {
        Self {
            cells: vec![0.0; resolution * resolution],
            resolution,
            extent,
        }
    }

    /// Get the ground height at a world XZ position.
    fn height_at(&self, x: f32, z: f32) -> f32 {
        let gx = ((x + self.extent) / (self.extent * 2.0) * self.resolution as f32) as usize;
        let gz = ((z + self.extent) / (self.extent * 2.0) * self.resolution as f32) as usize;
        let gx = gx.min(self.resolution - 1);
        let gz = gz.min(self.resolution - 1);
        self.cells[gz * self.resolution + gx]
    }

    /// Update the heightfield after placing a rock.
    /// Raises all cells covered by the rock's XZ footprint to the rock's top Y.
    fn place_rock(&mut self, center: Vec3, radius_xz: f32, top_y: f32) {
        let cell_size = self.extent * 2.0 / self.resolution as f32;
        for gz in 0..self.resolution {
            for gx in 0..self.resolution {
                let wx = -self.extent + (gx as f32 + 0.5) * cell_size;
                let wz = -self.extent + (gz as f32 + 0.5) * cell_size;
                let dx = wx - center.x;
                let dz = wz - center.z;
                if (dx * dx + dz * dz).sqrt() < radius_xz {
                    let idx = gz * self.resolution + gx;
                    self.cells[idx] = self.cells[idx].max(top_y);
                }
            }
        }
    }
}

fn generate_rock_pile(params: &RockPileParameters) -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();
    let mut rng = Rng::new(params.seed);
    let mut heightfield = Heightfield::new(params.pile_radius * 1.5, 16);

    // Sort rocks: place bigger ones first (at the bottom)
    let mut rock_sizes: Vec<(usize, f32)> = (0..params.num_rocks)
        .map(|i| {
            let mut size_rng = Rng::new(params.seed.wrapping_add(i as u64 * 7919));
            let size_mult = 1.0 - params.rock_size_variation * 0.5
                + size_rng.f32() * params.rock_size_variation;
            (i, size_mult)
        })
        .collect();
    rock_sizes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    for (i, _size_mult) in &rock_sizes {
        let t = *i as f32 / params.num_rocks.max(1) as f32;

        // Pick XZ position — tighter toward center for higher rocks
        let layer_radius = params.pile_radius * (1.0 - t * 0.6);
        let angle = rng.range(0.0, PI * 2.0);
        let dist = layer_radius * rng.f32().sqrt();
        let x = angle.cos() * dist;
        let z = angle.sin() * dist;

        // Generate the rock shape
        let mut rock = make_rock(
            params.rock_size,
            params.rock_size_variation,
            params.roughness,
            params.num_extrusions,
            params.subdivisions,
            &mut rng,
        )?;

        // Compute rock's bounding box to figure out how to place it
        let rock_report = rock.describe();
        let rock_bottom_y = rock_report.bounding_box.0.y;
        let rock_height = rock_report.dimensions.y;
        let rock_radius_xz = (rock_report.dimensions.x + rock_report.dimensions.z) * 0.25;

        // Drop the rock onto the heightfield
        let ground_y = heightfield.height_at(x, z);
        let place_y = ground_y - rock_bottom_y; // align rock bottom to ground level

        let all = rock.select_all();
        rock.translate(all, vec3(x, place_y, z))?;

        // Update heightfield
        let top_y = ground_y + rock_height;
        heightfield.place_rock(vec3(x, place_y, z), rock_radius_xz, top_y);

        mesh.combine_with(rock)?;
    }

    eprintln!("=== Rock Pile ===\n{}", mesh.describe());

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
            focus: vec3(0.0, 0.1, 0.0),
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
        // All rocks should be on or above ground
        assert!(report.bounding_box.0.y >= -0.01, "Rocks should sit on ground, got min y={}", report.bounding_box.0.y);
    }

    #[test]
    fn rock_pile_respects_seed() {
        let params = RockPileParameters::default();
        let mesh1 = generate_rock_pile(&params).unwrap();
        let mesh2 = generate_rock_pile(&params).unwrap();
        let r1 = mesh1.describe();
        let r2 = mesh2.describe();
        assert_eq!(r1.vertex_count, r2.vertex_count);
        assert_eq!(r1.face_count, r2.face_count);
    }

    #[test]
    fn rock_pile_with_few_rocks() {
        let params = RockPileParameters {
            num_rocks: 2,
            subdivisions: 0,
            ..Default::default()
        };
        let mesh = generate_rock_pile(&params).unwrap();
        let report = mesh.describe();
        assert!(report.is_manifold);
        assert_eq!(report.connected_components, 2);
    }
}
