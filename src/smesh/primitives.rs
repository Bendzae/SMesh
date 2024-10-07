use std::collections::HashMap;

use glam::{vec3, U16Vec3};
use itertools::Itertools;

use crate::prelude::*;

pub struct Cube {
    pub subdivision: U16Vec3,
}

pub struct CubeData {
    pub front_bottom_left_vertex: VertexId,
}

impl Cube {
    pub fn generate(self) -> SMeshResult<(SMesh, CubeData)> {
        let n_x = self.subdivision.x;
        let n_y = self.subdivision.y;
        let n_z = self.subdivision.z;
        let delta_x = 1.0 / (n_x as f32);
        let delta_y = 1.0 / (n_y as f32);
        let delta_z = 1.0 / (n_z as f32);

        // Construct SMesh
        let mut smesh = SMesh::new();
        let mut vertex_indices = HashMap::new();

        // Generate vertices on the cube's surface
        for x in 0..=n_x {
            for y in 0..=n_y {
                for z in 0..=n_z {
                    // Only add vertices on the surfaces (where x, y, or z is 0 or n)
                    if x == 0 || x == n_x || y == 0 || y == n_y || z == 0 || z == n_z {
                        let pos_x = -0.5 + (x as f32) * delta_x;
                        let pos_y = -0.5 + (y as f32) * delta_y;
                        let pos_z = -0.5 + (z as f32) * delta_z;
                        let position = vec3(pos_x, pos_y, pos_z);
                        let vertex = smesh.add_vertex(position);
                        vertex_indices.insert((x, y, z), vertex);
                    }
                }
            }
        }

        // Helper function to retrieve vertex indices
        let get_vertex = |x, y, z| -> VertexId { *vertex_indices.get(&(x, y, z)).unwrap() };

        // Generate faces for each side of the cube with correct winding order
        // Front face (z = n)
        for x in 0..n_x {
            for y in 0..n_y {
                let v0 = get_vertex(x, y, n_z);
                let v1 = get_vertex(x + 1, y, n_z);
                let v2 = get_vertex(x + 1, y + 1, n_z);
                let v3 = get_vertex(x, y + 1, n_z);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Correct winding
            }
        }

        // Back face (z = 0)
        for x in 0..n_x {
            for y in 0..n_y {
                let v0 = get_vertex(x, y, 0);
                let v1 = get_vertex(x, y + 1, 0);
                let v2 = get_vertex(x + 1, y + 1, 0);
                let v3 = get_vertex(x + 1, y, 0);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Correct winding
            }
        }

        // Left face (x = 0)
        for y in 0..n_y {
            for z in 0..n_z {
                let v0 = get_vertex(0, y, z);
                let v1 = get_vertex(0, y, z + 1);
                let v2 = get_vertex(0, y + 1, z + 1);
                let v3 = get_vertex(0, y + 1, z);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Corrected winding
            }
        }

        // Right face (x = n)
        for y in 0..n_y {
            for z in 0..n_z {
                let v0 = get_vertex(n_x, y, z);
                let v1 = get_vertex(n_x, y + 1, z);
                let v2 = get_vertex(n_x, y + 1, z + 1);
                let v3 = get_vertex(n_x, y, z + 1);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Corrected winding
            }
        }

        // Top face (y = n)
        for x in 0..n_x {
            for z in 0..n_z {
                let v0 = get_vertex(x, n_y, z);
                let v1 = get_vertex(x, n_y, z + 1);
                let v2 = get_vertex(x + 1, n_y, z + 1);
                let v3 = get_vertex(x + 1, n_y, z);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Corrected winding
            }
        }

        // Bottom face (y = 0)
        for x in 0..n_x {
            for z in 0..n_z {
                let v0 = get_vertex(x, 0, z);
                let v1 = get_vertex(x + 1, 0, z);
                let v2 = get_vertex(x + 1, 0, z + 1);
                let v3 = get_vertex(x, 0, z + 1);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Corrected winding
            }
        }

        smesh.recalculate_normals()?;
        Ok((
            smesh,
            CubeData {
                front_bottom_left_vertex: get_vertex(0, 0, n_z),
            },
        ))
    }
}

pub struct Icosphere {
    pub subdivisions: usize,
}

pub struct IcosphereData {
    pub top_vertex: VertexId,
    pub bottom_vertex: VertexId,
}

impl Icosphere {
    pub fn generate(self) -> SMeshResult<(SMesh, IcosphereData)> {
        let subdivisions = self.subdivisions;
        // Create an initial icosahedron
        let mut smesh = SMesh::new();
        let t = (1.0 + 5_f32.sqrt()) / 2.0;

        // Create initial vertices of an icosahedron
        let mut vertices = vec![
            vec3(-1.0, t, 0.0),
            vec3(1.0, t, 0.0),
            vec3(-1.0, -t, 0.0),
            vec3(1.0, -t, 0.0),
            vec3(0.0, -1.0, t),
            vec3(0.0, 1.0, t),
            vec3(0.0, -1.0, -t),
            vec3(0.0, 1.0, -t),
            vec3(t, 0.0, -1.0),
            vec3(t, 0.0, 1.0),
            vec3(-t, 0.0, -1.0),
            vec3(-t, 0.0, 1.0),
        ];

        // Normalize vertices to make them lie on the sphere surface
        for v in &mut vertices {
            *v = v.normalize();
        }

        // Add vertices to the mesh
        let mut vertex_ids = Vec::new();
        for &position in &vertices {
            let id = smesh.add_vertex(position * 0.5); // Scale to fit within [-0.5, 0.5]
            vertex_ids.push(id);
        }

        // Define the 20 faces of the icosahedron
        let faces = vec![
            // 5 faces around point 0
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            // 5 adjacent faces
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            // 5 faces around point 3
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            // 5 adjacent faces
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];

        // Create faces
        let mut face_ids = Vec::new();
        for &face in &faces {
            let [i0, i1, i2] = face;
            let v0 = vertex_ids[i0];
            let v1 = vertex_ids[i1];
            let v2 = vertex_ids[i2];
            let face = smesh.make_face(vec![v0, v1, v2])?;
            face_ids.push(face);
        }

        // Subdivide faces
        let mut mid_point_cache = HashMap::<(VertexId, VertexId), VertexId>::new();

        for _ in 0..subdivisions {
            let mut new_faces = Vec::new();
            let mut face_vertex_cache = Vec::new();
            for face in &face_ids {
                let face_vertices = face.vertices(&smesh).collect_vec();
                let v0 = face_vertices[0];
                let v1 = face_vertices[1];
                let v2 = face_vertices[2];

                // Create or get midpoints
                let a = get_midpoint(&mut smesh, &mut mid_point_cache, v0, v1);
                let b = get_midpoint(&mut smesh, &mut mid_point_cache, v1, v2);
                let c = get_midpoint(&mut smesh, &mut mid_point_cache, v2, v0);

                // Save new faces
                face_vertex_cache.push(vec![v0, a, c]);
                face_vertex_cache.push(vec![v1, b, a]);
                face_vertex_cache.push(vec![v2, c, b]);
                face_vertex_cache.push(vec![a, b, c]);
            }
            // Delete all connections
            for he in smesh.halfedges().collect_vec() {
                smesh.delete_only_edge(he)?;
            }
            for v in smesh.vertices().collect_vec() {
                smesh.get_mut(v).set_halfedge(None)?;
            }
            // Create faces
            for vertices in face_vertex_cache {
                new_faces.push(smesh.make_face(vertices)?);
            }
            face_ids = new_faces;
        }

        // Normalize all vertices to lie on the sphere
        for vertex_index in smesh.vertices().collect_vec() {
            let position = vertex_index.position(&smesh)?;
            let normalized_position = position.normalize() * 0.5; // Scale to radius 0.5
            smesh.positions.insert(vertex_index, normalized_position);
        }

        smesh.recalculate_normals()?;
        Ok((
            smesh,
            IcosphereData {
                top_vertex: vertex_ids[0],
                bottom_vertex: vertex_ids[1],
            },
        ))
    }
}

// Helper function to get the midpoint between two vertices and add it to the mesh
fn get_midpoint(
    smesh: &mut SMesh,
    cache: &mut HashMap<(VertexId, VertexId), VertexId>,
    v1: VertexId,
    v2: VertexId,
) -> VertexId {
    let smaller_index = std::cmp::min(v1, v2);
    let greater_index = std::cmp::max(v1, v2);
    let key = (smaller_index, greater_index);
    if let Some(&mid_vertex) = cache.get(&key) {
        mid_vertex
    } else {
        let p1 = v1.position(smesh).unwrap();
        let p2 = v2.position(smesh).unwrap();
        let mid_point = ((p1 + p2) * 0.5).normalize() * 0.5;
        let mid_vertex = smesh.add_vertex(mid_point);
        cache.insert(key, mid_vertex);
        mid_vertex
    }
}
