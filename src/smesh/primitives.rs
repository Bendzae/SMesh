use std::collections::HashMap;

use glam::vec3;

use crate::prelude::*;

pub struct Cube {
    pub subdivisions: usize,
}

pub struct CubeData {
    pub front_bottom_left_vertex: VertexId,
}

impl Cube {
    pub fn generate_s(self) -> SMeshResult<(SMesh, CubeData)> {
        let n = self.subdivisions;
        let delta = 1.0 / (n as f32);

        // Construct SMesh
        let mut smesh = SMesh::new();
        let mut vertex_indices = HashMap::new();

        // Generate vertices on the cube's surface
        for x in 0..=n {
            for y in 0..=n {
                for z in 0..=n {
                    // Only add vertices on the surfaces (where x, y, or z is 0 or n)
                    if x == 0 || x == n || y == 0 || y == n || z == 0 || z == n {
                        let pos_x = -0.5 + (x as f32) * delta;
                        let pos_y = -0.5 + (y as f32) * delta;
                        let pos_z = -0.5 + (z as f32) * delta;
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
        for x in 0..n {
            for y in 0..n {
                let v0 = get_vertex(x, y, n);
                let v1 = get_vertex(x + 1, y, n);
                let v2 = get_vertex(x + 1, y + 1, n);
                let v3 = get_vertex(x, y + 1, n);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Correct winding
            }
        }

        // Back face (z = 0)
        for x in 0..n {
            for y in 0..n {
                let v0 = get_vertex(x, y, 0);
                let v1 = get_vertex(x, y + 1, 0);
                let v2 = get_vertex(x + 1, y + 1, 0);
                let v3 = get_vertex(x + 1, y, 0);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Correct winding
            }
        }

        // Left face (x = 0)
        for y in 0..n {
            for z in 0..n {
                let v0 = get_vertex(0, y, z);
                let v1 = get_vertex(0, y, z + 1);
                let v2 = get_vertex(0, y + 1, z + 1);
                let v3 = get_vertex(0, y + 1, z);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Corrected winding
            }
        }

        // Right face (x = n)
        for y in 0..n {
            for z in 0..n {
                let v0 = get_vertex(n, y, z);
                let v1 = get_vertex(n, y + 1, z);
                let v2 = get_vertex(n, y + 1, z + 1);
                let v3 = get_vertex(n, y, z + 1);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Corrected winding
            }
        }

        // Top face (y = n)
        for x in 0..n {
            for z in 0..n {
                let v0 = get_vertex(x, n, z);
                let v1 = get_vertex(x, n, z + 1);
                let v2 = get_vertex(x + 1, n, z + 1);
                let v3 = get_vertex(x + 1, n, z);
                smesh.make_face(vec![v0, v1, v2, v3])?; // Corrected winding
            }
        }

        // Bottom face (y = 0)
        for x in 0..n {
            for z in 0..n {
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
                front_bottom_left_vertex: get_vertex(0, 0, n),
            },
        ))
    }
}
