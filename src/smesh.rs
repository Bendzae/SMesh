use crate::bail;
use crate::error::{SMeshError, SMeshResult};
use crate::mesh_query::{EvalMeshQuery, MeshQuery};
use glam::{Vec2, Vec3};
use itertools::Itertools;
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use std::rc::Rc;

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
    pub connectivity: Rc<Connectivity>,

    // Attributes
    pub positions: SecondaryMap<VertexId, Vec3>,
    pub normals: Option<SecondaryMap<VertexId, Vec3>>,
    pub uvs: Option<SecondaryMap<VertexId, Vec2>>,
}

/// Initialization
impl SMesh {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Add elements
impl SMesh {
    pub fn connectivity_mut(&mut self) -> &mut Connectivity {
        Rc::get_mut(&mut self.connectivity).unwrap()
    }
    pub fn vertices(&mut self) -> &mut SlotMap<VertexId, Vertex> {
        &mut self.connectivity_mut().vertices
    }
    pub fn halfedges(&mut self) -> &mut SlotMap<HalfedgeId, Halfedge> {
        &mut self.connectivity_mut().halfedges
    }
    pub fn faces(&mut self) -> &mut SlotMap<FaceId, Face> {
        &mut self.connectivity_mut().faces
    }

    pub fn add_vertex(&mut self, position: Vec3) -> VertexId {
        let id = self.vertices().insert(Vertex::default());
        self.positions.insert(id, position);
        id
    }

    pub fn add_edge(&mut self, v0: VertexId, v1: VertexId) -> HalfedgeId {
        let he_0_id = self.halfedges().insert(Halfedge::default());
        let he_1_id = self.halfedges().insert(Halfedge::default());
        let mut he_0 = self.halfedges().get_mut(he_0_id).unwrap();
        he_0.vertex = v0;
        he_0.opposite = Some(he_1_id);
        let mut he_1 = self.halfedges().get_mut(he_1_id).unwrap();
        he_1.vertex = v1;
        he_1.opposite = Some(he_0_id);
        he_0_id
    }

    pub fn add_face(&mut self, vertices: Vec<VertexId>) -> SMeshResult<FaceId> {
        let n = vertices.len();
        if n < 3 {
            bail!(DefaultError);
        }

        let mut halfedeges = Vec::with_capacity(n);

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
                    let he_id = self.add_edge(*v0, *v1);
                    halfedeges.push((he_id, true));
                }
            }
        }
        // re-link patches if necessary
        for ((inner_prev, prev_new), (inner_next, next_new)) in
            halfedeges.iter().circular_tuple_windows()
        {
            if !prev_new && !next_new {
                let next = self.q(*inner_prev).next().id();
                if next != Ok(*inner_next) {
                    // here comes the ugly part... we have to relink a whole patch

                    // search a free gap
                    // free gap will be between boundaryPrev and boundaryNext
                    let outer_prev = self.q(*inner_next).opposite();
                    let outer_next = self.q(*inner_prev).opposite();
                    let mut boundary_prev = outer_prev.clone();
                    loop {
                        let b = boundary_prev.next().opposite();
                        boundary_prev = b;
                    }
                }
            }
        }
        todo!()
    }
}
