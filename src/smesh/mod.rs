

use crate::smesh::mesh_query::*;

pub mod attribute;
pub mod edit_operations;
pub mod error;
pub mod iterators;
pub mod loops;
pub mod mesh_query;
pub mod model;
pub mod primitives;
pub mod selection;
pub mod topological_operations;
pub mod transform;
pub mod util;
pub mod uv_operations;

#[cfg(feature = "xatlas")]
pub mod xatlas_integration;
