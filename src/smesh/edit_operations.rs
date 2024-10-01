use std::thread::current;

use bevy::{log::info, reflect::List};
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

    pub fn extrude_edge_chain(&mut self, edges: Vec<HalfedgeId>) -> SMeshResult<Vec<HalfedgeId>> {
        let mut boundary_edges = Vec::new();
        for e in edges {
            let eb = match e.is_boundary(self) {
                true => e,
                false => {
                    let opposite = e.opposite().run(self)?;
                    if !opposite.is_boundary(self) {
                        bail!("Can only extrude boundary edges");
                    }
                    opposite
                }
            };
            boundary_edges.push(eb);
        }

        // Assert all are connected in sequence and check if they form a loop
        let mut is_loop = false;
        for (i, (current, next)) in boundary_edges.iter().circular_tuple_windows().enumerate() {
            let is_last_iteration = i == boundary_edges.len() - 1;
            match current.next().run(self) {
                Ok(he) => {
                    if he == *next {
                        if is_last_iteration {
                            is_loop = true;
                        }
                    } else {
                        bail!("Not an edge chain");
                    }
                }
                Err(_) => {
                    if !is_last_iteration {
                        bail!("Halfedge has no next")
                    }
                }
            }
        }

        let vertices = boundary_edges
            .iter()
            .flat_map(|e| {
                vec![
                    e.src_vert().run(self).unwrap(),
                    e.dst_vert().run(self).unwrap(),
                ]
            })
            .collect_vec();
        // Duplicate verts
        let mut vertex_pairs = Vec::new();
        for v in &vertices {
            let position = v.position(self)?;
            vertex_pairs.push((*v, self.add_vertex(position)));
        }

        assert_eq!(vertices.len(), vertex_pairs.len());

        // Make faces
        let mut new_edges = Vec::new();
        for ((old_0, new_0), (old_1, new_1)) in vertex_pairs
            .iter()
            .copied()
            .circular_tuple_windows()
            .take(vertex_pairs.len() - (if is_loop { 0 } else { 1 }))
        {
            self.make_quad(old_0, old_1, new_1, new_0)?;
            new_edges.push((new_0).halfedge_to(new_1).run(self)?);
        }
        Ok(new_edges)
    }

    pub fn extrude(&mut self, face: FaceId) -> SMeshResult<FaceId> {
        let vertices = face.vertices(self).collect_vec();
        // Duplicate verts
        let mut vertex_pairs = Vec::new();
        for v in &vertices {
            let position = v.position(self)?;
            vertex_pairs.push((*v, self.add_vertex(position)));
        }

        assert_eq!(vertices.len(), vertex_pairs.len());

        // Make faces
        for ((old_0, new_0), (old_1, new_1)) in
            vertex_pairs.iter().copied().circular_tuple_windows()
        {
            self.make_quad(old_0, old_1, new_1, new_0)?;
        }
        let top_face = self.make_face(vertex_pairs.iter().map(|(_old, new)| *new).collect_vec())?;
        Ok(top_face)
    }

    // Returns the normal of the face. The first three vertices are used to
    // compute the normal. If the vertices of the face are not coplanar,
    // the result will not be correct.
    fn calculate_face_normal(&self, face: FaceId) -> Option<Vec3> {
        let verts = face.vertices(self).collect_vec();
        if verts.len() >= 3 {
            let v01 = self.positions[verts[0]] - self.positions[verts[1]];
            let v12 = self.positions[verts[1]] - self.positions[verts[2]];
            Some(v01.cross(v12).normalize())
        } else {
            None
        }
    }
}
