# AI Agent API Improvements

Improvements to smesh that optimize for AI agent workflows when creating complex procedural meshes. The core insight: an agent's main limitation with procedural geometry isn't generating the code — it's verifying the result.

## P0 - Critical

### Mesh Introspection (`describe()` / `MeshReport`)
- Structured textual summary of a mesh an agent can parse
- Vertex/face/edge counts, bounding box, dimensions, center of mass
- Topology info: is_closed, is_manifold, boundary loops, connected components
- Face type breakdown (triangles, quads, n-gons)
- `Display` impl so it can be printed and read
- Per-face spatial report (`describe_faces()`) for debugging specific regions

### Mesh Validation with Actionable Diagnostics (`validate()`)
- Whole-mesh validator returning a list of `MeshIssue`s
- Issue types: degenerate faces, non-manifold vertices, flipped normals, duplicate vertices, zero-length edges, isolated vertices, non-planar faces, inconsistent winding
- Each issue references the specific element (FaceId, VertexId, etc.) so the agent can fix it

## P1 - High Value

### Semantic Face/Vertex Groups (Tags)
- Tag selections with string names during construction
- Retrieve tagged selections later by name
- Prevents the #1 agent failure mode: losing track of element IDs across many operations

### Spatial Queries
- `query_region(center, radius)` - find elements near a point
- `nearest_vertex(point)` - closest vertex lookup
- `raycast(origin, direction)` - ray-mesh intersection
- `faces_facing(direction, threshold)` - find faces by normal direction

## P2 - Nice to Have

### Higher-Level Procedural Ops
- `extrude_along_normal(faces, distance)` - extrude faces along averaged normal
- `inset_faces(faces, amount)` - scale toward centroid without extrusion
- `bevel_edges(edges, width, segments)` - edge beveling
- `loop_cut(edge, t)` - insert edge loop at parameter

### Headless Render-to-Image
- Software rasterizer, no GPU required
- Flat-shaded render from configurable viewpoints
- Wireframe overlay option
- Enables visual verification for multimodal agents

## P3 - Future

### Snapshot/Diff
- Lightweight mesh snapshots
- Diff two states: "Added 12 vertices, 8 faces. Bounding box grew by +0.5 on Y"
- Verify each procedural step did what was expected
