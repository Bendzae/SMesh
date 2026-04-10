---
name: generate-mesh
description: Generate a procedural 3D mesh from a user's description using the smesh library
argument-hint: [object description, e.g. "a table" or "a simple house"]
---

# Procedural Mesh Generation with SMesh

Generate a 3D mesh matching this description: **$ARGUMENTS**

## Step 1: Learn the API

Read the source files in `src/smesh/` to understand the available operations. Key modules:

- `primitives.rs` — starting shapes (Cube, Cylinder, Icosphere, Circle, Quad, Wedge)
- `edit_operations.rs` — topology editing (extrude, inset, subdivide, smooth_subdivide, loop_cut, bridge_vertices, merge_vertices, weld_vertices, combine_with)
- `transform.rs` — position changes (translate, scale, rotate, spherize, smooth, translate_proportional, Falloff curves, Pivot enum)
- `spatial_queries.rs` — finding elements (vertices_where, faces_facing, select_region, nearest_vertex, raycast)
- `selection.rs` — grouping elements (MeshSelection)
- `preview.rs` — rendering (save_composite_preview, save_preview_with_options, PreviewOptions)

Read `examples/chair.rs` as the reference for example file structure: parametric resource with inspector UI, generation function, live update system, ShowcasePlugin, PanOrbitCamera.

Skim other examples in `examples/` for usage patterns.

## Step 2: Write the Generation Function

Plan the construction approach:
- **Sculpting from a primitive** (extrude, inset, loop_cut, vertices_where) for organic/monolithic shapes
- **Combining parts** (combine_with + weld_vertices) for mechanical/multi-component objects

After topology-changing operations, call `recalculate_normals()` at the end.

## Step 3: Visual Verification

Render and read preview images to verify the mesh:

```rust
#[cfg(feature = "preview")]
{
    use smesh::smesh::preview::PreviewOptions;
    let opts = PreviewOptions::default().with_size(1024, 1024).with_wireframe();
    mesh.save_composite_preview(&opts, "/tmp/preview.png").unwrap();
}
```

Run with `cargo test --features preview --example <name> -- --nocapture`, then read `/tmp/preview.png`. Fix and re-iterate until correct.

## Step 4: Create the Example

Follow the structure in `examples/chair.rs`:
- `ShowcasePlugin` for lights/ground/ambient
- `PanOrbitCamera` with `focus`/`radius`/`yaw`/`pitch` fields (not Transform)
- `ResourceInspectorPlugin` for live parameter tweaking
- Tests that verify mesh validity and dimensions

### Hot Patching (required)

All examples must support hot patching for live code reload:

1. Import the prelude: `use bevy_hotpatching_experiments::prelude::*;`
2. Add `SimpleSubsecondPlugin::default()` to your app plugins
3. Annotate the update system with `#[hot]`
4. Add `mut hot_events: MessageReader<HotPatched>` as a system parameter
5. Trigger regeneration on hot patch events:
   ```rust
   let hot_patched = hot_events.read().count() > 0;
   if params.is_changed() || hot_patched {
       // regenerate mesh
   }
   ```
