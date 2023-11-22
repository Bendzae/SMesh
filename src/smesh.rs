use glam::{Vec2, Vec3};
use itertools::Itertools;
use slotmap::{new_key_type, SecondaryMap, SlotMap};

use crate::bail;
use crate::smesh::error::*;
use crate::smesh::query::*;

pub mod attribute;
pub mod error;
pub mod iterators;
pub mod mesh_query;
pub mod query;
pub mod topological_operations;

new_key_type! { pub struct VertexId; }
new_key_type! { pub struct HalfedgeId; }
new_key_type! { pub struct EdgeId; }
new_key_type! { pub struct FaceId; }

#[derive(Debug, Default, Clone)]
pub struct Vertex {
    pub halfedge: Option<HalfedgeId>,
}

#[derive(Debug, Default, Clone)]
pub struct Halfedge {
    pub vertex: VertexId,
    pub edge: Option<EdgeId>,
    pub face: Option<FaceId>,
    pub opposite: Option<HalfedgeId>,
    pub prev: Option<HalfedgeId>,
    pub next: Option<HalfedgeId>,
}

#[derive(Debug, Default, Clone)]
pub struct Edge {
    pub halfedge: Option<HalfedgeId>,
}

#[derive(Debug, Default, Clone)]
pub struct Face {
    pub halfedge: Option<HalfedgeId>,
}

#[derive(Debug, Clone, Default)]
pub struct Connectivity {
    pub vertices: SlotMap<VertexId, Vertex>,
    pub halfedges: SlotMap<HalfedgeId, Halfedge>,
    pub edges: SlotMap<EdgeId, EdgeId>,
    pub faces: SlotMap<FaceId, Face>,
}

impl Connectivity {
    pub fn vert_mut(&mut self, id: VertexId) -> &mut Vertex {
        self.vertices.get_mut(id).unwrap()
    }
    pub fn he_mut(&mut self, id: HalfedgeId) -> &mut Halfedge {
        self.halfedges.get_mut(id).unwrap()
    }
    pub fn face_mut(&mut self, id: FaceId) -> &mut Face {
        self.faces.get_mut(id).unwrap()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SMesh {
    pub connectivity: Connectivity,

    // Attributes
    pub positions: SecondaryMap<VertexId, Vec3>,
    pub normals: Option<SecondaryMap<VertexId, Vec3>>,
    pub uvs: Option<SecondaryMap<VertexId, Vec2>>,
}

/// Init, Getters
impl SMesh {
    pub fn new() -> Self {
        Self {
            connectivity: Connectivity::default(),
            ..Default::default()
        }
    }
    pub fn vertices(&self) -> &SlotMap<VertexId, Vertex> {
        &self.connectivity.vertices
    }
    pub fn halfedges(&self) -> &SlotMap<HalfedgeId, Halfedge> {
        &self.connectivity.halfedges
    }
    pub fn faces(&self) -> &SlotMap<FaceId, Face> {
        &self.connectivity.faces
    }
    pub fn vertices_mut(&mut self) -> &mut SlotMap<VertexId, Vertex> {
        &mut self.connectivity.vertices
    }
    pub fn halfedges_mut(&mut self) -> &mut SlotMap<HalfedgeId, Halfedge> {
        &mut self.connectivity.halfedges
    }
    pub fn faces_mut(&mut self) -> &mut SlotMap<FaceId, Face> {
        &mut self.connectivity.faces
    }
    pub fn vert_mut(&mut self, id: VertexId) -> &mut Vertex {
        self.vertices_mut().get_mut(id).unwrap()
    }
    pub fn he_mut(&mut self, id: HalfedgeId) -> &mut Halfedge {
        self.halfedges_mut().get_mut(id).unwrap()
    }
    pub fn face_mut(&mut self, id: FaceId) -> &mut Face {
        self.faces_mut().get_mut(id).unwrap()
    }
    pub fn vert_mutator(&mut self, id: VertexId) -> MeshMutator<VertexId> {
        MeshMutator {
            conn: &mut self.connectivity,
            value: id,
        }
    }
    pub fn he_mutator(&mut self, id: HalfedgeId) -> MeshMutator<HalfedgeId> {
        MeshMutator {
            conn: &mut self.connectivity,
            value: id,
        }
    }
    pub fn face_mutator(&mut self, id: FaceId) -> MeshMutator<FaceId> {
        MeshMutator {
            conn: &mut self.connectivity,
            value: id,
        }
    }
}

/// Operations for adding mesh elements
impl SMesh {
    /// Create an isolated vertex to the mesh
    pub fn add_vertex(&mut self, position: Vec3) -> VertexId {
        let id = self.vertices_mut().insert(Vertex::default());
        self.positions.insert(id, position);
        id
    }

    /// Create an edge (2 halfedges) between two isolated vertices
    /// CARE!: This does not take care of connectivity for next/prev edges
    pub fn add_edge(&mut self, v0: VertexId, v1: VertexId) -> (HalfedgeId, HalfedgeId) {
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

    /// Construct a new face from a list of existing vertices
    /// Takes care of connectivity
    pub fn add_face(&mut self, vertices: Vec<VertexId>) -> SMeshResult<FaceId> {
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
                    let (he_id, _) = self.add_edge(*v0, *v1);
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
                    let patch_end = inner_next.prev().run(self)?;

                    // save relink info
                    next_cache.push((boundary_prev, patch_start));
                    next_cache.push((patch_end, boundary_next));
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
                            let boundary_prev = boundary_next.prev().run(self)?;
                            next_cache.push((boundary_prev, outer_next));
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
            self.vert_mutator(v_id).adjust_outgoing_halfedge()?;
        }

        Ok(face_id)
    }
}

pub struct MeshMutator<'a, T> {
    conn: &'a mut Connectivity,
    value: T,
}

/// Vertex mut ops
impl MeshMutator<'_, VertexId> {
    /// Set outgoing halfedge
    pub fn set_halfedge(&mut self, id: Option<HalfedgeId>) {
        self.conn.vert_mut(self.value).halfedge = id;
    }

    /// Set outgoing halfedge to boundary edge if one exists
    pub(crate) fn adjust_outgoing_halfedge(&mut self) -> SMeshResult<()> {
        let initial_h = self.value.halfedge().run(self.conn)?;
        let mut h = initial_h;

        loop {
            if h.is_boundary_c(self.conn) {
                self.set_halfedge(Some(h));
                break;
            }
            h = h.cw_rotated_neighbour().run(self.conn)?;
            if h == initial_h {
                break;
            }
        }
        Ok(())
    }
}

/// Halfedge mut ops
impl MeshMutator<'_, HalfedgeId> {
    /// Set "next" id for this halfedge, and inversely the "prev" id for the next
    pub fn set_next(&mut self, next: Option<HalfedgeId>) {
        let he = self.value;
        self.conn.he_mut(he).next = next;
        if let Some(next) = next {
            self.conn.he_mut(next).prev = Some(self.value);
        }
    }

    /// Set "prev" id for this halfedge, and inversely the "next" id for the prev
    pub fn set_prev(&mut self, prev: Option<HalfedgeId>) {
        let he = self.value;
        self.conn.he_mut(he).prev = prev;
        if let Some(prev) = prev {
            self.conn.he_mut(prev).next = Some(self.value);
        }
    }

    /// Set "opposite" id for this halfedge, and this edge as "opposite" for the other
    pub fn set_opposite(&mut self, opposite: HalfedgeId) {
        let he = self.value;
        self.conn.he_mut(he).opposite = Some(opposite);
        self.conn.he_mut(opposite).opposite = Some(he);
    }

    /// Set the dst vertex id
    pub fn set_vertex(&mut self, vertex: VertexId) {
        self.conn.he_mut(self.value).vertex = vertex;
    }

    /// Set the face id
    pub fn set_face(&mut self, face: Option<FaceId>) {
        self.conn.he_mut(self.value).face = face;
    }
}

/// Face mut ops
impl MeshMutator<'_, FaceId> {
    /// Set halfedge
    pub fn set_halfedge(&mut self, id: Option<HalfedgeId>) {
        self.conn.face_mut(self.value).halfedge = id;
    }
}
