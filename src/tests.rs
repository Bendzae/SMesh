#[cfg(test)]
mod smesh_tests {
    use crate::prelude::*;
    use glam::vec3;

    #[test]
    fn empty_mesh() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        assert_eq!(mesh.vertices().len(), 0);
        assert_eq!(mesh.halfedges().len(), 0);
        assert_eq!(mesh.faces().len(), 0);
        Ok(())
    }

    #[test]
    fn insert_remove_single_vertex() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        assert_eq!(mesh.vertices().len(), 1);
        mesh.delete_vertex(v)?;
        assert_eq!(mesh.vertices().len(), 0);
        Ok(())
    }

    #[test]
    fn insert_remove_single_triangle() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let (_, _, _, f0) = add_triangle(mesh);
        assert_eq!(mesh.vertices().len(), 3);
        assert_eq!(mesh.halfedges().len(), 6);
        assert_eq!(mesh.faces().len(), 1);
        mesh.delete_face(f0)?;
        assert_eq!(mesh.vertices().len(), 0);
        assert_eq!(mesh.halfedges().len(), 0);
        assert_eq!(mesh.faces().len(), 0);
        Ok(())
    }

    /// Utils
    fn add_triangle(mesh: &mut SMesh) -> (VertexId, VertexId, VertexId, FaceId) {
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let f0 = mesh.add_face(vec![v0, v1, v2]).unwrap();
        (v0, v1, v2, f0)
    }

    fn add_triangles(mesh: &mut SMesh) -> (VertexId, VertexId, VertexId, VertexId, FaceId, FaceId) {
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let f0 = mesh.add_face(vec![v0, v1, v2]).unwrap();
        let f2 = mesh.add_face(vec![v1, v3, v2]).unwrap();
        (v0, v1, v2, v3, f0, f2)
    }

    fn add_quad(mesh: &mut SMesh) -> (VertexId, VertexId, VertexId, VertexId, FaceId) {
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let f0 = mesh.add_face(vec![v0, v1, v2, v3]).unwrap();
        (v0, v1, v2, v3, f0)
    }
}
