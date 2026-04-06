use crate::{bail, prelude::*};
use glam::{Quat, Vec3};
use itertools::Itertools;
use selection::MeshSelection;

/// Pivot point to use for transformations
pub enum Pivot {
    /// Use the origin (0,0,0) as the pivot point
    Origin,
    /// Use the center of gravity of the entire mesh as the pivot point
    MeshCog,
    /// Use the center of gravity of the selection as the pivot point
    SelectionCog,
    /// Use a specific point as the pivot point
    Point(Vec3),
}

impl Pivot {
    /// Calculate the concrete pivot point based on the pivot type
    fn calculate<S: Into<MeshSelection>>(&self, mesh: &SMesh, selection: S) -> SMeshResult<Vec3> {
        Ok(match self {
            Pivot::Origin => Vec3::ZERO,
            Pivot::MeshCog => mesh.center_of_gravity(mesh.vertices().collect_vec())?,
            Pivot::SelectionCog => mesh.center_of_gravity(selection)?,
            Pivot::Point(pos) => *pos,
        })
    }
}

/// Methods for transforming mesh elements
impl SMesh {
    /// Translates the selected vertices by a given vector.
    ///
    /// # Parameters
    ///
    /// - `selection`: The selection of vertices, edges, or faces to translate.
    ///   It can be any type that implements `Into<MeshSelection>`.
    /// - `translation`: The vector by which to translate the selected vertices.
    pub fn translate<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        translation: Vec3,
    ) -> SMeshResult<&mut SMesh> {
        let vertices = selection.into().resolve_to_vertices(self)?;
        for id in vertices {
            if let Some(pos) = self.positions.get(id) {
                self.positions.insert(id, *pos + translation);
            }
        }
        Ok(self)
    }

    pub fn set_position<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        translation: Vec3,
        pivot: Pivot,
    ) -> SMeshResult<&mut SMesh> {
        let s = selection.into();
        let p = pivot.calculate(self, s.clone())?;
        let vertices = s.resolve_to_vertices(self)?;
        for id in vertices {
            let mut pos = id.position(self)?;
            pos -= p;
            self.positions.insert(id, pos + translation);
        }
        Ok(self)
    }

    /// Scales the selected vertices by a given factor around a pivot point.
    ///
    /// # Parameters
    ///
    /// - `selection`: The selection of vertices, edges, or faces to scale.
    ///   It can be any type that implements `Into<MeshSelection>`.
    /// - `scale`: The scale factors along the X, Y, and Z axes.
    /// - `pivot`: The pivot point around which the scaling is performed.
    pub fn scale<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        scale: Vec3,
        pivot: Pivot,
    ) -> SMeshResult<&mut SMesh> {
        let s: MeshSelection = selection.into();
        let p = pivot.calculate(self, s.clone())?;
        self.scale_around(s, scale, p)?;
        Ok(self)
    }

    /// Rotates the selected vertices around a pivot point using a quaternion.
    ///
    /// This function calculates the pivot point based on the provided `Pivot` enum
    /// and then rotates the selected vertices accordingly.
    ///
    /// # Parameters
    ///
    /// - `selection`: The selection of vertices, edges, or faces to rotate.
    ///   It can be any type that implements `Into<MeshSelection>`.
    /// - `quaternion`: The rotation represented as a quaternion.
    /// - `pivot`: The pivot point around which the rotation is performed.
    pub fn rotate<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        quaternion: Quat,
        pivot: Pivot,
    ) -> SMeshResult<&mut SMesh> {
        let s: MeshSelection = selection.into();
        let p = pivot.calculate(self, s.clone())?;
        self.rotate_around(s, quaternion, p)?;
        Ok(self)
    }

    /// Calculates the center of gravity (centroid) of the selected vertices.
    ///
    /// This function computes the average position of all selected vertices.
    ///
    /// # Parameters
    ///
    /// - `selection`: The selection of vertices, edges, or faces for which to calculate the center of gravity.
    ///   It can be any type that implements `Into<MeshSelection>`.
    pub fn center_of_gravity<S: Into<MeshSelection>>(&self, selection: S) -> SMeshResult<Vec3> {
        let vertices = selection.into().resolve_to_vertices(self)?;

        if vertices.is_empty() {
            bail!("No vertices in selection");
        }

        let mut sum = Vec3::ZERO;
        for v_id in &vertices {
            let pos = v_id.position(self)?;
            sum += pos;
        }

        let center = sum / vertices.len() as f32;
        Ok(center)
    }

    fn scale_around<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        scale: Vec3,
        pivot: Vec3,
    ) -> SMeshResult<&mut SMesh> {
        let vertices = selection.into().resolve_to_vertices(self)?;
        for id in vertices {
            let mut position = id.position(self)?;
            // Translate vertex so that the pivot is at the desired point
            position -= pivot;
            // Scale the vertex
            position *= scale;
            // Translate the vertex back
            position += pivot;
            self.positions.insert(id, position);
        }
        Ok(self)
    }

    /// Moves selected vertices toward the surface of a sphere.
    ///
    /// Each vertex is lerped between its current position and the corresponding
    /// point on the target sphere (same direction from pivot, at target radius).
    ///
    /// # Parameters
    ///
    /// - `selection`: The vertices to spherize.
    /// - `amount`: Blend factor — 0.0 = no change, 1.0 = on sphere surface.
    /// - `pivot`: Center of the sphere.
    /// - `target_radius`: Radius of the target sphere. If `None`, uses the
    ///   average distance of selected vertices from the pivot.
    pub fn spherize<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        amount: f32,
        pivot: Pivot,
        target_radius: Option<f32>,
    ) -> SMeshResult<&mut SMesh> {
        let s: MeshSelection = selection.into();
        let center = pivot.calculate(self, s.clone())?;
        let vertices: Vec<VertexId> = s.resolve_to_vertices(self)?.into_iter().collect();

        // Compute target radius if not provided (average distance from center)
        let radius = target_radius.unwrap_or_else(|| {
            let total: f32 = vertices
                .iter()
                .filter_map(|v| v.position(self).ok())
                .map(|p| (p - center).length())
                .sum();
            total / vertices.len().max(1) as f32
        });

        for &v in &vertices {
            let pos = v.position(self)?;
            let offset = pos - center;
            let dist = offset.length();
            if dist < 1e-8 {
                continue; // vertex at center, can't determine direction
            }
            // Point on sphere in same direction
            let sphere_pos = center + offset.normalize() * radius;
            let new_pos = pos.lerp(sphere_pos, amount);
            self.positions.insert(v, new_pos);
        }

        Ok(self)
    }

    fn rotate_around<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        quaternion: Quat,
        pivot: Vec3,
    ) -> SMeshResult<&mut SMesh> {
        let vertices = selection.into().resolve_to_vertices(self)?;
        for id in vertices {
            let mut position = id.position(self)?;
            // Translate position so that the rotation origin is at the coordinate origin
            position -= pivot;
            // Apply the rotation
            position = quaternion * position;
            // Translate the position back to its original location
            position += pivot;
            // Update the position in your mesh data
            self.positions.insert(id, position);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn spherize_cube_full() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::splat(2),
        }
        .generate()?;

        let all_verts: Vec<VertexId> = mesh.vertices().collect();
        mesh.spherize(all_verts.clone(), 1.0, Pivot::Origin, None)?;

        // All vertices should now be at the same distance from origin
        let distances: Vec<f32> = all_verts
            .iter()
            .map(|v| v.position(&mesh).unwrap().length())
            .collect();
        let avg = distances.iter().sum::<f32>() / distances.len() as f32;
        for d in &distances {
            assert!(
                (d - avg).abs() < 0.01,
                "All verts should be equidistant after full spherize, got {} vs avg {}",
                d,
                avg
            );
        }

        Ok(())
    }

    #[test]
    fn spherize_zero_amount_no_change() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let positions_before: Vec<Vec3> = mesh
            .vertices()
            .map(|v| v.position(&mesh).unwrap())
            .collect();

        let all: Vec<VertexId> = mesh.vertices().collect();
        mesh.spherize(all, 0.0, Pivot::Origin, None)?;

        let positions_after: Vec<Vec3> = mesh
            .vertices()
            .map(|v| v.position(&mesh).unwrap())
            .collect();

        for (before, after) in positions_before.iter().zip(positions_after.iter()) {
            assert!(
                (*before - *after).length() < 1e-6,
                "Zero amount should not change positions"
            );
        }

        Ok(())
    }

    #[test]
    fn spherize_custom_radius() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let all: Vec<VertexId> = mesh.vertices().collect();
        mesh.spherize(all.clone(), 1.0, Pivot::Origin, Some(2.0))?;

        for v in &all {
            let dist = v.position(&mesh)?.length();
            assert!(
                (dist - 2.0).abs() < 0.01,
                "Full spherize with radius 2.0 should put verts at distance 2.0, got {}",
                dist
            );
        }

        Ok(())
    }
}
