use bevy::math::VectorSpace;
use glam::Vec3;
use selection::MeshSelection;

use crate::{bail, prelude::*};

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

    pub fn scale<S: Into<MeshSelection>>(&mut self, selection: S, scale: Vec3) -> SMeshResult<()> {
        self.scale_around(selection, scale, Vec3::ZERO)?;
        Ok(())
    }

    pub fn scale_around_cog<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        scale: Vec3,
    ) -> SMeshResult<()> {
        let s: MeshSelection = selection.into();
        let cog = self.center_of_gravity(s.clone())?;
        self.scale_around(s, scale, cog)?;
        Ok(())
    }

    pub fn scale_around<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        scale: Vec3,
        origin: Vec3,
    ) -> SMeshResult<()> {
        let vertices = selection.into().resolve_to_vertices(self)?;
        for id in vertices {
            let mut position = id.position(self)?;
            // Translate vertex so that the origin is at the desired point
            position -= origin;
            // Scale the vertex
            position *= scale;
            // Translate the vertex back
            position += origin;
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
