//! Core library modules.
//!
//! Most items are re-exported through [`crate::prelude`]; importing the prelude
//! is almost always sufficient. This module is organised into logical layers:
//!
//! - **Data**: [`model`], [`attribute`], [`error`]
//! - **Navigation**: [`mesh_query`], [`iterators`], [`loops`]
//! - **Inspection**: [`introspection`], [`validation`], [`spatial_queries`]
//! - **Authoring**: [`edit_operations`], [`topological_operations`], [`transform`],
//!   [`selection`], [`tags`], [`primitives`], [`uv_operations`], [`util`]
//! - **Optional**: `preview` (feature `preview`),
//!   `xatlas_integration` (feature `xatlas`)

use crate::smesh::mesh_query::*;

pub mod attribute;
pub mod edit_operations;
pub mod error;
pub mod introspection;
pub mod iterators;
pub mod loops;
pub mod mesh_query;
pub mod model;
pub mod primitives;
pub mod selection;
pub mod spatial_queries;
pub mod tags;
pub mod topological_operations;
pub mod transform;
pub mod util;
pub mod uv_operations;
pub mod validation;

#[cfg(feature = "preview")]
pub mod preview;

#[cfg(feature = "xatlas")]
pub mod xatlas_integration;
