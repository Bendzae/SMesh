use bevy::render::{
    mesh::{Indices, Mesh, PrimitiveTopology},
    render_asset::RenderAssetUsages,
};
use glam::{Vec2, Vec3};
use itertools::Itertools;

use crate::prelude::*;

impl From<SMesh> for Mesh {
    fn from(smesh: SMesh) -> Self {
        let buffers = smesh.to_buffers().unwrap();

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, buffers.positions)
        // .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, buffers.uvs)
        // .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, buffers.normals)
        .with_inserted_indices(Indices::U32(buffers.indices))
        .with_duplicated_vertices()
        .with_computed_flat_normals()
    }
}

/// Classical indexed mesh representation
#[derive(Clone, Debug)]
pub struct VertexIndexUvBuffers {
    /// Vertex positions, one per vertex.
    pub positions: Vec<Vec3>,
    /// Vertex normals, one per vertex.
    pub normals: Vec<Vec3>,
    /// UV coordinated, one per vertex
    pub uvs: Vec<Vec2>,
    /// Indices: 3*N where N is the number of triangles. Indices point to
    /// elements of `positions` and `normals`.
    pub indices: Vec<u32>,
}

impl SMesh {
    fn to_buffers(&self) -> Result<VertexIndexUvBuffers, SMeshError> {
        let mut positions = vec![];
        let mut uvs = vec![];
        let mut normals = vec![];

        for (face_id, _face) in self.faces() {
            let face_normal = self.face_normals.as_ref().map(|n| n[face_id]);
            let vertices: Vec<VertexId> = face_id.vertices(&self).collect();

            let v1 = vertices[0];

            for (&v2, &v3) in vertices[1..].iter().tuple_windows() {
                positions.push(self.positions[v1]);
                positions.push(self.positions[v2]);
                positions.push(self.positions[v3]);

                if let Some(mesh_uvs) = self.uvs.as_ref() {
                    uvs.push(mesh_uvs[v1.halfedge().run(self)?]);
                    uvs.push(mesh_uvs[v2.halfedge().run(self)?]);
                    uvs.push(mesh_uvs[v3.halfedge().run(self)?]);
                }
                if let Some(normal) = face_normal {
                    normals.push(normal);
                    normals.push(normal);
                    normals.push(normal);
                }
            }
        }

        Ok(VertexIndexUvBuffers {
            indices: (0u32..positions.len() as u32).collect(),
            positions,
            uvs,
            normals,
        })
    }
}
