
#[derive(Debug, Clone, Default)]
pub struct SMesh {
    pub connectivity: Connectivity,

    // Attributes
    pub positions: SecondaryMap<VertexId, Vec3>,
    pub face_normals: Option<SecondaryMap<FaceId, Vec3>>,
    pub vertex_normals: Option<SecondaryMap<VertexId, Vec3>>,
    pub uvs: Option<SecondaryMap<HalfedgeId, Vec2>>,
    vertex_attributes: HashMap<String, CustomAttributeMap<VertexId>>,
    edge_attributes: HashMap<String, CustomAttributeMap<HalfedgeId>>,
    face_attributes: HashMap<String, CustomAttributeMap<FaceId>>,
}
