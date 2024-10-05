use glam::vec3;

use crate::prelude::*;

pub struct Cube {
    pub subdivisions: usize,
}

pub struct CubeData {
    pub front_bottom_left_vertex: VertexId,
}

impl Cube {
    pub fn generate(self) -> SMeshResult<(SMesh, CubeData)> {
        // Construct SMesh
        let mut smesh = SMesh::new();
        let half_extent = 0.5;
        let v0 = smesh.add_vertex(vec3(-half_extent, -half_extent, half_extent));
        let v1 = smesh.add_vertex(vec3(half_extent, -half_extent, half_extent));
        let v2 = smesh.add_vertex(vec3(half_extent, half_extent, half_extent));
        let v3 = smesh.add_vertex(vec3(-half_extent, half_extent, half_extent));

        let v4 = smesh.add_vertex(vec3(-half_extent, -half_extent, -half_extent));
        let v5 = smesh.add_vertex(vec3(half_extent, -half_extent, -half_extent));
        let v6 = smesh.add_vertex(vec3(half_extent, half_extent, -half_extent));
        let v7 = smesh.add_vertex(vec3(-half_extent, half_extent, -half_extent));

        // Front
        smesh.make_face(vec![v0, v1, v2, v3])?;
        // Right
        smesh.make_face(vec![v1, v5, v6, v2])?;
        // Back
        smesh.make_face(vec![v5, v4, v7, v6])?;
        // Left
        smesh.make_face(vec![v4, v0, v3, v7])?;
        // Top
        smesh.make_face(vec![v3, v2, v6, v7])?;
        // Bottom
        smesh.make_face(vec![v4, v5, v1, v0])?;

        smesh.recalculate_normals()?;
        Ok((
            smesh,
            CubeData {
                front_bottom_left_vertex: v0,
            },
        ))
    }
}
