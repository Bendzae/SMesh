---
name: generate-mesh
description: Generate a procedural 3D mesh from a user's description using the smesh library
argument-hint: [object description, e.g. "a table" or "a simple house"]
---

# Procedural Mesh Generation with SMesh

Generate a 3D mesh matching this description: **$ARGUMENTS**

## Step 1: Learn the API

Before writing any code, read these source files to understand the smesh API:

1. **Primitives** — `src/smesh/primitives.rs` (Cube, Cylinder, Icosphere, Circle, Quad)
2. **Edit operations** — `src/smesh/edit_operations.rs` (extrude, inset, inset_faces, subdivide, combine_with)
3. **Transform** — `src/smesh/transform.rs` (translate, scale, rotate, Pivot enum)
4. **Introspection** — `src/smesh/introspection.rs` (describe, describe_selection, describe_faces)
5. **Validation** — `src/smesh/validation.rs` (validate)
6. **Tags** — `src/smesh/tags.rs` (tag, get_tag, take_tag)
7. **Spatial queries** — `src/smesh/spatial_queries.rs` (faces_facing, nearest_vertex, query_region, raycast)
8. **Preview rendering** — `src/smesh/preview.rs` (render_previews, save_preview_with_options, PreviewOptions)
9. **Showcase plugin** — `src/adapters/bevy.rs` (ShowcasePlugin)

**Read `examples/chair.rs` thoroughly** — it is the reference implementation showing the complete pattern: parametric resource with inspector UI, helper functions, generation function, live update system, ShowcasePlugin usage, PanOrbitCamera setup, and tests. Use it as your template for the example file structure, main(), init_system, and update_system.

Read ALL of these files before writing code. Do not guess at the API.

## Step 2: Plan the Geometry

Before coding, write a brief plan:
- What primitives and dimensions for each part? (use real-world meters)
- How will parts connect? (separate primitives via `combine_with` is usually cleanest)
- Which parameters should be user-tunable via inspector UI?

## Step 3: Write the Generation Function

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

1. **Prefer `combine_with` for separate parts** — building each part as its own primitive and combining produces much cleaner geometry than trying to extrude everything from one mesh.
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

// Find faces by direction
let top_faces = mesh.faces_facing(Vec3::Y, FRAC_PI_4);

// Filter faces by position
let back_faces: Vec<FaceId> = mesh.faces_facing(Vec3::NEG_Z, FRAC_PI_4)
    .into_iter()
    .filter(|f| mesh.get_face_centroid(*f).unwrap().y > 0.5)
    .collect();

// Inset then extrude for protrusions
let inner = mesh.inset(face, 0.7)?;  // 0.0=no change, 1.0=collapsed
let top = mesh.extrude(inner)?;
mesh.translate(top, Vec3::Y * height)?;

// Rotate a part before combining
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
eprintln!("=== After <step> ===\n{}", mesh.describe());

// Assert dimensions match your intent
assert!((report.dimensions.x - EXPECTED_WIDTH).abs() < 0.01,
    "Width should be ~{}, got {}", EXPECTED_WIDTH, report.dimensions.x);

let validation = mesh.validate();
eprintln!("{}", validation);
```

## Step 5: Visual Verification (CRITICAL)

Render preview images with wireframe and **read the image files** to visually inspect:

```rust
#[cfg(feature = "preview")]
{
    use smesh::smesh::preview::PreviewOptions;
    let opts = PreviewOptions::default().with_size(512, 512).with_wireframe();
    let paths = mesh.save_preview_with_options(&opts, "/tmp").unwrap();
    eprintln!("Saved previews: {:?}", paths);
}
```

Run `cargo test --features preview --example <name> -- --nocapture`, then read the images at `/tmp/front.png`, `/tmp/right.png`, `/tmp/top.png`, `/tmp/diagonal.png`. If something looks wrong, fix and re-run.

## Step 6: Create the Example

Follow the structure in `examples/chair.rs` exactly for:
- Imports, main(), init_system, update_system
- `ShowcasePlugin` for lights/ground/ambient
- `PanOrbitCamera` with `focus`/`radius`/`yaw`/`pitch` fields (NOT Transform — it gets overridden)
- `ResourceInspectorPlugin` for live parameter tweaking
- Tests that verify mesh validity and dimensions

## Iteration Workflow

1. Write initial generation code
2. Run `cargo test --features preview --example <name> -- --nocapture`
3. Read the preview images at `/tmp/*.png` to visually verify
4. If proportions are wrong → adjust dimensions, re-run
5. If topology is broken → check validate() output, fix
6. If parts are missing → check faces_facing() filter conditions
7. Repeat until the mesh matches the description

## Common Pitfalls

- **Don't use extrude + scale for protrusions** — creates flared transitions. Use inset + extrude.
- **Don't track FaceIds through long chains** — use `faces_facing()` and spatial queries.
- **Don't set camera position via Transform with PanOrbitCamera** — set `focus`, `radius`, `yaw`, `pitch` fields instead.
- **Prefer `combine_with`** for separate parts — cleaner geometry than extruding everything from one mesh.
- **Don't forget `recalculate_normals()`** — shading will be wrong without it.
- **Use helper functions** (`make_box`, `make_cylinder`, `make_sphere`) to reduce code and errors.
