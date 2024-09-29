use glam::Vec3;
use itertools::Itertools;

use crate::{bail, prelude::*};

impl SMesh {
    pub fn extrude_edge(&mut self, e0: HalfedgeId) -> SMeshResult<HalfedgeId> {
        // Find boundary halfedge
        let e0 = match e0.is_boundary(self) {
            true => e0,
            false => {
                let opposite = e0.opposite().run(self)?;
                if !opposite.is_boundary(self) {
                    bail!("Can only extrude boundary edges");
                }
                opposite
            }
        };

        let v0 = e0.src_vert().run(self)?;
        let v1 = e0.dst_vert().run(self)?;

        let pos0 = v0.position(self)?;
        let pos1 = v1.position(self)?;
        let v0_new = self.add_vertex(pos0);
        let v1_new = self.add_vertex(pos1);

        // TODO: maybe check vertex normals (if exist) to determine order?
        self.make_face(vec![v0, v1, v1_new, v0_new])?;
        let new_edge = v0_new.halfedge_to(v1_new).run(self)?;
        Ok(new_edge)
    }

    pub fn extrude_vertices(
        &mut self,
        vertices: Vec<VertexId>,
        direction: Vec3,
    ) -> SMeshResult<Vec<VertexId>> {
        let mut new_vertices = Vec::new();

        for v0 in vertices.iter() {
            // let v1 = self.extrude_vertex(*v0, direction)?;
            // new_vertices.push(v1);
        }

        for v0 in vertices.iter() {
            let connected = v0
                .vertices(self)
                .filter(|neighbour| vertices.contains(neighbour))
                .collect_vec();
        }
        todo!();
        Ok(new_vertices)
    }

    // Returns the normal of the face. The first three vertices are used to
    // compute the normal. If the vertices of the face are not coplanar,
    // the result will not be correct.
    fn face_normal(&self, face: FaceId) -> Option<Vec3> {
        let verts = face.vertices(self).collect_vec();
        eprintln!("Face verts: {:?}", verts);
        for v in &verts {
            eprintln!("V pos: {:?}", self.positions[*v]);
        }
        if verts.len() >= 3 {
            let v01 = self.positions[verts[0]] - self.positions[verts[1]];
            let v12 = self.positions[verts[1]] - self.positions[verts[2]];
            Some(v01.cross(v12).normalize())
        } else {
            None
        }
    }
}
