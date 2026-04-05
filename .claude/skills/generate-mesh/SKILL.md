---
name: generate-mesh
description: Generate a procedural 3D mesh from a user's description using the smesh library
argument-hint: [object description, e.g. "a table" or "a simple house"]
---

# Procedural Mesh Generation with SMesh

Generate a 3D mesh matching this description: **$ARGUMENTS**

## Step 1: Learn the API

Before writing any code, read these source files to understand the smesh API:

1. **Primitives** — `src/smesh/primitives.rs` (Cube, Cylinder, Icosphere, Circle, Quad, Wedge)
2. **Edit operations** — `src/smesh/edit_operations.rs` (extrude, inset, inset_faces, subdivide, loop_cut, weld_vertices, combine_with)
3. **Topological operations** — `src/smesh/topological_operations.rs` (insert_vertex, collapse, remove_edge, delete_face)
4. **Transform** — `src/smesh/transform.rs` (translate, scale, rotate, Pivot enum)
5. **Spatial queries** — `src/smesh/spatial_queries.rs` (vertices_where, faces_facing, nearest_vertex, query_region, raycast)
6. **Selection** — `src/smesh/selection.rs` (MeshSelection, resolve_to_vertices/faces/halfedges)
7. **Tags** — `src/smesh/tags.rs` (tag, get_tag, take_tag)
8. **Introspection** — `src/smesh/introspection.rs` (describe)
9. **Validation** — `src/smesh/validation.rs` (validate)
10. **Preview rendering** — `src/smesh/preview.rs` (save_preview_with_options, PreviewOptions)
11. **Showcase plugin** — `src/adapters/bevy.rs` (ShowcasePlugin, DebugDrawMode)

Read ALL of these files before writing code. Do not guess at the API.

**Read `examples/chair.rs` thoroughly** — it is the reference for the example file structure: parametric resource with inspector UI, helper functions, generation function, live update system, ShowcasePlugin, PanOrbitCamera, and tests.

Also skim `examples/house.rs` for examples of loop cuts, vertices_where, inset+extrude on existing geometry, and wedge primitives.

## Step 2: Plan the Geometry

Before coding, write a brief plan:
- What is the overall construction approach? Choose based on the object:
  - **Sculpting from a single primitive** (extrude, inset, loop_cut, vertices_where) works well for organic or monolithic shapes
  - **Combining separate parts** (combine_with + weld_vertices) works well for mechanical or multi-component objects
  - **Mix both** as needed — e.g. sculpt each part individually, then combine
- What are the real-world dimensions in meters?
- Which parameters should be user-tunable via inspector UI?

## Step 3: Write the Generation Function

Choose your construction techniques based on what fits the shape:

- `extrude` / `inset` — grow or recess regions of a face
- `loop_cut` — subdivide a mesh at precise positions to create new edge loops for further editing
- `vertices_where` — select vertices by spatial predicate for targeted transforms
- `faces_facing` — find faces by normal direction for selective operations
- `combine_with` + `weld_vertices` — join separate parts and merge coincident vertices at joints
- `subdivide` — add detail uniformly

After edit operations (inset, extrude, loop_cut) that change topology, clear stale normals before combining:
```rust
mesh.face_normals = None;
mesh.vertex_normals = None;
```

Call `recalculate_normals()` at the end of generation.

When combining many parts, call `weld_vertices(threshold)` at the end to merge coincident vertices — this fixes normals at joints and reduces vertex count.

## Step 4: Verify with Introspection

After each major construction step, print and check the describe output:

```rust
eprintln!("=== After <step> ===\n{}", mesh.describe());

let report = mesh.describe();
assert!((report.dimensions.x - EXPECTED_WIDTH).abs() < 0.1,
    "Width should be ~{}, got {}", EXPECTED_WIDTH, report.dimensions.x);

let validation = mesh.validate();
eprintln!("{}", validation);
```

## Step 5: Visual Verification

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
6. If parts are missing → check spatial query filter conditions
7. Repeat until the mesh matches the description

## Common Pitfalls

- **Don't use extrude + scale for protrusions** — creates flared transitions. Use inset + extrude instead.
- **Don't track FaceIds through long operation chains** — use `faces_facing()` and spatial queries to re-find faces.
- **Don't set camera position via Transform with PanOrbitCamera** — set `focus`, `radius`, `yaw`, `pitch` fields instead.
- **Don't forget `recalculate_normals()`** — shading will be wrong without it.
