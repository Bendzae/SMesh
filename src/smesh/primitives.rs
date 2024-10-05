use std::collections::HashMap;

use glam::{vec3, U16Vec3};

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
