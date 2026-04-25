//! Element id types and their payload structs for the halfedge mesh.
//!
//! Every element (vertex, halfedge, face) is addressed by a stable
//! [`slotmap`]-generated key. Keys remain valid as long as the element exists,
//! even as other elements are added or removed. The small payload structs
//! ([`Vertex`], [`Halfedge`], [`Face`]) store just enough connectivity to let
//! you reach any neighbour in O(1).
//!
//! Most code only needs the ids and the query/iterator traits — you should
//! rarely need to touch these structs directly.

use slotmap::new_key_type;

new_key_type! {
    /// Stable id of a vertex in an [`SMesh`](crate::prelude::SMesh).
    ///
    /// Valid for the lifetime of the vertex. Use the methods from
    /// [`VertexOps`](crate::prelude::VertexOps) (via the prelude) to navigate:
    /// `v.halfedge()`, `v.position(&mesh)`, `v.is_boundary(&mesh)`, and so on.
    pub struct VertexId;
}

new_key_type! {
    /// Stable id of a directed halfedge.
    ///
    /// Each undirected edge is represented by **two** halfedges pointing in
    /// opposite directions. A halfedge points *to* its destination vertex
    /// (`he.vert()` / `he.dst_vert()`) and belongs to either a face or the
    /// outer boundary (`he.face()`). Walk the mesh with
    /// [`HalfedgeOps`](crate::prelude::HalfedgeOps).
    pub struct HalfedgeId;
}

new_key_type! {
    /// Stable id of a face.
    ///
    /// A face stores a single halfedge; the rest of its boundary is recovered
    /// by repeatedly following `next`. See
    /// [`FaceOps`](crate::prelude::FaceOps) for queries and
    /// [`FaceIterators`](crate::prelude::FaceIterators) for iteration.
    pub struct FaceId;
}

/// Vertex payload.
///
/// Stores a single *outgoing* halfedge. For boundary vertices this is always a
/// boundary halfedge, ensuring iteration with
/// [`HalfedgeAroundVertexIter`](crate::prelude::HalfedgeAroundVertexIter)
/// behaves consistently. Positions, normals, UVs, and user attributes live in
/// secondary maps on [`SMesh`](crate::prelude::SMesh), not here.
#[derive(Debug, Default, Clone)]
pub struct Vertex {
    /// An arbitrary outgoing halfedge (boundary-preferred on boundary verts).
    pub halfedge: Option<HalfedgeId>,
}

/// Halfedge payload.
///
/// A halfedge is a *directed* half of an edge. It points at its destination
/// vertex, belongs to one face (or to no face if it is a boundary), and links
/// to the surrounding halfedges via `opposite`, `prev`, and `next`.
#[derive(Debug, Default, Clone)]
pub struct Halfedge {
    /// The destination vertex (end-point of this directed halfedge).
    pub vertex: VertexId,
    /// The face this halfedge belongs to, or `None` on a boundary.
    pub face: Option<FaceId>,
    /// The opposite halfedge (same edge, reversed direction).
    pub opposite: Option<HalfedgeId>,
    /// Previous halfedge around the same face loop.
    pub prev: Option<HalfedgeId>,
    /// Next halfedge around the same face loop.
    pub next: Option<HalfedgeId>,
}

/// Reserved for future edge-centric storage.
///
/// SMesh currently tracks edges implicitly via halfedge pairs; this struct is
/// not used as storage today but is kept in the public API for forward
/// compatibility with per-edge attributes.
#[derive(Debug, Default, Clone)]
pub struct Edge {
    /// Arbitrary halfedge on the edge.
    pub halfedge: Option<HalfedgeId>,
}

/// Face payload.
///
/// Stores one halfedge of its boundary; the full boundary is recovered by
/// following `next` links.
#[derive(Debug, Default, Clone)]
pub struct Face {
    /// One of the halfedges bounding this face (CCW when viewed from the
    /// outside of a manifold mesh).
    pub halfedge: Option<HalfedgeId>,
}
