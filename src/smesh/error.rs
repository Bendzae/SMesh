use crate::smesh::*;

#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Other
    DefaultError,
}

pub type SMeshResult<T> = Result<T, SMeshError>;

#[macro_export]
macro_rules! bail {
    ($error:ident) => {
        return Err(SMeshError::$error);
    };
    ($error:ident, $value:expr) => {
        return Err(SMeshError::$error($value));
    };
}
