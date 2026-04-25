//! Walkers over neighbouring elements.
//!
//! Each iterator visits the one-ring of a vertex or the boundary of a face
//! exactly once. They are produced by extension methods on
//! [`VertexId`] / [`FaceId`] or on the corresponding query builders — see
//! [`VertexIterators`] and [`FaceIterators`].
//!
//! ```
//! # use glam::vec3;
//! # use smesh::prelude::*;
//! # let mut mesh = SMesh::new();
//! # let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
//! # let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
//! # let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
//! # let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
//! # let f = mesh.make_quad(v0, v1, v2, v3).unwrap();
//! // Visit every vertex of a face in CCW order
//! let verts: Vec<VertexId> = f.vertices(&mesh).collect();
//! assert_eq!(verts.len(), 4);
//!
//! // Visit vertices around v0 (its neighbours)
//! let neighbours: Vec<VertexId> = v0.vertices(&mesh).collect();
//! ```
//!
//! All iterators yield elements a single time and terminate when the walk
//! returns to the starting halfedge.

use crate::prelude::model::connectivity::*;
use crate::prelude::model::mesh::*;
use crate::prelude::model::mesh_elements::*;
use crate::smesh::mesh_query::*;
use crate::smesh::*;

/// Iterator over the outgoing halfedges of a vertex, in CCW order.
#[derive(Debug, Clone)]
pub struct HalfedgeAroundVertexIter<'a> {
    conn: &'a Connectivity,
    start: HalfedgeId,
    current: Option<HalfedgeId>,
}
impl<'a> Iterator for HalfedgeAroundVertexIter<'a> {
    type Item = HalfedgeId;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current) = self.current else {
            return None;
        };
        let next = current.ccw_rotated_neighbour().run(self.conn).ok();
        self.current = if next == Some(self.start) { None } else { next };
        Some(current)
    }
}

/// Iterator over the neighbouring vertices of a vertex (the one-ring), in CCW order.
#[derive(Debug, Clone)]
pub struct VertexAroundVertexIter<'a> {
    conn: &'a Connectivity,
    start: HalfedgeId,
    current: Option<HalfedgeId>,
}
impl<'a> Iterator for VertexAroundVertexIter<'a> {
    type Item = VertexId;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current) = self.current else {
            return None;
        };
        let dst_vert = current.dst_vert().run(self.conn);
        let next = current.ccw_rotated_neighbour().run(self.conn).ok();
        self.current = if next == Some(self.start) { None } else { next };
        dst_vert.ok()
    }
}

/// Iterator over the faces incident to a vertex, in CCW order.
///
/// Skips boundary halfedges, so a vertex on a mesh boundary yields one fewer
/// face than its halfedge valence.
#[derive(Debug, Clone)]
pub struct FaceAroundVertexIter<'a> {
    conn: &'a Connectivity,
    start: HalfedgeId,
    current: Option<HalfedgeId>,
}
impl<'a> Iterator for FaceAroundVertexIter<'a> {
    type Item = FaceId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(current) = self.current else {
                return None;
            };

            let face = current.face().run(self.conn);
            let next = current.ccw_rotated_neighbour().run(self.conn).ok();
            self.current = if next == Some(self.start) { None } else { next };
            if let Ok(face) = face {
                return Some(face);
            }
        }
    }
}

/// Iterator over the vertices of a face, in the face's winding order.
#[derive(Debug, Clone)]
pub struct VertexAroundFaceIter<'a> {
    conn: &'a Connectivity,
    start: HalfedgeId,
    current: Option<HalfedgeId>,
}

impl<'a> Iterator for VertexAroundFaceIter<'a> {
    type Item = VertexId;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current) = self.current else {
            return None;
        };
        let dst_vert = current.dst_vert().run(self.conn);
        let next = current.next().run(self.conn).ok();
        self.current = if next == Some(self.start) { None } else { next };
        dst_vert.ok()
    }
}

/// Iterator over the halfedges bounding a face, in the face's winding order.
#[derive(Debug, Clone)]
pub struct HalfedgeAroundFaceIter<'a> {
    conn: &'a Connectivity,
    start: HalfedgeId,
    current: Option<HalfedgeId>,
}

impl<'a> Iterator for HalfedgeAroundFaceIter<'a> {
    type Item = HalfedgeId;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current) = self.current else {
            return None;
        };
        let next = current.next().run(self.conn).ok();
        self.current = if next == Some(self.start) { None } else { next };
        Some(current)
    }
}

/// Walkers over the one-ring of a vertex.
///
/// Implemented for both [`VertexId`] and
/// [`MeshQueryBuilder<VertexId>`](MeshQueryBuilder).
pub trait VertexIterators {
    /// Neighbouring vertices in CCW order.
    fn vertices(self, mesh: &SMesh) -> VertexAroundVertexIter<'_>;
    /// Outgoing halfedges in CCW order.
    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundVertexIter<'_>;
    /// Incident faces in CCW order (boundary gaps are skipped).
    fn faces(self, mesh: &SMesh) -> FaceAroundVertexIter<'_>;
}

/// Walkers over the boundary of a face.
///
/// Implemented for both [`FaceId`] and
/// [`MeshQueryBuilder<FaceId>`](MeshQueryBuilder).
pub trait FaceIterators {
    /// Vertices bounding this face in winding order.
    fn vertices(self, mesh: &SMesh) -> VertexAroundFaceIter<'_>;
    /// Halfedges bounding this face in winding order.
    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundFaceIter<'_>;
}

impl VertexIterators for MeshQueryBuilder<VertexId> {
    fn vertices(self, mesh: &SMesh) -> VertexAroundVertexIter<'_> {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        VertexAroundVertexIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }

    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundVertexIter<'_> {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        HalfedgeAroundVertexIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }

    fn faces(self, mesh: &SMesh) -> FaceAroundVertexIter<'_> {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        FaceAroundVertexIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }
}

impl VertexIterators for VertexId {
    fn vertices(self, mesh: &SMesh) -> VertexAroundVertexIter<'_> {
        self.q().vertices(mesh)
    }

    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundVertexIter<'_> {
        self.q().halfedges(mesh)
    }

    fn faces(self, mesh: &SMesh) -> FaceAroundVertexIter<'_> {
        self.q().faces(mesh)
    }
}

impl FaceIterators for MeshQueryBuilder<FaceId> {
    fn vertices(self, mesh: &SMesh) -> VertexAroundFaceIter<'_> {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        VertexAroundFaceIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }

    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundFaceIter<'_> {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        HalfedgeAroundFaceIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }
}

impl FaceIterators for FaceId {
    fn vertices(self, mesh: &SMesh) -> VertexAroundFaceIter<'_> {
        self.q().vertices(mesh)
    }

    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundFaceIter<'_> {
        self.q().halfedges(mesh)
    }
}

mod test {
    use glam::vec3;
    use itertools::Itertools;
    use crate::prelude::*;
    
    
    

    #[test]
    fn vertex_around_vertex() {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));
        let v4 = mesh.add_vertex(vec3(0.0, -2.0, 0.0));

        let _ = mesh.make_face(vec![v0, v1, v2, v3]);
        let _ = mesh.make_face(vec![v0, v4, v1]);

        let mut ids = vec![];
        for v_id in v0.vertices(mesh) {
            println!("{:?}", v_id);
            ids.push(v_id);
        }
        assert_eq!(ids, vec![v3, v4, v1]);

        let mut ids = vec![];
        for v_id in v0.vertices(mesh) {
            println!("{:?}", v_id);
            ids.push(v_id);
        }
        assert_eq!(ids, vec![v3, v4, v1]);
    }

    #[test]
    fn vertex_around_face() {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));
        let v4 = mesh.add_vertex(vec3(0.0, -2.0, 0.0));

        let f0 = mesh.make_face(vec![v0, v1, v2, v3]).unwrap();
        let f1 = mesh.make_face(vec![v0, v4, v1]).unwrap();

        let mut ids = f0.vertices(mesh).collect_vec();
        assert_eq!(ids, vec![v0, v1, v2, v3]);
        ids = f1.vertices(mesh).collect_vec();
        assert_eq!(ids, vec![v0, v4, v1,]);
    }
}
