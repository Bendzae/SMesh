use glam::Vec3;
use itertools::Itertools;
use slotmap::SecondaryMap;

use crate::prelude::*;

impl SMesh {
    pub fn recalculate_normals(&mut self) -> SMeshResult<()> {
        // # Step 1: Initialize all vertex normals to zero
        let mut vertex_normals = SecondaryMap::default();
        for v in self.vertices().keys() {
            vertex_normals.insert(v, Vec3::ZERO);
        }
        let mut face_normals = SecondaryMap::default();
        for f in self.faces().keys() {
            face_normals.insert(f, Vec3::ZERO);
        }
        // Step 2: Compute face normals and accumulate into vertex normals
        for face in self.faces().keys() {
            // Get the three vertices of the face
            let face_vertices = face.vertices(self).collect_vec();
            let v0 = face_vertices.get(0).unwrap();
            let v1 = face_vertices.get(1).unwrap();
            let v2 = face_vertices.get(2).unwrap();

            let p0 = *self.positions.get(*v0).unwrap();
            let p1 = *self.positions.get(*v1).unwrap();
            let p2 = *self.positions.get(*v2).unwrap();

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
}
