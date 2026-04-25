//! A polymorphic set of mesh elements.
//!
//! A [`MeshSelection`] can hold any mix of [`VertexId`], [`HalfedgeId`], and
//! [`FaceId`] values. When an operation needs a specific element kind it
//! asks the selection to `resolve_to_*`, which promotes contained elements
//! (e.g. a selected face contributes all its vertices for
//! [`resolve_to_vertices`](MeshSelection::resolve_to_vertices)).
//!
//! Most editing ops accept anything convertible to `MeshSelection` via
//! `Into<MeshSelection>`, including:
//!
//! - single ids (`v0`, `he`, `f`)
//! - iterators and collections of ids (`Vec<VertexId>`, `HashSet<FaceId>`, …)
//! - tags retrieved from the mesh (`mesh.take_tag("backrest").unwrap()`)

use std::collections::HashSet;

use crate::prelude::*;
use bevy::utils::default;
use itertools::Itertools;

use super::mesh_query::{HalfedgeOps, RunQuery};

/// An ordered-set of vertices, halfedges, and/or faces.
///
/// Construct via `From`/`FromIterator` conversions, or start empty with
/// [`MeshSelection::new`] and [`insert`](MeshSelectionOps::insert) elements.
/// Resolve to a concrete element type via
/// [`resolve_to_vertices`](MeshSelection::resolve_to_vertices),
/// [`resolve_to_halfedges`](MeshSelection::resolve_to_halfedges), or
/// [`resolve_to_faces`](MeshSelection::resolve_to_faces).
#[derive(Debug, Clone, Default)]
pub struct MeshSelection {
    vertices: HashSet<VertexId>,
    halfedges: HashSet<HalfedgeId>,
    faces: HashSet<FaceId>,
}

impl MeshSelection {
    /// Create an empty selection.
    pub fn new() -> Self {
        MeshSelection::default()
    }

    /// Merge `other` into `self` (union of vertices, halfedges, and faces).
    pub fn merge(&mut self, other: &MeshSelection) {
        self.vertices.extend(&other.vertices);
        self.halfedges.extend(&other.halfedges);
        self.faces.extend(&other.faces);
    }

    /// Resolve to the set of vertices covered by this selection.
    ///
    /// Stored vertices are included directly; halfedges contribute both
    /// endpoints; faces contribute every bounding vertex.
    pub fn resolve_to_vertices(&self, smesh: &SMesh) -> SMeshResult<HashSet<VertexId>> {
        let mut vertices = self.vertices.clone();
        for he in &self.halfedges {
            vertices.insert(he.src_vert().run(smesh)?);
            vertices.insert(he.dst_vert().run(smesh)?);
        }
        for f in &self.faces {
            for v in f.vertices(smesh) {
                vertices.insert(v);
            }
        }
        Ok(vertices)
    }

    /// Resolve to the set of faces covered by this selection.
    ///
    /// Stored faces are included directly; halfedges contribute their face
    /// (skipped for boundary halfedges); vertices only contribute a face when
    /// *every* vertex of that face is also in the selection (prevents
    /// over-growing from a single corner).
    pub fn resolve_to_faces(&self, smesh: &SMesh) -> SMeshResult<HashSet<FaceId>> {
        let mut faces = self.faces.clone();
        for he in &self.halfedges {
            faces.insert(he.face().run(smesh)?);
        }
        for v in &self.vertices {
            for f in v.faces(smesh) {
                if f.vertices(smesh)
                    .all(|face_v| self.vertices.contains(&face_v))
                {
                    faces.insert(f);
                }
            }
        }
        Ok(faces)
    }

    /// Resolve to the set of halfedges covered by this selection.
    ///
    /// Stored halfedges are included directly; faces contribute every
    /// halfedge on their boundary; vertices contribute outgoing halfedges
    /// whose destination is also in the selection (i.e. the edges between
    /// selected vertices).
    pub fn resolve_to_halfedges(&self, smesh: &SMesh) -> SMeshResult<HashSet<HalfedgeId>> {
        let mut edges = self.halfedges.clone();
        for face in &self.faces {
            for he in face.halfedges(smesh) {
                edges.insert(he);
            }
        }
        for v in &self.vertices {
            for he in v.halfedges(smesh) {
                if self.vertices.contains(&he.dst_vert().run(smesh)?) {
                    edges.insert(he);
                }
            }
        }
        Ok(edges)
    }
}

/// Insert an element of type `T` into a [`MeshSelection`].
///
/// Implemented uniformly for [`VertexId`], [`HalfedgeId`], and [`FaceId`] so
/// `selection.insert(id)` works regardless of element kind.
pub trait MeshSelectionOps<T> {
    /// Add `item` to the selection. Duplicates are silently ignored.
    fn insert(&mut self, item: T);
}

impl MeshSelectionOps<VertexId> for MeshSelection {
    fn insert(&mut self, item: VertexId) {
        self.vertices.insert(item);
    }
}

impl MeshSelectionOps<HalfedgeId> for MeshSelection {
    fn insert(&mut self, item: HalfedgeId) {
        self.halfedges.insert(item);
    }
}

impl MeshSelectionOps<FaceId> for MeshSelection {
    fn insert(&mut self, item: FaceId) {
        self.faces.insert(item);
    }
}

impl From<VertexId> for MeshSelection {
    fn from(value: VertexId) -> Self {
        Self::from_iter(vec![value])
    }
}

impl From<HalfedgeId> for MeshSelection {
    fn from(value: HalfedgeId) -> Self {
        Self::from_iter(vec![value])
    }
}

impl From<FaceId> for MeshSelection {
    fn from(value: FaceId) -> Self {
        Self::from_iter(vec![value])
    }
}

impl FromIterator<VertexId> for MeshSelection {
    fn from_iter<T: IntoIterator<Item = VertexId>>(iter: T) -> Self {
        MeshSelection {
            vertices: HashSet::from_iter(iter),
            ..default()
        }
    }
}

impl FromIterator<HalfedgeId> for MeshSelection {
    fn from_iter<T: IntoIterator<Item = HalfedgeId>>(iter: T) -> Self {
        MeshSelection {
            halfedges: HashSet::from_iter(iter),
            ..default()
        }
    }
}

impl FromIterator<FaceId> for MeshSelection {
    fn from_iter<T: IntoIterator<Item = FaceId>>(iter: T) -> Self {
        MeshSelection {
            faces: HashSet::from_iter(iter),
            ..default()
        }
    }
}

macro_rules! impl_from_for_mesh_selection {
    ($type:ident) => {
        impl From<$type<VertexId>> for MeshSelection {
            fn from(value: $type<VertexId>) -> Self {
                Self::from_iter(value)
            }
        }
        impl From<$type<HalfedgeId>> for MeshSelection {
            fn from(value: $type<HalfedgeId>) -> Self {
                Self::from_iter(value)
            }
        }
        impl From<$type<FaceId>> for MeshSelection {
            fn from(value: $type<FaceId>) -> Self {
                Self::from_iter(value)
            }
        }
    };
}

impl_from_for_mesh_selection!(Vec);
impl_from_for_mesh_selection!(HashSet);
