use crate::smesh::iterators::*;
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
        let h2 = h0.next().run(self).ok();
        let o0 = h0.opposite().run(self)?;
        let o2 = o0.prev().run(self).ok();
        let v2 = h0.dst_vert().run(self)?;
        let fh = h0.face().run(self).ok();
        let fo = o0.face().run(self).ok();

        let (h1, o1) = self.add_edge(v, v2);

        // adjust halfedge connectivity
        let mut h1_mut = self.he_mutator(h1);
        h1_mut.set_next(h2);
        h1_mut.set_vertex(v2);
        h1_mut.set_face(fh);
        let mut h0_mut = self.he_mutator(h0);
        h0_mut.set_next(Some(h1));
        h0_mut.set_vertex(v);

        let mut o1_mut = self.he_mutator(o1);
        o1_mut.set_next(Some(o0));
        o1_mut.set_vertex(v);
        o1_mut.set_face(fo);
        if let Some(o2) = o2 {
            self.he_mutator(o2).set_next(Some(o1));
        }
        // adjust vertex connectivity
        let mut v2_mut = self.vert_mutator(v2);
        v2_mut.set_halfedge(Some(o1));
        v2_mut.adjust_outgoing_halfedge()?;
        let mut v_mut = self.vert_mutator(v);
        v_mut.set_halfedge(Some(h1));
        v_mut.adjust_outgoing_halfedge()?;

        // adjust face connectivity
        if let Some(fh) = fh {
            self.face_mut(fh).halfedge = Some(h0);
        }
        if let Some(fo) = fo {
            self.face_mut(fo).halfedge = Some(o1);
        }
        Ok(o1)
    }

    /// \return whether the mesh a triangle mesh. this function simply tests
    /// each face, and therefore is not very efficient.
    pub fn is_triangle_mesh(&self) -> bool {
        for (id, _) in &self.connectivity.faces {
            if id.valence(self) != 3 {
                return false;
            }
        }
        true
    }

    /// \return whether the mesh a quad mesh. this function simply tests
    /// each face, and therefore is not very efficient.
    pub fn is_quad_mesh(&self) -> bool {
        for (id, _) in &self.connectivity.faces {
            if id.valence(self) != 4 {
                return false;
            }
        }
        true
    }

    /// \return whether collapsing the halfedge \p v0v1 is topologically legal.
    /// \attention This function is only valid for triangle meshes.
    pub fn is_collapse_ok(&self, v0v1: HalfedgeId) -> SMeshResult<()> {
        let v1v0 = v0v1.opposite().run(self)?;
        let v0 = v1v0.dst_vert().run(self)?;
        let v1 = v0v1.dst_vert().run(self)?;

        // the edges v1-vl and vl-v0 must not be both boundary edges
        let vl = if !v0v1.is_boundary(self) {
            let h1 = v0v1.next().run(self)?;
            let h2 = h1.next().run(self)?;
            if h1.opposite().is_boundary(self) && h2.opposite().is_boundary(self) {
                bail!(DefaultError);
            }
            h1.dst_vert().run(self).ok()
        } else {
            None
        };

        // the edges v0-vr and vr-v1 must not be both boundary edges
        let vr = if !v1v0.is_boundary(self) {
            let h1 = v1v0.next().run(self)?;
            let h2 = h1.next().run(self)?;
            if h1.opposite().is_boundary(self) && h2.opposite().is_boundary(self) {
                bail!(DefaultError);
            }
            h1.dst_vert().run(self).ok()
        } else {
            None
        };

        if vl.is_none() && vr.is_none() {
            bail!(DefaultError);
        }

        // edge between two boundary vertices should be a boundary edge
        if v0.is_boundary(self)
            && v1.is_boundary(self)
            && !v0v1.is_boundary(self)
            && !v1v0.is_boundary(self)
        {
            bail!(DefaultError);
        }

        // test intersection of the one-rings of v0 and v1
        for vv in v0.vertices(self) {
            if vv != v1
                && vv != vl.unwrap()
                && vv != vr.unwrap()
                && vv.halfedge_to(v1).run(self).is_ok()
            {
                bail!(DefaultError);
            }
        }

        Ok(())
    }

    /// Collapse the halfedge \p h by moving its start vertex into its target
    /// vertex. For non-boundary halfedges this function removes one vertex, three
    /// edges, and two faces. For boundary halfedges it removes one vertex, two
    /// edges and one face.
    /// \attention This function is only valid for triangle meshes.
    /// \attention Halfedge collapses might lead to invalid faces. Call
    /// is_collapse_ok(Halfedge) to be sure the collapse is legal.
    fn collapse(he: HalfedgeId) -> SMeshResult<VertexId> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;
    use glam::vec3;

    #[test]
    fn insert_vertex() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));
        let face = mesh.add_face(vec![v0, v1, v2, v3])?;
        assert_eq!(face.valence(mesh), 4);

        let he = v0.halfedge_to(v1).run(mesh)?;
        let v = mesh.add_vertex(vec3(0.0, -1.0, 0.0));
        let h_res = mesh.insert_vertex(he, v)?;
        assert_eq!(v1.halfedge_to(v).run(mesh)?, h_res);
        assert_eq!(face.valence(mesh), 5);
        assert_eq!(h_res.dst_vert().run(mesh)?, v);
        assert_eq!(h_res.src_vert().run(mesh)?, v1);
        assert_eq!(h_res.next().dst_vert().run(mesh)?, v0);

        let v0_v = v0.halfedge_to(v).run(mesh)?;
        let v_v1 = v.halfedge_to(v1).run(mesh)?;
        assert_eq!(v0_v.next().run(mesh)?, v_v1);
        assert_eq!(v_v1.prev().run(mesh)?, v0_v);

        let he_next = h_res.next().next();
        assert_eq!(he_next.prev().prev().run(mesh)?, h_res);
        Ok(())
    }

    // #[test]
    // fn collapse() -> SMeshResult<()> {
    //     bail!(DefaultError)
    // }
}
