use std::collections::HashSet;

use bevy::log::info;
use itertools::Itertools;

use crate::{bail, prelude::*};

///
/// Higher-level Topological Operations
///
impl SMesh {
    /// whether the mesh a triangle mesh. this function simply tests
    /// each face, and therefore is not very efficient.
    pub fn is_triangle_mesh(&self) -> bool {
        for (id, _) in &self.connectivity.faces {
            if id.valence(self) != 3 {
                return false;
            }
        }
        true
    }

    /// whether the mesh a quad mesh. this function simply tests
    /// each face, and therefore is not very efficient.
    pub fn is_quad_mesh(&self) -> bool {
        for (id, _) in &self.connectivity.faces {
            if id.valence(self) != 4 {
                return false;
            }
        }
        true
    }

    /// Subdivide the edge  e = (v0,v1) by splitting it into the two edge
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
        let mut h1_mut = self.get_mut(h1);
        h1_mut.set_next(h2)?;
        h1_mut.set_vertex(v2)?;
        h1_mut.set_face(fh)?;
        let mut h0_mut = self.get_mut(h0);
        h0_mut.set_next(Some(h1))?;
        h0_mut.set_vertex(v)?;

        let mut o1_mut = self.get_mut(o1);
        o1_mut.set_next(Some(o0))?;
        o1_mut.set_vertex(v)?;
        o1_mut.set_face(fo)?;
        if let Some(o2) = o2 {
            self.get_mut(o2).set_next(Some(o1))?;
        }
        // adjust vertex connectivity
        let mut v2_mut = self.get_mut(v2);
        v2_mut.set_halfedge(Some(o1))?;
        v2_mut.adjust_outgoing_halfedge()?;
        let mut v_mut = self.get_mut(v);
        v_mut.set_halfedge(Some(h1))?;
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

    pub fn delete_vertex(&mut self, v: VertexId) -> SMeshResult<()> {
        let incident_faces = v.faces(self).collect_vec();
        for f in incident_faces {
            self.delete_face(f)?;
        }
        let _ = self.get_mut(v).delete();
        Ok(())
    }

    pub fn delete_edge(&mut self, h: HalfedgeId) -> SMeshResult<()> {
        if let Ok(f) = h.face().run(self) {
            self.delete_face(f)?;
        }
        if let Ok(f) = h.opposite().face().run(self) {
            self.delete_face(f)?;
        }
        let _ = self.get_mut(h).delete();
        Ok(())
    }

    pub fn delete_face(&mut self, f: FaceId) -> SMeshResult<()> {
        let mut delete_edges = vec![];
        let mut adjust_edges = vec![];
        let mut delete_verts = vec![];
        // collect mesh elements to change
        for hc in f.halfedges(self) {
            adjust_edges.push(hc);
            if hc.opposite().is_boundary(self) {
                delete_edges.push(hc);
            }
            delete_verts.push(hc.dst_vert().run(self)?)
        }
        // remove face id from he's
        for he in adjust_edges {
            self.get_mut(he).set_face(None)?;
        }

        // delete face
        self.get_mut(f).delete()?;

        // delete all collected (half)edges
        // delete isolated vertices
        for h0 in delete_edges {
            let v0 = h0.dst_vert().run(self)?;
            let next0 = h0.next().run(self)?;
            let prev0 = h0.prev().run(self)?;

            let h1 = h0.opposite().run(self)?;
            let v1 = h1.dst_vert().run(self)?;
            let next1 = h1.next().run(self)?;
            let prev1 = h1.prev().run(self)?;

            // adjust next and prev handles
            self.get_mut(prev0).set_next(Some(next1))?;
            self.get_mut(prev1).set_next(Some(next0))?;

            // delete edge
            self.get_mut(h0).delete()?;

            // update v0
            if v0.halfedge().run(self)? == h1 {
                if next0 == h1 {
                    self.get_mut(v0).delete()?;
                } else {
                    self.get_mut(v0).set_halfedge(Some(next0))?;
                }
            }

            // update v1
            if v1.halfedge().run(self)? == h0 {
                if next1 == h0 {
                    self.get_mut(v1).delete()?;
                } else {
                    self.get_mut(v1).set_halfedge(Some(next1))?;
                }
            }
        }

        // update outgoing halfedge handles of remaining vertices
        for v in delete_verts {
            let _ = self.get_mut(v).adjust_outgoing_halfedge();
        }
        Ok(())
    }

    // Delete "only" the face without deleting any of its connected elements
    // Also removes any references to it in its neighbouring halfedges
    pub fn delete_only_face(&mut self, f: FaceId) -> SMeshResult<()> {
        let adjust_edges = f.halfedges(self).collect_vec();
        // remove face id from he's
        for he in adjust_edges {
            self.get_mut(he).set_face(None)?;
        }
        self.get_mut(f).delete()?;
        Ok(())
    }

    // Delete "only" the edge without deleting any of its connected elements
    pub fn delete_only_edge(&mut self, e: HalfedgeId) -> SMeshResult<()> {
        if !self.halfedges().contains(&e) {
            // Already deleted
            return Ok(());
        }
        let mut vert_needs_adjust = HashSet::new();
        let mut he_needs_adjust = HashSet::new();
        if let Ok(face) = e.face().run(self) {
            self.delete_only_face(face).ok();
        }
        vert_needs_adjust.insert(e.src_vert().run(self)?);
        if let Ok(prev) = e.prev().run(self) {
            he_needs_adjust.insert(prev);
        }
        if let Ok(opposite) = e.opposite().run(self) {
            if let Ok(face) = opposite.face().run(self) {
                self.delete_only_face(face)?;
            }
            if let Ok(prev) = opposite.prev().run(self) {
                he_needs_adjust.insert(prev);
            }
            vert_needs_adjust.insert(opposite.src_vert().run(self)?);
        }

        self.get_mut(e).delete().ok();
        for v_id in vert_needs_adjust {
            self.get_mut(v_id).adjust_outgoing_halfedge()?;
        }
        for h_id in he_needs_adjust {
            let new_next = h_id.dst_vert().halfedge().run(self).ok();
            self.get_mut(h_id).set_next(new_next).ok();
        }
        Ok(())
    }

    /// whether collapsing the halfedge  v0v1 is topologically legal.
    /// This function is only valid for triangle meshes.
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

    /// Collapse the halfedge h by moving its start vertex into its target
    /// vertex. For non-boundary halfedges this function removes one vertex, three
    /// edges, and two faces. For boundary halfedges it removes one vertex, two
    /// edges and one face.
    /// This function is only valid for triangle meshes.
    /// Halfedge collapses might lead to invalid faces. Call
    /// is_collapse_ok(Halfedge) to be sure the collapse is legal.
    pub fn collapse(&mut self, h: HalfedgeId) -> SMeshResult<()> {
        let h0 = h;
        let h1 = h0.prev().run(self)?;
        let o0 = h0.opposite();
        let o1 = o0.next().run(self)?;
        // remove edge
        self.remove_edge_helper(h)?;

        // remove loops
        if h1.next().next().run(self) == Ok(h1) {
            self.remove_loop_helper(h1)?;
        }

        if o1.next().next().run(self) == Ok(o1) {
            self.remove_loop_helper(o1)?;
        }
        Ok(())
    }

    pub fn is_removal_ok(&self, h0: HalfedgeId) -> SMeshResult<()> {
        let h1 = h0.opposite().run(self)?;
        let v0 = h0.dst_vert().run(self)?;
        let v1 = h1.dst_vert().run(self)?;

        // boundary?
        let f0 = h0.face().run(self)?;
        let f1 = h1.face().run(self)?;

        // same face?
        if f0 == f1 {
            bail!(TopologyError);
        }

        // are the two faces connect through another vertex?
        for v in f0.vertices(self) {
            if v != v0 && v != v1 {
                for f in v.faces(self) {
                    if f == f1 {
                        bail!(TopologyError);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn remove_edge(&mut self, h0: HalfedgeId) -> SMeshResult<()> {
        self.is_removal_ok(h0)?;

        let h1 = h0.opposite().run(self)?;

        let v0 = h0.dst_vert().run(self)?;
        let v1 = h1.dst_vert().run(self)?;

        let f0 = h0.face().run(self);
        let f1 = h1.face().run(self);

        let h0_prev = h0.prev().run(self);
        let h0_next = h0.next().run(self);
        let h1_prev = h1.prev().run(self);
        let h1_next = h1.next().run(self);

        // adjust vertex->halfedge
        if v0.halfedge().run(self) == Ok(h1) {
            self.get_mut(v0).set_halfedge(h0_next.ok())?;
        }
        if v1.halfedge().run(self) == Ok(h0) {
            self.get_mut(v1).set_halfedge(h1_next.ok())?;
        }

        // adjust halfedge->face
        if let Ok(f0) = f0 {
            let hes = f0.halfedges(self).collect_vec();
            for h in hes {
                self.get_mut(h).set_face(f1.ok())?;
            }
        }

        // adjust halfedge->halfedge
        if let Ok(h1_prev) = h1_prev {
            self.get_mut(h1_prev).set_next(h0_next.ok())?;
        }
        if let Ok(h0_prev) = h0_prev {
            self.get_mut(h0_prev).set_next(h1_next.ok())?;
        }

        // adjust face->halfedge
        // if (halfedge(f1) == h1)
        // set_halfedge(f1, h1_next);
        if let Ok(f1) = f1 {
            if f1.halfedge().run(self) == Ok(h1) {
                self.get_mut(f1).set_halfedge(h1_next.ok())?;
            }
        }

        // delete face f0 and edge e
        if let Ok(f0) = f0 {
            self.get_mut(f0).delete()?;
        }
        self.get_mut(h0).delete()?;
        Ok(())
    }

    fn remove_edge_helper(&mut self, h: HalfedgeId) -> SMeshResult<()> {
        let hn = h.next().run(self)?;
        let hp = h.prev().run(self)?;

        let o = h.opposite();
        let on = o.next().run(self)?;
        let op = o.prev().run(self)?;

        let fh = h.face().run(self).ok();
        let fo = o.face().run(self).ok();

        let vh = h.dst_vert().run(self)?;
        let vo = o.dst_vert().run(self)?;

        // halfedge -> vertex
        let he_to_update = vo
            .halfedges(self)
            .filter_map(|hc| hc.opposite().run(self).ok())
            .collect_vec();

        for hc_o in he_to_update {
            self.get_mut(hc_o).set_vertex(vh)?;
        }

        // halfedge -> halfedge
        self.get_mut(hp).set_next(Some(hn))?;
        self.get_mut(op).set_next(Some(on))?;

        // face -> halfedge
        if let Some(fh) = fh {
            self.get_mut(fh).set_halfedge(Some(hn))?;
        }
        if let Some(fo) = fo {
            self.get_mut(fo).set_halfedge(Some(on))?;
        }

        // vertex -> halfedge
        if vh.halfedge().run(self)? == o.run(self)? {
            self.get_mut(vh).set_halfedge(Some(hn))?;
        }
        self.get_mut(vh).adjust_outgoing_halfedge()?;
        self.get_mut(vo).set_halfedge(None)?;

        // Delete
        self.get_mut(vo).delete()?;
        self.get_mut(h).delete()?;

        Ok(())
    }

    fn remove_loop_helper(&mut self, h: HalfedgeId) -> SMeshResult<()> {
        let h0 = h;
        let h1 = h0.next().run(self)?;

        let o0 = h0.opposite().run(self)?;
        let o1 = h1.opposite().run(self)?;

        let v0 = h0.dst_vert().run(self)?;
        let v1 = h1.dst_vert().run(self)?;

        let fh = h0.face().run(self).ok();
        let fo = o0.face().run(self).ok();

        // is it a loop ?
        if !((h1.next().run(self)? == h0) && (h1 != o0)) {
            bail!(TopologyError)
        }

        // halfedge -> halfedge
        let o0_next = o0.next().run(self).ok();
        self.get_mut(h1).set_next(o0_next)?;
        let o0_prev = o0.prev().run(self)?;
        self.get_mut(o0_prev).set_next(Some(h1))?;

        // halfedge -> face
        self.get_mut(h1).set_face(fo)?;

        // vertex -> halfedge
        self.get_mut(v0).set_halfedge(Some(h1))?;
        self.get_mut(v0).adjust_outgoing_halfedge()?;
        self.get_mut(v1).set_halfedge(Some(o1))?;
        self.get_mut(v1).adjust_outgoing_halfedge()?;

        // face -> halfedge
        if let Some(fo) = fo {
            if fo.halfedge().run(self)? == o0 {
                self.get_mut(fo).set_halfedge(Some(h1))?;
            }
        }

        // Delete stuff
        if let Some(fh) = fh {
            self.get_mut(fh).delete()?;
        }
        self.get_mut(h).delete()?;

        Ok(())
    }

    /// Create an edge (2 halfedges) between two isolated vertices
    /// CARE!: This does not take care of connectivity for next/prev edges
    fn add_edge(&mut self, v0: VertexId, v1: VertexId) -> (HalfedgeId, HalfedgeId) {
        let halfedges = self.halfedges_mut();
        let he_0_id = halfedges.insert(Halfedge::default());
        let he_1_id = halfedges.insert(Halfedge::default());
        let he_0 = halfedges.get_mut(he_0_id).unwrap();
        he_0.vertex = v1;
        he_0.opposite = Some(he_1_id);
        let he_1 = halfedges.get_mut(he_1_id).unwrap();
        he_1.vertex = v0;
        he_1.opposite = Some(he_0_id);
        (he_0_id, he_1_id)
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
        let face = mesh.make_face(vec![v0, v1, v2, v3])?;
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
