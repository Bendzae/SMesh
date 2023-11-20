use crate::prelude::SMeshError::VertexNotFound;
use crate::q;
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

    /// \return whether the mesh a triangle mesh. this function simply tests
    /// each face, and therefore is not very efficient.
    pub fn is_triangle_mesh(&self) -> bool {
        for (id, _) in &self.connectivity.faces {
            if self.q(id).valence() != 3 {
                return false;
            }
        }
        true
    }

    /// \return whether the mesh a quad mesh. this function simply tests
    /// each face, and therefore is not very efficient.
    pub fn is_quad_mesh(&self) -> bool {
        for (id, _) in &self.connectivity.faces {
            if self.q(id).valence() != 4 {
                return false;
            }
        }
        true
    }

    /// \return whether collapsing the halfedge \p v0v1 is topologically legal.
    /// \attention This function is only valid for triangle meshes.
    pub fn is_collapse_ok(&self, v0v1: HalfedgeId) -> SMeshResult<()> {
        let v1v0 = self.q(v0v1).opposite().id()?;
        let v0 = self.q(v1v0).dst_vert().id()?;
        let v1 = self.q(v0v1).dst_vert().id()?;

        // the edges v1-vl and vl-v0 must not be both boundary edges
        let vl = if !self.q(v0v1).is_boundary() {
            let h1 = self.q(v0v1).next();
            let h2 = h1.next();
            if h1.opposite().is_boundary() && h2.opposite().is_boundary() {
                bail!(DefaultError);
            }
            h1.dst_vert().id().ok()
        } else {
            None
        };

        // the edges v0-vr and vr-v1 must not be both boundary edges
        let vr = if !self.q(v1v0).is_boundary() {
            let h1 = self.q(v1v0).next();
            let h2 = h1.next();
            if h1.opposite().is_boundary() && h2.opposite().is_boundary() {
                bail!(DefaultError);
            }
            h1.dst_vert().id().ok()
        } else {
            None
        };

        if vl.is_none() && vr.is_none() {
            bail!(DefaultError);
        }

        // edge between two boundary vertices should be a boundary edge
        if self.q(v0).is_boundary()
            && self.q(v1).is_boundary()
            && !self.q(v0v1).is_boundary()
            && !self.q(v1v0).is_boundary()
        {
            bail!(DefaultError);
        }

        // test intersection of the one-rings of v0 and v1
        for vv in self.q(v0).vertices() {
            if vv != v1
                && vv != vl.unwrap()
                && vv != vr.unwrap()
                && self.q(vv).halfedge_to(v1).id().is_ok()
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
