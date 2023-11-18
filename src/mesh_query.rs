use crate::error::*;
use crate::smesh::*;

pub struct MeshQuery<'a, T> {
    pub mesh: &'a SMesh,
    pub value: SMeshResult<T>,
}

impl<T> MeshQuery<'_, T> {
    pub fn run(self) -> SMeshResult<T> {
        self.value
    }
}

pub trait EvalMeshQuery<IdType, ResultType> {
    fn eval(&self) -> SMeshResult<(IdType, ResultType)>;

    fn res(&self) -> SMeshResult<ResultType> {
        self.eval().map(|(_, result)| result)
    }
}

macro_rules! eval_mesh_query_impl {
    ($type:ident, $id_type:ident, $container_name:ident, $error_type:ident) => {
        impl EvalMeshQuery<$id_type, $type> for MeshQuery<'_, $id_type> {
            fn eval(&self) -> SMeshResult<($id_type, $type)> {
                self.value
                    .and_then(|id| match self.mesh.$container_name.get(id) {
                        Some(element) => Ok((id, element.clone())),
                        None => Err(SMeshError::$error_type(id)),
                    })
            }
        }
    };
}

eval_mesh_query_impl!(Vertex, VertexId, vertices, VertexNotFound);
eval_mesh_query_impl!(Halfedge, HalfedgeId, halfedges, HalfedgeNotFound);
eval_mesh_query_impl!(Face, FaceId, faces, FaceNotFound);

///
/// Implementations
///

///
/// Vertex
///

impl MeshQuery<'_, VertexId> {
    pub fn halfedge(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, vertex)| vertex.halfedge.ok_or(SMeshError::VertexHasNoHalfEdge(id)));
        MeshQuery {
            mesh: self.mesh,
            value: res,
        }
    }

    pub fn is_boundary(&self) -> bool {
        self.halfedge().face().eval().is_err()
    }

    pub fn is_isolated(&self) -> bool {
        self.halfedge().eval().is_err()
    }
}

///
/// Halfedge
///

impl MeshQuery<'_, HalfedgeId> {
    pub fn vert(&self) -> MeshQuery<VertexId> {
        let res = self.res().map(|he| he.vertex);
        MeshQuery {
            mesh: self.mesh,
            value: res,
        }
    }

    pub fn edge(&self) -> MeshQuery<EdgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.edge.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        MeshQuery {
            mesh: self.mesh,
            value: res,
        }
    }

    pub fn face(&self) -> MeshQuery<FaceId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.face.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        MeshQuery {
            mesh: self.mesh,
            value: res,
        }
    }

    pub fn opposite(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.opposite.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        MeshQuery {
            mesh: self.mesh,
            value: res,
        }
    }

    pub fn prev(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.prev.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        MeshQuery {
            mesh: self.mesh,
            value: res,
        }
    }

    pub fn next(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.next.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        MeshQuery {
            mesh: self.mesh,
            value: res,
        }
    }
}

///
/// Face
///

impl MeshQuery<'_, FaceId> {
    pub fn halfedge(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, face)| face.halfedge.ok_or(SMeshError::FaceHasNoHalfEdge(id)));
        MeshQuery {
            mesh: self.mesh,
            value: res,
        }
    }
}

///
/// SMesh query initializers
///

impl SMesh {
    pub fn q_vert(&self, vertex_id: VertexId) -> MeshQuery<VertexId> {
        MeshQuery {
            mesh: self,
            value: Ok(vertex_id),
        }
    }

    pub fn q_he(&self, halfedge_id: HalfedgeId) -> MeshQuery<HalfedgeId> {
        MeshQuery {
            mesh: self,
            value: Ok(halfedge_id),
        }
    }

    pub fn q_edge(&self, edge_id: EdgeId) -> MeshQuery<EdgeId> {
        MeshQuery {
            mesh: self,
            value: Ok(edge_id),
        }
    }

    pub fn q_face(&self, face_id: FaceId) -> MeshQuery<FaceId> {
        MeshQuery {
            mesh: self,
            value: Ok(face_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        let mesh = SMesh::new();
    }
}
