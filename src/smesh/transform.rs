use glam::Vec3;
use selection::MeshSelection;

use crate::prelude::*;

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
}
