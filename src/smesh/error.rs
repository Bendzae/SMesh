//! Error type for mesh operations and a convenience [`bail!`](crate::bail) macro.
//!
//! Every fallible operation returns [`SMeshResult<T>`] — an alias for
//! `Result<T, SMeshError>`. Errors fall into three broad groups:
//!
//! 1. **Query errors** — a stale id, or the requested neighbour does not exist
//!    (e.g. asking for `next` on an unlinked halfedge). Usually indicates a
//!    programming mistake.
//! 2. **Topology errors** — an operation would produce invalid connectivity
//!    (e.g. creating a face that would result in a non-manifold vertex).
//! 3. **Custom / default** — catch-alls for operation-specific preconditions.

use crate::prelude::model::mesh_elements::{FaceId, HalfedgeId, VertexId};
use std::fmt::{Display, Formatter};
use thiserror::Error;

/// All errors that can be returned from SMesh operations.
#[derive(Error, Debug, Clone, Copy, PartialEq)]
pub enum SMeshError {
    // -- Query errors ---------------------------------------------------------
    /// The [`VertexId`] does not exist in this mesh.
    #[error("A Vertex with id `{0}` could not be found")]
    VertexNotFound(VertexId),
    /// The vertex is isolated — it has no outgoing halfedge.
    #[error("Vertex with id `{0}` has no associated halfedge")]
    VertexHasNoHalfEdge(VertexId),
    /// The id does not exist in this mesh.
    #[error("Halfedge with id `{0}` could not be found")]
    HalfedgeNotFound(HalfedgeId),
    /// The halfedge is a boundary halfedge with no face on this side.
    #[error("Halfedge with id `{0}` has no associated face")]
    HalfedgeHasNoFace(HalfedgeId),
    /// The halfedge has no `next` link (unlinked or mid-construction).
    #[error("Halfedge with id `{0}` has no next halfedge")]
    HalfedgeHasNoNext(HalfedgeId),
    /// The halfedge has no `prev` link (unlinked or mid-construction).
    #[error("Halfedge with id `{0}` has no prev halfedge")]
    HalfedgeHasNoPrev(HalfedgeId),
    /// The halfedge has no `opposite` pair (indicates broken connectivity).
    #[error("Halfedge with id `{0}` has no opposite halfedge")]
    HalfedgeHasNoOpposite(HalfedgeId),
    /// The [`FaceId`] does not exist in this mesh.
    #[error("A Face with id `{0}` could not be found")]
    FaceNotFound(FaceId),
    /// The face has no bounding halfedge (malformed / mid-construction).
    #[error("Face with id `{0}` has no associated halfedge")]
    FaceHasNoHalfEdge(FaceId),

    // -- Topology -------------------------------------------------------------
    /// The operation would produce invalid connectivity (e.g. attempting to
    /// build a face that would make a vertex non-manifold).
    #[error("Invalid mesh topology for this operation")]
    TopologyError,

    // -- Other ----------------------------------------------------------------
    /// The operation is not implemented for this element type / query path.
    #[error("Unsupported Operation")]
    UnsupportedOperation,
    /// Generic fallback used when the cause does not fit another variant.
    #[error("Default SMesh Error")]
    DefaultError,
    /// Operation-specific precondition failure carrying a human-readable message.
    #[error("Error: `{0}`")]
    CustomError(&'static str),
}

/// Short alias for `Result<T, SMeshError>` — returned by virtually every mesh operation.
pub type SMeshResult<T> = Result<T, SMeshError>;

/// Return an [`SMeshError`] from the current function.
///
/// Three forms:
///
/// - `bail!(VariantName)` — unit variant
/// - `bail!(VariantName, id)` — variant with a single id payload
/// - `bail!("message")` — shorthand for [`SMeshError::CustomError`]
///
/// ```
/// use smesh::prelude::*;
/// use smesh::bail;
///
/// fn demo() -> SMeshResult<()> {
///     if false { bail!(TopologyError); }
///     if false { bail!("unsupported configuration"); }
///     Ok(())
/// }
/// ```
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
