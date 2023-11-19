use crate::error::SMeshError;
use crate::mesh_query::{EvalMeshQuery, MeshQuery};
use crate::smesh::{Connectivity, HalfedgeId, VertexId};

pub struct VertexAroundVertexIter<'a> {
    conn: &'a Connectivity,
    start: HalfedgeId,
    current: Option<HalfedgeId>,
}

impl<'a> VertexAroundVertexIter<'a> {
    pub fn new(connectivity: &'a Connectivity, vertex_id: VertexId) -> VertexAroundVertexIter<'a> {
        let start = connectivity.q(vertex_id).halfedge().id().unwrap();
        VertexAroundVertexIter {
            conn: connectivity,
            start,
            current: Some(start),
        }
    }
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

mod test {
    use super::*;
    use crate::smesh::SMesh;
    use glam::vec3;

    #[test]
    fn vertex_around_vertex() {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v4 = mesh.add_vertex(vec3(0.0, -2.0, 0.0));

        let _ = mesh.add_face(vec![v0, v1, v2, v3]);
        let _ = mesh.add_face(vec![v0, v4, v1]);

        let iter = VertexAroundVertexIter::new(&mesh.connectivity, v0);
        let mut ids = vec![];
        for v_id in iter {
            println!("{:?}", v_id);
            ids.push(v_id);
        }
        assert_eq!(ids, vec![v3, v4, v1]);
    }
}
