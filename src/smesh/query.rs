use crate::bail;
use crate::prelude::{SMesh, SMeshError, SMeshResult};
use crate::smesh::{Connectivity, FaceId, HalfedgeId, VertexId};
use std::marker::PhantomData;

/// TODO: Different approach to mesh query where the q is only evaluated when calling run()
/// Would solve some issues with borrowing and be more ergonomic:
/// v: VertexId, self: &SMesh ->  v.halfedge().next().run(self)

#[derive(Debug, Clone, Copy, PartialEq)]
enum QueryParam {
    Vertex(VertexId),
    Halfedge(HalfedgeId),
    Face(FaceId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum QueryOp {
    Vertex,
    Halfedge,
    Opposite,
    Next,
    Previous,
    Face,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MQuery<T> {
    initial: QueryParam,
    history: Vec<QueryOp>,
    phantom_data: PhantomData<T>,
}

impl<T> MQuery<T> {
    fn push<E>(&self, op: QueryOp) -> MQuery<E> {
        let mut history = self.history.clone();
        history.push(op);
        MQuery {
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
                QueryParam::Vertex(id) => eval_vertex_op(c, id, *op)?,
                QueryParam::Halfedge(id) => eval_halfedge_op(c, id, *op)?,
                QueryParam::Face(id) => eval_face_op(c, id, *op)?,
            };
        }
        Ok(value)
    }
}

pub trait ToMQuery<T> {
    fn q(&self) -> MQuery<T>;
}
macro_rules! impl_mquery {
    ($type:ident, $enum_variant:ident) => {
        impl ToMQuery<$type> for $type {
            fn q(&self) -> MQuery<$type> {
                MQuery {
                    initial: QueryParam::$enum_variant(self.clone()),
                    history: vec![],
                    phantom_data: PhantomData,
                }
            }
        }

        impl MQuery<$type> {
            fn run(self, mesh: &SMesh) -> SMeshResult<$type> {
                match self.evaluate_operations(mesh)? {
                    QueryParam::$enum_variant(id) => Ok(id),
                    _ => Err(SMeshError::DefaultError),
                }
            }
        }
    };
}

impl_mquery!(VertexId, Vertex);
impl_mquery!(HalfedgeId, Halfedge);
impl_mquery!(FaceId, Face);

impl MQuery<VertexId> {
    pub fn halfedge(&self) -> MQuery<HalfedgeId> {
        self.push(QueryOp::Halfedge)
    }
}
impl MQuery<HalfedgeId> {
    pub fn vert(&self) -> MQuery<VertexId> {
        self.push(QueryOp::Vertex)
    }
    pub fn opposite(&self) -> MQuery<HalfedgeId> {
        self.push(QueryOp::Opposite)
    }
    pub fn next(&self) -> MQuery<HalfedgeId> {
        self.push(QueryOp::Next)
    }
    pub fn prev(&self) -> MQuery<HalfedgeId> {
        self.push(QueryOp::Previous)
    }
}

impl MQuery<FaceId> {
    pub fn halfedge(&self) -> MQuery<HalfedgeId> {
        self.push(QueryOp::Halfedge)
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
}
