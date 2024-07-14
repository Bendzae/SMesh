use crate::smesh::*;
use std::fmt::{Display, Formatter};
use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, PartialEq)]
pub enum SMeshError {
    /// Query errors
    #[error("A Vertex with id `{0}` could not be found")]
    VertexNotFound(VertexId),
    #[error("Vertex with id `{0}` has no associated halfedge")]
    VertexHasNoHalfEdge(VertexId),
    #[error("Halfedge with id `{0}` could not be found")]
    HalfedgeNotFound(HalfedgeId),
    #[error("Halfedge with id `{0}` has no associated face")]
    HalfedgeHasNoFace(HalfedgeId),
    #[error("Halfedge with id `{0}` has no next halfedge")]
    HalfedgeHasNoNext(HalfedgeId),
    #[error("Halfedge with id `{0}` has no prev halfedge")]
    HalfedgeHasNoPrev(HalfedgeId),
    #[error("Halfedge with id `{0}` has no opposite halfedge")]
    HalfedgeHasNoOpposite(HalfedgeId),
    #[error("A Face with id `{0}` could not be found")]
    FaceNotFound(FaceId),
    #[error("Face with id `{0}` has no associated halfedge")]
    FaceHasNoHalfEdge(FaceId),
    /// Topology
    #[error("Invalid mesh topology for this operation")]
    TopologyError,
    /// Other
    #[error("Unsupported Operation")]
    UnsupportedOperation,
    #[error("Default SMesh Error")]
    DefaultError,
    #[error("Error: `{0}`")]
    CustomError(&'static str),
}

pub type SMeshResult<T> = Result<T, SMeshError>;

#[macro_export]
macro_rules! bail {
    ($error:ident) => {
        return Err(SMeshError::$error)
    };
    ($error:ident, $value:expr) => {
        return Err(SMeshError::$error($value))
    };
    ($value:expr) => {
        return Err(SMeshError::CustomError($value))
    };
}

impl Display for VertexId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Display for HalfedgeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Display for FaceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
