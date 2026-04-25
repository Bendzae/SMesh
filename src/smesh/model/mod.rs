//! Core data types for the halfedge mesh.
//!
//! - [`mesh_elements`]: primitive element types — [`VertexId`](mesh_elements::VertexId),
//!   [`HalfedgeId`](mesh_elements::HalfedgeId), [`FaceId`](mesh_elements::FaceId) and
//!   their payload structs.
//! - [`connectivity`]: the three [`SlotMap`](slotmap::SlotMap)s that hold connectivity.
//! - [`mesh`]: [`SMesh`](mesh::SMesh) itself — connectivity plus positions, normals,
//!   UVs, and user-defined attributes.

pub mod connectivity;
pub mod mesh;
pub mod mesh_elements;
