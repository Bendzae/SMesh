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
9. **Existing examples** — Read `examples/chair.rs` and `examples/tree.rs` to understand patterns

Read ALL of these files before writing code. Do not guess at the API.

## Step 2: Plan the Geometry

Before coding, write a brief plan:
- What primitives will you start with?
- What dimensions should each part have? (use real-world meters as reference)
- What operations will shape each part? (extrude, inset, scale, etc.)
- How will parts connect? (connected via extrude, or separate via combine_with)

Define dimension constants at the top of your function for easy tuning.

## Step 3: Write the Generation Function

Create a function `fn generate_<object>() -> SMeshResult<SMesh>` that:

1. **Builds geometry step by step**, verifying after each major step
2. **Uses `faces_facing()` and spatial queries** to find faces by direction/position instead of tracking IDs through long chains
3. **Tags important regions** with `mesh.tag(selection, "name")` so you can refer to them later
4. **Uses `take_tag("name")`** when you need an owned selection for mutation (translate, scale, etc.)
5. **Uses `inset` then `extrude`** for creating protrusions (not extrude+scale, which creates tapered transitions)
6. **Calls `recalculate_normals()`** at the end

### Key API patterns:

```rust
// Start from a primitive
let (mut mesh, _) = primitives::Cube { subdivision: U16Vec3::new(2, 1, 2) }.generate()?;

// Scale and position
mesh.scale(mesh.select_all(), vec3(width, height, depth), Pivot::Origin)?;
mesh.translate(mesh.select_all(), vec3(0.0, y_offset, 0.0))?;

// Find faces by direction (instead of tracking IDs)
let top_faces = mesh.faces_facing(Vec3::Y, FRAC_PI_4);
let bottom_faces = mesh.faces_facing(Vec3::NEG_Y, FRAC_PI_4);

// Filter faces by position
let back_faces: Vec<FaceId> = mesh.faces_facing(Vec3::NEG_Z, FRAC_PI_4)
    .into_iter()
    .filter(|f| mesh.get_face_centroid(*f).unwrap().y > 0.5)
    .collect();

// Inset (shrink face in-place) then extrude (pull outward)
let inner = mesh.inset(face, 0.7)?;  // 0.0=no change, 1.0=collapsed
let top = mesh.extrude(inner)?;
mesh.translate(top, Vec3::Y * height)?;

// Tag for later reference
mesh.tag(top_faces.clone(), "roof");

// Combine separate meshes
let (mut part, _) = primitives::Cube { subdivision: U16Vec3::ONE }.generate()?;
part.scale(part.select_all(), dims, Pivot::Origin)?;
part.translate(part.select_all(), position)?;
mesh.combine_with(part)?;

// Always recalculate normals at the end
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

// Check topology
assert!(report.is_manifold, "Mesh should be manifold");

// Check tagged regions
if let Some(sel) = mesh.take_tag("part_name") {
    let part_report = mesh.describe_selection(sel);
    eprintln!("=== Part ===\n{}", part_report);
}
```

**Run `mesh.validate()`** and check for issues:
```rust
let validation = mesh.validate();
eprintln!("{}", validation);
// Fix any degenerate faces, non-manifold vertices, etc.
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
    // READ THE IMAGES to verify the mesh looks correct
}
```

After saving, **read the image files** to visually inspect the result. Check:
- Are proportions correct?
- Are all parts visible and in the right positions?
- Does the wireframe show the expected face structure?
- Are there any holes or missing faces?

If something looks wrong, go back to Step 3 and fix it. This visual check is the most important verification step.

## Step 6: Create the Example

Structure the example file like the existing ones (see `examples/chair.rs`):

```rust
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use smesh::{
    adapters::bevy::{DebugRenderSMesh, SMeshDebugDrawPlugin, Selection},
    prelude::*,
};
use primitives::Primitive;
use transform::Pivot;

fn generate_<object>() -> SMeshResult<SMesh> {
    // ... generation code with verification ...
}

fn init_system(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mesh = generate_<object>().unwrap();
    let v0 = mesh.vertices().next().unwrap();

    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(mesh.clone()))),
        MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.6, 0.35, 0.15), ..default() })),
        DebugRenderSMesh { mesh, selection: Selection::Vertex(v0), visible: false },
    ));

    // Add ground plane, lights, camera (see chair.rs for full pattern)
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight { color: Color::WHITE, brightness: 300.0, ..default() })
        .add_plugins((DefaultPlugins, PanOrbitCameraPlugin, SMeshDebugDrawPlugin, EguiPlugin::default()))
        .add_systems(Startup, init_system)
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generates_valid_mesh() {
        let mesh = generate_<object>().unwrap();
        let report = mesh.describe();
        let validation = mesh.validate();
        assert!(report.vertex_count > 0);
        assert!(report.is_manifold);
        // Add dimension checks specific to the object
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
- **Don't build the backrest by extruding vertical faces upward** — extrude horizontal (top-facing) faces instead, then scale thin.
- **For separate parts** (that don't need to share edges), use combine_with() with separate Cube/Cylinder primitives — it's simpler and produces cleaner geometry.
