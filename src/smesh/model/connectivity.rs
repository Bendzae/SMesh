use slotmap::SlotMap;

use crate::prelude::{model::mesh_elements::{Face, FaceId, Halfedge, HalfedgeId, Vertex, VertexId}, SMeshError};

#[derive(Debug, Clone, Default)]
pub struct Connectivity {
    pub vertices: SlotMap<VertexId, Vertex>,
    pub halfedges: SlotMap<HalfedgeId, Halfedge>,
    pub faces: SlotMap<FaceId, Face>,
}

impl Connectivity {
    pub fn vert_mut(&mut self, id: VertexId) -> Result<&mut Vertex, SMeshError> {
        self.vertices
            .get_mut(id)
            .ok_or(SMeshError::VertexNotFound(id))
    }
    pub fn he_mut(&mut self, id: HalfedgeId) -> Result<&mut Halfedge, SMeshError> {
        self.halfedges
            .get_mut(id)
            .ok_or(SMeshError::HalfedgeNotFound(id))
    }
    pub fn face_mut(&mut self, id: FaceId) -> Result<&mut Face, SMeshError> {
        self.faces.get_mut(id).ok_or(SMeshError::FaceNotFound(id))
    }
}
