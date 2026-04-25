//! Chainable DSL for navigating mesh connectivity.
//!
//! Every element id implements `.q()` via [`ToMeshQueryBuilder`], which begins
//! a lazy query. You then chain verbs that step through the halfedge
//! structure, and finish by `.run(&mesh)` (returning an id) or by calling an
//! eager helper like `.position(&mesh)`, `.valence(&mesh)`, or `.normal(&mesh)`.
//!
//! For convenience, the same verbs are also available as inherent methods on
//! the raw ids — `v.halfedge()` is equivalent to `v.q().halfedge()`.
//!
//! ```
//! # use glam::vec3;
//! # use smesh::prelude::*;
//! # let mut mesh = SMesh::new();
//! # let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
//! # let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
//! # let v2 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
//! # mesh.make_triangle(v0, v1, v2).unwrap();
//! // Walk: v0 → its outgoing halfedge → that halfedge's destination vertex
//! let dst: VertexId = v0.halfedge().vert().run(&mesh).unwrap();
//!
//! // Ask for the halfedge that connects v0 to v1 directly
//! let he = v0.halfedge_to(v1).run(&mesh).unwrap();
//! assert_eq!(he.src_vert().run(&mesh).unwrap(), v0);
//! ```
//!
//! ### Verbs by element
//!
//! | From a `VertexId` | From a `HalfedgeId` | From a `FaceId` |
//! |-------------------|---------------------|-----------------|
//! | `halfedge()` → HE | `vert()`, `src_vert()`, `dst_vert()` → Vert | `halfedge()` → HE |
//! | `halfedge_to(v)` → HE | `opposite()`, `next()`, `prev()` → HE | |
//! |                       | `face()` → Face                       | |
//! |                       | `cw_rotated_neighbour()`, `ccw_rotated_neighbour()` | |

use glam::Vec2;
use glam::Vec3;

use crate::bail;
use crate::prelude::*;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq)]
enum QueryParam {
    Vertex(VertexId),
    Halfedge(HalfedgeId),
    Face(FaceId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum QueryOp {
    // Basic
    Vertex,
    Halfedge,
    Opposite,
    Next,
    Previous,
    Face,
    // Complex
    HalfedgeTo(VertexId),
}

/// A lazy sequence of navigation steps, parameterised by the expected result
/// type (one of `VertexId`, `HalfedgeId`, `FaceId`).
///
/// Built by calling `.q()` on an element id (see [`ToMeshQueryBuilder`]) or
/// by chaining verbs such as `.halfedge()`, `.next()`, `.face()`.
/// Evaluated by calling [`RunQuery::run`] against an [`SMesh`] or a
/// [`Connectivity`].
#[derive(Debug, Clone, PartialEq)]
pub struct MeshQueryBuilder<T> {
    initial: QueryParam,
    history: Vec<QueryOp>,
    phantom_data: PhantomData<T>,
}

impl<T> MeshQueryBuilder<T> {
    fn push<E>(&self, op: QueryOp) -> MeshQueryBuilder<E> {
        let mut history = self.history.clone();
        history.push(op);
        MeshQueryBuilder {
            initial: self.initial,
            history,
            phantom_data: PhantomData,
        }
    }

    fn evaluate_operations(&self, c: &Connectivity) -> SMeshResult<QueryParam> {
        let mut value = self.initial;
        for op in &self.history {
            value = match value {
                QueryParam::Vertex(id) => eval_vertex_op(c, id, *op)?,
                QueryParam::Halfedge(id) => eval_halfedge_op(c, id, *op)?,
                QueryParam::Face(id) => eval_face_op(c, id, *op)?,
            };
        }
        Ok(value)
    }
}

/// Seed a [`MeshQueryBuilder`] from an element id.
///
/// `v.q()` produces an empty builder targeting the same element; typically
/// you immediately chain a verb (`v.q().halfedge().next()`). In practice the
/// inherent-method equivalents on the id types are shorter and read the same.
pub trait ToMeshQueryBuilder<T> {
    /// Start a query rooted at this element.
    fn q(&self) -> MeshQueryBuilder<T>;
}

/// Evaluate a [`MeshQueryBuilder`] against a mesh (or raw [`Connectivity`]).
///
/// Returns the element id reached by walking the chain, or an
/// [`SMeshError`] if any step encounters missing connectivity (stale id,
/// boundary halfedge asked for its face, etc.).
pub trait RunQuery<T, E> {
    /// Evaluate the query against `on` and return the resulting id.
    fn run(self, on: &E) -> SMeshResult<T>;
}

macro_rules! impl_mesh_query_for {
    ($type:ident, $enum_variant:ident) => {
        impl ToMeshQueryBuilder<$type> for $type {
            fn q(&self) -> MeshQueryBuilder<$type> {
                MeshQueryBuilder {
                    initial: QueryParam::$enum_variant(self.clone()),
                    history: vec![],
                    phantom_data: PhantomData,
                }
            }
        }

        impl RunQuery<$type, SMesh> for MeshQueryBuilder<$type> {
            fn run(self, mesh: &SMesh) -> SMeshResult<$type> {
                match self.evaluate_operations(&mesh.connectivity)? {
                    QueryParam::$enum_variant(id) => Ok(id),
                    _ => Err(SMeshError::DefaultError),
                }
            }
        }

        impl RunQuery<$type, Connectivity> for MeshQueryBuilder<$type> {
            fn run(self, connectivity: &Connectivity) -> SMeshResult<$type> {
                match self.evaluate_operations(connectivity)? {
                    QueryParam::$enum_variant(id) => Ok(id),
                    _ => Err(SMeshError::DefaultError),
                }
            }
        }
    };
}

impl_mesh_query_for!(VertexId, Vertex);
impl_mesh_query_for!(HalfedgeId, Halfedge);
impl_mesh_query_for!(FaceId, Face);

// TODO Doesnt work because of recursion, would be cool in the future
#[macro_export]
macro_rules! impl_id_extensions_for {
    ($type:ident, pub trait $trait_name:ident { $( fn $fn_name:ident($(&$self1:ident)? $($self2:ident)?  $(, $arg_name:ident : $arg_ty:ty )*) -> $ret:ty );*; }) => {
        pub trait $trait_name {
            $(
                fn $fn_name($(&$self1)?$($self2)?$(, $arg_name: $arg_ty, )*) -> $ret;
            )*
        }

        impl $trait_name for $type {
            $(
                fn $fn_name($(&$self1)?$($self2)?$(, $arg_name: $arg_ty, )*) -> $ret {
                    $($self1)?$($self2)?.$fn_name($($arg_name)*)
                }
            )*
        }
    };
}

/// Verbs that start at a vertex.
///
/// Implemented for both [`VertexId`] (eager, one-step inherent methods) and
/// [`MeshQueryBuilder<VertexId>`](MeshQueryBuilder) (chainable lazy form).
/// Navigation methods return a new builder; evaluation methods
/// (`is_boundary`, `position`, ...) take a `&SMesh`.
pub trait VertexOps {
    /// Arbitrary outgoing halfedge of this vertex (boundary-preferred on
    /// boundary vertices).
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId>;
    /// Halfedge going *from* this vertex *to* `dst_vertex`, if one exists.
    /// Fails with a custom error if no such edge is connected.
    fn halfedge_to(&self, dst_vertex: VertexId) -> MeshQueryBuilder<HalfedgeId>;
    /// `true` if any outgoing halfedge is on the mesh boundary.
    fn is_boundary(&self, mesh: &SMesh) -> bool;
    /// `true` if the vertex has no outgoing halfedge (never part of any face).
    fn is_isolated(&self, mesh: &SMesh) -> bool;
    /// Number of adjacent vertices (the vertex's "degree").
    fn valence(self, mesh: &SMesh) -> usize;
    /// `true` if this vertex has at most one boundary gap — i.e. the one-ring
    /// around it is topologically a disc or a half-disc.
    fn is_manifold(&self, mesh: &SMesh) -> bool;
    /// World-space position (`mesh.positions[v]`).
    fn position(self, mesh: &SMesh) -> SMeshResult<Vec3>;
    /// Vertex normal (populated by
    /// [`recalculate_normals`](SMesh::recalculate_normals)).
    fn normal(self, mesh: &SMesh) -> SMeshResult<Vec3>;
    /// Per-vertex UV (requires [`vertex_uvs`](SMesh::vertex_uvs)); for UVs
    /// with seams, use the halfedge variant via
    /// [`HalfedgeOps::uv`].
    fn uv(self, mesh: &SMesh) -> SMeshResult<Vec2>;
}
impl VertexOps for MeshQueryBuilder<VertexId> {
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::Halfedge)
    }

    fn halfedge_to(&self, dst_vertex: VertexId) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::HalfedgeTo(dst_vertex))
    }

    fn is_boundary(&self, mesh: &SMesh) -> bool {
        for he in self.clone().halfedges(mesh) {
            if he.is_boundary(mesh) {
                return true;
            }
        }
        false
    }

    fn is_isolated(&self, mesh: &SMesh) -> bool {
        self.halfedge().run(mesh).is_err()
    }

    fn valence(self, mesh: &SMesh) -> usize {
        self.vertices(mesh).count()
    }

    // The vertex is non-manifold if more than one gap exists, i.e.
    // more than one outgoing boundary halfedge.
    fn is_manifold(&self, mesh: &SMesh) -> bool {
        let n = self
            .clone()
            .halfedges(mesh)
            .filter(|he| (*he).is_boundary(mesh))
            .count();
        n < 2
    }

    fn position(self, mesh: &SMesh) -> SMeshResult<Vec3> {
        let v = self.run(mesh)?;
        v.position(mesh)
    }

    fn normal(self, mesh: &SMesh) -> SMeshResult<Vec3> {
        let v = self.run(mesh)?;
        v.normal(mesh)
    }

    fn uv(self, mesh: &SMesh) -> SMeshResult<Vec2> {
        let v = self.run(mesh)?;
        v.uv(mesh)
    }
}

impl VertexOps for VertexId {
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.q().halfedge()
    }

    fn halfedge_to(&self, dst_vertex: VertexId) -> MeshQueryBuilder<HalfedgeId> {
        self.q().halfedge_to(dst_vertex)
    }

    fn is_boundary(&self, mesh: &SMesh) -> bool {
        // TODO: fix
        self.q().is_boundary(mesh)
    }

    fn is_isolated(&self, mesh: &SMesh) -> bool {
        self.q().is_isolated(mesh)
    }

    fn valence(self, mesh: &SMesh) -> usize {
        self.q().valence(mesh)
    }

    fn is_manifold(&self, mesh: &SMesh) -> bool {
        self.q().is_manifold(mesh)
    }

    fn position(self, mesh: &SMesh) -> SMeshResult<Vec3> {
        mesh.positions
            .get(self)
            .copied()
            .ok_or(SMeshError::CustomError("Vertex has no position attribute"))
    }

    fn normal(self, mesh: &SMesh) -> SMeshResult<Vec3> {
        if let Some(vertex_normals) = &mesh.vertex_normals {
            return vertex_normals
                .get(self)
                .copied()
                .ok_or(SMeshError::CustomError("Vertex has no normal attribute"));
        }
        bail!("No attribute map for normals exists")
    }

    fn uv(self, mesh: &SMesh) -> SMeshResult<Vec2> {
        if let Some(uvs) = &mesh.vertex_uvs {
            return uvs
                .get(self)
                .copied()
                .ok_or(SMeshError::CustomError("Vertex has no uv attribute"));
        }
        bail!("No attribute map for uvs exists");
    }
}

/// Verbs that start at a halfedge.
///
/// A halfedge is directed and points *at* its destination vertex: `he.vert()`
/// and `he.dst_vert()` both give the destination, while `he.src_vert()` gives
/// the source (equivalent to `he.opposite().vert()`).
pub trait HalfedgeOps {
    /// Destination vertex (alias of [`dst_vert`](Self::dst_vert)).
    fn vert(&self) -> MeshQueryBuilder<VertexId>;
    /// Paired halfedge in the opposite direction (same edge, reversed).
    fn opposite(&self) -> MeshQueryBuilder<HalfedgeId>;
    /// Next halfedge around the same face loop (CCW on outer faces).
    fn next(&self) -> MeshQueryBuilder<HalfedgeId>;
    /// Previous halfedge around the same face loop.
    fn prev(&self) -> MeshQueryBuilder<HalfedgeId>;
    /// The face this halfedge bounds. Fails on boundary halfedges — check
    /// [`is_boundary`](Self::is_boundary) first if unsure.
    fn face(&self) -> MeshQueryBuilder<FaceId>;
    /// Step to the next outgoing halfedge of the *source* vertex, rotating
    /// counter-clockwise. Equivalent to `self.prev().opposite()`.
    fn ccw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId>;
    /// Step to the next outgoing halfedge of the *source* vertex, rotating
    /// clockwise. Equivalent to `self.opposite().next()`.
    fn cw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId>;
    /// Source vertex (opposite of [`dst_vert`](Self::dst_vert)).
    fn src_vert(&self) -> MeshQueryBuilder<VertexId>;
    /// Destination vertex the halfedge points at.
    fn dst_vert(&self) -> MeshQueryBuilder<VertexId>;
    /// `true` if this halfedge lies on the mesh boundary (no face on this side).
    fn is_boundary(&self, mesh: &SMesh) -> bool;
    /// Boundary check against a bare [`Connectivity`] — useful inside code
    /// paths that cannot borrow the full [`SMesh`].
    fn is_boundary_c(&self, connectivity: &Connectivity) -> bool;
    /// Per-halfedge UV (from [`halfedge_uvs`](SMesh::halfedge_uvs)).
    fn uv(self, mesh: &SMesh) -> SMeshResult<Vec2>;
}
impl HalfedgeOps for MeshQueryBuilder<HalfedgeId> {
    fn vert(&self) -> MeshQueryBuilder<VertexId> {
        self.push(QueryOp::Vertex)
    }
    fn opposite(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::Opposite)
    }
    fn next(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::Next)
    }
    fn prev(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::Previous)
    }
    fn face(&self) -> MeshQueryBuilder<FaceId> {
        self.push(QueryOp::Face)
    }
    fn ccw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.prev().opposite()
    }
    fn cw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.opposite().next()
    }
    fn src_vert(&self) -> MeshQueryBuilder<VertexId> {
        self.opposite().vert()
    }
    fn dst_vert(&self) -> MeshQueryBuilder<VertexId> {
        self.vert()
    }
    fn is_boundary(&self, mesh: &SMesh) -> bool {
        self.face().run(mesh).is_err()
    }

    // TODO: temp wortkaround
    fn is_boundary_c(&self, connectivity: &Connectivity) -> bool {
        self.face().run(connectivity).is_err()
    }

    fn uv(self, mesh: &SMesh) -> SMeshResult<Vec2> {
        let v = self.run(mesh)?;
        v.uv(mesh)
    }
}

impl HalfedgeOps for HalfedgeId {
    fn vert(&self) -> MeshQueryBuilder<VertexId> {
        self.q().vert()
    }

    fn opposite(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.q().opposite()
    }

    fn next(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.q().next()
    }

    fn prev(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.q().prev()
    }

    fn face(&self) -> MeshQueryBuilder<FaceId> {
        self.q().face()
    }

    fn ccw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.q().ccw_rotated_neighbour()
    }

    fn cw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.q().cw_rotated_neighbour()
    }

    fn src_vert(&self) -> MeshQueryBuilder<VertexId> {
        self.q().src_vert()
    }

    fn dst_vert(&self) -> MeshQueryBuilder<VertexId> {
        self.q().dst_vert()
    }

    fn is_boundary(&self, mesh: &SMesh) -> bool {
        self.q().is_boundary(mesh)
    }

    fn is_boundary_c(&self, connectivity: &Connectivity) -> bool {
        self.q().is_boundary_c(connectivity)
    }

    fn uv(self, mesh: &SMesh) -> SMeshResult<Vec2> {
        if let Some(uvs) = &mesh.halfedge_uvs {
            return uvs
                .get(self)
                .copied()
                .ok_or(SMeshError::CustomError("Halfedge has no uv attribute"));
        }
        bail!("No attribute map for uvs exists");
    }
}
/// Verbs that start at a face.
pub trait FaceOps {
    /// Arbitrary halfedge on the face boundary.
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId>;
    /// Number of vertices (or equivalently, edges) bounding this face.
    /// `3` for a triangle, `4` for a quad, etc.
    fn valence(self, mesh: &SMesh) -> usize;
    /// Face normal (populated by
    /// [`recalculate_normals`](SMesh::recalculate_normals)).
    fn normal(self, mesh: &SMesh) -> SMeshResult<Vec3>;
}
impl FaceOps for MeshQueryBuilder<FaceId> {
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::Halfedge)
    }

    fn valence(self, mesh: &SMesh) -> usize {
        self.vertices(mesh).count()
    }

    fn normal(self, mesh: &SMesh) -> SMeshResult<Vec3> {
        self.run(mesh)?.normal(mesh)
    }
}

impl FaceOps for FaceId {
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.q().halfedge()
    }

    fn valence(self, mesh: &SMesh) -> usize {
        self.q().valence(mesh)
    }

    fn normal(self, mesh: &SMesh) -> SMeshResult<Vec3> {
        if let Some(face_normals) = &mesh.face_normals {
            return face_normals
                .get(self)
                .copied()
                .ok_or(SMeshError::CustomError("Face has no normal attribute"));
        }
        bail!("No attribute map for face normals exists")
    }
}

fn eval_vertex_op(c: &Connectivity, id: VertexId, op: QueryOp) -> SMeshResult<QueryParam> {
    let Some(v) = c.vertices.get(id) else {
        bail!(VertexNotFound, id);
    };
    let r = match op {
        QueryOp::Halfedge => {
            QueryParam::Halfedge(v.halfedge.ok_or(SMeshError::VertexHasNoHalfEdge(id))?)
        }
        QueryOp::HalfedgeTo(dst_vertex) => {
            let initial_he = id.halfedge().run(c)?;
            let mut he = initial_he;

            if id == dst_vertex {
                bail!("HalfedgeTo: Inital and dst vertex are the same");
            }

            let r = loop {
                match he.dst_vert().run(c) {
                    Ok(id) => {
                        if id == dst_vertex {
                            break Ok(he);
                        }
                        he = he.cw_rotated_neighbour().run(c)?;
                        if he == initial_he {
                            bail!("HalfedgeTo: No connecting halfedge found");
                        }
                    }
                    Err(e) => {
                        break Err(e);
                    }
                }
            };
            QueryParam::Halfedge(r?)
        }
        _ => bail!(UnsupportedOperation),
    };
    Ok(r)
}

fn eval_halfedge_op(c: &Connectivity, id: HalfedgeId, op: QueryOp) -> SMeshResult<QueryParam> {
    let Some(h) = c.halfedges.get(id) else {
        bail!(HalfedgeNotFound, id);
    };
    let r = match op {
        QueryOp::Vertex => QueryParam::Vertex(h.vertex),
        QueryOp::Opposite => {
            QueryParam::Halfedge(h.opposite.ok_or(SMeshError::HalfedgeHasNoOpposite(id))?)
        }
        QueryOp::Next => QueryParam::Halfedge(h.next.ok_or(SMeshError::HalfedgeHasNoNext(id))?),
        QueryOp::Previous => QueryParam::Halfedge(h.prev.ok_or(SMeshError::HalfedgeHasNoPrev(id))?),
        QueryOp::Face => QueryParam::Face(h.face.ok_or(SMeshError::HalfedgeHasNoFace(id))?),
        _ => bail!(UnsupportedOperation),
    };
    Ok(r)
}

fn eval_face_op(c: &Connectivity, id: FaceId, op: QueryOp) -> SMeshResult<QueryParam> {
    let Some(f) = c.faces.get(id) else {
        bail!(FaceNotFound, id);
    };
    let r = match op {
        QueryOp::Halfedge => {
            QueryParam::Halfedge(f.halfedge.ok_or(SMeshError::FaceHasNoHalfEdge(id))?)
        }
        _ => bail!(UnsupportedOperation),
    };
    Ok(r)
}

#[cfg(test)]
mod test {
    use super::*;
    use glam::vec3;

    #[test]
    fn basic() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));

        let face_id = mesh.make_face(vec![v0, v1, v2, v3]);

        assert!(face_id.is_ok());

        let q = v0.halfedge();

        let h = q.opposite().run(mesh)?;
        let h1 = q.vert().run(mesh)?;
        let h_old = v0.halfedge().opposite().run(mesh)?;
        let h_1_old = v0.halfedge().vert().run(mesh)?;

        assert_eq!(h, h_old);
        assert_eq!(h1, h_1_old);

        Ok(())
    }
    #[test]
    fn halfedge_to() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));

        let face_id = mesh.make_face(vec![v0, v1, v2, v3]);

        assert!(face_id.is_ok());

        let he_0_to_1 = v0.halfedge_to(v1);
        assert_eq!(he_0_to_1.src_vert().run(mesh)?, v0);
        assert_eq!(he_0_to_1.dst_vert().run(mesh)?, v1);

        Ok(())
    }

    #[test]
    fn valence() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));

        let face_id = mesh.make_face(vec![v0, v1, v2, v3])?;

        assert_eq!(face_id.valence(mesh), 4);
        assert_eq!(v0.valence(mesh), 2);

        Ok(())
    }

    #[test]
    fn manifoldness() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));

        mesh.make_face(vec![v0, v1, v2, v3])?;

        assert!(v0.is_manifold(mesh));
        assert!(v3.is_manifold(mesh));

        Ok(())
    }
}
