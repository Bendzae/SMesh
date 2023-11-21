use glam::{Vec2, Vec3};
use itertools::Itertools;
use slotmap::{new_key_type, SecondaryMap, SlotMap};

use crate::bail;
use crate::smesh::error::*;
use crate::smesh::mesh_query::*;

pub mod attribute;
pub mod error;
pub mod iterators;
pub mod mesh_query;
pub mod topological_operations;
mod query;

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
            if !self.q(*v0).is_boundary() {
                bail!(TopologyError);
            }
            match self.q(*v0).halfedge_to(*v1).id() {
                Ok(he_id) => {
                    // Halfedge already exists
                    if !self.q(he_id).is_boundary() {
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
                let inner_prev: MeshQuery<HalfedgeId> = self.q(*inner_prev_id);
                let inner_next: MeshQuery<HalfedgeId> = self.q(*inner_next_id);
                if inner_prev.next() != inner_next {
                    // here comes the ugly part... we have to relink a whole patch

                    // search a free gap
                    // free gap will be between boundaryPrev and boundaryNext
                    let outer_prev = inner_next.opposite();
                    let outer_next = inner_prev.opposite();
                    let mut boundary_prev = outer_prev.id()?;
                    loop {
                        boundary_prev = self.q(boundary_prev).next().opposite().id()?;
                        if !self.q(boundary_prev).is_boundary()
                            || boundary_prev == inner_prev.id()?
                        {
                            break;
                        }
                    }
                    let boundary_next = self.q(boundary_prev).next().id()?;

                    if !self.q(boundary_prev).is_boundary()
                        || !self.q(boundary_next).is_boundary()
                        || boundary_next == inner_next.id()?
                    {
                        bail!(TopologyError);
                    }

                    // other halfedges' ids
                    let patch_start = inner_prev.next().id()?;
                    let patch_end = inner_next.prev().id()?;

                    // save relink info
                    next_cache.push((boundary_prev, patch_start));
                    next_cache.push((patch_end, boundary_next));
                    next_cache.push((inner_prev.id()?, inner_next.id()?));
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
                let outer_prev = self.q(inner_next).opposite().id()?;
                let outer_next = self.q(inner_prev).opposite().id()?;

                if prev_new && !next_new {
                    let boundary_prev = self.q(inner_next).prev().id()?;
                    next_cache.push((boundary_prev, outer_next));
                    self.vert_mut(v).halfedge = Some(outer_next);
                }
                if !prev_new && next_new {
                    let boundary_next = self.q(inner_prev).next().id()?;
                    next_cache.push((outer_prev, boundary_next));
                    self.vert_mut(v).halfedge = Some(boundary_next);
                }
                if prev_new && next_new {
                    match self.q(v).halfedge().id() {
                        Ok(boundary_next) => {
                            let boundary_prev = self.q(boundary_next).prev().id()?;
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
            } else if self.q(v).halfedge().id()? == inner_next {
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
            self.adjust_outgoing_halfedge(v_id)?;
        }

        Ok(face_id)
    }

    pub(crate) fn adjust_outgoing_halfedge(&mut self, vertex_id: VertexId) -> SMeshResult<()> {
        let initial_h = self.q(vertex_id).halfedge().id();
        let mut h = initial_h;

        loop {
            if self.q(h?).is_boundary() {
                self.vert_mut(vertex_id).halfedge = Some(h?);
                break;
            }
            h = self.q(h?).cw_rotated_neighbour().id();
            if h == initial_h {
                break;
            }
        }
        Ok(())
    }
}
