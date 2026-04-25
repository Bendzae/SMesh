use std::collections::HashMap;

use attribute::CustomAttributeMap;
use glam::{Vec2, Vec3};
use itertools::Itertools;
use slotmap::{SecondaryMap, SlotMap};

use crate::{bail, prelude::*};

/// A halfedge (polygon) mesh: connectivity plus attribute storage.
///
/// An `SMesh` is the main handle you interact with. It bundles:
///
/// - **Topology** — stored internally as three [`SlotMap`]s of vertices,
///   halfedges, and faces. See [`Connectivity`].
/// - **Geometry** — [`positions`](Self::positions) map each [`VertexId`] to a
///   world-space [`Vec3`]. This is the only attribute that always exists.
/// - **Derived attributes** — [`face_normals`](Self::face_normals),
///   [`vertex_normals`](Self::vertex_normals), [`vertex_uvs`](Self::vertex_uvs)
///   and [`halfedge_uvs`](Self::halfedge_uvs) are lazily populated (e.g. by
///   [`recalculate_normals`](Self::recalculate_normals) or UV ops).
/// - **User attributes** — named typed maps keyed by element id
///   ([`vertex_attributes`](Self::vertex_attributes) and siblings).
/// - **Tags** — named [`MeshSelection`]s retrievable by string; managed via
///   [`tag`](Self::tag) and [`get_tag`](Self::get_tag).
///
/// All editing methods take `&mut self`, live as extension impls in other
/// modules (see [`crate::smesh::edit_operations`], [`crate::smesh::transform`]),
/// and are brought into scope by `use smesh::prelude::*;`.
///
/// ```
/// use glam::vec3;
/// use smesh::prelude::*;
///
/// let mut mesh = SMesh::new();
/// let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
/// let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
/// let v2 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
/// mesh.make_triangle(v0, v1, v2).unwrap();
/// mesh.recalculate_normals().unwrap();
/// assert_eq!(mesh.vertices().len(), 3);
/// assert_eq!(mesh.faces().len(), 1);
/// ```
#[derive(Debug, Clone, Default)]
pub struct SMesh {
    pub(crate) connectivity: Connectivity,

    /// Per-vertex world-space positions. Required for every non-isolated vertex.
    pub positions: SecondaryMap<VertexId, Vec3>,
    /// Per-face normals. Populated by
    /// [`recalculate_normals`](Self::recalculate_normals).
    pub face_normals: Option<SecondaryMap<FaceId, Vec3>>,
    /// Per-vertex normals (area-weighted average of adjacent face normals).
    /// Populated by [`recalculate_normals`](Self::recalculate_normals).
    pub vertex_normals: Option<SecondaryMap<VertexId, Vec3>>,
    /// Per-vertex UV coordinates. Use this when every incident halfedge at a
    /// vertex shares the same UV. See also [`halfedge_uvs`](Self::halfedge_uvs)
    /// for UV seams.
    pub vertex_uvs: Option<SecondaryMap<VertexId, Vec2>>,
    /// Per-halfedge UV coordinates. Allows distinct UVs across seams (e.g.
    /// island boundaries on a cube). Only inner halfedges (those belonging to
    /// a face) carry UVs.
    pub halfedge_uvs: Option<SecondaryMap<HalfedgeId, Vec2>>,
    /// Named typed attribute maps keyed by [`VertexId`].
    /// Created via [`add_attribute_map::<VertexId>`](Self::add_attribute_map).
    pub vertex_attributes: HashMap<String, CustomAttributeMap<VertexId>>,
    /// Named typed attribute maps keyed by [`HalfedgeId`] (per-edge storage).
    pub edge_attributes: HashMap<String, CustomAttributeMap<HalfedgeId>>,
    /// Named typed attribute maps keyed by [`FaceId`].
    pub face_attributes: HashMap<String, CustomAttributeMap<FaceId>>,

    /// Named selections. Managed through [`tag`](Self::tag),
    /// [`get_tag`](Self::get_tag) and friends; not meant to be accessed
    /// directly.
    pub(crate) tags: HashMap<String, MeshSelection>,
}

/// Construction and direct element access.
impl SMesh {
    /// Create an empty mesh with no vertices, halfedges, or faces.
    pub fn new() -> Self {
        Self {
            connectivity: Connectivity::default(),
            ..Default::default()
        }
    }

    /// Iterate over every [`VertexId`] currently stored in the mesh.
    pub fn vertices(&self) -> slotmap::basic::Keys<'_, VertexId, Vertex> {
        self.connectivity.vertices.keys()
    }

    /// Iterate over every [`HalfedgeId`] (two per undirected edge).
    pub fn halfedges(&self) -> slotmap::basic::Keys<'_, HalfedgeId, Halfedge> {
        self.connectivity.halfedges.keys()
    }

    /// Iterate over every [`FaceId`] currently stored in the mesh.
    pub fn faces(&self) -> slotmap::basic::Keys<'_, FaceId, Face> {
        self.connectivity.faces.keys()
    }

    /// Mutable access to the underlying vertex slot map. Prefer higher-level
    /// editing ops unless you're implementing a primitive or low-level tool.
    pub fn vertices_mut(&mut self) -> &mut SlotMap<VertexId, Vertex> {
        &mut self.connectivity.vertices
    }

    /// Mutable access to the underlying halfedge slot map. See
    /// [`vertices_mut`](Self::vertices_mut) for usage guidance.
    pub fn halfedges_mut(&mut self) -> &mut SlotMap<HalfedgeId, Halfedge> {
        &mut self.connectivity.halfedges
    }

    /// Mutable access to the underlying face slot map. See
    /// [`vertices_mut`](Self::vertices_mut) for usage guidance.
    pub fn faces_mut(&mut self) -> &mut SlotMap<FaceId, Face> {
        &mut self.connectivity.faces
    }

    /// Unchecked mutable access to a [`Vertex`] payload.
    ///
    /// # Panics
    /// Panics if `id` does not exist. Use
    /// [`vertices_mut().get_mut(id)`](SlotMap::get_mut) or the
    /// [`Connectivity::vert_mut`](crate::prelude::Connectivity::vert_mut)
    /// helper for a checked alternative.
    pub fn vert_mut(&mut self, id: VertexId) -> &mut Vertex {
        self.vertices_mut().get_mut(id).unwrap()
    }

    /// Unchecked mutable access to a [`Halfedge`] payload. Panics on stale id.
    pub fn he_mut(&mut self, id: HalfedgeId) -> &mut Halfedge {
        self.halfedges_mut().get_mut(id).unwrap()
    }

    /// Unchecked mutable access to a [`Face`] payload. Panics on stale id.
    pub fn face_mut(&mut self, id: FaceId) -> &mut Face {
        self.faces_mut().get_mut(id).unwrap()
    }

    /// Obtain a [`MeshMutator`] for fluent connectivity edits on the given
    /// element. Only borrows [`Connectivity`], leaving attribute maps free to
    /// mutate in parallel code paths.
    pub fn get_mut<T>(&mut self, id: T) -> MeshMutator<'_, T> {
        MeshMutator {
            conn: &mut self.connectivity,
            value: id,
        }
    }
}

/// Operations for adding mesh elements.
impl SMesh {
    /// Add an isolated vertex at `position` and return its new [`VertexId`].
    ///
    /// The vertex has no halfedges until it becomes part of a face via
    /// [`make_face`](Self::make_face) or one of its wrappers.
    ///
    /// ```
    /// use glam::vec3;
    /// use smesh::prelude::*;
    ///
    /// let mesh = &mut SMesh::new();
    /// let v = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
    /// assert!(v.is_isolated(mesh));
    /// ```
    pub fn add_vertex(&mut self, position: Vec3) -> VertexId {
        let id = self.vertices_mut().insert(Vertex::default());
        self.positions.insert(id, position);
        id
    }

    /// Construct a triangular face `(v0, v1, v2)`. Winding order determines
    /// the face normal — CCW produces an outward normal under the right-hand
    /// rule. Thin wrapper over [`make_face`](Self::make_face).
    pub fn make_triangle(
        &mut self,
        v0: VertexId,
        v1: VertexId,
        v2: VertexId,
    ) -> SMeshResult<FaceId> {
        self.make_face(vec![v0, v1, v2])
    }

    /// Construct a quad face `(v0, v1, v2, v3)`. Thin wrapper over
    /// [`make_face`](Self::make_face); winding rules are identical.
    pub fn make_quad(
        &mut self,
        v0: VertexId,
        v1: VertexId,
        v2: VertexId,
        v3: VertexId,
    ) -> SMeshResult<FaceId> {
        self.make_face(vec![v0, v1, v2, v3])
    }

    /// Construct an n-gon face from a list of existing vertices.
    ///
    /// The vertex sequence defines the face loop in the intended winding
    /// direction (CCW when viewed from the outside of a manifold mesh).
    /// Missing halfedges are created, existing ones are reused, and
    /// neighbouring halfedge links are patched so the mesh remains a valid
    /// halfedge structure.
    ///
    /// # Errors
    /// - [`SMeshError::DefaultError`] if fewer than three vertices are given.
    /// - [`SMeshError::TopologyError`] if a new face here would make a vertex
    ///   non-manifold, or reuse an interior halfedge.
    pub fn make_face(&mut self, vertices: Vec<VertexId>) -> SMeshResult<FaceId> {
        let n = vertices.len();
        if n < 3 {
            bail!(DefaultError);
        }

        let mut halfedeges: Vec<(HalfedgeId, bool)> = Vec::with_capacity(n);
        let mut next_cache: Vec<(HalfedgeId, HalfedgeId)> = vec![];
        let mut needs_adjust: Vec<VertexId> = vec![];

        // test for topological errors and create new edges
        for (v0, v1) in vertices.iter().circular_tuple_windows() {
            if !(*v0).is_boundary(self) {
                bail!(TopologyError);
            }
            match (*v0).halfedge_to(*v1).run(self) {
                Ok(he_id) => {
                    // Halfedge already exists
                    if !he_id.is_boundary(self) {
                        bail!(TopologyError);
                    }
                    halfedeges.push((he_id, false));
                }
                Err(_) => {
                    // New halfedge
                    // TODO: Check if only one he should be added here?
                    let (he_id, _) = self.make_edge_internal(*v0, *v1);
                    halfedeges.push((he_id, true));
                }
            }
        }
        // re-link patches if necessary
        for ((inner_prev_id, prev_new), (inner_next_id, next_new)) in
            halfedeges.iter().circular_tuple_windows()
        {
            if !prev_new && !next_new {
                let inner_prev = *inner_prev_id;
                let inner_next = *inner_next_id;
                if inner_prev.next().run(self)? != inner_next {
                    // here comes the ugly part... we have to relink a whole patch

                    // search a free gap
                    // free gap will be between boundaryPrev and boundaryNext
                    let outer_prev = inner_next.opposite();
                    let outer_next = inner_prev.opposite();
                    let mut boundary_prev = outer_prev.run(self)?;
                    loop {
                        boundary_prev = boundary_prev.next().opposite().run(self)?;
                        if boundary_prev.is_boundary(self) || boundary_prev == inner_prev {
                            break;
                        }
                    }
                    let boundary_next = boundary_prev.next().run(self)?;

                    if !boundary_prev.is_boundary(self)
                        || !boundary_next.is_boundary(self)
                        || boundary_next == inner_next
                    {
                        bail!(TopologyError);
                    }

                    // other halfedges' ids
                    let patch_start = inner_prev.next().run(self)?;
                    let patch_end = inner_next.prev().run(self).ok();

                    // save relink info
                    next_cache.push((boundary_prev, patch_start));
                    next_cache.push((patch_end.unwrap(), boundary_next));
                    next_cache.push((inner_prev, inner_next));
                }
            }
        }

        // create the face
        let face = Face {
            halfedge: Some(halfedeges.get(n - 1).unwrap().0),
        };
        let face_id = self.faces_mut().insert(face);

        for (i, ii) in (0..n).circular_tuple_windows() {
            let v = vertices[ii];
            let (inner_prev, prev_new) = halfedeges[i];
            let (inner_next, next_new) = halfedeges[ii];

            if prev_new || next_new {
                let outer_prev = inner_next.opposite().run(self)?;
                let outer_next = inner_prev.opposite().run(self)?;

                if prev_new && !next_new {
                    let boundary_prev = inner_next.prev().run(self)?;
                    next_cache.push((boundary_prev, outer_next));
                    self.vert_mut(v).halfedge = Some(outer_next);
                }
                if !prev_new && next_new {
                    let boundary_next = inner_prev.next().run(self)?;
                    next_cache.push((outer_prev, boundary_next));
                    self.vert_mut(v).halfedge = Some(boundary_next);
                }
                if prev_new && next_new {
                    match v.halfedge().run(self) {
                        Ok(boundary_next) => {
                            if let Ok(boundary_prev) = boundary_next.prev().run(self) {
                                next_cache.push((boundary_prev, outer_next));
                            };
                            next_cache.push((outer_prev, boundary_next));
                        }
                        Err(_) => {
                            self.vert_mut(v).halfedge = Some(outer_next);
                            next_cache.push((outer_prev, outer_next))
                        }
                    }
                }
                // set inner link
                next_cache.push((inner_prev, inner_next));
            } else if v.halfedge().run(self)? == inner_next {
                needs_adjust.push(v);
            }

            // set face id
            self.he_mut(halfedeges[i].0).face = Some(face_id);
        }

        // process next halfedge cache
        for (first, second) in next_cache {
            self.he_mut(first).next = Some(second);
            self.he_mut(second).prev = Some(first);
        }

        for v_id in needs_adjust {
            self.get_mut(v_id).adjust_outgoing_halfedge()?;
        }

        Ok(face_id)
    }

    /// Low-level: create an edge (two paired halfedges) between `v0` and `v1`
    /// and return `(v0→v1, v1→v0)`.
    ///
    /// This is a building block for [`make_face`](Self::make_face); it only
    /// sets the `opposite` and destination vertex links. Callers are
    /// responsible for `next`/`prev`/face linkage. Prefer [`make_face`](Self::make_face)
    /// or [`make_quad`](Self::make_quad) unless you are implementing a
    /// custom topological operator.
    pub fn make_edge_internal(&mut self, v0: VertexId, v1: VertexId) -> (HalfedgeId, HalfedgeId) {
        let halfedges = self.halfedges_mut();
        let he_0_id = halfedges.insert(Halfedge::default());
        let he_1_id = halfedges.insert(Halfedge::default());
        let he_0 = halfedges.get_mut(he_0_id).unwrap();
        he_0.vertex = v1;
        he_0.opposite = Some(he_1_id);
        let he_1 = halfedges.get_mut(he_1_id).unwrap();
        he_1.vertex = v0;
        he_1.opposite = Some(he_0_id);
        (he_0_id, he_1_id)
    }
}

/// Fluent mutator returned by [`SMesh::get_mut`].
///
/// Parameterised by the element id type (`VertexId`, `HalfedgeId`, or
/// `FaceId`). Borrows only the connectivity, so attribute maps remain free for
/// use in the same scope.
pub struct MeshMutator<'a, T> {
    conn: &'a mut Connectivity,
    value: T,
}

/// Vertex-scoped mutations.
impl MeshMutator<'_, VertexId> {
    /// Overwrite this vertex's outgoing halfedge. Boundary preferences are
    /// maintained automatically by higher-level ops.
    pub fn set_halfedge(&mut self, id: Option<HalfedgeId>) -> SMeshResult<()> {
        self.conn.vert_mut(self.value)?.halfedge = id;
        Ok(())
    }

    /// If any outgoing halfedge of this vertex is a boundary halfedge, set it
    /// as the stored outgoing halfedge so iteration starts on the boundary.
    // Crate-private helper; the boundary-preferred invariant keeps
    // `set_halfedge` consumers simple.
    pub(crate) fn adjust_outgoing_halfedge(&mut self) -> SMeshResult<()> {
        let initial_h = self.value.halfedge().run(self.conn)?;
        let mut h = initial_h;

        loop {
            if h.is_boundary_c(self.conn) {
                self.set_halfedge(Some(h))?;
                break;
            }
            h = h.cw_rotated_neighbour().run(self.conn)?;
            if h == initial_h {
                break;
            }
        }
        Ok(())
    }

    /// Remove this vertex from the connectivity map.
    ///
    /// Does **not** detach incident halfedges — use
    /// [`SMesh::delete_vertex`](crate::prelude::SMesh::delete_vertex) for a
    /// safe high-level delete.
    pub fn delete(self) -> SMeshResult<()> {
        self.conn.vertices.remove(self.value);
        Ok(())
    }
}

/// Halfedge-scoped mutations.
///
/// Linking methods keep the paired direction consistent — e.g.
/// [`set_next`](Self::set_next) also updates the other halfedge's `prev`.
impl MeshMutator<'_, HalfedgeId> {
    /// Link this halfedge to `next` and update `next.prev` to match.
    pub fn set_next(&mut self, next: Option<HalfedgeId>) -> SMeshResult<()> {
        let he = self.value;
        self.conn.he_mut(he)?.next = next;
        if let Some(next) = next {
            self.conn.he_mut(next)?.prev = Some(self.value);
        }
        Ok(())
    }

    /// Link this halfedge to `prev` and update `prev.next` to match.
    pub fn set_prev(&mut self, prev: Option<HalfedgeId>) -> SMeshResult<()> {
        let he = self.value;
        self.conn.he_mut(he)?.prev = prev;
        if let Some(prev) = prev {
            self.conn.he_mut(prev)?.next = Some(self.value);
        }
        Ok(())
    }

    /// Set the opposite halfedge symmetrically (both halfedges point at each
    /// other).
    pub fn set_opposite(&mut self, opposite: HalfedgeId) -> SMeshResult<()> {
        let he = self.value;
        self.conn.he_mut(he)?.opposite = Some(opposite);
        self.conn.he_mut(opposite)?.opposite = Some(he);
        Ok(())
    }

    /// Set the destination vertex this halfedge points at.
    pub fn set_vertex(&mut self, vertex: VertexId) -> SMeshResult<()> {
        self.conn.he_mut(self.value)?.vertex = vertex;
        Ok(())
    }

    /// Set the face this halfedge bounds (use `None` for a boundary halfedge).
    pub fn set_face(&mut self, face: Option<FaceId>) -> SMeshResult<()> {
        self.conn.he_mut(self.value)?.face = face;
        Ok(())
    }

    /// Remove this halfedge and its opposite from the connectivity map.
    ///
    /// Does not relink surrounding halfedges; prefer
    /// [`SMesh::delete_only_edge`](crate::prelude::SMesh::delete_only_edge) or
    /// higher-level ops.
    pub fn delete(self) -> SMeshResult<()> {
        if let Ok(o) = self.value.opposite().run(self.conn) {
            self.conn.halfedges.remove(o);
        }
        self.conn.halfedges.remove(self.value);
        Ok(())
    }
}

/// Face-scoped mutations.
impl MeshMutator<'_, FaceId> {
    /// Set the halfedge used as the face's "entry point" when iterating.
    pub fn set_halfedge(&mut self, id: Option<HalfedgeId>) -> SMeshResult<()> {
        self.conn.face_mut(self.value)?.halfedge = id;
        Ok(())
    }

    /// Remove this face from the connectivity map.
    ///
    /// Does not update the bounding halfedges — they continue to reference the
    /// now-missing face id. Use
    /// [`SMesh::delete_only_face`](crate::prelude::SMesh::delete_only_face)
    /// for a safe delete that turns the halfedges into boundaries.
    pub fn delete(self) -> SMeshResult<()> {
        self.conn.faces.remove(self.value);
        Ok(())
    }
}
