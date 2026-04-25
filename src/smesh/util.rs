//! Normal, centroid, and selection-wide helpers on [`SMesh`].
//!
//! These are small utilities that don't belong to a larger theme but come up
//! everywhere — recomputing normals after an edit, getting the centre of a
//! face, selecting every vertex at once.

use glam::Vec3;
use itertools::Itertools;
use slotmap::SecondaryMap;

use crate::prelude::*;

impl SMesh {
    /// Recompute per-face and per-vertex normals from vertex positions.
    ///
    /// Face normals use the first three vertices of each face (valid for
    /// planar polygons). Vertex normals are the unweighted sum of incident
    /// face normals, then normalised.
    ///
    /// Idempotent: always safe to call after a geometry-changing edit.
    pub fn recalculate_normals(&mut self) -> SMeshResult<()> {
        // # Step 1: Initialize all vertex normals to zero
        let mut vertex_normals = SecondaryMap::default();
        for v in self.vertices() {
            vertex_normals.insert(v, Vec3::ZERO);
        }
        let mut face_normals = SecondaryMap::default();
        for f in self.faces() {
            face_normals.insert(f, Vec3::ZERO);
        }
        // Step 2: Compute face normals and accumulate into vertex normals
        for face in self.faces() {
            // Get the three vertices of the face
            let face_vertices = face.vertices(self).collect_vec();
            let v0 = face_vertices.get(0).unwrap();
            let v1 = face_vertices.get(1).unwrap();
            let v2 = face_vertices.get(2).unwrap();

            let p0 = v0.position(self)?;
            let p1 = v1.position(self)?;
            let p2 = v2.position(self)?;

            // Compute two edge vectors
            let edge1 = p1 - p0;
            let edge2 = p2 - p0;

            // Compute face normal using the cross product of the two edge vectors
            let face_normal = edge1.cross(edge2).normalize();
            face_normals.insert(face, face_normal);
            // Accumulate the face normal to each vertex's normal
            for v in face_vertices {
                let current_normal = *vertex_normals.get(v).unwrap();
                vertex_normals.insert(v, current_normal + face_normal);
            }
        }
        // Step 3: Normalize each vertex normal
        let mut normalized_vertex_normals = SecondaryMap::default();
        for (v, n) in vertex_normals {
            normalized_vertex_normals.insert(v, n.normalize());
        }
        // Apply to smesh
        self.vertex_normals = Some(normalized_vertex_normals);
        self.face_normals = Some(face_normals);
        Ok(())
    }

    /// Invert every face and vertex normal in place.
    ///
    /// **Incomplete**: the winding-order reversal needed to make flipping
    /// topologically consistent is still TODO, so this currently panics.
    /// Re-call [`recalculate_normals`](Self::recalculate_normals) with a
    /// mesh whose faces have the desired winding instead.
    pub fn flip_normals(&mut self) -> SMeshResult<()> {
        // Step 1: Flip Face Normals
        if let Some(face_normals) = &mut self.face_normals {
            for (_face, normal) in face_normals.iter_mut() {
                *normal = -*normal;
            }
        }

        // Step 2: Flip Vertex Normals
        if let Some(vertex_normals) = &mut self.vertex_normals {
            for (_vertex, normal) in vertex_normals.iter_mut() {
                *normal = -*normal;
            }
        }

        // // Step 3: Reverse the Winding Order of Each Face
        // let faces_to_update = self.faces().collect_vec();
        // for face in faces_to_update {
        //     // Collect the vertices of the face
        //     let vertices: Vec<VertexId> = face.vertices(self).collect();
        //
        //     // Delete the face
        //     self.delete_only_face(face)?;
        //
        //     // Reverse the order of vertices
        //     let reversed_vertices: Vec<VertexId> = vertices.into_iter().rev().collect();
        //
        //     // Recreate the face with reversed vertices
        //     self.make_face(reversed_vertices)?;
        // }
        todo!();

        Ok(())
    }
    /// Arithmetic mean of the face's vertex positions.
    pub fn get_face_centroid(&self, face: FaceId) -> SMeshResult<Vec3> {
        let face_vertices = face.vertices(self).collect_vec();
        let mut centroid = Vec3::ZERO;
        for v in face.vertices(self) {
            let vertex_position = v.position(self)?;
            centroid += vertex_position;
        }
        centroid /= face_vertices.len() as f32;
        Ok(centroid)
    }

    /// Return a [`MeshSelection`] containing every vertex in the mesh.
    /// Handy as a starting point for global transforms or subdivisions.
    pub fn select_all(&self) -> MeshSelection {
        self.vertices().collect_vec().into()
    }
}
