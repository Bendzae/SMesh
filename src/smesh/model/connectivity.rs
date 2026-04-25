//! Raw connectivity storage for [`SMesh`](crate::prelude::SMesh).
//!
//! [`Connectivity`] holds the three slot maps that back vertex, halfedge, and
//! face storage. Most operations never touch this struct directly — they go
//! through [`SMesh`](crate::prelude::SMesh) — but some low-level halfedge edits
//! and the [`MeshMutator`](crate::prelude::MeshMutator) helpers borrow only
//! connectivity in order to avoid mutably borrowing the full mesh (and its
//! attribute maps) at the same time.

use slotmap::SlotMap;

use crate::prelude::{
    model::mesh_elements::{Face, FaceId, Halfedge, HalfedgeId, Vertex, VertexId},
    SMeshError,
};

/// The raw halfedge connectivity of a mesh.
///
/// This is the "topology only" subset of an [`SMesh`](crate::prelude::SMesh) —
/// positions and attributes are stored separately. Public so that queries
/// built via [`MeshQueryBuilder`](crate::prelude::MeshQueryBuilder) can run
/// against either a full mesh or a bare `Connectivity`.
#[derive(Debug, Clone, Default)]
pub struct Connectivity {
    /// All vertices in the mesh.
    pub vertices: SlotMap<VertexId, Vertex>,
    /// All halfedges in the mesh (two per undirected edge).
    pub halfedges: SlotMap<HalfedgeId, Halfedge>,
    /// All faces in the mesh.
    pub faces: SlotMap<FaceId, Face>,
}

impl Connectivity {
    /// Mutable access to a [`Vertex`], or
    /// [`SMeshError::VertexNotFound`] if the id is stale.
    pub fn vert_mut(&mut self, id: VertexId) -> Result<&mut Vertex, SMeshError> {
        self.vertices
            .get_mut(id)
            .ok_or(SMeshError::VertexNotFound(id))
    }

    /// Mutable access to a [`Halfedge`], or
    /// [`SMeshError::HalfedgeNotFound`] if the id is stale.
    pub fn he_mut(&mut self, id: HalfedgeId) -> Result<&mut Halfedge, SMeshError> {
        self.halfedges
            .get_mut(id)
            .ok_or(SMeshError::HalfedgeNotFound(id))
    }

    /// Mutable access to a [`Face`], or
    /// [`SMeshError::FaceNotFound`] if the id is stale.
    pub fn face_mut(&mut self, id: FaceId) -> Result<&mut Face, SMeshError> {
        self.faces.get_mut(id).ok_or(SMeshError::FaceNotFound(id))
    }
}
