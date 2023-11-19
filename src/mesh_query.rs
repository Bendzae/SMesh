use crate::error::*;
use crate::smesh::*;

#[derive(Debug, Clone)]
pub struct MeshQuery<'a, T> {
    pub conn: &'a Connectivity,
    pub value: SMeshResult<T>,
}

impl<'a, T: PartialEq> PartialEq for MeshQuery<'a, T> {
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

impl<'a, E> MeshQuery<'a, E> {
    fn chain_result<T>(&self, result: SMeshResult<T>) -> MeshQuery<T> {
        MeshQuery {
            conn: self.conn,
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
        impl<'a> EvalMeshQuery<$id_type, $type> for MeshQuery<'a, $id_type> {
            fn id(&self) -> SMeshResult<$id_type> {
                self.value.clone()
            }
            fn eval(&self) -> SMeshResult<($id_type, $type)> {
                self.value
                    .and_then(|id| match self.conn.$container_name.get(id) {
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

impl<'a> MeshQuery<'a, VertexId> {
    pub fn halfedge(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, vertex)| vertex.halfedge.ok_or(SMeshError::VertexHasNoHalfEdge(id)));
        self.chain_result(res)
    }

    pub fn halfedge_to(&self, dst_vertex: VertexId) -> MeshQuery<HalfedgeId> {
        let initial_he = self.halfedge().id();
        let mut he = initial_he;

        let res = loop {
            let inner_he = self.chain_result(he);
            match inner_he.dst_vert().id() {
                Ok(id) => {
                    if id == dst_vertex {
                        break inner_he.id();
                    }
                    he = inner_he.cw_rotated_neighbour().id();
                    if he == initial_he {
                        break Err(SMeshError::DefaultError);
                    }
                }
                Err(e) => {
                    break Err(e);
                }
            }
        };
        self.chain_result(res)
    }

    pub fn is_boundary(&self) -> bool {
        self.halfedge().face().eval().is_err()
    }

    pub fn is_isolated(&self) -> bool {
        self.halfedge().eval().is_err()
    }

    pub fn valence(&self) -> usize {
        self.vertices().count()
    }
}

///
/// Halfedge
///

impl<'a> MeshQuery<'a, HalfedgeId> {
    pub fn vert(&self) -> MeshQuery<VertexId> {
        let res = self.eval().map(|(_, he)| he.vertex);
        self.chain_result(res)
    }

    pub fn edge(&self) -> MeshQuery<EdgeId> {
        let res = self
            .eval()
            .and_then(|(id, he)| he.edge.ok_or(SMeshError::HalfedgeHasNoRef(id)));
        self.chain_result(res)
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
    pub fn ccw_rotated_neighbour(&self) -> MeshQuery<HalfedgeId> {
        self.chain_result(self.prev().opposite().id())
    }
    pub fn cw_rotated_neighbour(&self) -> MeshQuery<HalfedgeId> {
        self.chain_result(self.opposite().next().id())
    }
    pub fn src_vert(&self) -> MeshQuery<VertexId> {
        self.chain_result(self.opposite().vert().id())
    }
    pub fn dst_vert(&self) -> MeshQuery<VertexId> {
        self.chain_result(self.vert().id())
    }
    pub fn is_boundary(&self) -> bool {
        self.face().eval().is_err()
    }
}

///
/// Face
///

impl<'a> MeshQuery<'a, FaceId> {
    pub fn halfedge(&self) -> MeshQuery<HalfedgeId> {
        let res = self
            .eval()
            .and_then(|(id, face)| face.halfedge.ok_or(SMeshError::FaceHasNoHalfEdge(id)));
        self.chain_result(res)
    }

    pub fn valence(&self) -> usize {
        self.vertices().count()
    }
}

///
/// Query initializers
///

impl SMesh {
    pub fn q<T>(&self, id: T) -> MeshQuery<T> {
        MeshQuery {
            conn: &self.connectivity,
            value: Ok(id),
        }
    }
}

impl Connectivity {
    pub fn q<T>(&self, id: T) -> MeshQuery<T> {
        MeshQuery {
            conn: &self,
            value: Ok(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(mesh.vertices().len(), 4);
        assert_eq!(mesh.halfedges().len(), 8);
        assert_eq!(mesh.faces().len(), 1);
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

        let he_0_to_1 = mesh.q(v0).halfedge_to(v1).id()?;
        assert_eq!(mesh.q(he_0_to_1).src_vert().id()?, v0);
        assert_eq!(mesh.q(he_0_to_1).dst_vert().id()?, v1);

        Ok(())
    }

    #[test]
    fn valence() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        let v0 = mesh.add_vertex(vec3(-1.0, -1.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, -1.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 1.0, 0.0));

        let face_id = mesh.add_face(vec![v0, v1, v2, v3]).unwrap();

        assert_eq!(mesh.q(face_id).valence(), 4);
        assert_eq!(mesh.q(v0).valence(), 2);

        Ok(())
    }
}
