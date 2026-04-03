---
name: generate-mesh
description: Generate a procedural 3D mesh from a user's description using the smesh library
argument-hint: [object description, e.g. "a table" or "a simple house"]
---

# Procedural Mesh Generation with SMesh

Generate a 3D mesh matching this description: **$ARGUMENTS**

## Step 1: Learn the API

Before writing any code, read these source files to understand the smesh API:

1. **Primitives** — `src/smesh/primitives.rs` (Cube, Cylinder, Icosphere, Circle, Quad — all with `.generate()`)
2. **Edit operations** — `src/smesh/edit_operations.rs` (extrude, extrude_faces, inset, inset_faces, subdivide, combine_with)
3. **Transform** — `src/smesh/transform.rs` (translate, scale, rotate, center_of_gravity, Pivot enum)
4. **Introspection** — `src/smesh/introspection.rs` (describe, describe_selection, describe_faces)
5. **Validation** — `src/smesh/validation.rs` (validate, validate_with_options)
6. **Tags** — `src/smesh/tags.rs` (tag, get_tag, take_tag for naming mesh regions)
7. **Spatial queries** — `src/smesh/spatial_queries.rs` (faces_facing, nearest_vertex, query_region, raycast)
8. **Preview rendering** — `src/smesh/preview.rs` (render_previews, render, save_preview_with_options, PreviewOptions)
9. **Showcase plugin** — `src/adapters/bevy.rs` (ShowcasePlugin — provides lights, ground, ambient)
10. **Chair example** — Read `examples/chair.rs` as the reference pattern for the full workflow

Read ALL of these files before writing code. Do not guess at the API.

## Step 2: Plan the Geometry

Before coding, write a brief plan:
- What primitives will you start with?
- What dimensions should each part have? (use real-world meters as reference)
- What operations will shape each part? (extrude, inset, scale, etc.)
- How will parts connect? (connected via extrude, or separate via combine_with)
- Which parameters should be user-tunable via the inspector UI?

Define dimension constants at the top of your function for easy tuning.

## Step 3: Write the Generation Function

Create a function that accepts a parameters struct:

```rust
fn generate_<object>(params: &<Object>Parameters) -> SMeshResult<SMesh>
```

### Helper functions

Define these helpers — they make building from separate parts much cleaner:

```rust
fn make_box(width: f32, height: f32, depth: f32, position: Vec3) -> SMeshResult<SMesh> {
    let (mut part, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = part.select_all();
    part.scale(all.clone(), vec3(width, height, depth), Pivot::Origin)?;
    part.translate(all, position)?;
    Ok(part)
}

fn make_cylinder(radius: f32, height: f32, segments: usize, position: Vec3) -> SMeshResult<SMesh> {
    let (mut cyl, _) = primitives::Cylinder { segments, height, radius }.generate()?;
    let all = cyl.select_all();
    cyl.translate(all, position)?;
    Ok(cyl)
}

fn make_sphere(radius: f32, subdivisions: usize, position: Vec3) -> SMeshResult<SMesh> {
    let (mut sphere, _) = primitives::Icosphere { subdivisions }.generate()?;
    let all = sphere.select_all();
    sphere.scale(all.clone(), Vec3::splat(radius * 2.0), Pivot::Origin)?;
    sphere.translate(all, position)?;
    Ok(sphere)
}
```

### Key principles

1. **Prefer `combine_with` for separate parts** — building each part as its own primitive (box, cylinder) and combining produces much cleaner geometry than trying to extrude everything from one mesh.
2. **Use `inset` then `extrude`** for protrusions that grow from a surface — not extrude+scale, which creates tapered/flared transitions.
3. **Use `faces_facing()` and spatial queries** to find faces by direction/position instead of tracking IDs.
4. **Tag important regions** with `mesh.tag(selection, "name")`.
5. **Call `recalculate_normals()`** at the end.

### Key API patterns:

```rust
// Build parts separately and combine
let mut mesh = SMesh::new();
mesh.combine_with(make_box(0.5, 0.04, 0.4, vec3(0.0, 0.48, 0.0))?)?;
mesh.combine_with(make_cylinder(0.02, 0.46, 8, vec3(0.2, 0.23, 0.15))?)?;

// Find faces by direction (instead of tracking IDs)
let top_faces = mesh.faces_facing(Vec3::Y, FRAC_PI_4);

// Filter faces by position
let back_faces: Vec<FaceId> = mesh.faces_facing(Vec3::NEG_Z, FRAC_PI_4)
    .into_iter()
    .filter(|f| mesh.get_face_centroid(*f).unwrap().y > 0.5)
    .collect();

// Inset (shrink face in-place) then extrude (pull outward)
let inner = mesh.inset(face, 0.7)?;  // 0.0=no change, 1.0=collapsed
let top = mesh.extrude(inner)?;
mesh.translate(top, Vec3::Y * height)?;

// Rotate a part before combining (e.g., diamond ornament)
let (mut diamond, _) = primitives::Cube { subdivision: U16Vec3::ONE }.generate()?;
let all = diamond.select_all();
diamond.scale(all.clone(), vec3(0.03, 0.03, 0.02), Pivot::Origin)?;
diamond.rotate(all.clone(), Quat::from_rotation_z(PI / 4.0), Pivot::Origin)?;
diamond.translate(all, position)?;
mesh.combine_with(diamond)?;

mesh.recalculate_normals()?;
```

## Step 4: Verify with Introspection (CRITICAL)

After each major construction step, print and CHECK the describe output:

```rust
let report = mesh.describe();
eprintln!("=== After <step> ===\n{}", report);

// Assert dimensions match your intent
assert!((report.dimensions.x - EXPECTED_WIDTH).abs() < 0.01,
    "Width should be ~{}, got {}", EXPECTED_WIDTH, report.dimensions.x);
```

**Run `mesh.validate()`** and check for issues:
```rust
let validation = mesh.validate();
eprintln!("{}", validation);
```

## Step 5: Visual Verification (CRITICAL)

Render preview images and LOOK at them before finishing:

```rust
#[cfg(feature = "preview")]
{
    use smesh::smesh::preview::{PreviewOptions, PreviewView};
    let opts = PreviewOptions::default()
        .with_size(512, 512)
        .with_wireframe();
    let paths = mesh.save_preview_with_options(&opts, "/tmp").unwrap();
    eprintln!("Saved previews: {:?}", paths);
}
```

After saving, **read the image files** (`/tmp/front.png`, `/tmp/right.png`, `/tmp/top.png`, `/tmp/diagonal.png`) to visually inspect the result. Check:
- Are proportions correct?
- Are all parts visible and in the right positions?
- Does the wireframe show the expected face structure?
- Are there any holes or missing faces?

If something looks wrong, go back and fix it. This visual check is the most important verification step.

## Step 6: Create the Example

Use `ShowcasePlugin` for scene setup (lights, ground, ambient). Make key parameters
tunable via `bevy-inspector-egui`. Follow `examples/chair.rs` as the reference.

```rust
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
struct ObjectParameters {
    #[inspector(min = 0.1, max = 1.0)]
    pub width: f32,
    // ... more params with inspector bounds
}

impl Default for ObjectParameters { ... }

#[derive(Component)]
struct ObjectTag;

fn generate_object(params: &ObjectParameters) -> SMeshResult<SMesh> {
    // ... generation code ...
}

fn update_system(
    params: Res<ObjectParameters>,
    objects: Query<Entity, With<ObjectTag>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if params.is_changed() {
        for e in &objects {
            let smesh = generate_object(&params).unwrap();
            let v0 = smesh.vertices().next().unwrap();
            commands.entity(e).insert((
                Mesh3d(meshes.add(Mesh::from(smesh.clone()))),
                DebugRenderSMesh { mesh: smesh, selection: Selection::Vertex(v0), visible: false },
            ));
        }
    }
}

fn init_system(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(ObjectParameters::default());

    commands.spawn((
        ObjectTag,
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.35, 0.15),
            perceptual_roughness: 0.7,
            ..default()
        })),
    ));

    // Camera — use PanOrbitCamera fields, NOT Transform (Transform gets overwritten)
    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        PanOrbitCamera {
            focus: vec3(0.0, 0.45, 0.0),  // center of your object
            radius: Some(2.0),
            yaw: Some(0.6),
            pitch: Some(0.4),
            ..default()
        },
    ));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ShowcasePlugin, PanOrbitCameraPlugin, EguiPlugin::default()))
        .add_plugins(ResourceInspectorPlugin::<ObjectParameters>::default())
        .add_systems(Startup, init_system)
        .add_systems(Update, update_system)
        .register_type::<ObjectParameters>()
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generates_valid_mesh() {
        let mesh = generate_object(&ObjectParameters::default()).unwrap();
        let report = mesh.describe();
        assert!(report.vertex_count > 0);
        assert!(report.is_manifold);
    }
}
```

## Iteration Workflow

1. Write initial generation code
2. Run `cargo test --features preview --example <name> -- --nocapture` to see describe output AND render previews
3. Read the preview images at `/tmp/*.png` to visually verify
4. If proportions are wrong → adjust dimension constants, re-run
5. If topology is broken → check validate() output, fix the operation that caused it
6. If parts are missing → check faces_facing() filter conditions
7. Repeat until the mesh matches the description

## Common Pitfalls

- **Don't use extrude + scale for protrusions** — creates tapered/flared transitions. Use inset + extrude instead.
- **Don't track FaceIds through long chains** — use faces_facing() and spatial queries to re-find faces after operations.
- **Don't forget recalculate_normals()** — shading will be wrong without it.
- **Don't set camera position via Transform when using PanOrbitCamera** — it overrides Transform on the first frame. Set `focus`, `radius`, `yaw`, `pitch` fields on PanOrbitCamera instead.
- **For separate parts** (that don't need to share edges), use combine_with() with separate Cube/Cylinder primitives — it's simpler and produces cleaner geometry than trying to extrude everything from one mesh.
- **Use helper functions** (`make_box`, `make_cylinder`, `make_sphere`) — they dramatically reduce code and errors when building multi-part objects.
- **Use ShowcasePlugin** — don't manually set up lights, ground plane, ambient. Just add it as a plugin.
