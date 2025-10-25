use std::collections::HashMap;

use glam::{Vec2, Vec3};
use itertools::Itertools;
use slotmap::{new_key_type, SecondaryMap, SlotMap};

use crate::prelude::{attribute::CustomAttributeMap, SMeshError::FaceNotFound};
use crate::smesh::error::*;
use crate::smesh::mesh_query::*;
use crate::{bail, prelude::attribute::MeshAttribute};

pub mod attribute;
pub mod edit_operations;
pub mod error;
pub mod iterators;
pub mod loops;
pub mod mesh_query;
pub mod model;
pub mod primitives;
pub mod selection;
pub mod topological_operations;
pub mod transform;
pub mod util;
pub mod uv_operations;

#[cfg(feature = "xatlas")]
pub mod xatlas_integration;
