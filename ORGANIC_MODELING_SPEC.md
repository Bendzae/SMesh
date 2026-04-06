# Organic Modeling Features Spec

Improvements to smesh that enable AI agents (and human users) to create organic, rounded, creature-like meshes. Derived from practical experience attempting to sculpt a reptile enemy from an icosphere using the current toolset.

## Context

The current smesh API is strong for hard-surface/architectural modeling (boxes, cylinders, loop cuts, inset+extrude). Organic modeling is significantly harder because:

- No way to smooth geometry after blocking out shapes
- Vertex selection is binary (in/out) with no spatial falloff
- Extrude chains on triangle meshes are fragile (frequent TopologyError)
- No way to join separately sculpted parts through smooth transitions
- Preview iteration is slow (4 separate image files per check)

---

## P0 — Implement Now

### 1. Laplacian Smooth

**Operation:** `mesh.smooth(selection, iterations, factor)`

Relaxes selected vertices toward the average position of their neighbors. Each iteration moves each vertex by `factor` (0.0–1.0) toward its neighbor centroid. Boundary vertices should be optionally pinned (not moved) to preserve mesh silhouette.

```rust
pub fn smooth<S: Into<MeshSelection>>(
    &mut self,
    selection: S,
    iterations: usize,
    factor: f32,          // 0.0 = no movement, 1.0 = full average
    pin_boundaries: bool, // if true, boundary vertices don't move
) -> SMeshResult<&mut SMesh>
```

**Use case:** After sculpting with `vertices_where` (which creates hard transitions), smooth the region to soften edges. Also useful for relaxing geometry after extrude chains.

**Notes:**
- Should work on any mesh topology (tris, quads, mixed).
- Known limitation: pure Laplacian smoothing shrinks the mesh over many iterations. This is acceptable — users can compensate with scale, or use smooth_subdivide for volume-preserving results.

---

### 2. Catmull-Clark Smooth Subdivision

**Operation:** `mesh.smooth_subdivide(selection, iterations)`

Combined subdivision + smoothing that places new vertices at mathematically correct positions to approximate a smooth limit surface. Uses Catmull-Clark rules for quad-dominant meshes and Loop subdivision rules for triangle meshes.

```rust
pub fn smooth_subdivide<S: Into<MeshSelection>>(
    &mut self,
    selection: S,
    iterations: usize,
) -> SMeshResult<MeshSelection>
```

**Algorithm (Catmull-Clark):**
1. For each face, compute a **face point** (centroid of face vertices).
2. For each edge, compute an **edge point** (average of edge endpoints and adjacent face points).
3. Move each original vertex to a weighted average: `(Q/n + 2R/n + S(n-3)/n)` where Q = avg of adjacent face points, R = avg of adjacent edge midpoints, S = original position, n = valence.
4. Connect face points to edge points to form new quad faces.

**Algorithm (Loop — for triangle meshes):**
1. Insert vertex at each edge midpoint, weighted by adjacent vertices.
2. Move original vertices using Loop's weighting scheme.
3. Reconnect into 4 triangles per original triangle.

**Auto-detection:** If the selection is all-quads, use Catmull-Clark. If all-tris, use Loop. If mixed, convert tris to quads first (pair adjacent tris), then Catmull-Clark.

**Use case:** The primary organic modeling tool. Build a rough blockout with extrusions, then smooth_subdivide to get round, organic shapes. This is how every 3D artist works.

---

### 3. Bridge Edge Loops

**Operation:** `mesh.bridge(loop_a, loop_b)`

Connects two boundary edge loops with a strip of quad faces.

```rust
pub fn bridge(
    &mut self,
    loop_a: Vec<HalfedgeId>,  // ordered boundary halfedges
    loop_b: Vec<HalfedgeId>,  // ordered boundary halfedges
) -> SMeshResult<Vec<FaceId>>
```

**Algorithm:**
1. Verify both loops are boundary (open) edge loops.
2. If vertex counts differ, the shorter loop's vertices are interpolated (or error).
3. Find optimal rotational alignment by minimizing total edge-crossing distance.
4. Create quad faces connecting corresponding vertex pairs.
5. If counts match exactly, produce clean quads. If not, insert triangles to transition.

**Simpler alternative — bridge by vertex pairs:**
```rust
pub fn bridge_vertices(
    &mut self,
    loop_a: Vec<VertexId>,
    loop_b: Vec<VertexId>,
) -> SMeshResult<Vec<FaceId>>
```
Takes two ordered vertex rings and connects them with quads. Easier to implement, still very useful.

**Use case:** Build head and body as separate meshes, delete facing faces, bridge the openings to create a smooth neck. Much cleaner than extruding a narrow neck from a few triangles.

---

## P1 — Important Follow-ups

### 4. Proportional Editing / Soft Selection

**Operation:** `mesh.vertices_weighted(center, radius, falloff)` or integrated into transform operations.

Returns a selection where each vertex has a weight based on distance from center, using a falloff curve (linear, smooth, sharp, sphere, etc.).

```rust
pub enum Falloff {
    Linear,
    Smooth,    // smoothstep
    Sharp,     // inverse square
    Sphere,    // sqrt falloff
    Constant,  // all 1.0 within radius
}

pub fn translate_proportional(
    &mut self,
    center: Vec3,
    radius: f32,
    falloff: Falloff,
    translation: Vec3,
) -> SMeshResult<&mut SMesh>
```

Each vertex within `radius` of `center` is translated by `translation * weight`, where weight decreases from 1.0 at center to 0.0 at radius according to the falloff curve.

Similarly: `scale_proportional`, `rotate_proportional`.

**Use case:** Sculpting muscle bulges, cheek shapes, organic bumps. Without this, `vertices_where` creates hard boundaries between moved and unmoved vertices.

---

### 5. Spherize / Shape Projection

**Operation:** `mesh.spherize(selection, amount, pivot)`

Moves selected vertices toward the surface of a sphere (or ellipsoid) centered at pivot.

```rust
pub fn spherize<S: Into<MeshSelection>>(
    &mut self,
    selection: S,
    amount: f32,           // 0.0 = no change, 1.0 = on sphere surface
    pivot: Pivot,
    target_radius: Option<f32>,  // None = auto from avg distance
) -> SMeshResult<&mut SMesh>
```

For each vertex, lerp between current position and the point on the target sphere in the same direction from pivot.

**Use case:** Rounding a subdivided cube into an organic base shape. Also useful for creating bulging eyes, rounded joints, etc.

---

### 6. Triangles to Quads

**Operation:** `mesh.tris_to_quads(selection, angle_threshold)`

Merges adjacent triangle pairs into quads where the shared edge's dihedral angle is below a threshold.

```rust
pub fn tris_to_quads<S: Into<MeshSelection>>(
    &mut self,
    selection: S,
    max_angle: f32,  // radians — max deviation from planar
) -> SMeshResult<usize>  // returns number of merges
```

**Algorithm:**
1. For each internal edge shared by two triangles, compute the dihedral angle.
2. Sort edges by angle (flattest first).
3. Greedily merge triangle pairs into quads, ensuring each triangle is only merged once.
4. Remove the shared edge for each merge.

**Use case:** Convert icosphere triangles into quads before Catmull-Clark subdivision or extrusion. Quad meshes are far more reliable for edit operations.

---

### 7. Extrude Along Path

**Operation:** `mesh.extrude_along_path(face, path, scales)`

Extrudes a face along a series of points, optionally scaling at each step.

```rust
pub fn extrude_along_path(
    &mut self,
    face: FaceId,
    path: &[Vec3],              // world-space points defining the path
    scales: Option<&[Vec3]>,    // optional per-point scale factors
) -> SMeshResult<Vec<FaceId>>   // face at each path point
```

**Algorithm:**
1. At each path point, extrude the current face.
2. Translate extruded vertices to the path point (maintaining face shape relative to path tangent).
3. Optionally scale the face at each step.
4. Compute smooth tangent frames along the path (Frenet or minimum rotation) for consistent orientation.

**Use case:** Tails, tentacles, horns, spines, winding tubes. Currently requires manual extrude+translate+scale chains that are tedious and error-prone.

---

### 8. Region Face Selection

**Operation:** `mesh.select_region(center, radius, direction)`

Selects a connected patch of faces near a point, optionally filtered by normal direction. Guarantees the result is extrusion-safe (connected, no topology conflicts).

```rust
pub fn select_region(
    &self,
    center: Vec3,
    radius: f32,
    normal_filter: Option<(Vec3, f32)>,  // (direction, max_angle)
) -> Vec<FaceId>
```

**Algorithm:**
1. Find seed face (closest centroid to center).
2. Flood-fill to neighboring faces within radius.
3. Optionally filter by face normal direction.
4. Validate the selection is topologically safe for extrusion (connected, no pinch points).

**Use case:** Replaces the current verbose pattern of `faces_facing` + centroid filter + sort + truncate that's needed before every extrusion.

---

## P2 — Preview / Debug Improvements

### 9. Composite Preview Image

**Operation:** `mesh.save_composite_preview(opts, path)`

Renders a single image with 2x2 grid of views (front, right, top, diagonal) instead of 4 separate files.

```rust
pub fn save_composite_preview(
    &self,
    opts: &PreviewOptions,
    path: &str,              // single output path
) -> Result<String, Box<dyn Error>>
```

**Use case:** AI agents burn 4 tool calls and significant context reading 4 images. A single composite image reduces this to 1 call. This directly speeds up the sculpt-preview-adjust iteration loop.

---

### 10. Annotated Preview

Add optional annotations to preview renders:

```rust
pub struct PreviewOptions {
    // ... existing fields ...
    pub show_normals: bool,
    pub highlight_selection: Option<MeshSelection>,
    pub labels: Vec<(Vec3, String)>,  // world-pos + text label
}
```

**Use case:** When an extrusion goes wrong, it's hard to tell *which* part of the mesh is broken from a flat shaded render. Highlighted selections and normal visualization help diagnose issues.

---

### 11. Preview Checkpoints

**Operation:** `mesh.save_checkpoint(name)` / `mesh.restore_checkpoint(name)`

Saves a snapshot of the mesh state that can be previewed or restored later.

```rust
pub fn save_checkpoint(&self, name: &str) -> SMesh  // returns clone
pub fn preview_checkpoint(
    &self,
    name: &str,
    opts: &PreviewOptions,
    dir: &str,
) -> Result<Vec<String>, Box<dyn Error>>
```

**Use case:** When step 5 of a 10-step sculpt fails, the agent can preview the checkpoint from step 4 to understand what went wrong without re-running the whole generation.

---
