use crate::error::*;
use crate::smesh::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct MeshQuery<T> {
    pub conn: Rc<RefCell<Connectivity>>,
    pub value: SMeshResult<T>,
}

impl<T: PartialEq> PartialEq for MeshQuery<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.value.is_err() || other.value.is_err() {
            return false;
        }
        self.value
            .as_ref()
            .unwrap()
            .eq(other.value.as_ref().unwrap())
    }
}

impl<E> MeshQuery<E> {
    fn chain_result<T>(&self, result: SMeshResult<T>) -> MeshQuery<T> {
        MeshQuery {
            conn: self.conn.clone(),
            value: result,
        }
    }
}

pub trait EvalMeshQuery<IdType, ResultType> {
    fn id(&self) -> SMeshResult<IdType>;
    fn eval(&self) -> SMeshResult<(IdType, ResultType)>;
    fn res(&self) -> SMeshResult<ResultType> {
        self.eval().map(|(_, result)| result)
    }
}

macro_rules! eval_mesh_query_impl {
    ($type:ident, $id_type:ident, $container_name:ident, $error_type:ident) => {
        impl EvalMeshQuery<$id_type, $type> for MeshQuery<$id_type> {
            fn id(&self) -> SMeshResult<$id_type> {
                self.value.clone()
            }
            fn eval(&self) -> SMeshResult<($id_type, $type)> {
                self.value
                    .and_then(|id| match self.conn.borrow().$container_name.get(id) {
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

impl MeshQuery<VertexId> {
    pub fn halfedge(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, vertex)| vertex.halfedge.ok_or(SMeshError::VertexHasNoHalfEdge(id)));
        self.chain_result(res)
    }

    pub fn halfedge_to(&self, dst_vertex: VertexId) -> MeshQuery<HalfedgeId> {
        let initial_he = self.halfedge();
        let mut he = self.halfedge();

        let res = loop {
            match &he.vert().id() {
                Ok(id) => {
                    if *id == dst_vertex {
                        break he.id();
                    }
                }
                Err(e) => {
                    break Err(*e);
                }
            }

            let he_rot = he.cw_rotated_neighbour();
            let he_rot_id = he_rot.id();

            if he_rot_id.is_err() {
                break he_rot_id;
            }
            if he_rot_id == initial_he.clone().id() {
                break Err(SMeshError::DefaultError);
            }
            he = he_rot;
        };
        self.chain_result(res)
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

impl MeshQuery<HalfedgeId> {
    pub fn vert(&self) -> MeshQuery<VertexId> {
        let res = self.eval().map(|(_, he)| he.vertex);
        self.chain_result(res)
    }

    pub fn edge(&self) -> MeshQuery<EdgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.edge.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        MeshQuery {
            conn: self.conn.clone(),
            value: res,
        }
    }

    pub fn face(&self) -> MeshQuery<FaceId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.face.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        self.chain_result(res)
    }

    pub fn opposite(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.opposite.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        self.chain_result(res)
    }

    pub fn prev(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.prev.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        self.chain_result(res)
    }

    pub fn next(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.next.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        self.chain_result(res)
    }

    pub fn cw_rotated_neighbour(&self) -> MeshQuery<HalfedgeId> {
        self.chain_result(self.opposite().next().id())
    }

    pub fn is_boundary(&self) -> bool {
        self.face().eval().is_err()
    }
}

///
/// Face
///

impl MeshQuery<FaceId> {
    pub fn halfedge(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, face)| face.halfedge.ok_or(SMeshError::FaceHasNoHalfEdge(id)));
        self.chain_result(res)
    }
}

///
/// SMesh query initializers
///

impl SMesh {
    pub fn q<T>(&self, id: T) -> MeshQuery<T> {
        MeshQuery {
            conn: self.connectivity.clone(),
            value: Ok(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;
    #[test]
    fn test() {
        let mut mesh = &mut SMesh::new();

        let verts = vec![
            mesh.add_vertex(vec3(-1.0, -1.0, 0.0)),
            mesh.add_vertex(vec3(-1.0, 1.0, 0.0)),
            mesh.add_vertex(vec3(1.0, 1.0, 0.0)),
            mesh.add_vertex(vec3(1.0, -1.0, 0.0)),
        ];

        let face_id = mesh.add_face(verts);

        assert!(face_id.is_ok())
    }
}
