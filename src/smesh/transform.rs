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
