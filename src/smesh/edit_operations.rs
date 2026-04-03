use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use slotmap::SecondaryMap;

use crate::{bail, prelude::*};

/// Edit operations
impl SMesh {
    pub fn extrude(&mut self, face: FaceId) -> SMeshResult<FaceId> {
        let vertices = face.vertices(self).collect_vec();
        // Duplicate verts
        let mut vertex_pairs = Vec::new();
        for v in &vertices {
            let position = v.position(self)?;
            vertex_pairs.push((*v, self.add_vertex(position)));
        }

        assert_eq!(vertices.len(), vertex_pairs.len());

        self.delete_only_face(face)?;
        // Make faces
        for ((old_0, new_0), (old_1, new_1)) in
            vertex_pairs.iter().copied().circular_tuple_windows()
        {
            self.make_quad(old_0, old_1, new_1, new_0)?;
        }
        let top_face = self.make_face(vertex_pairs.iter().map(|(_old, new)| *new).collect_vec())?;
        Ok(top_face)
    }

    pub fn extrude_faces(&mut self, faces: Vec<FaceId>) -> SMeshResult<Vec<FaceId>> {
        // self.add_attribute_map::<HalfedgeId>("debug").unwrap();
        // Step 1: Collect all unique vertices and create mapping to new vertices
        let mut vertex_map = HashMap::new();
        for &face in faces.iter() {
            for vertex in face.vertices(self).collect_vec() {
                if !vertex_map.contains_key(&vertex) {
                    let position = vertex.position(self)?;
                    let new_vertex = self.add_vertex(position);
                    vertex_map.insert(vertex, new_vertex);
                }
            }
        }

        // Step 2: Collect the vertices of each face before deleting
        let mut face_vertex_map = HashMap::new();
        for &face in faces.iter() {
            let vertices = face.vertices(self).collect_vec();
            face_vertex_map.insert(face, vertices);
        }

        // Step 3: Collect boundary half-edges
        let selected_faces: HashSet<FaceId> = faces.iter().cloned().collect();
        let mut boundary_half_edges = Vec::new();
        let mut inner_half_edges = Vec::new();
        let mut boundary_vertices = Vec::new();
        for &face in faces.iter() {
            for half_edge in face.halfedges(self) {
                let opp = half_edge.opposite().run(self)?;
                let adjacent_face = opp.face().run(self).ok();
                if adjacent_face.is_none() || !selected_faces.contains(&adjacent_face.unwrap()) {
                    boundary_half_edges.push(half_edge);
                    boundary_vertices.push(half_edge.src_vert().run(self)?);
                    boundary_vertices.push(half_edge.dst_vert().run(self)?);
                } else {
                    let face = opp.face().run(self).ok();
                    if face.is_some() && selected_faces.contains(&face.unwrap()) {
                        inner_half_edges.push(half_edge);
                    }
                }
            }
        }
        //
        // for &he in &boundary_half_edges {
        //     self.attribute_mut("debug")
        //         .unwrap()
        //         .insert(he, "red".to_string());
        // }

        // Step 4: Delete the old vertices/faces
        if faces.len() == 1 {
            self.delete_only_face(*faces.first().unwrap())?;
        }
        for vertex in faces
            .iter()
            .flat_map(|f| f.vertices(self))
            .filter(|v| !boundary_vertices.contains(v))
            .collect_vec()
        {
            self.delete_vertex(vertex)?;
        }
        for he in inner_half_edges {
            self.delete_only_edge(he)?;
        }

        // Step 5: Create side faces along boundary edges
        for edge in boundary_half_edges.iter() {
            let src_old = edge.src_vert().run(self)?;
            let dst_old = edge.dst_vert().run(self)?;
            let src_new = vertex_map[&src_old];
            let dst_new = vertex_map[&dst_old];
            self.make_quad(src_old, dst_old, dst_new, src_new)?;
        }

        // Step 6: Create new faces on top
        let mut new_faces = Vec::new();
        for &face in faces.iter() {
            let old_vertices = &face_vertex_map[&face];
            let new_vertices = old_vertices.iter().map(|&v| vertex_map[&v]).collect_vec();
            let new_face = self.make_face(new_vertices)?;
            new_faces.push(new_face);
        }

        Ok(new_faces)
    }
    /// Inset a single face by creating a smaller inner face connected to the
    /// original boundary by quad side faces. The inner face vertices are
    /// moved toward the face centroid by `amount` (0.0 = no inset, 1.0 =
    /// collapsed to centroid). Returns the new inner face.
    pub fn inset(&mut self, face: FaceId, amount: f32) -> SMeshResult<FaceId> {
        let vertices = face.vertices(self).collect_vec();
        let centroid = self.get_face_centroid(face)?;

        // Create new vertices lerped toward centroid
        let mut vertex_pairs = Vec::new();
        for v in &vertices {
            let pos = v.position(self)?;
            let new_pos = pos + (centroid - pos) * amount;
            vertex_pairs.push((*v, self.add_vertex(new_pos)));
        }

        self.delete_only_face(face)?;

        // Create quad side faces connecting outer to inner
        for ((old_0, new_0), (old_1, new_1)) in
            vertex_pairs.iter().copied().circular_tuple_windows()
        {
            self.make_quad(old_0, old_1, new_1, new_0)?;
        }

        // Create inner face
        let inner_face =
            self.make_face(vertex_pairs.iter().map(|(_old, new)| *new).collect_vec())?;
        Ok(inner_face)
    }

    /// Inset multiple faces together. Shared edges between selected faces
    /// are preserved (no side quads are created along them). Returns the
    /// new inner faces.
    pub fn inset_faces(
        &mut self,
        faces: Vec<FaceId>,
        amount: f32,
    ) -> SMeshResult<Vec<FaceId>> {
        // Step 1: Compute centroid per face and create new inset vertices
        let mut vertex_map: HashMap<VertexId, VertexId> = HashMap::new();
        let mut face_centroids: HashMap<FaceId, glam::Vec3> = HashMap::new();

        for &face in &faces {
            let centroid = self.get_face_centroid(face)?;
            face_centroids.insert(face, centroid);
        }

        // For each face, figure out which centroid to use for each vertex.
        // A vertex shared by multiple selected faces gets averaged centroids.
        let mut vertex_centroid_sum: HashMap<VertexId, (glam::Vec3, u32)> = HashMap::new();
        for &face in &faces {
            let centroid = face_centroids[&face];
            for v in face.vertices(self) {
                let entry = vertex_centroid_sum.entry(v).or_insert((glam::Vec3::ZERO, 0));
                entry.0 += centroid;
                entry.1 += 1;
            }
        }

        // Create inset vertices
        for (&v, &(centroid_sum, count)) in &vertex_centroid_sum {
            let avg_centroid = centroid_sum / count as f32;
            let pos = v.position(self)?;
            let new_pos = pos + (avg_centroid - pos) * amount;
            vertex_map.insert(v, self.add_vertex(new_pos));
        }

        // Step 2: Collect face vertices before deletion
        let mut face_vertex_map: HashMap<FaceId, Vec<VertexId>> = HashMap::new();
        for &face in &faces {
            face_vertex_map.insert(face, face.vertices(self).collect_vec());
        }

        // Step 3: Find boundary vs inner half-edges
        let selected_faces: HashSet<FaceId> = faces.iter().cloned().collect();
        let mut boundary_half_edges = Vec::new();
        let mut inner_half_edges = Vec::new();
        let mut boundary_vertices = Vec::new();

        for &face in &faces {
            for he in face.halfedges(self) {
                let opp = he.opposite().run(self)?;
                let adj_face = opp.face().run(self).ok();
                if adj_face.is_none() || !selected_faces.contains(&adj_face.unwrap()) {
                    boundary_half_edges.push(he);
                    boundary_vertices.push(he.src_vert().run(self)?);
                    boundary_vertices.push(he.dst_vert().run(self)?);
                } else if selected_faces.contains(&adj_face.unwrap()) {
                    inner_half_edges.push(he);
                }
            }
        }

        // Step 4: Delete old faces/edges
        if faces.len() == 1 {
            self.delete_only_face(*faces.first().unwrap())?;
        }
        for vertex in faces
            .iter()
            .flat_map(|f| f.vertices(self))
            .filter(|v| !boundary_vertices.contains(v))
            .collect_vec()
        {
            self.delete_vertex(vertex)?;
        }
        for he in inner_half_edges {
            self.delete_only_edge(he)?;
        }

        // Step 5: Create quad side faces along boundary edges
        for edge in &boundary_half_edges {
            let src_old = edge.src_vert().run(self)?;
            let dst_old = edge.dst_vert().run(self)?;
            let src_new = vertex_map[&src_old];
            let dst_new = vertex_map[&dst_old];
            self.make_quad(src_old, dst_old, dst_new, src_new)?;
        }

        // Step 6: Create inner faces
        let mut new_faces = Vec::new();
        for &face in &faces {
            let old_verts = &face_vertex_map[&face];
            let new_verts = old_verts.iter().map(|&v| vertex_map[&v]).collect_vec();
            new_faces.push(self.make_face(new_verts)?);
        }

        Ok(new_faces)
    }

    pub fn extrude_edge(&mut self, e0: HalfedgeId) -> SMeshResult<HalfedgeId> {
        // Find boundary halfedge
        let e0 = match e0.is_boundary(self) {
            true => e0,
            false => {
                let opposite = e0.opposite().run(self)?;
                if !opposite.is_boundary(self) {
                    bail!("Can only extrude boundary edges");
                }
                opposite
            }
        };

        let v0 = e0.src_vert().run(self)?;
        let v1 = e0.dst_vert().run(self)?;

        let pos0 = v0.position(self)?;
        let pos1 = v1.position(self)?;
        let v0_new = self.add_vertex(pos0);
        let v1_new = self.add_vertex(pos1);

        // TODO: maybe check vertex normals (if exist) to determine order?
        self.make_face(vec![v0, v1, v1_new, v0_new])?;
        let new_edge = v0_new.halfedge_to(v1_new).run(self)?;
        Ok(new_edge)
    }

    pub fn extrude_edge_chain(&mut self, edges: Vec<HalfedgeId>) -> SMeshResult<Vec<HalfedgeId>> {
        let mut boundary_edges = Vec::new();
        for e in edges {
            let eb = match e.is_boundary(self) {
                true => e,
                false => {
                    let opposite = e.opposite().run(self)?;
                    if !opposite.is_boundary(self) {
                        bail!("Can only extrude boundary edges");
                    }
                    opposite
                }
            };
            boundary_edges.push(eb);
        }

        // Assert all are connected in sequence and check if they form a loop
        let mut is_loop = false;
        for (i, (current, next)) in boundary_edges.iter().circular_tuple_windows().enumerate() {
            let is_last_iteration = i == boundary_edges.len() - 1;
            match current.next().run(self) {
                Ok(he) => {
                    if he == *next {
                        if is_last_iteration {
                            is_loop = true;
                        }
                    } else {
                        bail!("Not an edge chain");
                    }
                }
                Err(_) => {
                    if !is_last_iteration {
                        bail!("Halfedge has no next")
                    }
                }
            }
        }

        let vertices = boundary_edges
            .iter()
            .flat_map(|e| {
                vec![
                    e.src_vert().run(self).unwrap(),
                    e.dst_vert().run(self).unwrap(),
                ]
            })
            .collect_vec();
        // Duplicate verts
        let mut vertex_pairs = Vec::new();
        for v in &vertices {
            let position = v.position(self)?;
            vertex_pairs.push((*v, self.add_vertex(position)));
        }

        assert_eq!(vertices.len(), vertex_pairs.len());

        // Make faces
        let mut new_edges = Vec::new();
        for ((old_0, new_0), (old_1, new_1)) in vertex_pairs
            .iter()
            .copied()
            .circular_tuple_windows()
            .take(vertex_pairs.len() - (if is_loop { 0 } else { 1 }))
        {
            self.make_quad(old_0, old_1, new_1, new_0)?;
            new_edges.push((new_0).halfedge_to(new_1).run(self)?);
        }
        Ok(new_edges)
    }

    pub fn subdivide<T: Into<MeshSelection>>(
        &mut self,
        selection: T,
    ) -> SMeshResult<MeshSelection> {
        let s: MeshSelection = selection.into();
        let faces = s.clone().resolve_to_faces(self)?;
        let halfedges = s.clone().resolve_to_halfedges(self)?;

        let face_corners = self
            .faces()
            .map(|f| (f, f.halfedge().src_vert().run(self).unwrap()))
            .collect::<HashMap<FaceId, VertexId>>();

        let mut he_cache = HashSet::new();
        // Returned selection
        let mut selection = MeshSelection::new();
        for he in halfedges {
            let he_opposite = he.opposite().run(self)?;
            if he_cache.contains(&he) {
                continue;
            }
            let p0 = he.src_vert().position(self)?;
            let p1 = he.dst_vert().position(self)?;
            let v = self.add_vertex(0.5 * (p0 + p1));
            let new_he = self.insert_vertex(he, v)?;
            he_cache.insert(he);
            he_cache.insert(he_opposite);
            selection.insert(he);
            selection.insert(he_opposite);
            selection.insert(new_he);
            selection.insert(new_he.opposite().run(self)?);
        }
        for f in faces {
            let valence = f.valence(self) / 2;
            let corner = face_corners[&f];
            let corner_edge = f
                .halfedges(self)
                .find(|he| he.src_vert().run(self).unwrap() == corner)
                .unwrap();
            let he_loop = self.halfedge_loop(corner_edge.next().run(self)?);
            if valence == 3 {
                self.delete_only_face(f)?;
                for (h0, h1) in he_loop.iter().circular_tuple_windows().step_by(2) {
                    let f = self.make_triangle(
                        h0.src_vert().run(self)?,
                        h1.src_vert().run(self)?,
                        h1.dst_vert().run(self)?,
                    )?;
                    selection.insert(f);
                }
                // Middle tri
                let f = self.make_triangle(
                    he_loop[0].src_vert().run(self)?,
                    he_loop[2].src_vert().run(self)?,
                    he_loop[4].src_vert().run(self)?,
                )?;
                selection.insert(f);
            }
            if valence == 4 {
                let center = self.get_face_centroid(f)?;
                let v_c = self.add_vertex(center);
                self.delete_only_face(f)?;
                for (h0, h1) in he_loop.iter().circular_tuple_windows().step_by(2) {
                    let f = self.make_quad(
                        h0.src_vert().run(self)?,
                        h1.src_vert().run(self)?,
                        h1.dst_vert().run(self)?,
                        v_c,
                    )?;
                    selection.insert(f);
                }
            }
            // valence > 4: ngons simply stay as ngons (same behaviour as blender)
        }

        Ok(selection)
    }

    pub fn combine_with(&mut self, other: SMesh) -> SMeshResult<()> {
        // Copy verts
        let mut v_map = HashMap::new();
        for (id, v) in other.connectivity.vertices {
            let id_new = self.vertices_mut().insert(v.clone());
            v_map.insert(id, id_new);
        }
        // Copy faces
        let mut f_map = HashMap::new();
        for (id, f) in other.connectivity.faces {
            let id_new = self.faces_mut().insert(f.clone());
            f_map.insert(id, id_new);
        }
        // Copy halfedges
        let mut he_map = HashMap::new();
        for (id, he) in other.connectivity.halfedges {
            let mut halfedge = he.clone();
            halfedge.vertex = v_map[&halfedge.vertex];
            if let Some(f) = halfedge.face {
                halfedge.face = Some(f_map[&f]);
            }
            let id_new = self.halfedges_mut().insert(halfedge.clone());
            he_map.insert(id, id_new);
        }
        // Remap remaining ids in halfedges
        for id in he_map.values() {
            let he = self.halfedges_mut().get_mut(*id).unwrap();
            if let Some(opp) = he.opposite {
                he.opposite = Some(he_map[&opp]);
            }
            if let Some(next) = he.next {
                he.next = Some(he_map[&next]);
            }
            if let Some(prev) = he.prev {
                he.prev = Some(he_map[&prev]);
            }
        }
        // Remap remaining ids in vertices
        for id in v_map.values() {
            if let Ok(he) = id.halfedge().run(self) {
                self.get_mut(*id).set_halfedge(Some(he_map[&he]))?;
            }
        }

        // Remap remaining ids in faces
        for id in f_map.values() {
            if let Ok(he) = id.halfedge().run(self) {
                self.get_mut(*id).set_halfedge(Some(he_map[&he]))?;
            }
        }

        // Copy attributes
        for (id, value) in other.positions {
            self.positions.insert(v_map[&id], value);
        }
        if let Some(vertex_normals) = other.vertex_normals {
            for (id, value) in vertex_normals {
                if self.vertex_normals.is_none() {
                    self.vertex_normals = Some(SecondaryMap::new());
                }
                self.vertex_normals
                    .as_mut()
                    .unwrap()
                    .insert(v_map[&id], value);
            }
        }
        if let Some(face_normals) = other.face_normals {
            for (id, value) in face_normals {
                if self.face_normals.is_none() {
                    self.face_normals = Some(SecondaryMap::new());
                }
                self.face_normals
                    .as_mut()
                    .unwrap()
                    .insert(f_map[&id], value);
            }
        }
        if let Some(uvs) = other.vertex_uvs {
            for (id, value) in uvs {
                if self.vertex_uvs.is_none() {
                    self.vertex_uvs = Some(SecondaryMap::new());
                }
                self.vertex_uvs.as_mut().unwrap().insert(v_map[&id], value);
            }
        }
        // TODO: copy custom attributes 
        // for attr in self.vertex_attributes {
        //
        // }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn inset_single_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let f = mesh.make_quad(v0, v1, v2, v3)?;

        // 1 face, 4 verts, 4 edges
        assert_eq!(mesh.faces().len(), 1);
        assert_eq!(mesh.vertices().len(), 4);

        let inner = mesh.inset(f, 0.5)?;

        // Should now have: 4 side quads + 1 inner face = 5 faces
        assert_eq!(mesh.faces().len(), 5);
        // 4 original + 4 inner = 8 vertices
        assert_eq!(mesh.vertices().len(), 8);

        // Inner face centroid should be at the original centroid (origin)
        let inner_centroid = mesh.get_face_centroid(inner)?;
        assert!(inner_centroid.length() < 0.01);

        // Inner face should be smaller — check one vertex
        let inner_v = inner.vertices(mesh).next().unwrap();
        let pos = inner_v.position(mesh)?;
        // At 50% inset, vertices should be halfway to centroid
        assert!(pos.x.abs() < 0.6, "Inner vertex should be inset, got x={}", pos.x);
        assert!(pos.z.abs() < 0.6, "Inner vertex should be inset, got z={}", pos.z);

        Ok(())
    }

    #[test]
    fn inset_zero_amount_preserves_shape() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let f = mesh.make_quad(v0, v1, v2, v3)?;

        let inner = mesh.inset(f, 0.0)?;

        // Inner face should be same size as original (zero inset)
        let inner_v: Vec<_> = inner.vertices(mesh).collect();
        for v in inner_v {
            let pos = v.position(mesh)?;
            assert!(
                (pos.x.abs() - 1.0).abs() < 0.01 && (pos.z.abs() - 1.0).abs() < 0.01,
                "At 0 inset, inner verts should match original positions"
            );
        }

        Ok(())
    }

    #[test]
    fn inset_triangle() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(0.5, 0.0, 1.0));
        let f = mesh.make_triangle(v0, v1, v2)?;

        let inner = mesh.inset(f, 0.3)?;

        // 3 side quads + 1 inner tri = 4 faces
        assert_eq!(mesh.faces().len(), 4);
        assert_eq!(inner.valence(mesh), 3);

        Ok(())
    }

    #[test]
    fn inset_faces_multiple() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(0.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v3 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v4 = mesh.add_vertex(vec3(0.0, 0.0, 1.0));
        let v5 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let f0 = mesh.make_quad(v0, v1, v4, v5)?;
        let f1 = mesh.make_quad(v1, v2, v3, v4)?;

        assert_eq!(mesh.faces().len(), 2);

        let inner = mesh.inset_faces(vec![f0, f1], 0.3)?;
        assert_eq!(inner.len(), 2);

        // Shared edge between f0 and f1 should not produce side quads
        // Boundary edges: 6 (top, bottom, left, right of the 2x1 strip)
        // So: 6 side quads + 2 inner faces = 8 faces
        assert_eq!(mesh.faces().len(), 8);

        Ok(())
    }
}
