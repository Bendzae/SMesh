use crate::smesh::*;

#[derive(Debug, Clone, Copy)]
pub enum SMeshError {
    /// Query errors
    VertexNotFound(VertexId),
    VertexHasNoHalfEdge(VertexId),
    HalfedgeNotFound(HalfedgeId),
    HalfedgeHasNoRef(HalfedgeId),
    EdgeNotFound(EdgeId),
    FaceNotFound(FaceId),
    FaceHasNoHalfEdge(FaceId),
    /// Topology
    TopologyError,
}
