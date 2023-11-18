use crate::error::SMeshError;
use glam::{Vec2, Vec3};
use slotmap::{new_key_type, SecondaryMap, SlotMap};

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
pub struct SMesh {
    // Connectivity
    pub vertices: SlotMap<VertexId, Vertex>,
    pub halfedges: SlotMap<HalfedgeId, Halfedge>,
    pub edges: SlotMap<EdgeId, EdgeId>,
    pub faces: SlotMap<FaceId, Face>,

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
    pub fn add_vertex(&mut self, position: Vec3) -> VertexId {
        let id = self.vertices.insert(Vertex::default());
        self.positions.insert(id, position);
        id
    }

    pub fn add_face(&mut self, vertices: Vec<VertexId>) -> Result<FaceId, SMeshError> {
        for v_id in vertices.iter() {
            if !self.q_vert(*v_id).is_boundary() {
                return Err(SMeshError::TopologyError);
            }
            let v = self.q_vert(*v_id).halfedge().run()?;
        }
        todo!()
    }
}
