use std::collections::HashSet;

use crate::{bail, prelude::*};
use bevy::utils::default;
use itertools::Itertools;

use super::mesh_query::{HalfedgeOps, RunQuery};

#[derive(Debug, Clone, Default)]
pub struct MeshSelection {
    vertices: HashSet<VertexId>,
    halfedges: HashSet<HalfedgeId>,
    faces: HashSet<FaceId>,
}

impl MeshSelection {
    pub fn new() -> Self {
        MeshSelection::default()
    }

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

pub trait MeshSelectionOps<T> {
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
