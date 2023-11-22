use crate::impl_id_extensions_for;
use crate::smesh::query::*;
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
        let next = self.conn.q(current).ccw_rotated_neighbour().id().unwrap();
        self.current = if next == self.start { None } else { Some(next) };
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
        let dst_vert = self.conn.q(current).dst_vert().id();
        let next = self.conn.q(current).ccw_rotated_neighbour().id().unwrap();
        self.current = if next == self.start { None } else { Some(next) };
        dst_vert.ok()
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
        let dst_vert = self.conn.q(current).dst_vert().id();
        let next = self.conn.q(current).next().id().unwrap();
        self.current = if next == self.start { None } else { Some(next) };
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
        let next = self.conn.q(current).next().id().unwrap();
        self.current = if next == self.start { None } else { Some(next) };
        Some(current)
    }
}

impl MeshQuery<'_, VertexId> {
    pub fn vertices(&self) -> VertexAroundVertexIter {
        let start = self.halfedge().id().unwrap();
        VertexAroundVertexIter {
            conn: self.conn,
            start,
            current: Some(start),
        }
    }

    pub fn halfedges(&self) -> HalfedgeAroundVertexIter {
        let start = self.halfedge().id().unwrap();
        HalfedgeAroundVertexIter {
            conn: self.conn,
            start,
            current: Some(start),
        }
    }
}

impl MeshQuery<'_, FaceId> {
    pub fn vertices(&self) -> VertexAroundFaceIter {
        let start = self.halfedge().id().unwrap();
        VertexAroundFaceIter {
            conn: self.conn,
            start,
            current: Some(start),
        }
    }

    pub fn halfedges(&self) -> VertexAroundFaceIter {
        let start = self.halfedge().id().unwrap();
        VertexAroundFaceIter {
            conn: self.conn,
            start,
            current: Some(start),
        }
    }
}

impl_id_extensions_for!(
    VertexId,
    pub trait VertexIterators {
        fn vertices(self, mesh: &SMesh) -> VertexAroundVertexIter;
        fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundVertexIter;
    }
);
impl MeshQueryBuilder<VertexId> {
    pub fn vertices(self, mesh: &SMesh) -> VertexAroundVertexIter {
        let start = self.halfedge().run(mesh).unwrap();
        VertexAroundVertexIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }

    pub fn halfedges(self, mesh: &SMesh) -> HalfedgeAroundVertexIter {
        let start = self.halfedge().run(mesh).unwrap();
        HalfedgeAroundVertexIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }
}

impl_id_extensions_for!(
    FaceId,
    pub trait FaceIterators {
        fn vertices(self, mesh: &SMesh) -> VertexAroundFaceIter;
        fn halfedges(self, mesh: &SMesh) -> VertexAroundFaceIter;
    }
);
impl MeshQueryBuilder<FaceId> {
    pub fn vertices(self, mesh: &SMesh) -> VertexAroundFaceIter {
        let start = self.halfedge().run(mesh).unwrap();
        VertexAroundFaceIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
    }

    pub fn halfedges(self, mesh: &SMesh) -> VertexAroundFaceIter {
        let start = self.halfedge().run(mesh).unwrap();
        VertexAroundFaceIter {
            conn: &mesh.connectivity,
            start,
            current: Some(start),
        }
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
        for v_id in mesh.q(v0).vertices() {
            println!("{:?}", v_id);
            ids.push(v_id);
        }
        assert_eq!(ids, vec![v3, v4, v1]);

        let mut ids = vec![];
        for v_id in v0.q().vertices(mesh) {
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

        let mut ids = mesh.q(f0).vertices().collect_vec();
        assert_eq!(ids, vec![v0, v1, v2, v3]);
        ids = mesh.q(f1).vertices().collect_vec();
        assert_eq!(ids, vec![v0, v4, v1,]);
    }
}
