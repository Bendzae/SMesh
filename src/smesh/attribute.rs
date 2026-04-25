//! User-defined per-element attributes.
//!
//! Alongside the built-in position / normal / UV maps, every [`SMesh`]
//! carries string-keyed [`CustomAttributeMap`]s on vertices, halfedges, and
//! faces. Values are stored as [`MeshAttribute`] (an enum over a small set of
//! common types) and retrieved via the typed
//! [`CustomAttributeMapOps::get`] / [`CustomAttributeMapOps::insert`] accessors.
//!
//! ```
//! use glam::vec3;
//! use smesh::prelude::*;
//!
//! let mut mesh = SMesh::new();
//! let v = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
//!
//! // Create the map, then use CustomAttributeMapOps to write / read.
//! let map = mesh.add_attribute_map::<VertexId>("curvature").unwrap();
//! map.insert(v, 0.42_f32);
//! assert_eq!(map.get(v), Some(0.42_f32));
//! ```

use core::f32;

use glam::{i32, Vec2, Vec3};
use slotmap::SecondaryMap;

use crate::{bail, prelude::*};

/// Value stored in a [`CustomAttributeMap`].
///
/// One of a handful of common types. Conversion to/from the native Rust
/// representation is automatic when you use
/// [`CustomAttributeMapOps::insert`] / [`get`](CustomAttributeMapOps::get).
#[derive(Debug, Clone)]
pub enum MeshAttribute {
    /// 32-bit signed integer.
    Integer(i32),
    /// 32-bit float.
    Float(f32),
    /// 2D vector.
    Vec2(Vec2),
    /// 3D vector.
    Vec3(Vec3),
    /// UTF-8 string.
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

/// A named map from mesh element ids to [`MeshAttribute`] values.
///
/// Stored on the mesh under a name string — see
/// [`SMesh::attribute`](crate::prelude::SMesh::attribute) and
/// [`SMesh::add_attribute_map`](crate::prelude::SMesh::add_attribute_map).
/// Values are transparently converted to the `MeshAttribute` variant matching
/// their type, and `get`/`insert` round-trip back to the native type.
#[derive(Debug, Clone, Default)]
pub struct CustomAttributeMap<T: slotmap::Key> {
    inner_map: SecondaryMap<T, MeshAttribute>,
}

/// Typed access to a [`CustomAttributeMap`].
///
/// Implemented generically for any `V` that has a `From`/`TryFrom` pair with
/// [`MeshAttribute`] — the supported built-ins are `i32`, `f32`, `Vec2`,
/// `Vec3`, and `String`.
pub trait CustomAttributeMapOps<K: slotmap::Key, V>
where
    V: TryFrom<MeshAttribute>,
    MeshAttribute: From<V>,
{
    /// Fetch the value for `key`, converted to `V`. `None` if unset or if a
    /// different type was stored there.
    fn get(&self, key: K) -> Option<V>;
    /// Store `value` under `key`, returning the previous value if any.
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
    /// Borrow an existing attribute map by name. `None` if no such map has
    /// been created for the element type `K`.
    ///
    /// The element type `K` must be explicit — usually `VertexId`,
    /// `HalfedgeId`, or `FaceId`.
    pub fn attribute<K: slotmap::Key>(&self, key: &str) -> Option<&CustomAttributeMap<K>>
    where
        Self: CustomAttributeOps<K>,
    {
        self.attribute_internal(key)
    }

    /// Mutable counterpart to [`attribute`](Self::attribute).
    pub fn attribute_mut<K: slotmap::Key>(
        &mut self,
        key: &str,
    ) -> Option<&mut CustomAttributeMap<K>>
    where
        Self: CustomAttributeOps<K>,
    {
        self.attribute_mut_internal(key)
    }

    /// Create a new empty attribute map under `key` for element type `K` and
    /// return a mutable reference to it. Overwrites any existing map with
    /// the same name.
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
        smesh.add_attribute_map::<VertexId>("curvature").unwrap();
        smesh
            .attribute_mut("curvature")
            .unwrap()
            .insert(v0, "hello".to_string());
        assert_eq!(
            smesh.attribute("curvature").unwrap().get(v0),
            Some("hello".to_string())
        );
    }
}
