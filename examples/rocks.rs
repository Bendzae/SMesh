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
    #[inspector(min = 1, max = 3)]
    pub subdivisions: usize,
    #[inspector(min = 0.3, max = 1.0)]
    pub spherize: f32,
    #[inspector(min = 1.0, max = 8.0)]
    pub noise_scale: f32,
    pub seed: u64,
}

impl Default for RockPileParameters {
    fn default() -> Self {
        Self {
            num_rocks: 10,
            pile_radius: 0.35,
            rock_size: 0.10,
            rock_size_variation: 0.5,
            roughness: 0.15,
            subdivisions: 2,
            spherize: 0.7,
            noise_scale: 4.0,
            seed: 42,
        }
    }
}

#[derive(Component)]
struct RockPileTag;

// === Simple deterministic RNG ===

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

}

// === 3D value noise ===

/// Hash a 3D integer coordinate to a pseudo-random f32 in [-1, 1].
fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut n = (x.wrapping_mul(73856093)) ^ (y.wrapping_mul(19349663)) ^ (z.wrapping_mul(83492791));
    n = (n << 13) ^ n;
    n = n
        .wrapping_mul(15731)
        .wrapping_add(789221)
        .wrapping_mul(n)
        .wrapping_add(1376312589)
        .wrapping_mul(n);
    (n & 0x7fffffff) as f32 / 0x7fffffff as f32 * 2.0 - 1.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// 3D value noise returning a value in roughly [-1, 1].
fn noise3d(x: f32, y: f32, z: f32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;
    let fx = smoothstep(x - x.floor());
    let fy = smoothstep(y - y.floor());
    let fz = smoothstep(z - z.floor());

    let c000 = hash3(ix, iy, iz);
    let c100 = hash3(ix + 1, iy, iz);
    let c010 = hash3(ix, iy + 1, iz);
    let c110 = hash3(ix + 1, iy + 1, iz);
    let c001 = hash3(ix, iy, iz + 1);
    let c101 = hash3(ix + 1, iy, iz + 1);
    let c011 = hash3(ix, iy + 1, iz + 1);
    let c111 = hash3(ix + 1, iy + 1, iz + 1);

    let x00 = lerp(c000, c100, fx);
    let x10 = lerp(c010, c110, fx);
    let x01 = lerp(c001, c101, fx);
    let x11 = lerp(c011, c111, fx);

    let y0 = lerp(x00, x10, fy);
    let y1 = lerp(x01, x11, fy);

    lerp(y0, y1, fz)
}

/// Fractal brownian motion — layered noise for more natural look.
fn fbm3d(x: f32, y: f32, z: f32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_amp = 0.0;

    for _ in 0..octaves {
        value += noise3d(x * frequency, y * frequency, z * frequency) * amplitude;
        max_amp += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_amp
}

// === Rock generation ===

/// Create a single rock from an icosphere with non-uniform scale,
/// spherize pass, and FBM noise displacement for natural surface detail.
fn make_rock(
    base_size: f32,
    size_variation: f32,
    roughness: f32,
    subdivisions: usize,
    spherize: f32,
    noise_scale: f32,
    rng: &mut Rng,
) -> SMeshResult<SMesh> {
    // Start from an icosphere — clean manifold topology that survives
    // vertex displacement well. Use subdivisions for detail level.
    let (mut rock, _) = primitives::Icosphere {
        subdivisions: subdivisions.max(1),
    }
    .generate()?;

    // Random non-uniform scale to make rocks oblong/irregular
    let size_mult = 1.0 - size_variation * 0.5 + rng.f32() * size_variation;
    let sx = base_size * size_mult * rng.range(0.7, 1.4);
    let sy = base_size * size_mult * rng.range(0.4, 0.85); // flatter
    let sz = base_size * size_mult * rng.range(0.7, 1.4);
    let all = rock.select_all();
    rock.scale(all, vec3(sx, sy, sz), Pivot::Origin)?;

    // Spherize + noise displacement
    // Use a noise offset unique to this rock so each one has a different pattern
    let noise_offset = vec3(
        rng.range(-100.0, 100.0),
        rng.range(-100.0, 100.0),
        rng.range(-100.0, 100.0),
    );

    {
        let center = rock.center_of_gravity(rock.select_all())?;
        let verts: Vec<VertexId> = rock.vertices().collect();

        // Compute average distance from center
        let mut avg_dist = 0.0f32;
        let mut count = 0u32;
        for &v in &verts {
            if let Ok(pos) = v.position(&rock) {
                avg_dist += (pos - center).length();
                count += 1;
            }
        }
        avg_dist /= count.max(1) as f32;

        for &v in &verts {
            if let Ok(pos) = v.position(&rock) {
                let dir = (pos - center).normalize_or_zero();
                let current_dist = (pos - center).length();

                // Spherize
                let target_dist = current_dist + (avg_dist - current_dist) * spherize;

                // 3D noise based on vertex position (gives coherent surface detail)
                let sample = (pos - center) * noise_scale / base_size + noise_offset;
                let n = fbm3d(sample.x, sample.y, sample.z, 3);
                let displacement = n * roughness * base_size;

                let new_pos = center + dir * (target_dist + displacement);
                rock.positions.insert(v, new_pos);
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

    rock.recalculate_normals()?;
    Ok(rock)
}

// === Heightfield for stacking ===

struct Heightfield {
    cells: Vec<f32>,
    resolution: usize,
    extent: f32,
}

impl Heightfield {
    fn new(extent: f32, resolution: usize) -> Self {
        Self {
            cells: vec![0.0; resolution * resolution],
            resolution,
            extent,
        }
    }

    fn height_at(&self, x: f32, z: f32) -> f32 {
        let gx = ((x + self.extent) / (self.extent * 2.0) * self.resolution as f32) as usize;
        let gz = ((z + self.extent) / (self.extent * 2.0) * self.resolution as f32) as usize;
        let gx = gx.min(self.resolution - 1);
        let gz = gz.min(self.resolution - 1);
        self.cells[gz * self.resolution + gx]
    }

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

// === Generation ===

fn generate_rock_pile(params: &RockPileParameters) -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();
    let mut rng = Rng::new(params.seed);
    let mut heightfield = Heightfield::new(params.pile_radius * 1.5, 16);

    // Sort rocks: bigger first (bottom of pile)
    let mut rock_sizes: Vec<(usize, f32)> = (0..params.num_rocks)
        .map(|i| {
            let mut size_rng = Rng::new(params.seed.wrapping_add(i as u64 * 7919));
            let size_mult = 1.0 - params.rock_size_variation * 0.5
                + size_rng.f32() * params.rock_size_variation;
            (i, size_mult)
        })
        .collect();
    rock_sizes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    for (i, _) in &rock_sizes {
        let t = *i as f32 / params.num_rocks.max(1) as f32;

        let layer_radius = params.pile_radius * (1.0 - t * 0.6);
        let angle = rng.range(0.0, PI * 2.0);
        let dist = layer_radius * rng.f32().sqrt();
        let x = angle.cos() * dist;
        let z = angle.sin() * dist;

        let mut rock = make_rock(
            params.rock_size,
            params.rock_size_variation,
            params.roughness,
            params.subdivisions,
            params.spherize,
            params.noise_scale,
            &mut rng,
        )?;

        let rock_report = rock.describe();
        let rock_bottom_y = rock_report.bounding_box.0.y;
        let rock_height = rock_report.dimensions.y;
        let rock_radius_xz = (rock_report.dimensions.x + rock_report.dimensions.z) * 0.25;

        let ground_y = heightfield.height_at(x, z);
        let place_y = ground_y - rock_bottom_y;

        let all = rock.select_all();
        rock.translate(all, vec3(x, place_y, z))?;

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

// === Bevy integration ===

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
        assert!(report.bounding_box.0.y >= -0.01, "Rocks should sit on ground");
    }

    #[test]
    fn rock_pile_respects_seed() {
        let params = RockPileParameters::default();
        let r1 = generate_rock_pile(&params).unwrap().describe();
        let r2 = generate_rock_pile(&params).unwrap().describe();
        assert_eq!(r1.vertex_count, r2.vertex_count);
    }

    #[test]
    fn rock_pile_with_few_rocks() {
        let params = RockPileParameters {
            num_rocks: 2,
            subdivisions: 0,
            ..Default::default()
        };
        let mesh = generate_rock_pile(&params).unwrap();
        assert!(mesh.describe().is_manifold);
    }
}
