use crate::smesh::mesh_query::*;
use crate::smesh::*;

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

pub trait VertexIterators {
    fn vertices(self, mesh: &SMesh) -> VertexAroundVertexIter;
    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundVertexIter;
    fn faces(self, mesh: &SMesh) -> FaceAroundVertexIter;
}

pub trait FaceIterators {
    fn vertices(self, mesh: &SMesh) -> VertexAroundFaceIter;
    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundFaceIter;
}

impl VertexIterators for MeshQueryBuilder<VertexId> {
    fn vertices(self, mesh: &SMesh) -> VertexAroundVertexIter {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        VertexAroundVertexIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }

    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundVertexIter {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        HalfedgeAroundVertexIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }

    fn faces(self, mesh: &SMesh) -> FaceAroundVertexIter {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        FaceAroundVertexIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }
}

impl VertexIterators for VertexId {
    fn vertices(self, mesh: &SMesh) -> VertexAroundVertexIter {
        self.q().vertices(mesh)
    }

    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundVertexIter {
        self.q().halfedges(mesh)
    }

    fn faces(self, mesh: &SMesh) -> FaceAroundVertexIter {
        self.q().faces(mesh)
    }
}

impl FaceIterators for MeshQueryBuilder<FaceId> {
    fn vertices(self, mesh: &SMesh) -> VertexAroundFaceIter {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        VertexAroundFaceIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }

    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundFaceIter {
        let start = self.halfedge().run(mesh).unwrap_or(HalfedgeId::default());
        HalfedgeAroundFaceIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }
}

impl FaceIterators for FaceId {
    fn vertices(self, mesh: &SMesh) -> VertexAroundFaceIter {
        self.q().vertices(mesh)
    }

    fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundFaceIter {
        self.q().halfedges(mesh)
    }
}

mod test {
    use super::*;
    use glam::vec3;
    use itertools::Itertools;

    #[test]
    fn vertex_around_vertex() {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));
        let v4 = mesh.add_vertex(vec3(0.0, -2.0, 0.0));

        let _ = mesh.add_face(vec![v0, v1, v2, v3]);
        let _ = mesh.add_face(vec![v0, v4, v1]);

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

        let f0 = mesh.add_face(vec![v0, v1, v2, v3]).unwrap();
        let f1 = mesh.add_face(vec![v0, v4, v1]).unwrap();

        let mut ids = f0.vertices(mesh).collect_vec();
        assert_eq!(ids, vec![v0, v1, v2, v3]);
        ids = f1.vertices(mesh).collect_vec();
        assert_eq!(ids, vec![v0, v4, v1,]);
    }
}
