use core::f32;

use glam::{i32, Vec2, Vec3};
use slotmap::SecondaryMap;

use crate::{bail, prelude::*};

#[derive(Debug, Clone)]
pub enum MeshAttribute {
    Integer(i32),
    Float(f32),
    Vec2(Vec2),
    Vec3(Vec3),
    String(String),
}

impl From<i32> for MeshAttribute {
    fn from(value: i32) -> Self {
        Self::Integer(value)
    }
}
impl From<f32> for MeshAttribute {
    fn from(value: f32) -> Self {
        Self::Float(value)
    }
}

impl From<Vec2> for MeshAttribute {
    fn from(value: Vec2) -> Self {
        Self::Vec2(value)
    }
}

impl From<Vec3> for MeshAttribute {
    fn from(value: Vec3) -> Self {
        Self::Vec3(value)
    }
}

impl From<String> for MeshAttribute {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl TryFrom<MeshAttribute> for i32 {
    type Error = SMeshError;

    fn try_from(value: MeshAttribute) -> Result<Self, Self::Error> {
        match value {
            MeshAttribute::Integer(val) => Ok(val),
            _ => bail!(DefaultError),
        }
    }
}

impl TryFrom<MeshAttribute> for f32 {
    type Error = SMeshError;

    fn try_from(value: MeshAttribute) -> Result<Self, Self::Error> {
        match value {
            MeshAttribute::Float(val) => Ok(val),
            _ => bail!(DefaultError),
        }
    }
}

impl TryFrom<MeshAttribute> for Vec2 {
    type Error = SMeshError;

    fn try_from(value: MeshAttribute) -> Result<Self, Self::Error> {
        match value {
            MeshAttribute::Vec2(val) => Ok(val),
            _ => bail!(DefaultError),
        }
    }
}

impl TryFrom<MeshAttribute> for Vec3 {
    type Error = SMeshError;

    fn try_from(value: MeshAttribute) -> Result<Self, Self::Error> {
        match value {
            MeshAttribute::Vec3(val) => Ok(val),
            _ => bail!(DefaultError),
        }
    }
}

impl TryFrom<MeshAttribute> for String {
    type Error = SMeshError;

    fn try_from(value: MeshAttribute) -> Result<Self, Self::Error> {
        match value {
            MeshAttribute::String(val) => Ok(val),
            _ => bail!(DefaultError),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CustomAttributeMap<T: slotmap::Key> {
    inner_map: SecondaryMap<T, MeshAttribute>,
}

pub trait CustomAttributeMapOps<K: slotmap::Key, V>
where
    V: TryFrom<MeshAttribute>,
    MeshAttribute: From<V>,
{
    fn get(&self, key: K) -> Option<V>;
    fn insert(&mut self, key: K, value: V) -> Option<V>;
}

impl<K: slotmap::Key, V> CustomAttributeMapOps<K, V> for CustomAttributeMap<K>
where
    V: TryFrom<MeshAttribute>,
    MeshAttribute: From<V>,
{
    fn get(&self, key: K) -> Option<V> {
        self.inner_map.get(key)?.clone().try_into().ok()
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner_map
            .insert(key, value.into())?
            .clone()
            .try_into()
            .ok()
    }
}

trait CustomAttributeOps<K: slotmap::Key> {
    fn attribute_internal(&self, key: &str) -> Option<&CustomAttributeMap<K>>;
    fn attribute_mut_internal(&mut self, key: &str) -> Option<&mut CustomAttributeMap<K>>;
    fn add_attribute_map_internal(&mut self, key: &str) -> Option<&mut CustomAttributeMap<K>>;
}

impl CustomAttributeOps<VertexId> for SMesh {
    fn attribute_internal(&self, key: &str) -> Option<&CustomAttributeMap<VertexId>> {
        self.vertex_attributes.get(key)
    }
    fn attribute_mut_internal(&mut self, key: &str) -> Option<&mut CustomAttributeMap<VertexId>> {
        self.vertex_attributes.get_mut(key)
    }
    fn add_attribute_map_internal(
        &mut self,
        key: &str,
    ) -> Option<&mut CustomAttributeMap<VertexId>> {
        self.vertex_attributes
            .insert(key.to_string(), CustomAttributeMap::default());
        self.attribute_mut_internal(key)
    }
}

impl CustomAttributeOps<HalfedgeId> for SMesh {
    fn attribute_internal(&self, key: &str) -> Option<&CustomAttributeMap<HalfedgeId>> {
        self.edge_attributes.get(key)
    }
    fn attribute_mut_internal(&mut self, key: &str) -> Option<&mut CustomAttributeMap<HalfedgeId>> {
        self.edge_attributes.get_mut(key)
    }
    fn add_attribute_map_internal(
        &mut self,
        key: &str,
    ) -> Option<&mut CustomAttributeMap<HalfedgeId>> {
        self.edge_attributes
            .insert(key.to_string(), CustomAttributeMap::default());
        self.attribute_mut_internal(key)
    }
}

impl CustomAttributeOps<FaceId> for SMesh {
    fn attribute_internal(&self, key: &str) -> Option<&CustomAttributeMap<FaceId>> {
        self.face_attributes.get(key)
    }
    fn attribute_mut_internal(&mut self, key: &str) -> Option<&mut CustomAttributeMap<FaceId>> {
        self.face_attributes.get_mut(key)
    }
    fn add_attribute_map_internal(&mut self, key: &str) -> Option<&mut CustomAttributeMap<FaceId>> {
        self.face_attributes
            .insert(key.to_string(), CustomAttributeMap::default());
        self.attribute_mut_internal(key)
    }
}

impl SMesh {
    pub fn attribute<K: slotmap::Key>(&self, key: &str) -> Option<&CustomAttributeMap<K>>
    where
        Self: CustomAttributeOps<K>,
    {
        self.attribute_internal(key)
    }

    pub fn attribute_mut<K: slotmap::Key>(
        &mut self,
        key: &str,
    ) -> Option<&mut CustomAttributeMap<K>>
    where
        Self: CustomAttributeOps<K>,
    {
        self.attribute_mut_internal(key)
    }

    pub fn add_attribute_map<K: slotmap::Key>(
        &mut self,
        key: &str,
    ) -> Option<&mut CustomAttributeMap<K>>
    where
        Self: CustomAttributeOps<K>,
    {
        self.add_attribute_map_internal(key)
    }
}

#[cfg(test)]
mod test {
    use attribute::*;
    use glam::vec3;

    use crate::prelude::*;

    #[test]
    fn basic_integer() {
        let mut smesh = SMesh::new();
        let v0 = smesh.add_vertex(vec3(1.0, 1.0, 1.0));
        let curvature = smesh.add_attribute_map::<VertexId>("curvature").unwrap();
        curvature.insert(v0, 2);
        assert_eq!(curvature.get(v0), Some(2));
    }

    #[test]
    fn basic_float() {
        let mut smesh = SMesh::new();
        let v0 = smesh.add_vertex(vec3(1.0, 1.0, 1.0));
        let curvature = smesh.add_attribute_map::<VertexId>("curvature").unwrap();
        curvature.insert(v0, 2.1);
        assert_eq!(curvature.get(v0), Some(2.1));
    }

    #[test]
    fn basic_vec2() {
        let mut smesh = SMesh::new();
        let v0 = smesh.add_vertex(vec3(1.0, 1.0, 1.0));
        let curvature = smesh.add_attribute_map::<VertexId>("curvature").unwrap();
        curvature.insert(v0, glam::vec2(1.0, 1.0));
        assert_eq!(curvature.get(v0), Some(glam::vec2(1.0, 1.0)));
    }

    #[test]
    fn basic_vec3() {
        let mut smesh = SMesh::new();
        let v0 = smesh.add_vertex(vec3(1.0, 1.0, 1.0));
        let curvature = smesh.add_attribute_map::<VertexId>("curvature").unwrap();
        curvature.insert(v0, glam::vec3(1.0, 1.0, 1.0));
        assert_eq!(curvature.get(v0), Some(glam::vec3(1.0, 1.0, 1.0)));
    }

    #[test]
    fn basic_string() {
        let mut smesh = SMesh::new();
        let v0 = smesh.add_vertex(vec3(1.0, 1.0, 1.0));
        let curvature = smesh.add_attribute_map::<VertexId>("curvature").unwrap();
        curvature.insert(v0, "hello".to_string());
        assert_eq!(curvature.get(v0), Some("hello".to_string()));
    }
}
