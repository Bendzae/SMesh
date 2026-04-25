//! # SMesh
//!
//! A fast, ergonomic halfedge (polygon) mesh library for procedural 3D modeling.
//! Inspired by [pmp](https://www.pmp-library.org/) and aimed at interactive and batch
//! mesh authoring from code.
//!
//! ## Quickstart
//!
//! ```
//! use glam::vec3;
//! use smesh::prelude::*;
//!
//! let mut mesh = SMesh::new();
//! let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
//! let v1 = mesh.add_vertex(vec3( 1.0, 0.0, -1.0));
//! let v2 = mesh.add_vertex(vec3( 1.0, 0.0,  1.0));
//! let v3 = mesh.add_vertex(vec3(-1.0, 0.0,  1.0));
//! let face = mesh.make_quad(v0, v1, v2, v3).unwrap();
//! mesh.recalculate_normals().unwrap();
//! ```
//!
//! ## Module map
//!
//! All public items are re-exported by [`prelude`], so a single `use smesh::prelude::*;`
//! normally suffices. The library is internally organised as follows:
//!
//! | Module | What it provides |
//! |--------|------------------|
//! | [`smesh::model`] | Core types: [`SMesh`](prelude::SMesh), [`VertexId`](prelude::VertexId), [`HalfedgeId`](prelude::HalfedgeId), [`FaceId`](prelude::FaceId) |
//! | [`smesh::mesh_query`] | Chainable navigation DSL (`vertex.halfedge().next().face()`) |
//! | [`smesh::iterators`] | Walkers over neighbours (around vertex, around face) |
//! | [`smesh::introspection`] | [`describe`](prelude::SMesh::describe), [`describe_selection`](prelude::SMesh::describe_selection), reports |
//! | [`smesh::validation`] | [`validate`](prelude::SMesh::validate) — detect topology / geometry issues |
//! | [`smesh::spatial_queries`] | Raycasts, region selection, nearest vertex |
//! | [`smesh::selection`] | [`MeshSelection`](prelude::MeshSelection) — polymorphic sets of elements |
//! | [`smesh::tags`] | Named selections for retrieval by string |
//! | [`smesh::edit_operations`] | Extrude, inset, bevel, bridge, subdivide, collapse, merge, … |
//! | [`smesh::topological_operations`] | Low-level halfedge surgery (split, flip, delete) |
//! | [`smesh::transform`] | Translate, scale, rotate, spherize, smooth, proportional edit |
//! | [`smesh::attribute`] | Typed custom attributes on vertices / edges / faces |
//! | [`smesh::primitives`] | `Cube`, `Icosphere`, `Cylinder`, `Quad`, `Wedge`, `Circle` generators |
//! | [`smesh::uv_operations`] | UV planar projection, unwrapping, seams |
//! | [`smesh::util`] | [`recalculate_normals`](prelude::SMesh::recalculate_normals), centroid helpers |
//! | `smesh::preview` (feature `preview`) | In-memory software-rasterized PNG previews |
//! | `smesh::xatlas_integration` (feature `xatlas`) | UV unwrapping via xatlas |
//! | [`adapters::bevy`] (feature `bevy_adapter`) | Conversion to/from `bevy::Mesh`, debug-draw plugin |
//!
//! ## Mental model
//!
//! SMesh is a **halfedge** data structure. Every undirected edge is stored as two
//! opposite halfedges. A halfedge *points to* its destination vertex, belongs to
//! exactly one face (or `None` if it is a boundary), and has `next`, `prev`, and
//! `opposite` pointers. This lets you walk the mesh in O(1) per step without
//! storing explicit adjacency lists.
//!
//! Element identifiers ([`VertexId`](prelude::VertexId), [`HalfedgeId`](prelude::HalfedgeId),
//! [`FaceId`](prelude::FaceId)) are stable `slotmap` keys — deleting other elements does not
//! invalidate unrelated IDs. Positions, normals, UVs, and custom attributes live in
//! separate [`SecondaryMap`](slotmap::SecondaryMap)s keyed by those IDs.
//!
//! ## Navigating the mesh
//!
//! Build queries by chaining, then call `.run(&mesh)` or an eager method like
//! `.position(&mesh)`:
//!
//! ```
//! # use glam::vec3;
//! # use smesh::prelude::*;
//! # let mut mesh = SMesh::new();
//! # let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
//! # let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
//! # let v2 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
//! # mesh.make_triangle(v0, v1, v2).unwrap();
//! // Walk: v0 -> outgoing halfedge -> its next halfedge -> that halfedge's vertex
//! let target = v0.halfedge().next().vert().run(&mesh).unwrap();
//! // Eager lookup
//! let pos = v0.position(&mesh).unwrap();
//! ```
//!
//! See [`mesh_query`](smesh::mesh_query) for the full DSL.
//!
//! ## Cargo features
//!
//! - `bevy_adapter` (default) — pulls in [`bevy`] and enables [`adapters::bevy`].
//! - `preview` — adds `smesh::preview` for rendering a mesh to a PNG.
//! - `xatlas` — adds `smesh::xatlas_integration` for automatic UV unwrapping.

pub mod adapters;
pub mod prelude;
pub mod smesh;
mod test_utils;
mod tests;
