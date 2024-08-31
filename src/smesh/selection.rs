use std::collections::HashSet;

use crate::{
    bail,
    prelude::{FaceId, HalfedgeId, SMesh, SMeshError, SMeshResult, VertexId},
};

#[derive(Debug, Clone)]
pub struct Selection<'a, T> {
    smesh: &'a SMesh,
    elements: HashSet<T>,
}

impl<'a, T> Selection<'a, T>
where
    T: Clone,
    Selection<'a, T>: SelectionOps<T>,
{
    pub fn new(smesh: &'a SMesh) -> Self {
        Self {
            smesh,
            elements: HashSet::new(),
        }
    }

    pub fn elements(&self) -> &HashSet<T> {
        &self.elements
    }

    pub fn add_all(&mut self, ids: Vec<T>) -> SMeshResult<()> {
        for id in ids.iter() {
            self.add(id.clone())?;
        }
        Ok(())
    }
}

pub trait SelectionOps<T> {
    fn add(&mut self, id: T) -> SMeshResult<()>;
}

impl SelectionOps<VertexId> for Selection<'_, VertexId> {
    fn add(&mut self, id: VertexId) -> SMeshResult<()> {
        if !self.smesh.vertices().contains_key(id) {
            bail!(VertexNotFound, id);
        }
        self.elements.insert(id);
        Ok(())
    }
}

impl SelectionOps<HalfedgeId> for Selection<'_, HalfedgeId> {
    fn add(&mut self, id: HalfedgeId) -> SMeshResult<()> {
        if !self.smesh.halfedges().contains_key(id) {
            bail!(HalfedgeNotFound, id);
        }
        self.elements.insert(id);
        Ok(())
    }
}

impl<'a> Selection<'a, HalfedgeId> {
    pub fn to_vertex_selection(self) -> Selection<'a, VertexId> {
        Selection {
            smesh: self.smesh,
            elements: HashSet::new(),
        }
    }
}

impl SelectionOps<FaceId> for Selection<'_, FaceId> {
    fn add(&mut self, id: FaceId) -> SMeshResult<()> {
        if !self.smesh.faces().contains_key(id) {
            bail!(FaceNotFound, id);
        }
        self.elements.insert(id);
        Ok(())
    }
}
