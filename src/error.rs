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

pub type SMeshResult<T> = Result<T, SMeshError>;

#[macro_export]
macro_rules! bail {
    ($error:expr) => {
        return Err($error);
    };
}
