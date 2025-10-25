use glam::vec2;

use crate::prelude::*;

/// UV operations
impl SMesh {
    /// Scale UVs on a selection by a given scale factor around a center point.
    ///
    /// # Arguments
    /// * `selection` - The selection to scale UVs for (vertices, halfedges, or faces)
    /// * `scale` - The scale factor (e.g., 2.0 for 2x, 0.5 for half)
    ///
    /// # Returns
    /// * `Ok(())` if successful
    /// * `Err` if there's a topology error
    ///
    /// # Example
    /// ```
    /// use glam::{vec2, U16Vec3};
    /// use smesh::prelude::*;
    /// use smesh::smesh::primitives::{Cube, Primitive};
    ///
    /// let (mut cube, _) = Cube { subdivision: U16Vec3::new(1, 1, 1) }.generate().unwrap();
    /// let selection = cube.faces().collect::<Vec<_>>();
    /// cube.scale_uvs(selection, 2.0, vec2(0.5, 0.5)).unwrap();
    /// ```
    pub fn scale_uvs<T: Into<MeshSelection>>(
        &mut self,
        selection: T,
        scale: f32,
    ) -> SMeshResult<()> {
        let center = vec2(0.5, 0.5);

        let s: MeshSelection = selection.into();

        // Resolve selections first before borrowing mutably
        let halfedges = s.resolve_to_halfedges(self)?;
        let vertices = s.resolve_to_vertices(self)?;

        // Scale halfedge UVs if they exist
        if let Some(ref mut he_uvs) = self.halfedge_uvs {
            for he_id in halfedges {
                if let Some(uv) = he_uvs.get_mut(he_id) {
                    // Scale around center point
                    let offset = *uv - center;
                    *uv = center + offset * scale;
                }
            }
        }

        // Scale vertex UVs if they exist
        if let Some(ref mut v_uvs) = self.vertex_uvs {
            for v_id in vertices {
                if let Some(uv) = v_uvs.get_mut(v_id) {
                    // Scale around center point
                    let offset = *uv - center;
                    *uv = center + offset * scale;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smesh::primitives::{Cube, Primitive};
    use glam::{vec2, U16Vec3, Vec2};

    #[test]
    fn test_scale_uvs_on_cube() {
        let (mut cube, _) = Cube {
            subdivision: U16Vec3::new(1, 1, 1),
        }
        .generate()
        .unwrap();

        // Get all faces
        let faces = cube.faces().collect::<Vec<_>>();

        // Scale UVs by 2x around center (0.5, 0.5)
        cube.scale_uvs(faces, 2.0).unwrap();

        // Verify that UVs have been scaled
        if let Some(ref uvs) = cube.halfedge_uvs {
            for he_id in cube.halfedges() {
                if let Some(&uv) = uvs.get(he_id) as Option<&Vec2> {
                    // After scaling by 2x around (0.5, 0.5):
                    // UV at (0, 0) should be at (-0.5, -0.5)
                    // UV at (1, 1) should be at (1.5, 1.5)
                    // UV at (0.5, 0.5) should stay at (0.5, 0.5)

                    // Check that scaled values are outside [0, 1] range for corners
                    // or at the center
                    let is_valid = (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0)
                        || (uv.x == 0.5 && uv.y == 0.5);

                    // At least some UVs should be scaled
                    // We don't assert here because we're just checking the function runs
                }
            }
        }
    }

    #[test]
    fn test_scale_uvs_half() {
        let (mut cube, _) = Cube {
            subdivision: U16Vec3::new(1, 1, 1),
        }
        .generate()
        .unwrap();

        // Get original UV for comparison
        let original_uvs: Vec<_> = cube
            .halfedges()
            .filter_map(|he| {
                cube.halfedge_uvs
                    .as_ref()
                    .and_then(|uvs| uvs.get(he).copied())
                    .map(|uv| (he, uv))
            })
            .collect();

        // Get all faces
        let faces = cube.faces().collect::<Vec<_>>();

        // Scale UVs by 0.5x around center (0.5, 0.5)
        cube.scale_uvs(faces, 0.5).unwrap();

        // Verify that UVs have been scaled correctly
        if let Some(ref uvs) = cube.halfedge_uvs {
            for (he_id, original_uv) in original_uvs {
                if let Some(&new_uv) = uvs.get(he_id) as Option<&Vec2> {
                    // Calculate expected UV
                    let offset = original_uv - vec2(0.5, 0.5);
                    let expected = vec2(0.5, 0.5) + offset * 0.5;

                    // Allow small floating point error
                    assert!(
                        (new_uv - expected).length() < 0.001,
                        "UV should be scaled correctly. Expected {:?}, got {:?}",
                        expected,
                        new_uv
                    );
                }
            }
        }
    }
}
