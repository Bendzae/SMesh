use crate::bail;
use crate::prelude::{SMesh, SMeshError, SMeshResult};
use crate::smesh::{Connectivity, FaceId, HalfedgeId, VertexId};
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

    fn evaluate_operations(&self, mesh: &SMesh) -> SMeshResult<QueryParam> {
        let c = &mesh.connectivity;
        let mut value = self.initial;
        for op in &self.history {
            value = match value {
                QueryParam::Vertex(id) => eval_vertex_op(mesh, id, *op)?,
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

        impl MeshQueryBuilder<$type> {
            pub fn run(self, mesh: &SMesh) -> SMeshResult<$type> {
                match self.evaluate_operations(mesh)? {
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

impl_id_extensions_for!(
    VertexId,
    pub trait VertexOps {
        fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId>;
        fn halfedge_to(&self, dst_vertex: VertexId) -> MeshQueryBuilder<HalfedgeId>;
        fn is_boundary(&self, mesh: &SMesh) -> bool;
        fn is_isolated(&self, mesh: &SMesh) -> bool;
        fn valence(self, mesh: &SMesh) -> usize;
        fn is_manifold(&self, mesh: &SMesh) -> bool;
    }
);
impl VertexOps for MeshQueryBuilder<VertexId> {
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::Halfedge)
    }

    fn halfedge_to(&self, dst_vertex: VertexId) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::HalfedgeTo(dst_vertex))
    }

    fn is_boundary(&self, mesh: &SMesh) -> bool {
        self.halfedge().face().run(mesh).is_err()
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
            .filter(|he| (*he).q().is_boundary(mesh))
            .count();
        n < 2
    }
}
impl_id_extensions_for!(
    HalfedgeId,
    pub trait HalfedgeOps {
        fn vert(&self) -> MeshQueryBuilder<VertexId>;
        fn opposite(&self) -> MeshQueryBuilder<HalfedgeId>;
        fn next(&self) -> MeshQueryBuilder<HalfedgeId>;
        fn prev(&self) -> MeshQueryBuilder<HalfedgeId>;
        fn face(&self) -> MeshQueryBuilder<HalfedgeId>;
        fn ccw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId>;
        fn cw_rotated_neighbour(&self) -> MeshQueryBuilder<HalfedgeId>;
        fn src_vert(&self) -> MeshQueryBuilder<VertexId>;
        fn dst_vert(&self) -> MeshQueryBuilder<VertexId>;
        fn is_boundary(&self, mesh: &SMesh) -> bool;
    }
);
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
    fn face(&self) -> MeshQueryBuilder<HalfedgeId> {
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
}
impl_id_extensions_for!(
    FaceId,
    pub trait FaceOps {
        fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId>;
        fn valence(self, mesh: &SMesh) -> usize;
    }
);
impl FaceOps for MeshQueryBuilder<FaceId> {
    fn halfedge(&self) -> MeshQueryBuilder<HalfedgeId> {
        self.push(QueryOp::Halfedge)
    }

    fn valence(self, mesh: &SMesh) -> usize {
        self.vertices(mesh).count()
    }
}

fn eval_vertex_op(mesh: &SMesh, id: VertexId, op: QueryOp) -> SMeshResult<QueryParam> {
    let c = &mesh.connectivity;
    let Some(v) = c.vertices.get(id) else {
        bail!(VertexNotFound, id);
    };
    let r = match op {
        QueryOp::Halfedge => {
            QueryParam::Halfedge(v.halfedge.ok_or(SMeshError::VertexHasNoHalfEdge(id))?)
        }
        QueryOp::HalfedgeTo(dst_vertex) => {
            let initial_he = id.q().halfedge().run(mesh)?;
            let mut he = initial_he;

            let r = loop {
                match he.q().dst_vert().run(mesh) {
                    Ok(id) => {
                        if id == dst_vertex {
                            break Ok(he);
                        }
                        he = he.q().cw_rotated_neighbour().run(mesh)?;
                        if he == initial_he {
                            break Err(SMeshError::DefaultError);
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
            QueryParam::Halfedge(h.opposite.ok_or(SMeshError::HalfedgeHasNoRef(id))?)
        }
        QueryOp::Next => QueryParam::Halfedge(h.next.ok_or(SMeshError::HalfedgeHasNoRef(id))?),
        QueryOp::Previous => QueryParam::Halfedge(h.prev.ok_or(SMeshError::HalfedgeHasNoRef(id))?),
        QueryOp::Face => QueryParam::Face(h.face.ok_or(SMeshError::HalfedgeHasNoRef(id))?),
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
    use crate::prelude::EvalMeshQuery;
    use glam::vec3;

    #[test]
    fn basic() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));

        let face_id = mesh.add_face(vec![v0, v1, v2, v3]);

        assert!(face_id.is_ok());

        let q = v0.q().halfedge();

        let h = q.opposite().run(mesh)?;
        let h1 = q.vert().run(mesh)?;
        let h_old = mesh.q(v0).halfedge().opposite().id()?;
        let h_1_old = mesh.q(v0).halfedge().vert().id()?;

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

        let face_id = mesh.add_face(vec![v0, v1, v2, v3]);

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

        let face_id = mesh.add_face(vec![v0, v1, v2, v3])?;

        assert_eq!(face_id.q().valence(mesh), 4);
        assert_eq!(v0.q().valence(mesh), 2);

        Ok(())
    }

    #[test]
    fn manifoldness() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));

        mesh.add_face(vec![v0, v1, v2, v3])?;

        let n = v0
            .q()
            .halfedges(mesh)
            // .filter(|he| (*he).q().is_boundary(mesh))
            .count();

        println!("{}", n);

        let n = mesh
            .q(v0)
            .halfedges()
            // .filter(|he| (*he).q().is_boundary(mesh))
            .count();

        println!("{}", n);
        assert!(v0.q().is_manifold(mesh));
        assert!(v3.q().is_manifold(mesh));

        Ok(())
    }
}
