//! Re-exports of every commonly used type, trait, and function in the library.
//!
//! Importing `smesh::prelude::*` brings the full public API into scope:
//! [`SMesh`], element IDs ([`VertexId`], [`HalfedgeId`], [`FaceId`]), the query DSL
//! traits ([`VertexOps`], [`HalfedgeOps`], [`FaceOps`], [`RunQuery`]), iterators,
//! error types, selections, spatial queries, validation, UV ops, and all editing
//! extension methods on [`SMesh`].
//!
//! Anything that is not re-exported here is considered an internal implementation
//! detail and may change without notice.

pub use crate::smesh::{
    error::*, introspection::*, iterators::*, mesh_query::*, model::connectivity::*,
    model::mesh::*, model::mesh_elements::*, selection::*, spatial_queries::*,
    uv_operations::*, validation::*, *,
};

pub use slotmap::SecondaryMap;
