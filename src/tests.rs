#[cfg(test)]
mod smesh_tests {
    use crate::prelude::*;
    use crate::test_utils::{edge_onering, vertex_onering};
    use glam::vec3;
    use slotmap::KeyData;

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

    #[test]
    fn insert_remove_single_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let (_, _, _, _, f0) = add_quad(mesh);
        assert_eq!(mesh.vertices().len(), 4);
        assert_eq!(mesh.halfedges().len(), 8);
        assert_eq!(mesh.faces().len(), 1);
        mesh.delete_face(f0)?;
        assert_eq!(mesh.vertices().len(), 0);
        assert_eq!(mesh.halfedges().len(), 0);
        assert_eq!(mesh.faces().len(), 0);
        Ok(())
    }

    #[test]
    fn insert_remove_single_polygonal_face() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let v4 = mesh.add_vertex(vec3(0.5, 1.5, 0.0));
        let f0 = mesh.add_face(vec![v0, v1, v2, v3, v4]).unwrap();
        assert_eq!(mesh.vertices().len(), 5);
        assert_eq!(mesh.halfedges().len(), 10);
        assert_eq!(mesh.faces().len(), 1);
        mesh.delete_face(f0)?;
        assert_eq!(mesh.vertices().len(), 0);
        assert_eq!(mesh.halfedges().len(), 0);
        assert_eq!(mesh.faces().len(), 0);
        Ok(())
    }

    #[test]
    fn delete_center_vertex() -> SMeshResult<()> {
        let mut mesh = vertex_onering()?;
        assert_eq!(mesh.vertices().len(), 7);
        assert_eq!(mesh.faces().len(), 6);
        let v0 = mesh.vertices().keys().next().unwrap();
        // center vertex
        let v = v0.halfedge().cw_rotated_neighbour().dst_vert().run(&mesh)?;
        mesh.delete_vertex(v)?;
        assert_eq!(mesh.vertices().len(), 0);
        assert_eq!(mesh.halfedges().len(), 0);
        assert_eq!(mesh.faces().len(), 0);
        Ok(())
    }

    #[test]
    fn delete_side_vertex() -> SMeshResult<()> {
        let mut mesh = vertex_onering()?;
        assert_eq!(mesh.vertices().len(), 7);
        assert_eq!(mesh.faces().len(), 6);
        let v0 = mesh.vertices().keys().next().unwrap();
        mesh.delete_vertex(v0)?;
        assert_eq!(mesh.vertices().len(), 6);
        assert_eq!(mesh.faces().len(), 4);
        Ok(())
    }

    #[test]
    fn delete_center_edge() -> SMeshResult<()> {
        let mut mesh = edge_onering()?;
        assert_eq!(mesh.vertices().len(), 10);
        assert_eq!(mesh.faces().len(), 10);
        // the two vertices of the center edge
        let va = VertexId::from(KeyData::from_ffi(5));
        let vb = VertexId::from(KeyData::from_ffi(6));
        let e = va.halfedge_to(vb).run(&mesh)?;
        mesh.delete_edge(e)?;
        assert_eq!(mesh.vertices().len(), 10);
        assert_eq!(mesh.faces().len(), 8);
        Ok(())
    }

    #[test]
    fn clone() {
        let mut mesh = SMesh::new();
        add_triangle(&mut mesh);
        let mesh_2 = mesh.clone();
        assert_eq!(mesh_2.vertices().len(), 3);
        assert_eq!(mesh_2.halfedges().len(), 6);
        assert_eq!(mesh_2.faces().len(), 1);
    }

    // TODO: port tests for properties

    #[test]
    fn is_triangle_mesh() {
        let mut mesh = SMesh::new();
        add_triangle(&mut mesh);
        assert!(mesh.is_triangle_mesh());
        assert!(!mesh.is_quad_mesh());
    }

    #[test]
    fn is_quad_mesh() {
        let mut mesh = SMesh::new();
        add_quad(&mut mesh);
        assert!(!mesh.is_triangle_mesh());
        assert!(mesh.is_quad_mesh());
    }
    #[test]
    fn is_not_tri_or_quad_mesh() {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let v4 = mesh.add_vertex(vec3(0.5, 1.5, 0.0));
        mesh.add_face(vec![v0, v1, v2, v3, v4]).unwrap();
        assert!(!mesh.is_triangle_mesh());
        assert!(!mesh.is_quad_mesh());
    }

    #[test]
    fn vertex_valence() {
        let mesh = &mut SMesh::new();
        add_triangle(mesh);
        let v0 = mesh.vertices().keys().next().unwrap();
        assert_eq!(v0.valence(mesh), 2);
    }

    #[test]
    fn face_valence() {
        let mesh = &mut SMesh::new();
        add_triangle(mesh);
        let f0 = mesh.faces().keys().next().unwrap();
        assert_eq!(f0.valence(mesh), 3);
    }

    #[test]
    fn collapse() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let (_, _, v2, v3, _, _) = add_triangles(mesh);
        assert_eq!(mesh.faces().len(), 2);
        let h0 = v3.halfedge_to(v2).run(mesh)?;
        assert!(mesh.is_collapse_ok(h0).is_ok());
        mesh.collapse(h0)?;
        assert_eq!(mesh.faces().len(), 1);
        Ok(())
    }
    #[test]
    fn edge_removal_ok() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let (_, v1, v2, _, _, _) = add_triangles(mesh);
        let h = v1.halfedge_to(v2).run(mesh)?;
        assert!(mesh.is_removal_ok(h).is_ok());
        Ok(())
    }

    #[test]
    fn edge_removal_not_ok() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let (_, v1, v2, _) = add_triangle(mesh);
        let h = v1.halfedge_to(v2).run(mesh)?;
        assert!(mesh.is_removal_ok(h).is_err());
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
