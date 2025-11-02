use glam::Vec2;
use slotmap::SecondaryMap;
use xatlas_rs::{ChartOptions, MeshData, MeshDecl, PackOptions, Xatlas};

use crate::prelude::*;

pub struct XatlasOptions {
    pub chart: ChartOptions,
    pub pack: PackOptions,
}

impl Default for XatlasOptions {
    fn default() -> Self {
        Self {
            chart: ChartOptions::default(),
            pack: PackOptions::default(),
        }
    }
}

impl SMesh {
    /// Generate UV atlas using automatic unwrapping via xatlas.
    ///
    /// This method clears all existing UVs (both vertex and halfedge UVs) and generates
    /// new halfedge UVs using automatic chart-based unwrapping.
    pub fn auto_uv_unwrap(&mut self) -> SMeshResult<()> {
        self.auto_uv_unwrap_with_options(XatlasOptions::default())
    }

    /// Generate UV atlas with custom options using automatic unwrapping via xatlas.
    ///
    /// This method clears all existing UVs (both vertex and halfedge UVs) and generates
    /// new halfedge UVs using automatic chart-based unwrapping with the specified options.
    pub fn auto_uv_unwrap_with_options(&mut self, options: XatlasOptions) -> SMeshResult<()> {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_map = Vec::new();

        for face_id in self.faces() {
            let face_halfedges: Vec<_> = face_id.halfedges(&self).collect();

            if face_halfedges.len() < 3 {
                continue;
            }

            let base_index = (positions.len() / 3) as u32;

            for he_id in &face_halfedges {
                let vertex_id = he_id.dst_vert().run(self)?;
                let pos = self.positions.get(vertex_id).ok_or(SMeshError::TopologyError)?;
                positions.push(pos.x);
                positions.push(pos.y);
                positions.push(pos.z);
                vertex_map.push(*he_id);
            }

            for i in 1..(face_halfedges.len() - 1) {
                indices.push(base_index);
                indices.push(base_index + i as u32);
                indices.push(base_index + (i + 1) as u32);
            }
        }

        if positions.is_empty() || indices.is_empty() {
            return Err(SMeshError::TopologyError);
        }

        let mesh_decl = MeshDecl {
            vertex_position_data: MeshData::Contiguous(&positions),
            index_data: Some(xatlas_rs::IndexData::U32(&indices)),
            face_count: (indices.len() / 3) as u32,
            ..Default::default()
        };

        let (uv_data, atlas_width, atlas_height) = {
            let mut atlas = Xatlas::new();
            
            atlas.add_mesh(&mesh_decl).map_err(|_| SMeshError::TopologyError)?;
            atlas.generate(&options.chart, &options.pack);

            let meshes = atlas.meshes();
            if meshes.is_empty() {
                return Err(SMeshError::TopologyError);
            }

            let xatlas_mesh = &meshes[0];
            let result = xatlas_mesh
                .vertex_array
                .iter()
                .map(|vertex| (vertex.xref as usize, Vec2::new(vertex.uv[0], vertex.uv[1])))
                .collect::<Vec<_>>();
            
            let width = atlas.width();
            let height = atlas.height();
            
            drop(atlas);
            (result, width, height)
        };

        let atlas_width = atlas_width as f32;
        let atlas_height = atlas_height as f32;

        self.vertex_uvs = None;
        self.halfedge_uvs = Some(SecondaryMap::new());

        if let Some(ref mut he_uvs) = self.halfedge_uvs {
            for (xref, mut uv) in uv_data {
                if atlas_width > 0.0 {
                    uv.x /= atlas_width;
                }
                if atlas_height > 0.0 {
                    uv.y /= atlas_height;
                }
                let original_he = vertex_map[xref];
                he_uvs.insert(original_he, uv);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smesh::primitives::{Cube, Primitive};
    use glam::U16Vec3;

    #[test]
    fn test_xatlas_clears_vertex_uvs() {
        let (mut cube, _) = Cube {
            subdivision: U16Vec3::new(1, 1, 1),
        }
        .generate()
        .unwrap();

        cube.vertex_uvs = Some(Default::default());
        assert!(cube.vertex_uvs.is_some());

        cube.auto_uv_unwrap().unwrap();

        assert!(cube.vertex_uvs.is_none(), "vertex_uvs should be cleared");
        assert!(cube.halfedge_uvs.is_some(), "halfedge_uvs should be set");
    }

    #[test]
    fn test_generate_uv_atlas() {
        let (mut cube, _) = Cube {
            subdivision: U16Vec3::new(1, 1, 1),
        }
        .generate()
        .unwrap();

        cube.auto_uv_unwrap().unwrap();

        assert!(cube.halfedge_uvs.is_some());

        if let Some(ref uvs) = cube.halfedge_uvs {
            let uv_count = cube.halfedges().filter(|he| uvs.contains_key(*he)).count();
            assert!(uv_count > 0, "Should have generated UVs for halfedges");

            for he in cube.halfedges() {
                if let Some(uv) = uvs.get(he) {
                    assert!(
                        uv.x >= 0.0 && uv.x <= 1.0,
                        "UV x coordinate should be in [0,1] range"
                    );
                    assert!(
                        uv.y >= 0.0 && uv.y <= 1.0,
                        "UV y coordinate should be in [0,1] range"
                    );
                }
            }
        }
    }
}
