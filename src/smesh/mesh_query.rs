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

pub trait ToMeshQueryBuilder<T> {
    fn q(&self) -> MeshQueryBuilder<T>;
}

pub trait RunQuery<T, E> {
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

pub trait VertexOps {
    /// get outgoing halfege
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId>;
    fn halfedge_to(&self, dst_vertex: VertexId) -> MeshQueryBuilder<HalfedgeId>;
    fn is_boundary(&self, mesh: &SMesh) -> bool;
    fn is_isolated(&self, mesh: &SMesh) -> bool;
    fn valence(self, mesh: &SMesh) -> usize;
    fn is_manifold(&self, mesh: &SMesh) -> bool;
    fn position(self, mesh: &SMesh) -> SMeshResult<Vec3>;
    fn normal(self, mesh: &SMesh) -> SMeshResult<Vec3>;
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
        if let Some(uvs) = &mesh.uvs {
            return uvs
                .get(self)
                .copied()
                .ok_or(SMeshError::CustomError("Vertex has no uv attribute"));
        }
        bail!("No attribute map for uvs exists");
    }
}

pub trait HalfedgeOps {
    fn vert(&self) -> MeshQueryBuilder<VertexId>;
    fn opposite(&self) -> MeshQueryBuilder<HalfedgeId>;
    fn next(&self) -> MeshQueryBuilder<HalfedgeId>;
    fn prev(&self) -> MeshQueryBuilder<HalfedgeId>;
    fn face(&self) -> MeshQueryBuilder<FaceId>;
    fn ccw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId>;
    fn cw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId>;
    fn src_vert(&self) -> MeshQueryBuilder<VertexId>;
    fn dst_vert(&self) -> MeshQueryBuilder<VertexId>;
    fn is_boundary(&self, mesh: &SMesh) -> bool;
    // TODO: temp workaround
    fn is_boundary_c(&self, connectivity: &Connectivity) -> bool;
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
}
pub trait FaceOps {
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId>;
    fn valence(self, mesh: &SMesh) -> usize;
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
