use crate::{bail, prelude::*};
use glam::{Quat, Vec3};
use itertools::Itertools;
use selection::MeshSelection;

pub enum Pivot {
    Zero,
    MeshCog,
    SelectionCog,
    Local(Vec3),
}

impl Pivot {
    fn calculate<S: Into<MeshSelection>>(&self, mesh: &SMesh, selection: S) -> SMeshResult<Vec3> {
        Ok(match self {
            Pivot::Zero => Vec3::ZERO,
            Pivot::MeshCog => mesh.center_of_gravity(mesh.vertices().collect_vec())?,
            Pivot::SelectionCog => mesh.center_of_gravity(selection)?,
            Pivot::Local(pos) => *pos,
        })
    }
}

impl SMesh {
    pub fn translate<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        translation: Vec3,
    ) -> SMeshResult<()> {
        let vertices = selection.into().resolve_to_vertices(self)?;
        for id in vertices {
            if let Some(pos) = self.positions.get(id) {
                self.positions.insert(id, *pos + translation);
            }
        }
        Ok(())
    }

    pub fn scale<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        scale: Vec3,
        pivot: Pivot,
    ) -> SMeshResult<()> {
        let s: MeshSelection = selection.into();
        let p = pivot.calculate(self, s.clone())?;
        self.scale_around(s, scale, p)?;
        Ok(())
    }

    pub fn scale_around<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        scale: Vec3,
        pivot: Vec3,
    ) -> SMeshResult<()> {
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
        Ok(())
    }

    pub fn rotate<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        quaternion: Quat,
        pivot: Pivot,
    ) -> SMeshResult<()> {
        let s: MeshSelection = selection.into();
        let p = pivot.calculate(self, s.clone())?;
        self.rotate_around(s, quaternion, p)?;
        Ok(())
    }

    pub fn rotate_around<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        quaternion: Quat,
        pivot: Vec3,
    ) -> SMeshResult<()> {
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
        Ok(())
    }

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
}
