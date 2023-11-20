use crate::error::SMeshResult;
use crate::mesh_query::EvalMeshQuery;
use crate::smesh::*;

///
/// Higher-level Topological Operations
///
impl SMesh {
    /// Subdivide the edge \p e = (v0,v1) by splitting it into the two edge
    /// (v0,p) and (p,v1). Note that this function does not introduce any
    /// other edge or faces. It simply splits the edge. Returns halfedge that
    /// points to p.
    ///
    /// before:
    ///
    /// v0      h0       v2
    ///  o--------------->o
    ///   <---------------
    ///         o0
    ///
    /// after:
    ///
    /// v0  h0   v   h1   v2
    ///  o------>o------->o
    ///   <------ <-------
    ///     o0       o1
    pub fn insert_vertex(&mut self, h0: HalfedgeId, v: VertexId) -> SMeshResult<HalfedgeId> {
        let h2 = self.q(h0).next().id().ok();
        let o0 = self.q(h0).opposite().id()?;
        let o2 = self.q(o0).prev().id().ok();
        let v2 = self.q(h0).dst_vert().id()?;
        let fh = self.q(h0).face().id().ok();
        let fo = self.q(o0).face().id().ok();

        let (h1, o1) = self.add_edge(v, v2);

        // adjust halfedge connectivity
        let h1_mut = self.he_mut(h1);
        h1_mut.next = h2;
        h1_mut.vertex = v2;
        h1_mut.face = fh;
        let h0_mut = self.he_mut(h0);
        h0_mut.next = Some(h1);
        h0_mut.vertex = v;

        let o1_mut = self.he_mut(o1);
        o1_mut.next = Some(o0);
        o1_mut.vertex = v;
        o1_mut.face = fo;
        if let Some(o2) = o2 {
            self.he_mut(o2).next = Some(o1);
        }
        // adjust vertex connectivity
        self.vert_mut(v2).halfedge = Some(o1);
        self.adjust_outgoing_halfedge(v2)?;
        self.vert_mut(v).halfedge = Some(h1);
        self.adjust_outgoing_halfedge(v)?;

        // adjust face connectivity
        if let Some(fh) = fh {
            self.face_mut(fh).halfedge = Some(h0);
        }
        if let Some(fo) = fo {
            self.face_mut(fo).halfedge = Some(o1);
        }
        Ok(o1)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use glam::vec3;

    #[test]
    fn insert_vertex() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));
        let face = mesh.add_face(vec![v0, v1, v2, v3])?;
        assert_eq!(mesh.q(face).valence(), 4);

        let he = mesh.q(v0).halfedge_to(v1).id()?;
        let v = mesh.add_vertex(vec3(0.0, -1.0, 0.0));
        let h_res = mesh.insert_vertex(he, v)?;
        assert_eq!(mesh.q(face).valence(), 5);
        assert_eq!(mesh.q(h_res).dst_vert().id()?, v);
        assert_eq!(mesh.q(h_res).src_vert().id()?, v1);
        assert_eq!(mesh.q(h_res).next().dst_vert().id()?, v0);

        Ok(())
    }
}
